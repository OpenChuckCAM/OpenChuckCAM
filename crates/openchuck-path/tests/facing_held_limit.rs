// SPDX-License-Identifier: AGPL-3.0-or-later
//! OP-24, SET-09: beside a held region Facing faces what the tool reaches
//! with its clearance from the grip, leaves the rest of the requested
//! coverage unfaced and says where, why and how much in an information
//! note, not a warning (issue #500; decision 0685, amending 0252).
//! Expected numbers are worked by hand from the set-up below, never from the
//! generator's helpers.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::float_cmp,
    clippy::panic
)]
use openchuck_path::facing::plan::FacingSetup;
use openchuck_path::{
    AcceptAll, AnalyticStock, Facing, GeneratedPath, MeshSurface, ParameterValue, PathError,
    RotaryMapping, Solid, StrategyInput,
};
use openchuck_types::geometry::{Aabb, Point3};
use openchuck_types::ir::{MoveKind, PathTags};
use openchuck_types::job::{CancelToken, JobContext, NoProgress};
use openchuck_types::machine::MachineProfile;
use openchuck_types::mesh::TriangleMesh;
use openchuck_types::stock::Stock;
use openchuck_types::tool::{Tool, ToolShape};
use openchuck_types::units::Millimetres;
use std::collections::BTreeMap;

/// The flat tool's radius: 3 mm diameter.
const R: f64 = 1.5;
/// How far the whole tool keeps from a held region (decision 0161).
const CLEAR: f64 = 0.5;
/// The stock along X; the part (a 10 x 4 x 4 block) runs from X 10 to 20.
const STOCK: (f64, f64) = (5., 25.);

fn values(shape: &str, extent: &str) -> BTreeMap<String, ParameterValue> {
    let mut values: BTreeMap<String, ParameterValue> = [
        ("shape", ParameterValue::Text(shape.into())),
        ("extent", ParameterValue::Text(extent.into())),
        ("feed", ParameterValue::Number(600.)),
        ("plunge_feed", ParameterValue::Number(200.)),
        ("stepover", ParameterValue::Number(1.5)),
        ("stepdown", ParameterValue::Number(1.3)),
        ("stock_to_leave", ParameterValue::Number(0.2)),
    ]
    .into_iter()
    .map(|(k, v)| (k.into(), v))
    .collect();
    match shape {
        "box" => {
            values.insert("positive_z".into(), ParameterValue::Flag(true));
        }
        "flat" => {
            values.insert("angles".into(), ParameterValue::Text("0".into()));
            values.insert("size".into(), ParameterValue::Number(3.));
            values.insert("adaptive_spacing".into(), ParameterValue::Flag(false));
        }
        _ => {
            values.insert("size".into(), ParameterValue::Number(7.));
        }
    }
    values
}

