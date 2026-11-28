// SPDX-License-Identifier: AGPL-3.0-or-later
//! Face-flow finishing (OP-28; decision record 0525; lane FACEFLOW).
//!
//! The parts are built by hand as B-rep models with their edges and
//! vertices, so no STEP kernel is involved; the STEP blisk is the
//! application's reference job. Each check is made against the closed form
//! of the part (the plane, the bilinear patch, the cylinder), never with the
//! pathing library's own evaluators.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::indexing_slicing,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::too_many_lines
)]

mod support;

use std::collections::BTreeMap;
use std::f64::consts::{FRAC_PI_3, PI};
use std::sync::Arc;

use openchuck_path::face_flow::{self, FaceFlow, parameter as p};
use openchuck_path::params::names;
use openchuck_path::{
    AnalyticStock, CutPoint, Pass, RegionConstraint, RotaryMapping, Strategy, StrategyInput,
    StrategyOutput, UnreachedReason,
};
use openchuck_types::brep::{
    BSplinePatch, BrepEdge, BrepFace, BrepModel, BrepVertex, EdgeCurve, EdgeId, FaceId, Frame,
    Surface, TrimLoop, UvPoint, VertexId,
};
use openchuck_types::geometry::{Point3, Vec3};
use openchuck_types::machine::Clearance;
use openchuck_types::mesh::TriangleMesh;
use openchuck_types::tool::Tool;
use openchuck_types::units::Millimetres;
use support::{ball, machine, run, tags};

// ---- values -------------------------------------------------------------------

fn num(x: f64) -> openchuck_path::ParameterValue {
    openchuck_path::ParameterValue::Number(x)
}

fn text(s: &str) -> openchuck_path::ParameterValue {
    openchuck_path::ParameterValue::Text(s.to_owned())
}

fn flag(b: bool) -> openchuck_path::ParameterValue {
    openchuck_path::ParameterValue::Flag(b)
}

/// The operation's values. The tests of the passes themselves (their
/// geometry, spacing and order) set Stay down between passes off, so each
/// pass is its own run; the stay-down tests turn it on.
fn values(
    stepover: f64,
    extra: &[(&str, openchuck_path::ParameterValue)],
) -> BTreeMap<String, openchuck_path::ParameterValue> {
    let mut v: BTreeMap<String, openchuck_path::ParameterValue> = [
        (names::STEPOVER, stepover),
        (names::FEED, 300.0),
        (names::PLUNGE_FEED, 100.0),
        (names::TOLERANCE, 0.005),
    ]
    .into_iter()
    .map(|(k, x)| (k.to_owned(), num(x)))
    .collect();
    v.insert(p::STAY_DOWN.to_owned(), flag(false));
    for (k, x) in extra {
        v.insert((*k).to_owned(), x.clone());
    }
    v
}

// ---- a hand-built B-rep with edges and vertices ----------------------------------

fn p3(x: f64, y: f64, z: f64) -> Point3 {
    Point3::new(x, y, z)
}

/// A model under construction: faces with their edges, edges with their
/// vertices and curves.
#[derive(Default)]
struct Builder {
    model: BrepModel,
}

impl Builder {
    fn vertex(&mut self, p: Point3) -> VertexId {
        if let Some(v) = self
            .model
            .vertices
            .iter()
            .find(|v| v.point.distance(p) < 1e-9)
        {
            return v.id;
        }
        let id = VertexId(self.model.vertices.len() as u32);
        self.model.vertices.push(BrepVertex { id, point: p });
        id
    }

    /// The edge along `curve` from t 0 to 1, sampled finely, between the
    /// faces `faces`; a straight edge gets its exact line.
    fn edge(&mut self, faces: &[u32], curve: &dyn Fn(f64) -> Point3, straight: bool) -> EdgeId {
        let n = if straight { 1 } else { 256 };
        let polyline: Vec<Point3> = (0..=n)
            .map(|i| curve(f64::from(i) / f64::from(n)))
            .collect();
        let (a, b) = (polyline[0], polyline[polyline.len() - 1]);
        let id = EdgeId(self.model.edges.len() as u32);
        let vertices = Some([self.vertex(a), self.vertex(b)]);
        let exact = straight.then(|| {
            let d = b - a;
            let len = d.length();
            EdgeCurve::Line {
                origin: a,
                direction: d * (1.0 / len),
                first: 0.0,
                last: len,
            }
        });
        self.model.edges.push(BrepEdge {
            id,
            faces: faces.iter().map(|f| FaceId(*f)).collect(),
            vertices,
            polyline,
            curve: exact,
        });
        id
    }

    /// Adds face `id` (its position in the model) over `uv`, with the trim
    /// loop `trim` in (u, v) when it is not the whole box, bounded by
    /// `edges`, tessellated from `eval` where `inside`.
    #[allow(clippy::too_many_arguments)]
    fn face(
        &mut self,
        geometry: Surface,
        uv: [f64; 4],
        trim: Option<Vec<(f64, f64)>>,
        edges: Vec<EdgeId>,
        eval: &dyn Fn(f64, f64) -> Point3,
        inside: &dyn Fn(f64, f64) -> bool,
        reversed: bool,
    ) -> FaceId {
        let id = FaceId(self.model.faces.len() as u32);
        let (nu, nv) = (64usize, 64usize);
        let mut tess = TriangleMesh::default();
        for i in 0..=nu {
            for j in 0..=nv {
                let u = uv[0] + (uv[1] - uv[0]) * i as f64 / nu as f64;
                let v = uv[2] + (uv[3] - uv[2]) * j as f64 / nv as f64;
                tess.vertices.push(eval(u, v));
            }
        }
        let at = |i: usize, j: usize| u32::try_from(i * (nv + 1) + j).unwrap();
        for i in 0..nu {
            for j in 0..nv {
                let u = uv[0] + (uv[1] - uv[0]) * (i as f64 + 0.5) / nu as f64;
                let v = uv[2] + (uv[3] - uv[2]) * (j as f64 + 0.5) / nv as f64;
                if !inside(u, v) {
                    continue;
                }
                let (t1, t2) = (
                    [at(i, j), at(i + 1, j), at(i + 1, j + 1)],
                    [at(i, j), at(i + 1, j + 1), at(i, j + 1)],
                );
                if reversed {
                    tess.triangles.push([t1[0], t1[2], t1[1]]);
                    tess.triangles.push([t2[0], t2[2], t2[1]]);
                } else {
                    tess.triangles.push(t1);
                    tess.triangles.push(t2);
                }
            }
        }
        self.model.faces.push(BrepFace {
            id,
            surface: geometry.kind(),
            body: 0,
            edges,
            geometry,
            reversed,
            uv_bounds: uv,
            trims: trim
                .map(|t| {
                    vec![TrimLoop(
                        t.into_iter().map(|(u, v)| UvPoint { u, v }).collect(),
                    )]
                })
                .unwrap_or_default(),
            tessellation: tess,
        });
        id
    }
}

fn frame(o: Point3, x: Vec3, y: Vec3) -> Frame {
    Frame {
        origin: o,
        x,
        y,
        z: x.cross(y),
    }
}

/// A flat polygon at height `z`, facing +Z, with `corners` in (x, y)
/// counter-clockwise seen from above. The plane's (u, v) are X and Y. Its
/// edges are the polygon's sides in order; a side given in `shared` is
/// that existing edge, which this face then also bounds.
fn flat(b: &mut Builder, z: f64, corners: &[(f64, f64)], shared: &[Option<u32>]) -> FaceId {
    let id = b.model.faces.len() as u32;
    let mut edges = Vec::new();
    for (i, &(x0, y0)) in corners.iter().enumerate() {
        let (x1, y1) = corners[(i + 1) % corners.len()];
        if let Some(e) = shared.get(i).copied().flatten() {
            b.model.edges[e as usize].faces.push(FaceId(id));
            edges.push(EdgeId(e));
            continue;
        }
        edges.push(b.edge(
            &[id],
            &|t| p3(x0 + (x1 - x0) * t, y0 + (y1 - y0) * t, z),
            true,
        ));
    }
    let xs = corners.iter().map(|c| c.0);
    let ys = corners.iter().map(|c| c.1);
    let uv = [
        xs.clone().fold(f64::INFINITY, f64::min),
        xs.fold(f64::NEG_INFINITY, f64::max),
        ys.clone().fold(f64::INFINITY, f64::min),
        ys.fold(f64::NEG_INFINITY, f64::max),
    ];
    let polygon: Vec<(f64, f64)> = corners.to_vec();
    let inside = move |u: f64, v: f64| inside_polygon(&polygon, u, v);
    b.face(
        Surface::Plane {
            frame: frame(
                p3(0.0, 0.0, z),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ),
        },
        uv,
        Some(corners.to_vec()),
        edges,
        &|u, v| p3(u, v, z),
        &inside,
        false,
    )
}

fn inside_polygon(polygon: &[(f64, f64)], u: f64, v: f64) -> bool {
    let mut inside = false;
    for (i, &(x0, y0)) in polygon.iter().enumerate() {
        let (x1, y1) = polygon[(i + 1) % polygon.len()];
        if (y0 > v) != (y1 > v) && u < x0 + (v - y0) / (y1 - y0) * (x1 - x0) {
            inside = !inside;
        }
    }
    inside
}

/// A wall: the rectangle from `a` to `b` along the floor and `height` up
/// in +Z, facing `facing`, sharing its bottom edge `floor_edge` with the
/// floor. Its own top and end edges are added.
fn wall(
    b: &mut Builder,
    a: Point3,
    c: Point3,
    height: f64,
    facing: Vec3,
    floor_edge: EdgeId,
) -> FaceId {
    let id = b.model.faces.len() as u32;
    b.model.edges[floor_edge.0 as usize].faces.push(FaceId(id));
    let up = Vec3::new(0.0, 0.0, 1.0);
    let len = (c - a).length();
    let along = (c - a) * (1.0 / len);
    // u along the floor, v up: the normal is along x cross up, which must
    // be `facing`; otherwise run u the other way.
    let (origin, x) = if along.cross(up).dot(facing) > 0.0 {
        (a, along)
    } else {
        (c, -along)
    };
    let top = b.edge(&[id], &|t| origin + x * (len * t) + up * height, true);
    let e0 = b.edge(&[id], &|t| a + up * (height * t), true);
    let e1 = b.edge(&[id], &|t| c + up * (height * t), true);
    b.face(
        Surface::Plane {
            frame: frame(origin, x, up),
        },
        [0.0, len, 0.0, height],
        None,
        vec![floor_edge, top, e0, e1],
        &|u, v| origin + x * u + up * v,
        &|_, _| true,
        false,
    )
}

/// A twisted ruled face: the bilinear patch through four corners over X
/// from 0 to `length` and Y from -`half` to `half`, with heights `z` at
/// (0, -half), (length, -half), (0, half), (length, half). Every line of
/// constant X and of constant Y on it is straight. Returns the model and
/// the closed form height over (x, y).
fn twisted(length: f64, half: f64, z: [f64; 4]) -> (BrepModel, impl Fn(f64, f64) -> f64) {
    let height = move |x: f64, y: f64| {
        let (s, t) = (x / length, (y + half) / (2.0 * half));
        z[0] * (1.0 - s) * (1.0 - t) + z[1] * s * (1.0 - t) + z[2] * (1.0 - s) * t + z[3] * s * t
    };
    let point = move |u: f64, v: f64| {
        let (x, y) = (u * length, -half + 2.0 * half * v);
        p3(x, y, height(x, y))
    };
    let patch = BSplinePatch {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0.0, 0.0, 1.0, 1.0],
        knots_v: vec![0.0, 0.0, 1.0, 1.0],
        poles_u: 2,
        poles_v: 2,
        // v fastest: (u0 v0), (u0 v1), (u1 v0), (u1 v1).
        poles: vec![
            point(0.0, 0.0),
            point(0.0, 1.0),
            point(1.0, 0.0),
            point(1.0, 1.0),
        ],
        weights: vec![1.0; 4],
    };
    let mut b = Builder::default();
    let edges = vec![
        b.edge(&[0], &|t| point(t, 0.0), true),
        b.edge(&[0], &|t| point(1.0, t), true),
        b.edge(&[0], &|t| point(1.0 - t, 1.0), true),
        b.edge(&[0], &|t| point(0.0, 1.0 - t), true),
    ];
    b.face(
        Surface::BSpline(patch),
        [0.0, 1.0, 0.0, 1.0],
        None,
        edges,
        &point,
        &|_, _| true,
        false,
    );
    (b.model, height)
}

/// A patch of the cylinder of `radius` about the model X axis, round from
/// angle `from` to `to` (radians from +Y towards +Z) and along X from 0 to
/// `length`, facing away from the axis. Edges: the line at `from` (0), the
/// arc at X `length` (1), the line at `to` (2), the arc at X 0 (3).
fn cylinder_patch(radius: f64, from: f64, to: f64, length: f64) -> BrepModel {
    offset_cylinder(0.0, radius, from, to, length)
}

/// As [`cylinder_patch`], about the line parallel to X through (0, 0,
/// `centre`).
fn offset_cylinder(centre: f64, radius: f64, from: f64, to: f64, length: f64) -> BrepModel {
    let point = move |u: f64, v: f64| p3(v, radius * u.cos(), centre + radius * u.sin());
    let mut b = Builder::default();
    let edges = vec![
        b.edge(&[0], &|t| point(from, length * t), true),
        b.edge(&[0], &|t| point(from + (to - from) * t, length), false),
        b.edge(&[0], &|t| point(to, length * (1.0 - t)), true),
        b.edge(&[0], &|t| point(to - (to - from) * t, 0.0), false),
    ];
    b.face(
        Surface::Cylinder {
            frame: Frame {
                origin: p3(0.0, 0.0, centre),
                x: Vec3::new(0.0, 1.0, 0.0),
                y: Vec3::new(0.0, 0.0, 1.0),
                z: Vec3::new(1.0, 0.0, 0.0),
            },
            radius,
        },
        [from, to, 0.0, length],
        None,
        edges,
        &point,
        &|_, _| true,
        false,
    );
    b.model
}

/// A floor at Z 10 from X 0 to 30 and Y -`half` to `half`, between two
/// walls 6 mm high along its long sides. Face 0 is the floor; its edges are
/// 0 (Y -half), 1 (the end at X 30), 2 (Y half) and 3 (the end at X 0).
fn channel(half: f64) -> BrepModel {
    let mut b = Builder::default();
    flat(
        &mut b,
        10.0,
        &[(0.0, -half), (30.0, -half), (30.0, half), (0.0, half)],
        &[],
    );
    wall(
        &mut b,
        p3(0.0, -half, 10.0),
        p3(30.0, -half, 10.0),
        6.0,
        Vec3::new(0.0, 1.0, 0.0),
        EdgeId(0),
    );
    wall(
        &mut b,
        p3(30.0, half, 10.0),
        p3(0.0, half, 10.0),
        6.0,
        Vec3::new(0.0, -1.0, 0.0),
        EdgeId(2),
    );
    b.model
}

/// As [`channel`], with a third wall across the floor's end at X 30 (on its
/// edge 1), facing -X.
fn channel_with_end_wall(half: f64) -> BrepModel {
    let mut model = channel(half);
    let mut b = Builder { model };
    wall(
        &mut b,
        p3(30.0, -half, 10.0),
        p3(30.0, half, 10.0),
        6.0,
        Vec3::new(-1.0, 0.0, 0.0),
        EdgeId(1),
    );
    model = b.model;
    model
}

// ---- running the strategy ---------------------------------------------------------

struct Rig {
    model: Arc<BrepModel>,
    surface: openchuck_path::BrepSurface,
    stock: AnalyticStock,
    tool: openchuck_types::tool::profile::RevolvedProfile,
    rotary: RotaryMapping,
    tags: openchuck_types::ir::PathTags,
    machine: openchuck_types::machine::MachineProfile,
}

