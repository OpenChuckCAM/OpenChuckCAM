// SPDX-License-Identifier: AGPL-3.0-or-later
//! A ramp follows its own pass and is designed for the tool's load (OP-11,
//! OP-12, NFR-07, NFR-12; decision 0695, Q3 and Q4, the
//! owner's answers with the tool-load requirement; issue #786). A pass that
//! starts as a spiral is ramped back and forth over its own path, never
//! round the part; its legs are bounded so the depth per traverse is no
//! more than one turn of a spiral ramp at its start radius and the tool
//! radius, and a line's no more than the tool radius (Q8), and every
//! back-and-forth ramp's no more than the Largest depth per traverse (Q9),
//! the note naming the bound that held it; a whole ring whose one turn would
//! take more is ramped back and forth over itself (Q9 part 3, the owner's
//! answer A); and a pass is plunged only where no ramp fits, each such pass
//! named with its reason and its depth.
//! Synthetic fixtures only.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::indexing_slicing
)]

mod support;

use std::collections::BTreeMap;
use std::f64::consts::{PI, TAU};

use openchuck_path::params::names;
use openchuck_path::rotary::AxisPosition;
use openchuck_path::roughing::parameter as roughing;
use openchuck_path::{
    CutPoint, GeneratedPath, MeshSurface, ParameterValue, Pass, RotaryRoughing, Strategy,
};
use openchuck_testkit::parts::{Cone, Cylinder, RadialPart};
use openchuck_types::ir::{MoveKind, ToolPose};
use openchuck_types::tool::ToolShape;
use openchuck_types::units::Degrees;
use support::{Fixture, ball, generate, run, tool};

/// The Ramp angle of these tests, degrees.
const ANGLE: f64 = 3.0;

fn n(x: f64) -> ParameterValue {
    ParameterValue::Number(x)
}

fn flag(on: bool) -> ParameterValue {
    ParameterValue::Flag(on)
}

fn values() -> BTreeMap<String, ParameterValue> {
    let mut v = BTreeMap::new();
    v.insert(names::STEPOVER.into(), n(2.0));
    v.insert(names::FEED.into(), n(300.0));
    v.insert(names::PLUNGE_FEED.into(), n(100.0));
    v.insert(names::ENTRY.into(), ParameterValue::Text("ramp".into()));
    v.insert(names::RAMP_ANGLE.into(), n(ANGLE));
    v
}

/// A thin part, so the synthetic passes below are what is tested, in stock
/// 16 mm from the rotary axis: with the default approach distance of 1 mm
/// every ramp starts 17 mm from the axis.
fn fixture(tool: &openchuck_types::tool::Tool) -> Fixture {
    let part = Cylinder {
        radius: 0.5,
        length: 60.0,
    };
    Fixture::new(MeshSurface::new(&part.mesh(3, 36)).unwrap(), 16.0, tool)
}

/// One pass that starts as a spiral: the tip `radius` from the rotary axis,
/// from X `x0`, `pitch` mm along X per turn, for `turns` turns, 5 degrees a
/// point; `drop` lowers the first point (the pass then rises along it).
#[derive(Debug)]
struct Spiral {
    radius: f64,
    x0: f64,
    pitch: f64,
    turns: f64,
}

impl Spiral {
    fn along(&self, a: f64) -> f64 {
        self.x0 + self.pitch * a / 360.0
    }
}

impl Strategy for Spiral {
    fn name(&self) -> &str {
        "test_spiral"
    }
    fn label(&self) -> &str {
        "Spiral (test)"
    }
    fn description(&self) -> &str {
        "A strategy for tests."
    }
    fn schema(&self) -> openchuck_path::ParameterSchema {
        openchuck_path::ParameterSchema::with_common(Vec::new())
    }
    fn passes(
        &self,
        input: &openchuck_path::StrategyInput<'_>,
        _: &openchuck_path::Parameters,
        _: &openchuck_types::job::JobContext<'_>,
    ) -> openchuck_path::Result<Vec<Pass>> {
        let steps = (self.turns * 72.0).round() as i32;
        let points = (0..=steps)
            .map(|k| {
                let a = 5.0 * f64::from(k);
                let tip = input.rotary.to_model(
                    AxisPosition {
                        along: self.along(a),
                        across: 0.0,
                        height: self.radius,
                    },
                    Degrees(a),
                );
                CutPoint {
                    tip,
                    a: Degrees(a),
                    contact: tip,
                }
            })
            .collect();
        Ok(vec![Pass {
            helical_pass_start: false,
            low_link: false,
            points,
            stock: None,
            rapids: Vec::new(),
            plunge_fed: Vec::new(),
        }])
    }
}

/// One pass of the given tip positions `(X, height, A)` in the machine
/// frame, at Y 0.
#[derive(Debug)]
struct Points(Vec<(f64, f64, f64)>);

