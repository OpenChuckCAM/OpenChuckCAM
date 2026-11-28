// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact evaluation against the kernel (IMP-02; decision records 0017 and 0027): the SP-9 comparison, kept
//! as a regression test. Each STEP fixture is imported, its faces become the library's pure-Rust
//! `ExactFace`s, and point, normal, closest point, ray hit and trim test are compared with what OpenCASCADE
//! answers for the same face. Needs the kernel: `--features kernel-tests`.
#![cfg(feature = "kernel-tests")]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::indexing_slicing,
    clippy::cast_precision_loss
)]

use std::path::{Path, PathBuf};

use openchuck_import::kernel::Oracle;
use openchuck_import::step::StepImporter;
use openchuck_import::{ImportOptions, ModelImporter};
use openchuck_path::{BrepSurface, ExactFace, ParametricFace, SurfaceSource, Uv};
use openchuck_types::brep::{BrepModel, Surface, SurfaceKind};
use openchuck_types::geometry::{Point3, UnitVec3, Vec3};
use openchuck_types::job::{CancelToken, JobContext, NoProgress};
use openchuck_types::model::ModelGeometry;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata/step/valid")
        .join(format!("{name}.step"))
}

fn import(name: &str) -> BrepModel {
    let bytes = std::fs::read(fixture(name)).unwrap();
    let (cancel, progress) = (CancelToken::new(), NoProgress);
    let model = StepImporter::new()
        .import(
            &bytes,
            &ImportOptions::default(),
            &JobContext::new(&cancel, &progress),
        )
        .unwrap();
    match model.geometry {
        ModelGeometry::Brep(b) => b,
        ModelGeometry::Mesh(_) => panic!("not a B-rep"),
    }
}

fn angle(a: Vec3, b: [f64; 3]) -> f64 {
    let b = Vec3::new(b[0], b[1], b[2]);
    (a.dot(b) / (a.length() * b.length()))
        .clamp(-1.0, 1.0)
        .acos()
}

#[derive(Default)]
struct Worst {
    point: f64,
    normal: f64,
    distance: f64,
    ray: f64,
    trim_mismatch: usize,
    trim_total: usize,
    samples: usize,
    rays: usize,
}

const NAMES: &[&str] = &[
    "box",
    "cylinder",
    "cone",
    "sphere",
    "torus",
    "filleted_box",
    "rotary_blank_drilled",
    "bspline_face",
    "revolved_spline",
    "revolved_spline_partial",
    "extruded_spline",
    "hex_bar",
];

