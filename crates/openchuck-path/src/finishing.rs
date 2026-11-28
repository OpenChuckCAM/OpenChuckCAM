// SPDX-License-Identifier: AGPL-3.0-or-later
//! Settings the finishing operations share for their layers and their free
//! ends (decision records 0344 and 0345; OP-04, OP-09, TL-05, NFR-12).
//!
//! **Stepdown** is the depth of each layer along the tool axis. It has the
//! same name as roughing's, so the tool's cutting data for the operation's
//! material fills it when the operation sets none (TL-05, decision record
//! 0230). Nothing else gives it a value: there is no guessed depth of cut
//! (NFR-12). With **Finishing layers** on it is required; with them off it
//! is used only to face a free end, and zero means none was given.
//!
//! **Face free ends** turns at an end face of the part with nothing beyond
//! it to stand the tool on. It is on for a new operation; an operation saved
//! before it is migrated to off (project schema 23), so it machines as
//! before.

use crate::notice::{Acknowledgement, Level, Notice, topic};
use std::collections::BTreeMap;

use openchuck_types::geometry::{Point3, UnitVec3, Vec3};
use openchuck_types::job::JobContext;
use openchuck_types::tool::profile::RevolvedProfile;
use openchuck_types::units::{Degrees, Millimetres};

use crate::error::{PathError, Result};
use crate::faces::FreeEnd;
use crate::params::{
    ParameterKind, ParameterSchema, ParameterSpec, ParameterValue, Parameters, Unit,
};
use crate::rotary::AxisPosition;
use crate::strategy::{CutPoint, Pass, StrategyInput};

/// Parameter names.
pub mod parameter {
    /// Turn at the part's free ends: on or off.
    pub const FACE_FREE_ENDS: &str = "face_free_ends";
    /// Cut in layers along the tool axis down to the target: on or off.
    pub const FINISHING_LAYERS: &str = "finishing_layers";
    /// The depth of each layer along the tool axis, mm; the name roughing
    /// uses, so the tool's cutting data fills it.
    pub use crate::roughing::parameter::STEPDOWN;
    /// What the final pass alone takes, mm along the tool axis, with
    /// Finishing layers on: the layers stop this far above the target.
    pub const FINISH_ALLOWANCE: &str = "finish_allowance";
    /// Carry the finish on past a part's end that comes to a point, down to
    /// the rotary axis: on or off (decision record 0695, Q2).
    pub const FINISH_TO_A_POINT: &str = "finish_to_a_point";
    /// Which way the arcs that face an end not reaching round the rotary
    /// axis are cut: `meander`, `climb` or `conventional`, the names and
    /// meanings of face-flow finishing's (decision record 0700, the owner's
    /// answer J1; decision 0213). Whole turns keep the strategy's own way.
    pub const FREE_END_DIRECTION: &str = "free_end_direction";
}

/// The Finish to a point setting (decision record 0695, Q2 (a) A): off in the
/// schema, so an operation saved without it keeps its program; a new
/// operation gets it on (decision record 0545 D3's pattern).
pub(crate) fn point_spec() -> ParameterSpec {
    ParameterSpec {
        name: parameter::FINISH_TO_A_POINT.to_owned(),
        label: "Finish to a point".to_owned(),
        description: "Where the part ends in a point inside the stock, carry the finish on past \
                      the end, the tool following the part down until its centre is on the \
                      rotary axis, so the point is cut instead of left as a nub. This cuts the \
                      part free of the stock past the point."
            .to_owned(),
        kind: ParameterKind::Flag { default: false },
    }
}

/// The Finish allowance when the operation sets none, mm (decision record
/// 0345; the owner's example).
pub const DEFAULT_FINISH_ALLOWANCE: f64 = 0.1;

/// The Stepdown when it is required: a positive number with no default.
fn required_stepdown() -> ParameterKind {
    ParameterKind::Number {
        unit: Unit::Millimetres,
        min: Some(0.0),
        min_exclusive: true,
        max: None,
        default: None,
    }
}

/// The settings for a finishing strategy's schema: Face free ends when
/// `free_ends`, Finishing layers when `layers`, and Stepdown when either.
pub(crate) fn specs(free_ends: bool, layers: bool) -> Vec<ParameterSpec> {
    let mut out = Vec::new();
    if free_ends {
        out.push(ParameterSpec {
            name: parameter::FACE_FREE_ENDS.to_owned(),
            label: "Face free ends".to_owned(),
            description: "Turn at each end face of the part that has nothing beyond it, \
                          stepping down beside the face by the Stepdown, so the face keeps \
                          only the stock to leave."
                .to_owned(),
            kind: ParameterKind::Flag { default: true },
        });
        out.push(ParameterSpec {
            name: parameter::FREE_END_DIRECTION.to_owned(),
            label: "Cutting direction".to_owned(),
            description: "Which way the tool cuts the arcs that face an end that does not \
                          reach round the rotary axis: meander alternates them; climb and \
                          conventional cut every arc the same way relative to the end face \
                          and the spindle. Whole turns keep the strategy's own direction."
                .to_owned(),
            kind: ParameterKind::Choice {
                options: [
                    crate::face_flow::parameter::MEANDER,
                    crate::face_flow::parameter::CLIMB,
                    crate::face_flow::parameter::CONVENTIONAL,
                ]
                .iter()
                .map(|o| (*o).to_owned())
                .collect(),
                default: crate::face_flow::parameter::CLIMB.to_owned(),
            },
        });
    }
    if layers {
        out.push(ParameterSpec {
            name: parameter::FINISH_ALLOWANCE.to_owned(),
            label: "Finish allowance".to_owned(),
            description: "With finishing layers on, the layers stop this far above the \
                          finished surface and the final pass takes only this much. 0 makes \
                          the last layer the final pass."
                .to_owned(),
            kind: ParameterKind::Number {
                unit: Unit::Millimetres,
                min: Some(0.0),
                min_exclusive: false,
                max: None,
                default: Some(DEFAULT_FINISH_ALLOWANCE),
            },
        });
        out.push(ParameterSpec {
            name: parameter::FINISHING_LAYERS.to_owned(),
            label: "Finishing layers".to_owned(),
            description: "Cut in layers the Stepdown apart along the tool axis, the last on \
                          the finished surface with its stock to leave."
                .to_owned(),
            kind: ParameterKind::Flag { default: false },
        });
    }
    if free_ends || layers {
        out.push(ParameterSpec {
            name: parameter::STEPDOWN.to_owned(),
            label: "Stepdown".to_owned(),
            description: "The depth of each layer along the tool axis. Taken from the tool's \
                          cutting data when not set here. Required with finishing layers on; \
                          otherwise used only to face a free end."
                .to_owned(),
            kind: ParameterKind::Number {
                unit: Unit::Millimetres,
                min: Some(0.0),
                min_exclusive: false,
                max: None,
                default: Some(0.0),
            },
        });
    }
    out
}

