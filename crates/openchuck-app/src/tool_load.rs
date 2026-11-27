// SPDX-License-Identifier: AGPL-3.0-or-later
//! The load on the tool, as the stock simulation measured it (SIM-16;
//! decision record 0698, TL-1).
//!
//! The simulation notes what each move removed ([`openchuck_sim::ToolLoad`]);
//! this module chooses the limit an operation is measured against and words
//! what was found:
//!
//! - the **limit** is the largest cut section (material removed per
//!   millimetre of travel, mm²) allowed before a warning: the operation's
//!   own Largest cut section where it sets one, else twice a full-width slot
//!   at the stepdown of the tool's cutting data for the stock material, else,
//!   with no cutting data, the tool's diameter squared (a stepdown of half
//!   the diameter). The tool library's own value per tool and material
//!   follows in TL-1b;
//! - a **warning** where stretches of the path pass it, with the figures and
//!   the place; radial drilling is measured but not warned, since a drill
//!   cuts on its end by design;
//! - an **information line** per operation with its heaviest cut and its
//!   removal rate, and one for the whole job.
//!
//! None of it says a cut is safe for the tool or the machine (NFR-12): a
//! warning says the cut is heavy and how to relieve it; no warning is not a
//! statement that the load is acceptable.

use openchuck_path::Notice;
use openchuck_project::id::ToolId;
use openchuck_project::{Operation, Project};
use openchuck_sim::LoadSummary;
use openchuck_types::ir::ToolpathGroup;

use crate::lead_in::topic;

/// The operation parameter that sets the operation's own largest cut
/// section, mm² (decision record 0698, Q3: an override in the
/// pattern of the cutting-data fields). The application reads it; it is not
/// a parameter of any strategy, so it is taken out before the parameters go
/// to the pathing library.
pub const LARGEST_CUT_SECTION: &str = "largest_cut_section";

/// Where an operation's tool-load limit comes from.
#[derive(Debug, Clone, PartialEq)]
pub enum LimitSource {
    /// The operation's own Largest cut section.
    Operation,
    /// Twice a full-width slot at the stepdown of the tool's cutting data
    /// for the stock material.
    TwiceTheSlot {
        /// The stepdown of the cutting data, mm.
        stepdown: f64,
        /// The stock material.
        material: String,
    },
    /// The tool has no cutting data for the stock material: the diameter
    /// squared (twice the slot at a stepdown of half the diameter).
    NoCuttingData,
}

/// The largest cut section an operation is measured against, and where it
/// comes from.
#[derive(Debug, Clone, PartialEq)]
pub struct Limit {
    /// The limit, mm².
    pub value: f64,
    /// Where it comes from.
    pub source: LimitSource,
}

impl Limit {
    /// Where the limit comes from, in words for a message or a hint.
    #[must_use]
    pub fn source_words(&self) -> String {
        match &self.source {
            LimitSource::Operation => "this operation's Largest cut section".to_owned(),
            LimitSource::TwiceTheSlot { stepdown, material } => format!(
                "twice a full-width slot at this tool's {stepdown:.2} mm stepdown in {material}"
            ),
            LimitSource::NoCuttingData => {
                "the tool's diameter squared, as it has no cutting data for this material"
                    .to_owned()
            }
        }
    }

    /// The short source the operation's form shows beside the value when
    /// the operation sets none.
    #[must_use]
    pub fn hint_source(&self) -> &'static str {
        match self.source {
            LimitSource::Operation => "this operation",
            LimitSource::TwiceTheSlot { .. } => "twice the slot at the tool's stepdown",
            LimitSource::NoCuttingData => "the tool's diameter squared",
        }
    }
}

/// The operation's own Largest cut section, when it sets a usable one.
#[must_use]
pub fn own_limit(operation: &Operation) -> Option<f64> {
    match operation.parameters.get(LARGEST_CUT_SECTION)? {
        openchuck_project::ParameterValue::Number(v) => Some(*v),
        #[allow(clippy::cast_precision_loss)]
        openchuck_project::ParameterValue::Integer(v) => Some(*v as f64),
        _ => None,
    }
    .filter(|v| v.is_finite() && *v > 0.0)
}

