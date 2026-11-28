// SPDX-License-Identifier: AGPL-3.0-or-later
//! The gap over a tessellation standing for an exact curved face, judged
//! against the face itself (decision record SIMUL-EDGE-1, Q2 and the
//! project owner's answers of 27 Nov 2026, relayed): a sphere, tessellated
//! with its corners on it, whose exact depth below it is in closed form.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use openchuck_types::geometry::{Point3, UnitVec3, Vec3};
use openchuck_types::mesh::TriangleMesh;
use openchuck_types::tool::profile::{ProfilePoint, ProfileSegment, RevolvedProfile, ToolProfile};

use super::{Band, GapQuery};
use crate::cutter::ToolLine;
use crate::surface::MeshSurface;

const RADIUS: f64 = 3.5;
const TOLERANCE: f64 = 0.01;

/// A sphere of `RADIUS` about the origin: an icosahedron split `levels`
/// times, its corners pushed onto the sphere, wound outwards.
fn sphere(levels: u32) -> TriangleMesh {
    let g = 0.5 * (1.0 + 5.0_f64.sqrt());
    let mut vertices: Vec<Point3> = [
        (-1.0, g, 0.0),
        (1.0, g, 0.0),
        (-1.0, -g, 0.0),
        (1.0, -g, 0.0),
        (0.0, -1.0, g),
        (0.0, 1.0, g),
        (0.0, -1.0, -g),
        (0.0, 1.0, -g),
        (g, 0.0, -1.0),
        (g, 0.0, 1.0),
        (-g, 0.0, -1.0),
        (-g, 0.0, 1.0),
    ]
    .iter()
    .map(|&(x, y, z)| on_sphere(Vec3::new(x, y, z)))
    .collect();
    let mut triangles: Vec<[u32; 3]> = vec![
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];
    for _ in 0..levels {
        let mut middles = std::collections::BTreeMap::new();
        let mut middle = |a: u32, b: u32, vertices: &mut Vec<Point3>| {
            *middles.entry((a.min(b), a.max(b))).or_insert_with(|| {
                let (p, q) = (vertices[a as usize], vertices[b as usize]);
                vertices.push(on_sphere((p - Point3::ORIGIN) + (q - Point3::ORIGIN)));
                u32::try_from(vertices.len() - 1).unwrap()
            })
        };
        let mut next = Vec::new();
        for [a, b, c] in triangles {
            let (ab, bc, ca) = (
                middle(a, b, &mut vertices),
                middle(b, c, &mut vertices),
                middle(c, a, &mut vertices),
            );
            next.extend([[a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]]);
        }
        triangles = next;
    }
    TriangleMesh {
        vertices,
        triangles,
    }
}

fn on_sphere(v: Vec3) -> Point3 {
    Point3::ORIGIN + v * (RADIUS / v.length())
}

/// How far the sphere lies outside and inside each triangle: a triangle with
/// its corners on a sphere lies inside it, by at most the radius less the
/// distance of its nearest point from the centre.
fn bands(mesh: &TriangleMesh) -> Vec<Band> {
    (0..mesh.triangles.len())
        .map(|i| {
            let t = mesh.triangle(i).unwrap();
            let near =
                crate::surface::closest_on_triangle(Point3::ORIGIN, &t).distance(Point3::ORIGIN);
            Band {
                above: RADIUS - near,
                below: 0.0,
            }
        })
        .collect()
}

fn ball(radius: f64) -> RevolvedProfile {
    ToolProfile {
        segments: vec![
            ProfileSegment::Arc {
                to: ProfilePoint::new(radius, radius),
                centre: ProfilePoint::new(0.0, radius),
            },
            ProfileSegment::Line {
                to: ProfilePoint::new(radius, 4.0 * radius),
            },
        ],
    }
    .validate()
    .unwrap()
}

/// A ball of radius `r` pressed straight in towards the centre along `u`,
/// its deepest point `depth` inside the sphere (the exact depth below the
/// face along its normal, from its foot: the sphere's radius less the
/// point's distance from the centre).
fn pressed(u: Vec3, r: f64, depth: f64) -> ToolLine {
    let u = u * (1.0 / u.length());
    let centre = Point3::ORIGIN + u * (RADIUS + r - depth);
    ToolLine {
        origin: centre + u * (-r),
        axis: UnitVec3::new(u).unwrap(),
    }
}

/// The move check's question at one pose: counted to 1.25 times the
/// tolerance, answered once proven no deeper than the tolerance.
fn check(surface: &MeshSurface, mesh: &TriangleMesh, line: &ToolLine) -> super::ToolGap {
    let bands = bands(mesh);
    let largest = bands.iter().map(|b| b.most()).fold(0.0, f64::max);
    surface
        .tool_gap_with(
            &ball(1.0),
            line,
            &GapQuery::pose(1.25 * TOLERANCE).accept_above(-TOLERANCE),
            &|i| bands[i as usize],
            largest,
            &|_| Some(0),
        )
        .unwrap()
}

