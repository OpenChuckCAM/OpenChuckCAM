// SPDX-License-Identifier: AGPL-3.0-or-later
//! Parameter forms: an operation's parameters as a view model built from its
//! strategy's schema, with units, ranges, defaults and where each shown value
//! comes from (UI-07; work breakdown slice 8 item 5; decision record 0029
//! item 11).
//!
//! The front end draws one field per parameter and calls
//! [`crate::Application::set_parameter`], which checks the value against the
//! schema before it changes anything: an invalid value is refused with a
//! message naming the parameter, and the project is left as it was.

use std::collections::BTreeMap;

use openchuck_path::contact::parameter::EXACT_CONTACT;
use openchuck_path::params::{ParameterKind, ParameterSchema, names};
use openchuck_path::{ParameterValue as PathValue, Registry};
use openchuck_project::id::OperationId;
use openchuck_project::{Operation, ParameterValue, Project};

use crate::AppError;
use crate::operation::parameter_values;

pub use openchuck_path::facing::selection::{FaceOffset, SideRequest, decode_rows, encode_rows};
pub use openchuck_path::params::TOOL_DIAMETER;
pub use openchuck_types::units::Degrees;

/// What kind of value a field takes.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum FieldKind {
    /// A number in `unit` (a symbol such as "mm", empty for a plain number).
    Number {
        /// The unit symbol.
        unit: &'static str,
        /// Lowest value, if any.
        min: Option<f64>,
        /// True when `min` itself is not allowed.
        min_exclusive: bool,
        /// Highest value, if any.
        max: Option<f64>,
    },
    /// A whole number from `min` to `max`.
    Integer {
        /// Lowest value.
        min: i64,
        /// Highest value.
        max: i64,
    },
    /// On or off.
    Flag,
    /// Free text, such as a list of angles.
    Text,
    /// Ordered arbitrary A/optional plane-offset rows, stored as one text
    /// parameter and edited as a table rather than exposed JSON.
    IndexedFaces,
    /// One of a list of words.
    Choice {
        /// The words, in display order.
        options: Vec<String>,
    },
    /// A length in millimetres, or the operation's tool's diameter, stored
    /// as the word [`openchuck_path::params::TOOL_DIAMETER`] (issue #854):
    /// a number field with a Tool diameter choice beside it.
    LengthOrTool {
        /// Lowest value, if any (inclusive).
        min: Option<f64>,
        /// Highest value, if any.
        max: Option<f64>,
        /// The operation's tool's cutting diameter now, in millimetres, to
        /// show beside the choice; `None` with no tool chosen.
        tool_diameter: Option<f64>,
    },
}

/// Where a field's shown value comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Source {
    /// The operation sets it.
    Operation,
    /// The tool's cutting data for the setup's stock material (TL-05).
    CuttingData,
    /// The strategy's default.
    Default,
    /// Nothing gives it, and it is required.
    Missing,
}

/// The section of the operation inspector a field is shown in (interface
/// specification section 8; decision record 0114 C10). The Geometry section,
/// which holds the selection roles, has no fields and is not listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum FormSection {
    /// The feeds.
    ToolAndFeeds,
    /// The pattern of passes, their spacing and the tolerance.
    PatternAndTolerance,
    /// How the tool axis is set and how A turns.
    ToolAxisAndA,
    /// Entry, links and retract (decision record 0026).
    EntryLinksAndRetract,
    /// Limits that most operations leave as they are.
    Advanced,
}

impl FormSection {
    /// Every section, in the order the inspector shows them.
    pub const ALL: [Self; 5] = [
        Self::ToolAndFeeds,
        Self::PatternAndTolerance,
        Self::ToolAxisAndA,
        Self::EntryLinksAndRetract,
        Self::Advanced,
    ];

