// SPDX-License-Identifier: AGPL-3.0-or-later
//! The tool gap on a surface source (OP-12; decision record SIMUL-EDGE-1,
//! Q2, pull request SE-1): the smallest signed distance between the whole
//! tool, at one pose or swept along a short straight move, and the surface.
//! Each answer is checked against a closed form computed here from the
//! geometry alone, and against the independent bounded checker
//! `openchuck_testkit::contact` (decision record 0255), which shares no code
//! with the query.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::indexing_slicing
)]

use std::sync::Arc;

use openchuck_path::{
    BrepSurface, CompositeSurface, GAP_ACCURACY, GapQuery, MeshSurface, Solid, SurfaceSource,
    ToolGap, ToolLine,
};
use openchuck_testkit::contact::{ConvexTool, Geometry, Settings};
use openchuck_types::geometry::{Point3, UnitVec3, Vec3};
use openchuck_types::mesh::TriangleMesh;
use openchuck_types::tool::profile::{ProfilePoint, ProfileSegment, RevolvedProfile, ToolProfile};
use proptest::prelude::*;

const FLUTE: f64 = 8.0;
const UP: Vec3 = Vec3::new(0.0, 0.0, 1.0);

fn ball(radius: f64) -> RevolvedProfile {
    ToolProfile {
        segments: vec![
            ProfileSegment::Arc {
                to: ProfilePoint::new(radius, radius),
                centre: ProfilePoint::new(0.0, radius),
            },
            ProfileSegment::Line {
                to: ProfilePoint::new(radius, FLUTE),
            },
        ],
    }
    .validate()
    .unwrap()
}

fn flat(radius: f64) -> RevolvedProfile {
    ToolProfile {
        segments: vec![
            ProfileSegment::Line {
                to: ProfilePoint::new(radius, 0.0),
            },
            ProfileSegment::Line {
                to: ProfilePoint::new(radius, FLUTE),
            },
        ],
    }
    .validate()
    .unwrap()
}

fn bull(radius: f64, corner: f64) -> RevolvedProfile {
    ToolProfile {
        segments: vec![
            ProfileSegment::Line {
                to: ProfilePoint::new(radius - corner, 0.0),
            },
            ProfileSegment::Arc {
                to: ProfilePoint::new(radius, corner),
                centre: ProfilePoint::new(radius - corner, corner),
            },
            ProfileSegment::Line {
                to: ProfilePoint::new(radius, FLUTE),
            },
        ],
    }
    .validate()
    .unwrap()
}

fn line(tip: Point3, axis: Vec3) -> ToolLine {
    ToolLine {
        origin: tip,
        axis: UnitVec3::new(axis).unwrap(),
    }
}

/// A square of the plane z = 0, `half` either side of the origin, in
/// `n` by `n` cells.
fn plane(half: f64, n: u32) -> TriangleMesh {
    let mut vertices = Vec::new();
    let step = 2.0 * half / f64::from(n);
    for j in 0..=n {
        for i in 0..=n {
            vertices.push(Point3::new(
                -half + step * f64::from(i),
                -half + step * f64::from(j),
                0.0,
            ));
        }
    }
    let at = |i: u32, j: u32| j * (n + 1) + i;
    let mut triangles = Vec::new();
    for j in 0..n {
        for i in 0..n {
            triangles.push([at(i, j), at(i + 1, j), at(i + 1, j + 1)]);
            triangles.push([at(i, j), at(i + 1, j + 1), at(i, j + 1)]);
        }
    }
    TriangleMesh {
        vertices,
        triangles,
    }
}

/// A ridge along Y: the surface z = -|x| for |x| <= `half`, |y| <= `half`,
/// with a sharp convex edge on the Y axis.
fn ridge(half: f64) -> TriangleMesh {
    let v = |x: f64, y: f64| Point3::new(x, y, -x.abs());
    let vertices = vec![
        v(-half, -half),
        v(0.0, -half),
        v(half, -half),
        v(-half, half),
        v(0.0, half),
        v(half, half),
    ];
    TriangleMesh {
        vertices,
        triangles: vec![[0, 1, 4], [0, 4, 3], [1, 2, 5], [1, 5, 4]],
    }
}

fn mesh(m: &TriangleMesh) -> MeshSurface {
    MeshSurface::new(m).unwrap()
}

