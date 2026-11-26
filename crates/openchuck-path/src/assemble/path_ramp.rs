// SPDX-License-Identifier: AGPL-3.0-or-later
//! The parts of a ramp that follows the pass's own toolpath that do not
//! depend on the pass's shape (proposed decision record 0543; issue #639):
//! how high the ramp is at each point of its path, and the back-and-forth
//! ramp over an open stretch of the pass (a line, or an arc that stops
//! before a whole turn).

use openchuck_types::units::Degrees;

use crate::rotary::AxisPosition;

/// The most legs one back-and-forth ramp may take (decision 0242 item 6,
/// kept by decision 0543).
pub(super) const MAX_LEGS: usize = 10_000;

/// The most moves one back-and-forth ramp may have, checked before they
/// are built: the legs times the pass's points each leg goes over. The same
/// bound as the other entries' segments; a long pass (swarf's runs follow
/// whole passes, decision 0645) with many legs would otherwise fill memory
/// before the leg limit refuses it (#781 review).
pub(super) const MAX_MOVES: usize = 100_000;

/// The heights of a ramp over a path whose points are `travel` (distance
/// along the ramp from its start, rising) above `base` (the pass's own
/// height there). The ramp starts at `top` and ends at the last base. It
/// never falls faster than `tan` per unit of travel, and never goes below
/// the base; where the base itself falls faster just before the end, so
/// that no ramp within the angle could arrive at the end, the ramp follows
/// the base's fall there (decision 0543 D4, the owner's answer A). Also says
/// whether that happened.
pub(super) fn heights(travel: &[f64], base: &[f64], top: f64, tan: f64) -> (Vec<f64>, bool) {
    let samples: Vec<(f64, f64)> = travel.iter().copied().zip(base.iter().copied()).collect();
    // From the start: no faster a fall than the angle.
    let mut forward = Vec::with_capacity(samples.len());
    let mut last: Option<(f64, f64)> = None;
    for &(t, b) in &samples {
        let h = match last {
            None => top.max(b),
            Some((u, h)) => b.max(h - tan * (t - u)),
        };
        forward.push(h);
        last = Some((t, h));
    }
    // From the end: able to arrive there.
    let mut backward = Vec::with_capacity(samples.len());
    let mut last: Option<(f64, f64)> = None;
    for &(t, b) in samples.iter().rev() {
        let h = match last {
            None => b,
            Some((u, h)) => b.max(h + tan * (u - t)),
        };
        backward.push(h);
        last = Some((t, h));
    }
    backward.reverse();
    let mut steep = false;
    let mut out = Vec::with_capacity(samples.len());
    let mut previous: Option<(f64, f64)> = None;
    for ((&(t, _), f), b) in samples.iter().zip(&forward).zip(&backward) {
        let h = f.min(*b);
        if let Some((u, p)) = previous
            && p - h > tan * (t - u) + 1e-9
        {
            steep = true;
        }
        previous = Some((t, h));
        out.push(h);
    }
    (out, steep)
}

/// Why a back-and-forth ramp cannot be placed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Refusal {
    /// The stretch is shorter than the minimum (its length, mm).
    TooShort { length: f64 },
    /// More legs than [`MAX_LEGS`].
    TooManyLegs { legs: usize },
    /// More moves than [`MAX_MOVES`].
    MoveLimit { moves: usize },
}