    /// The section's title, in sentence case.
    pub const fn title(self) -> &'static str {
        match self {
            Self::ToolAndFeeds => "Tool and feeds",
            Self::PatternAndTolerance => "Pattern and tolerance",
            Self::ToolAxisAndA => "Tool axis and A",
            Self::EntryLinksAndRetract => "Entry, links and retract",
            Self::Advanced => "Advanced",
        }
    }

    /// A short key for remembering whether the section is open.
    pub const fn key(self) -> &'static str {
        match self {
            Self::ToolAndFeeds => "tool",
            Self::PatternAndTolerance => "pattern",
            Self::ToolAxisAndA => "axis",
            Self::EntryLinksAndRetract => "links",
            Self::Advanced => "advanced",
        }
    }

    /// The section of the parameter `name`. A parameter the list does not
    /// name, such as one a strategy adds later, is shown with the pattern.
    pub fn of(name: &str) -> Self {
        match name {
            "feed" | "plunge_feed" => Self::ToolAndFeeds,
            "angles" | "count" | "index_angle" | "start_angle" | "rotation" | "lead_angle"
            | "max_tilt" | "axis_tolerance" | "max_angle_step" | "tip_end" | "tip_extension" => {
                Self::ToolAxisAndA
            }
            // The basic entry settings (decision record PATH-QUALITY-1, Q9,
            // A's regrouping): Entry, Ramp angle and Plunge where no ramp
            // fits; the rest of the ramp, helix and lead settings are in
            // Advanced, below.
            "retract"
            | "link_height"
            | "approach_distance"
            | "entry"
            | "ramp_back_and_forth"
            | "allow_single_point_plunge"
            | "ramp_angle"
            | "reentry_clearance"
            | "retract_height"
            | "chip_break_retract"
            | "peck_mode"
            | "peck_depth" => Self::EntryLinksAndRetract,
            // Exact STEP surface contact is in Advanced for every strategy
            // that has it (decision record 0684 item 4).
            "diameter_tolerance" | "crease_angle" | "min_length" | "min_remaining" | "min_cut"
            | "max_material_left" | "through_allowance" | "max_step" | "max_x_step"
            | "depth_reference" | "exact_contact" => Self::Advanced,
            // The entry settings past the basic ones (decision record
            // PATH-QUALITY-1, Q9).
            "ramp_steeper_near_axis"
            | "ramp_angle_max"
            | "plunge_face_nubs"
            | "face_nub_diameter"
            | "ramp_depth_per_traverse"
            | "helix_diameter"
            | "lead_radius" => Self::Advanced,
            _ => Self::PatternAndTolerance,
        }
    }
}

/// One parameter of the form.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// The name stored in the operation.
    pub name: String,
    /// The label to show.
    pub label: String,
    /// What it does, for a tooltip.
    pub description: String,
    /// Its kind, unit and range.
    pub kind: FieldKind,
    /// The value the operation sets, if any.
    pub set: Option<ParameterValue>,
    /// The value that applies: the operation's, the cutting data's or the
    /// default. `None` when it is missing.
    pub value: Option<ParameterValue>,
    /// Where `value` comes from.
    pub source: Source,
    /// The inspector section it is shown in.
    pub section: FormSection,
    /// Why the setting cannot be used as it is, when that is known before
    /// the operation is generated, in plain words with what to change; shown
    /// under it.
    pub problem: Option<String>,
}

/// An operation's parameter form.
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterForm {
    /// The operation.
    pub operation: OperationId,
    /// Its strategy's registry name.
    pub strategy: String,
    /// The fields, in the schema's display order.
    pub fields: Vec<Field>,
    /// Notes: overrides of the cutting data, or why required values are
    /// missing.
    pub notes: Vec<String>,
}

fn from_path(v: &PathValue) -> ParameterValue {
    match v {
        PathValue::Integer(i) => ParameterValue::Integer(*i),
        PathValue::Number(n) => ParameterValue::Number(*n),
        PathValue::Flag(b) => ParameterValue::Flag(*b),
        PathValue::Text(t) => ParameterValue::Text(t.clone()),
    }
}

fn to_path(v: &ParameterValue) -> PathValue {
    match v {
        ParameterValue::Integer(i) => PathValue::Integer(*i),
        ParameterValue::Number(n) => PathValue::Number(*n),
        ParameterValue::Flag(b) => PathValue::Flag(*b),
        ParameterValue::Text(t) => PathValue::Text(t.clone()),
    }
}

