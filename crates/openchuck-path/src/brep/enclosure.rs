// SPDX-License-Identifier: AGPL-3.0-or-later
//! The enclosure of a B-rep part from its exact faces (OP-23, OP-24;
//! decision 0666 option C, issue #800): points whose convex hull holds every
//! face, so any bound taken as the largest value of a convex function over
//! them (a support in a direction, an extent along an axis, a projected
//! rectangle, a distance from the rotary axis) is never inside the part.
//! Facing reads the part only through such bounds.
//!
//! Each face is bounded from its exact geometry, never from its
//! tessellation:
//!
//! - **A plane** lies in the convex hull of its boundary, so it is bounded by
//!   its edges' exact curves. If an edge has no exact curve, the face falls
//!   back to its parameter box: the plane's parametrisation is affine, so the
//!   box's four corners hold it.
//! - **An arc** `c + a cos t + b sin t` (a circle, an ellipse, or a circle
//!   seen through an affine map) lies in the polygon of its end points and
//!   the corners where the tangents at the ends of `n` equal sub-arcs meet.
//!   Each corner lies `(1 / cos(d / 2) - 1) |a cos m + b sin m|` from the arc,
//!   at most [`ANALYTIC_EXCESS`].
//! - **A cylinder, cone, sphere or torus** is a meridian swept round its
//!   axis. Over the face's parameter box, the meridian (a segment, or an arc
//!   bounded as above) lies in a polygon in the plane through the axis; each
//!   polygon corner, swept over the box's angles, is an arc bounded as above.
//!   A point of the face is a combination of meridian corners at its own
//!   angle, each in the hull of its swept arc's points.
//! - **A B-spline face** (rational, with positive weights) lies in the hull
//!   of its control points. It is split exactly into Bezier pieces at its
//!   knots and the edges of its parameter box, and the pieces are halved by
//!   de Casteljau's algorithm until each control point lies within
//!   [`SPLINE_TARGET`] of the surface point at its Greville parameter (or a
//!   work limit is reached). Then the support of the points in any direction
//!   exceeds the patch's by at most the largest such distance, which is the
//!   [`Enclosure::spline_excess`] the notes state. B-spline edges are bounded
//!   the same way.
//!
//! Trims are bounded by the parameter box, which can only enlarge a bound.

use std::collections::BTreeMap;
use std::f64::consts::{FRAC_PI_2, TAU};

use openchuck_types::brep::{
    BSplineCurve, BSplinePatch, BrepEdge, BrepFace, BrepModel, EdgeCurve, EdgeId, Frame, Surface,
};
use openchuck_types::geometry::{Point3, Vec3};
use openchuck_types::mesh::TriangleMesh;

use super::hull::{Hom, Net, bezier_nets, lerp, pieces, project};
use crate::error::{PathError, Result};

/// The most an analytic face's bound lies outside it, millimetres: a
/// circumscribed arc's corners are placed no further out than this.
pub const ANALYTIC_EXCESS: f64 = 0.0005;

/// The refinement target for a B-spline face or edge, millimetres: its
/// control points are split until each lies within this of the surface (or
/// curve) point at its Greville parameter, or [`SPLINE_WORK`] is reached.
pub const SPLINE_TARGET: f64 = 0.001;

/// The most Bezier pieces one B-spline face or edge is split into.
const SPLINE_WORK: usize = 16384;

/// The most points an enclosure may hold.
const MAX_POINTS: usize = 4_000_000;

/// Points whose convex hull holds every face of a B-rep part, and how far
/// outside its faces a bound taken from them can lie.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Enclosure {
    /// The points, in face order (deterministic).
    pub points: Vec<Point3>,
    /// The most a bound lies outside the part's analytic faces and exact
    /// edges, millimetres (at most [`ANALYTIC_EXCESS`]; zero along a flat
    /// face's own extremes).
    pub analytic_excess: f64,
    /// The most a bound lies outside the part's B-spline faces and edges,
    /// millimetres, from their refined control points; `None` when the part
    /// has none.
    pub spline_excess: Option<f64>,
}

