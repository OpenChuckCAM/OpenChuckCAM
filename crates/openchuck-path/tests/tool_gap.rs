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
    BrepSurface, CompositeSurface, GAP_ACCURACY, GapQuery, MeshSurface, Rotation, Solid,
    SurfaceSource, ToolGap, ToolLine,
};
use openchuck_testkit::contact::{Bounded, ContactError, ConvexTool, Geometry, Settings};
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
    gap_within(surface, profile, l, sweep, 5.0)
}

fn gap_within(
    surface: &dyn SurfaceSource,
    profile: &RevolvedProfile,
    l: &ToolLine,
    sweep: Vec3,
    limit: f64,
) -> ToolGap {
    surface
        .tool_gap(profile, l, &GapQuery::swept(sweep, limit))
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
    // times sin 30 below the tip, and the gap is the corner's height above
    // the plane, below it as well: the depth the corner cuts along the
    // face's normal (decision record SIMUL-EDGE-1, Q2 as answered C).
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
        holds(&g, h - r * t.sin());
    }
}

#[test]
fn req_op_12_tool_gap_of_a_ball_over_a_sharp_edge() {
    // The ball's centre 0.5 above and 0.5 beside the ridge, on the plane of
    // the far face (z = x) half a unit beyond that face's edge. That face
    // counts the ball over itself only: its lowest point there is over the
    // edge, sqrt(1 - 1/2) below the plane, so the ball cuts the ridge that
    // deep along the far face's normal (Q2 answered C), deeper than its
    // nearest distance into the near face. A point counts only where it is
    // also at least the query's acceptance (here the search's accuracy)
    // below the near face's plane, its convex neighbour (option A): so the
    // deepest counted column is that far in from the edge.
    let surface = mesh(&ridge(5.0));
    let r = 1.0;
    let centre = Point3::new(0.5, 0.0, 0.5);
    let g = gap(
        &surface,
        &ball(r),
        &line(centre + Vec3::new(0.0, 0.0, -r), UP),
        Vec3::ZERO,
    );
    let inward = 0.5_f64.sqrt() + GAP_ACCURACY;
    holds(&g, -(r * r - inward * inward).sqrt());
    assert!(
        g.at.x.abs() < 1e-2,
        "the deepest point is on the edge: {:?}",
        g.at
    );
}

/// A block's square edge along Y at the origin: its top, z = 0 for x <= 0,
/// facing +Z, and its end, x = 0 for z <= 0, facing +X.
fn square_edge() -> TriangleMesh {
    TriangleMesh {
        vertices: vec![
            Point3::new(-5.0, -5.0, 0.0),
            Point3::new(0.0, -5.0, 0.0),
            Point3::new(0.0, 5.0, 0.0),
            Point3::new(-5.0, 5.0, 0.0),
            Point3::new(0.0, -5.0, -5.0),
            Point3::new(0.0, 5.0, -5.0),
        ],
        triangles: vec![[0, 1, 2], [0, 2, 3], [1, 4, 5], [1, 5, 2]],
    }
}

/// The move check's question at a tolerance of 0.01 mm: the layer a quarter
/// deeper, a gap of -0.01 accepted.
fn checked() -> GapQuery {
    GapQuery::pose(0.0125).accept_above(-0.01)
}

#[test]
fn req_op_12_tool_gap_of_a_ball_resting_beside_a_square_edge_reads_no_cut() {
    // The owner's answer of 26 Nov 2026 (decision record SIMUL-EDGE-1,
    // Terms, option A: the depth into the material). A ball resting on the
    // top 0.011 mm from a square edge touches the surface; along the end's
    // normal its contact is 0.011 mm below the end's plane, which measure C
    // alone read as a cut deeper than the tolerance. The end counts a tool
    // point only where it is also at least the acceptance below the top's
    // plane, so the ball reads no cut, proven, and its gap is 0.
    let surface = mesh(&square_edge());
    let tool = ball(1.0);
    let resting = line(Point3::new(-0.011, 0.0, 0.0), UP);
    let g = surface.tool_gap(&tool, &resting, &checked()).unwrap();
    assert!(g.lower >= -0.01, "{g:?}");
    holds(
        &gap_within(&surface, &tool, &resting, Vec3::ZERO, 0.0125),
        0.0,
    );
}

