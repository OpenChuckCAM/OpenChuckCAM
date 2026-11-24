// SPDX-License-Identifier: AGPL-3.0-or-later
//! The undo history popover (UI-22; specification section 12.3; decision
//! PROJUX-B-1), through real frames.
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

use eframe::egui::{Key, Modifiers};
use openchuck_app::Command;

use super::frame_tests::Rig;
use super::history::ROW_CLICK;

/// The cube project with two more steps: renamed to "First", then "Second".
fn rig(name: &str) -> Rig {
    let mut rig = Rig::cube(name, None);
    let app = rig.shell.app.as_mut().unwrap();
    app.execute(Command::RenameProject("First".into())).unwrap();
    app.execute(Command::RenameProject("Second".into()))
        .unwrap();
    rig.settle();
    rig
}

/// Where the text `needle` is painted, the first time it is.
fn at(rig: &Rig, needle: &str) -> eframe::egui::Rect {
    rig.text_rects()
        .into_iter()
        .find(|(t, _)| t == needle)
        .unwrap_or_else(|| panic!("'{needle}' is not drawn"))
        .1
}

fn position(rig: &Rig) -> usize {
    rig.shell.app.as_ref().unwrap().undo_history().position
}

#[test]
fn req_ui_22_req_ui_14_ctrl_h_lists_the_steps_newest_first() {
    let mut rig = rig("history-list");
    assert!(!rig.shows("UNDO HISTORY"));
    rig.press(Key::H, Modifiers::COMMAND);
    rig.frame(Vec::new());
    assert!(rig.shows("UNDO HISTORY"));
    let second = at(&rig, "Rename project to \"Second\"");
    let first = at(&rig, "Rename project to \"First\"");
    assert!(second.top() < first.top(), "newest first");
    // The project as opened is the last row.
    let start = at(&rig, "Opened project");
    assert!(first.top() < start.top());
    assert!(rig.shows("A new edit after undoing discards the undone steps."));
    // The current step has the keyboard, so Enter or Tab work from it.
    let focused = rig.focused.as_ref().expect("a row has the keyboard");
    assert!(focused.rect.contains(second.center()));
    // Esc closes it.
    rig.press(Key::Escape, Modifiers::NONE);
    rig.frame(Vec::new());
    assert!(!rig.shows("UNDO HISTORY"));
}

#[test]
fn req_ui_22_choosing_a_row_goes_back_to_it_and_forward_again() {
    let mut rig = rig("history-jump");
    let total = rig.shell.app.as_ref().unwrap().undo_history().steps.len();
    rig.press(Key::H, Modifiers::COMMAND);
    rig.frame(Vec::new());
    rig.click(at(&rig, "Opened project").center());
    rig.settle();
    assert_eq!(position(&rig), 0);
    // The steps stay listed, marked undone, and the list stays open.
    assert!(rig.shows("UNDO HISTORY"));
    assert!(rig.shows("undone"));
    rig.click(at(&rig, "Rename project to \"First\"").center());
    rig.settle();
    assert_eq!(position(&rig), total - 1);
    assert_eq!(rig.shell.app.as_ref().unwrap().project().name(), "First");
}

#[test]
fn req_ui_22_the_last_save_is_marked_under_its_step() {
    let mut rig = rig("history-saved");
    assert!(rig.shell.save());
    rig.settle();
    rig.press(Key::H, Modifiers::COMMAND);
    rig.frame(Vec::new());
    let saved = at(&rig, "Saved just now");
    let second = at(&rig, "Rename project to \"Second\"");
    let first = at(&rig, "Rename project to \"First\"");
    assert!(second.top() < saved.top() && saved.top() < first.top());
}

#[test]
fn req_ui_22_req_nfr_18_a_chosen_row_is_logged_without_its_text() {
    let mut rig = rig("history-log");
    rig.press(Key::H, Modifiers::COMMAND);
    rig.frame(Vec::new());
    rig.click(at(&rig, "Rename project to \"First\"").center());
    rig.settle();
    let clicks = &rig.shell.logged_clicks;
    assert!(clicks.iter().any(|c| c == ROW_CLICK), "{clicks:?}");
    assert!(!clicks.iter().any(|c| c.contains("First")), "{clicks:?}");
}

#[test]
fn req_ui_22_the_title_bar_button_opens_and_closes_it() {
    let mut rig = rig("history-button");
    rig.click(at(&rig, "\u{2630}").center());
    rig.frame(Vec::new());
    assert!(rig.shows("UNDO HISTORY"));
    rig.click(at(&rig, "\u{2630}").center());
    rig.frame(Vec::new());
    assert!(!rig.shows("UNDO HISTORY"));
    // A click elsewhere closes it too.
    rig.click(at(&rig, "\u{2630}").center());
    rig.frame(Vec::new());
    assert!(rig.shows("UNDO HISTORY"));
    rig.click(eframe::egui::pos2(700.0, 500.0));
    rig.frame(Vec::new());
    assert!(!rig.shows("UNDO HISTORY"));
}

