// SPDX-License-Identifier: AGPL-3.0-or-later
//! Showing and hiding the Facing overlay's labels (issue #881; decision
//! record MOPUP-1-A, the owner's answers A1 and B1 and the chip length rule
//! of 25 and 26 Nov 2026): a "Facing labels" group in the Show menu (V) and
//! under View > Facing labels, one chip with the toolpath moves that names
//! what is hidden, "Show everything" in the Show menu, and a per-user
//! setting. Display only (UI-03, UI-14, UI-15).

#![allow(clippy::unwrap_used)]

use openchuck_app::facing_labels::{FacingLabel, ShownFacingLabels};
use openchuck_app::overview::Step;
use openchuck_app::pathview::ShownMoves;
use openchuck_app::playback::MoveKind;
use openchuck_app::settings::SettingsFile;

use super::Selection;
use super::frame_tests::{Rig, temp};

/// The cube project generated, on the Operations step.
fn generated(name: &str, settings: Option<SettingsFile>) -> Rig {
    let mut rig = Rig::cube(name, settings);
    rig.shell.generate(true);
    rig.settle();
    rig.shell.open_step(Step::Operations);
    rig.settle();
    rig
}

/// Opens the Show menu as V does (V is tested with the view).
fn open_show_menu(rig: &mut Rig) {
    rig.shell.viewport.show_menu_open = true;
    rig.frame(Vec::new());
    rig.frame(Vec::new());
}

/// Clicks the text `name` where it is drawn, and draws.
fn click_text(rig: &mut Rig, name: &str) {
    let at = rig
        .text_rects()
        .into_iter()
        .find(|(t, _)| t == name)
        .unwrap_or_else(|| panic!("{name} is drawn"))
        .1;
    rig.click(at.center());
    rig.frame(Vec::new());
    rig.frame(Vec::new());
}

/// The text drawn that starts with "Hidden", if any.
fn chip(rig: &Rig) -> Option<(String, egui::Rect)> {
    rig.text_rects()
        .into_iter()
        .find(|(t, _)| t.starts_with("Hidden:") || t.starts_with("Hidden ("))
}

#[test]
fn req_ui_14_the_show_menu_has_a_facing_labels_group_that_sets_each_kind() {
    let mut rig = generated("facing-labels-menu", None);
    open_show_menu(&mut rig);
    for name in [
        "Facing labels",
        "Faces",
        "Margin",
        "Opposite faces",
        "Stops beside a grip",
    ] {
        assert!(rig.shows(name), "the Show menu lists {name}");
    }
    assert!(
        !rig.shows("Show everything"),
        "only while something is hidden"
    );
    click_text(&mut rig, "Margin");
    let mut expected = ShownFacingLabels::ALL;
    expected.set(FacingLabel::Margin, false);
    assert_eq!(rig.shell.viewport.shown_facing_labels(), expected);
    assert_eq!(chip(&rig).unwrap().0, "Hidden: Facing margin");
    // Partly ticked: a press on the group hides all four, as Entry and
    // exit's box does; the chip then names the group once.
    click_text(&mut rig, "Facing labels");
    assert_eq!(
        rig.shell.viewport.shown_facing_labels(),
        ShownFacingLabels::NONE
    );
    assert_eq!(chip(&rig).unwrap().0, "Hidden: Facing labels");
    click_text(&mut rig, "Facing labels");
    assert_eq!(
        rig.shell.viewport.shown_facing_labels(),
        ShownFacingLabels::ALL
    );
    assert!(chip(&rig).is_none());
}

/// The owner's answer B1: one chip names hidden moves, then hidden Facing
/// labels; "Show everything" in the Show menu brings both back.
#[test]
fn req_ui_14_req_sim_01_one_chip_names_moves_and_labels_and_show_everything_clears_it() {
    let mut rig = generated("facing-labels-chip", None);
    let gcode = rig.shell.run.last().map(|r| r.gcode.clone());
    let mut moves = ShownMoves::ALL;
    moves.set(MoveKind::Rapid, false);
    moves.set(MoveKind::LeadIn, false);
    rig.shell.viewport.set_shown_moves(moves);
    let mut labels = ShownFacingLabels::ALL;
    labels.set(FacingLabel::Margin, false);
    labels.set(FacingLabel::Stop, false);
    rig.shell.viewport.set_shown_facing_labels(labels);
    rig.frame(Vec::new());
    rig.frame(Vec::new());
    assert_eq!(
        chip(&rig).unwrap().0,
        "Hidden: Rapid, Lead-in and ramp +2 more"
    );
    open_show_menu(&mut rig);
    assert!(rig.shows("Show everything"));
    click_text(&mut rig, "Show everything");
    assert_eq!(rig.shell.viewport.shown_moves(), ShownMoves::ALL);
    assert_eq!(
        rig.shell.viewport.shown_facing_labels(),
        ShownFacingLabels::ALL
    );
    assert!(chip(&rig).is_none());
    // Only the view: the program is the one generated.
    assert_eq!(rig.shell.run.last().map(|r| r.gcode.clone()), gcode);
}