impl Rig {
    fn new(model: BrepModel, tool: &Tool) -> Self {
        model.validate().unwrap();
        let machine = machine();
        let surface = openchuck_path::BrepSurface::new(&model).unwrap();
        Self {
            model: Arc::new(model),
            surface,
            stock: AnalyticStock::Cylinder {
                radius: 25.0,
                x_start: -5.0,
                x_end: 50.0,
            },
            tool: tool.profile().unwrap(),
            rotary: RotaryMapping::of_profile(&machine),
            tags: tags(),
            machine,
        }
    }

    fn input(&self) -> StrategyInput<'_> {
        StrategyInput {
            surface: &self.surface,
            stock: &self.stock,
            collision: &openchuck_path::AcceptAll,
            tool: &self.tool,
            rotary: self.rotary,
            clearance: Clearance {
                radius: Millimetres(30.0),
                safe_z: Millimetres(40.0),
            },
            max_feeds: self.machine.max_feeds,
            tags: &self.tags,
            facing: None,
            keep_out: &[],
            shank: None,
        }
    }

    fn strategy(&self, faces: &[u32], guide: &[u32]) -> FaceFlow {
        FaceFlow::on_brep(
            Arc::clone(&self.model),
            faces.iter().map(|f| FaceId(*f)).collect(),
            guide.iter().map(|e| EdgeId(*e)).collect(),
        )
    }

    fn output(
        &self,
        strategy: &FaceFlow,
        v: &BTreeMap<String, openchuck_path::ParameterValue>,
    ) -> openchuck_path::Result<StrategyOutput> {
        let parameters = strategy.resolve_parameters(v)?;
        run(|ctx| strategy.passes_with_notes(&self.input(), None, &parameters, ctx))
    }

    fn passes(
        &self,
        faces: &[u32],
        guide: &[u32],
        v: &BTreeMap<String, openchuck_path::ParameterValue>,
    ) -> Vec<Pass> {
        self.output(&self.strategy(faces, guide), v).unwrap().passes
    }

    /// The tool's axis at a cut point.
    fn axis(&self, c: &CutPoint) -> Vec3 {
        self.rotary.tool_axis(c.a).get()
    }
}

fn contacts(pass: &Pass) -> Vec<Point3> {
    pass.points.iter().map(|c| c.contact).collect()
}

/// The distance from `q` to the polyline `line`.
fn to_polyline(q: Point3, line: &[Point3]) -> f64 {
    line.windows(2)
        .map(|w| {
            let (a, b) = (w[0], w[1]);
            let d = b - a;
            let t = ((q - a).dot(d) / d.dot(d)).clamp(0.0, 1.0);
            q.distance(a + d * t)
        })
        .fold(f64::INFINITY, f64::min)
}

/// The middle of a pass, by its contact points.
fn middle(pass: &Pass) -> Point3 {
    let c = contacts(pass);
    let n = c.len() as f64;
    let s = c.iter().fold(Vec3::ZERO, |s, p| s + p.to_vec());
    Point3::ORIGIN + s * (1.0 / n)
}

// ---- OP-28 point 1 and 2: passes on the face, parallel to the guide, at the stepover ----

#[test]
fn req_op_28_on_a_twisted_ruled_face_the_passes_lie_on_the_face_parallel_to_the_guide_at_the_stepover()
 {
    let (model, height) = twisted(30.0, 6.0, [10.0, 12.0, 11.0, 10.0]);
    let rig = Rig::new(model, &ball(6.0));
    let stepover = 0.8;
    let passes = rig.passes(&[0], &[], &values(stepover, &[]));
    assert!(passes.len() >= 15, "{} passes", passes.len());
    // Every contact is on the face, by its closed form.
    for c in passes.iter().flat_map(|p| &p.points) {
        let q = c.contact;
        assert!(
            q.x > -1e-6 && q.x < 30.0 + 1e-6 && q.y.abs() < 6.0 + 1e-6,
            "{q:?}"
        );
        assert!(
            (q.z - height(q.x, q.y)).abs() < 0.005,
            "{q:?} is off the face"
        );
    }
    // The default guide is the longest side: the edge at Y -6 (it rises by
    // 2 mm over its length; the one at Y 6 falls by 1 mm), so the first
    // pass runs along it and the last along the opposite side. On a twisted
    // face the ball's contact can sit a little off the pass's own point
    // where it rests at the edge, so within a few hundredths.
    let first = contacts(&passes[0]);
    let last = contacts(passes.last().unwrap());
    assert!(first.iter().all(|q| (q.y + 6.0).abs() < 0.05), "{first:?}");
    assert!(last.iter().all(|q| (q.y - 6.0).abs() < 0.05), "{last:?}");
    assert!(first.iter().any(|q| q.x < 1e-3) && first.iter().any(|q| q.x > 30.0 - 1e-3));
    // Neighbouring passes are no further apart than the stepover, measured
    // on the face, and not much closer either: parallel to the guide.
    for w in passes.windows(2) {
        let earlier = contacts(&w[0]);
        for q in contacts(&w[1]) {
            let d = to_polyline(q, &earlier);
            assert!(d <= stepover * 1.001 + 0.005, "a gap of {d} mm");
        }
    }
    for w in passes[..passes.len() - 2].windows(2) {
        let earlier = contacts(&w[0]);
        let d = contacts(&w[1])
            .iter()
            .map(|q| to_polyline(*q, &earlier))
            .fold(f64::INFINITY, f64::min);
        assert!(d > 0.6 * stepover, "passes crowd to {d} mm");
    }
}

#[test]
fn req_op_28_the_notes_name_the_guide_and_the_passes() {
    let (model, _) = twisted(30.0, 6.0, [10.0, 12.0, 11.0, 10.0]);
    let rig = Rig::new(model, &ball(6.0));
    let out = rig
        .output(&rig.strategy(&[0], &[]), &values(1.0, &[]))
        .unwrap();
    let notes = out.notes().join("\n");
    assert!(notes.contains("guide"), "{notes}");
    assert!(notes.contains("edge 0"), "{notes}");
    assert!(
        notes.contains(&format!("{} passes", out.passes.len())),
        "{notes}"
    );
}

// ---- the stepover on a cylinder, as a distance and as a scallop height ----------------

/// The cusp height between two ball passes round a cylinder of `radius`,
/// at angles `t1` and `t2`, for a ball of radius `rho`: the two balls'
/// centres are on the circle of `radius + rho`, and the cusp is where their
/// circles meet nearest the axis.
fn cusp(radius: f64, rho: f64, t1: f64, t2: f64) -> f64 {
    let big = radius + rho;
    let half = 0.5 * (t2 - t1).abs();
    let c = half.cos();
    let r = big * c - (big * big * c * c - (big * big - rho * rho)).sqrt();
    r - radius
}

fn angle(q: Point3) -> f64 {
    q.z.atan2(q.y)
}

#[test]
fn req_op_28_on_a_cylinder_the_scallop_height_is_met_between_passes() {
    let radius = 10.0;
    let rig = Rig::new(
        cylinder_patch(radius, FRAC_PI_3, 2.0 * FRAC_PI_3, 30.0),
        &ball(6.0),
    );
    let h = 0.01;
    let passes = rig.passes(&[0], &[], &values(5.0, &[(p::SCALLOP, num(h))]));
    // The passes run along X (the guide is the longer straight side) and
    // step round the axis.
    let angles: Vec<f64> = passes.iter().map(|p| angle(middle(p))).collect();
    for p in &passes {
        let c = contacts(p);
        let a0 = angle(c[0]);
        assert!(
            c.iter().all(|q| (angle(*q) - a0).abs() < 1e-4),
            "a pass turns round the axis"
        );
        assert!(
            c.iter()
                .all(|q| ((q.y * q.y + q.z * q.z).sqrt() - radius).abs() < 0.005)
        );
    }
    let mut highest: f64 = 0.0;
    for w in angles.windows(2) {
        // A gap too wide for the formula gives NaN, which `max` would drop
        // (issue #896 item 7).
        let ridge = cusp(radius, 3.0, w[0], w[1]);
        assert!(ridge.is_finite(), "no ridge between {} and {}", w[0], w[1]);
        highest = highest.max(ridge);
    }
    assert!(highest <= h + 1e-4, "a cusp of {highest} mm");
    // Not needlessly close: the first steps leave at least half the height.
    assert!(cusp(radius, 3.0, angles[0], angles[1]) > 0.5 * h);
}

#[test]
fn req_op_28_following_the_outline_on_a_cylinder_meets_the_scallop_height() {
    // CodeRabbit on #854: the outline's loops were spaced for the scallop
    // height on a flat, which leaves a higher ridge on a face curving away
    // from the tool. Along the patch's straight sides the loops step round
    // the axis, so the ridge between two is the one between two passes at
    // their angles.
    let radius = 10.0;
    let rig = Rig::new(
        cylinder_patch(radius, FRAC_PI_3, 2.0 * FRAC_PI_3, 30.0),
        &ball(6.0),
    );
    let h = 0.01;
    let passes = rig.passes(
        &[0],
        &[],
        &values(5.0, &[(p::SCALLOP, num(h)), (p::PATTERN, text(p::OUTLINE))]),
    );
    // Each loop's angle along the side at the start of the turn, away from
    // the ends.
    let mut angles: Vec<f64> = passes
        .iter()
        .filter_map(|pass| {
            let near: Vec<f64> = contacts(pass)
                .iter()
                .filter(|q| q.x > 12.0 && q.x < 18.0 && angle(**q) < 0.5 * PI)
                .map(|q| angle(*q))
                .collect();
            (!near.is_empty()).then(|| near.iter().sum::<f64>() / near.len() as f64)
        })
        .collect();
    angles.sort_by(f64::total_cmp);
    assert!(angles.len() > 4, "{angles:?}");
    let ridges: Vec<f64> = angles
        .windows(2)
        .map(|w| cusp(radius, 3.0, w[0], w[1]))
        .collect();
    // A gap too wide for the formula gives NaN, which `max` would drop
    // (issue #896 item 7).
    assert!(ridges.iter().all(|r| r.is_finite()), "{ridges:?}");
    let highest = ridges.iter().copied().fold(0.0, f64::max);
    // The loops follow the grid the offsets are traced on, a few hundredths
    // of the spacing either way.
    assert!(highest <= 1.05 * h, "a ridge of {highest} mm");
    assert!(highest > 0.5 * h, "a ridge of {highest} mm");
}

#[test]
fn req_op_28_on_a_cylinder_the_stepover_is_measured_on_the_face() {
    let radius = 10.0;
    let rig = Rig::new(
        cylinder_patch(radius, FRAC_PI_3, 2.0 * FRAC_PI_3, 30.0),
        &ball(6.0),
    );
    let stepover = 1.0;
    let passes = rig.passes(&[0], &[], &values(stepover, &[]));
    let angles: Vec<f64> = passes.iter().map(|p| angle(middle(p))).collect();
    let span = radius * FRAC_PI_3;
    assert_eq!(passes.len(), (span / stepover).ceil() as usize + 1);
    for w in angles.windows(2) {
        let arc = radius * (w[1] - w[0]).abs();
        assert!(arc <= stepover * 1.0005, "{arc} mm apart on the face");
        assert!(arc > 0.5 * stepover);
    }
}

#[test]
fn req_op_28_a_scallop_height_for_a_flat_end_is_ignored_with_a_warning() {
    let rig = Rig::new(
        cylinder_patch(10.0, FRAC_PI_3, 2.0 * FRAC_PI_3, 30.0),
        &support::tool(openchuck_types::tool::ToolShape::FlatEnd, 6.0),
    );
    let strategy = rig.strategy(&[0], &[]);
    let v = values(1.0, &[(p::SCALLOP, num(0.01))]);
    let parameters = strategy.resolve_parameters(&v).unwrap();
    let warnings = strategy.parameter_warnings(&rig.tool, &parameters);
    assert!(
        warnings.iter().any(|w| w.contains("scallop")),
        "{warnings:?}"
    );
    let with = rig.passes(&[0], &[], &v);
    let without = rig.passes(&[0], &[], &values(1.0, &[]));
    assert_eq!(with, without);
}

// ---- point 1: the blend between boundaries of different length -----------------------

/// A trapezoid at Z 10: the guide side from (0, -5) to (30, -5) and the
/// opposite side from (25, 5) to (5, 5). Edges 0 (bottom), 1, 2 (top), 3.
fn trapezoid() -> BrepModel {
    let mut b = Builder::default();
    flat(
        &mut b,
        10.0,
        &[(0.0, -5.0), (30.0, -5.0), (25.0, 5.0), (5.0, 5.0)],
        &[],
    );
    b.model
}

#[test]
fn req_op_28_passes_blend_from_the_guide_to_a_shorter_opposite_side() {
    let rig = Rig::new(trapezoid(), &ball(6.0));
    let stepover = 0.7;
    let passes = rig.passes(&[0], &[], &values(stepover, &[]));
    let first = contacts(&passes[0]);
    let last = contacts(passes.last().unwrap());
    // The first pass is the guide, the last the opposite side, end to end.
    assert!(first.iter().all(|q| (q.y + 5.0).abs() < 1e-6));
    assert!(last.iter().all(|q| (q.y - 5.0).abs() < 1e-6));
    let span = |c: &[Point3]| {
        (
            c.iter().map(|q| q.x).fold(f64::INFINITY, f64::min),
            c.iter().map(|q| q.x).fold(f64::NEG_INFINITY, f64::max),
        )
    };
    let (a, b) = span(&first);
    assert!(a.abs() < 1e-6 && (b - 30.0).abs() < 1e-6, "{a} {b}");
    let (a, b) = span(&last);
    assert!((a - 5.0).abs() < 1e-6 && (b - 25.0).abs() < 1e-6, "{a} {b}");
    // Each pass is on the face, and the passes in between shorten evenly:
    // each runs from the left side to the right side.
    for p in &passes {
        let c = contacts(p);
        for q in &c {
            assert!((q.z - 10.0).abs() < 1e-6);
            let left = 0.5 * (q.y + 5.0);
            assert!(
                q.x > left - 1e-6 && q.x < 30.0 - left + 1e-6,
                "{q:?} is off the face"
            );
        }
        let (a, b) = span(&c);
        let left = 0.5 * (c[0].y + 5.0);
        assert!(
            (a - left).abs() < 1e-3 && (b - (30.0 - left)).abs() < 1e-3,
            "{a} {b}"
        );
    }
    // No gap on the face wider than the stepover.
    for w in passes.windows(2) {
        let earlier = contacts(&w[0]);
        for q in contacts(&w[1]) {
            assert!(to_polyline(q, &earlier) <= stepover * 1.001);
        }
    }
}

#[test]
fn req_op_28_a_three_sided_face_closes_on_its_apex() {
    let mut b = Builder::default();
    flat(&mut b, 10.0, &[(0.0, -5.0), (30.0, -5.0), (15.0, 5.0)], &[]);
    let rig = Rig::new(b.model, &ball(6.0));
    let out = rig
        .output(&rig.strategy(&[0], &[]), &values(0.7, &[]))
        .unwrap();
    let passes = &out.passes;
    assert!(
        contacts(&passes[0])
            .iter()
            .all(|q| (q.y + 5.0).abs() < 1e-6)
    );
    let triangle = [(0.0, -5.0), (30.0, -5.0), (15.0, 5.0)];
    for q in passes.iter().flat_map(contacts) {
        let inside = inside_polygon(&triangle, q.x, q.y)
            || to_polyline(
                q,
                &[
                    p3(0.0, -5.0, 10.0),
                    p3(30.0, -5.0, 10.0),
                    p3(15.0, 5.0, 10.0),
                    p3(0.0, -5.0, 10.0),
                ],
            ) < 1e-6;
        assert!(inside, "{q:?} is off the face");
    }
    // The passes close on the apex, ending where they would be shorter than
    // the tool's diameter, and a note says so.
    let shortest = passes
        .iter()
        .map(|p| {
            let c = contacts(p);
            c[0].distance(c[c.len() - 1])
        })
        .fold(f64::INFINITY, f64::min);
    assert!(shortest >= 6.0 - 1e-6, "{shortest}");
    let top = passes
        .iter()
        .flat_map(contacts)
        .map(|q| q.y)
        .fold(f64::NEG_INFINITY, f64::max);
    assert!(
        top > 5.0 - 10.0 * 6.0 / 30.0 - 0.7 - 1e-6,
        "the passes stop at Y {top}"
    );
    assert!(out.notes().join(" ").contains("point"), "{:?}", out.notes());
}