/// Generate Facing on the block with the given held intervals, the host's
/// keep-out widened by the tool's radius and the clearance (or `keep_out`
/// when given), and hand the result to `f`.
fn generate<T>(
    values: &BTreeMap<String, ParameterValue>,
    held: [Option<(f64, f64)>; 2],
    keep_out: Option<Vec<(f64, f64)>>,
    f: impl FnOnce(Result<GeneratedPath, PathError>) -> T,
) -> T {
    let part = TriangleMesh {
        vertices: vec![
            Point3::new(10., -2., -2.),
            Point3::new(20., -2., -2.),
            Point3::new(10., 2., -2.),
            Point3::new(20., 2., -2.),
            Point3::new(10., -2., 2.),
            Point3::new(20., -2., 2.),
            Point3::new(10., 2., 2.),
            Point3::new(20., 2., 2.),
        ],
        triangles: vec![
            [0, 1, 2],
            [1, 3, 2],
            [4, 6, 5],
            [5, 6, 7],
            [0, 4, 1],
            [1, 4, 5],
            [2, 3, 6],
            [3, 7, 6],
            [0, 2, 4],
            [2, 6, 4],
            [1, 5, 3],
            [3, 5, 7],
        ],
    };
    // Round turns about the axis: a bar of 10 mm diameter, as it would be.
    let stock = if values.get("shape") == Some(&ParameterValue::Text("round".into())) {
        Stock::Cylinder {
            diameter: Millimetres(10.),
            length: Millimetres(STOCK.1 - STOCK.0),
            x_start: Millimetres(STOCK.0),
        }
    } else {
        Stock::Box {
            bounds: Aabb {
                min: Point3::new(STOCK.0, -4., -5.),
                max: Point3::new(STOCK.1, 4., 5.),
            },
        }
    };
    let tool = Tool {
        shape: ToolShape::FlatEnd,
        diameter: Millimetres(2. * R),
        flute_length: Millimetres(10.),
        overall_length: Millimetres(30.),
        shank_diameter: Millimetres(2. * R),
        flute_count: 2,
        material: None,
        centre_cutting: Some(true),
        stick_out: None,
    };
    let machine = MachineProfile::example_x_rotary();
    let protected: Vec<Solid> = held
        .iter()
        .flatten()
        .map(|&(lo, hi)| {
            Solid::cuboid(Aabb {
                min: Point3::new(lo, -4., -5.),
                max: Point3::new(hi, 4., 5.),
            })
            .unwrap()
        })
        .collect();
    let keep_out = keep_out.unwrap_or_else(|| {
        held.iter()
            .flatten()
            .map(|&(lo, hi)| (lo - R - CLEAR, hi + R + CLEAR))
            .collect()
    });
    let setup = FacingSetup {
        part: &part,
        stock: &stock,
        tool: &tool,
        machine: &machine,
        tabs: &[],
        protected: &protected,
        held,
        projection_error: Some(0.),
    };
    let token = CancelToken::new();
    let context = JobContext::new(&token, &NoProgress);
    let surface = MeshSurface::new(&part).unwrap();
    let profile = tool.profile().unwrap();
    let original = AnalyticStock::from_stock(&stock, None).unwrap();
    let tags = PathTags {
        operation: "facing".into(),
        setup: "setup".into(),
        tool: "tool".into(),
    };
    let input = StrategyInput {
        surface: &surface,
        stock: &original,
        collision: &AcceptAll,
        tool: &profile,
        rotary: RotaryMapping::of_profile(&machine),
        clearance: machine.clearance,
        max_feeds: machine.max_feeds,
        tags: &tags,
        facing: Some(&setup),
        keep_out: &keep_out,
        shank: None,
    };
    f(openchuck_path::generate(&Facing, &input, values, &context))
}

/// The information notes about the held region, after checking none of
/// them is a warning.
fn held_notes(path: &GeneratedPath) -> Vec<String> {
    let notes: Vec<_> = path
        .notices
        .iter()
        .filter(|n| n.topic.as_deref() == Some(openchuck_path::notice::topic::HELD_REGION))
        .collect();
    assert!(
        notes.iter().all(|n| !n.is_warning()),
        "the limit is information, not a warning: {notes:?}"
    );
    notes.iter().map(|n| n.text.clone()).collect()
}

/// The X span the final-depth cutting rows of a Box +Z face finish: the
/// union of each across-going row's [x - R, x + R], which must be one
/// interval. The finished plane is the part's top (Z 2) plus the 0.2 mm
/// stock to leave.
fn box_finished_span(path: &GeneratedPath) -> (f64, f64) {
    let mut rows = Vec::new();
    let mut from = path.group.start;
    for s in &path.group.segments {
        let to = s.end;
        if s.kind == MoveKind::Cutting
            && (from.tip.z - 2.2).abs() < 1e-6
            && (to.tip.z - 2.2).abs() < 1e-6
            && (from.tip.x - to.tip.x).abs() < 1e-9
            && (from.tip.y - to.tip.y).abs() > 1.
        {
            rows.push(from.tip.x);
        }
        from = to;
    }
    rows.sort_by(f64::total_cmp);
    assert!(!rows.is_empty());
    for w in rows.windows(2) {
        assert!(w[1] - w[0] <= 2. * R, "rows {w:?} leave a strip");
    }
    (rows[0] - R, rows[rows.len() - 1] + R)
}

/// No fed move's tip comes inside the held region widened by the tool's
/// radius and the clearance.
fn assert_clear(path: &GeneratedPath, held: [Option<(f64, f64)>; 2]) {
    let mut from = path.group.start;
    for s in &path.group.segments {
        let to = s.end;
        if s.kind.is_fed() {
            for (lo, hi) in held.iter().flatten() {
                let (a, b) = (from.tip.x.min(to.tip.x), from.tip.x.max(to.tip.x));
                assert!(
                    b <= lo - R - CLEAR + 1e-6 || a >= hi + R + CLEAR - 1e-6,
                    "a fed move from X {a} to {b} reaches over the grip ({lo}, {hi})"
                );
            }
        }
        from = to;
    }
}