/// `schema` as it applies to `values`: with Finishing layers on, the
/// Stepdown is required (decision record 0344, C1).
pub(crate) fn schema_for(
    mut schema: ParameterSchema,
    values: &BTreeMap<String, ParameterValue>,
) -> ParameterSchema {
    let layers = matches!(
        values.get(parameter::FINISHING_LAYERS),
        Some(ParameterValue::Flag(true))
    );
    if layers {
        if let Some(spec) = schema
            .parameters
            .iter_mut()
            .find(|p| p.name == parameter::STEPDOWN)
        {
            spec.kind = required_stepdown();
        }
    } else {
        // The Finish allowance shows only with layers on.
        schema
            .parameters
            .retain(|p| p.name != parameter::FINISH_ALLOWANCE);
    }
    schema
}

/// The angles round the axis the stock is read at for a free end.
const AROUND: usize = 72;

/// How many angles a turn is sampled at, at least, to find where an end that
/// does not reach round the rotary axis lies (decision record 0700).
const SAMPLES_A_TURN: usize = 360;

/// The most points the layers beside the free ends may have, the same limit
/// as the passes over the part (CodeRabbit on #968).
const MOST_LAYER_POINTS: f64 = 5.0e6;

/// What facing the free ends gives an operation: the turns, its notes and
/// its warnings.
#[derive(Debug, Default)]
pub(crate) struct FreeEnds {
    pub(crate) passes: Vec<Pass>,
    pub(crate) notes: Vec<Notice>,
    pub(crate) warnings: Vec<Notice>,
    /// The X of each end face faced down to the rotary axis, whose warnings
    /// say what that cuts free.
    pub(crate) faced: Vec<f64>,
}

