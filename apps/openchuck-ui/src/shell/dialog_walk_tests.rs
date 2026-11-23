// SPDX-License-Identifier: AGPL-3.0-or-later
//! Every dialog and tool window with a text field, walked (issue #871):
//! typing in its field and then using the dialog's own buttons, its close
//! box or Esc never asks the Unconfirmed changes question, whose subject is
//! the project's unconfirmed edits (decisions 0233 and 0246). A dialog added
//! later joins the walk ([`req_ui_07_every_dialog_with_a_field_is_walked`]).

#![allow(clippy::unwrap_used)]

use eframe::egui::{self, Event, Key, Modifiers};
use openchuck_app::overview::ItemRef;

use super::frame_tests::Rig;
use super::{Dialog, Shell};

const QUESTION: &str = "Unconfirmed changes";

/// How a walk leaves the dialog.
#[derive(Debug, Clone, Copy)]
enum Leave {
    /// A button of the dialog, by its text.
    Button(&'static str),
    /// The close box in the dialog's title bar.
    CloseBox,
    /// Esc, twice: the first ends the edit in the field.
    Esc,
}

/// One dialog with a text field.
struct Case {
    name: &'static str,
    /// The floating window's key, for one with a close box.
    window: Option<&'static str>,
    open: fn(&mut Shell),
    /// Gives the field the keyboard.
    focus: fn(&mut Rig),
    /// Whether the dialog is still open.
    shown: fn(&Shell) -> bool,
    /// The ways out to walk, each from a fresh dialog.
    leaves: &'static [Leave],
}

fn frames(rig: &mut Rig, n: usize) {
    for _ in 0..n {
        rig.frame(Vec::new());
    }
}

fn click_text(rig: &mut Rig, text: &str) {
    let at = rig
        .visible_text_rects()
        .into_iter()
        .rev()
        .find(|(label, _)| label == text)
        .map(|(_, rect)| rect.center())
        .unwrap_or_else(|| panic!("{text} is visible: {:?}", rig.texts()));
    rig.click(at);
    frames(rig, 3);
}

/// Clicks the text that starts with `start` (a field's hint).
fn click_starting(rig: &mut Rig, start: &str) {
    let at = rig
        .visible_text_rects()
        .into_iter()
        .find(|(label, _)| label.starts_with(start))
        .map(|(_, rect)| rect.center())
        .unwrap_or_else(|| panic!("{start}… is visible: {:?}", rig.texts()));
    rig.click(at);
    frames(rig, 1);
    rig.click(at);
    frames(rig, 1);
}

/// Clicks the entry box nearest below or beside the text `label`.
fn click_entry_near(rig: &mut Rig, label: &str) {
    let row = rig
        .visible_text_rects()
        .into_iter()
        .find(|(text, _)| text == label)
        .map(|(_, rect)| rect)
        .unwrap_or_else(|| panic!("{label} is visible: {:?}", rig.texts()));
    let near =
        |r: &egui::Rect| r.bottom() > row.top() && r.top() < row.bottom() + 40.0 && *r != row;
    let distance = |r: &egui::Rect| (r.center() - row.center()).length();
    // A dialog may recolour its boxes; then the box's painted text marks it.
    let at = rig
        .entry_rects()
        .into_iter()
        .filter(|r| r.width() > 60.0 && near(r))
        .min_by(|a, b| distance(a).total_cmp(&distance(b)))
        .or_else(|| {
            rig.visible_text_rects()
                .into_iter()
                .filter(|(t, r)| {
                    near(r) && r.top() > row.bottom() - 2.0 && !t.ends_with('\u{2026}')
                })
                .map(|(_, r)| r)
                .min_by(|a, b| distance(a).total_cmp(&distance(b)))
        })
        .unwrap_or_else(|| panic!("{label} has an entry box: {:?}", rig.texts()))
        .center();
    rig.click(at);
    frames(rig, 1);
    rig.click(at);
    frames(rig, 1);
}

/// Clicks midway between the texts `left` and `right` of one row: an empty
/// box drawn between its label and its button.
fn click_between(rig: &mut Rig, left: &str, right: &str) {
    let find = |text: &str| {
        rig.visible_text_rects()
            .into_iter()
            .find(|(t, _)| t == text)
            .map(|(_, r)| r)
            .unwrap_or_else(|| panic!("{text} is visible: {:?}", rig.texts()))
    };
    let (a, b) = (find(left), find(right));
    let at = egui::pos2((a.right() + b.left()) / 2.0, a.center().y);
    rig.click(at);
    frames(rig, 1);
    rig.click(at);
    frames(rig, 1);
}

/// The first Tab in a dialog that has just opened reaches its first field
/// (decision 0677 item 1).
fn first_tab(rig: &mut Rig) {
    rig.press(Key::Tab, Modifiers::NONE);
    frames(rig, 1);
}

fn close_box(rig: &mut Rig, key: &str) {
    let rect = rig
        .ctx
        .memory(|m| m.area_rect(egui::Id::unique(("dialog", key))))
        .unwrap_or_else(|| panic!("{key} is drawn"));
    let height = rig.ctx.global_style().spacing.interact_size.y;
    rig.click(egui::pos2(
        rect.right() - 8.0 - height / 2.0,
        rect.top() + 6.0 + height / 2.0,
    ));
    frames(rig, 3);
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            name: "Import model",
            window: Some("import model"),
            open: Shell::import_model,
            focus: |rig| click_starting(rig, "Path of a model file"),
            shown: |shell| shell.import.is_some(),
            leaves: &[
                Leave::Button("Cancel"),
                Leave::CloseBox,
                Leave::Esc,
                Leave::Button("Check and preview"),
            ],
        },
        Case {
            name: "New operation",
            window: Some("add operation"),
            open: Shell::new_operation,
            focus: first_tab,
            shown: |shell| shell.run.is_adding(),
            leaves: &[Leave::Button("Cancel"), Leave::CloseBox, Leave::Esc],
        },
        Case {
            name: "Rename",
            window: None,
            open: |shell| {
                let op = shell.app.as_ref().unwrap().project().operations()[0].id;
                shell.begin_rename(ItemRef::Operation(op));
            },
            // The name has the keyboard when the dialog opens.
            focus: |_| {},
            shown: |shell| matches!(shell.dialog, Some(Dialog::Rename { .. })),
            leaves: &[Leave::Button("Cancel"), Leave::Button("Rename")],
        },
        Case {
            name: "Open project",
            window: None,
            open: |shell| {
                shell.dialog = Some(Dialog::Folder {
                    create: false,
                    folder: String::new(),
                    name: String::new(),
                });
            },
            focus: first_tab,
            shown: |shell| matches!(shell.dialog, Some(Dialog::Folder { .. })),
            leaves: &[Leave::Button("Cancel")],
        },
        Case {
            name: "Post and export",
            window: Some("export dialog"),
            open: Shell::open_export,
            focus: |rig| click_between(rig, "Output folder", "Choose\u{2026}"),
            shown: |shell| shell.export.is_open(),
            leaves: &[Leave::Button("Cancel"), Leave::CloseBox, Leave::Esc],
        },
    ]
}

