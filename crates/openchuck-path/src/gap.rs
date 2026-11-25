// SPDX-License-Identifier: AGPL-3.0-or-later
//! The tool gap: the smallest signed distance between a tool and a surface
//! (OP-12; decision record SIMUL-EDGE-1, Q2).
//!
//! The tool is its cutting part, the profile revolved about its axis and
//! capped flat at the profile's top. Every profile the tool library makes
//! (flat, ball, bull nose, tapered ball) gives a convex solid; a profile that
//! is not convex is replaced by the cylinder of its cutting radius from the
//! tip to the top, which holds it, so the answer is never less safe. The
//! tool may also be swept along a short straight move without turning: the
//! gap is then the least over every pose on the move.
//!
//! The answer is an interval. `upper` is the signed distance of the tool, at
//! one pose of the move, from a surface point it found: a real value, so the
//! true gap is no larger. `lower` is proven: no surface point lies deeper
//! inside the tool at any pose. Negative means the surface lies inside the
//! tool by that much (the overcut depth).
//!
//! **Why it is proven.** For a convex solid `K` and any unit direction `u`,
//! every point `y` has a signed distance to `K` of at least
//! `u . y - h(u)`, where `h` is `K`'s support function (the plane square to
//! `u` touching `K` holds it on one side). Over the sweep the support grows by
//! at most `max(0, u . v)`. So over a triangle (or a piece of one) the gap is
//! at least the least of that bound at its corners, the bound being linear.
//! The search takes `u` from the nearest point of the tool to the piece's
//! centre and splits the piece with the smallest bound, smallest first, until
//! the smallest bound left is within [`GAP_ACCURACY`] of the best value found
//! (decision record 0255 item 1 makes the same argument for the independent
//! checker in `openchuck_testkit::contact`, which this code does not share).
//!
//! **Determinism (NFR-06).** One query runs on one thread. Triangles are
//! taken in index order, and pieces leave the queue by their bound, then by
//! triangle index, then by the order they were made, so the same query gives
//! the same answer, bit for bit, at any thread count.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use openchuck_types::geometry::{Aabb, Point3, UnitVec3, Vec3};
use openchuck_types::mesh::TriangleMesh;
use openchuck_types::tool::profile::{ProfilePiece, ProfilePoint, RevolvedProfile};

use crate::cutter::ToolLine;

/// How close, in millimetres, the proven `lower` is to the found `upper`
/// when a search finishes within its budget: a tenth of a micrometre, far
/// below any tolerance a toolpath uses and far above rounding.
pub const GAP_ACCURACY: f64 = 1e-4;

/// The most pieces one query splits before it answers with the interval it
/// has reached.
const MAX_PIECES: usize = 200_000;

/// A piece shorter than this along every side is not split again, mm.
const MIN_PIECE: f64 = 1e-9;

/// Steps of the golden-section search for the pose along a sweep. Each step
/// keeps 0.618 of the interval, so 40 steps leave about 4e-9 of the sweep:
/// the pose only sets `upper` and the direction of the bound, never `lower`'s
/// validity.
const SWEEP_STEPS: u32 = 40;

/// What a tool gap query asks (decision record SIMUL-EDGE-1, Q2): see
/// [`SurfaceSource::tool_gap`](crate::SurfaceSource::tool_gap).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GapQuery {
    /// The straight move the tool is swept along without turning, from the
    /// pose its line gives; zero for one pose.
    pub sweep: Vec3,
    /// How far from the tool the surface is looked for, mm: a gap larger
    /// than this gives no answer, and the search stops once it finds the
    /// surface deeper than this inside the tool.
    pub limit: f64,
    /// When set, the search stops as soon as the gap is proven at least this
    /// (mm; negative for an overcut depth a caller accepts), with whatever
    /// interval it has: for a caller that needs only to know whether the
    /// gap is above a limit. When not set, the interval is narrowed to
    /// [`GAP_ACCURACY`].
    pub accept: Option<f64>,
}