/// A back-and-forth ramp over `path` (the pass's own positions and angles
/// from its first point on, at most as far as the ramp may use), from `top`
/// down to the first point, at no more than `tan` per unit of travel. The
/// travel of a step is its length across the machine plus, where A turns,
/// the arc length at the step's mean distance from the rotary axis.
/// Legs run out along the path and back, an even number of them, so the
/// last ends at the first point; each is as long as the path, or shorter
/// when two legs are enough, but never shorter than `shortest_leg` where the
/// path is that long (the tool radius, decision 0543 point 4 as accepted:
/// the ramp is then shallower than the angle, never steeper), and never
/// longer than `longest_leg`: the depth per traverse at a leg's far end is
/// `2 · leg · tan`, so every ramp bounds its legs to keep that within the
/// tool radius, and a pass that starts as a spiral also within one turn of
/// a spiral ramp at its start radius, taking more legs instead (decision
/// 0695, Q3, the owner's tool-load requirement, and Q8;
/// `f64::INFINITY` for no bound).
/// Where the two bounds meet, the longest wins: the depth per traverse is
/// the owner's requirement, and a leg shorter than the tool radius is
/// listed for the host's check of a tool that cannot cut straight down.
/// Where the pass rises from its first point more steeply than the angle,
/// its floor holds an earlier visit high and a return drops more than
/// `2 · leg · tan`: the legs are then shortened until the drop the ramp cuts
/// between one visit to a place and the next is within `2 · longest_leg ·
/// tan`; where legs shorter than `minimum` would be needed (a step in the
/// pass), the ramp with the legs first fitted is kept, following the pass's
/// own steep drop (decision 0543 D4), and its depth says how far it drops.
/// Gives the moves, from above the first point at `top` to the first point;
/// the floor of each, the pass's own height on the tool line through it
/// (issue #687); the length of its legs and how many; and whether a steep
/// drop of the pass was followed (see [`heights`]); and the depth per
/// traverse it cuts.
pub(super) fn back_and_forth(
    path: &[(AxisPosition, Degrees)],
    top: f64,
    tan: f64,
    minimum: f64,
    shortest_leg: f64,
    longest_leg: f64,
) -> Result<Ramp, Refusal> {
    // Distance along the path at each point.
    let mut along = Vec::with_capacity(path.len());
    let mut total = 0.0;
    let mut previous: Option<(AxisPosition, Degrees)> = None;
    for &(p, a) in path {
        if let Some((q, b)) = previous {
            total += step(q, b, p, a);
        }
        along.push(total);
        previous = Some((p, a));
    }
    if total < minimum || path.len() < 2 {
        return Err(Refusal::TooShort { length: total });
    }
    let (Some(&(head, a0)), Some(low)) = (
        path.first(),
        path.iter().map(|(p, _)| p.height).reduce(f64::min),
    ) else {
        return Err(Refusal::TooShort { length: total });
    };
    let rise = (top - low).max(top - head.height).max(0.0);
    // Two legs over a length of rise / (2 tan) are enough when the path is
    // that long; otherwise legs over the whole path, an even number. Where
    // the pass rises near its first point the ramp needs more: two legs
    // more until it starts at `top`.
    let need = rise / tan;
    let mut cap = longest_leg.min(total);
    // The depth per traverse the legs are bounded to keep.
    let bound = 2.0 * longest_leg * tan;
    let bounded = bound.is_finite() && bound > 0.0;
    // The first ramp that fitted, kept while shorter legs are tried.
    let mut fitted: Option<Ramp> = None;
    let (mut leg, mut legs) = if need <= 2.0 * total {
        ((0.5 * need).max(shortest_leg.min(total)), 2.0)
    } else {
        (total, 2.0 * (need / (2.0 * total)).ceil())
    };
    // Whole legs: a count a rounding error above a whole number is that
    // number (a ramp that then starts a hair low takes two legs more below).
    let whole = |x: f64| (x - 1e-9).ceil().max(1.0);
    if leg > cap {
        leg = cap;
        legs = 2.0 * whole(need / (2.0 * leg));
    }
    loop {
        if !legs.is_finite() || legs > MAX_LEGS as f64 {
            if let Some(ramp) = fitted {
                return Ok(ramp);
            }
            return Err(Refusal::TooManyLegs {
                legs: if legs.is_finite() {
                    legs as usize
                } else {
                    usize::MAX
                },
            });
        }
        // The moves the legs would take, before any is built: each leg
        // goes over the pass's points up to its length, and the point at it.
        let per_leg = along.iter().take_while(|s| **s < leg - 1e-12).count() + 1;
        let moves = (legs as usize).saturating_mul(per_leg);
        if moves > MAX_MOVES {
            if let Some(ramp) = fitted {
                return Ok(ramp);
            }
            return Err(Refusal::MoveLimit { moves });
        }
        let ramp = legs_over(
            path,
            &along,
            head,
            a0,
            leg.max(f64::EPSILON),
            legs as usize,
            top,
            tan,
        );
        let at_top = ramp
            .moves
            .first()
            .is_some_and(|(p, _)| p.height >= top - 1e-9);
        if at_top && (!bounded || ramp.depth <= bound + 1e-9) {
            return Ok(ramp);
        }
        if at_top {
            // The pass rises from its first point more steeply than the
            // angle, and its floor held an earlier visit high, so a return
            // drops more than `2 · leg · tan` (CodeRabbit on #930). That drop
            // shrinks with the leg: shorter legs, more of them, down to the
            // shortest stretch a ramp goes over. Where none keeps it (a step
            // in the pass, which no leg shortens), the ramp first fitted
            // follows the pass's own steep drop, as decision 0543 D4 accepts,
            // and reports the drop it cuts.
            let shorter = leg * (bound / ramp.depth).min(0.99);
            let first = fitted.take().unwrap_or(ramp);
            if shorter < minimum || !shorter.is_finite() {
                return Ok(first);
            }
            fitted = Some(first);
            cap = shorter;
            leg = shorter;
            legs = 2.0 * whole(need / (2.0 * leg));
        } else if leg < cap {
            leg = cap;
            legs = legs.max(2.0 * whole(need / (2.0 * leg)));
        } else {
            legs += 2.0;
        }
    }
}