#[test]
fn req_op_12_tool_gap_of_a_ball_plunged_at_a_square_edge_is_a_cut() {
    // Plunged 1 mm at the edge, the ball is deep in the material below both
    // faces: the check finds the cut, on a mesh and on a STEP part's exact
    // faces (the shouldered shaft's end at X = 0 meets its cylinder of
    // radius 8 at a square edge).
    let surface = mesh(&square_edge());
    let tool = ball(1.0);
    let g = surface
        .tool_gap(&tool, &line(Point3::new(0.0, 0.0, -1.0), UP), &checked())
        .unwrap();
    assert!(g.lower < -0.01 && g.upper < -0.01, "{g:?}");
    let shaft = BrepSurface::new(&openchuck_testkit::brep::shouldered_shaft(4)).unwrap();
    let g = shaft
        .tool_gap(&tool, &line(Point3::new(0.0, 0.0, 7.0), UP), &checked())
        .unwrap();
    assert!(g.lower < -0.01 && g.upper < -0.01, "{g:?}");
    // Resting on the cylinder 0.011 mm in from the end it reads no cut.
    let g = shaft
        .tool_gap(&tool, &line(Point3::new(0.011, 0.0, 8.0), UP), &checked())
        .unwrap();
    assert!(g.lower >= -0.01, "{g:?}");
}

/// A face of the plane z = 0 from x = `from` to 6, |y| <= 6, facing +Z,
/// turned by `tilt` radians about the Y axis.
fn strip(from: f64, tilt: f64) -> TriangleMesh {
    let turn = |x: f64, y: f64| Point3::new(x * tilt.cos(), y, -x * tilt.sin());
    TriangleMesh {
        vertices: vec![
            turn(from, -6.0),
            turn(6.0, -6.0),
            turn(6.0, 6.0),
            turn(from, 6.0),
        ],
        triangles: vec![[0, 1, 2], [0, 2, 3]],
    }
}

/// A ball-end tool of radius `r` lying on its side along Y, its axis `h`
/// above the plane z = 0 at x = 0, the whole turned as [`strip`] is.
fn lying(r: f64, h: f64, tilt: f64) -> ToolLine {
    let turn = |v: Vec3| {
        Vec3::new(
            v.x * tilt.cos() + v.z * tilt.sin(),
            v.y,
            -v.x * tilt.sin() + v.z * tilt.cos(),
        )
    };
    let tip = Point3::ORIGIN + turn(Vec3::new(0.0, -5.0 - r, h));
    ToolLine {
        origin: tip,
        axis: UnitVec3::new(turn(Vec3::new(0.0, 1.0, 0.0))).unwrap(),
    }
}

#[test]
fn req_op_12_tool_gap_of_a_tool_grazing_a_face_from_beside_is_its_clearance() {
    // A tool lying on its side along a face's edge (a 3.175 mm ball's
    // flutes running beside a crank web, or along a blade): its body would
    // cut the plane, but the face ends where the tool's side is upright, so
    // over the face the tool's lowest point is its axis, h above, at the
    // face's edge; a face starting a little inside reads the tool's lower
    // surface there, still clear. The query answers both, proven, without
    // running out of pieces, flat and tilted as a blade side is. Where the
    // tool only touches the face's edge, along a line, that line is within
    // the side margin, so no point counts and none is an answer too.
    let (r, h) = (1.5875, 0.128);
    let tool = ball(r);
    for tilt in [0.0, 0.5] {
        for (from, exact) in [
            (r, h),
            (r - 1e-3, h - (r * r - (r - 1e-3) * (r - 1e-3)).sqrt()),
        ] {
            let surface = mesh(&strip(from, tilt));
            let line = lying(r, h, tilt);
            let checked =
                surface.tool_gap(&tool, &line, &GapQuery::pose(0.0125).accept_above(-0.01));
            assert!(
                checked.is_none_or(|g| g.lower >= -0.01),
                "tilt {tilt}, from {from}: {checked:?}"
            );
            match surface.tool_gap(&tool, &line, &GapQuery::pose(0.5)) {
                Some(g) => holds(&g, exact),
                None => assert!(from >= r, "tilt {tilt}, from {from}: none"),
            }
        }
    }
}

