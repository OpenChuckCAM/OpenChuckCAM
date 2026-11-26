// SPDX-License-Identifier: AGPL-3.0-or-later
//! Adaptive spacing on a stepped part beside a held region (OP-04, SET-09,
//! NFR-12; OI-2198, issue #826; decision 0695, Q6).
//!
//! Helical finishing with a flat end on the stepped cylinder of decision
//! 0680, its first step held: adaptive spacing measures only where the
//! tool may cut (not inside the held region's band), counts the face turns
//! of decision 0297 at the step walls, and so confirms the part without a
//! partial path.
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

mod support;

use std::collections::BTreeMap;

use openchuck_path::params::names;
use openchuck_path::topic;
use openchuck_path::{
    GeneratedPath, HelicalFinishing, MeshSurface, ParameterValue, StrategyInput, generate,
};
use openchuck_testkit::parts::{CylinderStep, RadialPart, SteppedCylinder};
use openchuck_types::tool::ToolShape;
use support::{Fixture, run};

/// The flat end's diameter, mm (the first-cut part's 1/8 inch end mill).
const DIAMETER: f64 = 3.175;
/// The held first step ends here, mm.
const HELD_TO: f64 = 20.0;

fn part() -> SteppedCylinder {
    SteppedCylinder {
        steps: vec![
            CylinderStep {
                radius: 10.0,
                length: HELD_TO,
            },
            CylinderStep {
                radius: 8.0,
                length: 12.0,
            },
            CylinderStep {
                radius: 6.0,
                length: 12.0,
            },
        ],
    }
}

fn fixture() -> Fixture {
    let mesh = part().mesh(0, 720);
    Fixture::new(
        MeshSurface::new(&mesh).unwrap(),
        11.0,
        &support::tool(ToolShape::FlatEnd, DIAMETER),
    )
}

/// The held first step's keep-out band: the tool's tip never enters it
/// (decision 0161), its body reaching one tool radius either side.
fn band() -> [(f64, f64); 1] {
    [(-50.0, HELD_TO + 0.5 * DIAMETER)]
}

fn values(adaptive: bool) -> BTreeMap<String, ParameterValue> {
    let mut v = BTreeMap::new();
    for (k, x) in [
        (names::STEPOVER, 1.0),
        (names::TOLERANCE, 0.002),
        (names::FEED, 600.0),
        (names::PLUNGE_FEED, 200.0),
        (names::STOCK_TO_LEAVE, 0.0),
    ] {
        v.insert(k.to_owned(), ParameterValue::Number(x));
    }
    v.insert(
        openchuck_path::adaptive::parameter::ADAPTIVE_SPACING.to_owned(),
        ParameterValue::Flag(adaptive),
    );
    v
}

fn generated(f: &Fixture, held: &[(f64, f64)], adaptive: bool) -> GeneratedPath {
    let input = StrategyInput {
        keep_out: held,
        ..f.input()
    };
    run(|ctx| generate(&HelicalFinishing, &input, &values(adaptive), ctx)).unwrap()
}

fn partial(path: &GeneratedPath) -> Vec<String> {
    path.notices
        .iter()
        .filter(|n| n.topic.as_deref() == Some(topic::PARTIAL_PATH))
        .map(|n| n.text.clone())
        .collect()
}

/// OI-2198: with the first step held, the flat end's helical finish with
/// adaptive spacing on is confirmed: no place is left unconfirmed, inside
/// the held region's band or at the step walls.
#[test]
fn req_op_04_req_set_09_adaptive_spacing_beside_a_held_step_confirms_a_flat_end() {
    let f = fixture();
    let path = generated(&f, &band(), true);
    assert_eq!(partial(&path), Vec::<String>::new());
}

/// The same part with no held region: the step walls alone do not leave
/// adaptive spacing unconfirmed (the face turns cut that material).
#[test]
fn req_op_04_adaptive_spacing_at_step_walls_confirms_a_flat_end() {
    let f = fixture();
    let path = generated(&f, &[], true);
    assert_eq!(partial(&path), Vec::<String>::new());
}
