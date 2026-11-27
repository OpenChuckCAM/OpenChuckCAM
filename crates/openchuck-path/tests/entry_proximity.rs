// SPDX-License-Identifier: AGPL-3.0-or-later
//! Lead and Helix entries of an operation limited to a selection are
//! measured against the part's faces outside the selection that the entry
//! comes near, and refused where they would cut one (OP-11, OP-12, NFR-07,
//! NFR-12; issue #957; decision record 0701).
//!
//! The case: the public shouldered shaft of the test kit (faces 0 to 7, see
//! `openchuck_testkit::brep::shouldered_shaft`). Selected alone, face 6 (a
//! cylinder of radius 8 from X 26 to 40) has a fillet (face 5) and then a
//! shoulder (face 4, X 24) and a larger cylinder (face 3, radius 12) beside
//! its end at X 26. A helix entry a little wider than the default comes down
//! through faces 3 to 5; the passes themselves do not reach them. Selected
//! alone, face 1 (radius 8, X 0 to 12) ends at a shoulder (face 2) that the
//! passes themselves cut into.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::float_cmp
)]

mod support;
use support::{generate_in_region, machine, run, tags, tool};

use std::collections::BTreeMap;

use openchuck_path::params::names;
use openchuck_path::{
    AcceptAll, AnalyticStock, BrepSurface, GeneratedPath, HelicalFinishing, ParameterValue,
    PathError, RegionConstraint, RotaryMapping, StrategyInput,
};
use openchuck_types::brep::{BrepModel, FaceId};
use openchuck_types::machine::Clearance;
use openchuck_types::tool::{RevolvedProfile, ToolShape};
use openchuck_types::units::Millimetres;

const TOLERANCE: f64 = 0.01;

struct Case {
    model: BrepModel,
    profile: RevolvedProfile,
    stock: AnalyticStock,
    tags: openchuck_types::ir::PathTags,
}

impl Case {
    fn new() -> Self {
        Self {
            model: openchuck_testkit::brep::shouldered_shaft(4),
            profile: tool(ToolShape::FlatEnd, 3.175).profile().unwrap(),
            stock: AnalyticStock::Cylinder {
                radius: 13.0,
                x_start: -1.0,
                x_end: 41.0,
            },
            tags: tags(),
        }
    }

    fn faces(&self, ids: &[u32]) -> BrepSurface {
        BrepSurface::with_faces(
            &self.model,
            &ids.iter().map(|k| FaceId(*k)).collect::<Vec<_>>(),
        )
        .unwrap()
    }

    /// Helical finishing of `selected` with `entry` (and its size), the
    /// faces `outside` (if any) given as the part outside the selection.
    fn run(
        &self,
        selected: u32,
        outside: Option<&[u32]>,
        entry: (&str, Option<(&str, f64)>),
    ) -> Result<GeneratedPath, PathError> {
        let surface = self.faces(&[selected]);
        let rest = outside.map(|ids| self.faces(ids));
        let m = machine();
        let input = StrategyInput {
            surface: &surface,
            stock: &self.stock,
            collision: &AcceptAll,
            tool: &self.profile,
            rotary: RotaryMapping::of_profile(&m),
            clearance: Clearance {
                radius: Millimetres(18.0),
                safe_z: Millimetres(28.0),
            },
            max_feeds: m.max_feeds,
            tags: &self.tags,
            facing: None,
            keep_out: &[],
            shank: None,
        };
        let mut values: BTreeMap<String, ParameterValue> = [
            (names::STEPOVER, ParameterValue::Number(1.0)),
            (names::FEED, ParameterValue::Number(600.0)),
            (names::PLUNGE_FEED, ParameterValue::Number(150.0)),
            (names::TOLERANCE, ParameterValue::Number(TOLERANCE)),
            (names::ENTRY, ParameterValue::Text(entry.0.into())),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v))
        .collect();
        if let Some((name, size)) = entry.1 {
            values.insert(name.to_owned(), ParameterValue::Number(size));
        }
        let region = RegionConstraint {
            region: Some(&surface),
            outside: rest.as_ref().map(|s| s as _),
            ..RegionConstraint::default()
        };
        run(|ctx| generate_in_region(&HelicalFinishing, &input, &region, &values, ctx))
    }
}

/// Every face of the shaft but `selected`.
fn rest_of(selected: u32) -> Vec<u32> {
    (0..8).filter(|k| *k != selected).collect()
}

