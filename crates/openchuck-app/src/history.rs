// SPDX-License-Identifier: AGPL-3.0-or-later
//! The undo history as the user sees it (UI-22; specification section 12.3;
//! decision PROJUX-B-1): each step named in the user's words with its item,
//! how long ago it was made, whether it is undone, and the step the project
//! was last saved at.
//!
//! A step's name is worked out once, when the command runs, against the
//! project as it was just before: the name of an item that a later step
//! renames or deletes stays as it was then. Values are in the project's own
//! units (millimetres and degrees, NFR-05). The list lives in memory only;
//! nothing is written to the project or the log (the log keeps its own
//! lines without values, NFR-18).

use std::time::Duration;

use openchuck_path::Registry;
use openchuck_path::params::{ParameterKind, Unit};
use openchuck_project::{Item, ItemId, ListKind, ParameterValue, Project};

use crate::command::Command;

/// The undo history, oldest step first.
#[derive(Debug, Clone, PartialEq)]
pub struct UndoHistory {
    /// What the history starts from: "Opened project", "Created project"
    /// or "Recovered project".
    pub start: &'static str,
    /// How long ago the project was opened, created or recovered.
    pub start_age: Duration,
    /// Every step, oldest first: the applied ones, then the undone ones.
    pub steps: Vec<HistoryStep>,
    /// How many steps are applied: the project is as it was after step
    /// `position` (0 is as opened). Steps from `position` on are undone.
    pub position: usize,
    /// Where the project was last saved in this session, while that state
    /// can still be reached; `None` when it was not saved in this session or
    /// a new edit discarded the saved state.
    pub saved: Option<SavedMark>,
}

/// One step of the [`UndoHistory`].
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryStep {
    /// The step in words, with its item: "Set stepover 1.2 mm (Roughing)".
    pub label: String,
    /// How long ago the step was first made.
    pub age: Duration,
    /// Whether the step is undone (a new edit discards it).
    pub undone: bool,
}

/// Where the project was last saved in the [`UndoHistory`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SavedMark {
    /// The position the save was made at (see [`UndoHistory::position`]).
    pub position: usize,
    /// How long ago.
    pub age: Duration,
}