/// The length rule: the long form while the strip has room, else
/// "Hidden (N)"; never cut off, and Simulate's inline Show choices give way
/// first (OI-529).
#[test]
fn req_ui_14_the_chip_is_short_when_the_strip_has_no_room_and_never_cut() {
    let mut rig = generated("facing-labels-chip-width", None);
    let mut moves = ShownMoves::ALL;
    moves.set(MoveKind::Rapid, false);
    moves.set(MoveKind::LeadIn, false);
    rig.shell.viewport.set_shown_moves(moves);
    rig.shell
        .viewport
        .set_shown_facing_labels(ShownFacingLabels::NONE);
    rig.shell.open_step(Step::Simulate);
    rig.settle();
    assert_eq!(
        chip(&rig).unwrap().0,
        "Hidden: Rapid, Lead-in and ramp +1 more"
    );
    // From a wide window to the smallest one UI-16 supports.
    let mut forms = Vec::new();
    for (width, height) in [
        (1920.0, 1080.0),
        (1600.0, 900.0),
        (1366.0, 768.0),
        (1366.0, 768.0),
    ] {
        rig.size = egui::vec2(width, height);
        rig.frame(Vec::new());
        rig.frame(Vec::new());
        let (text, rect) = chip(&rig).unwrap_or_else(|| panic!("a chip at {width}"));

        assert!(
            text == "Hidden: Rapid, Lead-in and ramp +1 more" || text == "Hidden (3)",
            "{text} at {width}"
        );
        // The whole text shows: its clip (the view's strip) cuts none of it.
        let (_, whole, shown) = rig
            .painted()
            .into_iter()
            .find(|(t, ..)| t.as_deref() == Some(text.as_str()))
            .unwrap();
        assert_eq!(whole, shown, "{text} cut at {width}");
        assert!(rect.max.x <= width, "{text} past the window at {width}");
        forms.push(text);
    }
    assert_eq!(forms[0], "Hidden: Rapid, Lead-in and ramp +1 more");
    assert_eq!(
        forms[3], "Hidden (3)",
        "1366 × 768 has no room for the long form"
    );
}

#[test]
fn req_ui_14_view_facing_labels_is_in_the_menu() {
    let mut rig = generated("facing-labels-view-menu", None);
    click_text(&mut rig, "View");
    assert!(rig.shows("Facing labels"), "View > Facing labels");
    click_text(&mut rig, "Facing labels");
    for name in ["Faces", "Margin", "Opposite faces", "Stops beside a grip"] {
        assert!(rig.shows(name), "View > Facing labels lists {name}");
    }
}

/// The owner's answer to Q3: the view is told which operation is selected,
/// so a selected Facing operation's labels alone are drawn (the filter is
/// tested with the view, `viewport`).
#[test]
fn req_ui_03_the_view_follows_the_selected_operation() {
    let mut rig = generated("facing-labels-selection", None);
    let op = rig.shell.app.as_ref().unwrap().project().operations()[0].id;
    rig.shell.selected = Some(Selection::Operation(op));
    rig.frame(Vec::new());
    assert_eq!(rig.shell.viewport.selected_operation(), Some(op));
    rig.shell.selected = None;
    rig.frame(Vec::new());
    assert_eq!(rig.shell.viewport.selected_operation(), None);
}

#[test]
fn req_ui_15_the_facing_labels_shown_are_kept_for_the_user_across_a_restart() {
    let config = temp("facing-labels-settings");
    let file = SettingsFile::in_folder(&config);
    let mut rig = generated("facing-labels-settings", Some(file.clone()));
    rig.dirs.push(config.clone());
    let mut labels = ShownFacingLabels::ALL;
    labels.set(FacingLabel::Separation, false);
    rig.shell.viewport.set_shown_facing_labels(labels);
    rig.frame(Vec::new());
    rig.shell.save_settings();
    let kept = file.load();
    assert!(kept.problem.is_none());
    assert_eq!(kept.settings.facing_labels, labels);
    let shell = super::Shell::with_settings(false, None, Some(file));
    assert_eq!(shell.viewport.shown_facing_labels(), labels);
}