/// The limit `operation` is measured against, or `None` when it has no tool.
#[must_use]
pub fn limit(project: &Project, operation: &Operation) -> Option<Limit> {
    let snapshot = operation
        .tool
        .and_then(|id| project.tools().iter().find(|t| t.id == id))?;
    if let Some(value) = own_limit(operation) {
        return Some(Limit {
            value,
            source: LimitSource::Operation,
        });
    }
    let diameter = snapshot.tool.diameter.0;
    if let Some((material, _, data)) = crate::requirements::cutting_data(project, operation) {
        let stepdown = data.stepdown.0;
        return Some(Limit {
            value: 2.0 * stepdown * diameter,
            source: LimitSource::TwiceTheSlot {
                stepdown,
                material: material.to_owned(),
            },
        });
    }
    Some(Limit {
        value: diameter * diameter,
        source: LimitSource::NoCuttingData,
    })
}

/// Whether an operation of `strategy` is warned of a heavy cut. Radial
/// drilling is measured but not warned: a drill cuts on its end by design,
/// and its pecking manages that (decision record 0698, Q4).
#[must_use]
pub fn warns(strategy: &str) -> bool {
    strategy != openchuck_path::drill::NAME
}

/// Where move `index` of `group` ends, as a message says it.
fn place(group: &ToolpathGroup, index: usize) -> String {
    group.segments.get(index).map_or_else(String::new, |s| {
        format!(
            " at X {:.3}, A {:.1} (move {})",
            s.end.tip.x,
            s.end.a.0,
            index + 1
        )
    })
}

/// The warning for an operation whose stretches pass its limit, or `None`
/// when none does. Its text has no lead-in: the application names the
/// operation and its tool where it shows it (topic [`topic::TOOL_LOAD`]).
#[must_use]
pub fn warning(
    summary: &LoadSummary,
    limit: &Limit,
    group: &ToolpathGroup,
    flute_length: f64,
) -> Option<Notice> {
    if summary.over == 0 {
        return None;
    }
    let heaviest = summary.heaviest?;
    let stretches = openchuck_path::counted(summary.over, "stretch", "stretches");
    let text = format!(
        "{stretches} of the path have a cut section (the material removed per \
         millimetre of travel) above {:.2} mm², the limit from {}. The heaviest has a \
         cut section of {:.2} mm²{}, with \
         {:.2} mm of the {:.2} mm flute in the material. A cut like this can be far \
         heavier than the tool and the machine are meant to take. Rough the setup \
         first, use Finishing layers where the operation has them, take a smaller \
         Stepdown or Stepover, or set this operation's Largest cut section if you \
         judge the load acceptable, and check the cut in the simulation. {}",
        limit.value,
        limit.source_words(),
        heaviest.section,
        place(group, heaviest.last),
        heaviest.engaged,
        flute_length,
        openchuck_types::notice::VERIFY_GCODE_NOTICE,
    );
    Some(
        Notice::warning(text)
            .about(topic::TOOL_LOAD)
            .with_setting(LARGEST_CUT_SECTION),
    )
}

/// The information line of a simulated operation: its heaviest cut, its
/// removal rate and its limit (decision record 0698, Q2 and Q3).
#[must_use]
pub fn information(summary: &LoadSummary, limit: &Limit) -> Notice {
    let heaviest = summary.heaviest.map_or_else(
        || "removes no material over a stretch of one tool diameter".to_owned(),
        |h| {
            format!(
                "the heaviest cut section is {:.2} mm² (limit {:.2})",
                h.section, limit.value
            )
        },
    );
    let engaged = summary
        .longest_engaged
        .map(|(e, _)| format!(", with up to {e:.2} mm of flute in the material"))
        .unwrap_or_default();
    let rates = summary.average_rate().map_or_else(String::new, |average| {
        let highest = summary
            .fastest
            .and_then(|f| f.rate)
            .map(|highest| format!(", {highest:.0} mm³/min at most"))
            .unwrap_or_default();
        format!("; removal rate {average:.0} mm³/min on average over the cutting time{highest}")
    });
    Notice::information(format!("{heaviest}{engaged}{rates}")).about(topic::TOOL_LOAD)
}