/// The turns at the part's free ends `ends` (decision record 0344): with
/// Face free ends on and a Stepdown, the tool (`grown` by the stock to leave)
/// goes down beside each free end face in layers of the Stepdown to the
/// rotary axis, one whole turn of A at constant X each, starting at A `start`
/// turning by `sign`, no more than `step` degrees between points, from the
/// stock earlier operations left or, for the first operation of a setup, the
/// setup's own stock, when the note says the real blank must match it
/// (OI-1394). Without a Stepdown, or on a stock of no known size, the face is
/// not faced and a warning says why (C1, C2). A face deeper than the flutes
/// is not faced either, and a warning says why and what to do (C3; decision
/// record 0531: it refused the operation before, OI-1396). Where
/// the stock carries on past the ring the tool cuts, a warning says the
/// stock there can be cut free (the owner's choice on OI-1390).
pub(crate) fn free_ends(
    input: &StrategyInput<'_>,
    grown: &RevolvedProfile,
    (ends, unfaced, loose): (
        &[FreeEnd],
        &[crate::faces::UnfacedEnd],
        &[crate::faces::LooseEnd],
    ),
    parameters: &Parameters,
    (start, sign, step): (f64, f64, f64),
) -> Result<FreeEnds> {
    let mut out = FreeEnds::default();
    // An end whose tab does not meet it is cut free of the tab by the turn
    // at its face, whether Face free ends is on or not (decision record
    // 0700, case (c)): decision 0344's warnings say so.
    if !parameters.flag(parameter::FACE_FREE_ENDS).unwrap_or(false) {
        loose_warnings(input, loose, &[], &mut out.warnings);
        return Ok(out);
    }
    let stock = input.stock;
    // Only an end the stock carries on past has anything to face: the end
    // of the stock itself is not a free end.
    let extent = stock.x_extent();
    let tolerance = 1e-3;
    // An end Face free ends cannot face is said, with what can face it
    // (decision record 0700, the owner's answer A to Q3 on #834): Facing,
    // with Shape End where it is the end opposite the setup's sole held
    // end.
    for e in unfaced.iter().filter(|e| match extent {
        Some((_, hi)) if e.far => hi > e.face + tolerance,
        Some((lo, _)) => lo < e.face - tolerance,
        None => true,
    }) {
        let opposite = !input.keep_out.is_empty()
            && input
                .keep_out
                .iter()
                .all(|&(a, b)| if e.far { b < e.face } else { a > e.face });
        out.warnings.push(Notice {
            level: Level::Warning,
            topic: Some(topic::FREE_ENDS.to_owned()),
            segment: None,
            text: format!(
                "the end face at X {:.3} has nothing beyond it but is not faced: it is not square \
                 to the rotary axis, so the tool cannot turn beside it, and it keeps what earlier \
                 operations left. Face it with a Facing operation{}, and check the path before \
                 you run it",
                e.face,
                if opposite { ", Shape End" } else { "" }
            ),
            setting: Some(parameter::FACE_FREE_ENDS.to_owned()),
            acknowledgement: None,
        });
    }
    // An end the stock does not carry on past has nothing to face: said, in
    // the user's words, so the end is not left in silence (issue #471, the
    // owner's note N2).
    for e in ends.iter().filter(|e| match extent {
        Some((_, hi)) if e.far => hi <= e.face + tolerance,
        Some((lo, _)) => lo >= e.face - tolerance,
        None => false,
    }) {
        // Information: there is nothing to face (decision 0536 step 2, the
        // owner's answer to question Q-B; P49).
        out.notes.push(
            Notice::information(format!(
                "the stock ends at the part's end at X {:.3}: there is no stock past this end \
                 to face. If the real blank is longer, set the stock's length to match it",
                e.face
            ))
            .about(topic::FREE_ENDS),
        );
    }
    let ends: Vec<&FreeEnd> = ends
        .iter()
        .filter(|e| match extent {
            Some((_, hi)) if e.far => hi > e.face + tolerance,
            Some((lo, _)) => lo < e.face - tolerance,
            None => true,
        })
        .collect();
    let stepdown = parameters.number(parameter::STEPDOWN).unwrap_or(0.0);
    let radius = grown.cutting_radius();
    let width = 2.0 * input.tool.cutting_radius();
    // The rings cut down to the axis, as (axis X, face X, far, X span), for
    // the check that the part stays held on its own side (issue #477).
    let mut rings: Vec<(f64, f64, bool, f64, f64, Option<f64>)> = Vec::new();
    for end in ends {
        let face = end.face;
        // Zero (none given) or NaN: no Stepdown.
        if stepdown.is_nan() || stepdown <= 0.0 {
            out.warnings.push(Notice {
                level: Level::Warning,
                topic: Some(topic::FREE_ENDS.to_owned()),
                segment: None,
                text: format!(
                    "the end face at X {face:.3} has nothing beyond it and is not faced: it \
                     keeps what earlier operations left. Set Stepdown to the depth of each \
                     layer, or add cutting data for this tool and material in the tool \
                     library and then choose Update from library in the operation's tool \
                     section: the project keeps its own copy of the tool, so a change in the \
                     library reaches it only then."
                ),
                setting: Some(parameter::STEPDOWN.to_owned()),
                acknowledgement: None,
            });
            continue;
        }
        // With no earlier operation in the setup its own stock is what is on
        // the machine (OI-1394); only a stock of no known size is skipped.
        if stock.max_radius().is_none() {
            out.warnings.push(Notice {
                level: Level::Warning,
                topic: Some(topic::FREE_ENDS.to_owned()),
                segment: None,
                text: format!(
                    "the end face at X {face:.3} has nothing beyond it and is not faced: the \
                     setup's stock has no known size, so the tool would cut there blind. \
                     Define the stock, or turn off Face free ends."
                ),
                setting: Some(parameter::FACE_FREE_ENDS.to_owned()),
                acknowledgement: None,
            });
            continue;
        }
        // The highest material the turn meets: the stock read from outside
        // along the tool axis at each angle, across the ring.
        let reach = stock.max_radius().unwrap_or(0.0) + 1.0;
        let top = (0..AROUND)
            .flat_map(|k| {
                let a = Degrees(360.0 * k as f64 / AROUND as f64);
                [-0.9, 0.0, 0.9].map(move |u| (a, end.x + u * radius))
            })
            .filter_map(|(a, x)| {
                let at = |h: f64| {
                    input.rotary.to_model(
                        AxisPosition {
                            along: x,
                            across: 0.0,
                            height: h,
                        },
                        a,
                    )
                };
                let down = UnitVec3::new(at(0.0) - at(1.0)).ok()?;
                let d = stock.ray_entry(at(reach), down, reach)?;
                Some(reach - d)
            })
            .fold(f64::NEG_INFINITY, f64::max);
        if !top.is_finite() || top <= 0.0 {
            continue;
        }
        // Not faced, with a warning at the switch the user acts on (decision
        // record 0531, the owner's decision of 6 Nov 2026; it was refused
        // before, OI-1396): the rest of the path is made, and no turn is
        // added whose depth is past the flutes.
        // The depth the flutes must reach, by the shared measure over the
        // tool's width at every half degree and between (decision record
        // 0509): a box corner between the readings above is not missed. The
        // layers keep the reading above, so the path is unchanged.
        let (deepest, exact) = crate::flutes::deepest_on_axis(input, end.x, input.tool.top());
        if top.max(deepest) > input.tool.top() + 1e-6 {
            // A bound, not a measure, is said to be one (CodeRabbit on #534).
            let (up_to, bound) = if deepest > top && !exact {
                (
                    "up to ",
                    " (a bound, from how far the stock reaches from the rotary axis: that \
                     stock is not measured exactly across the tool)",
                )
            } else {
                ("", "")
            };
            out.warnings.push(Notice {
                level: Level::Warning,
                topic: Some(topic::FLUTE_LENGTH.to_owned()),
                segment: None,
                text: format!(
                    "the end face at X {face:.3} has nothing beyond it and is not faced: the \
                     stock beside it is {up_to}{:.3} mm deep{bound}, more than the tool's \
                     {:.3} mm of flute, so no turn is cut there and the end keeps what earlier \
                     operations left; {}. Use a tool with longer flutes, or face it with a \
                     Facing operation, or turn off Face free ends for this operation. Check \
                     the path before you run it",
                    top.max(deepest),
                    input.tool.top(),
                    crate::flutes::stock_words(stock),
                ),
                setting: Some(parameter::FACE_FREE_ENDS.to_owned()),
                acknowledgement: None,
            });
            continue;
        }
        // A Stepdown so small that the layers would be too many points is
        // refused before any is made (CodeRabbit on #968), as the passes
        // over the part are.
        // A turn's points in each layer; arcs beside an end that does not
        // reach round the axis also walk every sampled angle in each layer
        // and can add an arc's ends at as many places, so each of their
        // layers counts the samples too (CodeRabbit's re-review of #968).
        let turn = (360.0 / step).ceil();
        let per_layer = if end.round {
            turn
        } else {
            turn + turn.max(SAMPLES_A_TURN as f64)
        };
        let made = (top / stepdown).ceil() * per_layer;
        if !made.is_finite() || made > MOST_LAYER_POINTS {
            return Err(PathError::parameter(
                parameter::STEPDOWN,
                format!(
                    "a Stepdown of {stepdown} mm over the {top:.3} mm beside the end face at X \
                     {face:.3} is too many points ({made:.0} at {step} degrees); increase the \
                     Stepdown"
                ),
            ));
        }
        let side = input.tool.cutting_radius();
        // How deep the end is faced: to the axis for an end round it; for
        // one to one side of it, to its lowest radius (decision 0700).
        let depth = if end.round {
            // Layers at whole Stepdowns above the axis, the first the highest
            // below the material's top, the last on the axis.
            let layers = ((top / stepdown).ceil() as usize).max(1);
            let points = ((360.0 / step).ceil() as usize).max(4);
            for m in (0..layers).rev() {
                let h = m as f64 * stepdown;
                out.passes
                    .push(turn_pass(input, end, h, (start, sign), points, side));
            }
            // How the end is faced is information; that it is faced from the
            // setup's own stock, which the real blank must match, is a
            // warning of its own (decision 0536 step 2, the owner's answer to
            // question Q-C; P50).
            out.notes.push(
                Notice::information(format!(
                    "the end face at X {face:.3} has nothing beyond it: the tool goes down \
                     beside it in {} of {stepdown} mm to the rotary axis, turning once round \
                     the axis in each",
                    crate::counted(layers, "layer", "layers")
                ))
                .about(topic::FREE_ENDS),
            );
            None
        } else {
            let Some(arcs) = arcs(
                input,
                grown,
                end,
                top,
                stepdown,
                (start, sign, step),
                parameters,
            )?
            else {
                // Not found again where the face search found it: said, not
                // skipped (CodeRabbit on #968).
                out.warnings.push(Notice {
                    level: Level::Warning,
                    topic: Some(topic::FREE_ENDS.to_owned()),
                    segment: None,
                    text: format!(
                        "the end face at X {face:.3} has nothing beyond it but is not faced: \
                         the tool finds no part beside it to face down to, and it keeps what \
                         earlier operations left. Face it with a Facing operation, and check \
                         the path before you run it"
                    ),
                    setting: Some(parameter::FACE_FREE_ENDS.to_owned()),
                    acknowledgement: None,
                });
                continue;
            };
            out.passes.extend(arcs.passes);
            out.notes.push(
                Notice::information(format!(
                    "the end face at X {face:.3} has nothing beyond it and does not reach round \
                     the rotary axis (its nearest point is {:.3} mm from it): the tool goes down \
                     beside it in {} of {stepdown} mm to {:.3} mm from the axis, in arcs over \
                     the angles where the face is",
                    arcs.nearest,
                    crate::counted(arcs.layers, "layer", "layers"),
                    arcs.last
                ))
                .about(topic::FREE_ENDS),
            );
            Some(arcs.last)
        };
        let (from, to) = (end.x - radius, end.x + radius);
        if !stock.left_by_earlier_operations() {
            out.warnings.push(
                Notice::warning(format!(
                    "the end face at X {face:.3} is faced from {top:.3} mm, the top of the setup's \
                 own stock with no earlier operation in this setup: the real blank must match it"
                ))
                .about(topic::FREE_ENDS),
            );
        }
        rings.push((end.x, face, end.far, from.min(to), from.max(to), depth));
        out.faced.push(face);
        // Stock past the ring that no held region past it grips: a cheap test
        // of whether there is any, not of whether it is joined to the rest
        // (OI-1393).
        let gripped = input
            .keep_out
            .iter()
            .any(|&(a, b)| if end.far { a > end.x } else { b < end.x });
        if !gripped && let Some((lo, hi)) = extent {
            let beyond = if end.far {
                (end.x + radius + 0.1, hi - 1e-3)
            } else {
                (lo + 1e-3, end.x - radius - 0.1)
            };
            let found = beyond.1 > beyond.0
                && (0..=4).any(|i| {
                    let x = beyond.0 + (beyond.1 - beyond.0) * i as f64 / 4.0;
                    (0..16).any(|k| {
                        let t = std::f64::consts::TAU * k as f64 / 16.0;
                        [0.25, 0.5, 0.75].into_iter().any(|f| {
                            let r = f * top;
                            stock.contains(Point3::ORIGIN + Vec3::new(x, r * t.cos(), r * t.sin()))
                        })
                    })
                });
            if found {
                out.warnings.push(Notice {
                    level: Level::Warning,
                    topic: Some(topic::PART_CUT_FREE.to_owned()),
                    segment: None,
                    text: format!(
                        "the end face at X {face:.3} is faced {}: the tool cuts {} {width:.3} mm \
                         wide from X {:.3} to {:.3} {}, and the stock past {} is not held by \
                         anything in this setup, so it can be cut free there and thrown while \
                         the rotary axis turns. Before running this program, check the stock and \
                         the workholding on the machine",
                        faced_words(depth),
                        cut_words(depth).0,
                        from.min(to),
                        from.max(to),
                        cut_words(depth).1,
                        cut_words(depth).2,
                    ),
                    setting: Some(parameter::FACE_FREE_ENDS.to_owned()),
                    acknowledgement: None,
                });
            }
        }
    }
    // The part's own side of each ring (issue #477, the owner's option A):
    // the ring down to the axis parts the part from whatever is past it, so
    // the part stays held only if a held region lies on its side, with no
    // other ring down to the axis between them. A tab joins the part to the
    // stock past it, but an end beside a tab is never a free end (decision
    // record 0344 item 4), so the held regions are what count here.
    for (i, &(x, face, far, from, to, depth)) in rings.iter().enumerate() {
        // Another cut between the ring and a held region parts them too: a
        // ring, or the turn beside an end whose tab misses it (CodeRabbit
        // on #968).
        let between = |held: f64| {
            rings
                .iter()
                .enumerate()
                .any(|(j, &(other, ..))| j != i && (other - x) * (held - other) > 0.0)
                || loose.iter().any(|l| (l.face - x) * (held - l.face) > 0.0)
        };
        let held = input.keep_out.iter().any(|&(a, b)| {
            if far {
                b < x && !between(b)
            } else {
                a > x && !between(a)
            }
        });
        if !held {
            out.warnings.push(Notice {
                level: Level::Warning,
                topic: Some(topic::PART_CUT_FREE.to_owned()),
                segment: None,
                text: format!(
                    "the end face at X {face:.3} is faced {}: the tool cuts {} {width:.3} mm wide \
                     from X {from:.3} to {to:.3} {}, and nothing in this setup holds the part on \
                     the other side of {} (no held region at its other end, and no tab joining \
                     it to one), so the part can be cut free there and thrown while the rotary \
                     axis turns. Before running this program, check the stock and the \
                     workholding on the machine",
                    faced_words(depth),
                    cut_words(depth).0,
                    cut_words(depth).1,
                    cut_words(depth).2,
                ),
                setting: Some(parameter::FACE_FREE_ENDS.to_owned()),
                // The part cut free can be thrown into the tool: a possible
                // collision, to acknowledge (decision record 0538, the
                // owner's answer to question 10).
                acknowledgement: Some(Acknowledgement::PossibleCollision),
            });
        }
    }
    let cuts: Vec<f64> = rings.iter().map(|r| r.0).collect();
    loose_warnings(input, loose, &cuts, &mut out.warnings);
    Ok(out)
}