/// Opens `case` on a fresh project, types into its field without Enter, and
/// leaves by `leave`. Returns what went wrong, if anything.
fn walk(case: &Case, leave: Leave, index: usize) -> Option<String> {
    let mut rig = Rig::cube(&format!("dialog-walk-{index}"), None);
    rig.size = egui::vec2(1920.0, 1080.0);
    (case.open)(&mut rig.shell);
    frames(&mut rig, 4);
    if !(case.shown)(&rig.shell) {
        return Some(format!("{}: did not open", case.name));
    }
    (case.focus)(&mut rig);
    rig.press(Key::A, Modifiers::COMMAND);
    rig.frame(vec![Event::Text("typed but not confirmed".to_owned())]);
    frames(&mut rig, 1);
    if crate::pending_edits::entries(&rig.ctx).is_empty() {
        return Some(format!(
            "{}: the field holds no unconfirmed edit",
            case.name
        ));
    }
    match leave {
        Leave::Button(text) => click_text(&mut rig, text),
        Leave::CloseBox => close_box(&mut rig, case.window.unwrap()),
        Leave::Esc => {
            rig.press(Key::Escape, Modifiers::NONE);
            frames(&mut rig, 2);
            rig.press(Key::Escape, Modifiers::NONE);
            frames(&mut rig, 3);
        }
    }
    frames(&mut rig, 3);
    rig.shows(QUESTION)
        .then(|| format!("{} {leave:?}: asks {QUESTION}", case.name))
}