/// The form of `operation` in `project`, from its strategy in `registry`.
pub fn parameter_form(
    project: &Project,
    registry: &Registry,
    operation: OperationId,
) -> Result<ParameterForm, AppError> {
    let op = project
        .operation(operation)
        .ok_or_else(|| AppError::UnknownOperation(operation.to_string()))?;
    let schema = crate::operation::effective_schema(op, registry.get(&op.strategy)?);
    let (resolved, mut notes) = match parameter_values(project, op, registry) {
        Ok(r) => (r.values, r.notes),
        Err(e) => (BTreeMap::new(), vec![e.user_message()]),
    };
    // Said in the form, before the operation is run (OI-96, OI-238;
    // decision 0111 items 1 and 3).
    if let Some(reason) = crate::operation::brep_only(project, registry, op)
        .or_else(|| crate::operation::needs_faces(project, op))
    {
        notes.insert(0, format!("{}.", reason.replacen("swarf", "Swarf", 1)));
    }
    // Every ramp follows the toolpath (decision 0543 D1): the
    // back-and-forth flag has no effect, and is shown only where a saved
    // operation has it on, as inactive.
    let reversing_set = matches!(
        op.parameters.get(names::RAMP_BACK_AND_FORTH),
        Some(ParameterValue::Flag(true))
    );
    let ramp = matches!(
        op.parameters.get(names::ENTRY),
        Some(ParameterValue::Text(style)) if style == "ramp"
    );
    let from_data = [names::FEED, names::PLUNGE_FEED, names::STEPOVER];
    // Exact STEP surface contact means something only on a STEP part
    // (decision record 0526): it is not shown for a mesh part.
    let on_step = crate::operation::on_step_part(project, op);
    let contact_problem = exact_contact_problem(project, registry, op);
    // The tool's cutting diameter, for a length that can be the tool's.
    let tool_diameter = project
        .tools()
        .iter()
        .find(|t| Some(t.id) == op.tool)
        .and_then(|t| t.tool.profile().ok())
        .map(|p| 2.0 * p.cutting_radius());
    let fields = schema
        .parameters
        .iter()
        .filter(|spec| on_step || spec.name != EXACT_CONTACT)
        .filter(|spec| reversing_set || spec.name != names::RAMP_BACK_AND_FORTH)
        .map(|spec| {
            let set = op.parameters.get(&spec.name).cloned();
            let (kind, default) = match &spec.kind {
                ParameterKind::Number {
                    unit,
                    min,
                    min_exclusive,
                    max,
                    default,
                } => (
                    FieldKind::Number {
                        unit: unit.symbol(),
                        min: *min,
                        min_exclusive: *min_exclusive,
                        max: *max,
                    },
                    crate::operation::default_override(project, op, &spec.name)
                        .or(*default)
                        .map(ParameterValue::Number),
                ),
                ParameterKind::Integer { min, max, default } => (
                    FieldKind::Integer {
                        min: *min,
                        max: *max,
                    },
                    default.map(ParameterValue::Integer),
                ),
                ParameterKind::Flag { default } => {
                    (FieldKind::Flag, Some(ParameterValue::Flag(*default)))
                }
                ParameterKind::AngleList { default } => {
                    (FieldKind::Text, Some(ParameterValue::Text(default.clone())))
                }
                ParameterKind::IndexedFaces { default } => (
                    FieldKind::IndexedFaces,
                    Some(ParameterValue::Text(default.clone())),
                ),
                ParameterKind::Choice { options, default } => (
                    FieldKind::Choice {
                        options: options.clone(),
                    },
                    Some(ParameterValue::Text(default.clone())),
                ),
                ParameterKind::LengthOrTool { min, max, default } => (
                    FieldKind::LengthOrTool {
                        min: *min,
                        max: *max,
                        tool_diameter,
                    },
                    Some(from_path(&default.stored())),
                ),
                // A kind added to the library later shows as text.
                _ => (
                    FieldKind::Choice {
                        options: Vec::new(),
                    },
                    None,
                ),
            };
            let data = (set.is_none() && from_data.contains(&spec.name.as_str()))
                .then(|| resolved.get(&spec.name).map(from_path))
                .flatten();
            let (value, source) = match (set.clone(), data, default) {
                (Some(v), _, _) => (Some(v), Source::Operation),
                (None, Some(v), _) => (Some(v), Source::CuttingData),
                (None, None, Some(v)) => (Some(v), Source::Default),
                (None, None, None) => (None, Source::Missing),
            };
            Field {
                section: FormSection::of(&spec.name),
                name: spec.name.clone(),
                label: if spec.name == names::RAMP_BACK_AND_FORTH
                    || (!ramp
                        && [
                            names::ALLOW_SINGLE_POINT_PLUNGE,
                            names::RAMP_STEEPER_NEAR_AXIS,
                            names::RAMP_ANGLE_MAX,
                            names::PLUNGE_FACE_NUBS,
                            names::FACE_NUB_DIAMETER,
                            names::RAMP_DEPTH_PER_TRAVERSE,
                        ]
                        .contains(&spec.name.as_str()))
                {
                    format!("{} (inactive)", spec.label)
                } else {
                    facing_face_label(project, &op.strategy, &spec.name)
                        .unwrap_or_else(|| spec.label.clone())
                },
                description: spec.description.clone(),
                kind,
                set,
                value,
                source,
                problem: match spec.name.as_str() {
                    EXACT_CONTACT => contact_problem.clone(),
                    _ => None,
                },
            }
        })
        .collect();
    Ok(ParameterForm {
        operation,
        strategy: op.strategy.clone(),
        fields,
        notes,
    })
}

