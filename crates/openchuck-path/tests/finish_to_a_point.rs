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
    let mesh = revolve(&profile, 180);
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

/// The warnings an operation gives on its Finish to a point setting.
fn point_warnings(p: &GeneratedPath) -> Vec<&openchuck_path::Notice> {
    p.notices
        .iter()
        .filter(|n| n.setting.as_deref() == Some(FINISH_TO_A_POINT))
        .collect()
}

/// Going down to the axis past the point cuts the part free of the stock
/// past it: with nothing held in the setup, both of decision record 0344's
/// warnings are given, on the Finish to a point setting, the part's own one
/// to acknowledge as a possible collision (decision 0695, Q2 (b) A; NFR-12).
/// Where Face free ends already faced the end down to the axis, its warnings
/// say so and none is added.
#[test]
fn req_nfr_12_finishing_to_a_point_warns_that_the_part_is_cut_free() {
    use openchuck_path::notice::{Acknowledgement, topic};
    let f = pointed();
    for s in [
        &HelicalFinishing as &dyn Strategy,
        &AxialFinishing,
        &IndexedFinishing,
    ] {
        let mut v = values(s, true);
        if s.schema().get(FACE_FREE_ENDS).is_some() {
            v.insert(FACE_FREE_ENDS.to_owned(), ParameterValue::Flag(false));
        }
        let p = path(s, &f, &v);
        let w = point_warnings(&p);
        assert_eq!(w.len(), 2, "{}: {w:?}", s.name());
        assert!(
            w.iter()
                .all(|n| n.topic.as_deref() == Some(topic::PART_CUT_FREE)
                    && n.text.contains("X 20.000")),
            "{}: {w:?}",
            s.name()
        );
        assert_eq!(
            w.iter()
                .filter(|n| n.acknowledgement == Some(Acknowledgement::PossibleCollision))
                .count(),
            1,
            "{}",
            s.name()
        );
    }
    for s in [&HelicalFinishing as &dyn Strategy, &AxialFinishing] {
        let p = path(s, &f, &values(s, true));
        assert!(point_warnings(&p).is_empty(), "{}", s.name());
        assert!(
            p.notices
                .iter()
                .any(|n| n.setting.as_deref() == Some(FACE_FREE_ENDS)
                    && n.topic.as_deref() == Some(topic::PART_CUT_FREE)),
            "{}",
            s.name()
        );
    }
}

/// A held region past the point grips the stock there: only the part's own
/// warning is given.
#[test]
fn req_set_09_req_nfr_12_a_held_region_past_the_point_grips_the_stock() {
    let f = pointed();
    let held = [(30.0, 40.0)];
    let mut input = f.input();
    input.keep_out = &held;
    let s = &HelicalFinishing;
    let mut v = values(s, true);
    v.insert(FACE_FREE_ENDS.to_owned(), ParameterValue::Flag(false));
    let p = run(|ctx| generate(s, &input, &v, ctx)).unwrap();
    let w = point_warnings(&p);
    assert_eq!(w.len(), 1, "{w:?}");
    assert!(w[0].text.contains("holds the part"), "{w:?}");
}

/// A point inside a held region's band is the held region's, not a free
/// end: nothing is carried on past it (SET-09).
#[test]
fn req_set_09_a_point_inside_a_held_band_is_not_finished_to() {
    let f = pointed();
    let held = [(19.0, 40.0)];
    let mut input = f.input();
    input.keep_out = &held;
    for s in [
        &HelicalFinishing as &dyn Strategy,
        &AxialFinishing,
        &IndexedFinishing,
    ] {
        let p = run(|ctx| generate(s, &input, &values(s, true), ctx)).unwrap();
        assert!(point_warnings(&p).is_empty(), "{}", s.name());
        assert!(
            centres(&p).iter().all(|&(_, r)| r > 0.05),
            "{}: a move reaches the axis",
            s.name()
        );
    }
}