#[test]
fn req_ui_08_req_ui_22_the_title_bar_shows_the_name_undo_redo_and_history() {
    // The menus no longer take the whole title bar: the project's name, its
    // save state, Undo, Redo and the history are on screen, inside the
    // window, in that order (specification section 3).
    let mut rig = rig("history-title-bar");
    rig.frame(Vec::new());
    let width = rig.size.x;
    let name = at(&rig, "Second");
    let help = at(&rig, "Help");
    let undo = at(&rig, "\u{21b6}");
    let redo = at(&rig, "\u{21b7}");
    let history = rig
        .text_rects()
        .into_iter()
        .filter(|(t, r)| t == "\u{2630}" && r.top() < 32.0)
        .map(|(_, r)| r)
        .next()
        .expect("the history button is in the title bar");
    assert!(help.right() < name.left());
    assert!(rig.shows("Edited, not saved"));
    assert!(name.right() < undo.left());
    assert!(undo.right() < redo.left() && redo.right() < history.left());
    assert!(history.right() <= width);
}

#[test]
fn req_ui_22_the_edit_menu_opens_it() {
    let mut rig = rig("history-menu");
    rig.click(at(&rig, "Edit").center());
    rig.frame(Vec::new());
    rig.click(at(&rig, "Undo history").center());
    rig.frame(Vec::new());
    assert!(rig.shows("UNDO HISTORY"));
}

/// The labels of the history's steps, oldest first.
fn labels(rig: &Rig) -> Vec<String> {
    rig.shell
        .app
        .as_ref()
        .unwrap()
        .undo_history()
        .steps
        .into_iter()
        .map(|s| s.label)
        .collect()
}

#[test]
fn req_ui_22_req_ui_07_a_value_typed_in_the_inspector_is_named_with_its_value() {
    // Through the real form: type, Enter, as the owner-check pack of #705
    // did; the step names the setting and the value, not "Edit operation".
    let mut rig = super::pending_edit_tests::generated("history-typed");
    let op = rig.shell.app.as_ref().unwrap().project().operations()[0]
        .name
        .clone();
    for value in ["0.5", "0.6"] {
        super::pending_edit_tests::type_unconfirmed(&mut rig, "Stepover", value);
        rig.press(Key::Enter, Modifiers::NONE);
        rig.frame(Vec::new());
    }
    let all = labels(&rig);
    assert_eq!(
        &all[all.len() - 2..],
        [
            format!("Set stepover 0.5 mm ({op})"),
            format!("Set stepover 0.6 mm ({op})"),
        ]
    );
}

#[test]
fn req_ui_22_req_ui_07_setup_values_kept_together_are_one_edit_setup_step() {
    // Keep on a setup applies its values through the setup form (decision
    // 0233); the step is "Edit setup".
    let mut rig = super::pending_edit_tests::generated("history-keep");
    let setup = rig.shell.app.as_ref().unwrap().project().setups()[0].clone();
    rig.shell.choose_setup(setup.id, None);
    rig.shell.open_step(openchuck_app::overview::Step::Setup);
    for _ in 0..5 {
        rig.frame(Vec::new());
    }
    let before = labels(&rig).len();
    super::pending_edit_tests::type_unconfirmed(&mut rig, "Diameter", "22");
    rig.shell
        .open_step(openchuck_app::overview::Step::Operations);
    for _ in 0..3 {
        rig.frame(Vec::new());
    }
    rig.click(at(&rig, "Keep").center());
    rig.settle();
    let all = labels(&rig);
    assert_eq!(all.len(), before + 1, "{all:?}");
    assert_eq!(
        all.last().map(String::as_str),
        Some(format!("Edit setup \"{}\"", setup.name).as_str())
    );
}

#[test]
fn req_ui_22_req_ui_14_the_foot_names_the_keys_the_edit_menu_shows() {
    let mut rig = rig("history-keys");
    rig.press(Key::H, Modifiers::COMMAND);
    rig.frame(Vec::new());
    let ctx = rig.ctx.clone();
    let foot = format!(
        "{} undo · {} redo. A new edit after undoing discards the undone steps.",
        ctx.format_shortcut(&super::UNDO),
        ctx.format_shortcut(&super::REDO),
    );
    assert!(rig.shows(&foot), "{:?}", rig.texts());
}