/// Decision 0344's two Part cut free warnings for the ends beside a tab that
/// does not meet them (decision record 0700, case (c)): the stock past the
/// end, unless a held region past it grips it, and the part itself, unless
/// a held region on its side holds it with no other cut between them (the
/// `rings` at their X, or another such end's turn; CodeRabbit on #968) (a
/// possible collision to acknowledge, decision 0538).
fn loose_warnings(
    input: &StrategyInput<'_>,
    loose: &[crate::faces::LooseEnd],
    rings: &[f64],
    warnings: &mut Vec<Notice>,
) {
    let extent = input.stock.x_extent();
    for (i, end) in loose.iter().enumerate() {
        let (from, to) = end.span;
        let face = end.face;
        let past = match extent {
            Some((_, hi)) if end.far => hi > to + 0.1,
            Some((lo, _)) => lo < from - 0.1,
            None => false,
        };
        let gripped = input
            .keep_out
            .iter()
            .any(|&(a, b)| if end.far { a > face } else { b < face });
        if past && !gripped {
            warnings.push(Notice {
                level: Level::Warning,
                topic: Some(topic::PART_CUT_FREE.to_owned()),
                segment: None,
                text: format!(
                    "the end face at X {face:.3} has a tab past it that does not meet the face: \
                     the turn beside the face cuts down to the tab from X {from:.3} to {to:.3}, \
                     and the stock past it is not held by anything in this setup, so it can be \
                     cut free there and thrown while the rotary axis turns. Before running this \
                     program, check the stock and the workholding on the machine"
                ),
                setting: None,
                acknowledgement: None,
            });
        }
        let between = |held: f64| {
            rings
                .iter()
                .copied()
                .chain(
                    loose
                        .iter()
                        .enumerate()
                        .filter(|&(j, _)| j != i)
                        .map(|(_, l)| l.face),
                )
                .any(|cut| (cut - face) * (held - cut) > 0.0)
        };
        let held = input.keep_out.iter().any(|&(a, b)| {
            if end.far {
                b < face && !between(b)
            } else {
                a > face && !between(a)
            }
        });
        if !held {
            warnings.push(Notice {
                level: Level::Warning,
                topic: Some(topic::PART_CUT_FREE.to_owned()),
                segment: None,
                text: format!(
                    "the end face at X {face:.3} has a tab past it that does not meet the face, \
                     so the turn beside the face cuts the part free of the tab from X {from:.3} \
                     to {to:.3}, and nothing in this setup holds the part on its other side (no \
                     held region at its other end, and no tab joining it to one), so the part \
                     can be cut free there and thrown while the rotary axis turns. Before \
                     running this program, check the stock and the workholding on the machine"
                ),
                setting: None,
                // The part cut free can be thrown into the tool: a possible
                // collision, to acknowledge (decision record 0538).
                acknowledgement: Some(Acknowledgement::PossibleCollision),
            });
        }
    }
}

/// How deep an end is faced, in the Part cut free warnings: to the axis
/// (`None`), or in arcs to a depth above it (decision record 0700).
fn faced_words(depth: Option<f64>) -> String {
    match depth {
        None => "down to the rotary axis".to_owned(),
        Some(h) => format!("in arcs down to {h:.3} mm from the rotary axis"),
    }
}