// ---- point 1: the guide, by default and chosen ---------------------------------------

#[test]
fn req_op_28_of_two_equal_sides_the_guide_is_the_one_with_the_lowest_edge() {
    // A rectangle: the sides along X are equal; edge 0 is at Y -5.
    let mut b = Builder::default();
    flat(
        &mut b,
        10.0,
        &[(0.0, -5.0), (20.0, -5.0), (20.0, 5.0), (0.0, 5.0)],
        &[],
    );
    let rig = Rig::new(b.model, &ball(6.0));
    let passes = rig.passes(&[0], &[], &values(1.0, &[]));
    assert!(
        contacts(&passes[0])
            .iter()
            .all(|q| (q.y + 5.0).abs() < 1e-6)
    );
}

#[test]
fn req_op_28_the_user_can_choose_the_guide() {
    let rig = Rig::new(trapezoid(), &ball(6.0));
    // The top side (edge 2) as the guide: the passes start there.
    let passes = rig.passes(&[0], &[2], &values(0.7, &[]));
    assert!(
        contacts(&passes[0])
            .iter()
            .all(|q| (q.y - 5.0).abs() < 1e-6)
    );
    assert!(
        contacts(passes.last().unwrap())
            .iter()
            .all(|q| (q.y + 5.0).abs() < 1e-6)
    );
    // A slanted side (edge 1) as the guide: the passes run along it.
    let passes = rig.passes(&[0], &[1], &values(0.7, &[]));
    let first = contacts(&passes[0]);
    let side = [p3(30.0, -5.0, 10.0), p3(25.0, 5.0, 10.0)];
    assert!(first.iter().all(|q| to_polyline(*q, &side) < 1e-6));
}

#[test]
fn req_op_28_a_guide_that_is_not_a_boundary_chain_of_the_faces_is_refused() {
    let rig = Rig::new(channel(4.0), &ball(6.0));
    // Edge 5 is a wall's top edge, not on the floor.
    let err = rig
        .output(&rig.strategy(&[0], &[5]), &values(1.0, &[]))
        .unwrap_err()
        .to_string();
    assert!(err.contains("edge 5") && err.contains("guide"), "{err}");
    // Edges 0 and 2 are the floor's two long sides, not joined end to end.
    let err = rig
        .output(&rig.strategy(&[0], &[0, 2]), &values(1.0, &[]))
        .unwrap_err()
        .to_string();
    assert!(err.contains("guide") && err.contains("joined"), "{err}");
    // An edge the model does not have.
    let err = rig
        .output(&rig.strategy(&[0], &[99]), &values(1.0, &[]))
        .unwrap_err()
        .to_string();
    assert!(err.contains("edge 99"), "{err}");
}

// ---- point 4: protection of neighbouring faces, and what cannot be reached --------------

/// The ball's centre at a cut point.
fn centre_of(rig: &Rig, c: &CutPoint, r: f64) -> Point3 {
    c.tip + rig.axis(c) * r
}

/// The straight machine move from `a` to `b`, at fraction `f`: the tip
/// and A, interpolated in machine coordinates (decision records 0115 and
/// 0133).
fn between(rig: &Rig, a: &CutPoint, b: &CutPoint, f: f64) -> CutPoint {
    let pa = rig.rotary.to_axis(a.tip, a.a);
    let pb = rig.rotary.to_axis(b.tip, b.a);
    let bb = openchuck_path::rotary::nearest_turn(b.a, a.a);
    let at = openchuck_types::units::Degrees(a.a.0 + (bb.0 - a.a.0) * f);
    let pos = openchuck_path::rotary::AxisPosition {
        along: pa.along + (pb.along - pa.along) * f,
        across: pa.across + (pb.across - pa.across) * f,
        height: pa.height + (pb.height - pa.height) * f,
    };
    let tip = rig.rotary.to_model(pos, at);
    CutPoint {
        tip,
        a: at,
        contact: tip,
    }
}

#[test]
fn req_op_28_the_walls_beside_a_floor_are_never_gouged_and_what_cannot_be_reached_is_reported() {
    let half = 4.0;
    let rig = Rig::new(channel(half), &ball(6.0));
    let out = rig
        .output(&rig.strategy(&[0], &[]), &values(0.5, &[]))
        .unwrap();
    assert!(!out.passes.is_empty());
    // Independent of the library: the ball's centre stays at least its
    // radius from the floor and from both walls, at every point and at the
    // middle and quarter points of every move.
    let tol = 0.005;
    let check = |c: &CutPoint| {
        let o = centre_of(&rig, c, 3.0);
        assert!(o.z - 10.0 >= 3.0 - tol, "into the floor at {o:?}");
        if o.z < 16.0 + 3.0 {
            assert!(o.y.abs() <= half - 3.0 + tol, "into a wall at {o:?}");
        }
    };
    for pass in &out.passes {
        for c in &pass.points {
            check(c);
        }
        for w in pass.points.windows(2) {
            for f in [0.25, 0.5, 0.75] {
                check(&between(&rig, &w[0], &w[1], f));
            }
        }
    }
    // The floor within a tool radius of each wall cannot be reached: it is
    // reported, not cut.
    assert!(
        out.passes
            .iter()
            .flat_map(contacts)
            .all(|q| q.y.abs() <= half - 3.0 + tol)
    );
    assert!(!out.unreached.is_empty());
    for u in &out.unreached {
        assert_eq!(u.reason, UnreachedReason::Blocked);
        assert!(u.area > 0.0 && !u.points.is_empty());
        assert!(u.points.iter().all(|q| q.y.abs() > half - 3.0 - 0.5 - tol));
    }
    let area: f64 = out.unreached.iter().map(|u| u.area).sum();
    let expected = 30.0 * 2.0 * 3.0;
    assert!(
        (area - expected).abs() < 0.2 * expected,
        "{area} mm² reported"
    );
    assert!(
        out.warnings()
            .into_iter()
            .any(|w| w.text.contains("cannot reach")),
        "{:?}",
        out.warnings()
    );
}

#[test]
fn req_op_28_a_floor_narrower_than_the_tool_is_reported_as_too_narrow() {
    let rig = Rig::new(channel(2.5), &ball(6.0));
    let err = rig
        .output(&rig.strategy(&[0], &[]), &values(0.5, &[]))
        .unwrap_err()
        .to_string();
    assert!(err.contains("too narrow"), "{err}");
}

#[test]
fn req_op_28_an_avoid_set_is_not_cut() {
    // Two flat faces side by side, both selected, the second also an avoid
    // set (OP-02): its surface is left as it is.
    let mut b = Builder::default();
    flat(
        &mut b,
        10.0,
        &[(0.0, -5.0), (15.0, -5.0), (15.0, 5.0), (0.0, 5.0)],
        &[],
    );
    flat(
        &mut b,
        10.0,
        &[(15.0, -5.0), (30.0, -5.0), (30.0, 5.0), (15.0, 5.0)],
        &[None, None, None, Some(1)],
    );
    let rig = Rig::new(b.model, &ball(6.0));
    let avoid = openchuck_path::BrepSurface::with_faces(&rig.model, &[FaceId(1)]).unwrap();
    let region = RegionConstraint {
        avoid: Some(&avoid),
        ..RegionConstraint::default()
    };
    let strategy = rig.strategy(&[0, 1], &[]);
    let parameters = strategy.resolve_parameters(&values(0.5, &[])).unwrap();
    let out = run(|ctx| strategy.passes_with_notes(&rig.input(), Some(&region), &parameters, ctx))
        .unwrap();
    let all: Vec<Point3> = out.passes.iter().flat_map(contacts).collect();
    assert!(all.iter().all(|q| q.x < 15.0), "a contact on the avoid set");
    assert!(all.iter().any(|q| q.x > 10.0) && all.iter().any(|q| q.x < 1.0));
    // Left out on purpose, not unreachable.
    assert!(out.unreached.is_empty());
    // The strategy says it honours regions, so a host gives it its sets
    // (OP-02), and its plain passes are those it notes (OP-16).
    assert!(strategy.honours_region());
    let plain = run(|ctx| strategy.passes(&rig.input(), &parameters, ctx)).unwrap();
    let noted = run(|ctx| strategy.passes_with_notes(&rig.input(), None, &parameters, ctx))
        .unwrap()
        .passes;
    assert_eq!(plain, noted);
}