fn gap(
    surface: &dyn SurfaceSource,
    profile: &RevolvedProfile,
    l: &ToolLine,
    sweep: Vec3,
) -> ToolGap {
    surface
        .tool_gap(profile, l, &GapQuery::swept(sweep, 5.0))
        .expect("a surface within the limit")
}

/// The interval holds `exact` and is no wider than the accuracy.
fn holds(g: &ToolGap, exact: f64) {
    assert!(
        g.lower <= exact + 1e-9 && exact <= g.upper + 1e-9,
        "{g:?} does not hold {exact}"
    );
    assert!(
        g.upper - g.lower <= GAP_ACCURACY + 1e-12,
        "{g:?} is too wide"
    );
}

#[test]
fn req_op_12_tool_gap_of_a_ball_above_and_into_a_plane() {
    let surface = mesh(&plane(10.0, 8));
    let tool = ball(1.5);
    for h in [0.3, 0.0, -0.05, -0.4] {
        let g = gap(
            &surface,
            &tool,
            &line(Point3::new(0.2, -0.3, h), UP),
            Vec3::ZERO,
        );
        holds(&g, h);
    }
}

#[test]
fn req_op_12_tool_gap_of_a_tilted_flat_end_reads_its_corner() {
    // A flat end tilted 30 degrees: its lowest point is the corner, radius
    // times sin 30 below the tip. Above the plane the gap is the corner's
    // height. Below it, the corner is a right-angled wedge whose faces rise
    // at 30 and 60 degrees; the plane point deepest inside the tool is on the
    // wedge's bisector (rising at 135 - 30 degrees), where it is as far from
    // both faces: depth p sin 45 / sin 105 for a corner p below the plane.
    let surface = mesh(&plane(10.0, 8));
    let r = 1.5;
    let t = 30.0_f64.to_radians();
    let axis = Vec3::new(t.sin(), 0.0, t.cos());
    for h in [1.0, 0.5, 0.2] {
        let g = gap(
            &surface,
            &flat(r),
            &line(Point3::new(0.0, 0.0, h), axis),
            Vec3::ZERO,
        );
        let corner = h - r * t.sin();
        let exact = if corner >= 0.0 {
            corner
        } else {
            corner * 45.0_f64.to_radians().sin() / (135.0_f64.to_radians() - t).sin()
        };
        holds(&g, exact);
    }
}

#[test]
fn req_op_12_tool_gap_of_a_ball_over_a_sharp_edge() {
    // The ball's centre 0.5 above and 0.5 beside the ridge: the edge is the
    // nearest point, 0.5 * sqrt 2 from the centre.
    let surface = mesh(&ridge(5.0));
    let r = 1.0;
    let centre = Point3::new(0.5, 0.0, 0.5);
    let g = gap(
        &surface,
        &ball(r),
        &line(centre + Vec3::new(0.0, 0.0, -r), UP),
        Vec3::ZERO,
    );
    holds(&g, 0.5_f64.hypot(0.5) - r);
    assert!(
        g.at.x.abs() < 1e-2,
        "the deepest point is on the edge: {:?}",
        g.at
    );
}

#[test]
fn req_op_12_tool_gap_swept_past_a_sharp_edge_is_the_deepest_pose() {
    // A ball swept straight over the ridge at a height of 0.9: at its ends
    // it is clear of the faces by more than a millimetre, but over the edge
    // the surface is 0.1 inside it. Only the swept query sees that.
    let surface = mesh(&ridge(5.0));
    let r = 1.0;
    let tool = ball(r);
    let start = Point3::new(-2.0, 0.0, 0.9 - r);
    let sweep = Vec3::new(4.0, 0.0, 0.0);
    let at_start = gap(&surface, &tool, &line(start, UP), Vec3::ZERO);
    let at_end = gap(&surface, &tool, &line(start + sweep, UP), Vec3::ZERO);
    let exact_end = (2.0 + 0.9) / 2.0_f64.sqrt() - r;
    holds(&at_start, exact_end);
    holds(&at_end, exact_end);
    let swept = gap(&surface, &tool, &line(start, UP), sweep);
    holds(&swept, 0.9 - r);
}