impl Strategy for Points {
    fn name(&self) -> &str {
        "test_points"
    }
    fn label(&self) -> &str {
        "Points (test)"
    }
    fn description(&self) -> &str {
        "A strategy for tests."
    }
    fn schema(&self) -> openchuck_path::ParameterSchema {
        openchuck_path::ParameterSchema::with_common(Vec::new())
    }
    fn passes(
        &self,
        input: &openchuck_path::StrategyInput<'_>,
        _: &openchuck_path::Parameters,
        _: &openchuck_types::job::JobContext<'_>,
    ) -> openchuck_path::Result<Vec<Pass>> {
        let points = self
            .0
            .iter()
            .map(|&(along, height, a)| {
                let tip = input.rotary.to_model(
                    AxisPosition {
                        along,
                        across: 0.0,
                        height,
                    },
                    Degrees(a),
                );
                CutPoint {
                    tip,
                    a: Degrees(a),
                    contact: tip,
                }
            })
            .collect();
        Ok(vec![Pass {
            helical_pass_start: false,
            low_link: false,
            points,
            stock: None,
            rapids: Vec::new(),
            plunge_fed: Vec::new(),
        }])
    }
}

fn make(
    f: &Fixture,
    s: &dyn Strategy,
    v: &BTreeMap<String, ParameterValue>,
) -> std::result::Result<GeneratedPath, String> {
    run(|ctx| generate(s, &f.input(), v, ctx)).map_err(|e| e.to_string())
}

/// The first entry of `path`: the pose before it (where the ramp starts)
/// and its `LeadIn` and `Plunge` moves, up to the first cutting move.
fn first_entry(path: &GeneratedPath) -> (ToolPose, Vec<ToolPose>) {
    let segs = &path.group.segments;
    let at = segs
        .iter()
        .position(|s| matches!(s.kind, MoveKind::LeadIn | MoveKind::Plunge))
        .unwrap();
    let before = if at == 0 {
        path.group.start
    } else {
        segs[at - 1].end
    };
    let moves = segs[at..]
        .iter()
        .take_while(|s| matches!(s.kind, MoveKind::LeadIn | MoveKind::Plunge))
        .map(|s| s.end)
        .collect();
    (before, moves)
}

/// What a ramp from `before` through `moves` is, measured on the moves
/// themselves: the steepest descent along the tip's real path (the arc at
/// the step's mean distance from the rotary axis where A turns, and the
/// travel across the machine, decision 0543 D4), as a tangent; the length
/// along that path; the rise; and the deepest depth per traverse, the drop
/// between one visit of the ramp to a place on the pass and its next, a
/// place being the pass's X and A (`place`).
struct Measured {
    steepest: f64,
    length: f64,
    rise: f64,
    depth: f64,
}

fn measure(
    f: &Fixture,
    before: ToolPose,
    moves: &[ToolPose],
    place: impl Fn(&AxisPosition, f64) -> (i64, i64),
) -> Measured {
    let (mut steepest, mut length, mut depth) = (0.0_f64, 0.0, 0.0_f64);
    let mut seen: BTreeMap<(i64, i64), f64> = BTreeMap::new();
    let start = f.rotary.to_axis(before.tip, before.a);
    seen.insert(place(&start, before.a.0), start.height);
    let mut p = before;
    for m in moves {
        let (from, at) = (f.rotary.to_axis(p.tip, p.a), f.rotary.to_axis(m.tip, m.a));
        let radius = 0.5 * (from.across.hypot(from.height) + at.across.hypot(at.height));
        let travel = (at.along - from.along).hypot(at.across - from.across)
            + radius * (m.a.0 - p.a.0).abs().to_radians();
        let drop = from.height - at.height;
        if travel > 1e-9 {
            steepest = steepest.max(drop / travel);
        } else {
            // A move that goes down without travelling is a plunge, never
            // part of a ramp (CodeRabbit on #930).
            assert!(drop <= 1e-9, "a descent of {drop} mm with no travel");
        }
        length += travel;
        let key = place(&at, m.a.0);
        if let Some(h) = seen.insert(key, at.height) {
            depth = depth.max(h - at.height);
        }
        p = *m;
    }
    let end = f.rotary.to_axis(p.tip, p.a);
    Measured {
        steepest,
        length,
        rise: start.height - end.height,
        depth,
    }
}

/// A place on a spiral, by its A to a micro-degree.
fn by_angle(_: &AxisPosition, a: f64) -> (i64, i64) {
    ((a * 1e6).round() as i64, 0)
}