/// A flat face at Z 10 bounded by `edges`, each a curve from t 0 to 1, in
/// order round the face counter-clockwise seen from above.
fn curved_flat(edges: &[&dyn Fn(f64) -> Point3]) -> BrepModel {
    let mut b = Builder::default();
    let ids: Vec<EdgeId> = edges.iter().map(|c| b.edge(&[0], *c, false)).collect();
    let outline: Vec<(f64, f64)> = edges
        .iter()
        .flat_map(|c| (0..256).map(move |i| c(f64::from(i) / 256.0)))
        .map(|p| (p.x, p.y))
        .collect();
    let xs = outline.iter().map(|c| c.0);
    let ys = outline.iter().map(|c| c.1);
    let uv = [
        xs.clone().fold(f64::INFINITY, f64::min),
        xs.fold(f64::NEG_INFINITY, f64::max),
        ys.clone().fold(f64::INFINITY, f64::min),
        ys.fold(f64::NEG_INFINITY, f64::max),
    ];
    let inside = {
        let outline = outline.clone();
        move |u: f64, v: f64| inside_polygon(&outline, u, v)
    };
    b.face(
        Surface::Plane {
            frame: frame(
                p3(0.0, 0.0, 10.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ),
        },
        uv,
        Some(outline),
        ids,
        &|u, v| p3(u, v, 10.0),
        &inside,
        false,
    );
    b.model
}

#[test]
fn req_op_28_a_round_face_is_cut_between_its_two_halves() {
    // One closed edge, a circle of radius 5 about (15, 0): no corners, so
    // it is cut at its two points furthest apart.
    let circle = |t: f64| {
        let a = std::f64::consts::TAU * t;
        p3(15.0 + 5.0 * a.cos(), 5.0 * a.sin(), 10.0)
    };
    let rig = Rig::new(curved_flat(&[&circle]), &ball(6.0));
    let out = rig
        .output(&rig.strategy(&[0], &[]), &values(0.8, &[]))
        .unwrap();
    let r = |q: &Point3| (q.x - 15.0).hypot(q.y);
    let all: Vec<Point3> = out.passes.iter().flat_map(contacts).collect();
    assert!(
        all.iter().all(|q| r(q) <= 5.0 + 1e-3),
        "a contact outside the face"
    );
    let first = contacts(&out.passes[0]);
    let last = contacts(out.passes.last().unwrap());
    assert!(first.iter().all(|q| (r(q) - 5.0).abs() < 1e-3));
    assert!(last.iter().all(|q| (r(q) - 5.0).abs() < 1e-3));
    // The two halves are opposite each other.
    let (a, b) = (middle(&out.passes[0]), middle(out.passes.last().unwrap()));
    assert!((a - p3(15.0, 0.0, 10.0)).dot(b - p3(15.0, 0.0, 10.0)) < 0.0);
    assert!(out.passes.len() >= 10);
}

#[test]
fn req_op_28_a_two_sided_face_is_cut_from_its_longer_side_to_the_other() {
    // Two arcs from (0, 0) to (20, 0), meeting at corners: the lower one
    // bulges 4 mm and is the longer, the upper one 2 mm.
    let arc = |sag: f64| {
        move |t: f64| {
            let x = 20.0 * t;
            // A circle through both ends with the sagitta `sag`.
            let radius = (100.0 + sag * sag) / (2.0 * sag);
            let y = ((radius * radius - (x - 10.0) * (x - 10.0)).sqrt() - (radius - sag)).max(0.0);
            p3(x, y, 10.0)
        }
    };
    let up = arc(2.0);
    let lower = arc(4.0);
    let down = move |t: f64| {
        let p = lower(t);
        p3(p.x, -p.y, 10.0)
    };
    let back = move |t: f64| up(1.0 - t);
    let rig = Rig::new(curved_flat(&[&down, &back]), &ball(6.0));
    let passes = rig.passes(&[0], &[], &values(0.5, &[]));
    let on = |q: &Point3, sag: f64, sign: f64| {
        let radius = (100.0 + sag * sag) / (2.0 * sag);
        let centre = p3(10.0, -sign * (radius - sag), 10.0);
        (q.distance(centre) - radius).abs() < 1e-3
    };
    assert!(contacts(&passes[0]).iter().all(|q| on(q, 4.0, -1.0)));
    assert!(
        contacts(passes.last().unwrap())
            .iter()
            .all(|q| on(q, 2.0, 1.0))
    );
}

#[test]
fn req_op_28_moves_round_a_small_face_off_the_axis_stay_within_the_tolerance() {
    // A narrow strip of a cylinder of radius 1.2 whose axis is 8 mm from
    // the rotary axis: the guide is its arc, so each pass goes round it and
    // the part turns under the tool. A straight machine move between two
    // points then leaves the face, and must be split to the tolerance
    // (decision records 0115 and 0133).
    let (centre, radius, r) = (8.0, 1.2, 1.0);
    let rig = Rig::new(
        offset_cylinder(centre, radius, PI / 6.0, 5.0 * PI / 6.0, 2.0),
        &ball(2.0 * r),
    );
    // A fine tolerance: the face's points along a pass must be closer than
    // the guide's samples for its chords to stay on it.
    let tol = 0.002;
    let passes = rig.passes(&[0], &[], &values(0.3, &[(names::TOLERANCE, num(tol))]));
    let longest = passes.iter().map(|p| p.points.len()).max().unwrap();
    assert!(longest > 40, "only {longest} points round the face");
    assert!(!passes.is_empty());
    let from_axis = |c: &CutPoint| {
        let o = centre_of(&rig, c, r);
        o.y.hypot(o.z - centre)
    };
    for pass in &passes {
        for c in &pass.points {
            let d = from_axis(c);
            assert!(d >= radius + r - tol - 1e-6, "into the face at {c:?}: {d}");
        }
        for w in pass.points.windows(2) {
            for f in [0.25, 0.5, 0.75] {
                let d = from_axis(&between(&rig, &w[0], &w[1], f));
                assert!(
                    d >= radius + r - tol - 1e-6,
                    "a move cuts into the face: {d}"
                );
                assert!(d <= radius + r + 2.0 * tol, "a move leaves {d} mm");
            }
        }
    }
}

#[test]
fn req_op_28_stock_to_leave_is_left_on_the_face() {
    let rig = Rig::new(trapezoid(), &ball(6.0));
    let passes = rig.passes(
        &[0],
        &[],
        &values(1.0, &[(names::STOCK_TO_LEAVE, num(0.2))]),
    );
    for c in passes.iter().flat_map(|p| &p.points) {
        // Within the exact drop's accuracy at the face's edge.
        assert!((c.tip.z - 10.2).abs() < 1e-4, "{c:?}");
    }
}

// ---- point 3: the tool axis ---------------------------------------------------------

/// The difference between two angles in degrees, in (-180, 180].
fn turn(a: f64, b: f64) -> f64 {
    (a - b + 540.0).rem_euclid(360.0) - 180.0
}

#[test]
fn req_op_28_the_tool_axis_follows_the_face_within_the_tilt_limit() {
    // Half a cylinder round the axis, 10 long: the guide is the half-circle
    // arc at X 0, so the passes run round and A follows the face.
    let rig = Rig::new(cylinder_patch(10.0, 0.0, PI, 10.0), &ball(6.0));
    // The axis leans by the lead towards the travel round the axis, and the
    // smoothing may move it by up to the tilt from there, as simultaneous
    // finishing's (OP-06).
    for (lead, tilt) in [(0.0, 5.0), (8.0, 10.0), (8.0, 0.0)] {
        let passes = rig.passes(
            &[0],
            &[],
            &values(1.0, &[(p::LEAD_ANGLE, num(lead)), (p::MAX_TILT, num(tilt))]),
        );
        for pass in &passes {
            let c = contacts(pass);
            // A pass goes round at one X.
            assert!(c.iter().all(|q| (q.x - c[0].x).abs() < 1e-6));
            for point in &pass.points {
                let q = point.contact;
                let normal = Vec3::new(0.0, q.y, q.z);
                let facing = rig.rotary.a_facing(normal).unwrap();
                let off = turn(point.a.0, facing.0).abs();
                assert!(off <= lead + tilt + 1e-6, "{off} degrees from the normal");
                if tilt == 0.0 {
                    assert!((off - lead).abs() < 1e-6, "{off} degrees, not the lead");
                }
            }
            let a: Vec<f64> = pass.points.iter().map(|c| c.a.0).collect();
            let spread = a.iter().fold(f64::NEG_INFINITY, |m, x| m.max(*x))
                - a.iter().fold(f64::INFINITY, |m, x| m.min(*x));
            assert!(spread > 150.0, "A turns only {spread} degrees along a pass");
        }
    }
}

// ---- point 5: direction, and several faces ----------------------------------------

/// The side the next pass is on, seen from the spindle along the feed:
/// positive on the tool's right.
fn sides(rig: &Rig, passes: &[Pass]) -> Vec<f64> {
    passes
        .windows(2)
        .map(|w| {
            let pass = &w[0];
            let i = pass.points.len() / 2;
            let (a, b) = (pass.points[i - 1], pass.points[i]);
            let feed = b.contact - a.contact;
            let right = feed.cross(rig.axis(&b));
            (middle(&w[1]) - middle(pass)).dot(right).signum()
        })
        .collect()
}

#[test]
fn req_op_28_climb_conventional_and_meander_direct_the_passes() {
    let rig = Rig::new(trapezoid(), &ball(6.0));
    let go = |d: &str| rig.passes(&[0], &[], &values(1.0, &[(p::DIRECTION, text(d))]));
    // Climb with M3: the uncut material, where the next pass goes, is on
    // the tool's right looking along the feed from the spindle (decision
    // record 0213); conventional on its left.
    assert!(sides(&rig, &go(p::CLIMB)).iter().all(|s| *s > 0.0));
    assert!(sides(&rig, &go(p::CONVENTIONAL)).iter().all(|s| *s < 0.0));
    // Meander, the default, alternates.
    let meander = go(p::MEANDER);
    let s = sides(&rig, &meander);
    assert!(s.windows(2).all(|w| w[0] == -w[1]), "{s:?}");
    assert_eq!(meander, rig.passes(&[0], &[], &values(1.0, &[])));
}

#[test]
fn req_op_28_several_faces_are_cut_one_after_another_along_their_shared_edges() {
    // Three flat faces in a row along X, at Z 10: face 0 from X 0 to 20,
    // face 1 from 20 to 45 (the longest guide) and face 2 from 45 to 55.
    let mut b = Builder::default();
    flat(
        &mut b,
        10.0,
        &[(0.0, -5.0), (20.0, -5.0), (20.0, 5.0), (0.0, 5.0)],
        &[],
    );
    // Face 1's side at X 20 is face 0's edge 1; face 2's side at X 45 is
    // face 1's edge 5.
    flat(
        &mut b,
        10.0,
        &[(20.0, -5.0), (45.0, -5.0), (45.0, 5.0), (20.0, 5.0)],
        &[None, None, None, Some(1)],
    );
    flat(
        &mut b,
        10.0,
        &[(45.0, -5.0), (55.0, -5.0), (55.0, 5.0), (45.0, 5.0)],
        &[None, None, None, Some(5)],
    );
    let rig = Rig::new(b.model, &ball(6.0));
    let passes = rig.passes(&[0, 1, 2], &[], &values(1.0, &[]));
    // Which face each pass is on, by its middle.
    let face = |p: &Pass| {
        let x = middle(p).x;
        if x < 20.0 {
            0
        } else if x < 45.0 {
            1
        } else {
            2
        }
    };
    let order: Vec<u32> = passes.iter().map(face).collect();
    let mut runs = order.clone();
    runs.dedup();
    // Face 1 first (its guide is the longest), then a face that shares an
    // edge with it, lowest ID first; each face once.
    assert_eq!(runs, vec![1, 0, 2], "{order:?}");
    // Within a face no pass waits for a full retract.
    for w in passes.windows(2) {
        if face(&w[0]) == face(&w[1]) {
            assert!(w[1].low_link);
        }
    }
}

// ---- point 6 and NFR-07: what is refused ------------------------------------------------

#[test]
fn req_op_28_the_strategy_needs_exact_faces() {
    let flow = FaceFlow::default();
    assert!(flow.needs_brep_part().is_some());
    assert_eq!(flow.name(), face_flow::NAME);
    // What it does, in plain words, never implying the output is safe to
    // run unchecked (OP-16, NFR-12; decision 0237).
    assert!(!flow.label().is_empty());
    assert!(flow.description().contains("STEP model"));
    assert!(
        flow.description()
            .contains("Check the toolpath and simulation")
    );
    // The registry's instance has no model: generating with it is refused
    // in plain words, never a crash.
    let rig = Rig::new(trapezoid(), &ball(6.0));
    let err = rig
        .output(&flow, &values(1.0, &[]))
        .unwrap_err()
        .to_string();
    assert!(err.contains("STEP"), "{err}");
}

#[test]
fn req_op_28_nfr_07_bad_input_is_refused_with_a_message() {
    let rig = Rig::new(trapezoid(), &ball(6.0));
    let err = rig
        .output(&rig.strategy(&[], &[]), &values(1.0, &[]))
        .unwrap_err()
        .to_string();
    assert!(err.contains("face"), "{err}");
    let err = rig
        .output(&rig.strategy(&[7], &[]), &values(1.0, &[]))
        .unwrap_err()
        .to_string();
    assert!(err.contains("face 7"), "{err}");
    // Too fine a stepover for the face.
    let err = rig
        .output(&rig.strategy(&[0], &[]), &values(1e-4, &[]))
        .unwrap_err()
        .to_string();
    assert!(err.contains("passes"), "{err}");
    // The whole boundary as the guide leaves nothing to step towards.
    let err = rig
        .output(&rig.strategy(&[0], &[0, 1, 2, 3]), &values(1.0, &[]))
        .unwrap_err()
        .to_string();
    assert!(err.contains("whole boundary"), "{err}");
    // A face whose edges do not close, and one with no edges at all.
    let mut open = trapezoid();
    open.faces[0].edges.pop();
    let broken = Rig::new(open, &ball(6.0));
    let err = broken
        .output(&broken.strategy(&[0], &[]), &values(1.0, &[]))
        .unwrap_err()
        .to_string();
    assert!(err.contains("does not close"), "{err}");
    let bare = Rig::new(
        openchuck_testkit::ruled::floor_plane(10.0, 0.0, 30.0, -5.0, 5.0),
        &ball(6.0),
    );
    let err = bare
        .output(&bare.strategy(&[0], &[]), &values(1.0, &[]))
        .unwrap_err()
        .to_string();
    assert!(err.contains("no boundary edges"), "{err}");
    let strategy = rig.strategy(&[0], &[]);
    for (name, value) in [
        (p::LEAD_ANGLE, num(40.0)),
        (p::LEAD_ANGLE, num(-40.0)),
        (p::MAX_TILT, num(-1.0)),
        (p::MAX_TILT, num(45.0)),
        (p::SCALLOP, num(-0.1)),
        (p::DIRECTION, text("sideways")),
        (names::STEPOVER, num(0.0)),
        (names::STEPOVER, num(f64::NAN)),
    ] {
        let err = rig.output(&strategy, &values(1.0, &[(name, value.clone())]));
        assert!(err.is_err(), "{name} = {value:?} was accepted");
    }
}

// ---- point 7: determinism --------------------------------------------------------------

#[test]
fn req_op_28_nfr_06_the_path_is_identical_at_1_2_and_n_threads() {
    let rig = Rig::new(channel(4.0), &ball(6.0));
    let strategy = rig.strategy(&[0], &[]);
    let v = values(0.5, &[(p::LEAD_ANGLE, num(3.0))]);
    let runs = openchuck_testkit::determinism::check(|| {
        let path = run(|ctx| openchuck_path::generate(&strategy, &rig.input(), &v, ctx)).unwrap();
        format!("{:?} {:?} {:?}", path.group, path.notices, path.unreached).into_bytes()
    })
    .unwrap();
    assert_eq!(runs.len(), 4);
}

// ---- issue #791: a face that closes round on itself ---------------------------------

/// A face of revolution about the model X axis that closes round on itself,
/// as a STEP file gives it: `surface` evaluated by `point(u, v)` over u from
/// 0 to a full turn and v from `v0` to `v1`; edge 0 the end at `v0`, edge 1
/// the end at `v1` (a single point where `point` collapses there), each
/// starting and ending at its one vertex on the seam, and edge 2 the seam
/// at u 0, which bounds the face twice but is listed once.
fn revolved(surface: Surface, v0: f64, v1: f64, point: &dyn Fn(f64, f64) -> Point3) -> BrepModel {
    let tau = std::f64::consts::TAU;
    let mut b = Builder::default();
    let edges = vec![
        b.edge(&[0], &|t| point(tau * t, v0), false),
        b.edge(&[0], &|t| point(tau * t, v1), false),
        b.edge(&[0], &|t| point(0.0, v0 + (v1 - v0) * t), false),
    ];
    b.face(
        surface,
        [0.0, tau, v0, v1],
        None,
        edges,
        point,
        &|_, _| true,
        false,
    );
    b.model
}

/// The frame of a surface of revolution about the model X axis.
fn about_x() -> Frame {
    Frame {
        origin: p3(0.0, 0.0, 0.0),
        x: Vec3::new(0.0, 1.0, 0.0),
        y: Vec3::new(0.0, 0.0, 1.0),
        z: Vec3::new(1.0, 0.0, 0.0),
    }
}

/// The full cylinder of `radius` about X from X 0 to `length`.
fn full_cylinder(radius: f64, length: f64) -> BrepModel {
    revolved(
        Surface::Cylinder {
            frame: about_x(),
            radius,
        },
        0.0,
        length,
        &|u, v| p3(v, radius * u.cos(), radius * u.sin()),
    )
}

/// The full cone about X from `r0` at X 0 to `r1` at X `length` (0 for its
/// apex).
fn full_cone(r0: f64, r1: f64, length: f64) -> BrepModel {
    let angle = ((r1 - r0) / length).atan();
    let slant = length / angle.cos();
    revolved(
        Surface::Cone {
            frame: about_x(),
            radius: r0,
            semi_angle: angle,
        },
        0.0,
        slant,
        &move |u, v| {
            let w = r0 + v * angle.sin();
            p3(v * angle.cos(), w * u.cos(), w * u.sin())
        },
    )
}

/// The rings of `passes`: for each, its X (every contact within 1e-3 of it)
/// and the angle it turns through round X.
fn rings(passes: &[Pass]) -> Vec<(f64, f64)> {
    passes
        .iter()
        .map(|p| {
            let c = contacts(p);
            let x = c[0].x;
            assert!(
                c.iter().all(|q| (q.x - x).abs() < 1e-3),
                "a pass leaves X {x}"
            );
            let mut turn = 0.0;
            for w in c.windows(2) {
                let mut d = angle(w[1]) - angle(w[0]);
                if d > PI {
                    d -= 2.0 * PI;
                } else if d < -PI {
                    d += 2.0 * PI;
                }
                turn += d;
            }
            (x, turn.abs())
        })
        .collect()
}

#[test]
fn req_op_28_a_cylinder_just_short_of_a_whole_turn_is_open() {
    // Issue #896 item 3: a face closes round only when it spans its whole
    // period, within 1e-6 of it. A cylinder 5e-5 rad short of a whole turn
    // has its two straight sides 0.0005 mm apart, nearer than the join
    // distance, yet it is open: a straight side is an edge it can be cut
    // from (on the full cylinder that side is the seam, refused as a guide),
    // and its passes run along X, each at one angle.
    let radius = 10.0;
    let rig = Rig::new(
        cylinder_patch(radius, 0.0, 2.0 * PI - 5e-5, 20.0),
        &ball(6.0),
    );
    let out = rig
        .output(&rig.strategy(&[0], &[0]), &values(1.0, &[]))
        .unwrap_or_else(|e| panic!("the open face was refused: {e}"));
    assert!(!out.passes.is_empty());
    for pass in &out.passes {
        let c = contacts(pass);
        let a0 = angle(c[0]);
        assert!(
            c.iter().all(|q| (angle(*q) - a0).abs() < 1e-4),
            "a pass turns round the axis"
        );
    }
}

/// The full cylinder of radius 10 about X from X 0 to 20 with a hole in it
/// from 0.2 to 0.37 of the way round and 0.3 to 2.5 mm along: a face that
/// closes round on itself and is trimmed, so the first places round it
/// where the rings' direction is read are off the face.
fn holed_cylinder() -> BrepModel {
    let tau = std::f64::consts::TAU;
    let (u0, u1, v0, v1) = (0.2 * tau, 0.37 * tau, 0.3, 2.5);
    let mut model = full_cylinder(10.0, 20.0);
    let square = |a: (f64, f64), b: (f64, f64)| {
        TrimLoop(
            [(a.0, a.1), (b.0, a.1), (b.0, b.1), (a.0, b.1)]
                .into_iter()
                .map(|(u, v)| UvPoint { u, v })
                .collect(),
        )
    };
    let face = &mut model.faces[0];
    face.trims = vec![square((0.0, 0.0), (tau, 20.0)), square((u0, v0), (u1, v1))];
    // The tessellation leaves the hole out too.
    let mesh = &mut face.tessellation;
    let inside = |q: Point3| {
        let u = q.z.atan2(q.y).rem_euclid(tau);
        !(u > u0 && u < u1 && q.x > v0 && q.x < v1)
    };
    let kept: Vec<[u32; 3]> = mesh
        .triangles
        .iter()
        .copied()
        .filter(|t| {
            let c = t
                .iter()
                .fold(Vec3::ZERO, |s, &i| s + mesh.vertices[i as usize].to_vec())
                * (1.0 / 3.0);
            inside(Point3::ORIGIN + c)
        })
        .collect();
    mesh.triangles = kept;
    model
}

/// The angle a pass turns through round X, signed: positive turning from
/// +Y towards +Z.
fn signed_turn(pass: &Pass) -> f64 {
    contacts(pass)
        .windows(2)
        .map(|w| {
            let d = angle(w[1]) - angle(w[0]);
            if d > PI {
                d - 2.0 * PI
            } else if d < -PI {
                d + 2.0 * PI
            } else {
                d
            }
        })
        .sum()
}

#[test]
fn req_op_28_req_op_25_on_a_trimmed_face_that_closes_round_climb_and_conventional_turn_opposite_ways()
 {
    // Issue #896 item 2: the rings' direction is read at the first place
    // round the face where two rings are on it; on this face the first two
    // places are in the hole. Climb and Conventional still turn their rings
    // opposite ways, every ring of one the same way.
    let rig = Rig::new(holed_cylinder(), &ball(6.0));
    let turns = |direction: &str| -> Vec<f64> {
        let out = rig
            .output(
                &rig.strategy(&[0], &[]),
                &values(1.0, &[(p::DIRECTION, text(direction))]),
            )
            .unwrap_or_else(|e| panic!("{direction}: refused: {e}"));
        out.passes
            .iter()
            .map(signed_turn)
            .filter(|t| t.abs() > PI)
            .collect()
    };
    let climb = turns(p::CLIMB);
    let conventional = turns(p::CONVENTIONAL);
    assert!(
        climb.len() > 4 && conventional.len() > 4,
        "{climb:?} {conventional:?}"
    );
    let one_way = |t: &[f64]| t.iter().all(|x| *x > 0.0) || t.iter().all(|x| *x < 0.0);
    assert!(
        one_way(&climb) && one_way(&conventional),
        "{climb:?} {conventional:?}"
    );
    assert!(
        climb[0] * conventional[0] < 0.0,
        "{climb:?} {conventional:?}"
    );
}

#[test]
fn req_op_28_a_full_cylinder_is_cut_in_rings_from_one_end_to_the_other() {
    // Decision 0675 option A: the seam is not a boundary; the passes are
    // rings round the face from its guide end (the end circles are equally
    // long, so the one with the lower edge ID, at X 0) to the other end.
    let (radius, length, stepover) = (10.0, 20.0, 1.0);
    let rig = Rig::new(full_cylinder(radius, length), &ball(6.0));
    let out = rig.output(&rig.strategy(&[0], &[]), &values(stepover, &[]));
    let out = out.unwrap_or_else(|e| panic!("refused: {e}"));
    for q in out.passes.iter().flat_map(contacts) {
        assert!(
            ((q.y * q.y + q.z * q.z).sqrt() - radius).abs() < 0.005,
            "{q:?}"
        );
        assert!(q.x > -1e-6 && q.x < length + 1e-6, "{q:?}");
    }
    let rings = rings(&out.passes);
    let fewest = (length / stepover).ceil() as usize + 1;
    assert!(
        rings.len() == fewest || rings.len() == fewest + 1,
        "{} rings",
        rings.len()
    );
    assert!(rings[0].0.abs() < 1e-3 && (rings.last().unwrap().0 - length).abs() < 1e-3);
    for (x, turn) in &rings {
        assert!(
            (turn - 2.0 * PI).abs() < 0.01,
            "the ring at X {x} turns {turn}"
        );
    }
    for w in rings.windows(2) {
        let step = w[1].0 - w[0].0;
        assert!(
            step > 0.5 * stepover && step <= stepover * 1.0005,
            "{step} mm apart"
        );
    }
    let notes = out.notes().join("\n");
    assert!(
        notes.contains("face 0 is cut from its guide, edge 0 "),
        "{notes}"
    );
}

#[test]
fn req_op_28_a_cone_between_two_diameters_is_cut_in_rings_from_its_larger_end() {
    let (r0, r1, length) = (6.0, 10.0, 12.0);
    let rig = Rig::new(full_cone(r0, r1, length), &ball(4.0));
    let out = rig
        .output(&rig.strategy(&[0], &[]), &values(1.0, &[]))
        .unwrap_or_else(|e| panic!("refused: {e}"));
    let radius_at = |x: f64| r0 + (r1 - r0) * x / length;
    for q in out.passes.iter().flat_map(contacts) {
        let r = (q.y * q.y + q.z * q.z).sqrt();
        assert!((r - radius_at(q.x)).abs() < 0.005, "{q:?} is off the cone");
    }
    let rings = rings(&out.passes);
    // The guide is the larger end, edge 1 at X 12.
    assert!((rings[0].0 - length).abs() < 1e-3, "{rings:?}");
    assert!(rings.last().unwrap().0.abs() < 1e-3, "{rings:?}");
    assert!(rings.windows(2).all(|w| w[1].0 < w[0].0));
    let notes = out.notes().join("\n");
    assert!(notes.contains("edge 1 "), "{notes}");
}

#[test]
fn req_op_28_a_cone_to_its_apex_closes_on_it() {
    let (r0, length) = (8.0, 10.0);
    let rig = Rig::new(full_cone(r0, 0.0, length), &ball(4.0));
    let out = rig
        .output(&rig.strategy(&[0], &[]), &values(1.0, &[]))
        .unwrap_or_else(|e| panic!("refused: {e}"));
    let rings = rings(&out.passes);
    assert!(rings[0].0.abs() < 1e-3, "{rings:?}");
    assert!(rings.windows(2).all(|w| w[1].0 > w[0].0));
    // Every ring goes once round, the apex's too (CodeRabbit on #854: a
    // point end must turn with the guide, or each ring stops short).
    for (x, turn) in &rings {
        assert!(
            (turn - 2.0 * PI).abs() < 0.01,
            "the ring at X {x} turns {turn}"
        );
    }
    // Rings are left out by their length round, not by the gap between
    // their ends: every ring at least the tool's diameter long is kept, so
    // the ones beside the guide are, and only those nearer the apex than a
    // ring of the tool's diameter are left out.
    let last = rings.last().unwrap().0;
    let r = r0 * (1.0 - last / length);
    assert!(2.0 * PI * r >= 4.0 - 1e-6, "a ring of radius {r} is cut");
    assert!(rings.len() >= 8, "{} rings; {:?}", rings.len(), out.notes());
    let apex_gap = length * (1.0 - 4.0 / (2.0 * PI * r0));
    assert!(last > apex_gap - 1.5, "the last ring at X {last}");
    let notes = out.notes().join("\n");
    assert!(notes.contains("towards the opposite point"), "{notes}");
    assert!(notes.contains("left out"), "{notes}");
}

#[test]
fn req_op_28_req_nfr_07_the_seam_as_guide_and_a_face_with_no_end_are_refused_plainly() {
    let rig = Rig::new(full_cylinder(10.0, 20.0), &ball(6.0));
    let err = rig
        .output(&rig.strategy(&[0], &[2]), &values(1.0, &[]))
        .unwrap_err()
        .to_string();
    assert!(err.contains("edge 2 is the seam of face 0"), "{err}");
    // A full torus: two seams and no end.
    let (major, minor) = (12.0, 3.0);
    let tau = std::f64::consts::TAU;
    let point = move |u: f64, v: f64| {
        let w = major + minor * v.cos();
        p3(minor * v.sin(), w * u.cos(), w * u.sin())
    };
    let mut b = Builder::default();
    let edges = vec![
        b.edge(&[0], &|t| point(tau * t, 0.0), false),
        b.edge(&[0], &|t| point(0.0, tau * t), false),
    ];
    b.face(
        Surface::Torus {
            frame: about_x(),
            major,
            minor,
        },
        [0.0, tau, 0.0, tau],
        None,
        edges,
        &point,
        &|_, _| true,
        false,
    );
    let rig = Rig::new(b.model, &ball(4.0));
    let err = rig
        .output(&rig.strategy(&[0], &[]), &values(1.0, &[]))
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("face 0 closes round on itself with no end edge"),
        "{err}"
    );
}

