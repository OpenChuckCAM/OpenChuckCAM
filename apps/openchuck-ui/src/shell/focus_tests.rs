// SPDX-License-Identifier: AGPL-3.0-or-later
//! Keyboard focus in the shell (UI-14; specification section 16): a tool
//! window that opens takes the keyboard (issue #580), and a control that Tab
//! reaches is scrolled into view (issue #577). Drawn whole without a window.

#![allow(clippy::unwrap_used)]

use eframe::egui::{self, Key, Modifiers};

use super::frame_tests::Rig;
use super::*;
use crate::machine_library_window::MachineLibraryWindow;
use crate::tool_library_window::{Purpose, ToolLibraryWindow};

impl Rig {
    /// The widget with the keyboard, with its rectangle and layer.
    pub(super) fn focused(&self) -> Option<egui::Response> {
        self.focused.clone()
    }

    /// Presses Tab (or Shift Tab) and draws two frames after it: egui makes a
    /// scroll asked for in one frame when the next starts.
    pub(super) fn tab(&mut self, back: bool) {
        let modifiers = if back {
            Modifiers::SHIFT
        } else {
            Modifiers::NONE
        };
        self.press(Key::Tab, modifiers);
        self.frame(Vec::new());
        self.frame(Vec::new());
    }
}

/// The five tool windows, each opened as the user opens it.
fn open_each(rig: &mut Rig, window: &str) {
    match window {
        "export dialog" => rig.press(Key::E, Modifiers::COMMAND),
        "import model" => rig.press(Key::I, Modifiers::COMMAND),
        "add operation" => {
            rig.press(Key::Num3, Modifiers::ALT);
            rig.press(Key::N, Modifiers::NONE);
        }
        "tool library" => rig.shell.open_tool_library(Purpose::Add, None),
        "machine library" => rig.shell.open_machine_library(),
        _ => unreachable!("{window}"),
    }
}

const WINDOWS: [&str; 5] = [
    "export dialog",
    "import model",
    "add operation",
    "tool library",
    "machine library",
];

/// Issue #580: a tool window that opens takes the keyboard. Its first
/// control has focus at once, and Tab moves on inside the window, not to the
/// File menu behind it.
#[test]
fn req_ui_14_a_new_window_takes_the_keyboard() {
    for window in WINDOWS {
        let mut rig = Rig::cube(&format!("focus-open-{}", window.replace(' ', "-")), None);
        let dir =
            super::frame_tests::temp(&format!("focus-open-libs-{}", window.replace(' ', "-")));
        rig.shell.tool_library = ToolLibraryWindow::new(Some(dir.join("tools")));
        rig.shell.machine_library =
            MachineLibraryWindow::new(Some(dir.join("machines")), Some(dir.join("tools")));
        rig.dirs.push(dir);
        // Focus starts in the main window, on its first control.
        rig.tab(false);
        let layer = crate::floating::window_layer(window);
        assert_ne!(rig.focused().map(|r| r.layer_id), Some(layer), "{window}");
        open_each(&mut rig, window);
        for _ in 0..4 {
            rig.frame(Vec::new());
        }
        let first = rig
            .focused()
            .unwrap_or_else(|| panic!("{window}: nothing has focus"));
        assert_eq!(
            first.layer_id, layer,
            "{window}: focus stayed behind the window"
        );
        rig.tab(false);
        let next = rig
            .focused()
            .unwrap_or_else(|| panic!("{window}: Tab lost the focus"));
        assert_eq!(next.layer_id, layer, "{window}: Tab left the window");
        assert_ne!(next.id, first.id, "{window}: Tab did not move");
    }
}

/// The letter keys of specification section 16 are left only to a text field
/// with the keyboard: a button or menu that Tab has reached does not take
/// them, so a keyboard user can still press N, G, F2 or Del (UI-14).
#[test]
fn req_ui_14_letter_keys_work_while_a_button_has_the_keyboard() {
    let mut rig = Rig::cube("focus-letter-keys", None);
    rig.press(Key::Num3, Modifiers::ALT);
    rig.tab(false);
    let focused = rig.focused().expect("Tab reached a control");
    assert!(
        !rig.ctx.text_edit_focused(),
        "{focused:?} is a button, not a text field"
    );
    rig.press(Key::N, Modifiers::NONE);
    rig.frame(Vec::new());
    rig.frame(Vec::new());
    assert!(rig.shows("Strategy"), "N opened the New operation dialog");
}