/// Issue #871: none of the walks asks the question.
#[test]
fn req_ui_07_req_ui_14_a_dialogs_own_ways_out_never_ask_about_unconfirmed_changes() {
    let mut found = Vec::new();
    let mut index = 0;
    for case in cases() {
        for leave in case.leaves {
            index += 1;
            found.extend(walk(&case, *leave, index));
        }
    }
    assert!(found.is_empty(), "{found:#?}");
}

/// Every dialog, tool window and modal question the application draws, by
/// source file: how many, and which of them [`cases`] walks. A file that
/// draws a dialog not listed here fails, so a new dialog is either walked
/// (it has a field the pending-edit guard tracks) or listed with why it is
/// not (issue #871).
const DIALOGS: &[(&str, usize, &str)] = &[
    (
        "shell.rs",
        1,
        "the shell's dialogs (Dialog enum): Open project and Rename walked; \
         New project's buttons resolve their own fields (decision 0246); \
         Turn on adaptive spacing's field is not tracked; the rest have no field",
    ),
    ("import_panel.rs", 1, "Import model: walked"),
    ("run_panel.rs", 1, "New operation: walked"),
    ("export_panel.rs", 1, "Post and export: walked"),
    (
        "export_panel/feedback.rs",
        1,
        "the Replace files question: no field",
    ),
    (
        "tool_library_window.rs",
        2,
        "the tool library and its questions: library drafts ask their own \
         Discard the changes? (decision 0127)",
    ),
    (
        "machine_library_window.rs",
        2,
        "the machine library and its question: library drafts ask their own \
         Discard the changes? (decision 0127)",
    ),
    (
        "selection_panel.rs",
        2,
        "Clear N faces? and Start a new selection?: no field",
    ),
    ("model_panel.rs", 1, "the Mesh check report: no field"),
    (
        "shell/pending.rs",
        2,
        "the Unconfirmed changes question itself and its working note",
    ),
    (
        "shell/checkpoints.rs",
        3,
        "the recovery dialogs: closing, choices (no field), and Locate moved \
         project, whose folder field is not tracked",
    ),
];

#[test]
fn req_ui_07_every_dialog_with_a_field_is_walked() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let name = path
                .strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if !name.ends_with(".rs") || name.ends_with("_tests.rs") || name == "floating.rs" {
                continue;
            }
            let text: String = std::fs::read_to_string(&path)
                .unwrap()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            let count = [
                "egui::Modal::new(",
                "egui::Window::new(",
                "floating_fixed(",
                "floating_unscrolled(",
            ]
            .iter()
            .map(|p| text.matches(p).count())
            .sum::<usize>();
            if count > 0 {
                found.push((name, count));
            }
        }
    }
    found.sort();
    let mut listed: Vec<_> = DIALOGS
        .iter()
        .map(|(file, count, _)| ((*file).to_owned(), *count))
        .collect();
    listed.sort();
    assert_eq!(
        found, listed,
        "a dialog was added or removed: walk it in cases() or list why not in DIALOGS"
    );
}

/// The walk case of each of the shell's dialogs, or why it has none. The
/// match has no wildcard: a dialog added to the enum does not build until it
/// is placed here (issue #871).
fn walked(dialog: &Dialog) -> Result<&'static str, &'static str> {
    match dialog {
        Dialog::Folder { create: false, .. } => Ok("Open project"),
        Dialog::Folder { create: true, .. } => {
            Err("its OK and Cancel resolve their own fields (decision 0246)")
        }
        Dialog::Rename { .. } => Ok("Rename"),
        Dialog::AdaptiveHeight { .. } => Err("its field is not tracked by the guard"),
        Dialog::FolderExists { .. }
        | Dialog::OfferCreate { .. }
        | Dialog::Unsaved(_)
        | Dialog::Keys
        | Dialog::About
        | Dialog::Settings
        | Dialog::Error(_)
        | Dialog::CopyLinked(_) => Err("no field"),
    }
}

#[test]
fn req_ui_07_the_shells_dialogs_with_a_field_are_walked() {
    let names: Vec<_> = cases().iter().map(|c| c.name).collect();
    for dialog in [
        Dialog::Folder {
            create: false,
            folder: String::new(),
            name: String::new(),
        },
        Dialog::Rename {
            item: ItemRef::Model(openchuck_app::project::id::ModelId::new()),
            name: String::new(),
        },
    ] {
        let name = walked(&dialog).unwrap();
        assert!(names.contains(&name), "{name} is walked");
    }
}