#[test]
fn req_op_11_req_op_12_a_spiral_is_ramped_back_and_forth_over_its_own_path_never_round_the_part() {
    // The tip 4 mm from the axis, a 6 mm ball: 13 mm of descent from 17 mm
    // at 3 degrees needs 13 / tan 3 = 248.05 mm of travel. Today's spiral
    // ramp (decision 0543 D2, amended by decision 0695 Q3) turned A
    // 248.05 / (2 pi 4) = 9.87 turns round the part at the start X, each
    // turn 2 pi 4 tan 3 = 1.317 mm below the one before.
    let f = fixture(&ball(6.0));
    let spiral = Spiral {
        radius: 4.0,
        x0: 10.0,
        pitch: 2.0,
        turns: 2.0,
    };
    let path = make(&f, &spiral, &values()).unwrap();
    let (before, moves) = first_entry(&path);
    assert!(moves.len() > 2, "a ramp, not a plunge: {}", moves.len());
    let tan = ANGLE.to_radians().tan();
    // Every pose is a pose of the pass raised along the tool axis: on the
    // pass's own X for its A, A within the pass, never below it.
    for m in std::iter::once(&before).chain(&moves) {
        let at = f.rotary.to_axis(m.tip, m.a);
        assert!(
            (-1e-9..=720.0 + 1e-9).contains(&m.a.0),
            "A {} is outside the pass: the ramp went round the part",
            m.a.0
        );
        assert!((at.along - spiral.along(m.a.0)).abs() < 1e-6, "{at:?}");
        assert!(at.across.abs() < 1e-6 && at.height >= 4.0 - 1e-9, "{at:?}");
    }
    let measured = measure(&f, before, &moves, by_angle);
    assert!((measured.rise - 13.0).abs() < 1e-6, "{}", measured.rise);
    // The Ramp angle along the tip's real path, and used: the same angle
    // as the spiral ramp it replaces.
    assert!(measured.steepest <= tan + 1e-9, "{}", measured.steepest);
    assert!(measured.steepest >= 0.99 * tan, "{}", measured.steepest);
    // Never shortened: at least h / tan, the spiral ramp's length.
    assert!(measured.length >= 248.05 - 0.01, "{}", measured.length);
    // The deepest depth per traverse: no more than one turn of the spiral
    // ramp at the start radius (1.317 mm), and no more than the tool radius
    // (3 mm).
    assert!(
        measured.depth <= 2.0 * PI * 4.0 * tan + 1e-6,
        "{}",
        measured.depth
    );
    assert!(measured.depth > 1.0, "{}", measured.depth);
    // A reverses during the entry only, and the note says so.
    assert!(
        path.notices.iter().any(|n| n.text.contains("reversing A")),
        "{:?}",
        path.notices
    );
}

#[test]
fn req_op_11_the_depth_per_traverse_of_a_spiral_s_ramp_is_at_most_the_tool_radius() {
    // The tip 12 mm from the axis, a 2 mm flat end (radius 1 mm): one turn
    // of a spiral ramp descends 2 pi 12 tan 3 = 3.95 mm, more than the tool
    // radius, so the legs are bounded by the radius: 1 / (2 tan 3) = 9.54
    // mm, and the 5 mm descent takes 95.4 mm in ten legs.
    let f = fixture(&tool(ToolShape::FlatEnd, 2.0));
    let spiral = Spiral {
        radius: 12.0,
        x0: 10.0,
        pitch: 2.0,
        turns: 1.0,
    };
    let path = make(&f, &spiral, &values()).unwrap();
    let (before, moves) = first_entry(&path);
    let tan = ANGLE.to_radians().tan();
    let measured = measure(&f, before, &moves, by_angle);
    assert!(measured.steepest <= tan + 1e-9, "{}", measured.steepest);
    assert!(measured.length >= 5.0 / tan - 0.01, "{}", measured.length);
    assert!(measured.depth <= 1.0 + 1e-6, "{}", measured.depth);
    assert!(measured.depth > 0.9, "{}", measured.depth);
    // The note gives the angle, the legs and the deepest depth per traverse.
    let note = path
        .notices
        .iter()
        .find(|n| n.text.contains("depth per traverse"))
        .unwrap_or_else(|| panic!("{:?}", path.notices));
    assert!(
        note.text.contains("3.00 degrees")
            && note.text.contains("10 legs")
            && note.text.contains("1.000 mm"),
        "{}",
        note.text
    );
    assert!(!note.is_warning());
}

#[test]
fn req_op_11_req_nfr_07_a_spiral_near_the_axis_is_steepened_then_plunged_only_where_no_ramp_fits() {
    // The tip 1 mm from the axis: 16 mm of descent at 3 degrees would
    // travel 16 / tan 3 / (2 pi) = 48.6 turns' length, more than the 10
    // allowed (decision 0543 D4). Fitted in 10 turns: atan(16 / 20 pi) =
    // 14.29 degrees.
    let f = fixture(&ball(1.0));
    let spiral = Spiral {
        radius: 1.0,
        x0: 10.0,
        pitch: 0.5,
        turns: 2.0,
    };
    let mut v = values();
    let err = make(&f, &spiral, &v).unwrap_err();
    assert!(
        err.contains("Ramp entry: pass 1")
            && err.contains("more than the 10 allowed")
            && err.contains("turn on Steeper ramp near the axis"),
        "{err}"
    );
    v.insert(names::RAMP_STEEPER_NEAR_AXIS.into(), flag(true));
    v.insert(names::RAMP_ANGLE_MAX.into(), n(15.0));
    let path = make(&f, &spiral, &v).unwrap();
    let (before, moves) = first_entry(&path);
    let measured = measure(&f, before, &moves, by_angle);
    let fitted = (16.0 / (20.0 * PI)).atan();
    assert!(
        measured.steepest <= fitted.tan() * (1.0 + 1e-5),
        "{} against {}",
        measured.steepest.atan().to_degrees(),
        fitted.to_degrees()
    );
    assert!(measured.length >= 16.0 / fitted.tan() - 0.01);
    // One turn at 1 mm at that angle: 2 pi tan = 1.6 mm.
    assert!(
        measured.depth <= TAU * fitted.tan() + 1e-6,
        "{}",
        measured.depth
    );
    assert!(
        path.notices
            .iter()
            .any(|n| n.text.contains("steeper, up to 14.29 degrees")),
        "{:?}",
        path.notices
    );
    // A maximum below that: refused naming it, never asking to turn on the
    // steeper ramp that is on.
    v.insert(names::RAMP_ANGLE_MAX.into(), n(10.0));
    let err = make(&f, &spiral, &v).unwrap_err();
    assert!(
        err.contains("even at the Maximum ramp angle of 10.00 degrees")
            && err.contains("Plunge where no ramp fits")
            && !err.contains("turn on Steeper ramp near the axis"),
        "{err}"
    );
    // No ramp fits: with Plunge where no ramp fits on, plunged, and said
    // with its reason and its depth, the whole descent below the stock's
    // 16 mm to the pass's 1 mm.
    v.insert(names::ALLOW_SINGLE_POINT_PLUNGE.into(), flag(true));
    let path = make(&f, &spiral, &v).unwrap();
    let (_, moves) = first_entry(&path);
    assert_eq!(moves.len(), 1, "a plunge");
    assert_eq!(path.straight_plunges, [0]);
    assert!(
        path.notices.iter().any(|n| n.text.contains("pass 1 turns")
            && n.text
                .contains("more than 10 turns even at the Maximum ramp angle")
            && n.text.contains("straight plunge")
            && n.text.contains("depth per traverse of 15.000 mm")),
        "{:?}",
        path.notices
    );
}