/// The ramp of [`back_and_forth`] with `legs` legs of length `leg`.
#[allow(clippy::too_many_arguments)]
fn legs_over(
    path: &[(AxisPosition, Degrees)],
    along: &[f64],
    head: AxisPosition,
    a0: Degrees,
    leg: f64,
    legs: usize,
    top: f64,
    tan: f64,
) -> Ramp {
    // The path's points up to `leg`, and the point at `leg`.
    let mut out_points: Vec<(f64, AxisPosition, Degrees)> = path
        .iter()
        .zip(along)
        .take_while(|(_, s)| **s < leg - 1e-12)
        .map(|(&(p, a), &s)| (s, p, a))
        .collect();
    out_points.push((leg, at(path, along, leg), angle_at(path, along, leg)));
    // The ramp's samples: out along the leg, back, out, ... ending back at
    // the first point.
    let mut samples: Vec<(f64, AxisPosition, Degrees)> = vec![(0.0, head, a0)];
    // Which of `out_points` each sample is at.
    let mut places: Vec<usize> = vec![0];
    let mut travelled = 0.0;
    for k in 0..legs {
        let outward = k % 2 == 0;
        let ordered: Vec<&(f64, AxisPosition, Degrees)> = if outward {
            out_points.iter().skip(1).collect()
        } else {
            out_points.iter().rev().skip(1).collect()
        };
        let mut from = if outward { 0.0 } else { leg };
        for (j, &&(s, p, a)) in ordered.iter().enumerate() {
            travelled += (s - from).abs();
            from = s;
            samples.push((travelled, p, a));
            places.push(if outward {
                j + 1
            } else {
                out_points.len() - 2 - j
            });
        }
        // A short outward list (one point) still turns at the end of the
        // leg.
        if outward && ordered.is_empty() {
            travelled += leg;
            samples.push((travelled, head, a0));
            places.push(0);
        }
    }
    let travel: Vec<f64> = samples.iter().map(|(t, _, _)| *t).collect();
    let base: Vec<f64> = samples.iter().map(|(_, p, _)| p.height).collect();
    let (h, steep) = heights(&travel, &base, top, tan);
    let mut moves: Vec<(AxisPosition, Degrees)> = samples
        .iter()
        .zip(h)
        .map(|(&(_, p, a), height)| (AxisPosition { height, ..p }, a))
        .collect();
    if let Some(last) = moves.last_mut() {
        *last = (head, a0);
    }
    // The depth per traverse it cuts: the deepest drop between one visit to
    // a place and the next, the start included. Between two samples both
    // visits are straight, so the drop there is between theirs.
    let mut visited: Vec<Option<f64>> = vec![None; out_points.len()];
    let mut depth = 0.0_f64;
    for (&place, (p, _)) in places.iter().zip(&moves) {
        if let Some(seen) = visited.get_mut(place) {
            if let Some(before) = *seen {
                depth = depth.max(before - p.height);
            }
            *seen = Some(p.height);
        }
    }
    Ramp {
        moves,
        floors: base,
        leg,
        legs,
        steep,
        depth,
    }
}

/// A back-and-forth ramp: its moves, from where the descent starts to the
/// pass's first point; the floor of each move, the height on the tool line
/// through it that the ramp stands on (the pass's own); the length of its
/// legs and how many there are; and whether it follows a steep drop of the
/// pass (see [`heights`]).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Ramp {
    pub moves: Vec<(AxisPosition, Degrees)>,
    pub floors: Vec<f64>,
    pub leg: f64,
    pub legs: usize,
    pub steep: bool,
    /// The deepest drop between one visit to a place and the next, the
    /// start included: the depth per traverse the ramp cuts, mm.
    pub depth: f64,
}