/// Presses Tab until focus comes back round (or `max` presses), and returns
/// every control it stopped on that was not wholly in view, with where it was.
fn hidden_stops(rig: &mut Rig, max: usize) -> Vec<String> {
    let mut hidden = Vec::new();
    let mut seen = Vec::new();
    for _ in 0..max {
        rig.tab(false);
        let Some(focused) = rig.focused() else {
            continue;
        };
        if seen.contains(&focused.id) {
            break;
        }
        seen.push(focused.id);
        // egui cuts a widget's interact rectangle to what its clip shows.
        let shown = focused.interact_rect;
        let whole = focused.rect;
        // A pixel or two lost at a frame's edge is not a hidden control.
        let cut = shown.height() < whole.height() - 2.0 || shown.width() < whole.width() - 2.0;
        if cut && whole.area() > 0.0 {
            hidden.push(format!(
                "{:?} shown {shown:?} of {whole:?}",
                focused.layer_id
            ));
        }
    }
    hidden
}

/// Issue #577: Tab never leaves the keyboard on a control the user cannot
/// see. At 1366 by 768 the inspector of an operation, the export dialog (with
/// its acknowledgement at the foot) and the machine library are taller than
/// the window; each control Tab reaches is scrolled into view.
#[test]
fn req_ui_14_tab_scrolls_the_focused_control_into_view() {
    let mut rig = Rig::cube("focus-scroll", None);
    rig.size = egui::vec2(1366.0, 768.0);
    rig.press(Key::Num3, Modifiers::ALT);
    let op = rig.shell.app.as_ref().unwrap().project().operations()[0].id;
    rig.shell.select(Selection::Operation(op));
    rig.settle();
    let hidden = hidden_stops(&mut rig, 400);
    assert!(hidden.is_empty(), "main window: {hidden:#?}");

    rig.press(Key::E, Modifiers::COMMAND);
    rig.frame(Vec::new());
    assert!(rig.shell.export.is_open());
    let hidden = hidden_stops(&mut rig, 200);
    assert!(hidden.is_empty(), "export dialog: {hidden:#?}");
    rig.shell.export = crate::export_panel::ExportPanel::new(None);
    rig.frame(Vec::new());

    let dir = super::frame_tests::temp("focus-scroll-libs");
    rig.shell.machine_library =
        MachineLibraryWindow::new(Some(dir.join("machines")), Some(dir.join("tools")));
    rig.dirs.push(dir);
    rig.shell.open_machine_library();
    rig.frame(Vec::new());
    let hidden = hidden_stops(&mut rig, 300);
    assert!(hidden.is_empty(), "machine library: {hidden:#?}");
}

/// Issue #578 and UI-10: a step out of reach says why, and a keyboard user
/// reads it by Tab: the step is a Tab stop whose tooltip opens on focus.
/// Enter on it does not open it.
#[test]
fn req_ui_10_req_ui_14_an_unavailable_steps_reason_opens_on_focus() {
    let mut rig = Rig::new(Shell::with_settings(false, None, None), Vec::new());
    let reason = |rig: &Rig| {
        rig.texts()
            .into_iter()
            .find(|(t, _)| t.starts_with("Setup: "))
            .map(|(t, _)| t)
    };
    assert_eq!(reason(&rig), None, "no tooltip before Tab");
    let mut found = None;
    for _ in 0..60 {
        rig.tab(false);
        if let Some(text) = reason(&rig) {
            found = Some(text);
            break;
        }
    }
    let text = found.expect("Tab reached the Setup step and showed why it is out of reach");
    assert!(text.len() > "Setup: ".len(), "{text}");
    rig.press(Key::Enter, Modifiers::NONE);
    rig.frame(Vec::new());
    assert_eq!(rig.shell.step, openchuck_app::overview::Step::Model);
}

/// Issue #581 F7: Help > Keyboard shortcuts names every key of specification
/// section 16 that the application has, so a keyboard user can find them.
#[test]
fn req_ui_14_the_shortcut_list_names_every_key() {
    let keys: Vec<&str> = super::frame::KEY_MAP.iter().map(|(k, _)| *k).collect();
    let listed = |key: &str| keys.iter().any(|k| k.split(", ").any(|part| part == key));
    for key in [
        "Ctrl N",
        "Ctrl O",
        "Ctrl S",
        "Ctrl I",
        "Ctrl E",
        "Ctrl Z",
        "Ctrl Y",
        "Ctrl Shift Z",
        "Alt 5",
        "Ctrl 1",
        "Ctrl 2",
        "Ctrl 3",
        "Ctrl +",
        "Ctrl −",
        "F",
        "V",
        "N",
        "Ctrl D",
        "Del",
        "F2",
        "Alt ↑",
        "Alt ↓",
        "Ctrl Enter",
        "Space",
        "←",
        "→",
        "Shift ←",
        "Shift →",
        "Esc",
        "Tab",
        "Shift Tab",
    ] {
        assert!(listed(key), "{key} is not in the list: {keys:?}");
    }
}

impl Rig {
    /// Clicks the text painted exactly `label`.
    fn click_label(&mut self, label: &str) {
        let at = self
            .text_rects()
            .iter()
            .find(|(t, _)| t == label)
            .map(|(_, r)| r.center())
            .unwrap_or_else(|| panic!("{label} is painted"));
        self.click(at);
        self.frame(Vec::new());
    }
}

