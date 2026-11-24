// SPDX-License-Identifier: AGPL-3.0-or-later
//! Showing and hiding each kind of toolpath move in the view (issue #789;
//! decision record PROJUX-A-2, the owner's answers P1 B, P2 A and P3 A):
//! the path legend's tick boxes on Operations, the Show menu (V) and View >
//! Toolpath moves set one choice; the playback panel draws the lines for it;
//! the user's settings keep it; a chip says what is hidden while anything
//! is. Display only (SIM-01, UI-14, UI-15).

#![allow(clippy::unwrap_used)]

use openchuck_app::overview::Step;
use openchuck_app::pathview::ShownMoves;
use openchuck_app::playback::MoveKind;
use openchuck_app::settings::SettingsFile;

use super::frame_tests::{Rig, temp};

/// The cube project generated, on the Operations step with its legend.
fn generated(name: &str, settings: Option<SettingsFile>) -> Rig {
    let mut rig = Rig::cube(name, settings);
    rig.shell.generate(true);
    rig.settle();
    rig.shell.open_step(Step::Operations);
    rig.settle();
    rig
}

/// Hides `kinds` as a control of the view would (the legend's tick boxes are
/// tested with the overlays, `viewport_overlays`: the test rig has no 3D
/// renderer, so the view and its overlays are not drawn here), and draws.
fn hide(rig: &mut Rig, kinds: &[MoveKind]) {
    let mut moves = rig.shell.viewport.shown_moves();
    for kind in kinds {
        moves.set(*kind, false);
    }
    rig.shell.viewport.set_shown_moves(moves);
    rig.frame(Vec::new());
    rig.frame(Vec::new());
}

#[test]
fn req_sim_01_a_kind_hidden_in_the_view_is_hidden_from_the_lines_and_named_in_the_chip() {
    let mut rig = generated("moves-wiring", None);
    assert!(!rig.shows("Hidden:"), "nothing hidden, no chip");
    let gcode = rig.shell.run.last().map(|r| r.gcode.clone());
    hide(&mut rig, &[MoveKind::Rapid]);
    let mut expected = ShownMoves::ALL;
    expected.set(MoveKind::Rapid, false);
    // The playback panel draws the lines for the same choice, on every step.
    assert_eq!(rig.shell.play.shown_moves(), expected);
    assert!(rig.shows("Hidden: Rapid · Show all"));
    hide(&mut rig, &ShownMoves::ENTRY);
    assert!(rig.shows("Hidden: Rapid, Entry and exit · Show all"));
    // Only the view: the program is the one generated.
    assert_eq!(rig.shell.run.last().map(|r| r.gcode.clone()), gcode);
}

#[test]
fn req_sim_01_the_hidden_chip_shows_every_kind_again() {
    let mut rig = generated("moves-chip", None);
    hide(&mut rig, &[MoveKind::Cutting]);
    assert!(rig.shows("Hidden: Cut · Show all"));
    // The chip is in the view's toolbar strip on every step.
    rig.shell.open_step(Step::Simulate);
    rig.settle();
    assert!(rig.shows("Hidden: Cut · Show all"));
    let chip = rig
        .text_rects()
        .into_iter()
        .find(|(t, _)| t == "Hidden: Cut · Show all")
        .unwrap()
        .1;
    rig.click(chip.center());
    rig.frame(Vec::new());
    assert_eq!(rig.shell.viewport.shown_moves(), ShownMoves::ALL);
    assert_eq!(rig.shell.play.shown_moves(), ShownMoves::ALL);
    assert!(!rig.shows("Hidden:"));
}

#[test]
fn req_ui_14_the_show_menu_lists_the_kinds_of_move() {
    let mut rig = generated("moves-menu", None);
    // V opens it (tested with the view); here it is opened as V does.
    rig.shell.viewport.show_menu_open = true;
    rig.frame(Vec::new());
    rig.frame(Vec::new());
    for name in [
        "Toolpath moves",
        "Cut",
        "Link",
        "Rapid",
        "Entry and exit",
        "Plunge",
        "Lead-in and ramp",
        "Lead-out",
    ] {
        assert!(rig.shows(name), "the Show menu lists {name}");
    }
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

#[test]
fn req_ui_14_req_sim_01_the_show_menu_tick_boxes_set_the_kinds() {
    let mut rig = generated("moves-menu-ticks", None);
    rig.shell.viewport.show_menu_open = true;
    rig.frame(Vec::new());
    rig.frame(Vec::new());
    click_text(&mut rig, "Rapid");
    assert_eq!(rig.shell.viewport.shown_moves().hidden(), [MoveKind::Rapid]);
    // Entry and exit sets its three kinds; one under it on its own.
    click_text(&mut rig, "Entry and exit");
    let mut hidden = vec![MoveKind::Rapid];
    hidden.extend(ShownMoves::ENTRY);
    assert_eq!(rig.shell.viewport.shown_moves().hidden(), hidden);
    click_text(&mut rig, "Plunge");
    assert!(rig.shell.viewport.shown_moves().shows(MoveKind::Plunge));
    // "Show all moves" while any is hidden.
    click_text(&mut rig, "Show all moves");
    assert_eq!(rig.shell.viewport.shown_moves(), ShownMoves::ALL);
    assert!(
        !rig.shows("Show all moves"),
        "only while something is hidden"
    );
}

#[test]
fn req_ui_14_view_toolpath_moves_is_in_the_menu() {
    let mut rig = generated("moves-view-menu", None);
    let view = rig
        .text_rects()
        .into_iter()
        .find(|(t, _)| t == "View")
        .unwrap()
        .1;
    rig.click(view.center());
    rig.frame(Vec::new());
    assert!(rig.shows("Toolpath moves"), "View > Toolpath moves");
}

#[test]
fn req_ui_15_req_sim_01_the_kinds_shown_are_kept_for_the_user_across_a_restart() {
    let config = temp("moves-settings");
    let file = SettingsFile::in_folder(&config);
    let mut rig = generated("moves-settings", Some(file.clone()));
    rig.dirs.push(config.clone());
    hide(&mut rig, &[MoveKind::Linking]);
    rig.shell.save_settings();
    let kept = file.load();
    assert!(kept.problem.is_none());
    assert_eq!(kept.settings.path_moves.hidden(), [MoveKind::Linking]);
    // A restart starts from it: the view, the lines and the chip.
    let shell = super::Shell::with_settings(false, None, Some(file));
    assert_eq!(shell.viewport.shown_moves().hidden(), [MoveKind::Linking]);
    assert_eq!(shell.play.shown_moves().hidden(), [MoveKind::Linking]);
}