#[test]
fn req_op_12_tool_gap_swept_past_a_sharp_edge_is_the_deepest_pose() {
    // A ball swept straight over the ridge at a height of 0.9: at its ends
    // it is clear of the faces by more than a millimetre, but over the edge
    // it cuts the ridge. Measured along the left face's normal (u along the
    // face from its edge, h above it), its centre is at u = (x + 0.9)/sqrt 2
    // and h = 1.8/sqrt 2 - u, and its lowest point over the face (u <= 0) is
    // h - sqrt(r^2 - u^2), least at u = r/sqrt 2: 1.8/sqrt 2 - r sqrt 2.
    // Only the swept query sees that. Counted no nearer the edge than the
    // query's acceptance (the search's accuracy), measured along the face
    // (option A), it is that much less deep.
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
    holds(&swept, (1.8 - 2.0 * r) / 2.0_f64.sqrt() + GAP_ACCURACY);
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
fn req_op_12_tool_gap_reads_a_cut_deeper_than_the_limit_as_the_limit() {
    // A tool deep in the surface: a face counts the tool only down to the
    // limit below it, and a convex tool cutting deeper passes through that
    // layer, so the gap reads the limit.
    let surface = mesh(&plane(10.0, 8));
    let g = surface
        .tool_gap(
            &ball(1.5),
            &line(Point3::new(0.0, 0.0, -1.0), UP),
            &GapQuery::pose(0.1),
        )
        .unwrap();
    holds(&g, -0.1);
}

#[test]
fn req_op_12_tool_gap_does_not_count_a_wall_from_behind() {
    // A slab 2 mm thick: a ball resting on its top is beside the slab's
    // underside, which faces down, from the material's side and 2 mm away;
    // the underside counts nothing deeper than the limit below it, so the
    // gap is the top's, not a cut through the slab.
    let top = plane(10.0, 4);
    let mut slab = top.clone();
    let below = u32::try_from(top.vertices.len()).unwrap();
    slab.vertices
        .extend(top.vertices.iter().map(|p| Point3::new(p.x, p.y, -2.0)));
    slab.triangles.extend(
        top.triangles
            .iter()
            .map(|[a, b, c]| [a + below, c + below, b + below]),
    );
    let surface = mesh(&slab);
    let g = gap_within(
        &surface,
        &ball(1.0),
        &line(Point3::ORIGIN, UP),
        Vec3::ZERO,
        1.0,
    );
    holds(&g, 0.0);
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
    // Within 1 mm: the tab's underside, 4 mm below, counts nothing.
    let g = gap_within(
        &joined,
        &ball(r),
        &line(Point3::new(2.0, 0.0, h), UP),
        Vec3::ZERO,
        1.0,
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
    // The fixture's cylinder, radius 10, is 96 facets round: each facet's
    // error against the cylinder is its sagitta, r (1 - cos(pi / 96)),
    // measured exactly when the source is built (OI-9974), not the import's
    // stated deflection.
    let model = openchuck_testkit::brep::banded_cylinder(10.0, 20.0, &[]);
    let surface = BrepSurface::new(&model).unwrap();
    let sagitta = 10.0 * (1.0 - (std::f64::consts::PI / 96.0).cos());
    assert!(
        (surface.tool_gap_accuracy() - sagitta).abs() < 1e-9,
        "{} against {sagitta}",
        surface.tool_gap_accuracy()
    );
    assert!(sagitta < model.tessellation.linear);
    // A ball above the cylinder's top line: its gap to the exact cylinder
    // is the tip's height above the radius, and the answer holds it, each
    // facet's error taken off its bound.
    let g = gap(
        &surface,
        &ball(1.0),
        &line(Point3::new(10.0, 0.0, 10.5), UP),
        Vec3::ZERO,
    );
    assert!(
        g.lower <= 0.5 + 1e-9 && g.upper >= 0.5 - 1e-9,
        "{g:?} does not hold 0.5"
    );
    assert!(
        g.upper - g.lower <= GAP_ACCURACY + 2.0 * sagitta + 1e-9,
        "{g:?}"
    );
    // A selection of the model's faces measures the same.
    let ids: Vec<_> = model.faces.iter().map(|f| f.id).collect();
    let chosen = BrepSurface::with_faces(&model, &ids[..1]).unwrap();
    assert!((chosen.tool_gap_accuracy() - sagitta).abs() < 1e-9);
}

#[test]
fn req_op_12_tool_gap_of_an_exact_part_takes_its_error_one_way() {
    // The project owner's answer of 27 Nov 2026: a triangle's error is taken
    // only the way the face lies from it. A convex cylinder bulges out past
    // its facets, so a tool above it may be closer than a facet says, never
    // farther; the answer's interval is then one sagitta wide (plus the
    // search's own accuracy), not two.
    let model = openchuck_testkit::brep::banded_cylinder(10.0, 20.0, &[]);
    let surface = BrepSurface::new(&model).unwrap();
    let sagitta = 10.0 * (1.0 - (std::f64::consts::PI / 96.0).cos());
    for (y, z) in [(0.0, 10.5), (0.7, 10.3), (-1.1, 10.05)] {
        let g = gap(
            &surface,
            &ball(1.0),
            &line(Point3::new(10.0, y, z), UP),
            Vec3::ZERO,
        );
        // The exact gap: the ball's centre's distance from the axis, less
        // the ball's radius and the cylinder's.
        let exact = Point3::new(10.0, y, z + 1.0).distance(Point3::new(10.0, 0.0, 0.0)) - 11.0;
        assert!(
            g.lower <= exact + 1e-9 && g.upper >= exact - 1e-9,
            "{g:?} does not hold {exact}"
        );
        assert!(
            g.upper - g.lower <= GAP_ACCURACY + sagitta + 1e-9,
            "{g:?}: wider than one sagitta {sagitta}"
        );
    }
    // A ball resting against the wall of a hole, a concave face that lies
    // inside its facets: the same, the other way.
    use openchuck_testkit::brep::{HoleSpec, drilled_blank};
    let (blank, _) = drilled_blank(10.0, 30.0, &[HoleSpec::through(15.0, [0.0, 1.0, 0.0], 6.0)]);
    let surface = BrepSurface::new(&blank).unwrap();
    let error = surface.tool_gap_accuracy();
    assert!(error > 1e-6, "the hole's facets lie off its wall");
    // Counted half a millimetre below each face: the blank's round face is
    // over 2 mm beyond the tool's top.
    let g = gap_within(
        &surface,
        &ball(1.0),
        &line(Point3::new(15.0, -1.0, 1.9), Vec3::new(0.0, 1.0, 0.0)),
        Vec3::ZERO,
        0.5,
    );
    // The ball's centre is 1.9 from the hole's axis, its surface 0.1 short
    // of the wall of radius 3 (the shank above it, as far out, the same).
    assert!(
        g.lower <= 0.1 + 1e-9 && g.upper >= 0.1 - 1e-9,
        "{g:?} does not hold 0.1"
    );
    assert!(
        g.upper - g.lower <= GAP_ACCURACY + error + 1e-9,
        "{g:?}: wider than one error {error}"
    );
}

#[test]
fn req_op_12_a_failed_pose_on_an_exact_part_is_rechecked_on_finer_triangles() {
    // The project owner's answer of 27 Nov 2026 (Q-B): a pose that fails
    // only because of the triangles' error is checked again on triangles
    // refined against the exact faces until their band is within 0.0001
    // mm. A ball cut 0.009 mm into a cylinder, within the tolerance of 0.01
    // mm but inside the band of the triangles refined at load (up to a
    // quarter of the layer), passes; one cut 0.0105 mm deep still fails.
    let model = openchuck_testkit::brep::banded_cylinder(10.0, 20.0, &[]);
    let surface = BrepSurface::new(&model).unwrap();
    let tolerance = 0.01;
    let query = GapQuery::pose(1.25 * tolerance).accept_above(-tolerance);
    let ask = |depth: f64, angle: f64| {
        // The ball's centre on a radius at `angle` from the top, the ball's
        // deepest point `depth` inside the cylinder; its axis along that
        // radius.
        let (s, c) = angle.sin_cos();
        let u = Vec3::new(0.0, s, c);
        let centre = Point3::new(10.0, 0.0, 0.0) + u * (10.0 + 1.0 - depth);
        surface
            .tool_gap(&ball(1.0), &line(centre + u * (-1.0), u), &query)
            .unwrap()
    };
    // Between two facets' corners, where a facet lies farthest inside the
    // cylinder (96 facets round).
    let middle = std::f64::consts::PI / 96.0;
    for angle in [0.0, 0.5 * middle, middle] {
        let g = ask(0.009, angle);
        assert!(
            g.lower >= -tolerance,
            "at {angle}: {g:?} fails a cut 0.009 deep"
        );
        let g = ask(0.0105, angle);
        assert!(
            g.lower < -tolerance,
            "at {angle}: {g:?} passes a cut 0.0105 deep"
        );
    }
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

/// The vector `v` turned by `a` radians about the unit vector `n`.
fn turn_vector(v: Vec3, n: Vec3, a: f64) -> Vec3 {
    let along = n * v.dot(n);
    let (s, c) = a.sin_cos();
    along + (v - along) * c + n.cross(v) * s
}

#[test]
fn req_op_12_tool_gap_turned_by_nothing_is_the_sweep() {
    let surface = mesh(&ridge(5.0));
    let tool = bull(1.5, 0.5);
    let axis = UnitVec3::new(Vec3::new(0.2, -0.1, 1.0)).unwrap();
    let l = ToolLine {
        origin: Point3::new(-1.0, 0.3, 0.4),
        axis,
    };
    let sweep = Vec3::new(2.0, -0.5, 0.1);
    let plain = surface.tool_gap(&tool, &l, &GapQuery::swept(sweep, 5.0));
    let none = Rotation {
        point: Point3::new(3.0, -2.0, 1.0),
        axis: UnitVec3::new(Vec3::new(1.0, 0.0, 0.0)).unwrap(),
        angle: 0.0,
    };
    let turned = surface.tool_gap(&tool, &l, &GapQuery::swept(sweep, 5.0).rotating(none));
    assert_eq!(plain, turned);
}

#[test]
fn req_op_12_tool_gap_of_a_tool_turning_over_a_plane_is_its_lowest_end() {
    // A flat end and a ball end turning about a fixed tip from 25 degrees
    // one way to 25 degrees the other, above a plane: the poses at the ends
    // reach lowest, the flat end's corner its radius times sin 25 below the
    // tip and the ball's lowest point its radius times (1 - cos 25) below
    // it: the gap is that point's height above the plane, below it as well.
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
            // Turned about the line through the tip along Y, by twice 25
            // degrees, from leaning one way to the other.
            let tip = Point3::new(0.1, 0.2, h);
            let rotation = Rotation {
                point: tip,
                axis: UnitVec3::new(Vec3::new(0.0, 1.0, 0.0)).unwrap(),
                angle: 2.0 * t,
            };
            let ends = turn_vector(from, Vec3::new(0.0, 1.0, 0.0), 2.0 * t);
            assert!((ends - to).length() < 1e-12);
            let query = GapQuery::pose(5.0).rotating(rotation);
            let g = surface
                .tool_gap(&tool, &line(Point3::new(0.1, 0.2, h), from), &query)
                .expect("a surface within the limit");
            let exact = h + lowest;
            assert!(
                g.lower <= exact + 1e-9 && exact <= g.upper + 1e-9,
                "{g:?} does not hold {exact}"
            );
            assert!(
                g.upper - g.lower <= GAP_ACCURACY + 1e-12,
                "{g:?} is too wide"
            );
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
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// A tool lying beside a face's edge, grazing it, at any height and
    /// lean, for ball and flat ends: the query and the independent checker
    /// overlap (decision record SIMUL-EDGE-1; the second bound the project
    /// owner approved on 27 Nov 2026).
    #[test]
    fn req_op_12_tool_gap_grazing_a_face_agrees_with_the_independent_checker(
        r in 0.5_f64..2.0,
        h in 0.01_f64..0.4,
        // Clear of the side margin either way: a touch along a line is
        // counted by the checker and not by the query, both within it.
        inset in 1e-5_f64..0.01,
        outside in prop::bool::ANY,
        tilt in -0.6_f64..0.6,
        round in prop::bool::ANY,
    ) {
        let m = strip(if outside { r + inset } else { r - inset }, tilt);
        let surface = mesh(&m);
        let geometry = Geometry::of_mesh(&m).unwrap();
        let profile = if round { ball(r) } else { flat(r) };
        let tool = ConvexTool::new(&profile).unwrap();
        let line = lying(r, h, tilt);
        let settings = Settings { margin: 5.0, tolerance: 5e-4, budget: 20_000 };
        let ours = surface.tool_gap(&profile, &line, &GapQuery::pose(0.5));
        let theirs = interval(geometry.face_gap(&tool, line.origin, line.axis.get(), 0.5, GAP_ACCURACY, &settings));
        if let Some(g) = ours {
            prop_assert!(g.lower <= theirs.upper + 1e-6, "{g:?} against {theirs:?}");
            prop_assert!(theirs.lower <= g.upper + 1e-6, "{g:?} against {theirs:?}");
        } else {
            // The checker found no real value under the limit either (it
            // may run out of budget proving a tool that ends a hair before
            // the face clear of it).
            prop_assert!(theirs.upper > 0.5 - 1e-3, "none against {theirs:?}");
        }
    }
}

/// The checker's proven interval: its answer, or, where its budget ran out
/// first (a tool silhouette running along a face's edge, where the measure
/// jumps), the interval it had reached, still proven.
fn interval(answer: Result<Bounded, ContactError>) -> Bounded {
    match answer {
        Ok(b) => b,
        Err(ContactError::Budget {
            cells,
            lower,
            upper,
        }) => Bounded {
            upper,
            lower,
            at: None,
            cells,
        },
        Err(e) => panic!("the checker refused: {e}"),
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 48, ..ProptestConfig::default() })]

    /// The query's interval and the independent checker's proven interval
    /// of the same measure (each face along its own normal, over itself,
    /// down to the limit) overlap at one pose, for ball, flat and bull ends
    /// at any lean: each holds the true gap. Swept, and turning about a
    /// line as well, the query's proven lower bound is never above what
    /// the checker finds at any of the poses along the move.
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
        let settings = Settings { margin: 5.0, tolerance: 5e-4, budget: 20_000 };
        let ours = surface.tool_gap(&profile, &line(tip, axis), &GapQuery::pose(4.0));
        let theirs = interval(geometry.face_gap(&tool, tip, unit, 4.0, GAP_ACCURACY, &settings));
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
                let at = interval(geometry.face_gap(&tool, tip + v * s, unit, 4.0, GAP_ACCURACY, &settings));
                prop_assert!(g.lower <= at.upper + 1e-6, "{g:?} above {at:?} at {s}");
            }
        }
        // Turning about a line along X as it moves (a rotary axis): the
        // bound holds at every pose between, each the first pose shifted
        // and then turned.
        let x = Vec3::new(1.0, 0.0, 0.0);
        let centre = Point3::new(0.0, 3.0 * turn.0, -4.0 + turn.0);
        let rotation = Rotation { point: centre, axis: UnitVec3::new(x).unwrap(), angle: turn.1 };
        let query = GapQuery::swept(v, 4.0).rotating(rotation);
        if let Some(g) = surface.tool_gap(&profile, &line(tip, axis), &query) {
            for k in 0..=16 {
                let s = f64::from(k) / 16.0;
                let a = s * turn.1;
                let at_tip = centre + turn_vector(tip + v * s - centre, x, a);
                let at_axis = turn_vector(unit, x, a);
                let at = interval(geometry.face_gap(&tool, at_tip, at_axis, 4.0, GAP_ACCURACY, &settings));
                prop_assert!(g.lower <= at.upper + 1e-6, "{g:?} above {at:?} at {s}");
            }
        }
    }
}

