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

use openchuck_types::geometry::{Point3, UnitVec3};
use openchuck_types::ir::MoveKind;
use openchuck_types::tool::profile::RevolvedProfile;
use openchuck_types::units::Degrees;

use super::beside::{Allowance, Beside};
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
    /// A whole ring whose one turn round itself would take more than the
    /// bound (decision 0695, Q9 part 3, the owner's answer A), at the
    /// tangent its ramp round the ring was planned at: the Ramp angle, or
    /// the steeper one near the axis.
    Ring(f64),
}

/// What holds a back-and-forth ramp's depth per traverse (decision
/// 0695, Q9, and its agreement with lane TOOL-LOAD): the bound
/// [`largest_depth_per_traverse`] took.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DepthBound {
    /// The tool's cutting radius.
    ToolRadius,
    /// One turn of a spiral ramp at its start radius.
    OneTurn,
    /// The Largest depth per traverse setting, below the tool radius.
    Setting,
    /// The cap lane TOOL-LOAD's TL-2 gives from Limit tool load (`L / D`);
    /// nothing passes one until TL-2 is built, so no ramp names it yet.
    ToolLoadLimit,
}

impl DepthBound {
    /// Its name in the ramp note.
    pub(super) const fn words(self) -> &'static str {
        match self {
            Self::ToolRadius => "the tool radius",
            Self::OneTurn => "one turn round the part",
            Self::Setting => "the Largest depth per traverse setting",
            Self::ToolLoadLimit => "the tool-load limit",
        }
    }
}

/// A back-and-forth ramp's largest depth per traverse, in mm, and the bound
/// that sets it (decision 0695, Q8 and Q9, the owner's
/// answers, and the agreement with lane TOOL-LOAD): the least of
///
/// - the tool's cutting radius;
/// - `one_turn`, one turn of a spiral ramp at its start radius (`2 pi r
///   tan`), for a pass that starts as a spiral only;
/// - `setting`, the Largest depth per traverse, where it is above 0 (0
///   means the tool radius, and more is held to it);
/// - `cap`, the tool-load limit in mm: `None` here; lane TOOL-LOAD's TL-2
///   passes `Some(L / D)` at the call site when Limit tool load is on.
///
/// A bound that is not a positive length (a NaN or a cap of 0) bounds
/// nothing (NFR-07). Where two are equal the earlier in that list is
/// named. It holds a whole ring too (Q9 part 3, the owner's answer A): a
/// ring whose one turn round itself would take more is ramped back and
/// forth over itself instead ([`Entry::ring_plan`]). A descent on the
/// rotary axis is not held by it (an open question for the owner, OI-2239).
fn largest_depth_per_traverse(
    tool_radius: f64,
    setting: f64,
    one_turn: Option<f64>,
    cap: Option<f64>,
) -> (f64, DepthBound) {
    let mut out = (tool_radius, DepthBound::ToolRadius);
    for (value, bound) in [
        (one_turn, DepthBound::OneTurn),
        (Some(setting), DepthBound::Setting),
        (cap, DepthBound::ToolLoadLimit),
    ] {
        if let Some(v) = value
            && v.is_finite()
            && v > 0.0
            && v < out.0
        {
            out = (v, bound);
        }
    }
    out
}

/// The entry [`Entry::ring_plan`] chooses.
enum RingEntry {
    /// A ramp round the ring, or a descent on the axis.
    Ramp(RingRamp),
    /// No ramp fits in 10 turns even at the Maximum ramp angle: plunged,
    /// as Plunge where no ramp fits asks.
    Plunge,
    /// A ring whose one turn round itself would take more than the largest
    /// depth per traverse: ramped back and forth over its own points up to
    /// `end`, at the tangent `tan` the ramp round it was planned at
    /// (decision 0695, Q9 part 3, the owner's answer A).
    BackAndForth { end: usize, tan: f64 },
}