/// One whole ring at X 20 that is not round: the tip 0.6 mm from the axis
/// at its first point and most of the way round, 1.5 mm over a stretch
/// just after it. Between about 19.3 and 21.1 degrees a ramp round it falls
/// a little short of a whole number of turns and travels less than the
/// angle needs, so its plan takes a whole turn more: the turns do not fall
/// in proportion to the angle, and a plan that fits 10 turns needs 21.2.
fn uneven_ring() -> Points {
    let r = |a: f64| -> f64 {
        if a <= 10.0 {
            0.6 + 0.9 * a / 10.0
        } else if a <= 60.0 {
            1.5
        } else if a < 70.0 {
            1.5 - 0.9 * (a - 60.0) / 10.0
        } else {
            0.6
        }
    };
    Points(
        (0..=72)
            .map(|k| {
                let a = 5.0 * f64::from(k);
                (20.0, r(a), a)
            })
            .collect(),
    )
}

#[test]
fn req_op_11_req_nfr_07_the_steeper_ramp_reaches_the_maximum_before_it_refuses() {
    // Decision 0695, Q4 (ii): the search scaled the angle
    // by the turns, four times, assuming they fall in proportion to it, and
    // stopped short of the maximum, then asked to turn on the setting
    // already on (on the baluster, 4.09 degrees against 15). From a Ramp
    // angle of 19.5 degrees here it stopped near 21.2 without reaching it.
    let f = fixture(&ball(1.0));
    let ring = uneven_ring();
    let mut v = values();
    v.insert(names::RAMP_ANGLE.into(), n(19.5));
    v.insert(names::RAMP_STEEPER_NEAR_AXIS.into(), flag(true));
    v.insert(names::RAMP_ANGLE_MAX.into(), n(45.0));
    let path = make(&f, &ring, &v).unwrap();
    let (before, moves) = first_entry(&path);
    let note = path
        .notices
        .iter()
        .find(|n| n.text.contains("steeper, up to"))
        .unwrap_or_else(|| panic!("{:?}", path.notices));
    // The shallowest angle that fits, not the maximum.
    assert!(note.text.contains("up to 21."), "{}", note.text);
    // One turn round this ring at that angle descends far more than the
    // ball's 0.5 mm radius, so the ring is then held to the bound (Q9 part
    // 3, the owner's answer A): back and forth over itself at the angle the
    // search found. The ring itself rises 0.9 mm in its first 10 degrees,
    // a step no leg can soften, so there the ramp follows the pass's own
    // drop (decision 0543 D4, OI-2240) and the note says so; it never drops
    // more than that step.
    let measured = measure(&f, before, &moves, by_angle);
    assert!(measured.depth <= 0.9 + 1e-6, "{}", measured.depth);
    assert!(reversals(&moves) > 0);
    let load = load_note(&path);
    assert!(
        load.contains("back and forth in up to")
            && load.contains(
                "more than the tool radius (0.500 mm), as it follows its pass's own steep drop"
            ),
        "{load}"
    );
    // Where even the maximum needs more than 10 turns, the refusal says so,
    // and does not ask to turn on Steeper ramp near the axis, which is on.
    v.insert(names::RAMP_ANGLE_MAX.into(), n(20.0));
    let err = make(&f, &ring, &v).unwrap_err();
    assert!(
        err.contains("even at the Maximum ramp angle of 20.00 degrees")
            && !err.contains("turn on Steeper ramp near the axis"),
        "{err}"
    );
}