/// Why Exact STEP surface contact does not apply to `operation`'s tool, in
/// plain words, when it does not (decision record 0526, NFR-07): the tool
/// then drops on the tessellation whatever the setting. `None` when it
/// applies, or when the strategy has no such setting or the tool is unknown.
fn exact_contact_problem(project: &Project, registry: &Registry, op: &Operation) -> Option<String> {
    use openchuck_path::contact::ExactContact;
    let kind = registry.get(&op.strategy).ok()?.exact_contact()?;
    let profile = project
        .tools()
        .iter()
        .find(|t| Some(t.id) == op.tool)?
        .tool
        .profile()
        .ok()?;
    if kind.supports(&profile) {
        return None;
    }
    Some(match kind {
        ExactContact::Ball => "This tool drops on the tessellation whatever this setting: in \
                               this strategy the exact contact is built for ball-end tools only."
            .to_owned(),
        _ => "This tool drops on the tessellation whatever this setting: the exact contact is \
              not built for its shape (a bull nose, a flat end with stock to leave, a sharp \
              V-bit or a concave profile)."
            .to_owned(),
    })
}

// Canonical model faces use the same fixed order as 0252/SideSelection.
// Show the actual machine sense, leaving the face name alone if it is unknown.
fn facing_face_label(project: &Project, strategy: &str, name: &str) -> Option<String> {
    if strategy != openchuck_path::facing::NAME {
        return None;
    }
    let (face, angle) = match name {
        "positive_z" => ("+Z", 0.),
        "positive_y" => ("+Y", 90.),
        "negative_z" => ("−Z", 180.),
        "negative_y" => ("−Y", 270.),
        _ => return None,
    };
    let machine = project.machine()?;
    let a = if name == "positive_z" {
        0.
    } else {
        angle * machine.a_sense.sign()
    };
    Some(format!("{face} (A {a:.0}°)"))
}

/// Checks one value of `operation`'s strategy against its schema: the name
/// must exist, and the value must have the right kind and be in range.
/// `None` (clear the value) is always allowed: the cutting data or the
/// default then applies, and a value still missing is reported when the
/// operation is generated.
pub(crate) fn check_parameter(
    project: &Project,
    registry: &Registry,
    operation: OperationId,
    name: &str,
    value: Option<&ParameterValue>,
) -> Result<(), AppError> {
    let op = project
        .operation(operation)
        .ok_or_else(|| AppError::UnknownOperation(operation.to_string()))?;
    let strategy = registry.get(&op.strategy)?;
    let schema = strategy.schema();
    let mut proposed = crate::operation::stored_values(op);
    match value {
        Some(value) => {
            proposed.insert(name.to_owned(), to_path(value));
        }
        None => {
            proposed.remove(name);
        }
    }
    let active = strategy.schema_for(&proposed);
    // Visible mode-specific controls use their active range. Known inactive
    // values remain editable through the host API using their union spec.
    let spec = active
        .get(name)
        .or_else(|| schema.get(name))
        .ok_or_else(|| {
            AppError::refused(
                openchuck_path::topic::SETTING,
                &op.name,
                format!("its strategy has no parameter '{name}'"),
            )
        })?;
    let Some(value) = value else { return Ok(()) };
    let single = ParameterSchema {
        parameters: vec![spec.clone()],
    };
    let mut values = BTreeMap::new();
    values.insert(name.to_owned(), to_path(value));
    single.resolve(&values)?;
    Ok(())
}