/// The travel of a step from `(q, b)` to `(p, a)`.
fn step(q: AxisPosition, b: Degrees, p: AxisPosition, a: Degrees) -> f64 {
    let radius = 0.5 * (p.across.hypot(p.height) + q.across.hypot(q.height));
    (p.along - q.along).hypot(p.across - q.across) + (a.0 - b.0).abs().to_radians() * radius
}

/// The position at distance `s` along `path` (cumulative `along`).
fn at(path: &[(AxisPosition, Degrees)], along: &[f64], s: f64) -> AxisPosition {
    let (i, f) = locate(along, s);
    match (path.get(i), path.get(i + 1)) {
        (Some(&(p, _)), Some(&(q, _))) => AxisPosition {
            along: p.along + f * (q.along - p.along),
            across: p.across + f * (q.across - p.across),
            height: p.height + f * (q.height - p.height),
        },
        (Some(&(p, _)), None) => p,
        _ => AxisPosition {
            along: 0.0,
            across: 0.0,
            height: 0.0,
        },
    }
}

/// The angle at distance `s` along `path`.
fn angle_at(path: &[(AxisPosition, Degrees)], along: &[f64], s: f64) -> Degrees {
    let (i, f) = locate(along, s);
    match (path.get(i), path.get(i + 1)) {
        (Some(&(_, a)), Some(&(_, b))) => Degrees(a.0 + f * (b.0 - a.0)),
        (Some(&(_, a)), None) => a,
        _ => Degrees(0.0),
    }
}

/// The segment `i` (from point `i` to `i + 1`) that holds distance `s`, and
/// the fraction along it.
fn locate(along: &[f64], s: f64) -> (usize, f64) {
    let i = along
        .iter()
        .rposition(|&t| t <= s)
        .unwrap_or(0)
        .min(along.len().saturating_sub(2));
    match (along.get(i), along.get(i + 1)) {
        (Some(&t0), Some(&t1)) if t1 - t0 > 1e-12 => (i, ((s - t0) / (t1 - t0)).clamp(0.0, 1.0)),
        _ => (i, 0.0),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::float_cmp)]
mod tests {
    use super::*;

    const TAN: f64 = 0.1;

    fn line(n: usize, step: f64, height: impl Fn(f64) -> f64) -> Vec<(AxisPosition, Degrees)> {
        (0..=n)
            .map(|i| {
                let x = i as f64 * step;
                (
                    AxisPosition {
                        along: x,
                        across: 0.0,
                        height: height(x),
                    },
                    Degrees(30.0),
                )
            })
            .collect()
    }

    /// Checks the ramp descends within the angle except where `steep`, never
    /// goes under the path, has an even number of legs and ends at the first
    /// point.
    fn check(moves: &[(AxisPosition, Degrees)], path: &[(AxisPosition, Degrees)], top: f64) {
        let first = moves.first().unwrap().0;
        assert!((first.height - top).abs() < 1e-9);
        assert_eq!(moves.last().unwrap().0, path[0].0);
        let mut turns = 0;
        for w in moves.windows(3) {
            let (d0, d1) = (w[1].0.along - w[0].0.along, w[2].0.along - w[1].0.along);
            if d0 * d1 < 0.0 {
                turns += 1;
            }
        }
        // An even number of legs is an odd number of turns.
        assert_eq!(turns % 2, 1, "{turns} turns");
        for w in moves.windows(2) {
            let run = (w[1].0.along - w[0].0.along).abs();
            let drop = w[0].0.height - w[1].0.height;
            assert!(drop <= TAN * run + 1e-9, "drop {drop} over {run}");
        }
    }

    #[test]
    fn req_op_11_two_legs_when_the_line_is_long_enough() {
        let path = line(100, 0.5, |_| 10.0);
        let Ramp { moves, steep, .. } =
            back_and_forth(&path, 12.0, TAN, 3.0, 0.0, f64::INFINITY).unwrap();
        assert!(!steep);
        check(&moves, &path, 12.0);
        // 2 mm of rise at 0.1 is 20 mm of travel: two legs of 10 mm.
        let far = moves.iter().map(|(p, _)| p.along).fold(0.0, f64::max);
        assert!((far - 10.0).abs() < 1e-9, "{far}");
        // Every position is over the pass's own line.
        assert!(moves.iter().all(|(p, a)| p.across == 0.0 && a.0 == 30.0));
    }