#[test]
fn req_op_28_rings_blend_between_ends_that_run_opposite_ways() {
    // The end at X 20 is stored running the other way round the axis, as a
    // CAD file may store it: the passes are still rings, not twisted.
    let (radius, length) = (10.0, 20.0);
    let tau = std::f64::consts::TAU;
    let point = move |u: f64, v: f64| p3(v, radius * u.cos(), radius * u.sin());
    let mut b = Builder::default();
    let edges = vec![
        b.edge(&[0], &|t| point(tau * t, 0.0), false),
        b.edge(&[0], &|t| point(tau * (1.0 - t), length), false),
        b.edge(&[0], &|t| point(0.0, length * t), false),
    ];
    b.face(
        Surface::Cylinder {
            frame: about_x(),
            radius,
        },
        [0.0, tau, 0.0, length],
        None,
        edges,
        &point,
        &|_, _| true,
        false,
    );
    let rig = Rig::new(b.model, &ball(6.0));
    let out = rig
        .output(&rig.strategy(&[0], &[]), &values(1.0, &[]))
        .unwrap_or_else(|e| panic!("refused: {e}"));
    for (x, turn) in rings(&out.passes) {
        assert!(
            (turn - 2.0 * PI).abs() < 0.01,
            "the ring at X {x} turns {turn}"
        );
    }
}

#[test]
fn req_op_28_the_user_chooses_which_end_of_a_closed_face_is_the_guide() {
    let rig = Rig::new(full_cylinder(10.0, 20.0), &ball(6.0));
    // Edge 1, the end at X 20, as the guide: the rings start there.
    let out = rig
        .output(&rig.strategy(&[0], &[1]), &values(1.0, &[]))
        .unwrap_or_else(|e| panic!("refused: {e}"));
    let rings = rings(&out.passes);
    assert!((rings[0].0 - 20.0).abs() < 1e-3, "{rings:?}");
    assert!(rings.last().unwrap().0.abs() < 1e-3, "{rings:?}");
    // Edges on both ends are not one guide.
    let err = rig
        .output(&rig.strategy(&[0], &[0, 1]), &values(1.0, &[]))
        .unwrap_err()
        .to_string();
    assert!(err.contains("are not all on one of its ends"), "{err}");
}

/// Two full cylinders about X, faces 0 (radius 10, X 0 to 10; edges 0,
/// 1, seam 2) and 1 (radius 12, X 20 to 30; edges 3, 4, seam 5), and, with
/// `hole`, a small closed edge on face 0's surface that goes nowhere round
/// it, as the edge of a hole: then edge 3, face 1's edges 4, 5 and 6.
fn two_cylinders(hole: bool) -> BrepModel {
    let tau = std::f64::consts::TAU;
    let mut b = Builder::default();
    for (id, radius, x0) in [(0u32, 10.0, 0.0), (1, 12.0, 20.0)] {
        let point = move |u: f64, v: f64| p3(v, radius * u.cos(), radius * u.sin());
        let mut edges = vec![
            b.edge(&[id], &|t| point(tau * t, x0), false),
            b.edge(&[id], &|t| point(tau * t, x0 + 10.0), false),
            b.edge(&[id], &|t| point(0.0, x0 + 10.0 * t), false),
        ];
        if hole && id == 0 {
            edges.push(b.edge(
                &[0],
                &|t| point(2.0 + 0.1 * (tau * t).cos(), 5.0 + 1.0 * (tau * t).sin()),
                false,
            ));
        }
        b.face(
            Surface::Cylinder {
                frame: about_x(),
                radius,
            },
            [0.0, tau, x0, x0 + 10.0],
            None,
            edges,
            &point,
            &|_, _| true,
            false,
        );
    }
    b.model
}

#[test]
fn req_op_28_a_guide_on_one_closed_face_leaves_the_others_their_own() {
    // CodeRabbit on #854: the Guide edge set holds an end of face 0 only;
    // face 1 takes its own default end rather than refusing the operation.
    let rig = Rig::new(two_cylinders(false), &ball(6.0));
    let out = rig
        .output(&rig.strategy(&[0, 1], &[1]), &values(1.0, &[]))
        .unwrap_or_else(|e| panic!("refused: {e}"));
    let notes = out.notes().join("\n");
    assert!(
        notes.contains("face 0 is cut from its guide, edge 1 "),
        "{notes}"
    );
    assert!(
        notes.contains("face 1 is cut from its guide, edge 3 "),
        "{notes}"
    );
}

#[test]
fn req_op_28_req_nfr_07_guide_edges_off_the_chosen_end_are_refused() {
    // CodeRabbit on #854: an end edge with a hole's edge is not one end;
    // the hole's edge is not silently ignored.
    let rig = Rig::new(two_cylinders(true), &ball(6.0));
    let err = rig
        .output(&rig.strategy(&[0], &[0, 3]), &values(1.0, &[]))
        .unwrap_err()
        .to_string();
    assert!(err.contains("are not all on one of its ends"), "{err}");
    // The end's own edges alone are a guide.
    rig.output(&rig.strategy(&[0], &[0]), &values(1.0, &[]))
        .unwrap_or_else(|e| panic!("refused: {e}"));
}

#[test]
fn req_op_28_req_gc_07_rings_are_the_same_at_1_2_and_n_threads() {
    let rig = Rig::new(full_cone(6.0, 10.0, 12.0), &ball(4.0));
    let strategy = rig.strategy(&[0], &[]);
    let v = values(1.0, &[]);
    let runs = openchuck_testkit::determinism::check(|| {
        let path = run(|ctx| openchuck_path::generate(&strategy, &rig.input(), &v, ctx)).unwrap();
        format!("{:?} {:?}", path.group, path.notices).into_bytes()
    })
    .unwrap();
    assert_eq!(runs.len(), 4);
}

// ---- issue #631: the two pass counts of the notes agree ------------------------------

/// The first number in `text` after `after`.
fn number_after(text: &str, after: &str) -> usize {
    let rest = &text[text
        .find(after)
        .unwrap_or_else(|| panic!("{after:?} in {text}"))
        + after.len()..];
    rest.trim_start()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .unwrap_or_else(|_| panic!("a number after {after:?} in {text}"))
}

#[test]
fn req_op_28_req_nfr_07_the_summary_counts_the_passes_the_face_note_says_cut() {
    // On the channel, the passes along the walls reach no part of the floor
    // the ball can touch: they are planned but cut nothing.
    let rig = Rig::new(channel(4.0), &ball(6.0));
    let out = rig
        .output(&rig.strategy(&[0], &[]), &values(0.5, &[]))
        .unwrap();
    let notes = out.notes();
    let summary = notes
        .iter()
        .find(|n| n.starts_with("cuts "))
        .unwrap_or_else(|| panic!("{notes:?}"));
    let face = notes
        .iter()
        .find(|n| n.starts_with("face 0 is cut from its guide"))
        .unwrap_or_else(|| panic!("{notes:?}"));
    let cut = number_after(summary, "cuts");
    let planned = number_after(face, "towards the opposite side in");
    assert!(cut < planned, "{summary} / {face}");
    // The face note says how many of its planned passes cut, and that is
    // the summary's number: the two read as one account.
    assert!(
        face.contains(&format!("{cut} of them cut")),
        "{summary} / {face}"
    );
    // The summary counts passes that cut, one per line across the face,
    // however many pieces the unreached stretches split it into.
    let lines: std::collections::BTreeSet<i64> = out
        .passes
        .iter()
        .map(|p| (middle(p).y * 1000.0).round() as i64)
        .collect();
    assert_eq!(lines.len(), cut, "{summary}");
}

// ---- stay down between passes (the owner's Q2 of 23 Nov 2026) -------------------------

/// The length of each move of `pass`, by its contacts.
fn steps_of(pass: &Pass) -> Vec<f64> {
    pass.points
        .windows(2)
        .map(|w| w[0].contact.distance(w[1].contact))
        .collect()
}

#[test]
fn req_op_28_req_op_14_meander_passes_stay_down_on_the_face() {
    let (model, height) = twisted(30.0, 6.0, [10.0, 12.0, 11.0, 10.0]);
    let rig = Rig::new(model, &ball(6.0));
    let stepover = 0.8;
    let apart = rig.passes(&[0], &[], &values(stepover, &[]));
    let joined = rig.passes(&[0], &[], &values(stepover, &[(p::STAY_DOWN, flag(true))]));
    // One run: the tool lifts only at the end.
    assert!(apart.len() >= 15, "{} passes", apart.len());
    assert_eq!(joined.len(), 1, "{} runs", joined.len());
    // Every point of the run, the steps' too, is on the face.
    for c in &joined[0].points {
        let q = c.contact;
        assert!(
            (q.z - height(q.x, q.y)).abs() < 0.005,
            "{q:?} is off the face"
        );
        assert!(
            q.x > -1e-6 && q.x < 30.0 + 1e-6 && q.y.abs() < 6.0 + 1e-6,
            "{q:?}"
        );
    }
    // The passes are all there, in the same places; between them only steps
    // along the face's ends, no longer than the stepover allows.
    let total: usize = apart.iter().map(|p| p.points.len()).sum();
    assert!(
        joined[0].points.len() > total,
        "{} points",
        joined[0].points.len()
    );
    for p in &apart {
        for c in &p.points {
            assert!(
                joined[0]
                    .points
                    .iter()
                    .any(|j| j.contact.distance(c.contact) < 1e-9),
                "a pass point {:?} is missing",
                c.contact
            );
        }
    }
    let longest = steps_of(&joined[0]).into_iter().fold(0.0, f64::max);
    assert!(longest <= 2.0 * stepover + 0.01, "a move of {longest} mm");
    // The steps run along the face's ends, X 0 and X 30.
    let at_ends = joined[0]
        .points
        .iter()
        .filter(|c| c.contact.x < 1e-3 || c.contact.x > 30.0 - 1e-3)
        .count();
    assert!(
        at_ends >= 2 * (apart.len() - 1),
        "{at_ends} points at the ends"
    );
}