#[test]
fn req_op_12_tool_gap_of_a_bull_nose_on_a_slope() {
    // A bull nose with its axis upright on the face z = x (the ridge's left
    // face): the corner torus touches first. Its lowest signed distance is
    // that of the corner circle's lowest point from the face, less the
    // corner radius.
    let surface = mesh(&ridge(5.0));
    let (r, c) = (1.5, 0.5);
    let tip = Point3::new(-3.0, 0.0, -1.0);
    let g = gap(&surface, &bull(r, c), &line(tip, UP), Vec3::ZERO);
    // The corner circle, radius r - c at height c, nearest the face z = x
    // (normal (-1, 0, 1)/sqrt 2) at the point (tip.x + (r - c), 0, tip.z + c).
    let q = Point3::new(tip.x + (r - c), 0.0, tip.z + c);
    let exact = (q.z - q.x) / 2.0_f64.sqrt() - c;
    holds(&g, exact);
}

#[test]
fn req_op_12_tool_gap_is_none_beyond_the_limit_and_for_bad_input() {
    let surface = mesh(&plane(10.0, 4));
    let tool = ball(1.0);
    assert!(
        surface
            .tool_gap(
                &tool,
                &line(Point3::new(0.0, 0.0, 3.0), UP),
                &GapQuery::pose(1.0)
            )
            .is_none()
    );
    // NFR-07: input that is not finite gives no answer, never a crash.
    let nan = Point3::new(f64::NAN, 0.0, 0.0);
    assert!(
        surface
            .tool_gap(&tool, &line(nan, UP), &GapQuery::pose(1.0))
            .is_none()
    );
    assert!(
        surface
            .tool_gap(&tool, &line(Point3::ORIGIN, UP), &GapQuery::pose(f64::NAN))
            .is_none()
    );
    let g = surface
        .tool_gap(
            &tool,
            &line(Point3::new(0.0, 0.0, 0.5), UP),
            &GapQuery::swept(Vec3::new(f64::INFINITY, 0.0, 0.0), 1.0),
        )
        .unwrap();
    holds(&g, 0.5);
}

#[test]
fn req_op_12_tool_gap_stops_once_deeper_than_the_limit() {
    // A tool deep in the surface: the search stops when it has found a cut
    // deeper than the limit, and the interval still holds the true gap.
    let surface = mesh(&plane(10.0, 8));
    let g = surface
        .tool_gap(
            &ball(1.5),
            &line(Point3::new(0.0, 0.0, -1.0), UP),
            &GapQuery::pose(0.1),
        )
        .unwrap();
    assert!(g.lower <= -1.0 + 1e-9 && g.upper < -0.1, "{g:?}");
}

#[test]
fn req_op_12_tool_gap_accepted_above_a_depth_ends_once_proven() {
    // A caller that only needs to know the tool cuts no deeper than 0.006:
    // the answer is proven above it and still holds the true gap; a tool
    // deeper than that is still found deeper.
    let surface = mesh(&ridge(5.0));
    let tool = ball(1.0);
    let resting = line(Point3::new(0.0, 0.0, 0.0), UP);
    let g = surface
        .tool_gap(&tool, &resting, &GapQuery::pose(5.0).accept_above(-0.006))
        .unwrap();
    assert!(
        g.lower >= -0.006 && g.lower <= 1e-9 && g.upper >= -1e-9,
        "{g:?}"
    );
    let deep = line(Point3::new(0.0, 0.0, -0.05), UP);
    let g = surface
        .tool_gap(&tool, &deep, &GapQuery::pose(5.0).accept_above(-0.006))
        .unwrap();
    assert!(g.upper < -0.006 && g.lower <= -0.05 + 1e-9, "{g:?}");
    // A bound that is not a number is ignored.
    let g = surface
        .tool_gap(&tool, &resting, &GapQuery::pose(5.0).accept_above(f64::NAN))
        .unwrap();
    holds(&g, 0.0);
}

#[test]
fn req_op_12_tool_gap_of_a_profile_that_is_not_convex_is_its_cylinder() {
    // A corner-rounding profile (a concave arc) is not convex; the query
    // answers for the cylinder of its cutting radius, which holds it, so
    // its answer is never less safe than the tool's own.
    let profile = ToolProfile {
        segments: vec![
            ProfileSegment::Line {
                to: ProfilePoint::new(0.5, 0.0),
            },
            ProfileSegment::Arc {
                to: ProfilePoint::new(1.5, 1.0),
                centre: ProfilePoint::new(1.5, 0.0),
            },
            ProfileSegment::Line {
                to: ProfilePoint::new(1.5, FLUTE),
            },
        ],
    }
    .validate()
    .unwrap();
    assert!(!profile.is_convex());
    let surface = mesh(&plane(10.0, 8));
    let t = 30.0_f64.to_radians();
    let axis = Vec3::new(t.sin(), 0.0, t.cos());
    let g = gap(
        &surface,
        &profile,
        &line(Point3::new(0.0, 0.0, 1.0), axis),
        Vec3::ZERO,
    );
    holds(&g, 1.0 - 1.5 * t.sin());
}