/// The tool a rotary roughing operation added before `operation` would cut
/// with (decision record 0698, TL-3), or why there is none: the operation is
/// warned of its tool load only where its strategy warns, it is not rotary
/// roughing itself (whose remedy is a smaller Stepdown), it must be the
/// first enabled operation to cut in its setup (otherwise the stock it meets
/// is already an earlier operation's), and the tool is a flat end with
/// cutting data for the operation's material: the operation's own tool where
/// it is one, else the first in the project's order.
///
/// # Errors
///
/// The reason, in words for a refusal, when no roughing can be offered.
pub fn roughing_tool(project: &Project, operation: &Operation) -> Result<ToolId, String> {
    if !warns(&operation.strategy) {
        return Err("its strategy is not warned of its tool load".to_owned());
    }
    if operation.strategy == openchuck_path::roughing::NAME {
        return Err("it is rotary roughing itself; take a smaller Stepdown instead".to_owned());
    }
    let first = project
        .operations()
        .iter()
        .filter(|o| o.enabled && o.setup == operation.setup)
        .map(|o| o.id)
        .next();
    if first != Some(operation.id) {
        return Err(
            "it is not the first operation to cut in its setup, so the stock it meets is an earlier operation's"
                .to_owned(),
        );
    }
    let material = crate::preset::material_of(project, operation)
        .ok_or_else(|| "its setup has no stock material".to_owned())?;
    let fits = |t: &&openchuck_project::ToolSnapshot| {
        matches!(t.tool.shape, openchuck_types::tool::ToolShape::FlatEnd)
            && openchuck_types::tool::find_cutting_data(&t.cutting_data, material).is_some()
    };
    let tools = project.tools();
    operation
        .tool
        .and_then(|id| tools.iter().find(|t| t.id == id))
        .filter(fits)
        .or_else(|| tools.iter().find(fits))
        .map(|t| t.id)
        .ok_or_else(|| format!("the project has no flat end with cutting data for {material}"))
}

/// The run's line for the whole job, in the one form of messages, over every simulated operation: the
/// material removed over the cutting time, and the highest rate with the
/// operation it is in. `None` when nothing was measured.
#[must_use]
pub fn job_line<'a>(
    measured: impl IntoIterator<Item = (&'a str, &'a LoadSummary)>,
) -> Option<String> {
    let mut removed = 0.0;
    let mut time = 0.0;
    let mut highest: Option<(f64, &str)> = None;
    for (name, summary) in measured {
        removed += summary.removed;
        time += summary.cutting_time;
        if let Some(rate) = summary.fastest.and_then(|f| f.rate)
            && highest.is_none_or(|(h, _)| rate > h)
        {
            highest = Some((rate, name));
        }
    }
    if time <= 0.0 {
        return None;
    }
    let average = removed / time * 60.0;
    let highest = highest
        .map(|(rate, name)| format!(", {rate:.0} mm³/min at most (operation '{name}')"))
        .unwrap_or_default();
    Some(crate::lead_in::in_form(
        topic::TOOL_LOAD,
        &crate::lead_in::Context::default(),
        &format!(
            "the removal rate over the job is {average:.0} mm³/min on average over the \
             cutting time{highest}"
        ),
    ))
}

/// What the run says when the project skips the stock simulation: the
/// operations it does not simulate are not measured.
pub const NOT_MEASURED: &str = "the tool load and the removal rate are measured only where \
     the stock is simulated, and this project skips the stock simulation (Skip stock \
     simulation, in Playback), so they were not measured for every operation";