#[test]
fn req_op_11_req_nfr_07_a_stretch_shorter_than_the_shortest_ramp_is_plunged_when_allowed() {
    // Decision 0695, Q4 (i): a stretch shorter than ten
    // path tolerances (0.1 mm at the default) is a plunge in all but name
    // (0543 amendment 4), so Plunge where no ramp fits covers it; it was
    // refused even with the setting on.
    let f = fixture(&ball(6.0));
    let short = Points(vec![(20.0, 10.0, 0.0), (20.05, 10.0, 0.0)]);
    let mut v = values();
    let err = make(&f, &short, &v).unwrap_err();
    assert!(
        err.contains("pass 1 starts with a stretch of 0.050 mm")
            && err.contains("Turn on Plunge where no ramp fits"),
        "{err}"
    );
    v.insert(names::ALLOW_SINGLE_POINT_PLUNGE.into(), flag(true));
    let path = make(&f, &short, &v).unwrap();
    let (_, moves) = first_entry(&path);
    assert_eq!(moves.len(), 1, "a plunge");
    assert_eq!(path.straight_plunges, [0]);
    // Its reason and its depth: the stock's 16 mm down to 10 mm.
    assert!(
        path.notices.iter().any(|n| n
            .text
            .contains("pass 1 starts with a stretch shorter than the shortest ramp")
            && n.text.contains("depth per traverse of 6.000 mm")),
        "{:?}",
        path.notices
    );
    // A single point says its depth too.
    let point = Points(vec![(20.0, 12.0, 0.0)]);
    let path = make(&f, &point, &v).unwrap();
    assert!(
        path.notices
            .iter()
            .any(|n| n.text.contains("pass 1 is a single point")
                && n.text.contains("depth per traverse of 4.000 mm")),
        "{:?}",
        path.notices
    );
}

/// How many passes a note names, as `passes_named` writes them: "pass 3",
/// "passes 1, 2 and 3" or "12 passes, from pass 1 to pass 40 (...)".
fn count_named(text: &str) -> usize {
    let mut words = text.split(' ');
    match words.next() {
        Some("pass") => 1,
        Some("passes") => words
            .take_while(|w| {
                let w = w.trim_end_matches(',');
                w == "and" || w.parse::<usize>().is_ok()
            })
            .filter(|w| *w != "and")
            .count(),
        Some(first) => first.parse().unwrap_or(0),
        None => 0,
    }
}

/// Every pass `path` enters by a plunge into the stock, by number, and the
/// passes its notes name as plunged where no ramp fits.
fn plunged_and_named(f: &Fixture, path: &GeneratedPath) -> (usize, usize) {
    let segs = &path.group.segments;
    let mut plunges = 0;
    let mut i = 0;
    while i < segs.len() {
        if matches!(segs[i].kind, MoveKind::LeadIn | MoveKind::Plunge) {
            let mut k = i;
            while k < segs.len() && matches!(segs[k].kind, MoveKind::LeadIn | MoveKind::Plunge) {
                k += 1;
            }
            if k == i + 1 && segs[i].kind == MoveKind::Plunge {
                let from = if i == 0 {
                    path.group.start
                } else {
                    segs[i - 1].end
                };
                let (a, b) = (
                    f.rotary.to_axis(from.tip, from.a),
                    f.rotary.to_axis(segs[i].end.tip, segs[i].end.a),
                );
                // A plunge through air to the stock's edge is not into it.
                if a.height - b.height > 1.0 + 1e-6 {
                    plunges += 1;
                }
            }
            i = k;
            continue;
        }
        i += 1;
    }
    let named = path
        .notices
        .iter()
        .filter(|n| n.text.contains("entered by a straight plunge"))
        .map(|n| count_named(&n.text))
        .sum();
    (plunges, named)
}

#[test]
fn req_op_11_req_op_03_at_the_new_defaults_only_passes_with_no_room_to_ramp_are_plunged() {
    // Decision 0695, Q4: with Plunge where no ramp fits
    // and Steeper ramp near the axis on (a new operation's defaults), a
    // roughing spiral's layers are ramped, and the only plunges are the
    // passes the notes name, each with its reason and depth.
    let cone = Cone {
        start_radius: 8.0,
        end_radius: 12.0,
        length: 30.0,
    };
    let f = Fixture::new(
        MeshSurface::new(&cone.mesh(40, 180)).unwrap(),
        16.0,
        &tool(ToolShape::FlatEnd, 6.0),
    );
    let mut v = values();
    v.insert(names::STEPOVER.into(), n(3.0));
    v.insert(names::STOCK_TO_LEAVE.into(), n(0.5));
    v.insert(roughing::STEPDOWN.into(), n(2.0));
    v.insert("pass_shape".into(), ParameterValue::Text("helix".into()));
    v.insert(names::ALLOW_SINGLE_POINT_PLUNGE.into(), flag(true));
    v.insert(names::RAMP_STEEPER_NEAR_AXIS.into(), flag(true));
    let path = make(&f, &RotaryRoughing, &v).unwrap();
    let (plunges, named) = plunged_and_named(&f, &path);
    assert!(plunges >= 1, "the cone's single-point pass");
    assert_eq!(plunges, named, "{:?}", path.notices);
    assert!(
        path.group
            .segments
            .iter()
            .filter(|s| s.kind == MoveKind::LeadIn)
            .count()
            > 10
    );
    // Every ramp of the run follows its own pass: no entry turns A below
    // the pass's own range at its start (round the part).
    assert!(
        path.notices
            .iter()
            .any(|n| n.text.contains("depth per traverse is")),
        "{:?}",
        path.notices
    );
}

#[test]
fn req_gc_07_spiral_ramps_are_the_same_at_one_two_and_many_threads() {
    let f = fixture(&ball(6.0));
    let spiral = Spiral {
        radius: 4.0,
        x0: 10.0,
        pitch: 2.0,
        turns: 2.0,
    };
    let v = values();
    let at = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| make(&f, &spiral, &v).unwrap().group.segments)
    };
    let one = at(1);
    assert_eq!(one, at(2));
    assert_eq!(one, at(8));
}

