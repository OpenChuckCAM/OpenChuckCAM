// SPDX-License-Identifier: AGPL-3.0-or-later
//! The undo history list (UI-22; specification section 12.3; decision
//! PROJUX-B-1): every step is listed in the user's words, newest last here
//! (the interface shows it newest first), undone steps are marked, the step
//! the project was last saved at is marked, and returning to a step is the
//! same as that many undos or redos.

// Test code may unwrap, index and panic (NFR-07 applies to library code).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::BTreeMap;

use openchuck_app::project::id::{ModelId, OperationId, SetupId, ToolId};
use openchuck_app::project::{
    ContentHash, Item, ItemId, ModelRef, Operation, OrderKey, ParameterValue, Placement,
    RelativePath, Setup, ToolSnapshot,
};
use openchuck_app::{AppError, Application, Command, Threads};
use openchuck_types::tool::{ControllerMapping, Tool, ToolShape};
use openchuck_types::units::Millimetres;

fn open_app(name: &str) -> Application {
    let dir = std::env::temp_dir().join(format!(
        "openchuck-undo-history-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    Application::create(&dir, "Demo", Threads::Exactly(1)).unwrap()
}

const SETUP: SetupId = SetupId::from_u128(1);
const TOOL: ToolId = ToolId::from_u128(2);
const OP: OperationId = OperationId::from_u128(3);
const MODEL: ModelId = ModelId::from_u128(4);

fn add_setup() -> Command {
    Command::Add {
        item: Item::Setup(Setup::new(SETUP, "Setup 1")),
        placement: Placement::End,
    }
}

fn add_tool() -> Command {
    Command::Add {
        item: Item::Tool(ToolSnapshot {
            id: TOOL,
            order: OrderKey::default(),
            name: "6 mm flat".into(),
            tool: Tool {
                shape: ToolShape::FlatEnd,
                diameter: Millimetres(6.0),
                flute_length: Millimetres(10.0),
                overall_length: Millimetres(25.0),
                shank_diameter: Millimetres(6.0),
                flute_count: 2,
                material: None,
                centre_cutting: None,
                stick_out: None,
            },
            controller: ControllerMapping::numbered(3),
            cutting_data: Default::default(),
        }),
        placement: Placement::End,
    }
}

fn add_operation() -> Command {
    Command::Add {
        item: Item::Operation(Operation {
            enabled: true,
            id: OP,
            order: OrderKey::default(),
            name: "Roughing".into(),
            setup: SETUP,
            strategy: "helical_finishing".into(),
            tool: Some(TOOL),
            selections: Vec::new(),
            boundary: None,
            avoid: Vec::new(),
            check: Vec::new(),
            restriction_options: Default::default(),
            spindle_speed: None,
            material: None,
            parameters: BTreeMap::new(),
        }),
        placement: Placement::End,
    }
}

fn add_model() -> Command {
    Command::Add {
        item: Item::Model(ModelRef::new(
            MODEL,
            RelativePath::new("models/shaft.stl").unwrap(),
            ContentHash::of_bytes(b"shaft"),
        )),
        placement: Placement::End,
    }
}

fn set(name: &str, value: Option<ParameterValue>) -> Command {
    Command::SetParameter {
        operation: OP,
        name: name.into(),
        value,
    }
}

/// The names of the steps, oldest first.
fn labels(app: &Application) -> Vec<String> {
    app.undo_history()
        .steps
        .iter()
        .map(|s| s.label.clone())
        .collect()
}

/// A project with a model, a setup, a tool and an operation: four steps.
fn project_with_operation(name: &str) -> Application {
    let mut app = open_app(name);
    for c in [add_model(), add_setup(), add_tool(), add_operation()] {
        app.execute(c).unwrap();
    }
    app
}

#[test]
fn req_ui_22_steps_are_named_in_the_users_words_with_their_item() {
    let mut app = project_with_operation("names");
    app.execute(set("stepover", Some(ParameterValue::Number(1.2))))
        .unwrap();
    app.execute(set("stepover", None)).unwrap();
    app.execute(Command::SetOperationEnabled {
        operation: OP,
        enabled: false,
    })
    .unwrap();
    app.execute(Command::SetPathsShown {
        operations: vec![OP],
        shown: false,
    })
    .unwrap();
    let mut renamed = app.project().setup(SETUP).unwrap().clone();
    renamed.name = "Flip".into();
    app.execute(Command::Replace(Item::Setup(renamed))).unwrap();
    let mut edited = app.project().setup(SETUP).unwrap().clone();
    edited.frame.part_offset = Millimetres(2.0);
    app.execute(Command::Replace(Item::Setup(edited))).unwrap();
    app.execute(Command::RenameProject("Shaft".into())).unwrap();
    app.execute(Command::Remove(ItemId::Operation(OP))).unwrap();
    assert_eq!(
        labels(&app),
        [
            "Import model \"shaft.stl\"",
            "Add setup \"Setup 1\"",
            "Add tool \"6 mm flat\"",
            "Add operation \"Roughing\"",
            "Set stepover 1.2 mm (Roughing)",
            "Reset stepover (Roughing)",
            "Turn off operation \"Roughing\"",
            "Hide the toolpath of \"Roughing\"",
            "Rename setup \"Setup 1\" to \"Flip\"",
            "Edit setup \"Flip\"",
            "Rename project to \"Shaft\"",
            "Delete operation \"Roughing\"",
        ]
    );
}

#[test]
fn req_ui_22_values_kept_together_are_one_step_named_by_their_item() {
    // Several values of one item applied together (Keep, decision
    // UI-BUGS-A) are one step, named by the item, and one undo reverses it.
    let mut app = project_with_operation("batch");
    app.execute(Command::Batch(vec![
        set("stepover", Some(ParameterValue::Number(1.0))),
        set("stepdown", Some(ParameterValue::Number(2.0))),
    ]))
    .unwrap();
    assert_eq!(
        labels(&app).last().map(String::as_str),
        Some("Edit operation \"Roughing\"")
    );
    // One command in a batch is named as that command.
    app.execute(Command::Batch(vec![set(
        "stepover",
        Some(ParameterValue::Number(0.5)),
    )]))
    .unwrap();
    assert_eq!(
        labels(&app).last().map(String::as_str),
        Some("Set stepover 0.5 mm (Roughing)")
    );
    assert_eq!(app.undo_history().steps.len(), 6);
}

#[test]
fn req_ui_22_undone_steps_are_marked_and_a_new_edit_discards_them() {
    let mut app = project_with_operation("undone");
    assert!(app.undo().unwrap());
    assert!(app.undo().unwrap());
    let history = app.undo_history();
    assert_eq!(history.position, 2);
    let undone: Vec<bool> = history.steps.iter().map(|s| s.undone).collect();
    assert_eq!(undone, [false, false, true, true]);
    // The undone steps keep their names, in the order they were made.
    assert_eq!(history.steps[2].label, "Add tool \"6 mm flat\"");
    assert_eq!(history.steps[3].label, "Add operation \"Roughing\"");

    app.execute(Command::RenameProject("Other".into())).unwrap();
    let history = app.undo_history();
    assert_eq!(history.position, 3);
    assert_eq!(
        labels(&app),
        [
            "Import model \"shaft.stl\"",
            "Add setup \"Setup 1\"",
            "Rename project to \"Other\"",
        ]
    );
    assert!(history.steps.iter().all(|s| !s.undone));
}

#[test]
fn req_ui_22_returning_to_a_step_gives_the_same_project_as_that_many_undos_or_redos() {
    let mut jumped = project_with_operation("jump");
    let mut stepped = project_with_operation("step");
    let total = jumped.undo_history().steps.len();
    // Back to every step, then forward to every step, from wherever the
    // last jump left the project.
    let targets: Vec<usize> = (0..=total).rev().chain(0..=total).collect();
    for target in targets {
        jumped.go_to_step(target).unwrap();
        while stepped.undo_history().position > target {
            assert!(stepped.undo().unwrap());
        }
        while stepped.undo_history().position < target {
            assert!(stepped.redo().unwrap());
        }
        assert_eq!(jumped.project(), stepped.project(), "step {target}");
        assert_eq!(jumped.undo_history().position, target);
        assert_eq!(jumped.can_undo(), target > 0);
        assert_eq!(jumped.can_redo(), target < total);
    }
}

#[test]
fn req_ui_22_req_nfr_07_a_step_that_does_not_exist_is_refused_and_changes_nothing() {
    let mut app = project_with_operation("refused");
    let before = app.project().clone();
    let revision = app.revision();
    let err = app.go_to_step(5).unwrap_err();
    assert!(matches!(err, AppError::NoHistoryStep { step: 5, last: 4 }));
    assert_eq!(
        err.to_string(),
        "there is no step 5 in the undo history; it has 4"
    );
    assert_eq!(app.project(), &before);
    assert_eq!(app.revision(), revision);
    assert_eq!(app.undo_history().position, 4);
    // Going to the step the project is at changes nothing either.
    app.go_to_step(4).unwrap();
    assert_eq!(app.revision(), revision);
}

#[test]
fn req_ui_22_the_last_save_is_marked_and_goes_when_its_state_is_discarded() {
    let mut app = project_with_operation("saved");
    // As opened: the project is the saved files, and nothing is marked
    // because nothing was saved in this session.
    let fresh = open_app("saved-fresh");
    assert_eq!(fresh.undo_history().saved, None);
    assert_eq!(fresh.undo_history().steps.len(), 0);

    app.save().unwrap();
    assert_eq!(app.undo_history().saved.map(|s| s.position), Some(4));
    // Undoing past the save keeps the mark where the save was.
    assert!(app.undo().unwrap());
    assert_eq!(app.undo_history().saved.map(|s| s.position), Some(4));
    assert!(app.is_modified());
    app.go_to_step(4).unwrap();
    assert!(!app.is_modified());
    // A new edit from an earlier step discards the saved state: no mark.
    app.go_to_step(3).unwrap();
    app.execute(Command::RenameProject("Other".into())).unwrap();
    assert_eq!(app.undo_history().saved, None);
    // A new edit after the saved step keeps the mark.
    app.save().unwrap();
    app.execute(Command::RenameProject("Again".into())).unwrap();
    assert_eq!(app.undo_history().saved.map(|s| s.position), Some(4));
}

#[test]
fn req_ui_22_the_history_starts_at_the_project_as_opened() {
    let app = project_with_operation("start");
    let dir = app.location().to_path_buf();
    app.save().unwrap();
    let reopened = Application::open(&dir, Threads::Exactly(1)).unwrap();
    let history = reopened.undo_history();
    assert_eq!(history.start, "Opened project");
    assert!(history.steps.is_empty());
    assert_eq!(history.position, 0);
    let created = open_app("created");
    assert_eq!(created.undo_history().start, "Created project");
}

#[test]
fn req_ui_22_project_settings_and_several_items_are_named_too() {
    let mut app = project_with_operation("settings");
    let second = OperationId::from_u128(5);
    let mut copy = app.project().operation(OP).unwrap().clone();
    copy.id = second;
    copy.name = "Finishing".into();
    app.execute(Command::Add {
        item: Item::Operation(copy),
        placement: Placement::End,
    })
    .unwrap();
    for c in [
        Command::SetWarnOnSelectionReset(false),
        Command::SetSkipStockSimulation(true),
        Command::SetSkipStockSimulation(false),
        Command::SetPathsShown {
            operations: vec![OP, second],
            shown: false,
        },
        Command::SetModelUnits {
            model: MODEL,
            units: Some(openchuck_app::model::LengthUnit::Inch),
        },
        // Several commands on different items: the first, and how many more.
        Command::Batch(vec![
            Command::SetOperationEnabled {
                operation: OP,
                enabled: false,
            },
            Command::SetModelUnits {
                model: MODEL,
                units: None,
            },
        ]),
        Command::Batch(vec![
            Command::RenameProject("A".into()),
            Command::RenameProject("B".into()),
        ]),
    ] {
        app.execute(c).unwrap();
    }
    let all = labels(&app);
    assert_eq!(
        &all[all.len() - 7..],
        [
            "Warn before a selection is reset: off",
            "Turn off the stock simulation",
            "Turn on the stock simulation",
            "Hide all toolpaths",
            "Read model \"shaft.stl\" in inches",
            "Turn off operation \"Roughing\" and 1 more",
            "Rename project to \"A\" and 1 more",
        ]
    );
}