    #[test]
    fn req_op_11_more_legs_over_a_short_line() {
        let path = line(4, 0.5, |_| 10.0);
        let Ramp { moves, .. } = back_and_forth(&path, 12.0, TAN, 1.0, 0.0, f64::INFINITY).unwrap();
        check(&moves, &path, 12.0);
        assert!(
            moves
                .iter()
                .all(|(p, _)| (0.0..=2.0 + 1e-9).contains(&p.along))
        );
    }

    #[test]
    fn req_op_11_req_nfr_07_a_line_shorter_than_the_minimum_is_refused() {
        let path = line(2, 0.5, |_| 10.0);
        assert_eq!(
            back_and_forth(&path, 12.0, TAN, 3.0, 0.0, f64::INFINITY),
            Err(Refusal::TooShort { length: 1.0 })
        );
    }

    #[test]
    fn req_op_11_req_nfr_07_too_many_legs_are_refused() {
        let path = line(1, 0.01, |_| 10.0);
        assert!(matches!(
            back_and_forth(&path, 1000.0, TAN, 0.001, 0.0, f64::INFINITY),
            Err(Refusal::TooManyLegs { .. })
        ));
    }

    #[test]
    fn req_op_11_the_ramp_stays_above_a_pass_that_rises_and_falls() {
        let path = line(100, 0.5, |x| 10.0 + (x * 0.5).sin());
        let Ramp { moves, .. } = back_and_forth(&path, 13.0, TAN, 3.0, 0.0, f64::INFINITY).unwrap();
        for (p, _) in &moves {
            let base = 10.0 + (p.along * 0.5).sin();
            assert!(p.height >= base - 1e-6, "{p:?} under {base}");
        }
    }

    #[test]
    fn req_op_11_a_steep_drop_just_before_the_first_point_is_followed() {
        // The base falls 1 mm in the last 0.1 mm of travel: no ramp at 0.1
        // can arrive there, so the ramp follows the fall (decision 0543 D4).
        let (h, steep) = heights(
            &[0.0, 10.0, 20.0, 20.1],
            &[10.0, 10.0, 11.0, 10.0],
            12.0,
            TAN,
        );
        assert!(steep);
        assert_eq!(h, vec![12.0, 11.0, 11.0, 10.0]);
        // On a line whose first point is 1 mm below the rest, the last leg
        // comes back over the higher pass and follows it down.
        let path = line(100, 0.5, |x| if x < 0.25 { 10.0 } else { 11.0 });
        let Ramp { moves, steep, .. } =
            back_and_forth(&path, 12.0, TAN, 3.0, 0.0, f64::INFINITY).unwrap();
        assert!(steep);
        assert_eq!(moves.last().unwrap().0, path[0].0);
    }

    #[test]
    fn req_op_11_req_op_15_a_partial_ring_reverses_a_only_inside_its_arc() {
        let path: Vec<(AxisPosition, Degrees)> = (0..=20)
            .map(|i| {
                (
                    AxisPosition {
                        along: 5.0,
                        across: 0.0,
                        height: 12.0,
                    },
                    Degrees(f64::from(i) * 5.0),
                )
            })
            .collect();
        let Ramp { moves, .. } = back_and_forth(&path, 14.0, TAN, 3.0, 0.0, f64::INFINITY).unwrap();
        assert!(
            moves
                .iter()
                .all(|(_, a)| (0.0..=100.0 + 1e-9).contains(&a.0))
        );
        assert!(moves.iter().all(|(p, _)| p.along == 5.0));
        assert_eq!(moves.last().unwrap().1, Degrees(0.0));
    }

    /// Issue #687: each move of a back-and-forth ramp stands on its floor,
    /// the pass's own height on the tool line through it, and never below
    /// it; the floor of the last move is the first point's.
    #[test]
    fn req_op_11_each_move_stands_on_the_pass_s_own_height_beneath_it() {
        // A pass that rises and falls under the ramp.
        let path = line(40, 0.5, |x| 10.0 + 0.3 * (x * 0.7).sin());
        let ramp = back_and_forth(&path, 13.0, TAN, 3.0, 0.0, f64::INFINITY).unwrap();
        assert_eq!(ramp.moves.len(), ramp.floors.len());
        for ((p, _), floor) in ramp.moves.iter().zip(&ramp.floors) {
            assert!(p.height >= floor - 1e-9, "{} below {floor}", p.height);
            // The floor is the pass's height at that point of its line.
            let x = p.along;
            assert!(
                (floor - (10.0 + 0.3 * (x * 0.7).sin())).abs() < 0.01,
                "{floor} at {x}"
            );
        }
        assert_eq!(ramp.floors.last(), Some(&path[0].0.height));
    }