impl Enclosure {
    /// The enclosure as Facing reads a part (OP-24): Facing on a machine that
    /// turns about X takes every bound of the part along X, across X or as
    /// the radius from X, so this is the prism over the convex outline of the
    /// points seen along X (their Y-Z convex hull), between the points'
    /// smallest and largest X. Its bounds equal the points' along X and in
    /// every direction across X, and its radius equals theirs; in an oblique
    /// direction it lies further out, never further in. A mesh whose triangles
    /// refer to every vertex, for readers that take bounds from the vertices
    /// a mesh's triangles refer to; the triangles have no surface meaning.
    pub fn mesh(&self) -> TriangleMesh {
        let (lo, hi) = self
            .points
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
                (lo.min(p.x), hi.max(p.x))
            });
        let outline = outline_yz(&self.points);
        let points: Vec<Point3> = outline
            .iter()
            .flat_map(|&[y, z]| [Point3::new(lo, y, z), Point3::new(hi, y, z)])
            .collect();
        points_mesh(points)
    }
}

/// The convex hull of the points' (y, z), counter-clockwise (Andrew's
/// monotone chain). A point is dropped only when it lies inside the chain
/// by more than the rounding of the test, so no point outside the true hull
/// is ever dropped (near-collinear points are kept). Fewer than three
/// distinct points are returned as they are.
fn outline_yz(points: &[Point3]) -> Vec<[f64; 2]> {
    let mut p: Vec<[f64; 2]> = points.iter().map(|p| [p.y, p.z]).collect();
    p.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    p.dedup();
    if p.len() < 3 {
        return p;
    }
    // Clearly inside: `a` lies to the right of `o -> b` by more than the
    // cross product's rounding, which is far below 1e-12 of its terms.
    let inside = |o: [f64; 2], a: [f64; 2], b: [f64; 2]| {
        let (ax, ay, bx, by) = (a[0] - o[0], a[1] - o[1], b[0] - o[0], b[1] - o[1]);
        let cross = ax * by - ay * bx;
        cross < -1e-12 * (ax.abs() + ay.abs()) * (bx.abs() + by.abs())
    };
    let chain = |iter: &mut dyn Iterator<Item = [f64; 2]>| {
        let mut out: Vec<[f64; 2]> = Vec::new();
        for q in iter {
            while out.len() >= 2 {
                let (Some(&o), Some(&a)) = (out.get(out.len() - 2), out.last()) else {
                    break;
                };
                if inside(o, a, q) {
                    out.pop();
                } else {
                    break;
                }
            }
            out.push(q);
        }
        out
    };
    // The lower chain left to right turns left; points to its right are
    // inside. The upper chain is the same walk right to left.
    let mut lower = chain(&mut p.iter().copied().map(|q| [q[0], q[1]]));
    let mut upper = chain(&mut p.iter().rev().copied());
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

/// A mesh whose triangles refer to every one of `points`.
fn points_mesh(points: Vec<Point3>) -> TriangleMesh {
    let n = points.len();
    let last = u32::try_from(n.saturating_sub(1)).unwrap_or(u32::MAX);
    let index = |i: usize| u32::try_from(i).map_or(last, |i| i.min(last));
    let triangles = (0..n)
        .step_by(3)
        .map(|i| [index(i), index(i + 1), index(i + 2)])
        .collect();
    TriangleMesh {
        vertices: points,
        triangles,
    }
}

/// The enclosure of `model`'s faces (see the module documentation). Fails,
/// naming the face, for a face it cannot bound: a surface kind it does not
/// know, geometry that is not finite, a rational patch with a weight that is
/// not positive, or a flat face with neither exact edges nor a finite
/// parameter box.
pub fn enclose(model: &BrepModel) -> Result<Enclosure> {
    if model.faces.is_empty() {
        return Err(PathError::refused(
            crate::topic::SURFACE,
            "the part has no faces to bound",
        ));
    }
    let edges: BTreeMap<EdgeId, &BrepEdge> = model.edges.iter().map(|e| (e.id, e)).collect();
    let mut b = Builder::default();
    for face in &model.faces {
        b.face(face, &edges)?;
        if b.points.len() > MAX_POINTS {
            return Err(PathError::refused(
                crate::topic::SURFACE,
                "the part's bound needs more points than the work limit allows",
            ));
        }
    }
    if b.points.iter().any(|p| !p.is_finite()) {
        return Err(PathError::refused(
            crate::topic::SURFACE,
            "the part's bound has a point that is not finite",
        ));
    }
    Ok(Enclosure {
        points: b.points,
        analytic_excess: b.analytic,
        spline_excess: b.spline,
    })
}

fn bad(face: u32, what: impl std::fmt::Display) -> PathError {
    PathError::refused(crate::topic::SURFACE, format!("face {face}: {what}"))
}

#[derive(Default)]
struct Builder {
    points: Vec<Point3>,
    analytic: f64,
    spline: Option<f64>,
}

fn finite_frame(f: &Frame) -> bool {
    f.origin.is_finite() && f.x.is_finite() && f.y.is_finite() && f.z.is_finite()
}

impl Builder {
    fn face(&mut self, face: &BrepFace, edges: &BTreeMap<EdgeId, &BrepEdge>) -> Result<()> {
        let n = face.id.0;
        let [u0, u1, v0, v1] = face.uv_bounds;
        let finite_box = face.uv_bounds.iter().all(|x| x.is_finite()) && u0 <= u1 && v0 <= v1;
        match &face.geometry {
            Surface::Plane { frame } => {
                if !finite_frame(frame) {
                    return Err(bad(n, "the plane's frame is not finite"));
                }
                if !face.edges.is_empty() && self.plane_edges(face, edges)? {
                    return Ok(());
                }
                if !finite_box {
                    return Err(bad(
                        n,
                        "a flat face needs exact edges or a finite parameter box to be bounded",
                    ));
                }
                for (u, v) in [(u0, v0), (u1, v0), (u1, v1), (u0, v1)] {
                    self.points.push(frame.origin + frame.x * u + frame.y * v);
                }
                Ok(())
            }
            Surface::Cylinder { frame, radius } => {
                self.revolved(n, frame, finite_box, face.uv_bounds, |m| {
                    m.segment([*radius, v0], [*radius, v1]);
                })
            }
            Surface::Cone {
                frame,
                radius,
                semi_angle,
            } => {
                let (s, c) = semi_angle.sin_cos();
                self.revolved(n, frame, finite_box, face.uv_bounds, |m| {
                    m.segment([radius + v0 * s, v0 * c], [radius + v1 * s, v1 * c]);
                })
            }
            Surface::Sphere { frame, radius } => {
                self.revolved(n, frame, finite_box, face.uv_bounds, |m| {
                    m.arc([0.0, 0.0], *radius, v0, v1);
                })
            }
            Surface::Torus {
                frame,
                major,
                minor,
            } => self.revolved(n, frame, finite_box, face.uv_bounds, |m| {
                m.arc([*major, 0.0], *minor, v0, v1);
            }),
            Surface::BSpline(patch) => {
                if !finite_box {
                    return Err(bad(n, "the parameter box is not finite"));
                }
                self.patch(n, patch, face.uv_bounds)
            }
            _ => Err(bad(n, "this surface kind has no bound")),
        }
    }

    /// Bounds a flat face by its edges' exact curves; `false` when an edge
    /// is missing or has no exact curve (and is not a point), so the caller
    /// falls back to the parameter box.
    fn plane_edges(
        &mut self,
        face: &BrepFace,
        edges: &BTreeMap<EdgeId, &BrepEdge>,
    ) -> Result<bool> {
        let mark = self.points.len();
        let (analytic, spline) = (self.analytic, self.spline);
        for id in &face.edges {
            let bounded = match edges.get(id) {
                Some(edge) => self.edge(face.id.0, edge)?,
                None => false,
            };
            if !bounded {
                self.points.truncate(mark);
                self.analytic = analytic;
                self.spline = spline;
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn edge(&mut self, face: u32, edge: &BrepEdge) -> Result<bool> {
        match &edge.curve {
            Some(EdgeCurve::Line {
                origin,
                direction,
                first,
                last,
            }) => {
                self.points.push(*origin + *direction * *first);
                self.points.push(*origin + *direction * *last);
                Ok(true)
            }
            Some(EdgeCurve::Circle {
                frame,
                radius,
                first,
                last,
            }) => {
                let excess = self.arc(
                    frame.origin,
                    frame.x * *radius,
                    frame.y * *radius,
                    *first,
                    *last,
                    ANALYTIC_EXCESS,
                );
                self.analytic = self.analytic.max(excess);
                Ok(true)
            }
            Some(EdgeCurve::Ellipse {
                frame,
                major,
                minor,
                first,
                last,
            }) => {
                let excess = self.arc(
                    frame.origin,
                    frame.x * *major,
                    frame.y * *minor,
                    *first,
                    *last,
                    ANALYTIC_EXCESS,
                );
                self.analytic = self.analytic.max(excess);
                Ok(true)
            }
            Some(EdgeCurve::BSpline(curve)) => self.curve(face, curve),
            _ => {
                // A degenerate edge (a point, such as a cone's apex) needs no
                // curve; any other edge without one cannot be bounded.
                let Some(&p) = edge.polyline.first() else {
                    return Ok(false);
                };
                if edge
                    .polyline
                    .iter()
                    .all(|q| q.is_finite() && q.distance(p) <= 1e-9)
                {
                    self.points.push(p);
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        }
    }

    /// Points whose hull holds `c + a cos t + b sin t` for `t` between `t0`
    /// and `t1` (either order; at most a full turn). Returns the most a
    /// point lies outside the arc.
    fn arc(&mut self, c: Point3, a: Vec3, b: Vec3, t0: f64, t1: f64, target: f64) -> f64 {
        let at = |t: f64| c + a * t.cos() + b * t.sin();
        let lo = t0.min(t1);
        let span = (t0.max(t1) - lo).min(TAU);
        let hi = lo + span;
        self.points.push(at(lo));
        self.points.push(at(hi));
        let scale = a.length().max(b.length());
        if scale.is_nan() || scale <= 0.0 || span.is_nan() || span <= 0.0 {
            return 0.0;
        }
        // The largest sub-arc whose corners stay within `target`: 1 / cos(d
        // / 2) - 1 <= target / scale. At most a quarter turn, so the corners
        // stay near the arc and the tangents meet.
        let step =
            (2.0 * (scale / (scale + target)).clamp(-1.0, 1.0).acos()).clamp(1e-6, FRAC_PI_2);
        let count = (span / step).ceil().clamp(1.0, 1e6);
        let d = span / count;
        let k = 1.0 / (0.5 * d).cos();
        let mut excess = 0.0f64;
        let steps = count as usize;
        for i in 0..steps {
            let m = lo + (i as f64 + 0.5) * d;
            let dir = a * m.cos() + b * m.sin();
            self.points.push(c + dir * k);
            excess = excess.max((k - 1.0) * dir.length());
        }
        excess
    }

    /// A face of revolution about `frame`'s `z`: its meridian, in (radius,
    /// height) about the axis, is bounded by `meridian`; each corner is swept
    /// over the face's angles as an arc.
    fn revolved(
        &mut self,
        n: u32,
        frame: &Frame,
        finite_box: bool,
        [u0, u1, _, _]: [f64; 4],
        meridian: impl FnOnce(&mut Meridian),
    ) -> Result<()> {
        if !finite_frame(frame) {
            return Err(bad(n, "the surface's frame is not finite"));
        }
        if !finite_box {
            return Err(bad(n, "the parameter box is not finite"));
        }
        let mut m = Meridian::default();
        meridian(&mut m);
        if m.corners
            .iter()
            .any(|c| !c.0[0].is_finite() || !c.0[1].is_finite())
        {
            return Err(bad(n, "the surface's meridian is not finite"));
        }
        for ([rho, h], base) in m.corners {
            let ring = self.arc(
                frame.origin + frame.z * h,
                frame.x * rho,
                frame.y * rho,
                u0,
                u1,
                0.5 * ANALYTIC_EXCESS,
            );
            self.analytic = self.analytic.max(base + ring);
        }
        Ok(())
    }

    fn patch(&mut self, n: u32, p: &BSplinePatch, uv: [f64; 4]) -> Result<()> {
        if p.weights.iter().any(|w| !(w.is_finite() && *w > 0.0)) {
            return Err(bad(
                n,
                "a weight of its B-spline is not positive, so its control points do not bound it",
            ));
        }
        let (pu, pv) = (p.degree_u as usize, p.degree_v as usize);
        let (nu, nv) = (p.poles_u as usize, p.poles_v as usize);
        let nets = bezier_nets(
            (pu, pv),
            (nu, nv),
            (&p.knots_u, &p.knots_v),
            &p.poles,
            &p.weights,
            uv,
        )
        .ok_or_else(|| bad(n, "its B-spline could not be split into bounded pieces"))?;
        // Worst first: the piece furthest from its surface is split next, so
        // a work limit leaves the pieces evenly refined. Ties go by order of
        // creation, so the result is the same every time.
        let mut heap = std::collections::BinaryHeap::new();
        let mut order = 0usize;
        for net in nets {
            let excess = net_excess(&net);
            if !excess.is_finite() {
                return Err(bad(n, "its B-spline has a point that is not finite"));
            }
            heap.push(Piece { excess, order, net });
            order += 1;
        }
        while heap.len() < SPLINE_WORK
            && heap.peek().is_some_and(|p| p.excess > SPLINE_TARGET)
            && let Some(Piece { net, .. }) = heap.pop()
        {
            let (a, b) = net.split(net.longer_in_u());
            for net in [a, b] {
                let excess = net_excess(&net);
                if !excess.is_finite() {
                    return Err(bad(n, "its B-spline has a point that is not finite"));
                }
                heap.push(Piece { excess, order, net });
                order += 1;
            }
        }
        let mut pieces: Vec<Piece> = heap.into_vec();
        pieces.sort_by_key(|p| p.order);
        let worst = pieces.iter().map(|p| p.excess).fold(0.0f64, f64::max);
        for p in pieces {
            self.points.extend(p.net.pts.iter().map(|&h| project(h)));
        }
        self.spline = Some(self.spline.unwrap_or(0.0).max(worst));
        Ok(())
    }

    /// Bounds a B-spline edge by its refined control points; `false` when
    /// its knots do not give clean pieces.
    fn curve(&mut self, face: u32, c: &BSplineCurve) -> Result<bool> {
        if c.validate().is_err() {
            return Ok(false);
        }
        if c.weights.iter().any(|w| w.is_nan() || *w <= 0.0) {
            return Err(bad(
                face,
                "a weight of an edge's B-spline is not positive, so its control points do not \
                 bound it",
            ));
        }
        let hom: Vec<Hom> = c
            .poles
            .iter()
            .zip(&c.weights)
            .map(|(q, &w)| [q.x * w, q.y * w, q.z * w, w])
            .collect();
        let (lo, hi) = (c.first.min(c.last), c.first.max(c.last));
        let degree = c.degree as usize;
        let mut queue: Vec<(f64, f64, Vec<Hom>)> = pieces(&c.knots, degree, &hom, &[lo, hi])
            .into_iter()
            .filter(|(t0, t1, _)| *t1 > lo && *t0 < hi)
            .collect();
        if queue.is_empty() {
            return Ok(false);
        }
        queue.reverse();
        let mut made = queue.len();
        let mut worst = 0.0f64;
        while let Some((t0, t1, pts)) = queue.pop() {
            let excess = curve_excess(&pts);
            if excess > SPLINE_TARGET && made < SPLINE_WORK {
                let (a, b) = halve(&pts);
                let m = 0.5 * (t0 + t1);
                made += 1;
                queue.push((m, t1, b));
                queue.push((t0, m, a));
                continue;
            }
            worst = worst.max(excess);
            self.points.extend(pts.iter().map(|&h| project(h)));
        }
        self.spline = Some(self.spline.unwrap_or(0.0).max(worst));
        Ok(true)
    }
}

/// A Bezier piece of a B-spline face waiting to be split, largest excess
/// first.
struct Piece {
    excess: f64,
    order: usize,
    net: Net,
}

impl PartialEq for Piece {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for Piece {}

impl PartialOrd for Piece {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Piece {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.excess
            .total_cmp(&other.excess)
            .then_with(|| other.order.cmp(&self.order))
    }
}

/// The corners of a polygon in (radius, height) about an axis that holds a
/// face's meridian over its parameter box, each with how far it lies from
/// the meridian.
#[derive(Default)]
struct Meridian {
    corners: Vec<([f64; 2], f64)>,
}

impl Meridian {
    fn segment(&mut self, a: [f64; 2], b: [f64; 2]) {
        self.corners.push((a, 0.0));
        self.corners.push((b, 0.0));
    }

    /// The arc of `radius` about `centre` from angle `v0` to `v1`, bounded
    /// as [`Builder::arc`] bounds an arc, within half the analytic excess.
    fn arc(&mut self, centre: [f64; 2], radius: f64, v0: f64, v1: f64) {
        let mut b = Builder::default();
        let c = Point3::new(centre[0], centre[1], 0.0);
        let a = Vec3::new(radius, 0.0, 0.0);
        let e = Vec3::new(0.0, radius, 0.0);
        let excess = b.arc(c, a, e, v0, v1, 0.5 * ANALYTIC_EXCESS);
        for (i, p) in b.points.iter().enumerate() {
            // The two end points lie on the arc; the corners at most
            // `excess` from it.
            let e = if i < 2 { 0.0 } else { excess };
            self.corners.push(([p.x, p.y], e));
        }
    }
}

/// The value of a Bezier curve given by homogeneous points at `t`.
fn blend(pts: &[Hom], t: f64) -> Hom {
    let mut w = pts.to_vec();
    let n = w.len();
    for level in 1..n {
        for k in 0..n - level {
            if let (Some(&a), Some(&b)) = (w.get(k), w.get(k + 1))
                && let Some(x) = w.get_mut(k)
            {
                *x = lerp(a, b, t);
            }
        }
    }
    w.first().copied().unwrap_or([f64::NAN; 4])
}

/// The two halves of a Bezier curve at its middle.
fn halve(pts: &[Hom]) -> (Vec<Hom>, Vec<Hom>) {
    let mut work = pts.to_vec();
    let n = work.len();
    let mut lo = Vec::with_capacity(n);
    let mut hi = Vec::with_capacity(n);
    for level in 0..n {
        if let (Some(&first), Some(&last)) = (work.first(), work.get(n - 1 - level)) {
            lo.push(first);
            hi.push(last);
        }
        for k in 0..n - 1 - level {
            if let (Some(&a), Some(&b)) = (work.get(k), work.get(k + 1))
                && let Some(x) = work.get_mut(k)
            {
                *x = lerp(a, b, 0.5);
            }
        }
    }
    hi.reverse();
    (lo, hi)
}

/// The largest distance from a Bezier curve's control point `i` to the
/// curve at `i / degree` (its Greville parameter).
fn curve_excess(pts: &[Hom]) -> f64 {
    let degree = pts.len().saturating_sub(1).max(1) as f64;
    pts.iter()
        .enumerate()
        .map(|(i, &h)| project(h).distance(project(blend(pts, i as f64 / degree))))
        .fold(0.0, f64::max)
}

/// [`curve_excess`] for a Bezier patch.
fn net_excess(net: &Net) -> f64 {
    let (du, dv) = (net.du, net.dv);
    let row = |i: usize| -> Vec<Hom> {
        (0..=dv)
            .map(|j| {
                net.pts
                    .get(i * (dv + 1) + j)
                    .copied()
                    .unwrap_or([f64::NAN; 4])
            })
            .collect()
    };
    let rows: Vec<Vec<Hom>> = (0..=du).map(row).collect();
    let (fu, fv) = (du.max(1) as f64, dv.max(1) as f64);
    let mut worst = 0.0f64;
    for j in 0..=dv {
        let t = j as f64 / fv;
        let column: Vec<Hom> = rows.iter().map(|r| blend(r, t)).collect();
        for i in 0..=du {
            let s = blend(&column, i as f64 / fu);
            let pole = net
                .pts
                .get(i * (dv + 1) + j)
                .copied()
                .unwrap_or([f64::NAN; 4]);
            let d = project(pole).distance(project(s));
            worst = if d.is_nan() { f64::NAN } else { worst.max(d) };
            if worst.is_nan() {
                return worst;
            }
        }
    }
    worst
}