/// The name of `command` in the history, read against `project` as it is
/// just before the command runs.
pub(crate) fn label(command: &Command, project: &Project, registry: &Registry) -> String {
    match command {
        Command::RenameProject(name) => format!("Rename project to \"{name}\""),
        Command::SetMachine(machine) | Command::SetMachineFrom { machine, .. } => match machine {
            Some(m) => format!("Choose machine \"{}\"", m.name),
            None => "Clear the machine".to_owned(),
        },
        Command::SetSimulation(_) => "Change the simulation settings".to_owned(),
        Command::SetKeepTangentEdges(on) => format!("Keep tangent edges: {}", on_off(*on)),
        Command::SetWarnOnSelectionReset(on) => {
            format!("Warn before a selection is reset: {}", on_off(*on))
        }
        Command::SetSkipStockSimulation(skip) => format!(
            "Turn {} the stock simulation",
            if *skip { "off" } else { "on" }
        ),
        Command::SetPathsShown { operations, shown } => {
            let verb = if *shown { "Show" } else { "Hide" };
            match operations.as_slice() {
                [one] => format!(
                    "{verb} the toolpath of {}",
                    quoted(project, ItemId::Operation(*one))
                ),
                many if many.len() == project.operations().len() => {
                    format!("{verb} all toolpaths")
                }
                many => format!("{verb} {} toolpaths", many.len()),
            }
        }
        Command::Add { item, .. } => match item {
            Item::Model(m) => format!("Import model \"{}\"", file_name(m.display_name())),
            _ => format!("Add {} \"{}\"", item.id().kind(), item_name(item)),
        },
        Command::Remove(id) => match id {
            ItemId::Model(_) => format!("Remove model {}", quoted(project, *id)),
            _ => format!("Delete {} {}", id.kind(), quoted(project, *id)),
        },
        Command::Replace(new) => {
            let id = new.id();
            let name = item_name(new);
            match crate::action_log::item_name(project, id) {
                Some(old) if old != name => {
                    format!("Rename {} \"{old}\" to \"{name}\"", id.kind())
                }
                _ => format!("Edit {} \"{name}\"", id.kind()),
            }
        }
        Command::Move { id, .. } => format!("Move {} {}", id.kind(), quoted(project, *id)),
        Command::SetOrderKeys(keys) => format!("Change the order of {} items", keys.len()),
        Command::Renumber(list) => format!("Renumber the {}", list_name(*list)),
        Command::SetParameter {
            operation,
            name,
            value,
        } => {
            let op = project.operation(*operation);
            // The schema the operation's form shows, so the name matches
            // the field's label.
            let spec = op.and_then(|op| {
                let strategy = registry.get(&op.strategy).ok()?;
                crate::operation::effective_schema(op, strategy)
                    .parameters
                    .into_iter()
                    .find(|p| &p.name == name)
            });
            let shown = spec
                .as_ref()
                .map_or_else(|| name.replace('_', " "), |s| lower_first(&s.label));
            let op_name = op.map_or_else(|| "operation".to_owned(), |op| op.name.clone());
            match value {
                Some(v) => format!(
                    "Set {shown} {} ({op_name})",
                    value_text(v, spec.as_ref().map(|s| &s.kind))
                ),
                None => format!("Reset {shown} ({op_name})"),
            }
        }
        Command::SetOperationEnabled { operation, enabled } => format!(
            "Turn {} operation {}",
            on_off(*enabled),
            quoted(project, ItemId::Operation(*operation))
        ),
        Command::SetModelUnits { model, units } => {
            let model = quoted(project, ItemId::Model(*model));
            match units {
                Some(unit) => format!("Read model {model} in {unit}"),
                None => format!("Read model {model} in its file's units"),
            }
        }
        Command::SetModelTransform { model, .. } => format!(
            "Change the placement of model {}",
            quoted(project, ItemId::Model(*model))
        ),
        Command::Batch(commands) => batch(commands, project, registry),
    }
}

/// Several commands as one step: one is named as itself; several on one
/// item are "Edit <item>"; otherwise the first, and how many more.
fn batch(commands: &[Command], project: &Project, registry: &Registry) -> String {
    match commands {
        [] => "No change".to_owned(),
        [one] => label(one, project, registry),
        [first, rest @ ..] => {
            let target = target(first);
            if target.is_some() && rest.iter().all(|c| self::target(c) == target) {
                if let Some(id) = target {
                    return format!("Edit {} {}", id.kind(), quoted(project, id));
                }
            }
            format!(
                "{} and {} more",
                label(first, project, registry),
                rest.len()
            )
        }
    }
}

/// The one item a command changes, if there is one.
fn target(command: &Command) -> Option<ItemId> {
    match command {
        Command::Add { item, .. } | Command::Replace(item) => Some(item.id()),
        Command::Remove(id) | Command::Move { id, .. } => Some(*id),
        Command::SetParameter { operation, .. }
        | Command::SetOperationEnabled { operation, .. } => Some(ItemId::Operation(*operation)),
        Command::SetModelUnits { model, .. } | Command::SetModelTransform { model, .. } => {
            Some(ItemId::Model(*model))
        }
        _ => None,
    }
}

/// `"name"` of an item in the project, or its kind when it is not there.
fn quoted(project: &Project, id: ItemId) -> String {
    crate::action_log::item_name(project, id)
        .map_or_else(|| id.kind().to_owned(), |n| format!("\"{n}\""))
}

fn item_name(item: &Item) -> String {
    match item {
        Item::Model(m) => file_name(m.display_name()),
        Item::Setup(s) => s.name.clone(),
        Item::Tool(t) => t.name.clone(),
        Item::SelectionSet(s) => s.name.clone(),
        Item::Operation(o) => o.name.clone(),
    }
}

