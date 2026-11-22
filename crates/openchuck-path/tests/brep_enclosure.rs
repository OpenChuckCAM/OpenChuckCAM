// SPDX-License-Identifier: AGPL-3.0-or-later
//! The enclosure of a STEP part from its exact faces (OP-23, OP-24; decision
//! 0666 option C, issue #800): points whose convex hull holds every exact
//! face, so a bound Facing takes of the part in any direction is never
//! inside it, and at most the stated excess outside it. Checked against the
//! faces' closed forms (and a rational Bezier patch evaluated here), never
//! against the enclosure's own arithmetic.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_precision_loss,
    // Bounds are compared exactly where they must be the same, not close.
    clippy::float_cmp
)]

use std::f64::consts::TAU;

use openchuck_path::{Enclosure, enclose};
use openchuck_testkit::ruled;
use openchuck_types::brep::{
    BSplinePatch, BrepEdge, BrepFace, BrepModel, EdgeCurve, EdgeId, FaceId, Frame, Surface,
};
use openchuck_types::geometry::{Point3, Vec3};
use openchuck_types::mesh::TriangleMesh;

/// Directions to take supports in: the six axes, the diagonals and a fixed
/// spread over the sphere.
fn directions() -> Vec<Vec3> {
    let mut out = vec![
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(-1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, -1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 0.0, -1.0),
    ];
    // A Fibonacci spread: deterministic, roughly even.
    let n = 200;
    let golden = std::f64::consts::PI * (3.0 - 5f64.sqrt());
    for i in 0..n {
        let z = 1.0 - 2.0 * (i as f64 + 0.5) / n as f64;
        let r = (1.0 - z * z).sqrt();
        let t = golden * i as f64;
        out.push(Vec3::new(r * t.cos(), r * t.sin(), z));
    }
    out
}

fn support(points: &[Point3], d: Vec3) -> f64 {
    points
        .iter()
        .map(|p| p.to_vec().dot(d))
        .fold(f64::NEG_INFINITY, f64::max)
}

/// `samples` points over the parameter box of a closed-form surface.
fn sample(surface: &Surface, uv: [f64; 4], n: usize) -> Vec<Point3> {
    let mut out = Vec::new();
    for i in 0..=n {
        for j in 0..=n {
            let u = uv[0] + (uv[1] - uv[0]) * i as f64 / n as f64;
            let v = uv[2] + (uv[3] - uv[2]) * j as f64 / n as f64;
            out.push(ruled::point(surface, u, v));
        }
    }
    out
}

/// Every support of the enclosure is at least the surface's, and at most the
/// surface's plus `excess` plus `slack` (how far the samples can fall short
/// of the surface's own support).
fn check(enclosure: &Enclosure, surface: &[Point3], excess: f64, slack: f64, what: &str) {
    for d in directions() {
        let outer = support(&enclosure.points, d);
        let inner = support(surface, d);
        assert!(
            outer >= inner - 1e-9,
            "{what}: the enclosure is inside the face in {d:?}: {outer} < {inner}"
        );
        assert!(
            outer <= inner + excess + slack,
            "{what}: the enclosure is {} past the face in {d:?}, more than {excess} + {slack}",
            outer - inner
        );
    }
}

fn frame(origin: Point3, x: Vec3, z: Vec3) -> Frame {
    Frame {
        origin,
        x,
        y: z.cross(x),
        z,
    }
}

#[test]
fn req_op_23_req_op_24_the_enclosure_holds_each_analytic_face_within_its_stated_excess() {
    let cases: Vec<(&str, BrepModel)> = vec![
        ("cylinder", ruled::coaxial_cylinder(12.0, 5.0, 40.0)),
        ("boss", ruled::boss_cylinder(20.0, 6.0, -3.0, 9.0)),
        ("cone", ruled::draft_cone(10.0, 7.0, 0.3, -4.0, 6.0)),
        ("sphere band", ruled::sphere_band(15.0, 9.0, -0.7, 1.1)),
        ("torus", ruled::torus_band(30.0, 11.0, 3.0)),
        (
            "partial cylinder",
            ruled::single_face(
                Surface::Cylinder {
                    frame: frame(
                        Point3::new(1.0, 2.0, 3.0),
                        Vec3::new(0.0, 1.0, 0.0),
                        Vec3::new(1.0, 0.0, 0.0),
                    ),
                    radius: 15.0,
                },
                [0.4, 2.9, -10.0, 25.0],
                false,
            ),
        ),
    ];
    for (what, model) in cases {
        let enclosure = enclose(&model).unwrap();
        assert!(enclosure.spline_excess.is_none(), "{what}");
        assert!(
            enclosure.analytic_excess > 0.0 && enclosure.analytic_excess <= 0.0005,
            "{what}: {}",
            enclosure.analytic_excess
        );
        let face = &model.faces[0];
        let samples = sample(&face.geometry, face.uv_bounds, 600);
        // 600 steps over at most a full turn of a radius under 15 mm: the
        // samples fall short of the surface by at most r (1 - cos(pi / 600)).
        check(
            &enclosure,
            &samples,
            enclosure.analytic_excess,
            2.5e-4,
            what,
        );
    }
}

