// SPDX-License-Identifier: AGPL-3.0-or-later
//! A setup value that conflicts with the setup's tabs or held regions
//! (issue #870; SET-09, UI-07, NFR-07; decisions 0233 and 0104): Keep checks
//! the setup as it would be stored, tabs included, before applying any
//! value.

#![allow(clippy::unwrap_used)]
#![allow(clippy::float_cmp)]

use eframe::egui::{self, Event, Key, Modifiers};
use openchuck_app::Command;
use openchuck_app::overview::{SetupPart, Step};
use openchuck_app::project::Item;
use openchuck_app::setup::{Millimetres, Stock, Tab, TabLayout, TabShape};

use super::Selection;
use super::frame_tests::Rig;

/// The cube project (stock X -7 to 7, Ø20) with one tab placed by hand at
/// X -6.5 to -5.5, its setup's Stock section open on Setup.
fn with_hand_tab(name: &str) -> Rig {
    let mut rig = Rig::cube(name, None);
    rig.size = egui::vec2(1920.0, 2400.0);
    let app = rig.shell.app.as_mut().unwrap();
    let mut setup = app.project().setups()[0].clone();
    setup.workholding.tabs = vec![Tab {
        auto: None,
        x_start: Millimetres(-6.5),
        length: Millimetres(1.0),
        shape: TabShape::Cylinder {
            diameter: Millimetres(3.0),
        },
        layout: TabLayout::OnAxis,
    }];
    let id = setup.id;
    app.execute(Command::Replace(Item::Setup(setup))).unwrap();
    rig.shell.overview_dirty = true;
    rig.shell.open_step(Step::Setup);
    rig.shell.choose_setup(id, Some(SetupPart::Stock));
    frames(&mut rig, 5);
    rig
}

fn frames(rig: &mut Rig, n: usize) {
    for _ in 0..n {
        rig.frame(Vec::new());
    }
}

fn entry_beside(rig: &Rig, label: &str) -> egui::Pos2 {
    let row = rig
        .visible_text_rects()
        .into_iter()
        .find(|(text, _)| text == label)
        .map(|(_, rect)| rect)
        .unwrap_or_else(|| panic!("{label} is visible: {:?}", rig.texts()));
    rig.entry_rects()
        .into_iter()
        .find(|rect| {
            rect.left() > row.right()
                && rect.width() > 40.0
                && (rect.center().y - row.center().y).abs() < 12.0
        })
        .unwrap_or_else(|| panic!("{label} has an entry box"))
        .center()
}