#[test]
fn req_op_12_a_cut_past_the_tolerance_beside_a_near_level_edge_is_not_passed() {
    // A ball pressed into the sphere at the middle of an edge between two
    // triangles, its deepest point 0.0115 mm inside the exact face: more
    // than the tolerance of 0.01 mm. Measured from the triangles the point
    // is only about 0.009 mm deep (the edge is a chord, below the face), so
    // it is less than the tolerance below both triangles' planes. With the
    // threshold the tolerance itself, neither triangle counts it and the
    // answer read no cut, less the error: a pass of a real overcut. With
    // the error in the threshold it counts, and the pose fails.
    let mesh = sphere(4);
    let surface = MeshSurface::new(&mesh).unwrap();
    let bands = bands(&mesh);
    let largest = bands.iter().map(|b| b.most()).fold(0.0, f64::max);
    assert!(
        largest > 0.25 * TOLERANCE && largest < 0.5 * TOLERANCE,
        "the fixture's error {largest}"
    );
    let [a, b, _] = mesh.triangles[0];
    let (p, q) = (mesh.vertices[a as usize], mesh.vertices[b as usize]);
    let edge = (p - Point3::ORIGIN) + (q - Point3::ORIGIN);
    let sag = RADIUS - (Point3::ORIGIN + edge * 0.5).distance(Point3::ORIGIN);
    let depth = 0.0115;
    assert!(depth - sag < TOLERANCE, "the edge's sag {sag}");
    let g = check(&surface, &mesh, &pressed(edge, 1.0, depth));
    assert!(g.lower < -TOLERANCE, "{g:?} passes a cut {depth} deep");
}

#[test]
fn req_op_12_a_cut_well_within_the_tolerance_on_a_curved_face_passes() {
    // The same places, cut less than the tolerance by more than the error:
    // the answer must pass them (no false gouge from the tessellation).
    let mesh = sphere(4);
    let surface = MeshSurface::new(&mesh).unwrap();
    let largest = bands(&mesh).iter().map(|b| b.most()).fold(0.0, f64::max);
    let [a, b, c] = mesh.triangles[0];
    let (p, q, r) = (
        mesh.vertices[a as usize] - Point3::ORIGIN,
        mesh.vertices[b as usize] - Point3::ORIGIN,
        mesh.vertices[c as usize] - Point3::ORIGIN,
    );
    for (u, name) in [
        (p + q, "an edge"),
        (p + q + r, "a triangle's middle"),
        (p, "a corner"),
    ] {
        let depth = TOLERANCE - largest - 0.001;
        let g = check(&surface, &mesh, &pressed(u, 1.0, depth));
        assert!(
            g.lower >= -TOLERANCE,
            "at {name}, {g:?} fails a cut {depth} deep"
        );
    }
}

proptest::proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig::with_cases(64))]

    #[test]
    fn req_op_12_the_gap_over_a_curved_face_never_passes_a_cut_past_the_tolerance(
        x in -1.0_f64..1.0,
        y in -1.0_f64..1.0,
        z in -1.0_f64..1.0,
        depth in 0.0_f64..0.02,
        radius in 0.3_f64..1.5,
    ) {
        // A ball pressed into the sphere anywhere, to any depth up to twice
        // the tolerance, judged against the exact sphere (its depth along the
        // radius, in closed form), not the triangles.
        let u = Vec3::new(x, y, z);
        proptest::prop_assume!(u.length() > 0.1);
        let mesh = sphere(4);
        let surface = MeshSurface::new(&mesh).unwrap();
        let bands = bands(&mesh);
        let largest = bands.iter().map(|b| b.most()).fold(0.0, f64::max);
        let line = pressed(u, radius, depth);
        let g = surface
            .tool_gap_with(
                &ball(radius),
                &line,
                &GapQuery::pose(1.25 * TOLERANCE).accept_above(-TOLERANCE),
                &|i| bands[i as usize],
                largest,
                &|_| Some(0),
            )
            .unwrap();
        let passed = g.lower >= -TOLERANCE;
        if depth > TOLERANCE + 1e-9 {
            proptest::prop_assert!(!passed, "{g:?} passes a cut {depth} deep");
        }
        // Clear of the error band (the error lies outside the triangles
        // only, so the band is one error wide), it passes.
        if depth < TOLERANCE - largest - 1e-4 {
            proptest::prop_assert!(passed, "{g:?} fails a cut {depth} deep");
        }
    }
}