#[test]
fn req_imp_02_exact_faces_agree_with_the_kernel() {
    const N: usize = 9;
    let mut worst = Worst::default();
    for name in NAMES {
        let model = import(name);
        let oracle =
            Oracle::open(fixture(name).to_str().unwrap()).expect("the kernel opens the fixture");
        for f in &model.faces {
            let face = ExactFace::new(f).unwrap();
            let id = f.id.0;
            let [u0, u1, v0, v1] = f.uv_bounds;
            let analytic = !matches!(f.geometry, Surface::BSpline(_));
            for iu in 0..N {
                for iv in 0..N {
                    let uv = Uv {
                        u: u0 + (u1 - u0) * (iu as f64 + 0.5) / N as f64,
                        v: v0 + (v1 - v0) * (iv as f64 + 0.5) / N as f64,
                    };
                    // A converted face (surface of revolution, extrusion, ...) has the patch's own
                    // parameters, not the kernel's: ask the kernel about the same 3D point instead.
                    let converted = f.surface == SurfaceKind::Other;
                    let Some(ours) = face.point_normal(uv) else {
                        continue;
                    };
                    let kernel_uv = if converted {
                        match oracle.project(id, [ours.point.x, ours.point.y, ours.point.z]) {
                            Some((pu, pv, _, d)) if d < 1e-6 => Some((pu, pv)),
                            Some((_, _, _, d)) => panic!(
                                "{name} face {id}: our point is {d:e} mm off the kernel's surface"
                            ),
                            None => None,
                        }
                    } else {
                        Some((uv.u, uv.v))
                    };
                    let Some((ku, kv)) = kernel_uv else { continue };
                    // Trim classification against the kernel's own classifier.
                    if let Some(k) = oracle.contains(id, ku, kv) {
                        worst.trim_total += 1;
                        if k != face.contains(uv) {
                            worst.trim_mismatch += 1;
                        }
                    }
                    if !face.contains(uv) {
                        continue;
                    }
                    let Some((kp, kn)) = oracle.eval(id, ku, kv) else {
                        continue;
                    };
                    worst.samples += 1;
                    let dp = ours.point.distance(Point3::new(kp[0], kp[1], kp[2]));
                    if dp > 1e-6 {
                        eprintln!(
                            "{name} face {id} ({:?}, reversed {}): point differs by {dp:e}",
                            f.surface, f.reversed
                        );
                    }
                    worst.point = worst.point.max(dp);
                    worst.normal = worst.normal.max(angle(ours.normal.get(), kn));
                    // Closest point from a point 0.25 mm off the surface on the outward side.
                    let n = ours.normal.get();
                    let q = ours.point + n * 0.25;
                    if let Some(c) = face.closest(q) {
                        // The kernel projects onto the untrimmed surface; compare when that point is on the face.
                        if let Some((pu, pv, _, d)) = oracle.project(id, [q.x, q.y, q.z])
                            && oracle.contains(id, pu, pv) == Some(true)
                            && (!analytic || d < 0.3)
                        {
                            worst.distance = worst.distance.max((c.distance - d).abs());
                        }
                    }
                    // A ray from 3 mm out along the inward normal hits the face where it started (or earlier).
                    let origin = ours.point + n * 3.0;
                    let dir = UnitVec3::new(-n).unwrap();
                    let hits = face.ray_hits(origin, dir, 10.0);
                    let theirs = oracle.ray(
                        id,
                        [origin.x, origin.y, origin.z],
                        [dir.get().x, dir.get().y, dir.get().z],
                        10.0,
                    );
                    worst.rays += 1;
                    match (hits.first(), theirs.first()) {
                        (Some(h), Some(t)) => worst.ray = worst.ray.max((h.distance - t.0).abs()),
                        (None, None) => {}
                        (Some(h), None) => {
                            panic!(
                                "{name} face {id}: we hit at {} and the kernel misses",
                                h.distance
                            )
                        }
                        (None, Some(t)) => {
                            panic!("{name} face {id}: the kernel hits at {} and we miss", t.0)
                        }
                    }
                }
            }
        }
    }
    eprintln!(
        "worst over {} samples and {} rays: point {:.2e} mm, normal {:.2e} rad, distance {:.2e} mm, ray {:.2e} mm; trim mismatches {} of {}",
        worst.samples,
        worst.rays,
        worst.point,
        worst.normal,
        worst.distance,
        worst.ray,
        worst.trim_mismatch,
        worst.trim_total
    );
    assert!(worst.samples > 500);
    assert!(worst.point < 1e-8, "point {:e}", worst.point);
    assert!(worst.normal < 1e-5, "normal {:e}", worst.normal);
    assert!(worst.distance < 1e-6, "distance {:e}", worst.distance);
    assert!(worst.ray < 1e-6, "ray {:e}", worst.ray);
    // SP-9: 195 of 196 trim samples agree on the six end discs; allow 2 % here.
    assert!(
        (worst.trim_mismatch as f64) <= 0.02 * worst.trim_total as f64,
        "{} of {}",
        worst.trim_mismatch,
        worst.trim_total
    );
}

