// SPDX-License-Identifier: AGPL-3.0-or-later
//! Analytic test parts (NFR-08; work breakdown slice 0 item 10).

// Test code may unwrap, index and panic (NFR-07 applies to library code).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::float_cmp
)]

use std::collections::BTreeMap;

use openchuck_testkit::parts::{RadialPart, RuledHyperboloid, benchmark_model_1m, standard_parts};

/// Every edge is shared by exactly two triangles, in opposite directions.
fn assert_closed(mesh: &openchuck_types::mesh::TriangleMesh, name: &str) {
    let mut edges: BTreeMap<(u32, u32), i32> = BTreeMap::new();
    for t in &mesh.triangles {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            assert_ne!(a, b, "{name}: degenerate triangle {t:?}");
            *edges.entry((a.min(b), a.max(b))).or_default() += if a < b { 1 } else { -1 };
        }
    }
    for (edge, balance) in edges {
        assert_eq!(
            balance, 0,
            "{name}: edge {edge:?} is open or inconsistently oriented"
        );
    }
}

#[test]
fn req_nfr_08_parts_lie_on_their_analytic_surface() {
    for part in standard_parts() {
        let mesh = part.mesh(41, 64);
        mesh.validate().unwrap();
        let (x0, x1) = part.x_range();
        for v in &mesh.vertices {
            if v.y == 0.0 && v.z == 0.0 {
                continue; // cap centres and poles
            }
            let angle = v.z.atan2(v.y);
            let r = v.y.hypot(v.z);
            assert!(v.x >= x0 - 1e-12 && v.x <= x1 + 1e-12);
            let exact = part.radius_at(v.x, angle);
            assert!(
                (r - exact).abs() < 1e-9,
                "{}: r {r} vs {exact}",
                part.name()
            );
        }
        assert_closed(&mesh, part.name());
        // Outward normals give a positive signed volume.
        let volume: f64 = (0..mesh.triangles.len())
            .map(|i| {
                let [p, q, r] = mesh.triangle(i).unwrap();
                p.to_vec().dot(q.to_vec().cross(r.to_vec())) / 6.0
            })
            .sum();
        assert!(volume > 0.0, "{}: normals point inwards", part.name());
    }
}

#[test]
fn req_nfr_08_ruled_surface_contains_its_rulings() {
    let part = RuledHyperboloid {
        end_radius: 15.0,
        length: 40.0,
        twist: 1.0,
    };
    for k in 0..12 {
        let (p, q) = part.ruling(f64::from(k) * 0.5);
        for i in 0..=10 {
            let t = f64::from(i) / 10.0;
            let x = p.x + (q.x - p.x) * t;
            let y = p.y + (q.y - p.y) * t;
            let z = p.z + (q.z - p.z) * t;
            let exact = part.radius_at(x, z.atan2(y));
            assert!((y.hypot(z) - exact).abs() < 1e-9);
        }
    }
}

#[test]
fn req_nfr_03_benchmark_model_has_about_one_million_triangles() {
    let mesh = benchmark_model_1m();
    assert_eq!(mesh.triangles.len(), 1_002_000);
    mesh.validate().unwrap();
}

/// The volume enclosed by a closed mesh (positive when its normals point out).
fn volume(mesh: &openchuck_types::mesh::TriangleMesh) -> f64 {
    mesh.triangles
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|i| mesh.vertices[i as usize]);
            a.to_vec().dot(b.to_vec().cross(c.to_vec())) / 6.0
        })
        .sum()
}

#[test]
fn req_op_20_a_prism_with_a_keyway_is_closed_outward_and_of_its_polygon() {
    // A shaft section with a keyway: not star-shaped about the axis, so the
    // caps need ear clipping (decision record 0368's test parts).
    let mut section: Vec<(f64, f64)> = Vec::new();
    let (r, half, bottom) = (10.0_f64, 2.0_f64, 7.0_f64);
    let start = (half / r).asin();
    let n = 64;
    // Counter-clockwise from the slot's left top edge round to its right,
    // then down the slot's right wall, across its bottom and up.
    for k in 0..=n {
        let t = std::f64::consts::FRAC_PI_2
            + start
            + (std::f64::consts::TAU - 2.0 * start) * f64::from(k) / f64::from(n);
        section.push((r * t.cos(), r * t.sin()));
    }
    section.push((half, bottom));
    section.push((-half, bottom));
    let area: f64 = 0.5
        * section
            .iter()
            .zip(section.iter().cycle().skip(1))
            .map(|(a, b)| a.0 * b.1 - b.0 * a.1)
            .sum::<f64>();
    assert!(area > 0.0, "{area}");
    let mesh = openchuck_testkit::parts::prism(&section, 0.0, 20.0, 40);
    mesh.validate().unwrap();
    assert_closed(&mesh, "keyway prism");
    assert!(
        (volume(&mesh) - 20.0 * area).abs() < 1e-6 * area,
        "{} {area}",
        volume(&mesh)
    );
    // Two prisms side by side stay two closed pieces.
    let pin = |cy: f64| -> Vec<(f64, f64)> {
        (0..32)
            .map(|k| {
                let t = std::f64::consts::TAU * f64::from(k) / 32.0;
                (cy + 4.0 * t.cos(), 6.0 + 4.0 * t.sin())
            })
            .collect()
    };
    let both = openchuck_testkit::parts::join(&[
        openchuck_testkit::parts::prism(&pin(-6.0), 0.0, 10.0, 20),
        openchuck_testkit::parts::prism(&pin(6.0), 0.0, 10.0, 20),
    ]);
    assert_closed(&both, "two pins");
    assert!(volume(&both) > 0.0);
}

