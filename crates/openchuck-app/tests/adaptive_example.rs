// SPDX-License-Identifier: AGPL-3.0-or-later
//! The tutorial's turned piece with adaptive spacing on (issue #462;
//! decision record 0239 items 8, 10 and 11, as amended by the PATHBUGS
//! record): helical finishing with its defaults makes a complete path, not
//! a partial one that must be acknowledged before export (OP-04, NFR-07,
//! NFR-12).
#![allow(clippy::unwrap_used, clippy::panic)]

use std::path::{Path, PathBuf};

use openchuck_app::{Application, Threads};

/// A copy of the example project in a fresh folder, with the Finish
/// operation's adaptive spacing set as `adaptive`.
fn example(tag: &str, adaptive: bool) -> PathBuf {
    let from = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata/examples/baluster/baluster-project");
    let dir = std::env::temp_dir().join(format!(
        "openchuck-adaptive-example-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    copy(&from, &dir);
    let finish = dir.join("operations/34e3aa5e-36cc-4614-be6d-ec39cf22f5f8.toml");
    let text = std::fs::read_to_string(&finish).unwrap();
    assert!(text.contains("strategy = \"helical_finishing\""));
    let text = text.replace(
        "adaptive_spacing = false",
        &format!("adaptive_spacing = {adaptive}"),
    );
    std::fs::write(&finish, text).unwrap();
    dir
}

fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

#[test]
fn req_op_04_req_nfr_12_the_tutorial_piece_finishes_completely_with_adaptive_spacing() {
    // Before: the query limit was met with half of it unused, and places
    // beside the flutes' edges were left unverified, so the path was
    // partial (issue #462).
    let dir = example("on", true);
    let app = Application::open(&dir, Threads::Exactly(4)).unwrap();
    let run = app.regenerate().unwrap();
    let partial = openchuck_app::partial_operations(app.project(), &run);
    let warnings: Vec<&str> = run
        .groups
        .iter()
        .flat_map(|g| &g.warnings)
        .filter(|w| w.acknowledgement.is_some())
        .map(|w| w.text.as_str())
        .collect();
    assert!(partial.is_empty(), "{partial:?}: {warnings:#?}");
    assert!(
        run.notes
            .iter()
            .any(|n| n.contains("Finish") && n.contains("passes added")),
        "{:#?}",
        run.notes
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The radius of material the Finish operation's cutting moves leave on the
/// rotary axis's side of the part past `face`, at each X in `xs`: the
/// least, over the moves whose ball (radius `r`, centred `r` up the tool
/// axis from the tip) reaches that X, of how far the ball's surface stays
/// from the axis there. The moves turn round the axis, so what they leave
/// is a solid of revolution.
fn left_past(run: &openchuck_app::Regeneration, r: f64, xs: &[f64]) -> Vec<f64> {
    let finish: openchuck_project::id::OperationId =
        "34e3aa5e-36cc-4614-be6d-ec39cf22f5f8".parse().unwrap();
    let index = run
        .groups
        .iter()
        .position(|g| g.operation == finish)
        .unwrap();
    let group = &run.canonical_toolpath.groups[index];
    let centres: Vec<(f64, f64)> = group
        .segments
        .iter()
        .filter(|s| s.kind == openchuck_types::ir::MoveKind::Cutting)
        .map(|s| {
            let c = s.end.tip + s.end.axis.get() * r;
            (c.x, c.y.hypot(c.z))
        })
        .collect();
    xs.iter()
        .map(|&x| {
            centres
                .iter()
                .filter(|&&(cx, _)| (cx - x).abs() < r)
                .map(|&(cx, rho)| (rho - (r * r - (cx - x).powi(2)).sqrt()).max(0.0))
                .fold(f64::INFINITY, f64::min)
        })
        .collect()
}

#[test]
fn req_op_04_req_nfr_12_the_tutorial_piece_is_finished_to_a_point() {
    // Decision 0695, Q2 (a) A, (b) A, (c) B (issue #779): with Finish to a
    // point on, the finial's point is cut down to the axis instead of
    // keeping a nub (0.733 mm at 0.25 mm past it before, the record's
    // table), and adaptive spacing confirms the material past the point:
    // the path is complete.
    let dir = example("point", true);
    let finish = dir.join("operations/34e3aa5e-36cc-4614-be6d-ec39cf22f5f8.toml");
    let text = std::fs::read_to_string(&finish).unwrap().replace(
        "adaptive_spacing = true",
        "adaptive_spacing = true\nfinish_to_a_point = true",
    );
    std::fs::write(&finish, text).unwrap();
    let app = Application::open(&dir, Threads::Exactly(4)).unwrap();
    let run = app.regenerate().unwrap();
    assert!(openchuck_app::partial_operations(app.project(), &run).is_empty());
    // The free end the Finish faces is the finial's point.
    let face: f64 = run
        .notes
        .iter()
        .find_map(|n| {
            let at = n.find("end face at X ")? + "end face at X ".len();
            n[at..].split(' ').next()?.parse().ok()
        })
        .unwrap();
    let r = 3.175 / 2.0;
    let xs: Vec<f64> = [0.25, 0.5, 0.75, 1.0, 1.25]
        .iter()
        .map(|d| face + d)
        .collect();
    let on = left_past(&run, r, &xs);
    assert!(on.iter().all(|&l| l < 0.05), "left past X {face}: {on:?}");
    let _ = std::fs::remove_dir_all(&dir);
}
