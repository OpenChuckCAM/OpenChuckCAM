// SPDX-License-Identifier: AGPL-3.0-or-later
//! Finish to a point: where the part ends in a point within the stock, the
//! finish carries on past the end, the tool following the drop down until
//! its centre is on the rotary axis at the point's X + R (decision record
//! 0695, Q2 (a) A, (b) A and (c) B; issue #779; OP-04, OP-12, OP-21, SET-09,
//! NFR-12).
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

mod support;

use std::collections::BTreeMap;

use openchuck_path::finishing::parameter::{FACE_FREE_ENDS, FINISH_TO_A_POINT};
use openchuck_path::params::ParameterKind;
use openchuck_path::{
    AnalyticStock, AxialFinishing, GeneratedPath, HelicalFinishing, IndexedFinishing, MeshSurface,
    ParameterValue, RestMachining, RotaryRoughing, SimultaneousFinishing, Strategy,
};
use openchuck_testkit::parts::revolve;
use openchuck_types::ir::MoveKind;
use support::{Fixture, ball, generate, run};

/// The ball's diameter and radius.
const D: f64 = 3.175;
const R: f64 = D / 2.0;
/// The part: a cylinder of radius 6 from X 0 to 14, then a hemisphere of
/// radius 6 centred at X 14, so the part ends in a point on the axis at X 20.
const POINT: f64 = 20.0;

fn pointed() -> Fixture {
    let mut profile = vec![(0.0, 6.0), (14.0, 6.0)];
    let n = 60;
    for i in 1..=n {
        let t = std::f64::consts::FRAC_PI_2 * f64::from(i) / f64::from(n);
        let r = (6.0 * t.cos()).max(1e-3);
        profile.push((14.0 + 6.0 * t.sin(), r));
    }
    let mesh = revolve(&profile, 360);
    let mut f = Fixture::new(MeshSurface::new(&mesh).unwrap(), 8.0, &ball(D));
    // The stock carries on well past the point, as a blank held at the
    // other end does.
    f.stock = AnalyticStock::Cylinder {
        radius: 8.0,
        x_start: -1.0,
        x_end: 40.0,
    };
    f
}

fn values(strategy: &dyn Strategy, point: bool) -> BTreeMap<String, ParameterValue> {
    let mut v: BTreeMap<String, ParameterValue> = [
        ("stepover", 0.5),
        ("feed", 600.0),
        ("plunge_feed", 150.0),
        ("tolerance", 0.01),
        ("stepdown", 1.0),
    ]
    .into_iter()
    .map(|(k, x)| (k.to_owned(), ParameterValue::Number(x)))
    .collect();
    if strategy.schema().get(FINISH_TO_A_POINT).is_some() {
        v.insert(FINISH_TO_A_POINT.to_owned(), ParameterValue::Flag(point));
    }
    v
}

fn path(
    strategy: &dyn Strategy,
    f: &Fixture,
    v: &BTreeMap<String, ParameterValue>,
) -> GeneratedPath {
    run(|ctx| generate(strategy, &f.input(), v, ctx)).unwrap()
}

/// The cutting moves' ends as (X of the ball's centre, radius of the ball's
/// centre from the rotary axis).
fn centres(p: &GeneratedPath) -> Vec<(f64, f64)> {
    p.group
        .segments
        .iter()
        .filter(|s| s.kind == MoveKind::Cutting)
        .map(|s| {
            let t = s.end.tip;
            // The centre is R up the tool axis from the tip.
            let c = t + s.end.axis.get() * R;
            (c.x, c.y.hypot(c.z))
        })
        .collect()
}

fn flag_default(strategy: &dyn Strategy, name: &str) -> Option<bool> {
    match strategy.schema().get(name).map(|p| p.kind.clone()) {
        Some(ParameterKind::Flag { default }) => Some(default),
        _ => None,
    }
}