#[test]
fn req_op_24_req_set_09_box_faces_up_to_the_held_start_and_says_what_it_leaves() {
    let held = [Some((5., 10.)), None];
    // Whole stock asks for X 5 to 25; the tool's edge stops 0.5 mm from the
    // grip at X 10.5, so X 5 to 10.5 (5.5 mm) is left.
    for (extent, left, length, end) in [("whole_stock", 5., 5.5, STOCK.1), ("part", 10., 0.5, 20.)]
    {
        generate(&values("box", extent), held, None, |path| {
            let path = path.unwrap_or_else(|e| panic!("{extent}: {e}"));
            assert_clear(&path, held);
            let (lo, hi) = box_finished_span(&path);
            assert!((lo - 10.5).abs() < 1e-9, "{extent}: finished from {lo}");
            assert!(hi >= end - 1e-9, "{extent}: finished to {hi}");
            let notes = held_notes(&path);
            assert_eq!(notes.len(), 1, "{notes:?}");
            let note = &notes[0];
            for part in [
                "held start (X 5.000 to 10.000)".to_string(),
                "within 0.500 mm".into(),
                "finished from X 10.500".into(),
                format!("X {left:.3} to 10.500 ({length:.3} mm along X)"),
                "left unfaced".into(),
            ] {
                assert!(note.contains(&part), "{extent}: {part} not in {note}");
            }
            assert!(
                !path.notices.iter().any(|n| n.is_warning()
                    && n.topic.as_deref() == Some(openchuck_path::notice::topic::HELD_REGION)),
                "{extent}"
            );
        });
    }
}

#[test]
fn req_op_24_req_set_09_box_faces_up_to_a_held_far_end_and_both_ends() {
    let far = [None, Some((22., 25.))];
    generate(&values("box", "whole_stock"), far, None, |path| {
        let path = path.unwrap();
        assert_clear(&path, far);
        let (lo, hi) = box_finished_span(&path);
        assert!(lo <= STOCK.0 + 1e-9);
        assert!((hi - 21.5).abs() < 1e-9, "finished to {hi}");
        let notes = held_notes(&path);
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert!(notes[0].contains("held far end (X 22.000 to 25.000)"));
        assert!(notes[0].contains("finished up to X 21.500"));
        assert!(notes[0].contains("X 21.500 to 25.000 (3.500 mm along X)"));
    });
    let both = [Some((5., 10.)), Some((22., 25.))];
    generate(&values("box", "whole_stock"), both, None, |path| {
        let path = path.unwrap();
        assert_clear(&path, both);
        let (lo, hi) = box_finished_span(&path);
        assert!((lo - 10.5).abs() < 1e-9 && (hi - 21.5).abs() < 1e-9);
        assert_eq!(held_notes(&path).len(), 2);
    });
    // The part (X 10 to 20) clears the far grip by more than the tool's
    // reach: nothing is left and nothing is said.
    generate(&values("box", "part"), far, None, |path| {
        let path = path.unwrap();
        assert!(held_notes(&path).is_empty());
    });
}

#[test]
fn req_op_24_req_nfr_07_box_with_nothing_left_beside_the_grip_refuses() {
    // The grip runs to X 22, so the tool's edge could start at X 22.5:
    // beyond the part extent's end at X 20. Nothing is cut.
    let held = [Some((5., 22.)), None];
    generate(&values("box", "part"), held, None, |path| {
        let error = path.unwrap_err().to_string();
        assert!(error.contains("nothing is left to face"), "{error}");
        assert!(error.contains("held start (X 5.000 to 22.000)"), "{error}");
    });
}

#[test]
fn req_op_24_req_set_09_a_host_keep_out_that_differs_from_the_grip_refuses() {
    // The host's keep-out says nothing of the grip: Facing cannot state a
    // limit it does not share with the host, and refuses (NFR-07).
    let held = [Some((5., 10.)), None];
    generate(
        &values("box", "whole_stock"),
        held,
        Some(Vec::new()),
        |path| {
            let error = path.unwrap_err().to_string();
            assert!(error.contains("held region"), "{error}");
        },
    );
}

