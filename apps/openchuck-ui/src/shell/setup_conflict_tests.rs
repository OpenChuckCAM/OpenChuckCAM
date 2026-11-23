// SPDX-License-Identifier: AGPL-3.0-or-later
//! A setup value that conflicts with the setup's tabs or held regions
//! (issue #870; SET-09, UI-07, NFR-07; decisions 0233 and 0104): Keep checks
//! the setup as it would be stored, tabs included, before applying any
//! value.

#![allow(clippy::unwrap_used)]

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