/// A source that answers only what the trait requires, so its tool gap is
/// the trait's own, on its tessellation.
#[derive(Debug)]
struct Plain(MeshSurface);

impl SurfaceSource for Plain {
    fn bounds(&self) -> openchuck_types::geometry::Aabb {
        self.0.bounds()
    }
    fn project(&self, point: Point3) -> Option<openchuck_path::SurfacePoint> {
        self.0.project(point)
    }
    fn cast_ray(
        &self,
        origin: Point3,
        direction: UnitVec3,
        max_distance: f64,
    ) -> Option<openchuck_path::RayHit> {
        self.0.cast_ray(origin, direction, max_distance)
    }
    fn tessellation(&self) -> &TriangleMesh {
        self.0.tessellation()
    }
    fn drop_tool(
        &self,
        profile: &RevolvedProfile,
        line: &ToolLine,
    ) -> Option<openchuck_path::Contact> {
        self.0.drop_tool(profile, line)
    }
}

#[test]
fn req_op_12_tool_gap_default_answers_as_the_mesh_does() {
    let m = ridge(5.0);
    let fast = mesh(&m);
    let plain = Plain(mesh(&m));
    for (tip, sweep) in [
        (Point3::new(-0.7, 0.2, -0.4), Vec3::ZERO),
        (Point3::new(-2.0, 0.0, -0.1), Vec3::new(4.0, 0.5, 0.0)),
    ] {
        let l = line(tip, Vec3::new(0.2, 0.0, 1.0));
        let a = gap(&fast, &ball(1.0), &l, sweep);
        let b = gap(&plain, &ball(1.0), &l, sweep);
        assert_eq!(a, b);
    }
    assert_eq!(plain.tool_gap_accuracy(), 0.0);
}

#[test]
fn req_op_12_tool_gap_of_a_part_with_a_tab_sees_the_tab() {
    // A cylinder solid (a tab) beside a plane part: a ball over the tab
    // reads the tab, and the composite states the chord error of the tab's
    // tessellation as its accuracy.
    let part: Arc<dyn SurfaceSource> = Arc::new(mesh(&plane(10.0, 4)));
    let tab = Solid::cylinder(
        Point3::new(0.0, 0.0, 0.0),
        UnitVec3::new(Vec3::new(1.0, 0.0, 0.0)).unwrap(),
        4.0,
        2.0,
    )
    .unwrap();
    let joined = CompositeSurface::new(part, vec![tab]);
    let r = 1.0;
    let h = 2.3; // the tool's tip above the tab's top (radius 2)
    let g = gap(
        &joined,
        &ball(r),
        &line(Point3::new(2.0, 0.0, h), UP),
        Vec3::ZERO,
    );
    let accuracy = joined.tool_gap_accuracy();
    assert!(accuracy > 0.0 && accuracy < 2.0 * (1.0 - (std::f64::consts::PI / 90.0).cos()));
    // The tab's tessellation is inscribed: the true gap is h - 2 and the
    // tessellation's at most `accuracy` more.
    assert!(
        g.lower <= h - 2.0 + accuracy + 1e-9 && g.upper >= h - 2.0 - 1e-9,
        "{g:?}"
    );
    // With no solids the composite answers as its part, with its accuracy.
    let alone = CompositeSurface::new(Arc::new(mesh(&plane(10.0, 4))), Vec::new());
    assert_eq!(alone.tool_gap_accuracy(), 0.0);
    holds(
        &gap(
            &alone,
            &ball(r),
            &line(Point3::new(2.0, 0.0, 0.4), UP),
            Vec3::ZERO,
        ),
        0.4,
    );
}

