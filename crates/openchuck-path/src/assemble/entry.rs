// SPDX-License-Identifier: AGPL-3.0-or-later
//! Entry and exit moves (OP-11, OP-14; decision record 0026).
//!
//! A pass starts with an entry and, for the `lead` style, ends with an exit.
//! They are placed here, with the A-axis management, clearance, collision,
//! feed and timing rules of [`super::assemble`], so a strategy never writes
//! them; the operation's `entry` parameters choose the style.
//!
//! - **Plunge**: a rapid to the approach distance above the stock, then a
//!   straight plunge at the plunge feed (decision record 0018 item 9).
//! - **Lead**: down beside the cut, then a quarter-circle tangent to the cut
//!   at the cutting feed (`LeadIn`), and the mirror image after the last
//!   point (`LeadOut`).
//! - **Ramp**: a straight descent at the ramp angle along the line of the
//!   cut, ending at the first cut point (`LeadIn` at the plunge feed).
//! - **Helix**: a circle the tool centre follows downwards, ending at the
//!   first cut point and tangent to the cut (`LeadIn` at the plunge feed).
//!
//! A does not move during a ramp or helix, so the entry lies in the plane of
//! the pass's start angle and cannot add a wrap or reversal (OP-15). Every
//! entry is checked against the finished surface, with the stock to leave
//! (the tool may not come closer than the path tolerance allows) and, for the
//! descent to a ramp's start, against the stock model. An entry that does not
//! fit is refused with a message that names the style and the size it needs;
//! it never changes style on its own.

use std::collections::BTreeMap;
use std::f64::consts::{FRAC_PI_2, TAU};

use openchuck_types::ir::MoveKind;
use openchuck_types::tool::profile::RevolvedProfile;
use openchuck_types::units::Degrees;

use super::pass_shape::{self, Start};
use super::ring_ramp::{self, RingRamp};
use super::{Builder, EPS, PlungeReason, View};
use crate::cutter::{self, ToolLine};
use crate::error::{PathError, Result};
use crate::helical::CHORD_FACTOR;
use crate::parallel;
use crate::params::{CommonParameters, EntryParameters, EntryStyle};
use crate::rotary::AxisPosition;
use crate::strategy::{CutPoint, StrategyInput};
use crate::surface::SurfaceSource;
use openchuck_types::geometry::{Point3, UnitVec3};

// The lead on a pass that turns A (decision 0544); a child module, so it
// shares this module's checks.
#[path = "lead.rs"]
mod lead;

/// The shortest stretch a back-and-forth ramp uses, in path tolerances
/// (decision 0543 D3, amended for issue #687, accepted), and never more than
/// the tool radius.
const RAMP_MINIMUM_TOLERANCES: f64 = 10.0;

/// The most segments one entry may take.
const MAX_SEGMENTS: usize = 100_000;

/// A horizontal direction in the fixed frame: (along, across).
type Dir = (f64, f64);

/// What a back-and-forth ramp goes over.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Shape {
    /// A line at a fixed A.
    Line,
    /// A partial ring, or a short line followed on along the pass.
    Arc,
    /// A pass that starts as a spiral, its tip this far from the rotary
    /// axis at its first point.
    Spiral(f64),
}

/// The entry [`Entry::ring_plan`] chooses.
enum RingEntry {
    /// A ramp round the ring, or a descent on the axis.
    Ramp(RingRamp),
    /// No ramp fits in 10 turns even at the Maximum ramp angle: plunged,
    /// as Allow single-point plunge asks.
    Plunge,
}

pub(super) struct Entry {
    style: EntryStyle,
    /// A single-point pass under a ramp is plunged (decision 0543 D7).
    single_point_plunge: bool,
    /// A ring or spiral ramp past 10 turns uses a steeper angle (decision
    /// 0543 section 4, point 3).
    steeper_near_axis: bool,
    /// The tangent of the steepest angle it may use.
    tan_max: f64,
    /// Roughing rings round the axis no wider than the nub are plunged
    /// (decision 0543 section 4, point 3): the nub's radius, or `None`.
    nub_radius: Option<f64>,
    /// The tool's cutting radius.
    tool_radius: f64,
    ramp_tan: f64,
    helix_radius: f64,
    lead_radius: f64,
    leave: f64,
    tolerance: f64,
    chord: f64,
    /// The profile grown by the stock to leave; `None` for a plain plunge.
    grown: Option<RevolvedProfile>,
    /// A height above all the stock (the retract height), from which the
    /// stock under a ramp's path is probed.
    probe_height: f64,
    /// Samples of the faces next to the face set (swarf's
    /// [`RegionConstraint::neighbours`](crate::RegionConstraint::neighbours)),
    /// with each face's number, sampled as swarf samples them (decision
    /// record 0532): a lead or helix may cut them no deeper than the pass
    /// itself cuts them where the move meets it, plus the tolerance
    /// (decision record 0682; issue #572). Empty for every other strategy.
    beside: Vec<(Point3, u32)>,
    /// The tool's profile, which the faces next to the face set are
    /// measured against, without the stock to leave, as swarf measures them.
    cutter: RevolvedProfile,
    /// For each of those faces, by number, how much deeper the tool can cut
    /// it between its samples than at the nearest one: the clearance changes
    /// no faster than the distance, and a sample is at most `s / sqrt 2`
    /// from any point of the face, for the face's samples `s` apart, which
    /// on a large face is coarser than the spacing asked for (OI-2078 is the
    /// same limit for the pass; CodeRabbit on #866).
    beside_slack: BTreeMap<u32, f64>,
}

impl Entry {
    pub(super) fn new(
        input: &StrategyInput<'_>,
        common: &CommonParameters,
        e: &EntryParameters,
        beside: Option<&dyn SurfaceSource>,
    ) -> Result<Self> {
        let radius = input.tool.cutting_radius();
        let leave = common.stock_to_leave.0;
        let beside = match (beside, e.style) {
            (Some(faces), EntryStyle::Lead | EntryStyle::Helix) => beside_samples(faces, radius),
            _ => (Vec::new(), BTreeMap::new()),
        };
        let grown = match e.style {
            EntryStyle::Plunge => None,
            _ => Some(cutter::grow_profile(input.tool, leave)?),
        };
        Ok(Self {
            style: e.style,
            single_point_plunge: e.allow_single_point_plunge,
            steeper_near_axis: e.steeper_near_axis,
            tan_max: e.ramp_angle_max.0.max(e.ramp_angle.0).to_radians().tan(),
            nub_radius: (e.style == EntryStyle::Ramp && e.plunge_face_nubs).then(|| {
                let diameter = 2.0 * radius;
                0.5 * if e.face_nub_diameter.0 > 0.0 {
                    e.face_nub_diameter.0.min(diameter)
                } else {
                    diameter
                }
            }),
            tool_radius: radius,
            ramp_tan: e.ramp_angle.0.to_radians().tan(),
            helix_radius: e.helix_diameter * radius,
            lead_radius: if e.lead_radius.0 > 0.0 {
                e.lead_radius.0
            } else {
                radius
            },
            leave,
            tolerance: common.tolerance.get().0,
            chord: CHORD_FACTOR * common.tolerance.get().0,
            grown,
            probe_height: match common.retract {
                crate::params::Retract::Clearance => input.clearance.radius.0,
                crate::params::Retract::SafeZ => input.clearance.safe_z.0,
            },
            beside: beside.0,
            cutter: input.tool.clone(),
            beside_slack: beside.1,
        })
    }