/// What the tool cuts, how deep, and what to call it, in the Part cut free
/// warnings.
fn cut_words(depth: Option<f64>) -> (&'static str, String, &'static str) {
    match depth {
        None => ("a ring", "down to the axis".to_owned(), "the ring"),
        Some(h) => (
            "beside the whole face, in arcs",
            format!("down to {h:.3} mm from the axis"),
            "the arcs",
        ),
    }
}

/// One whole turn of A at the free end `end`'s X with the tip `h` along the
/// tool axis from the rotary axis, from A `start` turning by `sign`, in
/// `points` steps (decision record 0344).
fn turn_pass(
    input: &StrategyInput<'_>,
    end: &FreeEnd,
    h: f64,
    (start, sign): (f64, f64),
    points: usize,
    side: f64,
) -> Pass {
    let angles: Vec<f64> = (0..=points)
        .map(|i| start + sign * 360.0 * i as f64 / points as f64)
        .collect();
    arc_pass(input, end, h, &angles, side)
}

/// A pass at the free end `end`'s X, the tip `h` from the rotary axis, at
/// the angles `angles` in order.
fn arc_pass(input: &StrategyInput<'_>, end: &FreeEnd, h: f64, angles: &[f64], side: f64) -> Pass {
    let toward = if end.far { -1.0 } else { 1.0 };
    let points = angles
        .iter()
        .map(|&a| {
            let a = Degrees(a);
            let place = |along: f64| {
                input.rotary.to_model(
                    AxisPosition {
                        along,
                        across: 0.0,
                        height: h,
                    },
                    a,
                )
            };
            CutPoint {
                tip: place(end.x),
                a,
                contact: place(end.x + toward * side),
            }
        })
        .collect();
    Pass {
        helical_pass_start: false,
        low_link: false,
        points,
        stock: None,
        rapids: Vec::new(),
        plunge_fed: Vec::new(),
    }
}

/// A probe a fiftieth of a millimetre wide and as long as `profile`: where
/// the part is along a line of A itself, not beside it (decision record
/// 0700).
pub(crate) fn thin_probe(profile: &RevolvedProfile) -> Result<RevolvedProfile> {
    Ok(openchuck_types::tool::Tool {
        shape: openchuck_types::tool::ToolShape::FlatEnd,
        diameter: Millimetres(0.02),
        flute_length: Millimetres(profile.top().max(1.0)),
        overall_length: Millimetres(profile.top().max(1.0) + 1.0),
        shank_diameter: Millimetres(0.02),
        flute_count: 2,
        material: None,
        centre_cutting: None,
        stick_out: None,
    }
    .profile()?)
}

/// The arcs that face an end not reaching round the rotary axis, with what
/// the note says of them (decision record 0700).
struct Arcs {
    passes: Vec<Pass>,
    /// How many layers.
    layers: usize,
    /// The tip's distance from the axis in the last layer, mm.
    last: f64,
    /// The face's nearest point to the axis, mm, as the samples found it.
    nearest: f64,
}

/// Where the surface is solid along the line through the rotary axis at X
/// `x` and A `a`: the heights along the tool axis, from `reach` on the side
/// of `a` down to `reach` on the other side, as (low, high) intervals,
/// highest first. A ray down the line meets the surface where it goes in
/// and where it comes out, so a hole or a gap between two lugs is kept
/// (decision record 0700; CodeRabbit on #968), where a drop from each side
/// gives only the outermost material.
/// How far from the rotary axis anything in `bounds` can be: the radial
/// extent of the box across the axis, not its largest Y or Z, so a ray
/// started that far out along any line of A starts outside the part
/// (CodeRabbit on #968).
pub(crate) fn radial_reach(bounds: openchuck_types::geometry::Aabb) -> f64 {
    let y = bounds.min.y.abs().max(bounds.max.y.abs());
    let z = bounds.min.z.abs().max(bounds.max.z.abs());
    y.hypot(z)
}

pub(crate) fn occupied(input: &StrategyInput<'_>, x: f64, a: f64, reach: f64) -> Vec<(f64, f64)> {
    let line = input
        .rotary
        .tool_line(Millimetres(x), Degrees(a), Millimetres(0.0));
    let Ok(down) = UnitVec3::new(line.axis.get() * -1.0) else {
        return Vec::new();
    };
    let mut heights = Vec::new();
    let mut from = reach;
    // A part has few walls along one line; the bound only stops a broken
    // surface from looping (NFR-07).
    while heights.len() < 256 && from > -reach {
        let Some(hit) = input.surface.cast_ray(line.at(from), down, from + reach) else {
            break;
        };
        let h = from - hit.distance;
        heights.push(h);
        from = h - 1.0e-7;
    }
    heights
        .chunks_exact(2)
        .filter_map(|p| match p {
            &[high, low] => Some((low, high)),
            _ => None,
        })
        .collect()
}

