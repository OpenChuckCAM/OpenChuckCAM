// SPDX-License-Identifier: AGPL-3.0-or-later
//! Simultaneous finishing beside a wall thinner than the move check's layer
//! (OP-12; decision record SIMUL-EDGE-1, Terms: *Layer* and *Thin wall*,
//! the owner's instruction of 26 Nov 2026): the moves are left out, as for
//! a gouge, but the warning says the wall is too thin, not that the tool
//! would gouge it.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use std::collections::BTreeMap;

use openchuck_path::params::names;
use openchuck_path::{
    GeneratedPath, MeshSurface, ParameterValue, SimultaneousFinishing, UnreachedReason,
};
use openchuck_types::geometry::Point3;
use openchuck_types::mesh::TriangleMesh;

mod support;
use support::{Fixture, ball, generate, run};

const TOLERANCE: f64 = 0.01;

/// A tube along X from 0 to `length`: outer radius `outer`, wall `wall`
/// thick, its ends closed by rings, `around` facets round, every triangle
/// wound counter-clockwise seen from outside the material.
fn tube(outer: f64, wall: f64, length: f64, around: u32) -> TriangleMesh {
    let inner = outer - wall;
    let mut vertices = Vec::new();
    // Rings: 0 outer at x 0, 1 outer at length, 2 inner at x 0, 3 inner at
    // length.
    for (r, x) in [(outer, 0.0), (outer, length), (inner, 0.0), (inner, length)] {
        for i in 0..around {
            let t = std::f64::consts::TAU * f64::from(i) / f64::from(around);
            vertices.push(Point3::new(x, r * t.cos(), r * t.sin()));
        }
    }
    let at = |ring: u32, i: u32| ring * around + i % around;
    let mut triangles = Vec::new();
    for i in 0..around {
        let j = i + 1;
        // Outer face, normal outwards.
        triangles.push([at(0, i), at(0, j), at(1, j)]);
        triangles.push([at(0, i), at(1, j), at(1, i)]);
        // Inner face, normal towards the axis.
        triangles.push([at(2, i), at(3, j), at(2, j)]);
        triangles.push([at(2, i), at(3, i), at(3, j)]);
        // The end at x 0, normal along -X; the end at length, along +X.
        triangles.push([at(0, i), at(2, j), at(0, j)]);
        triangles.push([at(0, i), at(2, i), at(2, j)]);
        triangles.push([at(1, i), at(1, j), at(3, j)]);
        triangles.push([at(1, i), at(3, j), at(3, i)]);
    }
    TriangleMesh {
        vertices,
        triangles,
    }
}

fn finish(wall: f64) -> GeneratedPath {
    let outer = 8.0;
    let f = Fixture::new(
        MeshSurface::new(&tube(outer, wall, 6.0, 96)).unwrap(),
        outer + 1.0,
        &ball(3.0),
    );
    let v: BTreeMap<String, ParameterValue> = [
        (names::STEPOVER, 1.5),
        (names::TOLERANCE, TOLERANCE),
        (names::FEED, 300.0),
        (names::PLUNGE_FEED, 100.0),
    ]
    .into_iter()
    .map(|(k, x)| (k.to_owned(), ParameterValue::Number(x)))
    .collect();
    run(|ctx| generate(&SimultaneousFinishing, &f.input(), &v, ctx)).unwrap()
}

fn texts(path: &GeneratedPath) -> Vec<String> {
    path.notices.iter().map(|n| n.text.clone()).collect()
}

#[test]
fn req_op_12_a_wall_thinner_than_the_layer_is_reported_as_a_thin_wall_not_a_gouge() {
    // A tube 0.004 mm thick, finished on its outside: the tool lies within
    // the layer (0.0125 mm) of the inside, so no move can be proven and the
    // moves are left out; the warning names the wall and its thickness and
    // says nothing would gouge.
    let path = finish(0.004);
    let texts = texts(&path);
    let thin: Vec<_> = texts
        .iter()
        .filter(|t| t.contains("where a wall is thinner than 0.0125 mm"))
        .collect();
    assert_eq!(thin.len(), 1, "{texts:#?}");
    assert!(
        thin[0].contains("a quarter more than the tolerance of 0.01 mm"),
        "{}",
        thin[0]
    );
    assert!(
        thin[0].contains("a wall about 0.004 mm thick"),
        "{}",
        thin[0]
    );
    assert!(
        thin[0].contains("unlikely to survive milling"),
        "{}",
        thin[0]
    );
    assert!(
        !texts.iter().any(|t| t.contains("would gouge")),
        "{texts:#?}"
    );
    assert!(!path.unreached.is_empty());
    for u in &path.unreached {
        match u.reason {
            UnreachedReason::ThinWall {
                thickness,
                layer,
                face,
            } => {
                assert!((thickness - 0.004).abs() < 1e-4, "{thickness}");
                assert!((layer - 1.25 * TOLERANCE).abs() < 1e-12, "{layer}");
                assert_eq!(face, None, "a mesh has no faces to name");
            }
            r => panic!("{r:?}"),
        }
    }
}

#[test]
fn req_op_12_a_wall_thicker_than_the_layer_gets_neither_warning() {
    let path = finish(1.0);
    let texts = texts(&path);
    assert!(
        !texts
            .iter()
            .any(|t| t.contains("thinner than") || t.contains("would gouge")),
        "{texts:#?}"
    );
    assert!(path.unreached.is_empty(), "{:?}", path.unreached);
}