    /// Moves from above the pass's first point to that point.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn enter(
        &self,
        b: &mut Builder<'_>,
        input: &StrategyInput<'_>,
        stock: View<'_>,
        common: &CommonParameters,
        pass: usize,
        points: &[CutPoint],
        rapids: &[usize],
    ) -> Result<()> {
        let Some(head) = points.first() else {
            return Ok(());
        };
        match self.ring_plan(b, input, stock, common, pass, points, rapids)? {
            Some(RingEntry::Ramp(plan)) => return self.ring_ramp(b, input, common, pass, &plan),
            Some(RingEntry::Plunge) => {
                return self.fallback_plunge(
                    b,
                    stock,
                    common,
                    pass,
                    head,
                    PlungeReason::TooManyTurns,
                );
            }
            None => {}
        }
        match self.style {
            EntryStyle::Plunge => plunge(b, stock, common, head),
            EntryStyle::Lead => match self.turning_lead_in(b, points, rapids) {
                Some((sign, behind)) => self.turning_lead_in_moves(
                    b, input, stock, common, pass, points, rapids, sign, behind,
                ),
                None if self.starts_on_axis(b, points, rapids) => {
                    b.straight_plunges.push(pass);
                    lead::note_axis_start(b, pass);
                    plunge(b, stock, common, head)
                }
                None => self.lead_in(b, input, stock, common, pass, head, points),
            },
            EntryStyle::Ramp => self.ramp(b, input, stock, common, pass, head, points, rapids),
            EntryStyle::Helix => self.helix(b, input, stock, common, pass, head, points),
        }
    }

    /// Moves from the pass's last point; only the `lead` style has one.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn exit(
        &self,
        b: &mut Builder<'_>,
        common: &CommonParameters,
        input: &StrategyInput<'_>,
        stock: View<'_>,
        pass: usize,
        points: &[CutPoint],
        rapids: &[usize],
    ) -> Result<()> {
        if self.style != EntryStyle::Lead {
            return Ok(());
        }
        if let Some(done) = self.turning_lead_out(b, input, stock, common, pass, points, rapids) {
            return done;
        }
        let Some(last) = points.last() else {
            return Ok(());
        };
        let name = "lead-out";
        let end = b.rotary.to_axis(last.tip, last.a);
        // A pass of one cut point was entered by a plunge and has nothing to
        // lead out along (decision 0204).
        let Some(d) = direction_at_end(b, points) else {
            return Ok(());
        };
        let r = self.lead_radius;
        let offset = last.contact - last.tip;
        let mut refusal = None;
        for side in [1.0, -1.0] {
            let n = (-d.1 * side, d.0 * side);
            let centre = (end.along + r * n.0, end.across + r * n.1);
            let at = |phi: f64| AxisPosition {
                along: centre.0 - r * n.0 * phi.cos() + r * d.0 * phi.sin(),
                across: centre.1 - r * n.1 * phi.cos() + r * d.1 * phi.sin(),
                height: end.height,
            };
            let m = self.arc_segments(r);
            let positions: Vec<AxisPosition> = (1..=m)
                .map(|j| at(FRAC_PI_2 * j as f64 / m as f64))
                .collect();
            if let Some(g) = self.gouge(b, input, last.a, &positions, end) {
                refusal.get_or_insert(g);
                continue;
            }
            b.contact = last.contact;
            for pos in &positions {
                let tip = b.rotary.to_model(*pos, last.a);
                let pose = b.pose(tip, last.a);
                b.push(
                    MoveKind::LeadOut,
                    pose,
                    Some(tip + offset),
                    Some(common.feed),
                )?;
            }
            return Ok(());
        }
        Err(self.refuse(
            crate::topic::LEAD_ENTRY,
            input,
            pass,
            name,
            &format!("an arc of radius {r:.3} mm"),
            refusal.unwrap_or_default(),
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn lead_in(
        &self,
        b: &mut Builder<'_>,
        input: &StrategyInput<'_>,
        stock: View<'_>,
        common: &CommonParameters,
        pass: usize,
        head: &CutPoint,
        points: &[CutPoint],
    ) -> Result<()> {
        let name = "lead-in";
        let goal = b.rotary.to_axis(head.tip, head.a);
        // A pass of one cut point (the part's tip just reaches this layer) has
        // no line to lead along: it is entered by a plunge (issue 199,
        // decision 0204).
        let Some(d) = direction_at_start(b, points) else {
            b.straight_plunges.push(pass);
            lead::note_single_point(b, pass);
            return plunge(b, stock, common, head);
        };
        let r = self.lead_radius;
        let offset = head.contact - head.tip;
        let mut refusal = None;
        for side in [1.0, -1.0] {
            let n = (-d.1 * side, d.0 * side);
            let centre = (goal.along + r * n.0, goal.across + r * n.1);
            let at = |phi: f64| AxisPosition {
                along: centre.0 - r * n.0 * phi.cos() - r * d.0 * phi.sin(),
                across: centre.1 - r * n.1 * phi.cos() - r * d.1 * phi.sin(),
                height: goal.height,
            };
            let m = self.arc_segments(r);
            let positions: Vec<AxisPosition> = (0..m)
                .map(|j| at(FRAC_PI_2 * (1.0 - j as f64 / m as f64)))
                .collect();
            if let Some(g) = self.gouge(b, input, head.a, &positions, goal) {
                refusal.get_or_insert(g);
                continue;
            }
            // Down beside the cut, then round into it.
            let Some(start) = positions.first().copied() else {
                continue;
            };
            // The move across at the link's height is checked against
            // the stock along its whole line first (issue #642).
            super::clear_traverse(
                b,
                stock,
                start,
                head.a,
                common.approach_distance.0,
                self.probe_height,
            )?;
            let here = b.at();
            let above = AxisPosition {
                height: here.height,
                ..start
            };
            let tip = b.rotary.to_model(above, head.a);
            let pose = b.pose(tip, head.a);
            b.push(MoveKind::Rapid, pose, None, None)?;
            let stop = stop_height(b, stock, common, start.height);
            // As for the lead on a pass that turns A: where the stock stands
            // above the arc's start, the plunge to it cuts on its way down
            // and is listed for the host's tool check (decision 0544).
            if stop > start.height + common.approach_distance.0 + EPS {
                b.straight_plunges.push(pass);
            }
            rapid_to(b, stop)?;
            let tip = b.rotary.to_model(start, head.a);
            let pose = b.pose(tip, head.a);
            b.push(MoveKind::Plunge, pose, None, Some(common.plunge_feed))?;
            b.contact = tip + offset;
            for pos in positions.iter().skip(1) {
                let tip = b.rotary.to_model(*pos, head.a);
                let pose = b.pose(tip, head.a);
                b.push(
                    MoveKind::LeadIn,
                    pose,
                    Some(tip + offset),
                    Some(common.feed),
                )?;
            }
            let pose = b.pose(head.tip, head.a);
            return b.push(
                MoveKind::LeadIn,
                pose,
                Some(head.contact),
                Some(common.feed),
            );
        }
        Err(self.refuse(
            crate::topic::LEAD_ENTRY,
            input,
            pass,
            name,
            &format!("an arc of radius {r:.3} mm"),
            refusal.unwrap_or_default(),
        ))
    }

    /// A ramp that follows the pass's own toolpath (decision 0543): over a
    /// line, a partial ring or a spiral, back and forth (decision record
    /// PATH-QUALITY-1, Q3: no ramp goes round the part); a pass that starts
    /// as a whole ring, or on the rotary axis, is ramped round the ring or
    /// descended on the axis before it reaches here ([`Self::ring_plan`]),
    /// and comes here only when it starts above the stock, with nothing to
    /// descend into.
    #[allow(clippy::too_many_arguments)]
    fn ramp(
        &self,
        b: &mut Builder<'_>,
        input: &StrategyInput<'_>,
        stock: View<'_>,
        common: &CommonParameters,
        pass: usize,
        head: &CutPoint,
        points: &[CutPoint],
        rapids: &[usize],
    ) -> Result<()> {
        let shape = pass_shape::start(&b.rotary, points, rapids, self.tolerance);
        // The last point before the pass's first rapid.
        let last = (1..points.len())
            .find(|k| rapids.contains(k))
            .map_or(points.len().saturating_sub(1), |k| k - 1);
        match shape {
            Start::SinglePoint => {
                self.fallback_plunge(b, stock, common, pass, head, PlungeReason::SinglePoint)
            }
            // A whole ring, or a pass on the rotary axis, reaches here only
            // when there is nothing to descend into (`ring_plan` plans its
            // ramp or its descent on the axis otherwise).
            Start::FullRing { .. } => plunge(b, stock, common, head),
            Start::Spiral { .. } if self.on_axis(b, Some(head)) => plunge(b, stock, common, head),
            // A pass that turns A while it moves along X: back and forth
            // over its own path up to its first rapid, A reversing during
            // the entry only, never round the part (decision record
            // PATH-QUALITY-1, Q3, amending 0543 D2 and 0645 item 1), its legs
            // bounded by the depth per traverse of one turn of a spiral ramp
            // at its start radius.
            Start::Spiral { .. } => {
                let at = b.rotary.to_axis(head.tip, head.a);
                let radius = at.across.hypot(at.height);
                self.follow(
                    b,
                    input,
                    stock,
                    common,
                    pass,
                    points,
                    last,
                    Shape::Spiral(radius),
                )
            }
            Start::Line { end } => {
                // A line shorter than the minimum (the step before a ring,
                // say) is followed on along the pass, up to its first rapid,
                // so the ramp goes back and forth over the pass's own path
                // past it, A reversing during the entry only (decision 0543
                // D3).
                let path: Vec<(AxisPosition, Degrees)> = points
                    .iter()
                    .take(end + 1)
                    .map(|p| (b.rotary.to_axis(p.tip, p.a), p.a))
                    .collect();
                if travel_of(&path) < input.tool.cutting_radius() {
                    self.follow(
                        b,
                        input,
                        stock,
                        common,
                        pass,
                        points,
                        last.max(end),
                        Shape::Arc,
                    )
                } else {
                    self.follow(b, input, stock, common, pass, points, end, Shape::Line)
                }
            }
            Start::PartialRing { end, .. } => {
                self.follow(b, input, stock, common, pass, points, end, Shape::Arc)
            }
        }
    }

    /// A pass with no room for a ramp: a single point, a stretch shorter
    /// than the shortest ramp, or a ring or spiral that would take more than
    /// 10 turns even at the Maximum ramp angle. Refused unless Allow
    /// single-point plunge is on; then plunged, listed for the host's check
    /// of a tool that cannot cut straight down, and named in a note with its
    /// reason and its depth, the whole descent in one move (decision 0543
    /// D7; decision record PATH-QUALITY-1, Q4, the owner's answer).
    fn fallback_plunge(
        &self,
        b: &mut Builder<'_>,
        stock: View<'_>,
        common: &CommonParameters,
        pass: usize,
        head: &CutPoint,
        reason: PlungeReason,
    ) -> Result<()> {
        if !self.single_point_plunge {
            return Err(PathError::refused(
                crate::topic::RAMP_ENTRY,
                format!(
                    "pass {} is a single point, so there is no path for the ramp to follow. Turn \
                     on Allow single-point plunge, or set Entry to Plunge or Helix.",
                    pass + 1
                ),
            )
            .with_setting(crate::params::names::ALLOW_SINGLE_POINT_PLUNGE));
        }
        let depth = plunge_depth(b, stock, common, head)?;
        b.fallback_plunges.push((pass, reason, depth));
        b.straight_plunges.push(pass);
        Ok(())
    }

    /// The back-and-forth ramp over the pass's own path up to `end`, from
    /// the approach distance above the highest stock under the stretch it
    /// uses (decision 0543 D3, D4, D5). On a pass that starts as a spiral
    /// its legs are bounded so the depth per traverse is no more than one
    /// turn of a spiral ramp at the pass's start radius and the tool radius,
    /// and its length is held to 10 turns at that radius, steepened near
    /// the axis as for a ring (decision record PATH-QUALITY-1, Q3 and Q4).
    #[allow(clippy::too_many_arguments)]
    fn follow(
        &self,
        b: &mut Builder<'_>,
        input: &StrategyInput<'_>,
        stock: View<'_>,
        common: &CommonParameters,
        pass: usize,
        points: &[CutPoint],
        end: usize,
        shape: Shape,
    ) -> Result<()> {
        let path: Vec<(AxisPosition, Degrees)> = points
            .iter()
            .take(end + 1)
            .map(|p| (b.rotary.to_axis(p.tip, p.a), p.a))
            .collect();
        let Some(&(goal, a0)) = path.first() else {
            return Ok(());
        };
        let head = CutPoint {
            tip: b.rotary.to_model(goal, a0),
            a: a0,
            contact: b.rotary.to_model(goal, a0),
        };
        let low = path
            .iter()
            .map(|(p, _)| p.height)
            .fold(f64::INFINITY, f64::min);
        let total = travel_of(&path);
        let approach = common.approach_distance.0;
        // The stock over the first point: none means nothing to descend into.
        let here = stock_top_along(b, stock, &path, 0.0, self.probe_height);
        if here <= goal.height + EPS {
            return plunge(b, stock, common, &head);
        }
        // D5: the highest stock under the stretch the ramp uses, which
        // depends on how high it starts and on the angle: a few rounds
        // settle it.
        let top_at = |tan: f64| {
            let mut top = here + approach;
            for _ in 0..8 {
                let reach = ((top - low) / tan).min(total);
                let next = stock_top_along(b, stock, &path, reach, self.probe_height) + approach;
                if next <= top + EPS {
                    break;
                }
                top = next;
            }
            top
        };
        // The shortest stretch a ramp goes back and forth over: ten path
        // tolerances, at most the tool radius (decision 0543 D3 as amended
        // for issue #687, accepted). The angle holds on any length (each leg
        // of length L descends L tan(angle)), so the length sets only the
        // number of legs, which `MAX_LEGS` bounds; a shorter stretch is
        // within the path's own precision, a plunge in all but name, and is
        // entered as a single point is (decision record PATH-QUALITY-1, Q4
        // (i)). Over a stretch shorter than the tool radius the tool's centre
        // may go down into stock its edge has not cut, so the pass is listed
        // for the host's check of a tool that cannot cut straight down.
        let radius = input.tool.cutting_radius();
        let minimum = (RAMP_MINIMUM_TOLERANCES * self.tolerance).min(radius);
        let what = match shape {
            Shape::Line => "line",
            Shape::Arc | Shape::Spiral(_) => "stretch",
        };
        if total < minimum || path.len() < 2 {
            if self.single_point_plunge {
                return self.fallback_plunge(
                    b,
                    stock,
                    common,
                    pass,
                    &head,
                    PlungeReason::ShortStretch,
                );
            }
            return Err(PathError::refused(
                crate::topic::RAMP_ENTRY,
                format!(
                    "pass {} starts with a {what} of {total:.3} mm, shorter than the {minimum:.3} \
                     mm ({why}) a ramp needs to go back and forth over. Turn on Allow \
                     single-point plunge, or set Entry to Plunge or Helix.",
                    pass + 1,
                    why = if minimum < radius {
                        "ten times the tolerance"
                    } else {
                        "the tool radius"
                    }
                ),
            )
            .with_setting(crate::params::names::ALLOW_SINGLE_POINT_PLUNGE));
        }
        // The angle: the Ramp angle, or for a spiral whose ramp would travel
        // more than 10 turns at its start radius, steeper as for a ring
        // (decision 0543 section 4, point 3), the angle fitted to the rise
        // the stock under the ramp actually gives at that angle.
        let (tan, top, longest) = match shape {
            Shape::Spiral(r) => {
                let per_turn = std::f64::consts::TAU * r.max(EPS);
                let (fitted, tan, at_max) = self.steepen(|tan| {
                    let top = top_at(tan);
                    let turns = (top - low.min(goal.height)) / tan / per_turn;
                    if !turns.is_finite() || turns > ring_ramp::MAX_TURNS {
                        Err(ring_ramp::Refusal::TooManyTurns { radius: r, turns })
                    } else {
                        Ok(top)
                    }
                });
                match fitted {
                    Ok(top) => {
                        if tan > self.ramp_tan * (1.0 + 1e-12) {
                            b.steepened_ramps.push((pass, tan.atan().to_degrees()));
                        }
                        // One turn of a spiral ramp at the start radius
                        // descends `2 pi r tan`; a leg of length S takes
                        // `2 S tan` at its far end: S at most pi r, and at
                        // most what keeps that within the tool radius.
                        (tan, top, (0.5 * per_turn).min(0.5 * radius / tan))
                    }
                    Err(ring_ramp::Refusal::TooManyTurns { .. })
                        if at_max && self.single_point_plunge =>
                    {
                        return self.fallback_plunge(
                            b,
                            stock,
                            common,
                            pass,
                            &head,
                            PlungeReason::TooManyTurns,
                        );
                    }
                    Err(ring_ramp::Refusal::TooManyTurns { turns, .. }) => {
                        return Err(PathError::refused(
                            crate::topic::RAMP_ENTRY,
                            format!(
                                "pass {} starts by turning round the part with its tip {r:.3} \
                                 mm from the rotary axis, so {} its ramp back and forth would \
                                 travel {turns:.1} times the length of one turn there, more than \
                                 the {:.0} allowed. {}",
                                pass + 1,
                                angle_words(tan, at_max),
                                ring_ramp::MAX_TURNS,
                                fix_words(at_max)
                            ),
                        )
                        .with_setting(fix_setting(at_max)));
                    }
                    Err(ring_ramp::Refusal::NothingToDescend) => {
                        return plunge(b, stock, common, &head);
                    }
                }
            }
            Shape::Line | Shape::Arc => (self.ramp_tan, top_at(self.ramp_tan), f64::INFINITY),
        };
        let super::path_ramp::Ramp {
            moves,
            floors,
            leg,
            legs,
            steep,
        } = super::path_ramp::back_and_forth(&path, top, tan, minimum, radius, longest).map_err(
            |refusal| {
                let (text, setting) = match refusal {
                    super::path_ramp::Refusal::TooShort { length } => (
                        format!(
                            "pass {} starts with a {what} of {length:.3} mm, shorter than the \
                         {minimum:.3} mm a ramp needs to go back and forth over. Turn on Allow \
                         single-point plunge, or set Entry to Plunge or Helix.",
                            pass + 1,
                        ),
                        crate::params::names::ALLOW_SINGLE_POINT_PLUNGE,
                    ),
                    super::path_ramp::Refusal::TooManyLegs { legs } => (
                        format!(
                            "pass {} would need {legs} legs to ramp down its {what} of \
                     {total:.3} mm, more than the {} allowed. Raise Ramp angle, or set Entry to \
                     Plunge or Helix.",
                            pass + 1,
                            super::path_ramp::MAX_LEGS
                        ),
                        crate::params::names::RAMP_ANGLE,
                    ),
                    super::path_ramp::Refusal::MoveLimit { moves } => (
                        format!(
                            "pass {} would need {} to ramp down its {what} of \
                             {total:.3} mm, more than the {} allowed. Raise Ramp angle, or set \
                             Entry to Plunge or Helix.",
                            pass + 1,
                            crate::counted(moves, "move", "moves"),
                            super::path_ramp::MAX_MOVES
                        ),
                        crate::params::names::RAMP_ANGLE,
                    ),
                };
                PathError::refused(crate::topic::RAMP_ENTRY, text).with_setting(setting)
            },
        )?;
        self.emit(b, input, common, pass, &moves, &floors, MoveKind::LeadIn)?;
        // Legs are at least the tool radius where the stretch allows, so
        // only a stretch shorter than the radius, or a spiral's legs bounded
        // near the axis, leave them shorter.
        if leg < radius - EPS {
            b.straight_plunges.push(pass);
        }
        if steep {
            b.steep_ramps.push(pass);
        }
        // The load figures for the note (decision record PATH-QUALITY-1, Q3
        // point 4): the angle, the legs, and the depth per traverse, the
        // drop between one visit to a leg's far end and the next.
        let rise = moves.first().map_or(0.0, |(p, _)| p.height) - goal.height;
        b.ramp_loads.push(super::RampLoad {
            pass,
            angle: tan.atan().to_degrees(),
            legs: Some(legs),
            turns: None,
            depth: (2.0 * leg * tan).min(rise.max(0.0)),
        });
        if !matches!(shape, Shape::Line) {
            let reversals = moves
                .windows(3)
                .filter(|w| match w {
                    [p, q, r] => (q.1.0 - p.1.0) * (r.1.0 - q.1.0) < 0.0,
                    _ => false,
                })
                .count();
            if reversals > 0 {
                b.reversed_ramps.push((pass, reversals));
            }
        }
        Ok(())
    }

    /// Emits the moves of a ramp that follows the toolpath: up to its start
    /// if the tool is lower, to its start's angle and position at the
    /// tool's height, down to the start, then along it.
    ///
    /// Each move stands on its floor: the height, on the tool line through
    /// it, of the pass's own point beneath it (or of the floor the plan
    /// used). Dropped along that line from the spindle side, the tool first
    /// touches the finished surface at one height, so a move cuts into it
    /// by exactly its floor's depth less its height above the floor: never
    /// deeper than the pass itself, which its strategy has checked (decision
    /// 0543 section 3; issue #687). Each move is checked to be no lower than
    /// its floor. A drop of the tool itself is not used here: where the pass
    /// runs with the tool's side exactly against a face, as beside a part's
    /// end, which triangles a drop meets depends on rounding, and the pass's
    /// own points failed the check its ramp was held to (#687).
    #[allow(clippy::too_many_arguments)]
    fn emit(
        &self,
        b: &mut Builder<'_>,
        _input: &StrategyInput<'_>,
        common: &CommonParameters,
        pass: usize,
        moves: &[(AxisPosition, Degrees)],
        floors: &[f64],
        kind: MoveKind,
    ) -> Result<()> {
        let Some(&(start, a)) = moves.first() else {
            return Ok(());
        };
        let below = moves
            .iter()
            .zip(floors)
            .find(|((pos, _), floor)| pos.height < **floor - self.tolerance);
        if moves.len() != floors.len() || below.is_some() {
            let (depth, x) = below.map_or((0.0, f64::NAN), |((pos, a), floor)| {
                (floor - pos.height, b.rotary.to_model(*pos, *a).x)
            });
            return Err(PathError::refused(
                crate::topic::RAMP_ENTRY,
                format!(
                    "pass {} would go {depth:.3} mm below the pass itself, at X {x:.3} mm. This \
                     is a fault in the program: please report it, and set Entry to Plunge or \
                     Helix meanwhile.",
                    pass + 1
                ),
            )
            .with_setting(crate::params::names::ENTRY));
        }
        if start.height > b.at().height + EPS {
            b.retract(start.height)?;
        }
        let height = b.at().height;
        let over = b.rotary.to_model(AxisPosition { height, ..start }, a);
        let pose = b.pose(over, a);
        b.push(MoveKind::Rapid, pose, None, None)?;
        rapid_to(b, start.height)?;
        for (pos, a) in moves.iter().skip(1) {
            let tip = b.rotary.to_model(*pos, *a);
            let pose = b.pose(tip, *a);
            b.push(kind, pose, None, Some(common.plunge_feed))?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn helix(
        &self,
        b: &mut Builder<'_>,
        input: &StrategyInput<'_>,
        stock: View<'_>,
        common: &CommonParameters,
        pass: usize,
        head: &CutPoint,
        points: &[CutPoint],
    ) -> Result<()> {
        let name = "helix";
        let goal = b.rotary.to_axis(head.tip, head.a);
        let top = stop_height(b, stock, common, goal.height);
        let rise = top - goal.height;
        if rise <= EPS {
            return plunge(b, stock, common, head);
        }
        // Any tangent will do when the pass has no direction.
        let d = direction_at_start(b, points).unwrap_or((1.0, 0.0));
        let n = (-d.1, d.0);
        let rho = self.helix_radius;
        let per_turn = TAU * rho * self.ramp_tan;
        let turns = (rise / per_turn).ceil().max(1.0);
        let total = TAU * turns;
        let step = self.angle_step(rho);
        let count = (total / step).ceil().max(1.0);
        if count > MAX_SEGMENTS as f64 {
            return Err(PathError::refused(crate::topic::HELIX_ENTRY, format!(
                "pass {}: the helix entry needs {count:.0} segments for {turns:.0} turns; use a \
                 larger ramp angle or a larger helix diameter",
                pass + 1
            ))
            .with_setting(crate::params::names::RAMP_ANGLE));
        }
        let count = count as usize;
        let centre = (goal.along + rho * n.0, goal.across + rho * n.1);
        let at = |j: usize| {
            let f = j as f64 / count as f64;
            let phi = total * (1.0 - f);
            AxisPosition {
                along: centre.0 - rho * n.0 * phi.cos() - rho * d.0 * phi.sin(),
                across: centre.1 - rho * n.1 * phi.cos() - rho * d.1 * phi.sin(),
                height: goal.height + rise * (1.0 - f),
            }
        };
        let positions: Vec<AxisPosition> = (1..count).map(at).collect();
        if let Some(g) = self.gouge(b, input, head.a, &positions, goal) {
            return Err(self.refuse(
                crate::topic::HELIX_ENTRY,
                input,
                pass,
                name,
                &format!(
                    "a circle {:.3} mm across and {turns:.0} turns to descend {rise:.3} mm at \
                     {:.2} degrees",
                    2.0 * rho,
                    self.ramp_tan.atan().to_degrees()
                ),
                g,
            ));
        }
        rapid_to(b, top)?;
        for pos in &positions {
            let tip = b.rotary.to_model(*pos, head.a);
            let pose = b.pose(tip, head.a);
            b.push(MoveKind::LeadIn, pose, None, Some(common.plunge_feed))?;
        }
        let pose = b.pose(head.tip, head.a);
        b.push(MoveKind::LeadIn, pose, None, Some(common.plunge_feed))
    }

    /// The ramp round the ring the pass starts with, when the entry is a
    /// ramp and the pass starts as a closed ring (decision 0543 section 2),
    /// or the descent on the axis of a pass that starts there (section 4,
    /// point 2); a plunge where neither fits in 10 turns even at the Maximum
    /// ramp angle and Allow single-point plunge is on (decision record
    /// PATH-QUALITY-1, Q4); `None` otherwise. A spiral off the axis is
    /// ramped over its own path ([`Self::ramp`]), never round the part
    /// (PATH-QUALITY-1, Q3).
    #[allow(clippy::too_many_arguments)]
    fn ring_plan(
        &self,
        b: &Builder<'_>,
        input: &StrategyInput<'_>,
        stock: View<'_>,
        common: &CommonParameters,
        pass: usize,
        points: &[CutPoint],
        rapids: &[usize],
    ) -> Result<Option<RingEntry>> {
        if self.style != EntryStyle::Ramp {
            return Ok(None);
        }
        let shape = pass_shape::start(&b.rotary, points, rapids, self.tolerance);
        if let (Some(head), Start::FullRing { sign, .. } | Start::Spiral { sign }) =
            (points.first(), shape)
            && self.on_axis(b, Some(head))
        {
            // An end-face ring: the tip descends on the axis while A
            // turns (decision 0543 section 4, point 2, option (b)).
            let at = b.rotary.to_axis(head.tip, head.a);
            let radius = input.tool.cutting_radius();
            let (planned, tan, at_max) = self.steepen(|tan| {
                ring_ramp::plan_on_axis(
                    b,
                    stock,
                    common.approach_distance.0,
                    tan,
                    at,
                    head.a.0,
                    sign,
                    self.probe_height,
                    radius,
                    self.angle_step(radius).to_degrees(),
                )
            });
            return match planned {
                Ok(plan) => self
                    .in_a_range(input, pass, self.steepened(plan, tan))
                    .map(|p| Some(RingEntry::Ramp(p))),
                Err(ring_ramp::Refusal::NothingToDescend) => Ok(None),
                Err(ring_ramp::Refusal::TooManyTurns { .. })
                    if at_max && self.single_point_plunge =>
                {
                    Ok(Some(RingEntry::Plunge))
                }
                Err(ring_ramp::Refusal::TooManyTurns { turns, .. }) => Err(PathError::refused(
                    crate::topic::RAMP_ENTRY,
                    format!(
                        "pass {} starts on the rotary axis, where the tool \
                         descends while the part turns; {}, measured at the tool's radius \
                         of {radius:.3} mm, that takes {turns:.1} turns, more than the {:.0} \
                         allowed. {}",
                        pass + 1,
                        angle_words(tan, at_max),
                        ring_ramp::MAX_TURNS,
                        fix_words(at_max)
                    ),
                )
                .with_setting(fix_setting(at_max))),
            };
        }
        let Start::FullRing { end, sign } = shape else {
            return Ok(None);
        };
        let (planned, tan, at_max) = self.steepen(|tan| {
            ring_ramp::plan(
                b,
                stock,
                common.approach_distance.0,
                tan,
                points,
                end,
                sign,
                self.probe_height,
            )
        });
        let plan = match planned {
            Ok(plan) => self.steepened(plan, tan),
            Err(ring_ramp::Refusal::NothingToDescend) => return Ok(None),
            Err(ring_ramp::Refusal::TooManyTurns { .. }) if at_max && self.single_point_plunge => {
                return Ok(Some(RingEntry::Plunge));
            }
            Err(ring_ramp::Refusal::TooManyTurns { radius, turns }) => {
                return Err(PathError::refused(
                    crate::topic::RAMP_ENTRY,
                    format!(
                        "pass {} turns round the part with its tip {radius:.3} mm from \
                     the rotary axis, so {} a ramp round it would take {turns:.1} turns, more \
                     than the {:.0} allowed. {}",
                        pass + 1,
                        angle_words(tan, at_max),
                        ring_ramp::MAX_TURNS,
                        fix_words(at_max)
                    ),
                )
                .with_setting(fix_setting(at_max)));
            }
        };
        self.in_a_range(input, pass, plan)
            .map(|p| Some(RingEntry::Ramp(p)))
    }

    /// `plan`, marked with the angle used when it is steeper than the Ramp
    /// angle.
    fn steepened(&self, mut plan: RingRamp, tan: f64) -> RingRamp {
        if tan > self.ramp_tan * (1.0 + 1e-12) {
            plan.steepened = Some(tan.atan().to_degrees());
        }
        plan
    }

    /// The radius of the nubs plunged, or 0 when that is off.
    pub(super) fn nub_radius(&self) -> f64 {
        self.nub_radius.unwrap_or(0.0)
    }

    /// The stretch along X of a nub pass (decision 0543 section 4, point 3):
    /// a pass that carries its stock (roughing), every point of it no
    /// further from the rotary axis than the nub's radius, where the part
    /// leaves nothing round the axis (the tool dropped onto the axis meets
    /// nothing). `None` for any other pass.
    pub(super) fn nub(
        &self,
        b: &Builder<'_>,
        input: &StrategyInput<'_>,
        carries_stock: bool,
        points: &[CutPoint],
    ) -> Option<(f64, f64)> {
        let radius = self.nub_radius?;
        if !carries_stock || points.is_empty() {
            return None;
        }
        let (mut low, mut high) = (f64::INFINITY, f64::NEG_INFINITY);
        for p in points {
            let q = b.rotary.to_axis(p.tip, p.a);
            if q.across.hypot(q.height) > radius + EPS {
                return None;
            }
            low = low.min(q.along);
            high = high.max(q.along);
        }
        let spacing = (0.5 * self.tool_radius).max(1e-3);
        let count = (((high - low) / spacing).ceil() as usize).min(10_000);
        for k in 0..=count {
            let along = if count == 0 {
                low
            } else {
                low + (high - low) * k as f64 / count as f64
            };
            let at = AxisPosition {
                along,
                across: 0.0,
                height: 0.0,
            };
            for j in 0..8 {
                if self.touch_height(b, input, at, Degrees(45.0 * f64::from(j))) > self.tolerance {
                    return None;
                }
            }
        }
        Some((low, high))
    }

    /// Removes the nub of a nub pass from `low` to `high` along X: at most
    /// a tool diameter apart, the tool plunges to the rotary axis at the
    /// plunge feed and the part turns a whole turn, then the tool retracts
    /// (decision 0543 section 4, point 3). A stretch already cleared by an
    /// earlier plunge is skipped. Gives the plunges made.
    pub(super) fn plunge_nub(
        &self,
        b: &mut Builder<'_>,
        input: &StrategyInput<'_>,
        stock: View<'_>,
        common: &CommonParameters,
        (low, high): (f64, f64),
        cleared: &mut Vec<(f64, f64)>,
    ) -> Result<usize> {
        if cleared
            .iter()
            .any(|&(a, z)| a <= low + EPS && high <= z + EPS)
        {
            return Ok(0);
        }
        let diameter = 2.0 * self.tool_radius;
        let count = (((high - low) / diameter).ceil() as usize + 1).max(1);
        let places: Vec<f64> = if count == 1 || high - low <= EPS {
            vec![0.5 * (low + high)]
        } else {
            (0..count)
                .map(|k| low + (high - low) * k as f64 / (count - 1) as f64)
                .collect()
        };
        for &along in &places {
            b.retract(self.probe_height)?;
            let a = b.pose.a;
            let height = b.at().height;
            let over = b.rotary.to_model(
                AxisPosition {
                    along,
                    across: 0.0,
                    height,
                },
                a,
            );
            let pose = b.pose(over, a);
            b.push(MoveKind::Rapid, pose, None, None)?;
            let stop = stop_height(b, stock, common, 0.0);
            rapid_to(b, stop)?;
            let at = AxisPosition {
                along,
                across: 0.0,
                height: 0.0,
            };
            let tip = b.rotary.to_model(at, a);
            let pose = b.pose(tip, a);
            b.push(MoveKind::Plunge, pose, None, Some(common.plunge_feed))?;
            // A whole turn with the tip on the axis, the way the A range
            // allows.
            let sign = match input.rotary.range {
                openchuck_types::machine::ARange::Limited { max, .. }
                    if a.0 + 360.0 > max.0 + EPS =>
                {
                    -1.0
                }
                _ => 1.0,
            };
            if let openchuck_types::machine::ARange::Limited { min, max } = input.rotary.range
                && (a.0 + sign * 360.0 < min.0 - EPS || a.0 + sign * 360.0 > max.0 + EPS)
            {
                return Err(PathError::refused(
                    crate::topic::FACE_NUBS,
                    format!(
                        "the nub at X {along:.3} mm needs a whole turn of A from {:.3} \
                     degrees, outside the machine's A range of {:.3} to {:.3} degrees. Widen A \
                     range in the setup's Machine settings, or turn off Plunge to remove face \
                     nubs.",
                        a.0, min.0, max.0
                    ),
                )
                .with_setting(crate::params::names::PLUNGE_FACE_NUBS));
            }
            for k in 1..=8 {
                let turned = Degrees(a.0 + sign * 45.0 * f64::from(k));
                let tip = b.rotary.to_model(at, turned);
                let pose = b.pose(tip, turned);
                b.push(MoveKind::Cutting, pose, Some(tip), Some(common.feed))?;
            }
            b.retract(self.probe_height)?;
        }
        cleared.push((low - self.tool_radius, high + self.tool_radius));
        Ok(places.len())
    }

    /// Plans with the Ramp angle and, when that needs more than 10 turns and
    /// the steeper ramp near the axis is on, again at the shallowest angle
    /// whose plan fits 10 turns, never steeper than the maximum (decision
    /// 0543 section 4, point 3). The turns are those actually planned, not a
    /// proportion of the angle: a ring ramp adds a whole turn where the pass
    /// rises before its first point, so a few proportional tries can stop
    /// short of a maximum that fits (decision record PATH-QUALITY-1, Q4
    /// (ii)). Gives the plan or refusal, the tangent of the angle used last,
    /// and whether the search reached the maximum: always so when it is on
    /// and still refuses, so no refusal asks to turn on a setting that is
    /// already on.
    fn steepen<T>(
        &self,
        mut attempt: impl FnMut(f64) -> std::result::Result<T, ring_ramp::Refusal>,
    ) -> (std::result::Result<T, ring_ramp::Refusal>, f64, bool) {
        let first = attempt(self.ramp_tan);
        let Err(ring_ramp::Refusal::TooManyTurns { mut turns, .. }) = first else {
            return (first, self.ramp_tan, false);
        };
        if !self.steeper_near_axis {
            return (first, self.ramp_tan, false);
        }
        if self.tan_max <= self.ramp_tan {
            return (first, self.ramp_tan, true);
        }
        // Turns fall about as the tangent rises: scale it to fit 10 turns, a
        // few times, which settles most passes at once.
        let mut tan = self.ramp_tan;
        for _ in 0..4 {
            tan = (tan * turns / ring_ramp::MAX_TURNS * (1.0 + 1e-6)).min(self.tan_max);
            match attempt(tan) {
                Ok(plan) => return (Ok(plan), tan, false),
                Err(ring_ramp::Refusal::TooManyTurns { turns: t, .. }) => turns = t,
                other => return (other, tan, false),
            }
            if tan >= self.tan_max {
                return (attempt(self.tan_max), self.tan_max, true);
            }
        }
        // Then by the turns actually planned: the maximum, and if that fits,
        // the shallowest angle between that fits.
        let (mut low, mut high) = (tan, self.tan_max);
        let mut best = match attempt(high) {
            Ok(plan) => plan,
            other => return (other, high, true),
        };
        for _ in 0..24 {
            let mid = 0.5 * (low + high);
            match attempt(mid) {
                Ok(plan) => {
                    best = plan;
                    high = mid;
                }
                Err(ring_ramp::Refusal::TooManyTurns { .. }) => low = mid,
                other => return (other, mid, false),
            }
            if high - low <= 1e-6 * high {
                break;
            }
        }
        (Ok(best), high, false)
    }

    /// Refuses `plan` when its start is outside a limited A range (decision
    /// 0543 D6).
    fn in_a_range(
        &self,
        input: &StrategyInput<'_>,
        pass: usize,
        plan: RingRamp,
    ) -> Result<RingRamp> {
        if let (openchuck_types::machine::ARange::Limited { min, max }, Some((_, a))) =
            (input.rotary.range, plan.start())
            && (a.0 < min.0 - EPS || a.0 > max.0 + EPS)
        {
            return Err(PathError::refused(
                crate::topic::RAMP_ENTRY,
                format!(
                    "pass {} needs A from {:.3} degrees to ramp round the part, outside \
                 the machine's A range of {:.3} to {:.3} degrees. Widen A range in the setup's \
                 Machine settings, or set Entry to Plunge or Helix.",
                    pass + 1,
                    a.0,
                    min.0,
                    max.0
                ),
            )
            .with_setting(crate::params::names::ENTRY));
        }
        Ok(plan)
    }

    /// Where the entry of this pass starts, when it is not over the pass's
    /// first cut point at the pass's first A: the link goes there
    /// (decision 0543 section 2).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn start(
        &self,
        b: &Builder<'_>,
        input: &StrategyInput<'_>,
        stock: View<'_>,
        common: &CommonParameters,
        pass: usize,
        points: &[CutPoint],
        rapids: &[usize],
    ) -> Result<Option<CutPoint>> {
        if let Some(start) = self.turning_lead_start(b, points, rapids) {
            return Ok(Some(start));
        }
        Ok(self
            .ring_plan(b, input, stock, common, pass, points, rapids)?
            .and_then(|entry| match entry {
                RingEntry::Ramp(plan) => plan.start(),
                RingEntry::Plunge => None,
            })
            .map(|(pos, a)| {
                let tip = b.rotary.to_model(pos, a);
                CutPoint {
                    tip,
                    a,
                    contact: tip,
                }
            }))
    }

    /// Emits a planned ring or spiral ramp.
    fn ring_ramp(
        &self,
        b: &mut Builder<'_>,
        input: &StrategyInput<'_>,
        common: &CommonParameters,
        pass: usize,
        plan: &RingRamp,
    ) -> Result<()> {
        let kind = if plan.on_axis.is_some() {
            MoveKind::Plunge
        } else {
            MoveKind::LeadIn
        };
        self.emit(b, input, common, pass, &plan.moves, &plan.floors, kind)?;
        if plan.follows_steep_drop {
            b.steep_ramps.push(pass);
        }
        if let Some(angle) = plan.steepened {
            b.steepened_ramps.push((pass, angle));
        }
        if let Some(turns) = plan.on_axis {
            b.on_axis_ramps.push((pass, turns));
            b.straight_plunges.push(pass);
        }
        // The load figures for the note (decision record PATH-QUALITY-1, Q3
        // point 4): a ring ramp descends one turn's depth per traverse; a
        // descent on the axis, the rise over its turns at the tool's edge.
        let (Some((first, a_first)), Some((last, a_last))) =
            (plan.moves.first().copied(), plan.moves.last().copied())
        else {
            return Ok(());
        };
        let rise = (first.height - last.height).max(0.0);
        let turns = (a_last.0 - a_first.0).abs() / 360.0;
        let travel: f64 = plan
            .moves
            .windows(2)
            .map(|w| match w {
                [(q, b0), (p, a)] => step_length(*q, *b0, *p, *a),
                _ => 0.0,
            })
            .sum();
        let tan = plan
            .steepened
            .map_or(self.ramp_tan, |d| d.to_radians().tan());
        let depth = if plan.on_axis.is_some() || travel <= EPS {
            rise / turns.max(1.0)
        } else {
            (rise * 360.0 / (a_last.0 - a_first.0).abs().max(EPS)).min(rise)
        };
        b.ramp_loads.push(super::RampLoad {
            pass,
            angle: tan.atan().to_degrees(),
            legs: None,
            turns: Some(turns),
            depth,
        });
        Ok(())
    }

    /// The height at which the tool, at `pos`'s X and Y and angle `a`,
    /// meets the finished surface with the stock to leave (negative
    /// infinity where it meets nothing).
    fn touch_height(
        &self,
        b: &Builder<'_>,
        input: &StrategyInput<'_>,
        pos: AxisPosition,
        a: Degrees,
    ) -> f64 {
        let Some(grown) = self.grown.as_ref() else {
            return f64::NEG_INFINITY;
        };
        let line = ToolLine {
            origin: b.rotary.to_model(
                AxisPosition {
                    height: self.probe_height,
                    ..pos
                },
                a,
            ),
            axis: b.rotary.tool_axis(a),
        };
        input
            .surface
            .drop_tool(grown, &line)
            .map_or(f64::NEG_INFINITY, |c| {
                self.probe_height + c.offset + self.leave
            })
    }

    /// Segments in a quarter circle of radius `r`.
    fn arc_segments(&self, r: f64) -> usize {
        ((FRAC_PI_2 / self.angle_step(r)).ceil() as usize).clamp(2, MAX_SEGMENTS)
    }

    /// The largest angle a chord of the path tolerance allows on radius `r`.
    fn angle_step(&self, r: f64) -> f64 {
        if self.chord >= r {
            FRAC_PI_2
        } else {
            (2.0 * (1.0 - self.chord / r).acos()).min(FRAC_PI_2)
        }
    }

    /// The worst gouge of the finished surface, or of the faces next to the
    /// face set, over `positions` (tool tip positions at angle `a`), if any
    /// exceeds what is allowed; `join` is the pass's point the move meets.
    fn gouge(
        &self,
        b: &Builder<'_>,
        input: &StrategyInput<'_>,
        a: Degrees,
        positions: &[AxisPosition],
        join: AxisPosition,
    ) -> Option<Gouge> {
        let moves: Vec<(AxisPosition, Degrees)> = positions.iter().map(|p| (*p, a)).collect();
        self.gouge_moves(b, input, &moves, (join, a))
    }

    /// The worst gouge over `moves`, each a tool tip position at its own A:
    /// of the finished surface, deeper than the path tolerance; failing
    /// that, of the faces next to the face set, deeper than the pass itself
    /// cuts them at `join` (the pass's point the move meets) plus the
    /// tolerance, as that cut is the pass's and is measured and warned of
    /// with it (decision records 0532 item 3 and 0682).
    pub(super) fn gouge_moves(
        &self,
        b: &Builder<'_>,
        input: &StrategyInput<'_>,
        moves: &[(AxisPosition, Degrees)],
        join: (AxisPosition, Degrees),
    ) -> Option<Gouge> {
        let grown = self.grown.as_ref()?;
        let rotary = b.rotary;
        let leave = self.leave;
        let line = |pos: AxisPosition, a: Degrees| ToolLine {
            origin: rotary.to_model(pos, a),
            axis: rotary.tool_axis(a),
        };
        let x = |i: usize| {
            moves
                .get(i)
                .map_or(f64::NAN, |(p, a)| rotary.to_model(*p, *a).x)
        };
        // Every position the tool will be sent to, in parallel and in order
        // (NFR-06); the depth of each below the finished surface.
        let depths: Vec<Option<f64>> = parallel::map_in_order(moves, |(pos, a)| {
            input
                .surface
                .drop_tool(grown, &line(*pos, *a))
                .map(|c| c.offset + leave)
        });
        let mut worst: Option<Gouge> = None;
        for (i, depth) in depths.into_iter().enumerate() {
            if let Some(d) = depth
                && d > self.tolerance
                && worst.is_none_or(|w| d > w.depth)
            {
                worst = Some(Gouge {
                    depth: d,
                    position: i,
                    x: x(i),
                    face: None,
                });
            }
        }
        if worst.is_some() {
            return worst;
        }
        if self.beside.is_empty() {
            return None;
        }
        let at = |(pos, a): (AxisPosition, Degrees)| (rotary.to_model(pos, a), rotary.tool_axis(a));
        // Face by face: the pass's own cut into that face where the move
        // meets it, the tolerance, and how much deeper a cut can be between
        // that face's samples than at them. A face the pass cuts (a floor
        // under a ball) excuses nothing on another face (CodeRabbit on #866).
        let own = self.beside_cuts(at(join));
        let cuts: Vec<BTreeMap<u32, f64>> =
            parallel::map_in_order(moves, |m| self.beside_cuts(at(*m)));
        let mut deepest: Option<Gouge> = None;
        for (i, cut) in cuts.into_iter().enumerate() {
            for (face, d) in cut {
                let allowed = own.get(&face).copied().unwrap_or(0.0)
                    + self.tolerance
                    + self.beside_slack.get(&face).copied().unwrap_or(0.0);
                if d > allowed && deepest.is_none_or(|w| d > w.depth) {
                    deepest = Some(Gouge {
                        depth: d,
                        position: i,
                        x: x(i),
                        face: Some(face),
                    });
                }
            }
        }
        deepest
    }

    /// The deepest cut into each face next to the face set by the tool with
    /// its tip at `tip` along `axis`, by face number; a face it does not cut
    /// is left out. A map in face-number order, so the same on any thread
    /// (NFR-06).
    fn beside_cuts(&self, (tip, axis): (Point3, UnitVec3)) -> BTreeMap<u32, f64> {
        let mut cuts: BTreeMap<u32, f64> = BTreeMap::new();
        for (p, face) in &self.beside {
            // Outside the tool's radius or below its tip: not in the tool.
            if cutter::bound_at(self.tool_radius, tip, axis, *p) >= 0.0 {
                continue;
            }
            let d = -cutter::clearance_at(&self.cutter, tip, axis, *p);
            if d > 0.0 {
                let deepest = cuts.entry(*face).or_insert(0.0);
                *deepest = deepest.max(d);
            }
        }
        cuts
    }

    /// The refusal of an entry or exit that would cut into the finished
    /// surface. Where that is a held region (the tool is then over its
    /// keep-out), the message names it, as the user may not see why a ramp
    /// beside the part fails (SET-09; decision records 0114, 0161).
    fn refuse(
        &self,
        topic: &str,
        input: &StrategyInput<'_>,
        pass: usize,
        name: &str,
        size: &str,
        found: Gouge,
    ) -> PathError {
        let mid = crate::strategy::part_middle(input);
        if let Some(end) = crate::strategy::held_end(input.keep_out, mid, found.x, found.x) {
            let held = crate::strategy::held_names(&std::collections::BTreeSet::from([end]));
            return PathError::refused(
                topic,
                format!(
                    "pass {}: the {name} needs {size}, and it would run over {held} at X \
                 {:.3} mm, where the workholding grips the stock; use a plunge entry, or a size \
                 or angle that keeps it clear of the held region",
                    pass + 1,
                    found.x
                ),
            )
            .with_setting(crate::params::names::ENTRY);
        }
        PathError::refused(
            topic,
            format!(
                "pass {}: the {name} needs {size}, and the tool would cut {:.3} mm into {} at \
             position {} of its path (the tolerance is {:.3} mm); try another entry style, a \
             smaller size or a steeper angle",
                pass + 1,
                found.depth,
                found.what(),
                found.position + 1,
                self.tolerance
            ),
        )
        .with_setting(crate::params::names::ENTRY)
    }
}

/// The deepest point an entry or exit would cut into the finished surface.
#[derive(Debug, Clone, Copy)]
pub(super) struct Gouge {
    /// How far into the surface, in millimetres.
    depth: f64,
    /// Its index among the positions checked.
    position: usize,
    /// The model X of the tool's tip there (NaN when not known).
    x: f64,
    /// The number of the face next to the face set it cuts, deeper than the
    /// pass does (decision record 0682); `None` for the finished surface.
    face: Option<u32>,
}

impl Gouge {
    /// What is cut, for a refusal: the finished surface, or the face next
    /// to the face set, named, with the pass's own cut allowed.
    fn what(&self) -> String {
        match self.face {
            None => "the finished surface".to_owned(),
            Some(n) => format!(
                "face {n}, which shares an edge with the face set, deeper than the pass itself \
                 cuts it"
            ),
        }
    }
}

impl Default for Gouge {
    fn default() -> Self {
        Self {
            depth: 0.0,
            position: 0,
            x: f64::NAN,
            face: None,
        }
    }
}

/// Samples of every exact face of `faces`, in face-number order, each with
/// its number, as swarf samples the faces next to a face (decision record
/// 0532 item 1) for a tool of cutting radius `radius`; and, for each face,
/// the most its cut can be deeper between its samples than at them, from
/// the spacing its samples actually have.
fn beside_samples(
    faces: &dyn SurfaceSource,
    radius: f64,
) -> (Vec<(Point3, u32)>, BTreeMap<u32, f64>) {
    let spacing = crate::swarf::guard_spacing(radius);
    let mut list = faces.exact_faces();
    list.sort_by_key(|f| f.number);
    list.dedup_by_key(|f| f.number);
    let mut points = Vec::new();
    let mut slack = BTreeMap::new();
    for f in list {
        let samples = crate::swarf::face_samples(f.face, spacing);
        let apart = samples
            .iter()
            .map(|(_, _, apart)| *apart)
            .fold(spacing, f64::max);
        slack.insert(f.number, apart * std::f64::consts::FRAC_1_SQRT_2);
        points.extend(samples.into_iter().map(|(p, _, _)| (p, f.number)));
    }
    (points, slack)
}

/// The height a rapid descent over the line the tool is on stops at: the
/// approach distance above where the stock begins on that line (or above the
/// goal, whichever is higher), and never above where the tool is.
fn stop_height(b: &Builder<'_>, stock: View<'_>, common: &CommonParameters, goal: f64) -> f64 {
    let here = b.at();
    let travel = (here.height - goal).max(0.0);
    let entry_height = super::footprint_entry(b, stock, here, b.pose.a, here.height, travel)
        .map(|d| here.height - d);
    let stop = goal.max(entry_height.unwrap_or(f64::NEG_INFINITY)) + common.approach_distance.0;
    stop.min(here.height)
}

/// A rapid straight down to `height`, if the tool is above it.
fn rapid_to(b: &mut Builder<'_>, height: f64) -> Result<()> {
    let here = b.at();
    if height < here.height - EPS {
        let tip = b.rotary.to_model(AxisPosition { height, ..here }, b.pose.a);
        let end = b.pose(tip, b.pose.a);
        b.push(MoveKind::Rapid, end, None, None)?;
    }
    Ok(())
}

/// Down to the approach distance above the stock, then a plunge at the plunge
/// feed.
fn plunge(
    b: &mut Builder<'_>,
    stock: View<'_>,
    common: &CommonParameters,
    target: &CutPoint,
) -> Result<()> {
    plunge_depth(b, stock, common, target).map(|_| ())
}

/// [`plunge`], giving the depth it plunges into the stock: the whole descent
/// below where the stock begins over the point, in one move (decision record
/// PATH-QUALITY-1, Q4: the depth per traverse of a plunge).
fn plunge_depth(
    b: &mut Builder<'_>,
    stock: View<'_>,
    common: &CommonParameters,
    target: &CutPoint,
) -> Result<f64> {
    let goal = b.rotary.to_axis(target.tip, target.a);
    let stop = stop_height(b, stock, common, goal.height);
    rapid_to(b, stop)?;
    let end = b.pose(target.tip, target.a);
    b.push(MoveKind::Plunge, end, None, Some(common.plunge_feed))?;
    Ok((stop - common.approach_distance.0 - goal.height).max(0.0))
}

/// The unit horizontal direction of the first move of the pass that has one.
fn direction_at_start(b: &Builder<'_>, points: &[CutPoint]) -> Option<Dir> {
    let first = points.first()?;
    let p = b.rotary.to_axis(first.tip, first.a);
    points.iter().skip(1).find_map(|q| {
        let q = b.rotary.to_axis(q.tip, q.a);
        unit((q.along - p.along, q.across - p.across))
    })
}

/// The unit horizontal direction of the last move of the pass that has one.
fn direction_at_end(b: &Builder<'_>, points: &[CutPoint]) -> Option<Dir> {
    let last = points.last()?;
    let p = b.rotary.to_axis(last.tip, last.a);
    points.iter().rev().skip(1).find_map(|q| {
        let q = b.rotary.to_axis(q.tip, q.a);
        unit((p.along - q.along, p.across - q.across))
    })
}

fn unit(v: Dir) -> Option<Dir> {
    let n = v.0.hypot(v.1);
    (n > 1e-6).then(|| (v.0 / n, v.1 / n))
}

/// The travel along `path`: its length across the machine plus, where A
/// turns, the arc length at the step's mean distance from the rotary axis.
fn travel_of(path: &[(AxisPosition, Degrees)]) -> f64 {
    path.windows(2)
        .map(|w| match w {
            [(q, b), (p, a)] => step_length(*q, *b, *p, *a),
            _ => 0.0,
        })
        .sum()
}

/// The travel of one step (see [`travel_of`]).
fn step_length(q: AxisPosition, b: Degrees, p: AxisPosition, a: Degrees) -> f64 {
    let radius = 0.5 * (p.across.hypot(p.height) + q.across.hypot(q.height));
    (p.along - q.along).hypot(p.across - q.across) + (a.0 - b.0).abs().to_radians() * radius
}

/// The highest stock under the tool's footprint along `path` from its first
/// point up to a travel of `reach` (decision 0543 D5), probed from `probe`
/// (above all the stock) at most half a tool radius apart; negative
/// infinity where there is none.
fn stock_top_along(
    b: &Builder<'_>,
    stock: View<'_>,
    path: &[(AxisPosition, Degrees)],
    reach: f64,
    probe: f64,
) -> f64 {
    let spacing = (0.5 * b.tool_radius).max(1e-3);
    let mut top = f64::NEG_INFINITY;
    let mut probe_at = |pos: AxisPosition, a: Degrees| {
        let over = AxisPosition {
            height: probe,
            ..pos
        };
        if let Some(d) = super::footprint_entry(b, stock, over, a, probe, 2.0 * probe.abs() + 1.0) {
            top = top.max(probe - d);
        }
    };
    let Some(&(first, a0)) = path.first() else {
        return top;
    };
    probe_at(first, a0);
    let mut travelled = 0.0;
    for w in path.windows(2) {
        let [(q, b0), (p, a)] = w else { continue };
        if travelled >= reach {
            break;
        }
        let length = step_length(*q, *b0, *p, *a);
        let n = ((length / spacing).ceil() as usize).clamp(1, 10_000);
        for k in 1..=n {
            let f = k as f64 / n as f64;
            if travelled + f * length > reach + spacing {
                break;
            }
            probe_at(
                AxisPosition {
                    along: q.along + f * (p.along - q.along),
                    across: q.across + f * (p.across - q.across),
                    height: q.height + f * (p.height - q.height),
                },
                Degrees(b0.0 + f * (a.0 - b0.0)),
            );
        }
        travelled += length;
    }
    top
}

/// The angle of a refusal of too many turns: the Ramp angle, or the
/// Maximum ramp angle when the steeper ramp near the axis reached it.
fn angle_words(tan: f64, at_max: bool) -> String {
    let degrees = tan.atan().to_degrees();
    if at_max {
        format!("even at the Maximum ramp angle of {degrees:.2} degrees")
    } else {
        format!("at {degrees:.2} degrees")
    }
}

/// What to change after a refusal of too many turns.
/// The setting [`fix_words`] names first.
fn fix_setting(at_max: bool) -> &'static str {
    if at_max {
        crate::params::names::RAMP_ANGLE_MAX
    } else {
        crate::params::names::RAMP_ANGLE
    }
}

fn fix_words(at_max: bool) -> &'static str {
    if at_max {
        // Reached only with Allow single-point plunge off: on, such a pass
        // is plunged (decision record PATH-QUALITY-1, Q4).
        "Raise Maximum ramp angle, or turn on Allow single-point plunge, or set Entry to \
         Plunge or Helix."
    } else {
        "Raise Ramp angle, or turn on Steeper ramp near the axis, or set Entry to Plunge or Helix."
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::BrepSurface;
    use openchuck_testkit::ruled::wall_plane;

    /// CodeRabbit on #866 (decision record 0682): a face sampled no finer
    /// than 400 points each way is coarser than the spacing asked for when
    /// it is large, so its slack comes from its own samples' spacing.
    #[test]
    fn req_op_12_a_large_face_next_to_the_face_set_takes_the_slack_of_its_own_samples() {
        let radius = 3.0;
        let asked = crate::swarf::guard_spacing(radius);
        let slack = |x1: f64| {
            let surface = BrepSurface::new(&wall_plane(8.0, 0.0, x1, 0.0, 10.0)).unwrap();
            let (points, slack) = beside_samples(&surface, radius);
            assert!(!points.is_empty());
            assert_eq!(slack.len(), 1);
            slack.values().copied().next().expect("one face")
        };
        let small = slack(10.0);
        assert!(
            (small - asked * std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-9,
            "{small}"
        );
        // 1000 mm on at most 400 samples: about 2.5 mm apart.
        let large = slack(1000.0);
        assert!(large > 2.4 * std::f64::consts::FRAC_1_SQRT_2, "{large}");
    }
}