/// The arcs that face the free end `end`, which does not reach round the
/// rotary axis (decision record 0700, the owner's improved B; issue #834).
///
/// At each of the angles a whole turn would sample, the tool (`grown` by the
/// stock to leave) is dropped on the part alone a tool radius inside the
/// face, from that angle and from the opposite one: the face is there when
/// the first finds the part on the tool's own side of the axis, and the
/// second gives how near the axis the part comes along that line. An angle
/// is cut in every layer from the material's `top` down to the first whole
/// Stepdown at or below where the tool's side reaches that nearest point,
/// so each layer's arc lies within the one above it and no layer goes down
/// into stock the layer above left. The layers stop at the end's nearest
/// point, not the axis. Each layer's angles make one or more arcs, widened
/// by a sample at each end; a layer whose arcs cover the whole turn is a
/// whole turn, cut the strategy's own way (`start`, `sign`). The arcs are cut
/// by the Cutting direction: climb puts the end face on the tool's right
/// looking along the feed from the spindle (M3, decision 0213),
/// conventional on its left, and meander alternates them, arc by arc.
/// `None` when no angle finds the face.
fn arcs(
    input: &StrategyInput<'_>,
    grown: &RevolvedProfile,
    end: &FreeEnd,
    top: f64,
    stepdown: f64,
    (start, sign, step): (f64, f64, f64),
    parameters: &Parameters,
) -> Result<Option<Arcs>> {
    let toward = if end.far { -1.0 } else { 1.0 };
    let side = input.tool.cutting_radius();
    let lift = input.tool.height_at(side).unwrap_or(0.0);
    let radius = grown.cutting_radius();
    // Sampled at a degree or finer, whatever the largest angle step, so an
    // end the face search found between two steps is found here too
    // (CodeRabbit on #968); the arcs themselves keep the step.
    let n = ((360.0 / step).ceil() as usize).max(SAMPLES_A_TURN);
    let da = 360.0 / n as f64;
    let inside = end.face + toward * radius;
    let bounds = input.surface.bounds();
    let reach = radial_reach(bounds) + 1.0;
    // A probe a fiftieth of a millimetre wide: how near the axis the part
    // comes along the tool's own line, not beside it (the tool's side
    // reaches a point beside the line from that point's own angle).
    let thin = thin_probe(grown)?;
    let drop = |a: f64| {
        let line = input
            .rotary
            .tool_line(Millimetres(inside), Degrees(a), Millimetres(0.0));
        input
            .surface
            .drop_on_part(&thin, &line)
            .map(|c| input.rotary.to_axis(c.tip, Degrees(a)).height)
    };
    // At each angle, where the face is on the line: how near the axis the
    // part comes along it, the inner edge of the material the line meets
    // first (0 where that material goes on across the axis). A hole, or a
    // lug on the other side, is not taken for material at the axis.
    let samples: Vec<Option<f64>> = (0..n)
        .map(|k| {
            let a = start + da * k as f64;
            let near = drop(a).filter(|&h| h > 0.0)?;
            let inner = occupied(input, inside, a, reach)
                .into_iter()
                .find(|&(lo, hi)| hi > 0.0 && lo < near)
                .map_or(0.0, |(lo, _)| lo);
            Some(inner.clamp(0.0, near))
        })
        .collect();
    let Some(nearest) = samples.iter().flatten().copied().reduce(f64::min) else {
        return Ok(None);
    };
    let layer_of = |b: f64| ((b - lift).max(0.0) / stepdown + 1e-9).floor() * stepdown;
    let last = layer_of(nearest);
    let first = ((top / stepdown).ceil() - 1.0).max(0.0) * stepdown;
    let first = first.max(last);
    let layers = ((first - last) / stepdown).round() as usize + 1;
    let direction = parameters
        .text(parameter::FREE_END_DIRECTION)
        .unwrap_or(crate::face_flow::parameter::CLIMB);
    // The A sense that is climb here: the face on the tool's right looking
    // along the feed from the spindle.
    let climb = {
        let place = |a: f64| {
            input.rotary.to_model(
                AxisPosition {
                    along: end.x,
                    across: 0.0,
                    height: 10.0,
                },
                Degrees(a),
            )
        };
        let feed = place(start + 1.0) - place(start);
        let right = feed.cross(input.rotary.tool_axis(Degrees(start + 1.0)).get());
        if right.x * toward > 0.0 { 1.0 } else { -1.0 }
    };
    let points = n;
    let mut passes = Vec::new();
    let mut count = 0_usize;
    for m in 0..layers {
        let h = first - m as f64 * stepdown;
        let cut: Vec<bool> = samples
            .iter()
            .map(|s| s.is_some_and(|b| layer_of(b) <= h + 1e-9))
            .collect();
        if cut.iter().all(|c| *c) {
            passes.push(turn_pass(input, end, h, (start, sign), points, side));
            continue;
        }
        // Runs of angles, round the circle from one not cut.
        let Some(off) = cut.iter().position(|c| !*c) else {
            continue;
        };
        let mut runs: Vec<(usize, usize)> = Vec::new();
        let mut open: Option<usize> = None;
        for j in 1..=n {
            let k = (off + j) % n;
            let here = cut.get(k).copied().unwrap_or(false);
            match (here, open) {
                (true, None) => open = Some(off + j),
                (false, Some(o)) => {
                    runs.push((o, off + j - 1));
                    open = None;
                }
                _ => {}
            }
        }
        for (lo, hi) in runs {
            let (a0, a1) = (
                start + da * (lo as f64 - 1.0),
                start + da * (hi as f64 + 1.0),
            );
            let steps = (((a1 - a0) / step).ceil() as usize).max(1);
            let mut angles: Vec<f64> = (0..=steps)
                .map(|i| a0 + (a1 - a0) * i as f64 / steps as f64)
                .collect();
            let sense = match direction {
                crate::face_flow::parameter::CONVENTIONAL => -climb,
                crate::face_flow::parameter::MEANDER if count % 2 == 1 => -climb,
                _ => climb,
            };
            if sense < 0.0 {
                angles.reverse();
            }
            passes.push(arc_pass(input, end, h, &angles, side));
            count += 1;
        }
    }
    Ok(Some(Arcs {
        passes,
        layers,
        last,
        nearest,
    }))
}

/// `passes`, the operation's final passes on the target, with the Finishing
/// layers before them when they are on (decision record 0345; OI-1391
/// option 1, OI-1392 option 3), and the note that says so.
///
/// The layers stand at the Finish allowance F plus whole Stepdowns S above
/// the target, along the tool axis: each a copy of the final passes lifted
/// by F + k S, top first, so the layers of neighbouring passes line up. A
/// layer is cut only where the stock (what earlier operations left, or the
/// setup's own stock) holds material above it there, read along the tool
/// axis at each point, so no layer cuts air. Where F is zero the lowest
/// layer is the final pass itself. The final passes come last, unchanged.
pub(crate) fn layers(
    input: &StrategyInput<'_>,
    passes: Vec<Pass>,
    parameters: &Parameters,
) -> Result<(Vec<Pass>, Option<Notice>)> {
    if !parameters
        .flag(parameter::FINISHING_LAYERS)
        .unwrap_or(false)
    {
        return Ok((passes, None));
    }
    let stepdown = parameters.number(parameter::STEPDOWN)?;
    let allowance = parameters
        .number(parameter::FINISH_ALLOWANCE)
        .unwrap_or(DEFAULT_FINISH_ALLOWANCE);
    if allowance >= stepdown {
        return Err(PathError::parameter(
            parameter::FINISH_ALLOWANCE,
            format!("must be less than the Stepdown, {stepdown} mm; it is {allowance} mm"),
        ));
    }
    let flutes = input.tool.top();
    if stepdown > flutes {
        return Err(PathError::parameter(
            parameter::STEPDOWN,
            format!(
                "{stepdown} mm is more than the tool's {flutes:.3} mm of flute; a layer that \
                 deep would cut with the shank"
            ),
        ));
    }
    let stock = input.stock;
    let reach = 2.0 * (stock.max_radius().unwrap_or(0.0) + 1.0);
    // How high the stock's material reaches above each point along the tool
    // axis; zero where there is none.
    let tops: Vec<Vec<f64>> = passes
        .iter()
        .map(|pass| {
            pass.points
                .iter()
                .map(|p| {
                    let axis = input.rotary.tool_axis(p.a).get();
                    let Ok(down) = UnitVec3::new(axis * -1.0) else {
                        return 0.0;
                    };
                    let length = reach + (p.tip - Point3::ORIGIN).length();
                    stock
                        .ray_entry(p.tip + axis * length, down, length)
                        .map_or(0.0, |d| (length - d).max(0.0))
                })
                .collect()
        })
        .collect();
    let highest = tops.iter().flatten().copied().fold(0.0_f64, f64::max);
    // The layers F + k S below the highest material, top first; with F zero
    // the k = 0 layer is the final pass.
    let first = usize::from(allowance <= 0.0);
    let mut heights: Vec<f64> = (first..)
        .map(|k| allowance + k as f64 * stepdown)
        .take_while(|h| *h < highest - LAYER_SLACK)
        .collect();
    heights.reverse();
    let mut out = Vec::new();
    for &h in &heights {
        for (pass, top) in passes.iter().zip(&tops) {
            let mut run: Vec<CutPoint> = Vec::new();
            for (p, t) in pass.points.iter().zip(top) {
                if *t > h + LAYER_SLACK {
                    let axis = input.rotary.tool_axis(p.a).get();
                    run.push(CutPoint {
                        tip: p.tip + axis * h,
                        a: p.a,
                        contact: p.contact + axis * h,
                    });
                } else if !run.is_empty() {
                    push_run(&mut out, std::mem::take(&mut run));
                }
            }
            push_run(&mut out, run);
        }
    }
    let note = (!heights.is_empty()).then(|| {
        let from = if stock.left_by_earlier_operations() {
            "the stock the earlier operations left, known to a simulation cell"
        } else {
            "the setup's own stock, with no earlier operation in this setup"
        };
        Notice::information(format!(
            "up to {} of {stepdown} mm above a finish allowance of \
             {allowance} mm, then the final pass on the surface; the layers start from {from}, \
             so the depth of each cut is not uniform and no engagement is promised",
            crate::counted(heights.len(), "layer", "layers")
        ))
        .about(topic::FINISHING_LAYERS)
    });
    out.extend(passes);
    Ok((out, note))
}

