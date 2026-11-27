// SPDX-License-Identifier: AGPL-3.0-or-later
//! Lengths and feeds shown and typed in inches, through real frames
//! (UI-18; decision PROJUX-B-2): the operation inspector's number boxes, a
//! unit typed with a value, and the footer's switch.
#![allow(clippy::unwrap_used, clippy::panic, clippy::float_cmp)]

use eframe::egui::{Key, Modifiers};
use openchuck_app::project::ParameterValue;
use openchuck_app::project::id::OperationId;

use super::frame_tests::Rig;
use super::pending_edit_tests::{field, generated, type_unconfirmed};
use crate::units::Display;

fn op(rig: &Rig) -> OperationId {
    rig.shell.app.as_ref().unwrap().project().operations()[0].id
}

fn stepover(rig: &Rig) -> Option<ParameterValue> {
    rig.shell
        .app
        .as_ref()
        .unwrap()
        .project()
        .operation(op(rig))
        .unwrap()
        .parameters
        .get("stepover")
        .cloned()
}

/// Types `text` in the box labelled `label` and confirms it with Enter.
fn enter(rig: &mut Rig, label: &str, text: &str) {
    type_unconfirmed(rig, label, text);
    rig.press(Key::Enter, Modifiers::NONE);
    rig.frame(Vec::new());
}

/// The rig with lengths shown in inches.
fn inches(name: &str) -> Rig {
    let mut rig = generated(name);
    rig.shell.set_units(Display::Inches);
    for _ in 0..3 {
        rig.frame(Vec::new());
    }
    rig
}

#[test]
fn req_ui_18_req_nfr_05_a_length_box_shows_and_reads_inches_and_stores_millimetres() {
    let mut rig = inches("units-inches");
    // Stepover 0.8 mm reads 0.0315 in.
    assert!(rig.shows("0.0315"), "{:?}", rig.texts());
    enter(&mut rig, "Stepover", "0.1");
    assert_eq!(stepover(&rig), Some(ParameterValue::Number(2.54)));
    // The project file holds millimetres.
    let app = rig.shell.app.as_ref().unwrap();
    app.save().unwrap();
    let text = openchuck_app::project::file::render(app.project()).unwrap();
    assert!(
        text.values().any(|t| t.contains("stepover = 2.54")),
        "{text:?}"
    );
}

#[test]
fn req_ui_18_a_shown_value_confirmed_unchanged_stores_nothing_new() {
    let mut rig = inches("units-unchanged");
    let before = rig.shell.app.as_ref().unwrap().revision();
    // Enter on the shown 0.0315 in, which is 0.8001 mm, stores nothing.
    rig.click(field(&rig, "Stepover"));
    rig.press(Key::Enter, Modifiers::NONE);
    rig.frame(Vec::new());
    assert_eq!(stepover(&rig), Some(ParameterValue::Number(0.8)));
    assert_eq!(rig.shell.app.as_ref().unwrap().revision(), before);
}

#[test]
fn req_ui_18_a_unit_typed_with_a_value_converts_and_says_what_was_stored() {
    // In millimetres, a value typed in inches.
    let mut rig = generated("units-typed");
    enter(&mut rig, "Stepover", "0.005 in");
    assert_eq!(stepover(&rig), Some(ParameterValue::Number(0.127)));
    assert!(rig.shows("Stored as 0.127 mm"), "{:?}", rig.texts());
    // In inches, a value typed in millimetres, and a sum with a unit.
    rig.shell.set_units(Display::Inches);
    rig.frame(Vec::new());
    enter(&mut rig, "Stepover", "0.2 mm");
    assert_eq!(stepover(&rig), Some(ParameterValue::Number(0.2)));
    assert!(
        rig.shows("Stored as 0.2 mm (0.0079 in)"),
        "{:?}",
        rig.texts()
    );
    enter(&mut rig, "Stepover", "1/8 in");
    assert_eq!(stepover(&rig), Some(ParameterValue::Number(3.175)));
}

#[test]
fn req_ui_18_req_nfr_07_an_unknown_unit_is_refused_and_the_last_value_kept() {
    let mut rig = generated("units-refused");
    enter(&mut rig, "Stepover", "2 ft");
    assert_eq!(stepover(&rig), Some(ParameterValue::Number(0.8)));
    assert!(
        rig.shows("this box reads mm, cm, in or \" after the value."),
        "{:?}",
        rig.texts()
    );
}

#[test]
fn req_ui_18_the_footer_switch_changes_the_units_and_remembers_them() {
    let mut rig = generated("units-switch");
    assert!(rig.shows("0.8"));
    let at = rig
        .text_rects()
        .into_iter()
        .find(|(t, r)| t == "in" && r.top() > rig.size.y - 60.0)
        .unwrap_or_else(|| panic!("the footer has 'in': {:?}", rig.texts()))
        .1;
    rig.click(at.center());
    rig.frame(Vec::new());
    assert_eq!(rig.shell.settings.units, "in");
    assert!(rig.shows("0.0315"), "{:?}", rig.texts());
    // A box's unit follows.
    assert!(
        rig.text_rects()
            .iter()
            .any(|(t, _)| t == "in/min" || t == "in")
    );
}

#[test]
fn req_ui_18_req_ui_07_switching_units_resolves_a_typed_value_first() {
    // A number typed in millimetres is never read in inches: the switch
    // asks about it first, as leaving the form does (decision 0233).
    let mut rig = generated("units-guard");
    type_unconfirmed(&mut rig, "Stepover", "1.6");
    rig.shell.set_units(Display::Inches);
    for _ in 0..4 {
        rig.frame(Vec::new());
    }
    assert!(rig.shows("Unconfirmed changes"), "{:?}", rig.texts());
    assert_eq!(rig.shell.settings.units, "mm");
    // Keep stores 1.6 mm, then the switch goes ahead.
    let keep = rig
        .text_rects()
        .into_iter()
        .find(|(t, _)| t == "Keep")
        .unwrap()
        .1;
    rig.click(keep.center());
    rig.settle();
    assert_eq!(stepover(&rig), Some(ParameterValue::Number(1.6)));
    assert_eq!(rig.shell.settings.units, "in");
}