/// The last component of a model's path.
fn file_name(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).to_owned()
}

const fn on_off(on: bool) -> &'static str {
    if on { "on" } else { "off" }
}

const fn list_name(list: ListKind) -> &'static str {
    match list {
        ListKind::Models => "models",
        ListKind::Setups => "setups",
        ListKind::Tools => "tools",
        ListKind::SelectionSets => "selection sets",
        ListKind::Operations => "operations",
    }
}

/// A parameter label as it reads inside a sentence: "Stepover" becomes
/// "stepover", but "A step" and "XY feed" keep their capitals.
fn lower_first(label: &str) -> String {
    let mut chars = label.chars();
    match (chars.next(), chars.next()) {
        (Some(first), Some(second)) if first.is_uppercase() && second.is_lowercase() => {
            first.to_lowercase().chain(label.chars().skip(1)).collect()
        }
        _ => label.to_owned(),
    }
}

/// The longest text value shown, in characters.
const TEXT_SHOWN: usize = 24;

/// A value as the history shows it: a number with its unit and no trailing
/// zeros, "on" or "off", or the text, shortened.
fn value_text(value: &ParameterValue, kind: Option<&ParameterKind>) -> String {
    match value {
        ParameterValue::Number(n) => {
            let number = number(*n);
            match kind {
                Some(ParameterKind::Number { unit, .. }) => match unit {
                    Unit::Degrees => format!("{number}°"),
                    Unit::None => number,
                    other => format!("{number} {}", other.symbol()),
                },
                _ => number,
            }
        }
        ParameterValue::Integer(n) => n.to_string(),
        ParameterValue::Flag(on) => on_off(*on).to_owned(),
        ParameterValue::Text(text) => {
            let text = text.replace('_', " ");
            if text.chars().count() > TEXT_SHOWN {
                let cut: String = text.chars().take(TEXT_SHOWN - 1).collect();
                format!("\"{cut}…\"")
            } else {
                format!("\"{text}\"")
            }
        }
    }
}

/// A number to three decimal places at most, with no trailing zeros.
fn number(value: f64) -> String {
    let text = format!("{value:.3}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" {
        "0".to_owned()
    } else {
        text.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn req_ui_22_numbers_drop_trailing_zeros() {
        assert_eq!(number(1.2), "1.2");
        assert_eq!(number(50.0), "50");
        assert_eq!(number(0.0050), "0.005");
        assert_eq!(number(-0.0001), "0");
        assert_eq!(number(12.3456), "12.346");
    }

    #[test]
    fn req_ui_22_labels_read_inside_a_sentence() {
        assert_eq!(lower_first("Stepover"), "stepover");
        assert_eq!(lower_first("A step"), "A step");
        assert_eq!(lower_first("XY feed"), "XY feed");
        assert_eq!(lower_first(""), "");
    }

    #[test]
    fn req_ui_22_values_show_their_unit() {
        let deg = ParameterKind::Number {
            unit: Unit::Degrees,
            min: None,
            min_exclusive: false,
            max: None,
            default: None,
        };
        assert_eq!(value_text(&ParameterValue::Number(4.0), Some(&deg)), "4°");
        assert_eq!(value_text(&ParameterValue::Number(2.5), None), "2.5");
        assert_eq!(value_text(&ParameterValue::Flag(true), None), "on");
        assert_eq!(value_text(&ParameterValue::Integer(3), None), "3");
        assert_eq!(
            value_text(&ParameterValue::Text("climb_milling".into()), None),
            "\"climb milling\""
        );
        let long = "0, 45, 90, 135, 180, 225, 270, 315";
        assert_eq!(
            value_text(&ParameterValue::Text(long.into()), None),
            "\"0, 45, 90, 135, 180, 22…\""
        );
    }
}