#[test]
fn req_op_04_finish_to_a_point_is_off_by_default_on_helical_axial_and_indexed_and_absent_elsewhere()
{
    // Off in the schema, so an operation saved without it keeps its program
    // (decision 0545 D3's pattern); the application turns it on for a new
    // operation.
    for s in [
        &HelicalFinishing as &dyn Strategy,
        &AxialFinishing,
        &IndexedFinishing,
    ] {
        assert_eq!(
            flag_default(s, FINISH_TO_A_POINT),
            Some(false),
            "{}",
            s.name()
        );
    }
    for s in [
        &SimultaneousFinishing as &dyn Strategy,
        &RotaryRoughing,
        &RestMachining,
    ] {
        assert_eq!(flag_default(s, FINISH_TO_A_POINT), None, "{}", s.name());
    }
}

/// The finish reaches the axis past the point: some cutting move has the
/// ball's centre on the axis (within the tolerance) at the point's X + R,
/// and none goes further along X than that.
fn assert_reaches_the_point(strategy: &dyn Strategy) {
    let f = pointed();
    let p = path(strategy, &f, &values(strategy, true));
    let c = centres(&p);
    let tip = POINT + R;
    assert!(
        c.iter().any(|&(x, r)| (x - tip).abs() < 0.05 && r < 0.02),
        "{}: no cutting move ends with the ball's centre on the axis at X {tip:.3}; \
         the nearest past X {POINT}: {:?}",
        strategy.name(),
        c.iter()
            .filter(|&&(x, _)| x > POINT)
            .min_by(|a, b| a.1.total_cmp(&b.1))
    );
    assert!(
        c.iter().all(|&(x, _)| x <= tip + 0.02),
        "{}: a cutting move runs past the point's X + R",
        strategy.name()
    );
}

#[test]
fn req_op_04_helical_finish_to_a_point_reaches_the_axis_at_the_points_x_plus_r() {
    assert_reaches_the_point(&HelicalFinishing);
}

#[test]
fn req_op_04_axial_finish_to_a_point_reaches_the_axis_at_the_points_x_plus_r() {
    assert_reaches_the_point(&AxialFinishing);
}

#[test]
fn req_op_04_indexed_finish_to_a_point_reaches_the_axis_at_the_points_x_plus_r() {
    assert_reaches_the_point(&IndexedFinishing);
}

/// The ball's centre is never nearer the part than R: the part is the
/// cylinder of radius 6 to X 14 and the hemisphere of radius 6 centred at
/// X 14, so the run past the point follows it without cutting into it.
#[test]
fn req_op_04_req_op_12_the_run_past_a_point_never_cuts_the_part() {
    let f = pointed();
    for s in [
        &HelicalFinishing as &dyn Strategy,
        &AxialFinishing,
        &IndexedFinishing,
    ] {
        let p = path(s, &f, &values(s, true));
        for (x, r) in centres(&p) {
            let gap = if x >= 14.0 {
                (x - 14.0).hypot(r) - 6.0
            } else if x >= 0.0 {
                r - 6.0
            } else {
                f64::INFINITY
            };
            // The mesh's facets stand inside the true sphere by up to its
            // sagitta, so the drop stands a hair nearer.
            assert!(
                gap > R - 0.02,
                "{}: X {x:.3} r {r:.3}: gap {gap:.4}",
                s.name()
            );
        }
    }
}

/// Off, the path is as it was: past the point helical and axial finishing
/// keep the ball's centre at least R from the axis, as the table in decision
/// 0695 shows, and indexed finishing, whose passes already run on past the
/// ends by the tool's radius (decision 0343), stops short of the axis.
#[test]
fn req_op_04_finish_to_a_point_off_leaves_the_point_as_before() {
    let f = pointed();
    for (s, least) in [
        (&HelicalFinishing as &dyn Strategy, R - 0.05),
        (&AxialFinishing, R - 0.05),
        (&IndexedFinishing, 0.05),
    ] {
        let mut v = values(s, false);
        if s.schema().get(FACE_FREE_ENDS).is_some() {
            v.insert(FACE_FREE_ENDS.to_owned(), ParameterValue::Flag(false));
        }
        let p = path(s, &f, &v);
        let lowest = centres(&p)
            .into_iter()
            .filter(|&(x, _)| x > POINT)
            .map(|(_, r)| r)
            .fold(f64::INFINITY, f64::min);
        assert!(lowest > least, "{}: {lowest}", s.name());
    }
}