#[test]
fn req_op_20_a_revolved_profile_may_turn_back_for_an_undercut() {
    // A dovetail groove round the axis: the profile turns back in X under
    // each lip, and the surface stays closed and outward.
    let mesh = openchuck_testkit::parts::revolve(
        &[
            (0.0, 10.0),
            (11.0, 10.0),
            (10.0, 6.0),
            (20.0, 6.0),
            (19.0, 10.0),
            (30.0, 10.0),
        ],
        180,
    );
    assert_closed(&mesh, "dovetail groove");
    assert!(volume(&mesh) > 0.0);
}

/// The gate 5 reference part is a stepped cylinder (decision record STEPPED-CYL-A,
/// issue #826): each step is a cylinder of its own radius, and the faces
/// between steps are square to the axis, so a micrometer reads a step's
/// diameter wherever along it it sits and a face gives a position along X.
#[test]
fn req_nfr_08_a_stepped_cylinder_has_cylindrical_steps_and_square_faces() {
    use openchuck_testkit::parts::{CylinderStep, SteppedCylinder};
    let part = SteppedCylinder {
        steps: vec![
            CylinderStep {
                radius: 12.0,
                length: 7.0,
            },
            CylinderStep {
                radius: 8.5,
                length: 8.0,
            },
            CylinderStep {
                radius: 5.0,
                length: 10.0,
            },
        ],
    };
    assert_eq!(part.x_range(), (0.0, 25.0));
    assert_eq!(part.faces(), vec![7.0, 15.0]);
    // The radius is the step's at every angle, from its start face up to
    // (not including) the next; the last step includes the end.
    for (x, r) in [
        (0.0, 12.0),
        (3.5, 12.0),
        (6.999, 12.0),
        (7.0, 8.5),
        (14.999, 8.5),
        (15.0, 5.0),
        (25.0, 5.0),
    ] {
        for angle in [0.0, 1.0, 4.0] {
            assert_eq!(part.radius_at(x, angle), r, "x {x}, angle {angle}");
        }
    }
    let around = 360;
    let mesh = part.mesh(2, around);
    mesh.validate().unwrap();
    assert_closed(&mesh, "stepped cylinder");
    // Every vertex is on a step's circle at one of the step boundaries,
    // or on the axis at a cap; the faces are exact annuli at X 7 and 15.
    let boundaries = [0.0, 7.0, 15.0, 25.0];
    for v in &mesh.vertices {
        assert!(boundaries.contains(&v.x), "vertex off a boundary: {v:?}");
        let r = v.y.hypot(v.z);
        assert!(
            r < 1e-12 || [12.0, 8.5, 5.0].iter().any(|s| (r - s).abs() < 1e-9),
            "vertex off the steps: {v:?}"
        );
    }
    // Each triangle is on one step's side (radius constant) or on one face
    // or cap (X constant): no face is sloped.
    for t in &mesh.triangles {
        let [a, b, c] = t.map(|i| mesh.vertices[i as usize]);
        let flat = a.x == b.x && b.x == c.x;
        let side = [a, b, c]
            .map(|p| p.y.hypot(p.z))
            .windows(2)
            .all(|w| (w[0] - w[1]).abs() < 1e-9);
        assert!(flat || side, "sloped triangle {a:?} {b:?} {c:?}");
    }
    // The volume is the steps' cylinders within the facets' chord error.
    let exact = std::f64::consts::PI * (144.0 * 7.0 + 72.25 * 8.0 + 25.0 * 10.0);
    let v = volume(&mesh);
    assert!(
        v > 0.0 && v < exact && (exact - v) / exact < 1e-3,
        "{v} against {exact}"
    );
}