/// A place on a line, by its X to a micro-millimetre.
fn by_x(at: &AxisPosition, _: f64) -> (i64, i64) {
    ((at.along * 1e6).round() as i64, 0)
}

#[test]
fn req_op_11_the_depth_per_traverse_of_a_line_s_ramp_is_at_most_the_tool_radius() {
    // Decision 0695, Q8 (the owner's answer A): the
    // tool-radius bound reaches every back-and-forth ramp, lines and partial
    // rings too. A straight pass 12 mm from the axis, 50 mm long at A 0, and
    // a 2 mm flat end (radius 1 mm): legs over the whole pass would step
    // 2 * 50 * tan 3 = 5.2 mm per traverse; bounded, each leg is at most
    // 1 / (2 tan 3) = 9.54 mm, so the step is at most the radius. The angle
    // and the length the descent needs are kept.
    let f = fixture(&tool(ToolShape::FlatEnd, 2.0));
    let line = Points(
        (0..=100)
            .map(|k| (10.0 + 0.5 * f64::from(k), 12.0, 0.0))
            .collect(),
    );
    let path = make(&f, &line, &values()).unwrap();
    let (before, moves) = first_entry(&path);
    let tan = ANGLE.to_radians().tan();
    let measured = measure(&f, before, &moves, by_x);
    assert!(measured.rise > 1.0, "{}", measured.rise);
    assert!(measured.steepest <= tan + 1e-9, "{}", measured.steepest);
    assert!(
        measured.length >= measured.rise / tan - 0.01,
        "{} for a rise of {}",
        measured.length,
        measured.rise
    );
    assert!(measured.depth <= 1.0 + 1e-6, "{}", measured.depth);
    assert!(measured.depth > 0.5, "{}", measured.depth);
    // Every ramp pose stays over the pass (X 10 to 60, A 0).
    for m in &moves {
        let at = f.rotary.to_axis(m.tip, m.a);
        assert!((10.0 - 1e-9..=60.0 + 1e-9).contains(&at.along), "{at:?}");
        assert!(m.a.0.abs() < 1e-9, "{:?}", m.a);
    }
}

/// A straight pass 12 mm from the axis, 50 mm long at A 0.
fn long_line() -> Points {
    Points(
        (0..=100)
            .map(|k| (10.0 + 0.5 * f64::from(k), 12.0, 0.0))
            .collect(),
    )
}

/// The note giving what the ramps ask of the tool.
fn load_note(path: &GeneratedPath) -> String {
    path.notices
        .iter()
        .find(|n| n.text.contains("depth per traverse is"))
        .map(|n| n.text.clone())
        .unwrap_or_else(|| panic!("{:?}", path.notices))
}

#[test]
fn req_op_11_the_largest_depth_per_traverse_setting_holds_a_line_and_a_spiral_ramp() {
    // Decision 0695, Q9 (the owner's answer B): Largest
    // depth per traverse, below the tool radius, holds every back-and-forth
    // ramp to it, taking more legs at the same angle. A 2 mm flat end
    // (radius 1 mm). The line: legs at most 0.4 / (2 tan 3) = 3.82 mm.
    let f = fixture(&tool(ToolShape::FlatEnd, 2.0));
    let tan = ANGLE.to_radians().tan();
    let mut v = values();
    v.insert(names::RAMP_DEPTH_PER_TRAVERSE.into(), n(0.4));
    let path = make(&f, &long_line(), &v).unwrap();
    let (before, moves) = first_entry(&path);
    let measured = measure(&f, before, &moves, by_x);
    assert!(measured.steepest <= tan + 1e-9, "{}", measured.steepest);
    assert!(measured.steepest >= 0.99 * tan, "{}", measured.steepest);
    assert!(
        measured.length >= measured.rise / tan - 0.01,
        "{} for {}",
        measured.length,
        measured.rise
    );
    assert!(measured.depth <= 0.4 + 1e-6, "{}", measured.depth);
    assert!(measured.depth > 0.3, "{}", measured.depth);
    let note = load_note(&path);
    assert!(
        note.contains("the deepest depth per traverse is 0.400 mm, at pass 1")
            && note.contains("the Largest depth per traverse setting"),
        "{note}"
    );
    // The spiral 12 mm from the axis: one turn descends 3.95 mm, the radius
    // is 1 mm, and the setting 0.5 mm holds it.
    let spiral = Spiral {
        radius: 12.0,
        x0: 10.0,
        pitch: 2.0,
        turns: 1.0,
    };
    v.insert(names::RAMP_DEPTH_PER_TRAVERSE.into(), n(0.5));
    let path = make(&f, &spiral, &v).unwrap();
    let (before, moves) = first_entry(&path);
    let measured = measure(&f, before, &moves, by_angle);
    assert!(measured.steepest <= tan + 1e-9, "{}", measured.steepest);
    assert!(measured.length >= 5.0 / tan - 0.01, "{}", measured.length);
    assert!(measured.depth <= 0.5 + 1e-6, "{}", measured.depth);
    assert!(measured.depth > 0.4, "{}", measured.depth);
    let note = load_note(&path);
    assert!(
        note.contains("the Largest depth per traverse setting"),
        "{note}"
    );
}

