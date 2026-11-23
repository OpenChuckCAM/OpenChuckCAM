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

/// Issue #580 (the owner's answer Q1 B): a tool window that opens takes the
/// keyboard as a whole: nothing behind it keeps focus and no control in it has
/// focus yet; the first Tab goes to its first control, not to the File menu
/// behind it, and the next Tab stays inside.
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
        assert!(
            rig.focused().is_none(),
            "{window}: {:?} has focus",
            rig.focused()
        );
        rig.tab(false);
        let first = rig
            .focused()
            .unwrap_or_else(|| panic!("{window}: the first Tab gave nothing focus"));
        assert_eq!(
            first.layer_id, layer,
            "{window}: the first Tab went behind the window"
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
        "1",
        "3",
        "7",
        "0",
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

/// Whether the tool window `window` is open.
fn is_open(rig: &Rig, window: &str) -> bool {
    match window {
        "export dialog" => rig.shell.export.is_open(),
        "import model" => rig.shell.import.is_some(),
        "add operation" => rig.shell.run.is_adding(),
        "tool library" => rig.shell.tool_library.is_open(),
        "machine library" => rig.shell.machine_library.is_open(),
        _ => unreachable!("{window}"),
    }
}

/// Issue #581 F11 (the owner's answer Q6 A): Esc does what each tool window's
/// Close or Cancel does. Esc in a text field ends the edit only, and a second
/// Esc closes; a machine library draft with changes asks first, as Close does.
#[test]
fn req_ui_14_esc_closes_every_tool_window_as_its_close_does() {
    for window in WINDOWS {
        let mut rig = Rig::cube(&format!("focus-esc-{}", window.replace(' ', "-")), None);
        let dir = super::frame_tests::temp(&format!("focus-esc-libs-{}", window.replace(' ', "-")));
        rig.shell.tool_library = ToolLibraryWindow::new(Some(dir.join("tools")));
        rig.shell.machine_library =
            MachineLibraryWindow::new(Some(dir.join("machines")), Some(dir.join("tools")));
        rig.dirs.push(dir);
        open_each(&mut rig, window);
        for _ in 0..3 {
            rig.frame(Vec::new());
        }
        assert!(is_open(&rig, window), "{window} opened");
        rig.press(Key::Escape, Modifiers::NONE);
        rig.frame(Vec::new());
        assert!(!is_open(&rig, window), "Esc closed {window}");
    }

    // Esc in a text field: the import window's first control is its file field.
    let mut rig = Rig::cube("focus-esc-field", None);
    open_each(&mut rig, "import model");
    rig.frame(Vec::new());
    rig.tab(false);
    assert!(rig.ctx.text_edit_focused(), "Tab reached the file field");
    rig.press(Key::Escape, Modifiers::NONE);
    rig.frame(Vec::new());
    assert!(
        is_open(&rig, "import model"),
        "the first Esc ends the edit only"
    );
    rig.press(Key::Escape, Modifiers::NONE);
    rig.frame(Vec::new());
    assert!(!is_open(&rig, "import model"), "the second Esc closes");

    // A machine library draft with changes asks before it is dropped.
    let mut rig = Rig::cube("focus-esc-draft", None);
    let dir = super::frame_tests::temp("focus-esc-draft-libs");
    rig.shell.machine_library =
        MachineLibraryWindow::new(Some(dir.join("machines")), Some(dir.join("tools")));
    rig.dirs.push(dir);
    rig.shell.open_machine_library();
    rig.frame(Vec::new());
    rig.shell.machine_library.new_machine();
    for _ in 0..3 {
        rig.frame(Vec::new());
    }
    rig.press(Key::Escape, Modifiers::NONE);
    rig.frame(Vec::new());
    for _ in 0..2 {
        rig.frame(Vec::new());
    }
    assert!(
        rig.shell.machine_library.is_open(),
        "the draft was not dropped"
    );
    assert!(rig.shows("Discard the changes to this machine?"));
}

/// The regions Tab passes through, in order, each counted once per visit,
/// from `presses` presses of Tab (or Shift Tab). A stop outside every region
/// (a pop-up) is left out.
fn region_walk(rig: &mut Rig, back: bool, presses: usize) -> Vec<(&'static str, usize)> {
    let mut walk: Vec<(&'static str, usize)> = Vec::new();
    for _ in 0..presses {
        rig.tab(back);
        if let Some(region) = crate::focus::focused_region(&rig.ctx)
            && walk.last() != Some(&region)
        {
            walk.push(region);
        }
    }
    walk
}

/// Issue #581 F4 (the owner's answer Q3 A): Tab goes through the main window
/// in reading order: menus, step tabs, the health badge, the workflow
/// actions, the project panel, the view, the dock, the inspector, the footer
/// with the primary button last; then round to the menus. Shift Tab goes the
/// other way.
#[test]
fn req_ui_14_tab_follows_reading_order() {
    let mut rig = Rig::cube("focus-order", None);
    rig.press(Key::Num3, Modifiers::ALT);
    let op = rig.shell.app.as_ref().unwrap().project().operations()[0].id;
    rig.shell.select(Selection::Operation(op));
    rig.settle();
    let main = |index| ("main window", index);
    let forward = region_walk(&mut rig, false, 400);
    let first_lap: Vec<_> = forward.iter().take(10).copied().collect();
    assert_eq!(
        first_lap,
        (0..9).map(main).chain([main(0)]).collect::<Vec<_>>(),
        "forward: {forward:?}"
    );
    // The primary button is the footer's last stop.
    let mut rig = Rig::cube("focus-order-back", None);
    rig.press(Key::Num3, Modifiers::ALT);
    // This project's own operation: each project makes its own IDs.
    let op = rig.shell.app.as_ref().unwrap().project().operations()[0].id;
    rig.shell.select(Selection::Operation(op));
    rig.settle();
    let back = region_walk(&mut rig, true, 400);
    let first_lap: Vec<_> = back.iter().take(10).copied().collect();
    assert_eq!(
        first_lap,
        [8, 7, 6, 5, 4, 3, 2, 1, 0, 8].map(main).to_vec(),
        "backward: {back:?}"
    );
}

/// Issue #581 F4 (Q3 A): in every tool window Tab reads the body's parts in
/// order (a part with no control is passed), then the footer, then round
/// inside the window; it never goes behind it.
#[test]
fn req_ui_14_a_dialog_reads_its_body_then_its_footer() {
    let parts = [
        ("export dialog", 2),
        ("import model", 3),
        ("add operation", 2),
        ("tool library", 4),
        ("machine library", 3),
    ];
    for (window, count) in parts {
        let mut rig = Rig::cube(&format!("focus-order-{}", window.replace(' ', "-")), None);
        let dir =
            super::frame_tests::temp(&format!("focus-order-libs-{}", window.replace(' ', "-")));
        rig.shell.tool_library = ToolLibraryWindow::new(Some(dir.join("tools")));
        rig.shell.machine_library =
            MachineLibraryWindow::new(Some(dir.join("machines")), Some(dir.join("tools")));
        rig.dirs.push(dir);
        open_each(&mut rig, window);
        rig.frame(Vec::new());
        let walk = region_walk(&mut rig, false, 150);
        assert!(
            walk.iter().all(|(scope, _)| *scope == window),
            "{window}: {walk:?}"
        );
        let lap: Vec<usize> = walk.iter().map(|(_, i)| *i).collect();
        let end = lap
            .iter()
            .position(|i| *i == count - 1)
            .expect("the footer is reached");
        assert!(
            lap[..=end].windows(2).all(|w| w[0] < w[1]),
            "{window}: {lap:?}"
        );
        assert_eq!(
            lap.get(end + 1),
            lap.first(),
            "{window}: round to the start: {lap:?}"
        );
    }
}

/// Esc closes the tool window on top of those still shown: with the tool
/// library over the machine library, Esc closes the tool library, and Esc
/// again closes the machine library (egui keeps a closed window's place in
/// its order, which must not hold the second Esc back; CodeRabbit on #858).
#[test]
fn req_ui_14_esc_closes_the_window_left_when_the_top_one_has_closed() {
    let mut rig = Rig::cube("focus-esc-two", None);
    let dir = super::frame_tests::temp("focus-esc-two-libs");
    rig.shell.tool_library = ToolLibraryWindow::new(Some(dir.join("tools")));
    rig.shell.machine_library =
        MachineLibraryWindow::new(Some(dir.join("machines")), Some(dir.join("tools")));
    rig.dirs.push(dir);
    rig.shell.open_machine_library();
    for _ in 0..3 {
        rig.frame(Vec::new());
    }
    rig.shell.open_tool_library(Purpose::Add, None);
    for _ in 0..3 {
        rig.frame(Vec::new());
    }
    rig.press(Key::Escape, Modifiers::NONE);
    rig.frame(Vec::new());
    assert!(!rig.shell.tool_library.is_open(), "the top window closed");
    assert!(rig.shell.machine_library.is_open(), "only the top one");
    for _ in 0..3 {
        rig.frame(Vec::new());
    }
    rig.press(Key::Escape, Modifiers::NONE);
    rig.frame(Vec::new());
    assert!(
        !rig.shell.machine_library.is_open(),
        "the window left closed"
    );
}

/// While a job runs, an open tool window takes Esc first (as its Close), and
/// the job goes on; with no tool window open, Esc cancels the job
/// (specification section 16; CodeRabbit on #858).
#[test]
fn req_ui_14_an_open_tool_window_takes_esc_before_a_running_job() {
    let mut rig = Rig::cube("focus-esc-running", None);
    let dir = super::frame_tests::temp("focus-esc-running-libs");
    rig.shell.machine_library =
        MachineLibraryWindow::new(Some(dir.join("machines")), Some(dir.join("tools")));
    rig.dirs.push(dir);
    rig.shell.open_machine_library();
    for _ in 0..3 {
        rig.frame(Vec::new());
    }
    rig.shell.generate(true);
    assert!(rig.shell.run.is_running());
    rig.press(Key::Escape, Modifiers::NONE);
    assert!(
        !rig.shell.machine_library.is_open(),
        "Esc closed the window"
    );
    assert!(rig.shell.run.is_running(), "and left the job running");
    // With no tool window open, Esc cancels the job.
    rig.press(Key::Escape, Modifiers::NONE);
    rig.settle();
    assert!(!rig.shell.run.is_running());
}

/// A collapsed panel keeps its place in the order: its rail's open button
/// is the region's Tab stop (CodeRabbit on #858).
#[test]
fn req_ui_14_a_collapsed_panel_keeps_its_place_in_the_tab_order() {
    let mut rig = Rig::cube("focus-order-collapsed", None);
    rig.press(Key::Num3, Modifiers::ALT);
    rig.press(Key::Num1, Modifiers::COMMAND);
    rig.press(Key::Num3, Modifiers::COMMAND);
    rig.settle();
    let walk = region_walk(&mut rig, false, 200);
    let lap: Vec<usize> = walk
        .iter()
        .map(|(_, i)| *i)
        .take_while(|i| *i != 8)
        .collect();
    assert!(lap.contains(&4), "the project panel's rail: {walk:?}");
    assert!(lap.contains(&6), "the dock's rail: {walk:?}");
    assert!(lap.windows(2).all(|w| w[0] < w[1]), "{walk:?}");
}