    /// Decision 0543 point 4 as accepted (the owner's option (b), 10 Nov
    /// 2026, relayed; CodeRabbit on #761): where little descent is needed
    /// the two legs are still at least the tool radius when the path is that
    /// long, so the tool's centre never goes down within less than a radius;
    /// the ramp is shallower than the angle, never steeper, and stands on
    /// the pass where it reaches it.
    #[test]
    fn req_op_11_legs_are_at_least_the_tool_radius_where_the_path_allows() {
        let path = line(100, 0.5, |_| 10.0);
        // 0.2 mm of rise at 0.1 needs 2 mm of travel: 1 mm legs without the
        // radius, 3 mm with it.
        let short = back_and_forth(&path, 10.2, TAN, 0.1, 0.0, f64::INFINITY).unwrap();
        assert!((short.leg - 1.0).abs() < 1e-9, "{}", short.leg);
        let ramp = back_and_forth(&path, 10.2, TAN, 0.1, 3.0, f64::INFINITY).unwrap();
        assert!((ramp.leg - 3.0).abs() < 1e-9, "{}", ramp.leg);
        check(&ramp.moves, &path, 10.2);
        let far = ramp.moves.iter().map(|(p, _)| p.along).fold(0.0, f64::max);
        assert!((far - 3.0).abs() < 1e-9, "{far}");
        for ((p, _), floor) in ramp.moves.iter().zip(&ramp.floors) {
            assert!(p.height >= floor - 1e-9);
        }
        // A path shorter than the radius: legs over the whole of it.
        let path = line(4, 0.5, |_| 10.0);
        let ramp = back_and_forth(&path, 10.05, TAN, 0.1, 3.0, f64::INFINITY).unwrap();
        assert!((ramp.leg - 2.0).abs() < 1e-9, "{}", ramp.leg);
    }

    /// Decision 0695, Q3 (the owner's tool-load
    /// requirement): a leg is never longer than the bound it is given, so
    /// the depth per traverse at its far end, `2 · leg · tan`, stays within
    /// it; the ramp takes more legs instead, and keeps its angle and its
    /// whole length.
    #[test]
    fn req_op_11_legs_are_shortened_to_bound_the_depth_per_traverse() {
        let path = line(100, 0.5, |_| 10.0);
        // 5 mm of rise at 0.1 needs 50 mm of travel: two legs of 25 mm,
        // a depth per traverse of 5 mm; bounded to 5 mm legs, ten legs and
        // 1 mm.
        let free = back_and_forth(&path, 15.0, TAN, 0.1, 0.0, f64::INFINITY).unwrap();
        assert_eq!((free.legs, free.leg), (2, 25.0));
        let ramp = back_and_forth(&path, 15.0, TAN, 0.1, 0.0, 5.0).unwrap();
        assert_eq!(ramp.legs, 10);
        assert!((ramp.leg - 5.0).abs() < 1e-9, "{}", ramp.leg);
        check(&ramp.moves, &path, 15.0);
        let far = ramp.moves.iter().map(|(p, _)| p.along).fold(0.0, f64::max);
        assert!((far - 5.0).abs() < 1e-9, "{far}");
        let travel: f64 = ramp
            .moves
            .windows(2)
            .map(|w| (w[1].0.along - w[0].0.along).abs())
            .sum();
        assert!(travel >= 50.0 - 1e-9, "{travel}");
        // The deepest depth per traverse: between visits to the far end.
        let ends: Vec<f64> = ramp
            .moves
            .iter()
            .filter(|(p, _)| (p.along - 5.0).abs() < 1e-9)
            .map(|(p, _)| p.height)
            .collect();
        assert_eq!(ends.len(), 5);
        for w in ends.windows(2) {
            assert!(w[0] - w[1] <= 2.0 * 5.0 * TAN + 1e-9, "{w:?}");
        }
    }