#[test]
fn req_op_28_req_op_14_climb_and_conventional_lift_between_passes() {
    // Their passes all run the same way, so a pass ends where the next does
    // not start: there is no step to make.
    let (model, _) = twisted(30.0, 6.0, [10.0, 12.0, 11.0, 10.0]);
    let rig = Rig::new(model, &ball(6.0));
    for direction in [p::CLIMB, p::CONVENTIONAL] {
        let off = rig.passes(&[0], &[], &values(0.8, &[(p::DIRECTION, text(direction))]));
        let on = rig.passes(
            &[0],
            &[],
            &values(
                0.8,
                &[(p::DIRECTION, text(direction)), (p::STAY_DOWN, flag(true))],
            ),
        );
        assert_eq!(off, on, "{direction}");
    }
}

/// Checks the ball's centre stays out of the channel's floor and walls at
/// every point and at the middle and quarter points of every move.
fn check_channel(rig: &Rig, passes: &[Pass], half: f64, end_wall: bool) {
    let tol = 0.005;
    let check = |c: &CutPoint| {
        let o = centre_of(rig, c, 3.0);
        assert!(o.z - 10.0 >= 3.0 - tol, "into the floor at {o:?}");
        if o.z < 16.0 + 3.0 {
            assert!(o.y.abs() <= half - 3.0 + tol, "into a wall at {o:?}");
            if end_wall {
                assert!(o.x <= 30.0 - 3.0 + tol, "into the end wall at {o:?}");
            }
        }
    };
    for pass in passes {
        for c in &pass.points {
            check(c);
        }
        for w in pass.points.windows(2) {
            for f in [0.25, 0.5, 0.75] {
                check(&between(rig, &w[0], &w[1], f));
            }
        }
    }
}

#[test]
fn req_op_28_req_op_12_staying_down_between_walls_never_gouges() {
    let half = 4.0;
    let rig = Rig::new(channel(half), &ball(6.0));
    let on = values(0.5, &[(p::STAY_DOWN, flag(true))]);
    let joined = rig.passes(&[0], &[], &on);
    // The open ends at X 0 and 30 are clear, so the passes that cut join.
    assert_eq!(joined.len(), 1, "{} runs", joined.len());
    check_channel(&rig, &joined, half, false);
}

#[test]
fn req_op_28_req_op_12_a_wall_at_a_pass_end_makes_the_tool_lift_there() {
    // With a wall across the end at X 30, no pass reaches that end, so
    // there the tool lifts; at the open end, X 0, it stays down.
    let half = 4.0;
    let rig = Rig::new(channel_with_end_wall(half), &ball(6.0));
    let apart = rig.passes(&[0], &[], &values(0.5, &[]));
    let joined = rig.passes(&[0], &[], &values(0.5, &[(p::STAY_DOWN, flag(true))]));
    let n = apart.len();
    assert!(n >= 3, "{n} passes");
    // The first pass runs from X 0 towards the wall, so the joins at X 0 are
    // after the second pass, the fourth, and so on.
    assert_eq!(
        joined.len(),
        n - (n - 1) / 2,
        "{} runs of {n} passes",
        joined.len()
    );
    check_channel(&rig, &joined, half, true);
}

#[test]
fn req_op_28_req_gc_07_staying_down_is_the_same_at_1_2_and_n_threads() {
    let rig = Rig::new(channel(4.0), &ball(6.0));
    let strategy = rig.strategy(&[0], &[]);
    let v = values(0.5, &[(p::STAY_DOWN, flag(true))]);
    let runs = openchuck_testkit::determinism::check(|| {
        let path = run(|ctx| openchuck_path::generate(&strategy, &rig.input(), &v, ctx)).unwrap();
        format!("{:?} {:?} {:?}", path.group, path.notices, path.unreached).into_bytes()
    })
    .unwrap();
    assert_eq!(runs.len(), 4);
}

// ---- follow the outline (the owner's Q1 of 23 Nov 2026: the pattern is automatic) ----

/// A flat comb at Z 10: a base from X 0 to 30, Y -10 to -2, with two fingers
/// 5 mm wide reaching to Y 10 (X 5 to 10 and X 20 to 25), as the blisk's
/// outer face's lands are fingers of its rim. Its outline, counter-clockwise.
const COMB: [(f64, f64); 12] = [
    (0.0, -10.0),
    (30.0, -10.0),
    (30.0, -2.0),
    (25.0, -2.0),
    (25.0, 10.0),
    (20.0, 10.0),
    (20.0, -2.0),
    (10.0, -2.0),
    (10.0, 10.0),
    (5.0, 10.0),
    (5.0, -2.0),
    (0.0, -2.0),
];

fn comb() -> BrepModel {
    let mut b = Builder::default();
    flat(&mut b, 10.0, &COMB, &[]);
    b.model
}

/// The distance from (x, y) to the comb's outline.
fn to_comb_outline(x: f64, y: f64) -> f64 {
    let outline: Vec<Point3> = COMB
        .iter()
        .chain(COMB.first())
        .map(|&(u, v)| p3(u, v, 10.0))
        .collect();
    to_polyline(p3(x, y, 10.0), &outline)
}

#[test]
fn req_op_28_a_face_with_fingers_is_cut_along_its_outline_in_few_runs() {
    let rig = Rig::new(comb(), &ball(3.0));
    let stepover = 0.8;
    let v = values(
        stepover,
        &[(p::STAY_DOWN, flag(true)), (p::PATTERN, text(p::AUTOMATIC))],
    );
    let out = rig
        .output(&rig.strategy(&[0], &[]), &v)
        .unwrap_or_else(|e| panic!("refused: {e}"));
    let notes = out.notes().join("\n");
    assert!(notes.contains("follows its outline"), "{notes}");
    // Every contact is on the face.
    let all: Vec<Point3> = out.passes.iter().flat_map(contacts).collect();
    for q in &all {
        assert!((q.z - 10.0).abs() < 1e-6, "{q:?}");
        assert!(
            inside_polygon(&COMB, q.x, q.y) || to_comb_outline(q.x, q.y) < 0.01,
            "{q:?} is off the face"
        );
    }
    // The face is covered: every point of it is within a stepover of a
    // contact (where offsets from both sides meet, the last pass can be up
    // to a stepover from the middle), the fingers' tips too.
    let mut worst: f64 = 0.0;
    let mut at = (0.0, 0.0);
    for i in 0..=60 {
        for j in 0..=40 {
            let (x, y) = (
                30.0 * f64::from(i) / 60.0,
                -10.0 + 20.0 * f64::from(j) / 40.0,
            );
            if !inside_polygon(&COMB, x, y) {
                continue;
            }
            let d = all
                .iter()
                .map(|q| (q.x - x).hypot(q.y - y))
                .fold(f64::INFINITY, f64::min);
            if d > worst {
                worst = d;
                at = (x, y);
            }
        }
    }
    assert!(
        worst <= stepover + 0.1,
        "a point at {at:?} is {worst} mm from the nearest contact"
    );
    // The first pass follows the outline, round the fingers and their tips.
    let first = contacts(&out.passes[0]);
    assert!(
        first.iter().filter(|q| q.y > 9.9).count() >= 10,
        "the fingers' tips are not followed"
    );
    // Few runs: the tool stays down from one offset to the next where it can.
    assert!(out.passes.len() <= 4, "{} runs", out.passes.len());
    // Along the guide, the same face's passes break at the notches.
    let blend = rig.passes(
        &[0],
        &[],
        &values(
            stepover,
            &[
                (p::STAY_DOWN, flag(true)),
                (p::PATTERN, text(p::ALONG_GUIDE)),
            ],
        ),
    );
    assert!(
        blend.len() > 4 * out.passes.len(),
        "{} runs along the guide",
        blend.len()
    );
}

#[test]
fn req_op_28_a_plain_face_keeps_the_passes_along_its_guide() {
    // Automatic keeps the guide's passes where they stay whole: the
    // trapezoid's passes are as before.
    let rig = Rig::new(trapezoid(), &ball(6.0));
    let auto = rig.passes(&[0], &[], &values(0.7, &[]));
    let guide = rig.passes(
        &[0],
        &[],
        &values(0.7, &[(p::PATTERN, text(p::ALONG_GUIDE))]),
    );
    assert_eq!(auto, guide);
    let full = rig.passes(&[0], &[], &values(0.7, &[(p::PATTERN, text(p::AUTOMATIC))]));
    assert_eq!(auto, full);
    // A full cylinder keeps its rings with Automatic.
    let rig = Rig::new(full_cylinder(10.0, 20.0), &ball(6.0));
    let notes = rig
        .output(
            &rig.strategy(&[0], &[]),
            &values(1.0, &[(p::PATTERN, text(p::AUTOMATIC))]),
        )
        .unwrap()
        .notes()
        .join("\n");
    assert!(notes.contains("is cut from its guide"), "{notes}");
}

#[test]
fn req_op_28_follow_the_outline_offsets_a_rectangle_inwards_at_the_stepover() {
    // Chosen by the user on a plain rectangle: closed loops, each the
    // stepover inside the last, measured on the face.
    let mut b = Builder::default();
    flat(
        &mut b,
        10.0,
        &[(0.0, 0.0), (20.0, 0.0), (20.0, 12.0), (0.0, 12.0)],
        &[],
    );
    let rig = Rig::new(b.model, &ball(3.0));
    let stepover = 1.0;
    let passes = rig.passes(
        &[0],
        &[],
        &values(stepover, &[(p::PATTERN, text(p::OUTLINE))]),
    );
    // Each pass is a loop at a constant distance from the outline.
    let edge = |q: &Point3| q.x.min(20.0 - q.x).min(q.y).min(12.0 - q.y);
    let mut levels: Vec<f64> = Vec::new();
    for pass in &passes {
        let c = contacts(pass);
        let d: Vec<f64> = c.iter().map(edge).collect();
        let lo = d.iter().copied().fold(f64::INFINITY, f64::min);
        // Away from the corners every point of a loop is at its level.
        let mid = d.iter().filter(|x| (*x - lo).abs() < 0.12).count();
        assert!(mid * 10 >= d.len() * 8, "a loop strays from its level");
        levels.push(lo);
    }
    levels.sort_by(f64::total_cmp);
    assert!(levels[0] < 0.05, "{levels:?}");
    for w in levels.windows(2) {
        assert!(
            w[1] - w[0] <= stepover * 1.05 + 0.1 && w[1] - w[0] > 0.5 * stepover,
            "{levels:?}"
        );
    }
    assert!(*levels.last().unwrap() > 5.0 - stepover, "{levels:?}");
}

#[test]
fn req_op_28_req_gc_07_following_the_outline_is_the_same_at_1_2_and_n_threads() {
    let rig = Rig::new(comb(), &ball(3.0));
    let strategy = rig.strategy(&[0], &[]);
    let v = values(
        0.8,
        &[(p::STAY_DOWN, flag(true)), (p::PATTERN, text(p::AUTOMATIC))],
    );
    let runs = openchuck_testkit::determinism::check(|| {
        let path = run(|ctx| openchuck_path::generate(&strategy, &rig.input(), &v, ctx)).unwrap();
        format!("{:?} {:?} {:?}", path.group, path.notices, path.unreached).into_bytes()
    })
    .unwrap();
    assert_eq!(runs.len(), 4);
}

// ---- joining rings (the owner's decision of 23 Nov 2026: as the helical rings,
// a step along the seam or a blend over Blend over degrees; decision 0545's
// parameters) ----

use openchuck_path::helical::parameter::{BLENDED, RING_BLEND_ANGLE, RING_STEP, SHARP};

/// A along a run, in order, in degrees.
fn a_of(pass: &Pass) -> Vec<f64> {
    pass.points.iter().map(|c| c.a.0).collect()
}

/// Whether A never reverses along `a` (a turn one way, with steps at a
/// held A allowed).
fn one_way(a: &[f64]) -> bool {
    let d: Vec<f64> = a
        .windows(2)
        .map(|w| w[1] - w[0])
        .filter(|d| d.abs() > 1e-6)
        .collect();
    d.iter().all(|x| *x > 0.0) || d.iter().all(|x| *x < 0.0)
}

fn ring_values(
    join: &str,
    direction: &str,
    extra: &[(&str, openchuck_path::ParameterValue)],
) -> BTreeMap<String, openchuck_path::ParameterValue> {
    let mut v = values(
        2.0,
        &[
            (p::STAY_DOWN, flag(true)),
            (RING_STEP, text(join)),
            (p::DIRECTION, text(direction)),
        ],
    );
    for (k, x) in extra {
        v.insert((*k).to_owned(), x.clone());
    }
    v
}

#[test]
fn req_op_28_req_op_14_blended_rings_are_whole_turns_joined_by_diagonal_blends() {
    for (what, model) in [
        ("cylinder", full_cylinder(10.0, 20.0)),
        ("cone", full_cone(10.0, 0.0, 15.0)),
    ] {
        let rig = Rig::new(model, &ball(6.0));
        let rings = rings(&rig.passes(&[0], &[], &values(2.0, &[]))).len();
        for direction in [p::MEANDER, p::CLIMB, p::CONVENTIONAL] {
            for phi in [45.0, 90.0] {
                let out = rig
                    .output(
                        &rig.strategy(&[0], &[]),
                        &ring_values(BLENDED, direction, &[(RING_BLEND_ANGLE, num(phi))]),
                    )
                    .unwrap_or_else(|e| panic!("{what} {direction}: refused: {e}"));
                assert_eq!(out.passes.len(), 1, "{what} {direction} {phi}: one run");
                let a = a_of(&out.passes[0]);
                assert!(one_way(&a), "{what} {direction} {phi}: A reverses");
                // Whole turns, each blend φ more: 360 per ring and φ per blend.
                let total = (a.last().unwrap() - a[0]).abs();
                let want = 360.0 * rings as f64 + phi * (rings - 1) as f64;
                assert!(
                    (total - want).abs() < 1.0,
                    "{what} {direction} {phi}: A turns {total}, not {want}"
                );
                // X never goes back, and the last ring (no blend after it)
                // turns a whole turn at its place.
                let c = contacts(&out.passes[0]);
                // (to within the contact solve, 1e-4 mm)
                let back = c.windows(2).map(|w| w[0].x - w[1].x).fold(0.0, f64::max);
                assert!(back < 1e-4, "{what} {direction}: X goes back {back}");
                let end = c.last().unwrap().x;
                let last: Vec<f64> = out.passes[0]
                    .points
                    .iter()
                    .filter(|q| (q.contact.x - end).abs() < 1e-3)
                    .map(|q| q.a.0)
                    .collect();
                let span = last.iter().copied().fold(f64::NEG_INFINITY, f64::max)
                    - last.iter().copied().fold(f64::INFINITY, f64::min);
                assert!(
                    (span - 360.0).abs() < 1.0,
                    "{what} {direction}: the last ring turns {span}"
                );
                // Over a blend X and A move together: no point of the run
                // moves in X with A held.
                for w in out.passes[0].points.windows(2) {
                    let dx = (w[1].contact.x - w[0].contact.x).abs();
                    if dx > 1e-4 {
                        assert!(
                            (w[1].a.0 - w[0].a.0).abs() > 1e-3,
                            "{what} {direction}: a step with A held"
                        );
                    }
                }
                let notes = out.notes().join("\n");
                assert!(
                    notes.contains(&format!("blended over {phi}")),
                    "{what}: {notes}"
                );
            }
        }
    }
}