#[test]
fn req_op_12_tool_gap_of_an_exact_part_states_its_tessellation() {
    let model = openchuck_testkit::brep::banded_cylinder(10.0, 20.0, &[]);
    let surface = BrepSurface::new(&model).unwrap();
    assert_eq!(surface.tool_gap_accuracy(), model.tessellation.linear);
    // A ball above the cylinder's top line: the gap on the tessellation is
    // the height above the radius, within the stated deviation.
    let g = gap(
        &surface,
        &ball(1.0),
        &line(Point3::new(10.0, 0.0, 10.5), UP),
        Vec3::ZERO,
    );
    let d = surface.tool_gap_accuracy();
    assert!(g.lower <= 0.5 + d + 1e-9 && g.upper >= 0.5 - 1e-9, "{g:?}");
    // A selection of the model's faces states the same accuracy.
    let ids: Vec<_> = model.faces.iter().map(|f| f.id).collect();
    let chosen = BrepSurface::with_faces(&model, &ids[..1]).unwrap();
    assert_eq!(chosen.tool_gap_accuracy(), model.tessellation.linear);
}

#[test]
fn req_gc_07_req_nfr_06_tool_gap_is_the_same_on_every_thread() {
    let surface = Arc::new(mesh(&ridge(5.0)));
    let ask = |s: &MeshSurface| {
        let l = line(Point3::new(-1.3, 0.4, -0.8), Vec3::new(0.3, 0.1, 1.0));
        s.tool_gap(
            &bull(1.5, 0.4),
            &l,
            &GapQuery::swept(Vec3::new(2.5, -0.3, 0.2), 5.0),
        )
    };
    let here = ask(&surface);
    let answers: Vec<_> = (0..4)
        .map(|_| {
            let s = Arc::clone(&surface);
            std::thread::spawn(move || ask(&s))
        })
        .map(|h| h.join().unwrap())
        .collect();
    for a in answers {
        let (a, b) = (a.unwrap(), here.unwrap());
        assert_eq!(a.lower.to_bits(), b.lower.to_bits());
        assert_eq!(a.upper.to_bits(), b.upper.to_bits());
        assert_eq!(a.at, b.at);
    }
}

#[test]
fn req_op_12_tool_gap_turned_to_its_own_axis_is_the_sweep() {
    let surface = mesh(&ridge(5.0));
    let tool = bull(1.5, 0.5);
    let axis = UnitVec3::new(Vec3::new(0.2, -0.1, 1.0)).unwrap();
    let l = ToolLine {
        origin: Point3::new(-1.0, 0.3, 0.4),
        axis,
    };
    let sweep = Vec3::new(2.0, -0.5, 0.1);
    let plain = surface.tool_gap(&tool, &l, &GapQuery::swept(sweep, 5.0));
    let turned = surface.tool_gap(&tool, &l, &GapQuery::swept(sweep, 5.0).turned(axis));
    assert_eq!(plain, turned);
}

#[test]
fn req_op_12_tool_gap_of_a_tool_turning_over_a_plane_is_its_lowest_end() {
    // A flat end and a ball end turning about a fixed tip from 25 degrees
    // one way to 25 degrees the other, above a plane: the poses at the ends
    // reach lowest, the flat end's corner its radius times sin 25 below the
    // tip and the ball's lowest point its radius times (1 - cos 25) below
    // it. Where that is above the plane it is the gap. Where the flat end's
    // corner is below the plane, the gap is the depth of the deepest plane
    // point inside the tool (the record's definition of the overcut
    // depth): in the section through the axis, where its depth above the
    // end face equals its depth inside the side.
    let surface = mesh(&plane(10.0, 8));
    let t = 25.0_f64.to_radians();
    let (from, to) = (
        Vec3::new(-t.sin(), 0.0, t.cos()),
        Vec3::new(t.sin(), 0.0, t.cos()),
    );
    let r = 1.5;
    let (s, c) = t.sin_cos();
    for (tool, lowest) in [(flat(r), -r * s), (ball(r), -r * (1.0 - c))] {
        for h in [1.2, 0.6] {
            let query = GapQuery::pose(5.0).turned(UnitVec3::new(to).unwrap());
            let g = surface
                .tool_gap(&tool, &line(Point3::new(0.1, 0.2, h), from), &query)
                .expect("a surface within the limit");
            let exact = if h + lowest >= 0.0 {
                h + lowest
            } else {
                // x s - h c = r - (x c + h s), x along the plane from the tip.
                let x = (r + h * c - h * s) / (s + c);
                -(x * s - h * c)
            };
            assert!(
                g.lower <= exact + 1e-9 && exact <= g.upper + 1e-9,
                "{g:?} does not hold {exact}"
            );
            assert!(g.upper - g.lower <= GAP_ACCURACY + 1e-12, "{g:?} is too wide");
        }
    }
}