pub(super) struct Entry<'a> {
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
    /// The Largest depth per traverse setting, mm: 0 means the tool radius.
    depth_setting: f64,
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
    /// The faces beside the operation's surface a lead or helix is also
    /// measured against, with their allowance: swarf's faces next to the
    /// face set (decision record 0682; issue #572), or every other
    /// strategy's faces outside the selection (decision record 0701; issue
    /// #957). `None` for a plain plunge or ramp, or with neither.
    beside: Option<Beside<'a>>,
    /// The tool's profile, which those faces are measured against, without
    /// the stock to leave, as swarf measures them.
    cutter: RevolvedProfile,
}

impl<'a> Entry<'a> {
    pub(super) fn new(
        input: &StrategyInput<'_>,
        common: &CommonParameters,
        e: &EntryParameters,
        beside: Option<(&'a dyn SurfaceSource, Allowance)>,
    ) -> Result<Self> {
        let radius = input.tool.cutting_radius();
        let leave = common.stock_to_leave.0;
        let beside = match (beside, e.style) {
            (Some((faces, allowance)), EntryStyle::Lead | EntryStyle::Helix) => {
                Beside::new(faces, radius, allowance)
            }
            _ => None,
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
            depth_setting: e.ramp_depth_per_traverse.0,
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
            beside,
            cutter: input.tool.clone(),
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
            Some(RingEntry::BackAndForth { end, tan }) => {
                return self.follow(b, input, stock, common, pass, points, end, Shape::Ring(tan));
            }
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
    /// line, a partial ring or a spiral, back and forth (decision
    /// 0695, Q3: no ramp goes round the part); a pass that starts
    /// as a whole ring, or on the rotary axis, is ramped round the ring (or
    /// back and forth over it, held to the bound) or descended on the axis
    /// before it reaches here ([`Self::ring_plan`]),
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
        // The last point before the pass's first rapid, in one scan of the
        // rapids.
        let last = rapids
            .iter()
            .copied()
            .filter(|k| (1..points.len()).contains(k))
            .min()
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
            // the entry only, never round the part (decision
            // 0695, Q3, amending 0543 D2 and 0645 item 1), its legs
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
    /// 10 turns even at the Maximum ramp angle. Refused unless Plunge where
    /// no ramp fits is on; then plunged, listed for the host's check
    /// of a tool that cannot cut straight down, and named in a note with its
    /// reason and its depth, the whole descent in one move (decision 0543
    /// D7; decision 0695, Q4, the owner's answer).
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
                     on Plunge where no ramp fits, or set Entry to Plunge or Helix.",
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
    /// the axis as for a ring (decision 0695, Q3 and Q4). A whole ring
    /// held to the bound goes back and forth over itself at the angle its
    /// ramp round it was planned at, steepened or not (Q9 part 3, the
    /// owner's answer A).
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
        let total = travel_of(&path);
        let approach = common.approach_distance.0;
        // The stock over the first point: none means nothing to descend into.
        let here = stock_top_along(b, stock, &path, 0.0, self.probe_height);
        if here <= goal.height + EPS {
            return plunge(b, stock, common, &head);
        }
        // The shortest stretch a ramp goes back and forth over: ten path
        // tolerances, at most the tool radius (decision 0543 D3 as amended
        // for issue #687, accepted). The angle holds on any length (each leg
        // of length L descends L tan(angle)), so the length sets only the
        // number of legs, which `MAX_LEGS` bounds; a shorter stretch is
        // within the path's own precision, a plunge in all but name, and is
        // entered as a single point is (decision 0695, Q4
        // (i)). Over a stretch shorter than the tool radius the tool's centre
        // may go down into stock its edge has not cut, so the pass is listed
        // for the host's check of a tool that cannot cut straight down.
        let radius = input.tool.cutting_radius();
        let minimum = (RAMP_MINIMUM_TOLERANCES * self.tolerance).min(radius);
        // The longest leg at an angle, and the largest depth per traverse
        // that holds it with its bound. A leg of length S takes `2 S tan`
        // at its far end, so S is at most half the largest depth per
        // traverse over `tan` (decision 0695, Q8, the owner's answer A, and
        // Q9, B: lines and partial rings too, taking more legs instead). On
        // a pass that starts as a spiral, one turn of a spiral ramp at the
        // start radius descends `2 pi r tan`, and held by it S is half the
        // turn, `pi r`, taken as that (not `2 pi r tan / 2 tan`) so the
        // program is the one it was before the bound had a function.
        let longest_at = |tan: f64| match shape {
            Shape::Spiral(r) => {
                let per_turn = std::f64::consts::TAU * r.max(EPS);
                let (depth, bound) = largest_depth_per_traverse(
                    radius,
                    self.depth_setting,
                    Some(per_turn * tan),
                    None,
                );
                let longest = if bound == DepthBound::OneTurn {
                    0.5 * per_turn
                } else {
                    0.5 * depth / tan
                };
                (longest, (bound, depth))
            }
            Shape::Line | Shape::Arc | Shape::Ring(_) => {
                let (depth, bound) =
                    largest_depth_per_traverse(radius, self.depth_setting, None, None);
                (0.5 * depth / tan, (bound, depth))
            }
        };
        // D5: the highest stock under the stretch the ramp uses. No leg goes
        // further along the path than the longest leg at the angle (nor than
        // the path), and the legs may be lengthened to that while the ramp is
        // fitted, so the stock is probed that far: never less, so the ramp
        // starts above the stock under every leg (OP-12), and no further,
        // so taller stock past the legs neither raises the ramp's start nor
        // refuses it (CodeRabbit on #930). Shortened legs (see
        // `path_ramp::back_and_forth`) stay within it.
        let top_at = |tan: f64| {
            let reach = longest_at(tan).0.min(total);
            here.max(stock_top_along(b, stock, &path, reach, self.probe_height)) + approach
        };
        let what = match shape {
            Shape::Line => "line",
            Shape::Arc | Shape::Spiral(_) => "stretch",
            Shape::Ring(_) => "ring",
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
                     mm ({why}) a ramp needs to go back and forth over. Turn on Plunge where \
                     no ramp fits, or set Entry to Plunge or Helix.",
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
        let (tan, top, longest, bound) = match shape {
            Shape::Spiral(r) => {
                let per_turn = std::f64::consts::TAU * r.max(EPS);
                let (fitted, tan, at_max) = self.steepen(|tan| {
                    let top = top_at(tan);
                    // The descent down to the lowest point of the stretch
                    // the legs use, as the stock is probed (D5), not of the
                    // whole pass (CodeRabbit on #930, issue #951).
                    let low = lowest_along(&path, longest_at(tan).0.min(total));
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
                        let (longest, bound) = longest_at(tan);
                        (tan, top, longest, bound)
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
                                fix_words(at_max, self.tan_max > self.ramp_tan)
                            ),
                        )
                        .with_setting(fix_setting(at_max)));
                    }
                    Err(ring_ramp::Refusal::NothingToDescend) => {
                        return plunge(b, stock, common, &head);
                    }
                }
            }
            // Every back-and-forth ramp keeps its depth per traverse within
            // its bound (see `longest_at`): lines and partial rings too.
            Shape::Line | Shape::Arc => {
                let (longest, bound) = longest_at(self.ramp_tan);
                (self.ramp_tan, top_at(self.ramp_tan), longest, bound)
            }
            // A whole ring held to the bound: the angle its ramp round it
            // was planned at, named in the note below when steeper.
            Shape::Ring(tan) => {
                let (longest, bound) = longest_at(tan);
                (tan, top_at(tan), longest, bound)
            }
        };
        if matches!(shape, Shape::Ring(_)) && tan > self.ramp_tan * (1.0 + 1e-12) {
            b.steepened_ramps.push((pass, tan.atan().to_degrees()));
        }
        let super::path_ramp::Ramp {
            moves,
            floors,
            leg,
            legs,
            steep,
            depth: cut,
        } = super::path_ramp::back_and_forth(&path, top, tan, minimum, radius, longest).map_err(
            |refusal| {
                let (text, setting) = match refusal {
                    super::path_ramp::Refusal::TooShort { length } => (
                        format!(
                            "pass {} starts with a {what} of {length:.3} mm, shorter than the \
                         {minimum:.3} mm a ramp needs to go back and forth over. Turn on \
                         Plunge where no ramp fits, or set Entry to Plunge or Helix.",
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
        // The load figures for the note (decision 0695, Q3
        // point 4): the angle, the legs, and the depth per traverse, the
        // deepest drop the ramp cuts between one visit to a place and the
        // next (`2 · leg · tan` at a leg's far end, unless the pass rises
        // under it; CodeRabbit on #930), to the nanometre so that ramps of
        // the same drop name the first of them as the deepest.
        b.ramp_loads.push(super::RampLoad {
            pass,
            angle: tan.atan().to_degrees(),
            legs: Some(legs),
            turns: None,
            depth: (cut * 1e9).round() / 1e9,
            bound: Some(bound),
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
    /// ramp angle and Plunge where no ramp fits is on (decision
    /// 0695, Q4); `None` otherwise. A spiral off the axis is
    /// ramped over its own path ([`Self::ramp`]), never round the part
    /// (decision 0695, Q3). A ring whose ramp round itself would cut more
    /// than the largest depth per traverse in a turn is ramped back and
    /// forth over itself instead, at the angle planned for it (Q9 part 3,
    /// the owner's answer A); a descent on the axis is not (OI-2239).
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
                        fix_words(at_max, self.tan_max > self.ramp_tan)
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
            // One turn round the ring deeper than the bound: back and forth
            // over the ring (Q9 part 3, the owner's answer A), under the one
            // bound function, which `follow` applies to the legs. The ring
            // is measured as its ramp round it would cut it,
            // `min(2 pi r tan, rise)`, the figure the note gives for it.
            Ok(plan)
                if plan.depth_per_traverse()
                    > largest_depth_per_traverse(
                        self.tool_radius,
                        self.depth_setting,
                        None,
                        None,
                    )
                    .0 + EPS =>
            {
                return Ok(Some(RingEntry::BackAndForth { end, tan }));
            }
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
                        fix_words(at_max, self.tan_max > self.ramp_tan)
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
    /// short of a maximum that fits (decision 0695, Q4
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
                RingEntry::Plunge | RingEntry::BackAndForth { .. } => None,
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
        // The load figures for the note (decision 0695, Q3
        // point 4): a ring ramp descends one turn's depth per traverse; a
        // descent on the axis, the rise over its turns at the tool's edge.
        let (Some((_, a_first)), Some((_, a_last))) =
            (plan.moves.first().copied(), plan.moves.last().copied())
        else {
            return Ok(());
        };
        let turns = (a_last.0 - a_first.0).abs() / 360.0;
        let tan = plan
            .steepened
            .map_or(self.ramp_tan, |d| d.to_radians().tan());
        let depth = plan.depth_per_traverse();
        b.ramp_loads.push(super::RampLoad {
            pass,
            angle: tan.atan().to_degrees(),
            legs: None,
            turns: Some(turns),
            depth,
            bound: None,
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
        let beside = self.beside.as_ref()?;
        let at = |(pos, a): (AxisPosition, Degrees)| (rotary.to_model(pos, a), rotary.tool_axis(a));
        // Only the faces the tool's cutting part comes near over these
        // moves and where they meet the pass (decision 0701, per-entry
        // proximity): the box of its tip and on up its axis at each, grown
        // by its radius, the stock to leave and the tolerance.
        let positions: Vec<(Point3, UnitVec3)> = moves
            .iter()
            .chain(std::iter::once(&join))
            .map(|m| at(*m))
            .collect();
        let reach = beside.reach(
            &positions,
            self.cutter.top(),
            self.tool_radius + leave + self.tolerance,
        )?;
        let near = beside.near(&reach);
        if near.is_empty() {
            return None;
        }
        // Face by face: the pass's own cut into that face where the move
        // meets it, which the faces next to swarf's face set allow (0682)
        // and the faces outside a selection do not (0701, the owner's Q3
        // answer); the tolerance; and how much deeper a cut can be between
        // that face's samples than at them. A face the pass cuts (a floor
        // under a ball) excuses nothing on another face (CodeRabbit on #866).
        let own = beside.cuts(&near, &self.cutter, self.tool_radius, at(join));
        let cuts: Vec<BTreeMap<usize, f64>> = parallel::map_in_order(moves, |m| {
            beside.cuts(&near, &self.cutter, self.tool_radius, at(*m))
        });
        let (i, face, depth, pass_cut) =
            deepest_beside(&cuts, &own, beside.allowance, self.tolerance, |f| {
                beside.slack(f)
            })?;
        Some(Gouge {
            depth,
            position: i,
            // The join is the position after the moves.
            x: if i < moves.len() {
                x(i)
            } else {
                rotary.to_model(join.0, join.1).x
            },
            face: Some(Struck {
                number: beside.number(face),
                allowance: beside.allowance,
                pass_cut: (pass_cut > self.tolerance + beside.slack(face)).then_some(pass_cut),
            }),
        })
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
        if let Some(text) = found.pass_cut_refusal(pass, name, self.tolerance) {
            return PathError::refused(topic, text).with_setting(crate::params::names::ENTRY);
        }
        let advice = if found.outside() {
            "set Entry to Plunge or Ramp, or use a smaller size"
        } else {
            "try another entry style, a smaller size or a steeper angle"
        };
        PathError::refused(
            topic,
            format!(
                "pass {}: the {name} needs {size}, and the tool would cut {:.3} mm into {} at \
             position {} of its path (the tolerance is {:.3} mm); {advice}",
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
    /// The face beside the operation's surface it cuts, deeper than its
    /// allowance; `None` for the finished surface.
    face: Option<Struck>,
}

/// The deepest cut into a face beside the operation's surface that is deeper
/// than its allowance, as (position, face, depth, the pass's own cut into
/// that face where the move meets it). `cuts` are each position's cuts by
/// face and `own` the cuts where the move meets the pass (the join).
/// Beside swarf's face set the join is the allowance (decision 0682); for
/// the faces outside a selection the join is a position the tool is sent to
/// like the others, after them, with only the tolerance and the slack
/// allowed (decision 0701, the owner's Q3 answer; CodeRabbit on #972).
pub(super) fn deepest_beside(
    cuts: &[BTreeMap<usize, f64>],
    own: &BTreeMap<usize, f64>,
    allowance: Allowance,
    tolerance: f64,
    slack: impl Fn(usize) -> f64,
) -> Option<(usize, usize, f64, f64)> {
    let join = match allowance {
        Allowance::PassCut => None,
        Allowance::Tolerance => Some(own),
    };
    let mut deepest: Option<(usize, usize, f64, f64)> = None;
    for (i, cut) in cuts.iter().chain(join).enumerate() {
        for (&face, &d) in cut {
            let pass_cut = own.get(&face).copied().unwrap_or(0.0);
            let allowed = match allowance {
                Allowance::PassCut => pass_cut,
                Allowance::Tolerance => 0.0,
            } + tolerance
                + slack(face);
            if d > allowed && deepest.is_none_or(|w| d > w.2) {
                deepest = Some((i, face, d, pass_cut));
            }
        }
    }
    deepest
}

/// A face beside the operation's surface that an entry or exit would cut.
#[derive(Debug, Clone, Copy)]
pub(super) struct Struck {
    number: u32,
    allowance: Allowance,
    /// How deep the pass itself cuts the face where the move meets it, when
    /// that is more than the tolerance (and the slack) allow: the faces
    /// outside a selection allow none of it (decision 0701).
    pass_cut: Option<f64>,
}

impl Gouge {
    /// What is cut, for a refusal: the finished surface, or the face next
    /// to the face set, named, with the pass's own cut allowed.
    fn what(&self) -> String {
        match self.face {
            None => "the finished surface".to_owned(),
            Some(Struck {
                number: n,
                allowance: Allowance::PassCut,
                ..
            }) => format!(
                "face {n}, which shares an edge with the face set, deeper than the pass itself \
                 cuts it"
            ),
            Some(Struck { number: n, .. }) => {
                format!("face {n}, outside the operation's selection,")
            }
        }
    }

    /// The pass's own cut into a face outside the selection where the move
    /// meets it, when it is more than allowed (decision 0701): the refusal
    /// then says so, as no lead or helix there can keep clear of the face.
    fn pass_cut(&self) -> Option<(u32, f64)> {
        match self.face {
            Some(Struck {
                number,
                allowance: Allowance::Tolerance,
                pass_cut: Some(d),
            }) => Some((number, d)),
            _ => None,
        }
    }

    /// The refusal where the pass itself cuts a face outside the selection
    /// where the move `name` of pass `pass` meets it (decision 0701, the
    /// owner's Q3 answer): no lead or helix there can keep clear of the
    /// face, so the message says so and what to do, honestly naming the
    /// pass's own cut, which nothing else measures.
    pub(super) fn pass_cut_refusal(
        &self,
        pass: usize,
        name: &str,
        tolerance: f64,
    ) -> Option<String> {
        let (n, own) = self.pass_cut()?;
        Some(format!(
            "pass {}: the pass itself cuts {own:.3} mm into face {n}, outside the operation's \
             selection, where the {name} meets it, so the {name} would cut it too ({:.3} mm at \
             position {} of its path; the tolerance is {tolerance:.3} mm). Set Entry to Plunge \
             or Ramp, or change the selection or the stock to leave so that the pass keeps \
             clear of face {n}",
            pass + 1,
            self.depth,
            self.position + 1,
        ))
    }

    /// Whether the face cut is outside the operation's selection.
    pub(super) fn outside(&self) -> bool {
        matches!(
            self.face,
            Some(Struck {
                allowance: Allowance::Tolerance,
                ..
            })
        )
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

/// The height a rapid descent over the line the tool is on stops at: the
/// approach distance above where the stock begins on that line (or above the
/// goal, whichever is higher), and never above where the tool is.
fn stop_height(b: &Builder<'_>, stock: View<'_>, common: &CommonParameters, goal: f64) -> f64 {
    stop_and_entry(b, stock, common, goal).0
}

/// [`stop_height`], and the height where the stock begins on that line
/// (none where there is no stock above the goal).
fn stop_and_entry(
    b: &Builder<'_>,
    stock: View<'_>,
    common: &CommonParameters,
    goal: f64,
) -> (f64, Option<f64>) {
    let here = b.at();
    let travel = (here.height - goal).max(0.0);
    let entry_height = super::footprint_entry(b, stock, here, b.pose.a, here.height, travel)
        .map(|d| here.height - d);
    let stop = goal.max(entry_height.unwrap_or(f64::NEG_INFINITY)) + common.approach_distance.0;
    (stop.min(here.height), entry_height)
}

/// The depth a plunge from `here` down to `goal` cuts into stock that
/// begins at `entry` on its line: from where the stock begins, or from
/// `here` when the tool is already lower than that, never from where the
/// rapid stopped (CodeRabbit on #930, issue #951); 0 with no stock.
fn depth_into_stock(here: f64, entry: Option<f64>, goal: f64) -> f64 {
    entry.map_or(0.0, |e| (e.min(here) - goal).max(0.0))
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
/// below where the stock begins over the point, in one move (decision
/// 0695, Q4: the depth per traverse of a plunge).
fn plunge_depth(
    b: &mut Builder<'_>,
    stock: View<'_>,
    common: &CommonParameters,
    target: &CutPoint,
) -> Result<f64> {
    let goal = b.rotary.to_axis(target.tip, target.a);
    let here = b.at().height;
    let (stop, entry) = stop_and_entry(b, stock, common, goal.height);
    rapid_to(b, stop)?;
    let end = b.pose(target.tip, target.a);
    b.push(MoveKind::Plunge, end, None, Some(common.plunge_feed))?;
    Ok(depth_into_stock(here, entry, goal.height))
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
pub(super) fn step_length(q: AxisPosition, b: Degrees, p: AxisPosition, a: Degrees) -> f64 {
    let radius = 0.5 * (p.across.hypot(p.height) + q.across.hypot(q.height));
    (p.along - q.along).hypot(p.across - q.across) + (a.0 - b.0).abs().to_radians() * radius
}

/// The lowest height of `path` from its first point up to a travel of
/// `reach`, the point at `reach` included.
fn lowest_along(path: &[(AxisPosition, Degrees)], reach: f64) -> f64 {
    let Some(&(first, _)) = path.first() else {
        return f64::INFINITY;
    };
    let mut low = first.height;
    let mut travelled = 0.0;
    for w in path.windows(2) {
        let [(q, b0), (p, a)] = w else { continue };
        let length = step_length(*q, *b0, *p, *a);
        if travelled + length >= reach {
            let f = if length > EPS {
                ((reach - travelled) / length).clamp(0.0, 1.0)
            } else {
                1.0
            };
            return low.min(q.height + f * (p.height - q.height));
        }
        low = low.min(p.height);
        travelled += length;
    }
    low
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

/// `can_steepen` is whether the Maximum ramp angle is steeper than the Ramp
/// angle: where it is not, turning on Steeper ramp near the axis cannot
/// help, so it is not asked for (CodeRabbit on #930, issue #951).
fn fix_words(at_max: bool, can_steepen: bool) -> &'static str {
    if at_max {
        // Reached only with Plunge where no ramp fits off: on, such a pass
        // is plunged (decision 0695, Q4).
        "Raise Maximum ramp angle, or turn on Plunge where no ramp fits, or set Entry to \
         Plunge or Helix."
    } else if can_steepen {
        "Raise Ramp angle, or turn on Steeper ramp near the axis, or set Entry to Plunge or Helix."
    } else {
        "Raise Ramp angle, or set Entry to Plunge or Helix."
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::BrepSurface;
    use openchuck_testkit::ruled::wall_plane;

    /// Decision 0695, Q9 (the owner's answer B) and the
    /// agreement with lane TOOL-LOAD: a back-and-forth ramp's largest depth
    /// per traverse is the least of the tool radius, one turn of a spiral
    /// ramp at its start radius, the Largest depth per traverse setting (0
    /// the tool radius, more held to it) and the tool-load cap, and says
    /// which held it.
    #[test]
    fn req_op_11_a_ramp_s_largest_depth_per_traverse_is_the_least_of_its_bounds() {
        use DepthBound::{OneTurn, Setting, ToolLoadLimit, ToolRadius};
        let bound = largest_depth_per_traverse;
        assert_eq!(bound(1.5, 0.0, None, None), (1.5, ToolRadius));
        assert_eq!(bound(1.5, 3.0, None, None), (1.5, ToolRadius));
        assert_eq!(bound(1.5, 1.5, None, None), (1.5, ToolRadius));
        assert_eq!(bound(1.5, 0.5, None, None), (0.5, Setting));
        assert_eq!(bound(1.5, 0.0, Some(1.2), None), (1.2, OneTurn));
        assert_eq!(bound(1.5, 0.0, Some(4.0), None), (1.5, ToolRadius));
        assert_eq!(bound(1.5, 0.5, Some(1.2), None), (0.5, Setting));
        assert_eq!(bound(1.5, 0.5, Some(0.3), None), (0.3, OneTurn));
        assert_eq!(bound(1.5, 0.5, Some(1.2), Some(0.2)), (0.2, ToolLoadLimit));
        assert_eq!(bound(1.5, 0.0, None, Some(2.0)), (1.5, ToolRadius));
        // A cap that is not a positive length bounds nothing (NFR-07).
        assert_eq!(bound(1.5, 0.0, None, Some(0.0)), (1.5, ToolRadius));
        assert_eq!(bound(1.5, 0.0, None, Some(f64::NAN)), (1.5, ToolRadius));
        assert_eq!(bound(1.5, f64::NAN, None, None), (1.5, ToolRadius));
        assert_eq!(ToolRadius.words(), "the tool radius");
        assert_eq!(OneTurn.words(), "one turn round the part");
        assert_eq!(Setting.words(), "the Largest depth per traverse setting");
        assert_eq!(ToolLoadLimit.words(), "the tool-load limit");
    }

    /// Issue #951, item 1 (CodeRabbit on #930): the depth a fallback plunge
    /// names is the descent below where the stock begins, also when the
    /// tool is already less than the approach distance above the stock,
    /// where the rapid stops where the tool is.
    #[test]
    fn req_op_11_req_nfr_12_a_plunge_s_depth_is_from_where_the_stock_begins() {
        // Stock begins at 10, the plunge goes to 7.
        assert!((depth_into_stock(30.0, Some(10.0), 7.0) - 3.0).abs() < 1e-12);
        // The tool 0.5 above the stock, less than an approach distance of 1.
        assert!((depth_into_stock(10.5, Some(10.0), 7.0) - 3.0).abs() < 1e-12);
        // Already in the stock: from where the tool is.
        assert!((depth_into_stock(9.0, Some(10.0), 7.0) - 2.0).abs() < 1e-12);
        // No stock over the point, or the stock below the goal.
        assert!(depth_into_stock(30.0, None, 7.0).abs() < 1e-12);
        assert!(depth_into_stock(30.0, Some(6.0), 7.0).abs() < 1e-12);
    }

    /// CodeRabbit on #972: where the lead-in meets the pass (the join) is a
    /// position the tool is sent to, so a face outside the selection that
    /// only the join cuts deeper than the tolerance (and the slack) is
    /// refused there, as the pass's own cut; beside swarf's face set the
    /// pass's own cut there is the allowance, as before (decision 0682).
    #[test]
    fn req_op_11_req_op_12_the_join_is_measured_against_the_faces_outside_the_selection() {
        let none: Vec<BTreeMap<usize, f64>> = vec![BTreeMap::new(), BTreeMap::new()];
        let own = BTreeMap::from([(3, 0.5)]);
        let slack = |_: usize| 0.02;
        assert_eq!(
            deepest_beside(&none, &own, Allowance::Tolerance, 0.01, slack),
            Some((2, 3, 0.5, 0.5))
        );
        assert_eq!(
            deepest_beside(&none, &own, Allowance::PassCut, 0.01, slack),
            None
        );
        // A move that cuts deeper than the join is the one named.
        let deeper = vec![BTreeMap::from([(3, 0.8)]), BTreeMap::new()];
        assert_eq!(
            deepest_beside(&deeper, &own, Allowance::Tolerance, 0.01, slack),
            Some((0, 3, 0.8, 0.5))
        );
        assert_eq!(
            deepest_beside(&deeper, &own, Allowance::PassCut, 0.01, slack),
            Some((0, 3, 0.8, 0.5))
        );
        // Within the tolerance and the slack: nothing.
        let shallow = BTreeMap::from([(3, 0.02)]);
        assert_eq!(
            deepest_beside(&none, &shallow, Allowance::Tolerance, 0.01, slack),
            None
        );
    }

    /// CodeRabbit on #866 (decision record 0682): a face sampled no finer
    /// than 400 points each way is coarser than the spacing asked for when
    /// it is large, so its slack comes from its own samples' spacing.
    #[test]
    fn req_op_12_a_large_face_next_to_the_face_set_takes_the_slack_of_its_own_samples() {
        let radius = 3.0;
        let asked = crate::swarf::guard_spacing(radius);
        let slack = |x1: f64| {
            let surface = BrepSurface::new(&wall_plane(8.0, 0.0, x1, 0.0, 10.0)).unwrap();
            let beside = Beside::new(&surface, radius, Allowance::Tolerance).expect("one face");
            assert_eq!(beside.near(&beside_all()), vec![0]);
            beside.slack(0)
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

    /// A box that holds everything.
    fn beside_all() -> openchuck_types::geometry::Aabb {
        let far = 1.0e6;
        openchuck_types::geometry::Aabb {
            min: openchuck_types::geometry::Point3::new(-far, -far, -far),
            max: openchuck_types::geometry::Point3::new(far, far, far),
        }
    }
}