/// Issue #581 F8 (UI-14, "every command is available from the menus"): Add
/// setup is in the Project menu, and Edit > Delete deletes a selected
/// selection set as the Del key does.
#[test]
fn req_ui_14_every_command_is_in_a_menu() {
    use openchuck_app::project::{ItemId, SelectionTarget};
    let mut rig = Rig::cube("focus-menus", None);
    let app = rig.shell.app.as_mut().unwrap();
    let project = app.project();
    let removes = project
        .operations()
        .iter()
        .map(|o| Command::Remove(ItemId::Operation(o.id)))
        .chain(
            project
                .setups()
                .iter()
                .map(|s| Command::Remove(ItemId::Setup(s.id))),
        )
        .collect();
    app.execute(Command::Batch(removes)).unwrap();
    assert!(app.project().setups().is_empty());
    rig.settle();
    rig.click_label("Project");
    rig.click_label("Add setup");
    assert_eq!(rig.shell.app.as_ref().unwrap().project().setups().len(), 1);

    let app = rig.shell.app.as_mut().unwrap();
    let model = app.project().models()[0].id;
    let set = app
        .save_selection("Rib", model, SelectionTarget::Triangles([1].into()))
        .unwrap();
    rig.settle();
    rig.shell.select(Selection::Set(set));
    rig.frame(Vec::new());
    rig.click_label("Edit");
    rig.click_label("Delete");
    let sets = rig
        .shell
        .app
        .as_ref()
        .unwrap()
        .project()
        .selection_sets()
        .len();
    assert_eq!(sets, 0, "the set was deleted");
}

impl Rig {
    /// Whether a rectangle filled `fill` is painted under `at`, at whatever
    /// opacity (a window fades in over its first frames).
    fn filled_at(&self, at: egui::Pos2, fill: egui::Color32) -> bool {
        fn same(painted: egui::Color32, fill: egui::Color32) -> bool {
            let painted = painted.to_opaque();
            painted.a() > 0
                && [
                    (painted.r(), fill.r()),
                    (painted.g(), fill.g()),
                    (painted.b(), fill.b()),
                ]
                .iter()
                .all(|(a, b)| a.abs_diff(*b) <= 3)
        }
        fn walk(shape: &egui::Shape, at: egui::Pos2, fill: egui::Color32) -> bool {
            match shape {
                egui::Shape::Rect(r) => r.fill.a() > 0 && same(r.fill, fill) && r.rect.contains(at),
                egui::Shape::Vec(v) => v.iter().any(|s| walk(s, at, fill)),
                _ => false,
            }
        }
        self.shapes().iter().any(|s| walk(&s.shape, at, fill))
    }
}

/// Issue #581 F10 (specification section 5, "Dialogs follow the rule"): the
/// New operation dialog's "Add" is its one filled button, bottom right, with
/// Cancel to its left; Tab reaches Cancel before Add (reading order).
#[test]
fn req_ui_10_the_new_operation_dialog_has_its_primary_action_bottom_right() {
    let mut rig = Rig::cube("focus-new-operation", None);
    rig.press(Key::Num3, Modifiers::ALT);
    rig.press(Key::N, Modifiers::NONE);
    for _ in 0..3 {
        rig.frame(Vec::new());
    }
    let rects = rig.text_rects();
    let find = |label: &str| {
        rects
            .iter()
            .find(|(t, _)| t == label)
            .map(|(_, r)| *r)
            .unwrap_or_else(|| panic!("{label} is painted"))
    };
    let add = find("Add");
    let cancel = find("Cancel");
    assert!(
        add.left() > cancel.right(),
        "Add {add:?} right of Cancel {cancel:?}"
    );
    let title = find("Add operation");
    assert!(
        add.top() > title.bottom() + 300.0,
        "Add is at the foot: {add:?}"
    );
    // Right-most: nothing painted further right in the dialog's row.
    assert!(
        rects
            .iter()
            .filter(|(_, r)| (r.center().y - add.center().y).abs() < 4.0)
            .all(|(_, r)| r.right() <= add.right() + 0.5),
        "Add is right-most"
    );
    assert!(
        rig.filled_at(add.center(), crate::theme::DARK.primary),
        "Add is filled"
    );
    // Tab order: Cancel, then Add.
    let mut order = Vec::new();
    for _ in 0..40 {
        rig.tab(false);
        if let Some(r) = rig.focused() {
            if r.rect.contains(cancel.center()) {
                order.push("Cancel");
            } else if r.rect.contains(add.center()) {
                order.push("Add");
            }
        }
        if order.len() == 2 {
            break;
        }
    }
    assert_eq!(order, ["Cancel", "Add"]);
}