#[test]
fn req_op_11_the_note_names_the_bound_that_holds_a_back_and_forth_ramp() {
    // At the default the tool radius holds the line (1 mm) and one turn
    // round the part the spiral 4 mm from the axis with a 6 mm ball (one
    // turn 2 pi 4 tan 3 = 1.317 mm, below the 3 mm radius).
    let f = fixture(&tool(ToolShape::FlatEnd, 2.0));
    let note = load_note(&make(&f, &long_line(), &values()).unwrap());
    assert!(
        note.contains("the deepest depth per traverse is 1.000 mm, at pass 1")
            && note.contains("the tool radius"),
        "{note}"
    );
    let spiral = Spiral {
        radius: 4.0,
        x0: 10.0,
        pitch: 2.0,
        turns: 2.0,
    };
    let note = load_note(&make(&fixture(&ball(6.0)), &spiral, &values()).unwrap());
    assert!(
        note.contains("1.317 mm") && note.contains("one turn round the part"),
        "{note}"
    );
}

#[test]
fn req_op_11_a_largest_depth_per_traverse_above_the_tool_radius_is_held_to_the_radius() {
    // A value above the radius is held to it: the same program as the
    // default, and the note names the tool radius.
    let f = fixture(&tool(ToolShape::FlatEnd, 2.0));
    let default = make(&f, &long_line(), &values()).unwrap();
    let mut v = values();
    v.insert(names::RAMP_DEPTH_PER_TRAVERSE.into(), n(5.0));
    let path = make(&f, &long_line(), &v).unwrap();
    assert_eq!(path.group.segments, default.group.segments);
    let note = load_note(&path);
    assert!(
        note.contains("1.000 mm") && note.contains("the tool radius"),
        "{note}"
    );
}

#[test]
fn req_op_11_req_gc_07_at_the_default_the_largest_depth_per_traverse_changes_no_ramp() {
    // 0, the default, means the tool radius: an operation that sets it to
    // 0 and one saved without it (the schema's default) give the same
    // program, at one, two and many threads.
    let f = fixture(&tool(ToolShape::FlatEnd, 2.0));
    let spiral = Spiral {
        radius: 12.0,
        x0: 10.0,
        pitch: 2.0,
        turns: 1.0,
    };
    let mut zero = values();
    zero.insert(names::RAMP_DEPTH_PER_TRAVERSE.into(), n(0.0));
    let at = |threads: usize, s: &dyn Strategy, v: &BTreeMap<String, ParameterValue>| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| make(&f, s, v).unwrap().group.segments)
    };
    for s in [&long_line() as &dyn Strategy, &spiral] {
        let saved = at(1, s, &values());
        for threads in [1, 2, 8] {
            assert_eq!(saved, at(threads, s, &zero), "{threads} threads");
        }
    }
}

#[test]
fn req_gc_07_ramps_held_by_the_largest_depth_per_traverse_are_the_same_at_one_two_and_many_threads()
{
    let f = fixture(&tool(ToolShape::FlatEnd, 2.0));
    let mut v = values();
    v.insert(names::RAMP_DEPTH_PER_TRAVERSE.into(), n(0.4));
    let at = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| make(&f, &long_line(), &v).unwrap().group.segments)
    };
    let one = at(1);
    assert_eq!(one, at(2));
    assert_eq!(one, at(8));
}

/// One whole ring at X 20, the tip `radius` from the rotary axis, 5
/// degrees a point.
fn ring(radius: f64) -> Points {
    Points(
        (0..=72)
            .map(|k| (20.0, radius, 5.0 * f64::from(k)))
            .collect(),
    )
}

/// The turns of A that reverse in `poses`.
fn reversals(poses: &[ToolPose]) -> usize {
    poses
        .windows(3)
        .filter(|w| (w[1].a.0 - w[0].a.0) * (w[2].a.0 - w[1].a.0) < 0.0)
        .count()
}

/// The cutting moves of `path`'s first pass, from its first cut point.
fn cutting(path: &GeneratedPath) -> Vec<ToolPose> {
    path.group
        .segments
        .iter()
        .skip_while(|s| s.kind != MoveKind::Cutting)
        .take_while(|s| s.kind == MoveKind::Cutting)
        .map(|s| s.end)
        .collect()
}

