// SPDX-License-Identifier: AGPL-3.0-or-later
//! A click on empty space in the project panel clears the selection
//! (issue #925; the owner's answers Q1 A and Q2 A): no item is selected
//! and the inspector shows what it shows when a project opens. A click on a
//! row still chooses it; while a context menu is open or a text field has
//! the keyboard the click only ends that; Esc does not clear the selection.

#![allow(clippy::unwrap_used)]

use eframe::egui::{self, Event, Key, Modifiers, PointerButton};

use super::frame_tests::Rig;
use super::{Selection, TreeAction};

/// The inspector's words with nothing selected, as when a project opens.
const NOTHING_CHOSEN: &str = "Choose an item in the project panel to see it here.";

/// The empty space below the tree's last row, as the tree last drew it.
fn empty_space(rig: &Rig) -> egui::Rect {
    rig.ctx
        .data(|d| d.get_temp::<egui::Rect>(egui::Id::unique("tree empty space")))
        .expect("the tree has empty space below its rows")
}

/// A rig with the cube project open and its first operation selected.
fn with_operation_selected(name: &str) -> (Rig, Selection) {
    let mut rig = Rig::cube(name, None);
    rig.settle();
    let id = rig.shell.app.as_ref().unwrap().project().operations()[0].id;
    rig.shell.tree_action(TreeAction::Select(id));
    rig.settle();
    let selected = Selection::Operation(id);
    assert_eq!(rig.shell.selected, Some(selected));
    assert!(!rig.shows(NOTHING_CHOSEN));
    (rig, selected)
}

/// The centre of the project panel's row whose text ends with `name`.
fn row_centre(rig: &Rig, name: &str) -> egui::Pos2 {
    let panel = empty_space(rig);
    rig.text_rects()
        .into_iter()
        .find(|(t, r)| t.ends_with(name) && r.right() <= panel.right())
        .unwrap_or_else(|| panic!("no row {name:?} in the project panel"))
        .1
        .center()
}

#[test]
fn req_ui_06_a_click_on_empty_tree_space_clears_the_selected_operation() {
    let (mut rig, _) = with_operation_selected("tree-clear-operation");
    let space = empty_space(&rig);
    assert!(space.height() > 20.0, "{space:?}");
    rig.click(space.center());
    rig.settle();
    assert_eq!(rig.shell.selected, None);
    assert!(
        rig.shows(NOTHING_CHOSEN),
        "the inspector shows what it shows at open"
    );
    // A click on a row still chooses it.
    let name = rig.shell.app.as_ref().unwrap().project().operations()[0]
        .name
        .clone();
    rig.click(row_centre(&rig, &name));
    rig.settle();
    assert!(matches!(rig.shell.selected, Some(Selection::Operation(_))));
    std::fs::remove_dir_all(&rig.dirs[0]).ok();
}

#[test]
fn req_ui_06_a_click_on_empty_tree_space_clears_a_selected_model_or_setup() {
    let mut rig = Rig::cube("tree-clear-model-setup", None);
    rig.settle();
    let project = rig.shell.app.as_ref().unwrap().project();
    let (model, setup) = (project.models()[0].id, project.setups()[0].id);
    for action in [
        TreeAction::SelectModel(model, None),
        TreeAction::SelectSetup(setup, None),
    ] {
        rig.shell.tree_action(action);
        rig.settle();
        assert!(rig.shell.selected.is_some());
        rig.click(empty_space(&rig).center());
        rig.settle();
        assert_eq!(rig.shell.selected, None);
        assert!(rig.shows(NOTHING_CHOSEN));
    }
    std::fs::remove_dir_all(&rig.dirs[0]).ok();
}

#[test]
fn req_ui_06_while_a_context_menu_is_open_the_click_only_closes_it() {
    let (mut rig, selected) = with_operation_selected("tree-clear-menu");
    let name = rig.shell.app.as_ref().unwrap().project().operations()[0]
        .name
        .clone();
    let at = row_centre(&rig, &name);
    let button = |pressed| Event::PointerButton {
        pos: at,
        button: PointerButton::Secondary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    rig.frame(vec![Event::PointerMoved(at)]);
    rig.frame(vec![button(true)]);
    rig.frame(vec![button(false)]);
    rig.settle();
    assert!(egui::Popup::is_any_open(&rig.ctx), "the row's menu is open");
    rig.click(empty_space(&rig).center());
    rig.settle();
    assert!(
        !egui::Popup::is_any_open(&rig.ctx),
        "the click closes the menu"
    );
    assert_eq!(rig.shell.selected, Some(selected), "and only that");
    std::fs::remove_dir_all(&rig.dirs[0]).ok();
}

#[test]
fn req_ui_06_req_ui_14_while_a_text_field_has_the_keyboard_the_click_only_ends_the_edit() {
    let (mut rig, selected) = with_operation_selected("tree-clear-text");
    let inspector = rig.size.x - 340.0;
    let field = rig
        .entry_rects()
        .into_iter()
        .find(|r| r.left() >= inspector && r.width() >= 100.0 && r.height() < 24.0)
        .expect("the operation's inspector has a text box");
    rig.click(field.center());
    rig.settle();
    assert!(rig.ctx.text_edit_focused(), "the box has the keyboard");
    rig.click(empty_space(&rig).center());
    rig.settle();
    assert!(!rig.ctx.text_edit_focused(), "the click ends the edit");
    assert_eq!(rig.shell.selected, Some(selected), "and only that");
    std::fs::remove_dir_all(&rig.dirs[0]).ok();
}

#[test]
fn req_ui_14_esc_does_not_clear_the_selection() {
    let (mut rig, selected) = with_operation_selected("tree-clear-esc");
    let key = |pressed| Event::Key {
        key: Key::Escape,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    rig.frame(vec![key(true), key(false)]);
    rig.settle();
    assert_eq!(rig.shell.selected, Some(selected));
    std::fs::remove_dir_all(&rig.dirs[0]).ok();
}