#[test]
fn req_op_28_req_op_14_sharp_ring_steps_go_along_the_seam_and_rings_never_reverse() {
    let rig = Rig::new(full_cylinder(10.0, 20.0), &ball(6.0));
    let n = rings(&rig.passes(&[0], &[], &values(2.0, &[]))).len();
    for direction in [p::MEANDER, p::CLIMB, p::CONVENTIONAL] {
        let out = rig
            .output(
                &rig.strategy(&[0], &[]),
                &ring_values(SHARP, direction, &[]),
            )
            .unwrap_or_else(|e| panic!("{direction}: refused: {e}"));
        assert_eq!(out.passes.len(), 1, "{direction}: one run");
        let points = &out.passes[0].points;
        // Every move along X is a step at the seam with A held.
        for w in points.windows(2) {
            if (w[1].contact.x - w[0].contact.x).abs() > 1e-4 {
                assert!(
                    (w[1].a.0 - w[0].a.0).abs() < 1e-6,
                    "{direction}: A turns on a step"
                );
                assert!(
                    angle(w[0].contact).abs() < 1e-3,
                    "{direction}: a step off the seam"
                );
            }
        }
        let a = a_of(&out.passes[0]);
        let total = (a.last().unwrap() - a[0]).abs();
        // Rings never reverse (the owner's correction of 23 Nov 2026):
        // every ring the same way, a whole turn each, Meander too.
        assert!(one_way(&a), "{direction}: A reverses");
        assert!(
            (total - 360.0 * n as f64).abs() < 1.0,
            "{direction}: {total}"
        );
        if direction == p::MEANDER {
            let notes = out.notes().join("\n");
            assert!(
                notes.contains("every ring still runs the same way round"),
                "{notes}"
            );
        }
    }
}

#[test]
fn req_op_28_req_op_12_ring_joins_leave_no_strip_and_never_gouge() {
    let (radius, length, stepover) = (10.0, 20.0, 2.0);
    let rig = Rig::new(full_cylinder(radius, length), &ball(6.0));
    for join in [SHARP, BLENDED] {
        for direction in [p::MEANDER, p::CLIMB] {
            let passes = rig.passes(&[0], &[], &ring_values(join, direction, &[]));
            let all: Vec<Point3> = passes.iter().flat_map(contacts).collect();
            for q in &all {
                assert!(
                    (q.y.hypot(q.z) - radius).abs() < 0.005,
                    "{join} {direction}: {q:?}"
                );
                assert!(
                    q.x > -1e-6 && q.x < length + 1e-6,
                    "{join} {direction}: {q:?}"
                );
            }
            // Every point of the face is within half the stepover (and a
            // sample) of a contact, along the seam too.
            let mut worst: f64 = 0.0;
            for i in 0..=40 {
                for j in 0..72 {
                    let (x, t) = (
                        length * f64::from(i) / 40.0,
                        std::f64::consts::TAU * f64::from(j) / 72.0,
                    );
                    let q = p3(x, radius * t.cos(), radius * t.sin());
                    let d = all
                        .iter()
                        .map(|c| c.distance(q))
                        .fold(f64::INFINITY, f64::min);
                    worst = worst.max(d);
                }
            }
            assert!(
                worst <= 0.5 * stepover + 0.3,
                "{join} {direction}: a point {worst} mm from the nearest contact"
            );
        }
    }
}

#[test]
fn req_op_28_req_nfr_07_blend_over_out_of_range_is_refused_by_name() {
    let rig = Rig::new(full_cylinder(10.0, 20.0), &ball(6.0));
    for phi in [0.0, -10.0, 181.0, f64::NAN] {
        let e = rig
            .output(
                &rig.strategy(&[0], &[]),
                &ring_values(BLENDED, p::MEANDER, &[(RING_BLEND_ANGLE, num(phi))]),
            )
            .err()
            .unwrap_or_else(|| panic!("{phi} degrees accepted"));
        assert!(
            e.to_string().contains(RING_BLEND_ANGLE) || e.to_string().contains("Blend over"),
            "{phi}: {e}"
        );
    }
}

#[test]
fn req_op_28_req_gc_07_ring_joins_are_the_same_at_1_2_and_n_threads() {
    let rig = Rig::new(full_cone(10.0, 0.0, 15.0), &ball(6.0));
    let strategy = rig.strategy(&[0], &[]);
    for join in [SHARP, BLENDED] {
        let v = ring_values(join, p::CLIMB, &[]);
        openchuck_testkit::determinism::check(|| {
            let path =
                run(|ctx| openchuck_path::generate(&strategy, &rig.input(), &v, ctx)).unwrap();
            format!("{:?} {:?} {:?}", path.group, path.notices, path.unreached).into_bytes()
        })
        .unwrap();
    }
}

#[test]
fn req_op_28_face_flow_offers_the_ring_steps_of_the_ring_strategies() {
    let rig = Rig::new(full_cylinder(10.0, 20.0), &ball(6.0));
    let schema = rig.strategy(&[0], &[]).schema();
    let step = schema.get(RING_STEP).expect("Steps between rings");
    assert_eq!(step.label, "Steps between rings");
    let blend = schema.get(RING_BLEND_ANGLE).expect("Blend over");
    assert_eq!(blend.label, "Blend over");
    // A saved operation without them keeps sharp steps and does not stay
    // down (decision 0545 D3's pattern); a new one is written with both.
    match &step.kind {
        openchuck_path::params::ParameterKind::Choice { default, .. } => assert_eq!(default, SHARP),
        other => panic!("{other:?}"),
    }
    match &schema.get(p::STAY_DOWN).expect("Stay down").kind {
        openchuck_path::params::ParameterKind::Flag { default } => assert!(!default),
        other => panic!("{other:?}"),
    }
    match &schema.get(p::PATTERN).expect("Pattern").kind {
        openchuck_path::params::ParameterKind::Choice { default, .. } => {
            assert_eq!(default, p::ALONG_GUIDE)
        }
        other => panic!("{other:?}"),
    }
    // Shortest cut is a length or the tool's diameter, 0 for a saved
    // operation (a new one is written with Tool diameter).
    match &schema.get(p::SHORTEST_CUT).expect("Shortest cut").kind {
        openchuck_path::params::ParameterKind::LengthOrTool { default, min, .. } => {
            assert_eq!(
                *default,
                openchuck_path::params::LengthOrTool::Millimetres(0.0)
            );
            assert_eq!(*min, Some(0.0));
        }
        other => panic!("{other:?}"),
    }
}

// ---- Shortest cut (the owner's answer of 23 Nov 2026 to Q3: applied last, to
// whole runs; default the tool's diameter, 0 off) ----

fn run_length(pass: &Pass) -> f64 {
    contacts(pass).windows(2).map(|w| w[0].distance(w[1])).sum()
}

#[test]
fn req_op_28_req_op_20_runs_shorter_than_the_shortest_cut_are_left_out_and_reported() {
    // The comb along its guide: runs from 8 mm long, broken at the notches.
    let rig = Rig::new(comb(), &ball(3.0));
    let with = |shortest: Option<f64>| {
        let mut v = values(
            0.8,
            &[
                (p::STAY_DOWN, flag(true)),
                (p::PATTERN, text(p::ALONG_GUIDE)),
            ],
        );
        if let Some(x) = shortest {
            v.insert(p::SHORTEST_CUT.to_owned(), num(x));
        }
        rig.output(&rig.strategy(&[0], &[]), &v).unwrap()
    };
    let every = with(Some(0.0));
    assert!(
        every
            .unreached
            .iter()
            .all(|u| !matches!(u.reason, UnreachedReason::TooShort { .. }))
    );
    // An operation saved without it keeps every run (its program is
    // unchanged); a new operation is written with the tool's diameter.
    assert_eq!(
        format!("{:?}", with(None).passes),
        format!("{:?}", every.passes)
    );
    let limit = 9.0;
    let short: Vec<&Pass> = every
        .passes
        .iter()
        .filter(|p| run_length(p) < limit)
        .collect();
    assert!(
        !short.is_empty() && short.len() < every.passes.len(),
        "{} short runs",
        short.len()
    );
    let out = with(Some(limit));
    // Nothing inside a run is removed: the runs kept are exactly the long
    // ones, point for point.
    let long: Vec<String> = every
        .passes
        .iter()
        .filter(|p| run_length(p) >= limit)
        .map(|p| format!("{:?}", p.points))
        .collect();
    let kept: Vec<String> = out
        .passes
        .iter()
        .map(|p| format!("{:?}", p.points))
        .collect();
    assert_eq!(kept, long);
    // Each run left out is reported as an area left uncut, with its place
    // and length, and named in a warning.
    let dropped: Vec<_> = out
        .unreached
        .iter()
        .filter(|u| matches!(u.reason, UnreachedReason::TooShort { .. }))
        .collect();
    assert_eq!(dropped.len(), short.len());
    for (u, p) in dropped.iter().zip(&short) {
        let UnreachedReason::TooShort { length } = u.reason else {
            unreachable!()
        };
        assert!((length - run_length(p)).abs() < 1e-6, "{length}");
        let xs: Vec<f64> = contacts(p).iter().map(|q| q.x).collect();
        let lo = xs.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        assert!(
            (u.x.0 - lo).abs() < 1e-6 && (u.x.1 - hi).abs() < 1e-6,
            "{:?}",
            u.x
        );
        assert!(!u.points.is_empty());
    }
    let warnings: Vec<&str> = out
        .notices
        .iter()
        .filter(|n| n.is_warning())
        .map(|n| n.text.as_str())
        .collect();
    let warnings = warnings.join("\n");
    assert!(
        warnings.contains("shorter than the Shortest cut"),
        "{warnings}"
    );
}

#[test]
fn req_op_28_req_nfr_07_a_negative_shortest_cut_is_refused_by_name() {
    let rig = Rig::new(comb(), &ball(3.0));
    for bad in [-1.0, f64::NAN] {
        let e = rig
            .output(
                &rig.strategy(&[0], &[]),
                &values(0.8, &[(p::SHORTEST_CUT, num(bad))]),
            )
            .err()
            .unwrap_or_else(|| panic!("{bad} accepted"));
        assert!(e.to_string().contains(p::SHORTEST_CUT), "{bad}: {e}");
    }
}

#[test]
fn req_op_28_a_shortest_cut_of_tool_diameter_is_the_tools_diameter() {
    // The owner's decision of 23 Nov 2026: Tool diameter is a choice of its
    // own, read from the operation's tool when it is generated.
    // The comb's runs along its guide are 8 to 10 mm long or more, so a 6 mm
    // ball keeps them all and a 10 mm ball leaves the shorter ones out: the
    // token visibly follows the tool (issue #896 item 5).
    let word = text(openchuck_path::params::TOOL_DIAMETER);
    let mut kept = Vec::new();
    for d in [6.0, 10.0] {
        let rig = Rig::new(comb(), &ball(d));
        let run_with = |x: openchuck_path::ParameterValue| {
            let v = values(
                0.8,
                &[
                    (p::STAY_DOWN, flag(true)),
                    (p::PATTERN, text(p::ALONG_GUIDE)),
                    (p::SHORTEST_CUT, x),
                ],
            );
            rig.passes(&[0], &[], &v)
        };
        let by_word = run_with(word.clone());
        assert_eq!(
            format!("{by_word:?}"),
            format!("{:?}", run_with(num(d))),
            "a {d} mm ball"
        );
        let every = run_with(num(0.0));
        assert!(
            every.iter().any(|p| (6.0..10.0).contains(&run_length(p))),
            "a run between the two diameters"
        );
        kept.push(by_word.len());
    }
    assert!(
        kept[1] < kept[0],
        "runs kept by 6 and 10 mm balls: {kept:?}"
    );
}

/// A flat 20 by 20 mm square at Z 10 with a round bite of radius 5 taken
/// out of the middle of its top side: a boundary that curves into the face.
/// Returns the model and its outline in (x, y), counter-clockwise.
fn bitten() -> (BrepModel, Vec<(f64, f64)>) {
    let z = 10.0;
    let mut b = Builder::default();
    let arc = |t: f64| {
        // From (15, 20) round under the centre (10, 20) to (5, 20).
        let a = PI * t;
        p3(10.0 + 5.0 * a.cos(), 20.0 - 5.0 * a.sin(), z)
    };
    let line = |a: (f64, f64), c: (f64, f64)| {
        move |t: f64| p3(a.0 + (c.0 - a.0) * t, a.1 + (c.1 - a.1) * t, z)
    };
    let edges = vec![
        b.edge(&[0], &line((0.0, 0.0), (20.0, 0.0)), true),
        b.edge(&[0], &line((20.0, 0.0), (20.0, 20.0)), true),
        b.edge(&[0], &line((20.0, 20.0), (15.0, 20.0)), true),
        b.edge(&[0], &arc, false),
        b.edge(&[0], &line((5.0, 20.0), (0.0, 20.0)), true),
        b.edge(&[0], &line((0.0, 20.0), (0.0, 0.0)), true),
    ];
    let mut outline = vec![(0.0, 0.0), (20.0, 0.0), (20.0, 20.0)];
    outline.extend((0..=256).map(|i| {
        let q = arc(f64::from(i) / 256.0);
        (q.x, q.y)
    }));
    outline.push((0.0, 20.0));
    let polygon = outline.clone();
    b.face(
        Surface::Plane {
            frame: frame(
                p3(0.0, 0.0, z),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ),
        },
        [0.0, 20.0, 0.0, 20.0],
        Some(outline.clone()),
        edges,
        &|u, v| p3(u, v, z),
        &move |u, v| inside_polygon(&polygon, u, v),
        false,
    );
    (b.model, outline)
}

#[test]
fn req_op_28_following_the_outline_stays_on_a_face_whose_boundary_curves_into_it() {
    // Issue #896 item 4: the outline itself (level 0) is the face's own
    // edges, joined between their samples. Where the boundary curves into
    // the face, a chord between two samples leaves it by the sagitta. Every
    // contact of every loop is on the face or within 0.005 mm of its edge.
    let (model, outline) = bitten();
    let rig = Rig::new(model, &ball(3.0));
    let passes = rig.passes(&[0], &[], &values(1.0, &[(p::PATTERN, text(p::OUTLINE))]));
    assert!(passes.len() > 3, "{} loops", passes.len());
    let ring: Vec<Point3> = outline
        .iter()
        .chain(outline.first())
        .map(|&(x, y)| p3(x, y, 10.0))
        .collect();
    for q in passes.iter().flat_map(contacts) {
        assert!((q.z - 10.0).abs() < 1e-6, "{q:?}");
        assert!(
            inside_polygon(&outline, q.x, q.y) || to_polyline(q, &ring) < 0.005,
            "{q:?} is off the face by {}",
            to_polyline(q, &ring)
        );
    }
}

#[test]
fn req_op_28_req_op_25_following_the_outline_climb_and_conventional_run_opposite_ways() {
    // Climb and Conventional keep the uncut material, inside the face, on the
    // side they ask for: every loop of one runs round the same way, and the
    // two run round opposite ways (decision 0686 item 2; decision 0213).
    let mut b = Builder::default();
    flat(
        &mut b,
        10.0,
        &[(0.0, 0.0), (20.0, 0.0), (20.0, 12.0), (0.0, 12.0)],
        &[],
    );
    let rig = Rig::new(b.model, &ball(3.0));
    // The signed area of each pass's contacts seen from +Z: the way it runs
    // round, positive counter-clockwise.
    let ways = |direction: &str| -> Vec<f64> {
        let v = values(
            1.0,
            &[
                (p::PATTERN, text(p::OUTLINE)),
                (p::DIRECTION, text(direction)),
            ],
        );
        rig.passes(&[0], &[], &v)
            .iter()
            .map(|pass| {
                let c = contacts(pass);
                c.windows(2)
                    .map(|w| w[0].x * w[1].y - w[1].x * w[0].y)
                    .sum::<f64>()
            })
            .filter(|area| area.abs() > 1.0)
            .collect()
    };
    let climb = ways(p::CLIMB);
    let conventional = ways(p::CONVENTIONAL);
    assert!(
        climb.len() > 2 && conventional.len() == climb.len(),
        "{climb:?} {conventional:?}"
    );
    let sign = climb[0].signum();
    assert!(climb.iter().all(|a| a.signum() == sign), "{climb:?}");
    assert!(
        conventional.iter().all(|a| a.signum() == -sign),
        "{conventional:?}"
    );
    // Meander alternates.
    let meander = ways(p::MEANDER);
    assert!(
        meander.windows(2).all(|w| w[0].signum() != w[1].signum()),
        "{meander:?}"
    );
}