#[test]
fn req_imp_02_exact_surface_beats_the_tessellation_it_was_built_from() {
    // SP-9: the mesh is off the surface by up to 0.011 mm at 0.01/0.1. Projecting onto the exact
    // faces of the sphere gives the true distance; onto the mesh it does not.
    let model = import("sphere");
    let exact = BrepSurface::new(&model).unwrap();
    let mesh = openchuck_path::MeshSurface::new(exact.tessellation()).unwrap();
    let mut worst_exact = 0.0_f64;
    let mut worst_mesh = 0.0_f64;
    for i in 0..400 {
        let t = f64::from(i) * 2.399_963; // golden angle spiral over the sphere
        let z = 1.0 - 2.0 * (f64::from(i) + 0.5) / 400.0;
        let r = (1.0 - z * z).sqrt();
        let q = Point3::new(20.0 * r * t.cos(), 20.0 * r * t.sin(), 20.0 * z);
        let want = 5.0; // 20 from the centre, the sphere has radius 15
        let d_exact = exact.distance(q).unwrap();
        let d_mesh = mesh.distance(q).unwrap();
        worst_exact = worst_exact.max((d_exact - want).abs());
        worst_mesh = worst_mesh.max((d_mesh - want).abs());
    }
    eprintln!("sphere r15: exact {worst_exact:.2e} mm, mesh {worst_mesh:.2e} mm");
    assert!(worst_exact < 1e-9, "{worst_exact:e}");
    assert!(worst_mesh > worst_exact);
}

/// Issue #917 B (OI-2271), the owner's answer F1: each face's trim used to
/// be its p-curves sampled at 48 steps on their own, a chord of the true
/// edge, so beside a curved edge a strip up to the chord's sag (0.02 mm on
/// a round edge, 0.04 mm on the blisk) lay outside both faces' trims and a
/// ray through it hit nothing. The importer now samples each edge once
/// along its 3D curve to 1 µm and takes both faces' trims at those
/// parameters, so a ray aimed along the normal at a point 2 µm or 10 µm in
/// from any vertex of a shared edge hits that face.
#[test]
fn req_imp_02_a_ray_just_inside_a_shared_edge_hits_its_face() {
    const PARTS: &[&str] = &[
        "blisk",
        "box",
        "cone",
        "crankshaft",
        "cylinder",
        "extruded_spline",
        "filleted_box",
        "hex_bar",
        "multi_body",
        "rotary_blank_drilled",
        "shouldered_shaft",
    ];
    const IN: [f64; 2] = [0.002, 0.010];
    // Above the ray's target, along the face's normal.
    const ABOVE: f64 = 1.0;
    let mut misses = Vec::new();
    for name in PARTS {
        let model = import(name);
        let surface = BrepSurface::new(&model).unwrap();
        let faces = surface.faces();
        let (mut tried, mut missed) = (0usize, 0usize);
        for e in &model.edges {
            if e.faces.len() != 2 || e.polyline.len() < 2 {
                continue;
            }
            for (k, p) in e.polyline.iter().enumerate() {
                let (q0, q1) = match e.polyline.get(k + 1) {
                    Some(next) => (*p, *next),
                    None => (e.polyline[k - 1], *p),
                };
                let along = q1 - q0;
                if along.length() < 1e-9 {
                    continue;
                }
                let along = along * (1.0 / along.length());
                for f in &e.faces {
                    let Some(c) = faces[f.0 as usize].closest_within(*p, 0.1) else {
                        continue;
                    };
                    let n = c.point.normal.get();
                    let across = n.cross(along);
                    if across.length() < 1e-9 {
                        continue;
                    }
                    let across = across * (1.0 / across.length());
                    let hits = |target: Point3| {
                        surface
                            .cast_ray(target + n * ABOVE, UnitVec3::new(-n).unwrap(), 2.0 * ABOVE)
                            .is_some_and(|hit| {
                                (hit.distance - ABOVE).abs() < 0.05
                                    && hit.position.is_some_and(|at| at.face == f.0)
                            })
                    };
                    // The side of the edge the face is on: where it is hit
                    // well inside it.
                    let Some(side) = [1.0, -1.0]
                        .into_iter()
                        .find(|s| hits(*p + across * (0.3 * s)))
                    else {
                        continue;
                    };
                    for d in IN {
                        tried += 1;
                        if !hits(*p + across * (d * side)) {
                            missed += 1;
                        }
                    }
                }
            }
        }
        eprintln!("{name}: {missed} of {tried} rays just inside a shared edge miss its face");
        assert!(
            tried > 0 || ["hex_bar", "box"].contains(name),
            "{name}: nothing tried"
        );
        if missed > 0 {
            misses.push(format!("{name}: {missed} of {tried}"));
        }
    }
    assert!(misses.is_empty(), "rays that miss: {misses:?}");
}