/// Types `text` into the box beside `label` without Enter.
fn type_in(rig: &mut Rig, label: &str, text: &str) {
    let at = entry_beside(rig, label);
    rig.click(at);
    frames(rig, 2);
    rig.click(entry_beside(rig, label));
    rig.press(Key::A, Modifiers::COMMAND);
    rig.frame(vec![Event::Text(text.to_owned())]);
    assert!(
        crate::pending_edits::entries(&rig.ctx)
            .iter()
            .any(|e| e.label == label),
        "{label} holds an unconfirmed edit"
    );
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

fn stock(rig: &Rig) -> (f64, f64, f64) {
    let app = rig.shell.app.as_ref().unwrap();
    match app.project().setups()[0].stock.as_ref().unwrap() {
        Stock::Cylinder {
            diameter,
            length,
            x_start,
        } => (diameter.0, length.0, x_start.0),
        other => panic!("{other:?}"),
    }
}

/// Decision 0233 item 4: Keep checks every pending entry before applying
/// any. A Start X that would leave the tab outside the stock is refused by
/// the check, so the valid Diameter typed with it is not applied alone.
#[test]
fn req_ui_07_req_set_09_keep_checks_a_setup_value_against_the_tabs_before_applying_any() {
    let mut rig = with_hand_tab("setup-conflict-keep");
    type_in(&mut rig, "Diameter", "18");
    type_in(&mut rig, "Start X", "-5");
    let op = rig.shell.app.as_ref().unwrap().project().operations()[0].id;
    rig.shell.select(Selection::Operation(op));
    frames(&mut rig, 3);
    assert!(rig.shows("Unconfirmed changes"));
    click_text(&mut rig, "Keep");
    frames(&mut rig, 10);
    assert_eq!(stock(&rig), (20.0, 14.0, -7.0), "nothing is applied");
    assert!(
        rig.shell.selected != Some(Selection::Operation(op)),
        "the departure stops"
    );
}

/// The setup's stored tabs.
fn tab_start(rig: &Rig) -> f64 {
    let app = rig.shell.app.as_ref().unwrap();
    app.project().setups()[0].workholding.tabs[0].x_start.0
}

/// Types `text` into `label`'s box and presses Enter.
fn enter(rig: &mut Rig, label: &str, text: &str) {
    type_in(rig, label, text);
    rig.press(Key::Enter, Modifiers::NONE);
    frames(rig, 3);
}

/// Issue #870, the owner's answer A: a Start X refused because the tab would
/// be outside the stock says so in the one form, naming the tab and both
/// ways out; it does not block the Apply that moves the tab, and once the
/// tab allows it, Enter applies it, in place.
#[test]
fn req_set_09_req_ui_07_a_value_refused_for_the_tabs_never_blocks_the_edit_that_fixes_it() {
    let mut rig = with_hand_tab("setup-conflict-unlock");
    enter(&mut rig, "Start X", "-5");
    assert_eq!(stock(&rig).2, -7.0, "refused");
    let texts: Vec<String> = rig.texts().into_iter().map(|(t, _)| t).collect();
    let message = texts
        .iter()
        .find(|t| t.starts_with("Workholding (setup 'Setup 1'): tab 1 (X"))
        .unwrap_or_else(|| panic!("the refusal names tab 1: {texts:?}"));
    assert!(
        message.contains("Change Start X, or move or remove tab 1"),
        "{message}"
    );
    assert!(!message.contains("Invalid setup"), "{message}");
    // Move the tab inside the stock that -5 would give, and Apply.
    rig.shell
        .workholding_panel
        .as_mut()
        .unwrap()
        .draft_mut()
        .tabs[0]
        .x_start = Millimetres(-4.5);
    frames(&mut rig, 3);
    click_text(&mut rig, "Apply");
    frames(&mut rig, 20);
    assert!(!rig.shows("Unconfirmed changes"), "Apply is not blocked");
    assert_eq!(tab_start(&rig), -4.5, "the tab is applied");
    // Start X still holds -5, now with nothing against it.
    assert!(rig.shows("-5"), "the typed value stays in its box");
    assert!(
        rig.texts()
            .iter()
            .any(|(t, _)| t.contains("Press Enter to apply")),
        "{:?}",
        rig.texts()
    );
    let at = entry_beside(&rig, "Start X");
    rig.click(at);
    frames(&mut rig, 1);
    rig.press(Key::Enter, Modifiers::NONE);
    frames(&mut rig, 3);
    assert_eq!(stock(&rig).2, -5.0, "Enter applies it");
}

/// The owner's answer A to Q8: setup values kept together by Keep are
/// checked together and applied as one undoable step, so a pair valid only
/// together is kept, and one Undo puts both back.
#[test]
fn req_ui_07_req_ui_08_setup_values_kept_together_are_one_undoable_step() {
    let mut rig = with_hand_tab("setup-conflict-together");
    // Length 1 alone (X -7 to -6) leaves the tab out; with Start X -6.5
    // the stock is X -6.5 to -5.5, the tab's span.
    type_in(&mut rig, "Length", "1");
    type_in(&mut rig, "Start X", "-6.5");
    let op = rig.shell.app.as_ref().unwrap().project().operations()[0].id;
    rig.shell.select(Selection::Operation(op));
    frames(&mut rig, 3);
    assert!(rig.shows("Unconfirmed changes"));
    click_text(&mut rig, "Keep");
    frames(&mut rig, 10);
    assert_eq!(stock(&rig), (20.0, 1.0, -6.5), "both are applied");
    assert_eq!(rig.shell.selected, Some(Selection::Operation(op)));
    rig.shell.undo();
    frames(&mut rig, 3);
    assert_eq!(stock(&rig), (20.0, 14.0, -7.0), "one Undo puts both back");
}

/// Issue #870 (from the review of pull request #891): a value already
/// refused for a conflict with the tabs waits in its box and takes no part
/// in a departure. The question lists only the other unconfirmed values,
/// and Keep applies them without it, so a valid value is never held back by
/// the refused one.
#[test]
fn req_set_09_req_ui_07_a_refused_value_never_holds_back_a_valid_one() {
    let mut rig = with_hand_tab("setup-conflict-with-valid");
    enter(&mut rig, "Start X", "-5");
    assert_eq!(stock(&rig).2, -7.0, "refused");
    type_in(&mut rig, "Diameter", "18");
    let op = rig.shell.app.as_ref().unwrap().project().operations()[0].id;
    rig.shell.select(Selection::Operation(op));
    frames(&mut rig, 3);
    assert!(rig.shows("Unconfirmed changes"), "Diameter is asked about");
    let listed: Vec<String> = crate::pending_edits::entries(&rig.ctx)
        .into_iter()
        .map(|e| e.label)
        .collect();
    assert_eq!(listed, ["Diameter"], "the refused Start X is not listed");
    click_text(&mut rig, "Keep");
    frames(&mut rig, 10);
    assert_eq!(stock(&rig), (18.0, 14.0, -7.0), "Diameter is applied alone");
    assert_eq!(
        rig.shell.selected,
        Some(Selection::Operation(op)),
        "the departure goes ahead"
    );
}

/// Issue #870 (from the second review of pull request #891): a refused
/// value that the tabs come to allow waits for its own Enter. When Keep
/// applies another value of the setup that makes it allowed, Keep does not
/// apply it too, unasked and as a second step.
#[test]
fn req_set_09_req_ui_08_an_allowed_value_still_waits_for_its_own_enter() {
    let mut rig = with_hand_tab("setup-conflict-waits");
    // Length 1 from X -7 ends at -6: tab 1 (X -6.5 to -5.5) is left out.
    enter(&mut rig, "Length", "1");
    assert_eq!(stock(&rig).1, 14.0, "Length 1 is refused");
    // Start X -6.5 is allowed with the stored length, and with it Length 1
    // would hold the tab (X -6.5 to -5.5).
    type_in(&mut rig, "Start X", "-6.5");
    let op = rig.shell.app.as_ref().unwrap().project().operations()[0].id;
    rig.shell.select(Selection::Operation(op));
    frames(&mut rig, 3);
    assert!(rig.shows("Unconfirmed changes"));
    click_text(&mut rig, "Keep");
    frames(&mut rig, 10);
    assert_eq!(
        stock(&rig),
        (20.0, 14.0, -6.5),
        "Start X is kept; Length stays as stored until its own Enter"
    );
    let app = rig.shell.app.as_mut().unwrap();
    assert!(app.undo().unwrap());
    assert_eq!(
        app.project().setups()[0].stock.as_ref().map(|_| ()),
        Some(())
    );
    rig.shell.overview_dirty = true;
    frames(&mut rig, 2);
    assert_eq!(stock(&rig), (20.0, 14.0, -7.0), "one Undo undoes the Keep");
}

/// Issue #870 (from the second review of pull request #891): a value the
/// tabs allowed is checked again when the setup changes once more, and is
/// refused again if it no longer fits, rather than saying it is ready.
#[test]
fn req_set_09_req_nfr_07_an_allowed_value_is_checked_again_after_a_later_change() {
    let mut rig = with_hand_tab("setup-conflict-recheck");
    enter(&mut rig, "Length", "1");
    enter(&mut rig, "Start X", "-6.5");
    assert_eq!(stock(&rig), (20.0, 14.0, -6.5));
    assert!(
        rig.texts()
            .iter()
            .any(|(t, _)| t.contains("Press Enter to apply")),
        "Length 1 is now allowed: {:?}",
        rig.texts()
    );
    // From X -7.5, Length 1 ends at -6.5 and leaves tab 1 out again.
    enter(&mut rig, "Start X", "-7.5");
    assert_eq!(
        stock(&rig).2,
        -7.5,
        "Start X -7.5 holds the tab with Length 14"
    );
    frames(&mut rig, 2);
    let texts: Vec<String> = rig.texts().into_iter().map(|(t, _)| t).collect();
    assert!(
        !texts.iter().any(|t| t.contains("Press Enter to apply")),
        "no longer ready: {texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|t| t.starts_with("Workholding (setup 'Setup 1'): tab 1 (X")),
        "refused again, naming tab 1: {texts:?}"
    );
}