impl GapQuery {
    /// One pose, the surface looked for within `limit`.
    pub fn pose(limit: f64) -> Self {
        Self {
            sweep: Vec3::ZERO,
            limit,
            accept: None,
        }
    }

    /// The tool swept along `sweep`, the surface looked for within `limit`.
    pub fn swept(sweep: Vec3, limit: f64) -> Self {
        Self {
            sweep,
            limit,
            accept: None,
        }
    }

    /// This query, ending as soon as the gap is proven at least `gap`.
    #[must_use]
    pub fn accept_above(self, gap: f64) -> Self {
        Self {
            accept: Some(gap),
            ..self
        }
    }
}

/// The smallest signed distance between a tool and a surface, as an interval
/// (decision record SIMUL-EDGE-1, Q2): see [`SurfaceSource::tool_gap`].
///
/// [`SurfaceSource::tool_gap`]: crate::SurfaceSource::tool_gap
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolGap {
    /// Proven: no surface point is nearer the tool than this, or deeper
    /// inside it, at any pose. Negative when the surface may lie inside the
    /// tool by up to that much.
    pub lower: f64,
    /// Found: the signed distance from the tool, at one pose, of the surface
    /// point [`at`](Self::at). The true gap lies between `lower` and this.
    pub upper: f64,
    /// The surface point where `upper` was found.
    pub at: Point3,
}

impl ToolGap {
    /// The deeper of two answers, for a source made of several surfaces:
    /// the smaller `lower` and the smaller `upper` with its point. Ties keep
    /// `self`, so the order the parts are asked in decides them (NFR-06).
    #[must_use]
    pub fn min(self, other: Self) -> Self {
        let lower = self.lower.min(other.lower);
        if other.upper < self.upper {
            Self { lower, ..other }
        } else {
            Self { lower, ..self }
        }
    }
}

/// The cutting part's outline in the tool's half-plane (radius, height from
/// the tip), from the tip round to the top of the axis: the profile's pieces
/// and the cap.
#[derive(Debug, Clone)]
pub(crate) struct Outline {
    pieces: Vec<ProfilePiece>,
    radius: f64,
    top: f64,
    /// Whether the outline is the profile's own (convex) or its bounding
    /// cylinder.
    own: bool,
    lower: Vec<ProfilePiece>,
}

impl Outline {
    pub(crate) fn new(profile: &RevolvedProfile) -> Self {
        let radius = profile.cutting_radius();
        let top = profile.top();
        let tip = ProfilePoint::TIP;
        if profile.is_convex() {
            let mut pieces = profile.pieces().to_vec();
            let end = profile.end();
            if pr(end) > 0.0 {
                pieces.push(ProfilePiece::Line {
                    start: end,
                    end: ProfilePoint::new(0.0, top),
                });
            }
            let lower = profile.pieces().to_vec();
            return Self {
                pieces,
                radius,
                top,
                own: true,
                lower,
            };
        }
        let corner = ProfilePoint::new(radius, 0.0);
        let rim = ProfilePoint::new(radius, top);
        let lower = vec![
            ProfilePiece::Line {
                start: tip,
                end: corner,
            },
            ProfilePiece::Line {
                start: corner,
                end: rim,
            },
        ];
        let mut pieces = lower.clone();
        pieces.push(ProfilePiece::Line {
            start: rim,
            end: ProfilePoint::new(0.0, top),
        });
        Self {
            pieces,
            radius,
            top,
            own: false,
            lower,
        }
    }

    /// Whether `(r, z)`, with `r >= 0`, lies inside the outline.
    fn contains(&self, r: f64, z: f64) -> bool {
        if !(0.0..=self.top).contains(&z) || r > self.radius {
            return false;
        }
        if !self.own {
            return true;
        }
        // The lowest point of the tool at radius r: the outline is convex
        // and monotone, so a point is inside when it is on or above it.
        let mut lowest = 0.0;
        for p in &self.lower {
            let (s, e) = (p.start(), p.end());
            if pr(e) - pr(s) > 1e-12 && r <= pr(e) {
                lowest = p.height_at(r.max(pr(s)));
                return z >= lowest;
            }
            lowest = pz(e);
        }
        z >= lowest
    }