    /// The deepest drop between one visit of `moves` to a position along a
    /// line at a fixed A and the next visit to it, the start included: the
    /// depth per traverse the ramp actually cuts (CodeRabbit on #930).
    fn measured_depth(moves: &[(AxisPosition, Degrees)]) -> f64 {
        let mut seen: Vec<(f64, f64)> = Vec::new();
        let mut deepest = 0.0_f64;
        for (p, _) in moves {
            match seen.iter_mut().find(|(x, _)| (x - p.along).abs() < 1e-9) {
                Some((_, h)) => {
                    deepest = deepest.max(*h - p.height);
                    *h = p.height;
                }
                None => seen.push((p.along, p.height)),
            }
        }
        deepest
    }

    /// Decision 0695, Q3 and Q8 (CodeRabbit on #930): where the pass rises
    /// from its first point more steeply than the ramp angle, its floor
    /// holds an earlier visit high, so the last return to a place drops more
    /// than `2 · leg · tan`. The legs are shortened until the drop the ramp
    /// actually cuts is within the bound, and the ramp reports that drop.
    #[test]
    fn req_op_11_a_pass_rising_from_its_first_point_keeps_the_depth_per_traverse() {
        // 1 mm rising from 10 to 11; a bound of 0.2 mm is 1 mm legs at 0.1.
        let path = line(10, 0.1, |x| 10.0 + x);
        let ramp = back_and_forth(&path, 11.0, TAN, 0.01, 0.0, 1.0).unwrap();
        let measured = measured_depth(&ramp.moves);
        assert!(
            measured <= 0.2 + 1e-9,
            "{measured} over {} mm legs",
            ramp.leg
        );
        assert!(
            (ramp.depth - measured).abs() < 1e-9,
            "{} {measured}",
            ramp.depth
        );
        assert!(ramp.leg < 1.0, "{}", ramp.leg);
        assert_eq!(ramp.moves.first().unwrap().0.height, 11.0);
        assert_eq!(ramp.moves.last().unwrap().0, path[0].0);
        for ((p, _), floor) in ramp.moves.iter().zip(&ramp.floors) {
            assert!(p.height >= floor - 1e-9);
        }
        // A flat pass keeps its legs, and its depth is `2 · leg · tan`.
        let flat = line(100, 0.5, |_| 10.0);
        let ramp = back_and_forth(&flat, 15.0, TAN, 0.1, 0.0, 5.0).unwrap();
        assert_eq!((ramp.legs, ramp.leg), (10, 5.0));
        assert!((ramp.depth - 1.0).abs() < 1e-9, "{}", ramp.depth);
        assert!((measured_depth(&ramp.moves) - ramp.depth).abs() < 1e-9);
    }

    /// Decision 0543 D4: a pass that rises from its first point so steeply
    /// (a step) that no leg the ramp may take keeps the drop within the
    /// bound keeps the ramp first fitted, which follows the pass's own drop
    /// and says how deep it is: never a depth it does not cut.
    #[test]
    fn req_op_11_a_rise_no_leg_can_bound_follows_the_pass_s_drop() {
        let mut path = line(100, 0.5, |_| 11.0);
        path[0].0.height = 10.0;
        path[1].0.along = 1e-6;
        let ramp = back_and_forth(&path, 11.5, TAN, 0.1, 0.0, 1.0).unwrap();
        assert!(ramp.steep);
        assert!((ramp.leg - 1.0).abs() < 1e-9, "{}", ramp.leg);
        let measured = measured_depth(&ramp.moves);
        assert!(measured > 0.9, "{measured}");
        assert!(
            (ramp.depth - measured).abs() < 1e-9,
            "{} {measured}",
            ramp.depth
        );
        assert_eq!(ramp.moves.last().unwrap().0, path[0].0);
    }

    /// #781 review: a long pass with many legs is refused by its count of
    /// moves before any move is built, not by filling memory first.
    #[test]
    fn req_nfr_07_a_ramp_of_too_many_moves_is_refused_before_it_is_built() {
        // 30 mm in 30,001 points; 100 mm of rise at 0.1 needs 1,000 mm of
        // travel, so 34 legs over the whole line: over a million moves.
        let path = line(30_000, 0.001, |_| 10.0);
        match back_and_forth(&path, 110.0, TAN, 0.1, 0.0, f64::INFINITY) {
            Err(Refusal::MoveLimit { moves }) => assert!(moves > MAX_MOVES, "{moves}"),
            other => panic!("{other:?}"),
        }
    }
}