const WIDE_HELIX: (&str, Option<(&str, f64)>) = ("helix", Some((names::HELIX_DIAMETER, 1.5)));

#[test]
fn req_op_11_req_op_12_a_helix_that_cuts_a_face_outside_the_selection_is_refused() {
    // Decision 0701 (the owner's answers: per-entry proximity, refuse): the
    // helix comes down through the fillet, the shoulder and the larger
    // cylinder beside the selected face. Without the faces outside the
    // selection it is made; with them it is refused, naming a face, the
    // depth and what to do.
    let case = Case::new();
    assert!(case.run(6, None, WIDE_HELIX).is_ok(), "made without them");
    let e = case.run(6, Some(&rest_of(6)), WIDE_HELIX).unwrap_err();
    let text = e.to_string();
    assert!(text.contains("outside the operation's selection"), "{text}");
    assert!(text.contains("into face"), "{text}");
    assert!(text.contains("Plunge or Ramp"), "{text}");
    assert!(matches!(e, PathError::Refused { .. }), "{e:?}");
}

#[test]
fn req_op_11_req_op_12_faces_that_share_no_edge_with_the_selection_are_measured() {
    // Decision 0701 (proximity, not shared edges): with only faces that
    // share no edge with face 6 given (the shoulder and the larger cylinder,
    // past the fillet), the helix is still refused, naming one of them.
    let case = Case::new();
    let text = case
        .run(6, Some(&[0, 1, 2, 3, 4]), WIDE_HELIX)
        .unwrap_err()
        .to_string();
    assert!(
        text.contains("into face 3") || text.contains("into face 4"),
        "{text}"
    );
}

#[test]
fn req_op_11_the_default_helix_into_the_fillet_is_refused() {
    // The default Helix diameter comes down into the fillet beside the
    // selected face, which nothing measured before: refused, naming it.
    let case = Case::new();
    assert!(case.run(6, None, ("helix", None)).is_ok());
    let text = case
        .run(6, Some(&rest_of(6)), ("helix", None))
        .unwrap_err()
        .to_string();
    assert!(text.contains("into face 5"), "{text}");
}

#[test]
fn req_op_11_req_nfr_12_where_the_pass_itself_cuts_a_face_the_refusal_says_so() {
    // The owner's Q3 answer (the allowance is the tolerance and the sampling
    // slack only): the passes on face 1 cut into the shoulder beside it, so
    // a lead there cuts it too and is refused; the refusal says that the
    // pass itself cuts the face, and what to do.
    let case = Case::new();
    assert!(case.run(1, None, ("lead", None)).is_ok());
    let text = case
        .run(1, Some(&rest_of(1)), ("lead", None))
        .unwrap_err()
        .to_string();
    assert!(text.contains("the pass itself cuts"), "{text}");
    assert!(text.contains("outside the operation's selection"), "{text}");
    assert!(
        text.contains("Plunge or Ramp") && text.contains("change the selection"),
        "{text}"
    );
}

#[test]
fn req_op_11_an_entry_that_keeps_clear_of_them_is_made() {
    // The default lead on face 6 comes near no face outside the selection,
    // so it is made: the measure only refuses, and never moves an entry.
    let case = Case::new();
    assert!(case.run(6, Some(&rest_of(6)), ("lead", None)).is_ok());
}

#[test]
fn req_op_11_plunge_and_ramp_are_not_measured_against_them() {
    // Plunge goes straight down above its pass's first point and a ramp
    // follows its pass (decision records 0645, 0695): neither is measured
    // against the faces outside the selection, so on face 1, where a lead
    // is refused, both are made.
    let case = Case::new();
    for entry in ["plunge", "ramp"] {
        assert!(
            case.run(1, Some(&rest_of(1)), (entry, None)).is_ok(),
            "{entry}"
        );
    }
}

#[test]
fn req_nfr_06_req_gc_07_the_measure_is_the_same_at_1_2_and_n_threads() {
    let case = Case::new();
    let at = |threads: usize, entry: (&str, Option<(&str, f64)>)| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| {
                case.run(6, Some(&rest_of(6)), entry)
                    .map(|p| p.group)
                    .map_err(|e| e.to_string())
            })
    };
    for entry in [("lead", None), WIDE_HELIX] {
        let one = at(1, entry);
        assert_eq!(one, at(2, entry));
        assert_eq!(one, at(8, entry));
    }
}