/// How far, mm, the material must reach past a layer for the layer to cut
/// there.
const LAYER_SLACK: f64 = 1e-6;

/// Adds a run of a layer as a pass when it has a move in it.
fn push_run(out: &mut Vec<Pass>, points: Vec<CutPoint>) {
    if points.len() >= 2 {
        out.push(Pass {
            helical_pass_start: false,
            low_link: false,
            points,
            stock: None,
            rapids: Vec::new(),
            plunge_fed: Vec::new(),
        });
    }
}

/// The note for a selected region that lies wholly on faces across the
/// rotary axis (issue #471, the owner's note N1): `strategy` cuts with the
/// tool pointing at the axis, so it reaches such a face only with the side
/// of the tool. "Wholly" is 99% of the region's area on faces whose normal
/// is within 10 degrees of the axis.
pub(crate) fn across_axis_warning(
    region: Option<&crate::region::RegionConstraint<'_>>,
    strategy: &str,
) -> Option<Notice> {
    let mesh = region?.region?.tessellation();
    let cos = 10.0_f64.to_radians().cos();
    let (mut across, mut total) = (0.0, 0.0);
    for t in &mesh.triangles {
        let corner = |k: usize| {
            t.get(k)
                .and_then(|&i| mesh.vertices.get(i as usize))
                .copied()
        };
        let (Some(a), Some(b), Some(c)) = (corner(0), corner(1), corner(2)) else {
            continue;
        };
        let n = (b - a).cross(c - a);
        let area = n.length();
        if !area.is_finite() || area <= 0.0 {
            continue;
        }
        total += area;
        if n.x.abs() >= cos * area {
            across += area;
        }
    }
    (total > 0.0 && across >= 0.99 * total).then(|| {
        Notice::warning(format!(
            "the selected faces lie across the rotary axis. {strategy} cuts with the tool \
             pointing at the axis, so it reaches such a face only with the side of the tool, in \
             one turn where the part lies on both sides of it. To face the end of the part or \
             the stock, use Facing with Shape End"
        ))
        .about(topic::FACES_ACROSS_THE_AXIS)
    })
}

/// The most whole turns that take the tool down to the axis past a point.
const MOST_TURNS: usize = 1000;

/// The most points in one of those turns: a tenth of a degree apart.
const MOST_POINTS: usize = 3600;

/// A free end of the part that comes to a point, past which Finish to a
/// point carries the finish on (decision record 0695, Q2 (a) A).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PointEnd {
    /// The X of the end itself.
    pub(crate) face: f64,
    /// The last X of the tool's axis where the drop still finds the part:
    /// the finish follows the part down to here.
    pub(crate) edge: f64,
    /// Whether the end looks along +X (the part's far end).
    pub(crate) far: bool,
}

/// Whether the stock carries on past the end face at `face`: only such an
/// end has anything past it to cut (decision record 0344).
fn stock_past(input: &StrategyInput<'_>, face: f64, far: bool) -> bool {
    let tolerance = 1e-3;
    match input.stock.x_extent() {
        Some((_, hi)) if far => hi > face + tolerance,
        Some((lo, _)) => lo < face - tolerance,
        None => true,
    }
}