/// A ridge with a random height and a random second edge, for the
/// independent checker.
fn uneven(seed: [f64; 4]) -> TriangleMesh {
    let [a, b, c, d] = seed;
    let v = |x: f64, y: f64, z: f64| Point3::new(x, y, z);
    TriangleMesh {
        vertices: vec![
            v(-4.0, -4.0, -2.0 + a),
            v(0.3 * b, -4.0, 0.0),
            v(4.0, -4.0, -1.0 + c),
            v(-4.0, 4.0, -2.0 + d),
            v(-0.3 * b, 4.0, 0.2 * a),
            v(4.0, 4.0, -1.5 + 0.5 * c),
        ],
        triangles: vec![[0, 1, 4], [0, 4, 3], [1, 2, 5], [1, 5, 4]],
    }
}

fn profiles() -> impl Strategy<Value = RevolvedProfile> {
    prop_oneof![
        (0.5_f64..2.0).prop_map(ball),
        (0.5_f64..2.0).prop_map(flat),
        (1.0_f64..2.0, 0.1_f64..0.9).prop_map(|(r, c)| bull(r, c * r)),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 48, ..ProptestConfig::default() })]

    /// The query's interval and the independent checker's proven interval
    /// overlap, at one pose, for ball, flat and bull ends at any lean: each
    /// holds the true gap. Swept, the query's proven lower bound is never
    /// above what the checker finds at any of the poses along the move,
    /// turning as well (its axis the normalised blend of the two).
    #[test]
    fn req_op_12_tool_gap_agrees_with_the_independent_checker(
        seed in prop::array::uniform4(0.0_f64..1.0),
        profile in profiles(),
        tip in (-2.0_f64..2.0, -2.0_f64..2.0, -1.0_f64..1.5),
        lean in (-0.8_f64..0.8, -0.8_f64..0.8),
        sweep in (-2.0_f64..2.0, -2.0_f64..2.0, -0.5_f64..0.5),
        turn in (-0.8_f64..0.8, -0.8_f64..0.8),
    ) {
        let m = uneven(seed);
        let surface = mesh(&m);
        let geometry = Geometry::of_mesh(&m).unwrap();
        let tool = ConvexTool::new(&profile).unwrap();
        let axis = Vec3::new(lean.0, lean.1, 1.0);
        let unit = UnitVec3::new(axis).unwrap().get();
        let tip = Point3::new(tip.0, tip.1, tip.2);
        let settings = Settings { margin: 5.0, tolerance: 1e-4, budget: 2_000_000 };
        let ours = surface.tool_gap(&profile, &line(tip, axis), &GapQuery::pose(4.0));
        let theirs = geometry.clearance(&tool, tip, unit, &settings).unwrap();
        if let Some(g) = ours {
            prop_assert!(g.lower <= theirs.upper + 1e-6, "{g:?} against {theirs:?}");
            prop_assert!(theirs.lower <= g.upper + 1e-6, "{g:?} against {theirs:?}");
        } else {
            prop_assert!(theirs.lower > 4.0 - 1e-3, "none against {theirs:?}");
        }
        let v = Vec3::new(sweep.0, sweep.1, sweep.2);
        if let Some(g) = surface.tool_gap(&profile, &line(tip, axis), &GapQuery::swept(v, 4.0)) {
            for k in 0..=16 {
                let s = f64::from(k) / 16.0;
                let at = geometry.clearance(&tool, tip + v * s, unit, &settings).unwrap();
                prop_assert!(g.lower <= at.upper + 1e-6, "{g:?} above {at:?} at {s}");
            }
        }
        // Turning as it moves: the bound holds at every pose between.
        let end = UnitVec3::new(Vec3::new(turn.0, turn.1, 1.0)).unwrap();
        let query = GapQuery::swept(v, 4.0).turned(end);
        if let Some(g) = surface.tool_gap(&profile, &line(tip, axis), &query) {
            for k in 0..=16 {
                let s = f64::from(k) / 16.0;
                let between = UnitVec3::new(unit * (1.0 - s) + end.get() * s).unwrap();
                let at = geometry.clearance(&tool, tip + v * s, between.get(), &settings).unwrap();
                prop_assert!(g.lower <= at.upper + 1e-6, "{g:?} above {at:?} at {s}");
            }
        }
    }
}