#[test]
fn req_op_12_tool_gap_within_no_pieces_is_from_the_bounds_alone() {
    // A query given no pieces to split (lane SE-PERF: a stretch of a move
    // bounded before it is sampled) answers from each face's own bounds:
    // a ball 0.002 above a plane, sliding 3 mm along it and turning 0.02
    // rad about a line 10 mm below, dips 0.0002 into it, and is proven no
    // deeper than 0.01 as it stands; started 0.02 into the plane it is not,
    // and the interval it gives still holds the true depth.
    let surface = mesh(&plane(5.0, 10));
    let tool = ball(1.0);
    let rotation = Rotation {
        point: Point3::new(0.0, 0.0, -10.0),
        axis: UnitVec3::new(Vec3::new(1.0, 0.0, 0.0)).unwrap(),
        angle: 0.02,
    };
    let query = GapQuery::swept(Vec3::new(3.0, 0.0, 0.0), 0.0125)
        .rotating(rotation)
        .accept_above(-0.01)
        .within_pieces(0);
    let above = line(Point3::new(-1.5, 0.0, 0.002), UP);
    let g = surface.tool_gap(&tool, &above, &query).unwrap();
    assert!(g.lower >= -0.01 && g.lower <= -0.0002 + 1e-6, "{g:?}");
    let cutting = line(Point3::new(-1.5, 0.0, -0.02), UP);
    let g = surface.tool_gap(&tool, &cutting, &query).unwrap();
    assert!(g.lower < -0.01 && g.upper >= -0.0125, "{g:?}");
    // Without an acceptance there is nothing to prove from the bounds: the
    // interval is whatever the first bounds give, and holds the true gap.
    let open = GapQuery::pose(5.0).within_pieces(0);
    let g = surface.tool_gap(&tool, &above, &open).unwrap();
    assert!(g.lower <= 0.002 + 1e-9 && g.upper >= 0.002 - 1e-9, "{g:?}");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    /// A stretch of a move proven from the bounds alone (a query within no
    /// pieces, lane SE-PERF) cuts no deeper than the acceptance at any pose
    /// along it, as the independent checker measures each pose: for ball,
    /// flat and bull ends resting near an uneven part, shifting and turning
    /// about a line along X.
    #[test]
    fn req_op_12_a_stretch_proven_within_no_pieces_cuts_no_deeper_at_any_pose(
        seed in prop::array::uniform4(0.0_f64..1.0),
        profile in profiles(),
        tip in (-2.0_f64..2.0, -2.0_f64..2.0),
        lean in (-0.5_f64..0.5, -0.5_f64..0.5),
        height in -0.004_f64..0.03,
        sweep in (-1.0_f64..1.0, -1.0_f64..1.0, -0.02_f64..0.02),
        turn in (-0.5_f64..0.5, -0.03_f64..0.03),
    ) {
        let allowed = 0.01;
        let m = uneven(seed);
        let surface = mesh(&m);
        let geometry = Geometry::of_mesh(&m).unwrap();
        let tool = ConvexTool::new(&profile).unwrap();
        let axis = Vec3::new(lean.0, lean.1, 1.0);
        let unit = UnitVec3::new(axis).unwrap().get();
        // Rested near the part: lowered by its gap from above, then set
        // `height` from there.
        let start = Point3::new(tip.0, tip.1, 3.0);
        let Some(g0) = surface.tool_gap(&profile, &line(start, axis), &GapQuery::pose(8.0)) else {
            return Ok(());
        };
        let tip = start + UP * (height - g0.upper);
        let v = Vec3::new(sweep.0, sweep.1, sweep.2);
        let x = Vec3::new(1.0, 0.0, 0.0);
        let centre = Point3::new(0.0, 3.0 * turn.0, -6.0 + turn.0);
        let rotation = Rotation { point: centre, axis: UnitVec3::new(x).unwrap(), angle: turn.1 };
        let query = GapQuery::swept(v, 1.25 * allowed)
            .rotating(rotation)
            .accept_above(-allowed)
            .within_pieces(0);
        let ours = surface.tool_gap(&profile, &line(tip, axis), &query);
        if ours.is_none_or(|g| g.lower >= -allowed) {
            let settings = Settings { margin: 5.0, tolerance: 1e-4, budget: 20_000 };
            for k in 0..=24 {
                let s = f64::from(k) / 24.0;
                let a = s * turn.1;
                let at_tip = centre + turn_vector(tip + v * s - centre, x, a);
                let at_axis = turn_vector(unit, x, a);
                let at = interval(geometry.face_gap(&tool, at_tip, at_axis, 1.25 * allowed, allowed, &settings));
                prop_assert!(at.upper >= -allowed - 1e-6, "{ours:?} above {at:?} at {s}");
            }
        }
    }
}