/// The free ends `ends` that come to a point, with Finish to a point on
/// (decision record 0695, Q2): an end the stock carries on past, where the
/// tool (`grown` by the stock to leave) at the end's last drop touches the
/// part nearer the rotary axis than its own cutting radius at every angle,
/// so the part there is narrower than the tool. A wider end is an end face,
/// which Face free ends turns at (decision record 0344). An end under a part
/// of the surface with its own stock to leave (`zones`) is left out: going
/// down to the axis there would cut its allowance away (CodeRabbit on #956).
/// With a selected `region`, an end is kept only where the region allows the
/// tool at its last drop all round (OP-02; CodeRabbit on #956).
pub(crate) fn point_ends(
    input: &StrategyInput<'_>,
    grown: &RevolvedProfile,
    ends: &[FreeEnd],
    (region, zones): (
        Option<&crate::region::RegionConstraint<'_>>,
        &[crate::strategy::LeaveZone<'_>],
    ),
    parameters: &Parameters,
) -> Vec<PointEnd> {
    if !parameters
        .flag(parameter::FINISH_TO_A_POINT)
        .unwrap_or(false)
    {
        return Vec::new();
    }
    let radius = grown.cutting_radius();
    ends.iter()
        .filter(|e| stock_past(input, e.face, e.far))
        .filter(|e| {
            let Some(region) = region.filter(|r| !r.is_empty()) else {
                return true;
            };
            (0..AROUND).all(|k| {
                let a = Degrees(360.0 * k as f64 / AROUND as f64);
                let line = input
                    .rotary
                    .tool_line(Millimetres(e.edge), a, Millimetres(0.0));
                input
                    .surface
                    .drop_on_part(grown, &line)
                    .is_some_and(|c| region.allows(c.contact, c.tip, grown, &line))
            })
        })
        .filter(|e| {
            // No zone's surface under the tool at the end's last drop.
            !zones.iter().any(|z| {
                (0..AROUND).any(|k| {
                    let a = Degrees(360.0 * k as f64 / AROUND as f64);
                    let line = input
                        .rotary
                        .tool_line(Millimetres(e.edge), a, Millimetres(0.0));
                    z.surface.drop_tool(grown, &line).is_some()
                })
            })
        })
        .filter(|e| {
            (0..AROUND).all(|k| {
                let a = Degrees(360.0 * k as f64 / AROUND as f64);
                let line = input
                    .rotary
                    .tool_line(Millimetres(e.edge), a, Millimetres(0.0));
                input
                    .surface
                    .drop_on_part(grown, &line)
                    .is_some_and(|c| c.contact.y.hypot(c.contact.z) < radius)
            })
        })
        .map(|e| PointEnd {
            face: e.face,
            edge: e.edge,
            far: e.far,
        })
        .collect()
}

/// Whole turns of A at `x`, the tool's tip stepping down by `stepdown` from
/// `from` (mm along the tool axis from the rotary axis), the last with the
/// tool's end at its cutting radius on the axis, as the drop onto a point on
/// the axis puts it: a ball's centre on the axis (decision record 0695, Q2
/// (b) A). Starting at A `start`, turning by `sign`, no more than `step`
/// degrees between points, but never more than `MOST_POINTS` (3600, a
/// tenth of a degree apart) in a turn however small `step` is: at a radius
/// of at most the tool radius that is a chord under 0.003 mm, and on the
/// final turn the tool's centre is on the axis and does not move (issue
/// #959). Each turn its own pass, as Face free ends cuts them (decision
/// record 0344). With no Stepdown, one turn.
pub(crate) fn turns_to_the_axis(
    input: &StrategyInput<'_>,
    end: &PointEnd,
    from: f64,
    stepdown: f64,
    (start, sign, step): (f64, f64, f64),
    context: &JobContext<'_>,
) -> Result<Vec<Pass>> {
    let side = input.tool.cutting_radius();
    let last = -input.tool.height_at(side).unwrap_or(0.0);
    let toward = if end.far { -1.0 } else { 1.0 };
    // At most `MOST_POINTS` points a turn, whatever the largest angle step
    // (CodeRabbit on #956).
    let points = (360.0 / step).ceil();
    let points = if points.is_finite() && points >= 4.0 {
        (points as usize).min(MOST_POINTS)
    } else if points.is_finite() {
        4
    } else {
        MOST_POINTS
    };
    let mut heights = Vec::new();
    if stepdown.is_finite() && stepdown > 0.0 && from > last {
        // Whole Stepdowns above the last turn, at most `MOST_TURNS` of them:
        // a Stepdown too small for that is widened to fit (CodeRabbit on
        // #956), so the loop always ends.
        let steps = ((from - last) / stepdown).ceil();
        let (count, step) = if steps.is_finite() && steps <= MOST_TURNS as f64 {
            (steps as usize, stepdown)
        } else {
            (MOST_TURNS, (from - last) / MOST_TURNS as f64)
        };
        heights.extend(
            (1..count)
                .map(|k| from - k as f64 * step)
                .filter(|&h| h > last + 1e-9),
        );
    }
    heights.push(last);
    // Checked for cancellation before each turn (CodeRabbit on #956).
    let mut out = Vec::with_capacity(heights.len());
    for h in heights {
        context.check_cancelled()?;
        out.push(Pass {
            helical_pass_start: false,
            low_link: false,
            points: (0..=points)
                .map(|i| {
                    let a = Degrees(start + sign * 360.0 * i as f64 / points as f64);
                    let place = |along: f64| {
                        input.rotary.to_model(
                            AxisPosition {
                                along,
                                across: 0.0,
                                height: h,
                            },
                            a,
                        )
                    };
                    CutPoint {
                        tip: place(end.edge),
                        a,
                        contact: place(end.edge + toward * side),
                    }
                })
                .collect(),
            stock: None,
            rapids: Vec::new(),
            plunge_fed: Vec::new(),
        });
    }
    Ok(out)
}

/// The tool's tip at `end`'s last drop, mm along the tool axis from the
/// rotary axis: the highest round the axis, for the tool `grown` by `leave`.
pub(crate) fn top_at_edge(
    input: &StrategyInput<'_>,
    grown: &RevolvedProfile,
    leave: f64,
    end: &PointEnd,
) -> f64 {
    let top = (0..AROUND)
        .filter_map(|k| {
            let a = Degrees(360.0 * k as f64 / AROUND as f64);
            let line = input
                .rotary
                .tool_line(Millimetres(end.edge), a, Millimetres(0.0));
            input.surface.drop_tool(grown, &line)
        })
        .map(|c| c.offset + leave)
        .fold(f64::NEG_INFINITY, f64::max);
    if top.is_finite() { top } else { 0.0 }
}

/// What finishing to the points `ends` cuts free (decision record 0695, Q2
/// (b) A, with decision record 0344's warnings): the tool goes down to the
/// rotary axis past each point, so the stock past it is cut free unless a
/// held region past it grips it, and the part is cut free unless a held
/// region on its own side holds it. An end already in `faced` (Face free
/// ends faced it down to the axis) has those warnings already.
pub(crate) fn point_warnings(
    input: &StrategyInput<'_>,
    ends: &[PointEnd],
    faced: &[f64],
) -> Vec<Notice> {
    let radius = input.tool.cutting_radius();
    let width = 2.0 * radius;
    let mut out = Vec::new();
    for end in ends {
        if faced.iter().any(|&f| (f - end.face).abs() <= 1e-6) {
            continue;
        }
        let (from, to) = (end.edge - radius, end.edge + radius);
        let face = end.face;
        let gripped = input
            .keep_out
            .iter()
            .any(|&(a, b)| if end.far { a > end.edge } else { b < end.edge });
        if !gripped {
            out.push(Notice {
                level: Level::Warning,
                topic: Some(topic::PART_CUT_FREE.to_owned()),
                segment: None,
                text: format!(
                    "the point at X {face:.3} is finished down to the rotary axis: the tool cuts \
                     a ring {width:.3} mm wide from X {from:.3} to {to:.3} down to the axis, and \
                     the stock past the point is not held by anything in this setup, so it can \
                     be cut free there and thrown while the rotary axis turns. Before running \
                     this program, check the stock and the workholding on the machine"
                ),
                setting: Some(parameter::FINISH_TO_A_POINT.to_owned()),
                acknowledgement: None,
            });
        }
        // A held region holds the part only with no other ring down to the
        // axis between them: another point's, or a face Face free ends
        // faced (as `free_ends` checks; CodeRabbit on #956).
        let rings: Vec<f64> = ends
            .iter()
            .filter(|o| !std::ptr::eq(*o, end))
            .map(|o| o.edge)
            .chain(faced.iter().copied())
            .collect();
        let between = |held: f64| rings.iter().any(|&r| (r - end.edge) * (held - r) > 0.0);
        let held = input.keep_out.iter().any(|&(a, b)| {
            if end.far {
                b < end.edge && !between(b)
            } else {
                a > end.edge && !between(a)
            }
        });
        if !held {
            out.push(Notice {
                level: Level::Warning,
                topic: Some(topic::PART_CUT_FREE.to_owned()),
                segment: None,
                text: format!(
                    "the point at X {face:.3} is finished down to the rotary axis: the tool cuts \
                     a ring {width:.3} mm wide from X {from:.3} to {to:.3} down to the axis, and \
                     nothing in this setup holds the part on the other side of the ring (no held \
                     region at its other end, and no tab joining it to one), so the part can be \
                     cut free there and thrown while the rotary axis turns. Before running this \
                     program, check the stock and the workholding on the machine"
                ),
                setting: Some(parameter::FINISH_TO_A_POINT.to_owned()),
                // As decision record 0538 has it for a free end: a possible
                // collision, to acknowledge.
                acknowledgement: Some(Acknowledgement::PossibleCollision),
            });
        }
    }
    out
}