    /// The signed distance of `(r, z)` (with `r >= 0`) from the outline and
    /// the outward unit normal (radial, axial) of the boundary at its nearest
    /// point: negative inside.
    fn signed(&self, r: f64, z: f64) -> (f64, (f64, f64)) {
        let mut best = f64::INFINITY;
        let mut near = (r, z);
        let mut piece_normal = (0.0, -1.0);
        for p in &self.pieces {
            let (q, n) = nearest_on_piece(p, r, z);
            let d = (r - q.0).hypot(z - q.1);
            if d < best {
                best = d;
                near = q;
                piece_normal = n;
            }
        }
        let inside = self.contains(r, z);
        let (dr, dz) = (r - near.0, z - near.1);
        let len = dr.hypot(dz);
        let normal = if len > 1e-12 {
            if inside {
                (-dr / len, -dz / len)
            } else {
                (dr / len, dz / len)
            }
        } else {
            piece_normal
        };
        (if inside { -best } else { best }, normal)
    }

    /// The support of the outline (mirrored about the axis) in the
    /// half-plane direction `(a, b)` with `a >= 0`: the largest
    /// `a * r + b * z` over the outline.
    fn support(&self, a: f64, b: f64) -> f64 {
        let mut best = 0.0_f64; // the tip, (0, 0)
        for p in &self.pieces {
            for q in [p.start(), p.end()] {
                best = best.max(a * pr(q) + b * pz(q));
            }
            if let ProfilePiece::Arc {
                start,
                end,
                centre,
                radius,
                convex: true,
            } = *p
            {
                let len = a.hypot(b);
                if len > 0.0 {
                    let angle = b.atan2(a);
                    let (ta, tb) = arc_range(start, end, centre);
                    if angle >= ta && angle <= tb {
                        best = best.max(a * pr(centre) + b * pz(centre) + radius * len);
                    }
                }
            }
        }
        best
    }
}

/// The angles of a convex arc's ends about its centre, lower first.
fn arc_range(start: ProfilePoint, end: ProfilePoint, centre: ProfilePoint) -> (f64, f64) {
    let ta = (pz(start) - pz(centre)).atan2(pr(start) - pr(centre));
    let tb = (pz(end) - pz(centre)).atan2(pr(end) - pr(centre));
    (ta.min(tb), ta.max(tb))
}