/// An end narrower than the tool but not on the axis (a cone cut off at
/// radius 1 at X 20) is finished to the axis too: whole turns take the
/// ball's centre down to it beside the end, without cutting into the end
/// (decision 0695, Q2 (b) A).
#[test]
fn req_op_04_req_op_12_an_end_off_the_axis_narrower_than_the_tool_goes_to_the_axis() {
    let mesh = revolve(&[(0.0, 6.0), (14.0, 6.0), (POINT, 1.0)], 360);
    let mut f = Fixture::new(MeshSurface::new(&mesh).unwrap(), 8.0, &ball(D));
    f.stock = AnalyticStock::Cylinder {
        radius: 8.0,
        x_start: -1.0,
        x_end: 40.0,
    };
    for s in [
        &HelicalFinishing as &dyn Strategy,
        &AxialFinishing,
        &IndexedFinishing,
    ] {
        let p = path(s, &f, &values(s, true));
        let c = centres(&p);
        assert!(
            c.iter()
                .any(|&(x, r)| (x - (POINT + R)).abs() < 0.05 && r < 0.02),
            "{}: never on the axis past the end",
            s.name()
        );
        for (x, r) in c {
            // The end face at X 20, radius 1: the ball keeps R from its
            // disc.
            if r <= 1.0 {
                assert!(x - POINT > R - 0.02, "{}: X {x:.3} r {r:.3}", s.name());
            }
        }
        // With a Stepdown of 0.25 mm the tool goes down beside the end in
        // whole turns 0.25 mm apart: about 1 mm from where the drop leaves
        // it to the axis takes several.
        let mut v = values(s, true);
        v.insert("stepdown".to_owned(), ParameterValue::Number(0.25));
        let mut levels: Vec<i64> = centres(&path(s, &f, &v))
            .into_iter()
            .filter(|&(x, r)| (x - (POINT + R)).abs() < 0.02 && r < 1.0)
            .map(|(_, r)| (r * 100.0).round() as i64)
            .collect();
        levels.sort_unstable();
        levels.dedup();
        assert!(levels.len() >= 4, "{}: {levels:?}", s.name());
    }
}

/// Adaptive spacing measures the material left between the passes past
/// the point as it does over the part, and adds passes there where the
/// curve needs them, with the same Allowed excess and Smallest spacing
/// (decision 0695, Q2 (c) B): with it on, more cutting moves run past the
/// point than with it off, and the material there is confirmed (no
/// partial-path warning). Indexed finishing at 8 positions: at 4, the
/// tool's reach between positions is too short near the point to confirm
/// it, and the partial-path warning names the places, as anywhere else.
#[test]
fn req_op_04_adaptive_spacing_measures_and_adds_passes_past_the_point() {
    let f = pointed();
    for s in [
        &HelicalFinishing as &dyn Strategy,
        &AxialFinishing,
        &IndexedFinishing,
    ] {
        let mut off = values(s, true);
        if s.schema().get("count").is_some() {
            off.insert("count".to_owned(), ParameterValue::Integer(8));
        }
        let mut on = off.clone();
        on.insert("adaptive_spacing".to_owned(), ParameterValue::Flag(true));
        let past = |p: &GeneratedPath| centres(p).iter().filter(|&&(x, _)| x > POINT).count();
        let (off, on) = (path(s, &f, &off), path(s, &f, &on));
        assert!(past(&on) > past(&off), "{}", s.name());
        assert!(
            !on.notices
                .iter()
                .any(|n| n.text.contains("could not confirm")),
            "{}",
            s.name()
        );
    }
}