#[test]
fn req_op_24_req_set_09_a_host_keep_out_with_an_extra_span_refuses() {
    // Issue #895 item 2: every span of the host's keep-out must be one the
    // grip gives, not only the other way round. The grip's own span with a
    // second one beside it is not the keep-out this set-up gives (NFR-07).
    let held = [Some((5., 10.)), None];
    let own = (5. - R - CLEAR, 10. + R + CLEAR);
    generate(
        &values("box", "whole_stock"),
        held,
        Some(vec![own, (own.0, own.1 + 2.)]),
        |path| {
            let error = path.unwrap_err().to_string();
            assert!(
                error.contains("held region") && error.contains(&format!("{:.3}", own.1 + 2.)),
                "{error}"
            );
        },
    );
    // The same spans in another order are the same keep-out.
    let held = [Some((5., 10.)), Some((25., 30.))];
    let spans: Vec<(f64, f64)> = held
        .iter()
        .flatten()
        .rev()
        .map(|&(lo, hi)| (lo - R - CLEAR, hi + R + CLEAR))
        .collect();
    generate(&values("box", "whole_stock"), held, Some(spans), |path| {
        let error = path.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(
            !error.contains("is not the one this set-up gives"),
            "{error}"
        );
    });
}

#[test]
fn req_op_24_req_set_09_restricted_flat_and_round_face_up_to_the_held_start() {
    let held = [Some((5., 10.)), None];
    // Flat passes run along X and end at the tool's centre limit, X 12: their
    // round ends are not counted, so the face is finished from X 12 and
    // X 10 to 12 (2 mm) of the part extent is left.
    generate(&values("flat", "part"), held, None, |path| {
        let path = path.unwrap_or_else(|e| panic!("flat: {e}"));
        assert_clear(&path, held);
        let notes = held_notes(&path);
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert!(notes[0].contains("finished from X 12.000"), "{}", notes[0]);
        assert!(
            notes[0].contains("X 10.000 to 12.000 (2.000 mm along X)"),
            "{}",
            notes[0]
        );
    });
    // Round turns a whole ring at the centre limit, X 12; its check counts
    // the tool's radius less 0.001 mm, so it is finished from X 10.501 and
    // X 10 to 10.501 (0.501 mm) is left.
    generate(&values("round", "part"), held, None, |path| {
        let path = path.unwrap_or_else(|e| panic!("round: {e}"));
        assert_clear(&path, held);
        let notes = held_notes(&path);
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert!(notes[0].contains("finished from X 10.501"), "{}", notes[0]);
        assert!(
            notes[0].contains("X 10.000 to 10.501 (0.501 mm along X)"),
            "{}",
            notes[0]
        );
    });
}

#[test]
fn req_op_24_req_nfr_07_one_note_names_every_face_that_stops_at_the_grip() {
    // All four Box sides stop at the same place beside the held start: one
    // note names them all, in cut order.
    let held = [Some((5., 10.)), None];
    let mut v = values("box", "part");
    for side in ["positive_y", "negative_z", "negative_y"] {
        v.insert(side.into(), ParameterValue::Flag(true));
    }
    generate(&v, held, None, |path| {
        let path = path.unwrap();
        let notes = held_notes(&path);
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert!(
            notes[0].contains(
                "Facing at A 0.000°, at A 90.000°, at A 180.000° and at A 270.000° stops short"
            ),
            "{}",
            notes[0]
        );
    });
}

#[test]
fn req_op_24_req_set_09_restricted_flat_faces_up_to_a_held_far_end() {
    // The far grip runs from X 15: the tool's centre stays below X 13, where
    // Flat's passes along X end, so the face is finished up to X 13.000 and
    // X 13 to 20 (7 mm) of the part extent is left.
    let held = [None, Some((15., 25.))];
    generate(&values("flat", "part"), held, None, |path| {
        let path = path.unwrap_or_else(|e| panic!("flat: {e}"));
        assert_clear(&path, held);
        let notes = held_notes(&path);
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert!(
            notes[0].contains("held far end (X 15.000 to 25.000)"),
            "{}",
            notes[0]
        );
        assert!(notes[0].contains("finished up to X 13.000"), "{}", notes[0]);
        assert!(
            notes[0].contains("X 13.000 to 20.000 (7.000 mm along X)"),
            "{}",
            notes[0]
        );
    });
}