/// The words the inspector shows for the stored word `value` of the choice
/// `name`, or `None` when there is none (issue 613).
#[must_use]
pub fn choice_label(name: &str, value: &str) -> Option<&'static str> {
    Some(match (name, value) {
        ("shape", "flat") => "Flat along stock",
        ("shape", "round") => "Round",
        ("shape", "end") => "Unheld end",
        ("shape", "box") => "Box sides",
        ("extent", "whole_stock") => "Whole stock",
        ("extent", "part") => "Part only",
        ("extent", "part_plus_tabs") => "Part plus tabs",
        ("layer_distribution", "full") => "Full stepdown, shorter final pass",
        ("layer_distribution", "equal") => "Equal stepdowns",
        ("pass_shape", "helix") => "Spiral",
        ("pass_shape", "rings") => "Rings",
        ("link_height", "retract") => "Retract to the clearance",
        ("link_height", "low") => "Stay low where clear",
        ("ring_step", "sharp") => "Sharp",
        ("ring_step", "blended") => "Blended",
        ("retract", "clearance") => "Safe cylinder",
        ("retract", "safe_z") => "Safe Z",
        ("entry", "plunge") => "Plunge",
        ("entry", "lead") => "Tangent arc",
        ("entry", "ramp") => "Straight ramp",
        ("entry", "helix") => "Descending helix",
        ("direction", "zigzag") => "Zig-zag",
        ("direction", "climb") => "Climb",
        ("direction", "conventional") => "Conventional",
        ("direction", "meander") => "Meander",
        ("direction", "forward") => "Forward",
        ("direction", "reverse") => "Reverse",
        ("direction", "usual for the form") => "Usual for the form",
        ("direction", "along the length of the part") => "Along the length of the part",
        ("direction", "around the part") => "Around the part",
        ("extend", "no") => "Neither end",
        ("extend", "both ends") => "Both ends",
        ("extend", "start end") => "Start end",
        ("extend", "far end") => "Far end",
        ("mode", "engrave") => "Engrave",
        ("mode", "carve") => "Carve",
        ("start", "first") => "As stored",
        ("start", "reversed") => "Reversed",
        ("start", "low_x") => "Lowest X",
        ("start", "high_x") => "Highest X",
        ("start", "angle") => "Nearest the start angle",
        ("rotation", "positive") => "Positive",
        ("rotation", "negative") => "Negative",
        ("pass_order", "by_region") => "By region",
        ("pass_order", "after_main") => "After main passes",
        ("peck_mode", "full_retract") => "Full retract",
        ("peck_mode", "chip_break") => "Chip break",
        ("depth_reference", "full_diameter") => "Full diameter",
        ("depth_reference", "tip") => "Tool tip",
        ("form", "continuous") => "Continuous",
        ("form", "indexed") => "Indexed",
        ("pattern", "spiral") => "Spiral",
        ("pattern", "axial") => "Axial",
        ("pattern", "automatic") => "Automatic",
        ("pattern", "along_guide") => "Along the guide",
        ("pattern", "outline") => "Follow the outline",
        ("tip_end", "auto") => "Automatic",
        ("tip_end", "reversed") => "Reversed",
        _ => return None,
    })
}

#[cfg(test)]
mod section_tests {
    use super::FormSection;