#[test]
fn req_op_28_req_nfr_07_the_outline_note_counts_the_loops_the_shortest_cut_leaves() {
    // CodeRabbit on #854: where Shortest cut leaves out some of the loops,
    // the face note says how many still cut, as along the guide.
    let mut b = Builder::default();
    flat(
        &mut b,
        10.0,
        &[(0.0, 0.0), (20.0, 0.0), (20.0, 12.0), (0.0, 12.0)],
        &[],
    );
    let rig = Rig::new(b.model, &ball(3.0));
    let out = rig
        .output(
            &rig.strategy(&[0], &[]),
            &values(
                1.0,
                &[
                    (p::PATTERN, text(p::OUTLINE)),
                    (p::STAY_DOWN, flag(false)),
                    (p::SHORTEST_CUT, num(30.0)),
                ],
            ),
        )
        .unwrap();
    let note = out
        .notices
        .iter()
        .map(|n| n.text.as_str())
        .find(|t| t.contains("follows its outline"))
        .expect("the face note");
    let planned = number_after(note, "inwards in ");
    let cut = out.passes.len();
    assert!(cut > 0 && cut < planned, "{note}");
    assert!(
        note.contains(&format!("; {cut} of them cut")),
        "{cut} of {planned}: {note}"
    );
}

#[test]
fn req_op_28_req_nfr_07_a_shortest_cut_that_leaves_nothing_is_refused_naming_it() {
    // CodeRabbit on #854: when Shortest cut leaves out every run, the
    // refusal says so and names the setting, rather than that the faces
    // cannot be reached.
    let rig = Rig::new(comb(), &ball(3.0));
    let v = values(
        0.8,
        &[
            (p::STAY_DOWN, flag(true)),
            (p::PATTERN, text(p::ALONG_GUIDE)),
            (p::SHORTEST_CUT, num(10_000.0)),
        ],
    );
    let e = rig
        .output(&rig.strategy(&[0], &[]), &v)
        .expect_err("every run left out is refused");
    let text = e.to_string();
    assert!(text.contains("Shortest cut"), "{text}");
    assert!(!text.contains("cannot reach"), "{text}");
}

#[test]
fn req_op_28_req_nfr_07_an_outline_needing_too_many_levels_is_refused() {
    // CodeRabbit on #854: a stepover so fine that the outline would need
    // more offsets than the pass limit is refused before they are traced.
    let mut b = Builder::default();
    flat(
        &mut b,
        10.0,
        &[(0.0, 0.0), (20.0, 0.0), (20.0, 12.0), (0.0, 12.0)],
        &[],
    );
    let rig = Rig::new(b.model, &ball(3.0));
    let v = values(0.0001, &[(p::PATTERN, text(p::OUTLINE))]);
    let started = std::time::Instant::now();
    let e = rig
        .output(&rig.strategy(&[0], &[]), &v)
        .expect_err("refused");
    assert!(e.to_string().contains("raise the stepover"), "{e}");
    assert!(started.elapsed().as_secs() < 20, "{:?}", started.elapsed());
}

#[test]
fn req_op_28_req_nfr_07_a_sliver_of_a_cylinder_is_not_taken_for_a_closed_face() {
    // CodeRabbit on #854: a cylinder face whose two straight edges lie within
    // the join distance of each other (a sliver a few millionths of a turn
    // wide) is not a face that closes round on itself: only a span of a whole
    // turn is.
    let (radius, length, span) = (10.0, 20.0, 5e-5);
    let point = move |u: f64, v: f64| p3(v, radius * u.cos(), radius * u.sin());
    let mut b = Builder::default();
    let edges = vec![
        b.edge(&[0], &|t| point(span * t, 0.0), false),
        b.edge(&[0], &|t| point(span, length * t), true),
        b.edge(&[0], &|t| point(span * (1.0 - t), length), false),
        b.edge(&[0], &|t| point(0.0, length * (1.0 - t)), true),
    ];
    b.face(
        Surface::Cylinder {
            frame: about_x(),
            radius,
        },
        [0.0, span, 0.0, length],
        None,
        edges,
        &point,
        &|_, _| true,
        false,
    );
    let rig = Rig::new(b.model, &ball(6.0));
    let said = match rig.output(
        &rig.strategy(&[0], &[]),
        &values(1.0, &[(p::DIRECTION, text(p::MEANDER))]),
    ) {
        Ok(out) => out.notes().join("\n"),
        Err(e) => e.to_string(),
    };
    // Taken for a closed face, its straight edges were read as seams and its
    // boundary did not close.
    assert!(!said.contains("does not close"), "{said}");
    assert!(!said.contains("every ring"), "{said}");
    assert!(!said.contains("closes round on itself"), "{said}");
}

#[test]
fn req_op_28_req_nfr_07_the_pass_counts_agree_after_the_shortest_cut() {
    // CodeRabbit on #854: the first note counts the passes that still cut
    // once Shortest cut has left runs out, and the face's note says the same
    // number (issue #631's agreement).
    let rig = Rig::new(comb(), &ball(3.0));
    let counts = |shortest: f64| {
        let v = values(
            0.8,
            &[
                (p::STAY_DOWN, flag(false)),
                (p::PATTERN, text(p::ALONG_GUIDE)),
                (p::SHORTEST_CUT, num(shortest)),
            ],
        );
        let notes = rig
            .output(&rig.strategy(&[0], &[]), &v)
            .unwrap()
            .notes()
            .join("\n");
        let number = |after: &str, before: &str| -> Option<usize> {
            let i = notes.find(after)? + after.len();
            let j = notes[i..].find(before)? + i;
            notes[i..j].trim().parse().ok()
        };
        let summary = number("cuts ", " pass").expect("summary");
        let planned = number(" in ", " passes up to").expect("planned");
        let cut = number("; ", " of them cut").unwrap_or(planned);
        (summary, cut, notes)
    };
    let (all, all_face, _) = counts(0.0);
    assert_eq!(all, all_face);
    let (fewer, face, notes) = counts(20.0);
    assert_eq!(fewer, face, "{notes}");
    assert!(fewer < all, "{fewer} against {all}: {notes}");
}

#[test]
fn req_op_28_follow_the_outline_needs_no_guide() {
    // CodeRabbit on #854: Follow the outline takes no guide, so a guide that
    // Along the guide refuses (here a full cylinder's seam, edge 2) does not
    // stop it.
    let rig = Rig::new(full_cylinder(10.0, 20.0), &ball(6.0));
    let guide = rig.output(
        &rig.strategy(&[0], &[2]),
        &values(1.0, &[(p::PATTERN, text(p::ALONG_GUIDE))]),
    );
    assert!(
        guide.is_err(),
        "the seam as guide is refused along the guide"
    );
    let out = rig
        .output(
            &rig.strategy(&[0], &[2]),
            &values(1.0, &[(p::PATTERN, text(p::OUTLINE))]),
        )
        .unwrap_or_else(|e| panic!("refused: {e}"));
    let notes = out.notes().join("\n");
    assert!(notes.contains("face 0 follows its outline"), "{notes}");
    for q in out.passes.iter().flat_map(contacts) {
        assert!((q.y.hypot(q.z) - 10.0).abs() < 0.01, "{q:?}");
    }
}

#[test]
fn req_op_28_following_the_outline_spaces_offsets_along_a_curved_face() {
    // CodeRabbit on #854: the offsets are the stepover apart measured along
    // the face, not in a straight line through the part. A sector of a
    // cylinder of radius 10 mm, a third of a turn round and 40 mm long: far
    // from its straight sides, a straight-line measure puts the offsets about
    // a tenth further apart than the stepover along the face.
    let (radius, length, span) = (10.0, 40.0, std::f64::consts::TAU / 3.0);
    let point = move |u: f64, v: f64| p3(v, radius * u.cos(), radius * u.sin());
    let mut b = Builder::default();
    let edges = vec![
        b.edge(&[0], &|t| point(span * t, 0.0), false),
        b.edge(&[0], &|t| point(span, length * t), true),
        b.edge(&[0], &|t| point(span * (1.0 - t), length), false),
        b.edge(&[0], &|t| point(0.0, length * (1.0 - t)), true),
    ];
    b.face(
        Surface::Cylinder {
            frame: about_x(),
            radius,
        },
        [0.0, span, 0.0, length],
        None,
        edges,
        &point,
        &|_, _| true,
        false,
    );
    let rig = Rig::new(b.model, &ball(1.0));
    let stepover = 1.0;
    let passes = rig.passes(
        &[0],
        &[],
        &values(stepover, &[(p::PATTERN, text(p::OUTLINE))]),
    );
    // Along the middle of the sector's length, each pass crosses the angle
    // round the face; the arc between neighbouring crossings on one side is
    // the spacing along the face.
    let mut angles: Vec<f64> = passes
        .iter()
        .flat_map(contacts)
        .filter(|q| (q.x - 0.5 * length).abs() < 10.0)
        .map(|q| q.z.atan2(q.y))
        .filter(|a| *a < 0.5 * span - 0.05)
        .collect();
    angles.sort_by(f64::total_cmp);
    angles.dedup_by(|a, b| (*a - *b).abs() * radius < 0.2);
    assert!(angles.len() >= 6, "{angles:?}");
    for w in angles.windows(2) {
        let along = (w[1] - w[0]) * radius;
        assert!(
            along <= stepover * 1.05 + 0.02,
            "{along} mm apart along the face: {angles:?}"
        );
    }
}

/// A flat 30 by 10 mm plate at Z 10 with a 2 by 4 mm hole near its left
/// end (X 3 to 5, Y 3 to 7): its passes along the bottom edge run at one Y
/// each, and those crossing the hole are cut in two runs, 3 mm and 25 mm
/// long.
fn plate_with_hole() -> BrepModel {
    let mut b = Builder::default();
    flat(
        &mut b,
        10.0,
        &[(0.0, 0.0), (30.0, 0.0), (30.0, 10.0), (0.0, 10.0)],
        &[],
    );
    let hole = [(3.0, 3.0), (5.0, 3.0), (5.0, 7.0), (3.0, 7.0)];
    b.model.faces[0].trims.push(TrimLoop(
        hole.iter().map(|&(u, v)| UvPoint { u, v }).collect(),
    ));
    let mesh = &mut b.model.faces[0].tessellation;
    let kept: Vec<[u32; 3]> = mesh
        .triangles
        .iter()
        .copied()
        .filter(|t| {
            let c = t
                .iter()
                .fold(Vec3::ZERO, |s, &i| s + mesh.vertices[i as usize].to_vec())
                * (1.0 / 3.0);
            !inside_polygon(&hole, c.x, c.y)
        })
        .collect();
    mesh.triangles = kept;
    b.model
}

#[test]
fn req_op_28_req_op_20_a_pass_with_a_run_left_out_and_a_run_kept_still_counts_as_cutting() {
    // Issue #896 item 8: after Shortest cut, the passes that still cut are
    // counted by their runs' first contacts. On the plate, the 12 passes
    // that cross the hole are each cut in a 2.75 mm run and a 25 mm run.
    let rig = Rig::new(plate_with_hole(), &ball(3.0));
    let run = |stay: bool, shortest: f64| {
        let v = values(
            1.0,
            &[
                (p::STAY_DOWN, flag(stay)),
                (p::PATTERN, text(p::ALONG_GUIDE)),
                (p::SHORTEST_CUT, num(shortest)),
            ],
        );
        rig.output(&rig.strategy(&[0], &[]), &v).unwrap()
    };
    // Each pass's Y (a tenth of a mm is far less than the 0.9 mm between
    // passes), from the runs cut one by one with nothing left out.
    let line = |q: Point3| (q.y * 10.0).round() as i64;
    let every = run(false, 0.0);
    let planned: std::collections::BTreeSet<i64> =
        every.passes.iter().map(|p| line(contacts(p)[0])).collect();
    assert_eq!(planned.len(), 12, "{planned:?}");
    // The passes with a run of 26 mm or more: those without the hole.
    let long: std::collections::BTreeSet<i64> = every
        .passes
        .iter()
        .filter(|p| run_length(p) >= 26.0)
        .map(|p| line(contacts(p)[0]))
        .collect();
    assert!(long.len() < planned.len(), "{long:?}");
    // One by one, a 26 mm Shortest cut leaves out every run of the passes
    // that cross the hole: only the long passes still cut.
    let notes = run(false, 26.0).notes().join("\n");
    assert!(
        notes.contains(&format!("; {} of them cut", long.len())),
        "{notes}"
    );
    // With Stay down, the two short runs of the passes beside the hole's
    // middle are joined into one 6.4 mm pass, which is left out, but each
    // of those passes keeps its 25 mm run, joined into a longer pass: all
    // 12 still cut.
    let joined = run(true, 26.0);
    let kept: std::collections::BTreeSet<i64> = joined
        .passes
        .iter()
        .flat_map(contacts)
        .map(line)
        .filter(|y| planned.contains(y))
        .collect();
    assert_eq!(kept, planned);
    assert!(
        joined
            .unreached
            .iter()
            .any(|u| matches!(u.reason, UnreachedReason::TooShort { .. })),
        "the short pass is reported"
    );
    let notes = joined.notes().join("\n");
    assert!(!notes.contains("of them cut"), "{notes}");
}

// ---- issue #862: a face across the rotary axis --------------------------------------

/// A step: a floor at Z 4 from X 10 to 20 (face 0), and a wall at X 10
/// facing +X from Z 4 to 12 (face 1), across the rotary axis: its normal is
/// along the axis, so the tool, whose axis is always square to it, reaches
/// it only with the side of its tip.
fn step_across_the_axis() -> BrepModel {
    let mut b = Builder::default();
    flat(
        &mut b,
        4.0,
        &[(10.0, -5.0), (20.0, -5.0), (20.0, 5.0), (10.0, 5.0)],
        &[],
    );
    wall(
        &mut b,
        p3(10.0, 5.0, 4.0),
        p3(10.0, -5.0, 4.0),
        8.0,
        Vec3::new(1.0, 0.0, 0.0),
        EdgeId(3),
    );
    b.model
}

#[test]
fn req_op_28_req_op_12_a_face_across_the_rotary_axis_is_cut_with_the_side_of_the_tip() {
    // Issue #862, the owner's answer A: the drop along a tool line parallel
    // to such a face is tangent to it all the way down, so whether it read
    // contact depended on rounding: the tool either stopped above the face
    // (read as blocked) or slid past the point down to the floor. Now each
    // point the tool can reach with its side is cut there, with the ball
    // beside the wall, and the passes are whole.
    let rig = Rig::new(step_across_the_axis(), &ball(6.0));
    let out = rig
        .output(&rig.strategy(&[1], &[]), &values(0.5, &[]))
        .unwrap();
    assert!(!out.passes.is_empty());
    let tol = 0.005;
    let r = 3.0;
    for pass in &out.passes {
        for c in &pass.points {
            let o = centre_of(&rig, c, r);
            assert!(
                (o.x - 10.0 - r).abs() <= tol + 0.001,
                "the ball is {} mm off the wall at {o:?}",
                o.x - 10.0 - r
            );
            assert!(o.z >= 4.0 + r - tol, "into the floor at {o:?}");
        }
        // Nor at the middle and quarter points of each move, by the closed
        // form of the step.
        for w in pass.points.windows(2) {
            for f in [0.25, 0.5, 0.75] {
                let o = centre_of(&rig, &between(&rig, &w[0], &w[1], f), r);
                assert!(o.x >= 10.0 + r - tol, "into the wall at {o:?}");
                assert!(o.z >= 4.0 + r - tol, "into the floor at {o:?}");
            }
        }
        let ys = contacts(pass).iter().map(|q| q.y).collect::<Vec<_>>();
        let span = ys.iter().fold(f64::NEG_INFINITY, |m, y| m.max(*y))
            - ys.iter().fold(f64::INFINITY, |m, y| m.min(*y));
        assert!(span > 9.0, "a pass is a fragment of {span} mm");
    }
    let notes = out.notes().join("\n");
    assert!(
        notes.contains(
            "face 1 lies across the rotary axis: the tool can reach it only with the side of \
             its tip, so the passes on it are cut that way."
        ),
        "{notes}"
    );
    // The floor is not across the axis, and gets no such note.
    let floor = rig
        .output(&rig.strategy(&[0], &[]), &values(0.5, &[]))
        .unwrap();
    assert!(
        !floor.notes().join("\n").contains("across the rotary axis"),
        "{:?}",
        floor.notes()
    );
}