/// A rod about X from `x0` to `x1` of `radius`: its side, and two flat ends
/// whose edges are the exact circles, as a STEP import gives them.
fn rod(radius: f64, x0: f64, x1: f64) -> BrepModel {
    let axis = |x: f64| {
        frame(
            Point3::new(x, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
        )
    };
    let circle = |id: u32, x: f64, faces: [u32; 2]| BrepEdge {
        id: EdgeId(id),
        faces: faces.into_iter().map(FaceId).collect(),
        vertices: None,
        polyline: Vec::new(),
        curve: Some(EdgeCurve::Circle {
            frame: axis(x),
            radius,
            first: 0.0,
            last: TAU,
        }),
    };
    let face = |id: u32, geometry: Surface, uv: [f64; 4], edges: Vec<u32>| BrepFace {
        id: FaceId(id),
        surface: geometry.kind(),
        body: 0,
        edges: edges.into_iter().map(EdgeId).collect(),
        geometry,
        reversed: false,
        uv_bounds: uv,
        trims: Vec::new(),
        tessellation: TriangleMesh::default(),
    };
    BrepModel {
        faces: vec![
            face(
                0,
                Surface::Plane { frame: axis(x0) },
                [-radius, radius, -radius, radius],
                vec![0],
            ),
            face(
                1,
                Surface::Cylinder {
                    frame: axis(0.0),
                    radius,
                },
                [0.0, TAU, x0, x1],
                vec![0, 1],
            ),
            face(
                2,
                Surface::Plane { frame: axis(x1) },
                [-radius, radius, -radius, radius],
                vec![1],
            ),
        ],
        edges: vec![circle(0, x0, [0, 1]), circle(1, x1, [1, 2])],
        vertices: Vec::new(),
        tessellation: Default::default(),
    }
}

#[test]
fn req_op_23_req_op_24_a_flat_end_is_bounded_exactly_and_the_round_side_within_the_excess() {
    // The issue #800 case: End facing reads the part's extent along X. A
    // flat end is bounded exactly, so the cut is placed at the real end
    // plus the stock to leave, never short of it; the round side reads at
    // most the analytic excess outside the radius, never inside it.
    let model = rod(12.0, 3.0, 61.5);
    let e = enclose(&model).unwrap();
    assert_eq!(support(&e.points, Vec3::new(1.0, 0.0, 0.0)), 61.5);
    assert_eq!(-support(&e.points, -Vec3::new(1.0, 0.0, 0.0)), 3.0);
    for k in 0..360 {
        let t = TAU * f64::from(k) / 360.0;
        let d = Vec3::new(0.0, t.cos(), t.sin());
        let s = support(&e.points, d);
        assert!(
            (12.0..=12.0 + e.analytic_excess + 1e-12).contains(&s),
            "radius {s} at {k} degrees"
        );
    }
    // The disc ends do not widen the bound past the side: their exact edge
    // circles bound them, not their parameter squares.
    let radius = e.points.iter().map(|p| p.y.hypot(p.z)).fold(0.0, f64::max);
    assert!(radius <= 12.0 * (1.0 + 1e-4), "{radius}");
}

#[test]
fn req_op_23_req_nfr_07_a_flat_face_without_exact_edges_is_bounded_by_its_parameter_box() {
    // The shouldered shaft's edges are polylines only (no exact curve): its
    // flat faces fall back to their parameter boxes, which hold them, so the
    // bound is still never inside the part.
    let model = openchuck_testkit::brep::shouldered_shaft(4);
    let e = enclose(&model).unwrap();
    assert_eq!(support(&e.points, Vec3::new(1.0, 0.0, 0.0)), 40.0);
    assert_eq!(-support(&e.points, -Vec3::new(1.0, 0.0, 0.0)), 0.0);
    for d in directions() {
        let face_max = model
            .faces
            .iter()
            .flat_map(|f| f.tessellation.vertices.iter())
            .map(|p| p.to_vec().dot(d))
            .fold(f64::NEG_INFINITY, f64::max);
        assert!(support(&e.points, d) >= face_max - 1e-9, "{d:?}");
    }
}

/// A rational bicubic-by-quadratic Bezier patch bulging towards +Z, with
/// unequal weights.
fn bulge() -> BSplinePatch {
    let mut poles = Vec::new();
    let mut weights = Vec::new();
    for i in 0..4 {
        for j in 0..3 {
            let x = 10.0 + 4.0 * f64::from(i);
            let y = -6.0 + 6.0 * f64::from(j);
            let z = 8.0 + [0.0, 3.5, 2.0, 0.5][i as usize] * [0.4, 1.0, 0.6][j as usize];
            poles.push(Point3::new(x, y, z));
            weights.push(1.0 + 0.3 * f64::from((i + j) % 3));
        }
    }
    BSplinePatch {
        degree_u: 3,
        degree_v: 2,
        knots_u: vec![0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0],
        knots_v: vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        poles_u: 4,
        poles_v: 3,
        poles,
        weights,
    }
}

/// A model of the one B-spline face `patch` over `uv` (no tessellation: the
/// enclosure never reads it).
fn spline_face(patch: BSplinePatch, uv: [f64; 4]) -> BrepModel {
    BrepModel {
        faces: vec![BrepFace {
            id: FaceId(0),
            surface: openchuck_types::brep::SurfaceKind::BSpline,
            body: 0,
            edges: Vec::new(),
            geometry: Surface::BSpline(patch),
            reversed: false,
            uv_bounds: uv,
            trims: Vec::new(),
            tessellation: TriangleMesh::default(),
        }],
        edges: Vec::new(),
        vertices: Vec::new(),
        tessellation: Default::default(),
    }
}

/// The rational Bezier patch at (u, v), by its Bernstein form.
fn bezier(p: &BSplinePatch, u: f64, v: f64) -> Point3 {
    fn bernstein(n: usize, i: usize, t: f64) -> f64 {
        let c = (1..=i).fold(1.0, |c, k| c * (n + 1 - k) as f64 / k as f64);
        c * t.powi(i as i32) * (1.0 - t).powi((n - i) as i32)
    }
    let (nu, nv) = (p.poles_u as usize, p.poles_v as usize);
    let (mut s, mut w) = (Vec3::ZERO, 0.0);
    for i in 0..nu {
        for j in 0..nv {
            let b = bernstein(nu - 1, i, u) * bernstein(nv - 1, j, v) * p.weights[i * nv + j];
            s = s + p.poles[i * nv + j].to_vec() * b;
            w += b;
        }
    }
    Point3::ORIGIN + s * (1.0 / w)
}

#[test]
fn req_op_23_req_op_24_a_b_spline_extreme_is_bounded_and_the_most_it_can_stop_short_is_stated() {
    // A B-spline face is bounded by its control net, refined: the bound is
    // never inside the face and at most the stated spline excess outside it
    // (decision 0666: the notes say how far Facing may stop short there).
    let patch = bulge();
    let uv = [0.15, 0.9, 0.0, 0.8];
    let model = spline_face(patch.clone(), uv);
    let e = enclose(&model).unwrap();
    let excess = e.spline_excess.expect("a B-spline face states its excess");
    assert!(excess > 0.0 && excess <= 0.001, "{excess}");
    let mut samples = Vec::new();
    for i in 0..=400 {
        for j in 0..=400 {
            let u = uv[0] + (uv[1] - uv[0]) * f64::from(i) / 400.0;
            let v = uv[2] + (uv[3] - uv[2]) * f64::from(j) / 400.0;
            samples.push(bezier(&patch, u, v));
        }
    }
    // The patch's speed is under 30 mm per unit parameter, so samples
    // 1/400 apart fall short of its support by under 0.04 mm only in
    // the worst case; in practice by far less. The upper check takes 0.002.
    check(&e, &samples, excess, 0.002, "bulge");
    // The control net itself (unrefined) reaches 3.5 mm above the plane of
    // its corners; the refined bound does not.
    let top = support(&e.points, Vec3::new(0.0, 0.0, 1.0));
    let real = support(&samples, Vec3::new(0.0, 0.0, 1.0));
    assert!(top - real <= excess + 1e-6, "{top} {real}");
}

#[test]
fn req_op_23_req_nfr_07_a_patch_it_cannot_bound_is_refused_with_the_face() {
    // A rational patch with a weight that is not positive has no convex
    // hull property: the face is named and nothing is guessed.
    let mut patch = bulge();
    patch.weights[4] = -0.5;
    let model = spline_face(patch, [0.0, 1.0, 0.0, 1.0]);
    let e = enclose(&model).unwrap_err().to_string();
    assert!(e.contains("face 0") && e.contains("weight"), "{e}");
    // An edge with no exact curve that is not a point, on a plane face
    // whose parameter box is not finite, cannot be bounded either.
    let mut model = rod(5.0, 0.0, 10.0);
    model.faces[0].uv_bounds = [f64::NEG_INFINITY, 1.0, 0.0, 1.0];
    model.edges[0].curve = None;
    model.edges[0].polyline = vec![Point3::new(0.0, 5.0, 0.0), Point3::new(0.0, -5.0, 0.0)];
    let e = enclose(&model).unwrap_err().to_string();
    assert!(e.contains("face 0"), "{e}");
}

#[test]
fn req_gc_07_req_nfr_06_the_enclosure_is_the_same_every_time() {
    let model = rod(9.0, -2.0, 30.0);
    let a = enclose(&model).unwrap();
    let b = enclose(&model).unwrap();
    assert_eq!(a.points, b.points);
    assert_eq!(a.mesh(), b.mesh());
}

#[test]
fn req_op_23_req_op_24_facing_reads_the_same_bounds_from_the_mesh_as_from_the_points() {
    // Facing on an X-rotary machine takes bounds along X, across X and the
    // radius from X. The mesh it is given (a prism over the points' outline
    // seen along X) gives exactly the points' bounds in those directions,
    // and never less in any other, so no bound is inside the part.
    let mut model = rod(9.0, -2.0, 30.0);
    model
        .faces
        .extend(ruled::sphere_band(40.0, 6.0, -1.0, 1.2).faces);
    model.faces[3].id = FaceId(3);
    let e = enclose(&model).unwrap();
    let mesh = e.mesh();
    mesh.validate().unwrap();
    let mut seen = vec![false; mesh.vertices.len()];
    for t in &mesh.triangles {
        for &i in t {
            seen[i as usize] = true;
        }
    }
    assert!(seen.iter().all(|s| *s), "every vertex is referenced");
    assert!(
        mesh.vertices.len() < e.points.len(),
        "the outline drops inner points"
    );
    for k in 0..720 {
        let t = TAU * f64::from(k) / 720.0;
        let d = Vec3::new(0.0, t.cos(), t.sin());
        assert_eq!(support(&mesh.vertices, d), support(&e.points, d), "{k}");
    }
    for d in [Vec3::new(1.0, 0.0, 0.0), Vec3::new(-1.0, 0.0, 0.0)] {
        assert_eq!(support(&mesh.vertices, d), support(&e.points, d));
    }
    let radius = |p: &[Point3]| p.iter().map(|p| p.y.hypot(p.z)).fold(0.0, f64::max);
    assert_eq!(radius(&mesh.vertices), radius(&e.points));
    for d in directions() {
        assert!(
            support(&mesh.vertices, d) >= support(&e.points, d) - 1e-12,
            "{d:?}"
        );
    }
}

#[test]
fn req_op_23_req_op_24_a_flat_face_is_bounded_by_its_exact_line_ellipse_b_spline_and_point_edges() {
    // A flat face lies in the convex hull of its boundary, so it is bounded
    // by its edges' exact curves: a line by its ends, an ellipse by
    // circumscribed corners, a B-spline by its refined control points, and a
    // degenerate edge (a point, such as a cone's apex) by itself. Checked
    // against points of the exact curves, from the library's evaluator.
    let plane = Frame {
        origin: Point3::new(0.0, 0.0, 0.0),
        x: Vec3::new(0.0, 1.0, 0.0),
        y: Vec3::new(0.0, 0.0, 1.0),
        z: Vec3::new(1.0, 0.0, 0.0),
    };
    let curves = vec![
        EdgeCurve::Line {
            origin: Point3::new(0.0, -10.0, -4.0),
            direction: Vec3::new(0.0, 1.0, 0.0),
            first: 0.0,
            last: 20.0,
        },
        EdgeCurve::Ellipse {
            frame: frame(
                Point3::new(0.0, 10.0, 3.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(1.0, 0.0, 0.0),
            ),
            major: 7.0,
            minor: 4.0,
            first: -1.4,
            last: 1.9,
        },
        EdgeCurve::BSpline(openchuck_types::brep::BSplineCurve {
            degree: 3,
            knots: vec![0.0, 0.0, 0.0, 0.0, 0.4, 1.0, 1.0, 1.0, 1.0],
            poles: vec![
                Point3::new(0.0, 10.0, 10.0),
                Point3::new(0.0, 4.0, 19.0),
                Point3::new(0.0, -2.0, 8.0),
                Point3::new(0.0, -8.0, 21.0),
                Point3::new(0.0, -10.0, 12.0),
            ],
            weights: vec![1.0, 1.4, 0.8, 1.0, 1.0],
            first: 0.05,
            last: 0.97,
        }),
    ];
    let edge = |id: u32, curve: Option<EdgeCurve>, polyline: Vec<Point3>| BrepEdge {
        id: EdgeId(id),
        faces: vec![FaceId(0)],
        vertices: None,
        polyline,
        curve,
    };
    let mut edges: Vec<BrepEdge> = curves
        .iter()
        .enumerate()
        .map(|(i, c)| edge(i as u32, Some(c.clone()), Vec::new()))
        .collect();
    let apex = Point3::new(0.0, -14.0, 2.0);
    edges.push(edge(3, None, vec![apex, apex]));
    let model = BrepModel {
        faces: vec![BrepFace {
            id: FaceId(0),
            surface: openchuck_types::brep::SurfaceKind::Plane,
            body: 0,
            edges: (0..4).map(EdgeId).collect(),
            geometry: Surface::Plane { frame: plane },
            reversed: false,
            uv_bounds: [-1e3, 1e3, -1e3, 1e3],
            trims: Vec::new(),
            tessellation: TriangleMesh::default(),
        }],
        edges,
        vertices: Vec::new(),
        tessellation: Default::default(),
    };
    let e = enclose(&model).unwrap();
    // The parameter box (2 m across) was not used: the edges bounded it.
    assert!(support(&e.points, Vec3::new(0.0, 1.0, 0.0)) < 100.0);
    let excess = e
        .analytic_excess
        .max(e.spline_excess.expect("a B-spline edge"));
    assert!(excess <= 0.001, "{excess}");
    let mut samples = vec![apex];
    for c in &curves {
        let (a, b) = match c {
            EdgeCurve::Line { first, last, .. } | EdgeCurve::Ellipse { first, last, .. } => {
                (*first, *last)
            }
            EdgeCurve::BSpline(s) => (s.first, s.last),
            _ => unreachable!(),
        };
        for k in 0..=4000 {
            let t = a + (b - a) * f64::from(k) / 4000.0;
            samples.push(openchuck_path::edge_curve::point(c, t).unwrap());
        }
    }
    check(&e, &samples, excess, 2e-4, "edges");
}

#[test]
fn req_op_23_req_nfr_07_a_flat_face_with_an_edge_it_cannot_bound_falls_back_to_its_box() {
    // An edge whose exact curve is missing (and is not a point) cannot bound
    // the face, so the whole face falls back to its parameter box: nothing
    // from its other edges is kept half-done.
    let mut model = rod(5.0, 0.0, 10.0);
    model.edges[0].curve = None;
    model.edges[0].polyline = vec![Point3::new(0.0, 5.0, 0.0), Point3::new(0.0, -5.0, 0.0)];
    let e = enclose(&model).unwrap();
    // The end disc's parameter square reaches the corner at radius 5 * sqrt 2.
    let r = e.points.iter().map(|p| p.y.hypot(p.z)).fold(0.0, f64::max);
    assert!((r - 5.0 * 2f64.sqrt()).abs() < 1e-9, "{r}");
}