/// [`NOT_MEASURED`] in the one form of messages.
#[must_use]
pub fn not_measured() -> String {
    crate::lead_in::in_form(
        topic::TOOL_LOAD,
        &crate::lead_in::Context::default(),
        NOT_MEASURED,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use openchuck_sim::Stretch;

    fn summary(over: usize, section: f64) -> LoadSummary {
        let stretch = Stretch {
            first: 0,
            last: 0,
            section,
            rate: Some(600.0),
            engaged: 21.5,
        };
        LoadSummary {
            limit: 6.35,
            stretches: 10,
            over,
            heaviest: Some(stretch),
            fastest: Some(stretch),
            longest_engaged: Some((21.5, 0)),
            removed: 100.0,
            cutting_time: 10.0,
            total_time: 20.0,
        }
    }

    fn group() -> ToolpathGroup {
        let pose = openchuck_types::ir::ToolPose {
            tip: openchuck_types::geometry::Point3::new(20.0, 0.0, 5.0),
            axis: openchuck_types::geometry::UnitVec3::Z,
            a: openchuck_types::units::Degrees(132.0),
        };
        ToolpathGroup {
            rotary_reset_after: Vec::new(),
            tags: openchuck_types::ir::PathTags {
                operation: "op".into(),
                setup: "s".into(),
                tool: "t".into(),
            },
            start: pose,
            segments: vec![openchuck_types::ir::Segment {
                kind: openchuck_types::ir::MoveKind::Cutting,
                end: pose,
                contact_feed: Some(openchuck_types::units::FeedRate(500.0)),
                duration: openchuck_types::units::Seconds(1.0),
                contact_length: openchuck_types::units::Millimetres(1.0),
            }],
        }
    }

    fn twice() -> Limit {
        Limit {
            value: 6.35,
            source: LimitSource::TwiceTheSlot {
                stepdown: 1.0,
                material: "Hardwood".into(),
            },
        }
    }

    #[test]
    fn req_sim_16_req_nfr_12_the_warning_gives_the_figures_the_place_and_the_remedies() {
        let w = warning(&summary(4757, 185.7), &twice(), &group(), 22.0).unwrap();
        assert!(w.is_warning());
        assert_eq!(w.topic.as_deref(), Some(topic::TOOL_LOAD));
        assert_eq!(w.setting.as_deref(), Some(LARGEST_CUT_SECTION));
        let t = &w.text;
        for part in [
            "4757 stretches",
            "6.35 mm²",
            "twice a full-width slot at this tool's 1.00 mm stepdown in Hardwood",
            "a cut section of 185.70 mm² at X 20.000, A 132.0 (move 1)",
            "21.50 mm of the 22.00 mm flute",
            "Rough the setup first",
            "Largest cut section",
        ] {
            assert!(t.contains(part), "missing {part:?} in {t}");
        }
        // It ends with the notice that the user verifies the program, and
        // calls nothing safe of its own (NFR-12).
        assert!(
            t.ends_with(openchuck_types::notice::VERIFY_GCODE_NOTICE),
            "{t}"
        );
        let own = t.trim_end_matches(openchuck_types::notice::VERIFY_GCODE_NOTICE);
        assert!(!own.to_lowercase().contains("safe"), "{own}");
    }

    #[test]
    fn req_sim_16_no_stretch_past_the_limit_gives_no_warning() {
        assert!(warning(&summary(0, 5.3), &twice(), &group(), 22.0).is_none());
    }

    #[test]
    fn req_sim_16_one_stretch_is_counted_in_the_singular() {
        let w = warning(&summary(1, 7.0), &twice(), &group(), 22.0).unwrap();
        assert!(w.text.starts_with("1 stretch of the path"), "{}", w.text);
    }

    #[test]
    fn req_sim_16_the_information_line_gives_the_heaviest_cut_and_the_rates() {
        let n = information(&summary(0, 5.3), &twice());
        assert!(!n.is_warning());
        assert!(
            n.text
                .contains("the heaviest cut section is 5.30 mm² (limit 6.35)"),
            "{}",
            n.text
        );
        assert!(
            n.text
                .contains("600 mm³/min on average over the cutting time"),
            "{}",
            n.text
        );
        assert!(n.text.contains("600 mm³/min at most"), "{}", n.text);
    }

    #[test]
    fn req_sim_16_the_average_rate_is_given_when_no_cut_is_one_tool_diameter_long() {
        // Cuts shorter than one tool diameter give no stretch, so no highest
        // rate, but the average over the cutting time is still measured.
        let mut s = summary(0, 5.3);
        s.fastest = None;
        let n = information(&s, &twice());
        assert!(
            n.text
                .contains("600 mm³/min on average over the cutting time"),
            "{}",
            n.text
        );
        assert!(!n.text.contains("at most"), "{}", n.text);
    }

    #[test]
    fn req_sim_16_the_job_line_is_the_total_over_the_cutting_time() {
        let a = summary(0, 5.0);
        let mut b = summary(0, 5.0);
        b.removed = 300.0;
        b.cutting_time = 10.0;
        // (100 + 300) mm³ over 20 s: 1200 mm³/min, not the mean of 600 and 1800.
        let line = job_line([("Rough", &a), ("Finish", &b)]).unwrap();
        assert!(line.contains("1200 mm³/min on average"), "{line}");
        assert!(line.starts_with("Tool load: "), "{line}");
        assert!(job_line(std::iter::empty()).is_none());
    }

    #[test]
    fn req_sim_16_drilling_is_measured_but_not_warned() {
        assert!(!warns(openchuck_path::drill::NAME));
        assert!(warns("simultaneous_finishing"));
    }
}