    /// Each parameter goes in the inspector section of specification
    /// section 8; one the list does not name goes with the pattern.
    #[test]
    fn req_ui_07_parameters_are_placed_in_the_sections_of_section_8() {
        assert_eq!(FormSection::of("feed"), FormSection::ToolAndFeeds);
        assert_eq!(FormSection::of("max_tilt"), FormSection::ToolAxisAndA);
        assert_eq!(FormSection::of("entry"), FormSection::EntryLinksAndRetract);
        assert_eq!(FormSection::of("min_length"), FormSection::Advanced);
        assert_eq!(FormSection::of("exact_contact"), FormSection::Advanced);
        assert_eq!(
            FormSection::of("ramp_angle"),
            FormSection::EntryLinksAndRetract
        );
        assert_eq!(
            FormSection::of("allow_single_point_plunge"),
            FormSection::EntryLinksAndRetract
        );
        for name in [
            "ramp_steeper_near_axis",
            "ramp_angle_max",
            "plunge_face_nubs",
            "face_nub_diameter",
            "ramp_depth_per_traverse",
            "helix_diameter",
            "lead_radius",
        ] {
            assert_eq!(FormSection::of(name), FormSection::Advanced, "{name}");
        }
        assert_eq!(
            FormSection::of("stepover"),
            FormSection::PatternAndTolerance
        );
        assert_eq!(
            FormSection::of("a_new_one"),
            FormSection::PatternAndTolerance
        );
        let titles: Vec<&str> = FormSection::ALL.iter().map(|s| s.title()).collect();
        assert_eq!(
            titles,
            [
                "Tool and feeds",
                "Pattern and tolerance",
                "Tool axis and A",
                "Entry, links and retract",
                "Advanced"
            ]
        );
        let keys: Vec<&str> = FormSection::ALL.iter().map(|s| s.key()).collect();
        assert_eq!(keys, ["tool", "pattern", "axis", "links", "advanced"]);
    }
}

#[cfg(test)]
mod choice_label_tests {
    #![allow(clippy::unwrap_used)]

    use openchuck_path::{ParameterKind, Registry};

    use super::choice_label;

    /// Every option of every choice of every built-in strategy, as
    /// `(strategy, setting, option)`.
    fn every_choice() -> Vec<(String, String, String)> {
        let registry = Registry::builtin();
        let mut found = Vec::new();
        for strategy in registry.iter() {
            for spec in strategy.schema().parameters {
                if let ParameterKind::Choice { options, .. } = &spec.kind {
                    for option in options {
                        found.push((
                            strategy.name().to_owned(),
                            spec.name.clone(),
                            option.clone(),
                        ));
                    }
                }
            }
        }
        found
    }

    /// The inspector never shows a stored word: each option of each choice has
    /// a label written in plain words, so a new option cannot be added
    /// without one (issue 613).
    #[test]
    fn req_ui_12_every_choice_of_every_strategy_has_a_label() {
        let missing: Vec<String> = every_choice()
            .into_iter()
            .filter(|(_, setting, option)| choice_label(setting, option).is_none())
            .map(|(strategy, setting, option)| format!("{strategy}: {setting} = {option}"))
            .collect();
        assert!(missing.is_empty(), "no label for:\n{}", missing.join("\n"));
    }

    /// A label is words: it starts with a capital, has no underscore, and two
    /// options of one setting never share one.
    #[test]
    fn req_ui_12_choice_labels_are_plain_words_and_differ() {
        let mut seen = std::collections::BTreeMap::<(String, String), String>::new();
        for (strategy, setting, option) in every_choice() {
            let Some(label) = choice_label(&setting, &option) else {
                continue;
            };
            assert!(
                !label.contains('_'),
                "{strategy}: {setting} = {option}: {label}"
            );
            assert!(
                label.chars().next().is_some_and(char::is_uppercase),
                "{strategy}: {setting} = {option}: {label}"
            );
            let key = (strategy.clone(), setting.clone());
            let clash = seen.insert(
                (strategy.clone(), format!("{setting}\u{0}{label}")),
                option.clone(),
            );
            assert!(
                clash.is_none() || clash.as_deref() == Some(option.as_str()),
                "{key:?} {label}"
            );
        }
    }

    /// The pass order of rest machining and the other strategies with extra
    /// passes reads in words (issue 613); the stored words are unchanged.
    #[test]
    fn req_op_14_the_pass_order_choice_reads_in_words() {
        assert_eq!(choice_label("pass_order", "by_region"), Some("By region"));
        assert_eq!(
            choice_label("pass_order", "after_main"),
            Some("After main passes")
        );
    }
}