/// The nearest point of a piece to `(r, z)` and the piece's outward normal
/// there.
fn nearest_on_piece(p: &ProfilePiece, r: f64, z: f64) -> ((f64, f64), (f64, f64)) {
    match *p {
        ProfilePiece::Line { start, end } => {
            let (dr, dz) = (pr(end) - pr(start), pz(end) - pz(start));
            let len2 = dr * dr + dz * dz;
            let t = if len2 > 0.0 {
                (((r - pr(start)) * dr + (z - pz(start)) * dz) / len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let len = len2.sqrt();
            let normal = if len > 0.0 {
                (dz / len, -dr / len)
            } else {
                (0.0, -1.0)
            };
            ((pr(start) + t * dr, pz(start) + t * dz), normal)
        }
        ProfilePiece::Arc {
            start,
            end,
            centre,
            radius,
            convex,
        } => {
            let (ta, tb) = if convex {
                arc_range(start, end, centre)
            } else {
                // Not reached: a profile with a concave arc is not convex and
                // is replaced by its bounding cylinder. Kept as a chord.
                let q = ((pr(start) + pr(end)) * 0.5, (pz(start) + pz(end)) * 0.5);
                return (q, (0.0, -1.0));
            };
            let angle = (z - pz(centre)).atan2(r - pr(centre)).clamp(ta, tb);
            let (c, s) = (angle.cos(), angle.sin());
            ((pr(centre) + radius * c, pz(centre) + radius * s), (c, s))
        }
    }
}

/// The tool at its first pose and the straight move it is swept along.
#[derive(Debug, Clone)]
pub(crate) struct Body<'a> {
    outline: &'a Outline,
    tip: Point3,
    axis: Vec3,
    sweep: Vec3,
    /// A direction square to the axis, for a point on the axis.
    across: Vec3,
}

impl<'a> Body<'a> {
    pub(crate) fn new(outline: &'a Outline, line: &ToolLine, sweep: Vec3) -> Self {
        let axis = line.axis.get();
        let pick = if axis.x.abs() < 0.9 {
            Vec3::new(1.0, 0.0, 0.0)
        } else {
            Vec3::new(0.0, 1.0, 0.0)
        };
        let across = UnitVec3::new(pick - axis * pick.dot(axis))
            .map_or(Vec3::new(0.0, 0.0, 1.0), UnitVec3::get);
        Self {
            outline,
            tip: line.origin,
            axis,
            sweep: if sweep.is_finite() { sweep } else { Vec3::ZERO },
            across,
        }
    }

    /// A box holding every pose of the tool, grown by `margin`.
    pub(crate) fn bounds(&self, margin: f64) -> Aabb {
        let ends = [
            self.tip,
            self.tip + self.axis * self.outline.top,
            self.tip + self.sweep,
            self.tip + self.sweep + self.axis * self.outline.top,
        ];
        let grow = self.outline.radius + margin.max(0.0);
        let mut min = ends[0];
        let mut max = ends[0];
        for p in ends {
            min = Point3::new(min.x.min(p.x), min.y.min(p.y), min.z.min(p.z));
            max = Point3::new(max.x.max(p.x), max.y.max(p.y), max.z.max(p.z));
        }
        Aabb {
            min: Point3::new(min.x - grow, min.y - grow, min.z - grow),
            max: Point3::new(max.x + grow, max.y + grow, max.z + grow),
        }
    }

    /// The signed distance of `x` from the tool at the pose `s` along the
    /// sweep (0 to 1), and the outward unit normal of the tool there.
    fn at_pose(&self, x: Point3, s: f64) -> (f64, Vec3) {
        let d = x - (self.tip + self.sweep * s);
        let z = d.dot(self.axis);
        let radial = d - self.axis * z;
        let r = radial.length();
        let w = if r > 1e-12 {
            radial * (1.0 / r)
        } else {
            self.across
        };
        let (dist, (nr, nz)) = self.outline.signed(r, z);
        (dist, w * nr + self.axis * nz)
    }

    /// The least signed distance of `x` from the tool over the sweep, as
    /// found (an upper bound of the true least), the normal there, and how
    /// far above the true least it may be: 0 for one pose; for a sweep, the
    /// sweep's length times the width of the search's last interval, since
    /// the distance is convex along the sweep (so the least lies in that
    /// interval) and changes no faster than the sweep's length per unit.
    fn upper(&self, x: Point3) -> (f64, Vec3, f64) {
        let length = self.sweep.length();
        if length <= 0.0 {
            let (d, n) = self.at_pose(x, 0.0);
            return (d, n, 0.0);
        }
        // The distance is convex along the sweep: golden-section search.
        let g = 0.5 * (5.0_f64.sqrt() - 1.0);
        let (mut a, mut b) = (0.0_f64, 1.0_f64);
        let mut c = b - g * (b - a);
        let mut d = a + g * (b - a);
        let mut fc = self.at_pose(x, c);
        let mut fd = self.at_pose(x, d);
        for _ in 0..SWEEP_STEPS {
            if fc.0 <= fd.0 {
                b = d;
                d = c;
                fd = fc;
                c = b - g * (b - a);
                fc = self.at_pose(x, c);
            } else {
                a = c;
                c = d;
                fc = fd;
                d = a + g * (b - a);
                fd = self.at_pose(x, d);
            }
        }
        let mut best = if fc.0 <= fd.0 { fc } else { fd };
        for end in [self.at_pose(x, 0.0), self.at_pose(x, 1.0)] {
            if end.0 < best.0 {
                best = end;
            }
        }
        (best.0, best.1, length * (b - a))
    }

    /// The proven least signed distance over the triangle `t` from the tool
    /// at every pose, by the plane square to `u`.
    fn lower(&self, t: &[Point3; 3], u: Vec3) -> f64 {
        let a = self.axis.dot(u);
        let across = (u - self.axis * a).length();
        let h = u.dot(self.tip - Point3::ORIGIN) + self.outline.support(across, a);
        let grow = u.dot(self.sweep).max(0.0);
        t.iter()
            .map(|p| u.dot(*p - Point3::ORIGIN))
            .fold(f64::INFINITY, f64::min)
            - h
            - grow
    }
}

/// One piece of a triangle in the search queue.
#[derive(Debug, Clone, Copy)]
struct Piece {
    lower: f64,
    triangle: u32,
    order: u64,
    corners: [Point3; 3],
}

impl PartialEq for Piece {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Piece {}

impl PartialOrd for Piece {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Piece {
    // A max-heap: the smallest bound comes out first, then the lowest
    // triangle, then the earliest made.
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .lower
            .total_cmp(&self.lower)
            .then_with(|| other.triangle.cmp(&self.triangle))
            .then_with(|| other.order.cmp(&self.order))
    }
}

/// The tool gap over `triangles` (index and corners, in index order), within
/// `limit`: `None` when no surface lies within `limit` of the tool. The
/// search stops early once it finds the surface deeper than `limit` inside
/// the tool; the interval then holds the true gap but may be wider than
/// [`GAP_ACCURACY`].
pub(crate) fn search(
    body: &Body<'_>,
    triangles: impl IntoIterator<Item = (u32, [Point3; 3])>,
    limit: f64,
    accept: Option<f64>,
) -> Option<ToolGap> {
    let accept = accept.filter(|a| a.is_finite()).unwrap_or(f64::INFINITY);
    let mut state = Search {
        body,
        queue: BinaryHeap::new(),
        order: 0,
        best: None,
    };
    for (i, t) in triangles {
        if t.iter().all(|p| p.is_finite()) {
            state.push(t, i);
        }
    }
    let mut pieces = 0_usize;
    let lower = loop {
        let found = state.best.map_or(f64::INFINITY, |b| b.0);
        let Some(piece) = state.queue.pop() else {
            break found;
        };
        if piece.lower >= found - GAP_ACCURACY
            || piece.lower >= accept
            || piece.lower > limit
            || found < -limit
        {
            break piece.lower.min(found);
        }
        let (a, b, c, len) = longest_side(&piece.corners);
        if len < MIN_PIECE || pieces >= MAX_PIECES {
            break piece.lower.min(found);
        }
        pieces += 2;
        let mid = midpoint(a, b);
        state.push([a, mid, c], piece.triangle);
        state.push([mid, b, c], piece.triangle);
    };
    let (upper, at) = state.best?;
    (lower <= limit).then_some(ToolGap {
        lower: lower.min(upper),
        upper,
        at,
    })
}

/// The state of one search: the queue of pieces and the best value found.
struct Search<'a, 'b> {
    body: &'a Body<'b>,
    queue: BinaryHeap<Piece>,
    order: u64,
    best: Option<(f64, Point3)>,
}

impl Search<'_, '_> {
    /// Bounds a piece, notes its centre's value, and queues it. The bound is
    /// the better of two proven ones: the plane square to the normal at the
    /// centre's nearest tool point, and the centre's least distance less the
    /// piece's reach from it (the distance changes no faster than the point
    /// moves) and less the search's own slack.
    fn push(&mut self, corners: [Point3; 3], triangle: u32) {
        let centre = centroid(&corners);
        let (value, normal, slack) = self.body.upper(centre);
        if self.best.is_none_or(|(b, _)| value < b) {
            self.best = Some((value, centre));
        }
        let reach = corners
            .iter()
            .map(|p| p.distance(centre))
            .fold(0.0, f64::max);
        let mut lower = self.body.lower(&corners, normal).max(value - slack - reach);
        // The piece lies in its triangle's plane: the gap is at least the
        // gap between the tool and that plane, from either side. Exact at
        // once where the tool rests on the triangle's face.
        let (a, b, c) = (corners[0], corners[1], corners[2]);
        if let Ok(m) = UnitVec3::new((b - a).cross(c - a)) {
            let m = m.get();
            lower = lower
                .max(self.body.lower(&corners, m))
                .max(self.body.lower(&corners, -m));
        }
        self.order += 1;
        self.queue.push(Piece {
            lower,
            triangle,
            order: self.order,
            corners,
        });
    }
}

fn centroid(t: &[Point3; 3]) -> Point3 {
    Point3::new(
        (t[0].x + t[1].x + t[2].x) / 3.0,
        (t[0].y + t[1].y + t[2].y) / 3.0,
        (t[0].z + t[1].z + t[2].z) / 3.0,
    )
}

fn midpoint(a: Point3, b: Point3) -> Point3 {
    Point3::new(0.5 * (a.x + b.x), 0.5 * (a.y + b.y), 0.5 * (a.z + b.z))
}

/// The corners of `t` reordered so that `a` to `b` is its longest side, with
/// that side's length.
fn longest_side(t: &[Point3; 3]) -> (Point3, Point3, Point3, f64) {
    let [p, q, r] = *t;
    let sides = [(p, q, r), (q, r, p), (r, p, q)];
    let mut best = sides[0];
    let mut len = p.distance(q);
    for s in &sides[1..] {
        let l = s.0.distance(s.1);
        if l > len {
            len = l;
            best = *s;
        }
    }
    (best.0, best.1, best.2, len)
}

/// Whether two boxes overlap.
pub(crate) fn overlaps(a: &Aabb, b: &Aabb) -> bool {
    a.min.x <= b.max.x
        && b.min.x <= a.max.x
        && a.min.y <= b.max.y
        && b.min.y <= a.max.y
        && a.min.z <= b.max.z
        && b.min.z <= a.max.z
}

/// The tool gap over every triangle of `mesh`, without a search structure:
/// the answer of [`SurfaceSource::tool_gap`](crate::SurfaceSource::tool_gap)
/// for a source that has none of its own.
pub(crate) fn on_mesh(
    mesh: &TriangleMesh,
    profile: &RevolvedProfile,
    line: &ToolLine,
    query: &GapQuery,
) -> Option<ToolGap> {
    if !line.origin.is_finite() || !query.limit.is_finite() {
        return None;
    }
    let outline = Outline::new(profile);
    let body = Body::new(&outline, line, query.sweep);
    let near = body.bounds(query.limit);
    let triangles = (0..mesh.triangles.len()).filter_map(|i| {
        let t = mesh.triangle(i)?;
        let index = u32::try_from(i).ok()?;
        overlaps(&near, &triangle_bounds(&t)).then_some((index, t))
    });
    search(&body, triangles, query.limit, query.accept)
}

/// The box of a triangle.
pub(crate) fn triangle_bounds(t: &[Point3; 3]) -> Aabb {
    Aabb {
        min: Point3::new(
            t[0].x.min(t[1].x).min(t[2].x),
            t[0].y.min(t[1].y).min(t[2].y),
            t[0].z.min(t[1].z).min(t[2].z),
        ),
        max: Point3::new(
            t[0].x.max(t[1].x).max(t[2].x),
            t[0].y.max(t[1].y).max(t[2].y),
            t[0].z.max(t[1].z).max(t[2].z),
        ),
    }
}

/// A profile point's radius.
fn pr(p: ProfilePoint) -> f64 {
    p.radius.0
}

/// A profile point's height.
fn pz(p: ProfilePoint) -> f64 {
    p.height.0
}