#[test]
fn req_op_11_a_whole_ring_deeper_per_turn_than_the_bound_is_ramped_back_and_forth_over_itself() {
    // Decision 0695, Q9 part 3, the owner's answer A (V1): a whole ring
    // whose one turn would take more than the bound is ramped back and
    // forth over its own ring at the same Ramp angle, under the rule and
    // the bound of lines and spirals. The ring 12 mm from the axis with a
    // 2 mm flat end (radius 1 mm): one turn round it descends
    // 2 pi 12 tan 3 = 3.95 mm, and the bound is the tool radius, 1 mm. The
    // descent from 17 mm is 5 mm, so the ramp is at least 5 / tan 3 =
    // 95.4 mm long, more than the ring's 75.4 mm: it goes back and forth.
    let f = fixture(&tool(ToolShape::FlatEnd, 2.0));
    let path = make(&f, &ring(12.0), &values()).unwrap();
    let (before, moves) = first_entry(&path);
    let tan = ANGLE.to_radians().tan();
    let measured = measure(&f, before, &moves, by_angle);
    assert!((measured.rise - 5.0).abs() < 1e-6, "{}", measured.rise);
    assert!(measured.steepest <= tan + 1e-9, "{}", measured.steepest);
    assert!(measured.steepest >= 0.99 * tan, "{}", measured.steepest);
    assert!(
        measured.length >= 5.0 / tan - 0.01,
        "{} for a rise of 5 mm",
        measured.length
    );
    assert!(measured.depth <= 1.0 + 1e-6, "{}", measured.depth);
    assert!(measured.depth > 0.5, "{}", measured.depth);
    // Every pose over the ring itself (X 20, A 0 to 360), raised along the
    // tool axis.
    for m in std::iter::once(&before).chain(&moves) {
        let at = f.rotary.to_axis(m.tip, m.a);
        assert!((at.along - 20.0).abs() < 1e-9, "{at:?}");
        assert!(at.across.abs() < 1e-9, "{at:?}");
        assert!(at.height >= 12.0 - 1e-9, "{at:?}");
        assert!((-1e-9..=360.0 + 1e-9).contains(&m.a.0), "{:?}", m.a);
    }
    // A reverses during the entry, and only there: the ring itself is cut
    // one way round.
    assert!(reversals(&moves) > 0, "{moves:?}");
    let cut = cutting(&path);
    assert!(cut.len() > 10, "{}", cut.len());
    assert_eq!(reversals(&cut), 0);
}

#[test]
fn req_op_11_a_whole_ring_within_the_bound_still_ramps_round_itself() {
    // The ring 6 mm from the axis with a 6 mm ball (radius 3 mm): one turn
    // descends 2 pi 6 tan 3 = 1.976 mm, within the radius, so the ring
    // ramps round itself as before, A one way, never reversing.
    let f = fixture(&ball(6.0));
    let path = make(&f, &ring(6.0), &values()).unwrap();
    let (before, moves) = first_entry(&path);
    assert_eq!(reversals(&moves), 0);
    let turned = (moves.last().unwrap().a.0 - before.a.0).abs();
    // 11 mm of descent at 1.976 mm a turn: about 5.6 turns.
    assert!(turned > 5.0 * 360.0 && turned <= 3600.0, "{turned}");
    let note = load_note(&path);
    assert!(
        note.contains("turning up to")
            && note.contains("1.976 mm")
            && !note.contains("back and forth"),
        "{note}"
    );
}

#[test]
fn req_op_11_the_largest_depth_per_traverse_setting_holds_a_whole_ring() {
    // Decision 0695, Q9 part 3, the owner's answer A: the setting holds a
    // whole ring too. The ring above, within the tool radius at the
    // default, is deeper per turn than a setting of 0.5 mm, so it is
    // ramped back and forth over itself, each traverse at most 0.5 mm.
    let f = fixture(&ball(6.0));
    let tan = ANGLE.to_radians().tan();
    let mut v = values();
    v.insert(names::RAMP_DEPTH_PER_TRAVERSE.into(), n(0.5));
    let path = make(&f, &ring(6.0), &v).unwrap();
    let (before, moves) = first_entry(&path);
    let measured = measure(&f, before, &moves, by_angle);
    assert!((measured.rise - 11.0).abs() < 1e-6, "{}", measured.rise);
    assert!(measured.steepest <= tan + 1e-9, "{}", measured.steepest);
    assert!(measured.length >= 11.0 / tan - 0.01, "{}", measured.length);
    assert!(measured.depth <= 0.5 + 1e-6, "{}", measured.depth);
    assert!(measured.depth > 0.4, "{}", measured.depth);
    assert!(reversals(&moves) > 0);
    for m in std::iter::once(&before).chain(&moves) {
        let at = f.rotary.to_axis(m.tip, m.a);
        assert!((at.along - 20.0).abs() < 1e-9 && at.height >= 6.0 - 1e-9);
    }
}

#[test]
fn req_op_11_the_note_names_the_bound_that_holds_a_whole_ring_ramped_back_and_forth() {
    // The tool radius holds the ring 12 mm from the axis with the 2 mm
    // flat end; the setting holds the ring 6 mm from the axis with the
    // 6 mm ball.
    let note = load_note(
        &make(
            &fixture(&tool(ToolShape::FlatEnd, 2.0)),
            &ring(12.0),
            &values(),
        )
        .unwrap(),
    );
    assert!(
        note.contains("back and forth in up to")
            && note.contains("the deepest depth per traverse is 1.000 mm, at pass 1")
            && note.contains("within the tool radius (1.000 mm)")
            && !note.contains("turning up to"),
        "{note}"
    );
    let mut v = values();
    v.insert(names::RAMP_DEPTH_PER_TRAVERSE.into(), n(0.5));
    let note = load_note(&make(&fixture(&ball(6.0)), &ring(6.0), &v).unwrap());
    assert!(
        note.contains("the deepest depth per traverse is 0.500 mm, at pass 1")
            && note.contains("within the Largest depth per traverse setting (0.500 mm)"),
        "{note}"
    );
}

#[test]
fn req_gc_07_whole_rings_held_back_and_forth_are_the_same_at_one_two_and_many_threads() {
    let f = fixture(&tool(ToolShape::FlatEnd, 2.0));
    let held = ring(12.0);
    let v = values();
    let at = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| make(&f, &held, &v).unwrap().group.segments)
    };
    let one = at(1);
    assert_eq!(one, at(2));
    assert_eq!(one, at(8));
}