/// The same project gives the same path whatever the thread count (GC-07,
/// NFR-06), with Finish to a point and adaptive spacing over the point.
#[test]
fn req_nfr_06_req_gc_07_finish_to_a_point_is_the_same_at_one_two_and_many_threads() {
    let f = pointed();
    for s in [
        &HelicalFinishing as &dyn Strategy,
        &AxialFinishing,
        &IndexedFinishing,
    ] {
        let mut v = values(s, true);
        v.insert("adaptive_spacing".to_owned(), ParameterValue::Flag(true));
        if s.schema().get("count").is_some() {
            v.insert("count".to_owned(), ParameterValue::Integer(8));
        }
        let at = |threads: usize| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap()
                .install(|| path(s, &f, &v).group)
        };
        let one = at(1);
        for threads in [2, 8] {
            assert_eq!(one, at(threads), "{}: {threads} threads", s.name());
        }
    }
}

/// The same part turned end for end: the point at X 0, at the low end, the
/// stock carried on past it.
fn pointed_at_the_start() -> Fixture {
    let n = 60;
    let mut profile: Vec<(f64, f64)> = (0..n)
        .map(|i| {
            let t = std::f64::consts::FRAC_PI_2 * f64::from(n - i) / f64::from(n);
            (6.0 - 6.0 * t.sin(), (6.0 * t.cos()).max(1e-3))
        })
        .collect();
    profile.extend([(6.0, 6.0), (20.0, 6.0)]);
    let mesh = revolve(&profile, 180);
    let mut f = Fixture::new(MeshSurface::new(&mesh).unwrap(), 8.0, &ball(D));
    f.stock = AnalyticStock::Cylinder {
        radius: 8.0,
        x_start: -20.0,
        x_end: 21.0,
    };
    f
}

/// A point at the part's start is finished to as one at its end is: the
/// ball's centre reaches the axis at the point's X − R and no cutting move
/// runs further along X, with adaptive spacing off and on.
#[test]
fn req_op_04_a_point_at_the_start_of_the_part_is_finished_to() {
    let f = pointed_at_the_start();
    for s in [
        &HelicalFinishing as &dyn Strategy,
        &AxialFinishing,
        &IndexedFinishing,
    ] {
        for adaptive in [false, true] {
            let mut v = values(s, true);
            v.insert(
                "adaptive_spacing".to_owned(),
                ParameterValue::Flag(adaptive),
            );
            if s.schema().get("count").is_some() {
                v.insert("count".to_owned(), ParameterValue::Integer(8));
            }
            let c = centres(&path(s, &f, &v));
            assert!(
                c.iter().any(|&(x, r)| (x + R).abs() < 0.05 && r < 0.02),
                "{} adaptive {adaptive}: never on the axis at X {:.3}",
                s.name(),
                -R
            );
            assert!(c.iter().all(|&(x, _)| x >= -R - 0.02), "{}", s.name());
        }
    }
}

/// Helical finishing in rings finishes to the point in rings: rings no
/// more than the pitch apart up to the end's last drop, then the turn on
/// the axis; with adaptive spacing on, passes are added past the point and
/// the material there is confirmed.
#[test]
fn req_op_04_helical_finishing_in_rings_finishes_to_a_point() {
    use openchuck_path::helical::parameter::{PASS_SHAPE, RINGS};
    let f = pointed();
    let s = &HelicalFinishing;
    let mut v = values(s, true);
    v.insert(
        PASS_SHAPE.to_owned(),
        ParameterValue::Text(RINGS.to_owned()),
    );
    let off = path(s, &f, &v);
    let c = centres(&off);
    assert!(
        c.iter()
            .any(|&(x, r)| (x - (POINT + R)).abs() < 0.05 && r < 0.02)
    );
    assert!(c.iter().all(|&(x, _)| x <= POINT + R + 0.02));
    v.insert("adaptive_spacing".to_owned(), ParameterValue::Flag(true));
    let on = path(s, &f, &v);
    let past = |p: &GeneratedPath| centres(p).iter().filter(|&&(x, _)| x > POINT).count();
    assert!(past(&on) > past(&off));
    assert!(
        !on.notices
            .iter()
            .any(|n| n.text.contains("could not confirm"))
    );
}
