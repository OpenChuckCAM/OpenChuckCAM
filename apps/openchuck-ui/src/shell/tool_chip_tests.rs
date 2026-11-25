// SPDX-License-Identifier: AGPL-3.0-or-later
//! The tool chip of an operation row in the project panel (issue #790;
//! decision record 0690, the owner's answers T1 A and T2 A of 25 Nov
//! 2026): the chip shows the tool's number, and its tooltip names the tool
//! and says what it is; an operation with no tool, or whose tool is missing
//! from the project's tool list, has a chip that says so with the operation's
//! own reason. The tooltip opens on hover over the chip and while the row
//! has the keyboard; the chip is not a Tab stop of its own (UI-06, UI-14).

#![allow(clippy::unwrap_used)]

use eframe::egui::{self, Event};

use super::frame_tests::Rig;
use super::tree::{ToolChip, tool_chip};
use super::*;

/// The cube project's first operation's tool: "4 mm ball", a 4 mm ball end
/// mill with 2 flutes, controller number 1 (`testing::cube_project`).
const NAME: &str = "4 mm ball";
const WHAT: &str = "T1 · Ball end mill · Ø4 · 2 flutes";

fn first_operation(rig: &Rig) -> openchuck_app::project::Operation {
    rig.shell.app.as_ref().unwrap().project().operations()[0].clone()
}

/// Replaces the first operation's tool reference with `tool`.
fn set_tool(rig: &mut Rig, tool: Option<openchuck_app::project::id::ToolId>) {
    use openchuck_app::project::Item;
    let app = rig.shell.app.as_mut().unwrap();
    let mut op = app.project().operations()[0].clone();
    op.tool = tool;
    app.execute(Command::Replace(Item::Operation(op))).unwrap();
    rig.settle();
}

/// The rectangle the first operation's tool chip was drawn in.
fn chip_rect(rig: &Rig) -> egui::Rect {
    let id = first_operation(rig).id;
    rig.ctx
        .data(|d| d.get_temp::<egui::Rect>(egui::Id::unique(("operation tool chip", id))))
        .expect("the operation row has a tool chip")
}

/// Whether a tooltip with the tool's two lines is painted.
fn shows_tool_tip(rig: &Rig) -> bool {
    rig.shows(NAME) && rig.shows(WHAT)
}

#[test]
fn req_ui_06_req_ui_14_the_tool_chip_tooltip_names_the_tool_and_says_what_it_is() {
    let rig = Rig::cube("tool-chip-text", None);
    let app = rig.shell.app.as_ref().unwrap();
    let op = first_operation(&rig);
    let chip = tool_chip(app.project().tools(), op.tool);
    assert_eq!(
        chip,
        ToolChip {
            text: "T1".to_owned(),
            title: Some(NAME.to_owned()),
            line: WHAT.to_owned(),
            problem: false,
        }
    );
}

#[test]
fn req_ui_06_req_ui_14_an_operation_without_a_tool_has_a_chip_with_its_reason() {
    let rig = Rig::cube("tool-chip-none", None);
    let app = rig.shell.app.as_ref().unwrap();
    // No tool chosen: the operation's own reason, "it has no tool".
    assert_eq!(
        tool_chip(app.project().tools(), None),
        ToolChip {
            text: "No tool".to_owned(),
            title: None,
            line: "It has no tool. Choose a tool in the operation's Tool section.".to_owned(),
            problem: true,
        }
    );
    // A tool that is not in the project's tool list: "its tool is missing".
    let gone = openchuck_app::project::id::ToolId::new();
    assert_eq!(
        tool_chip(app.project().tools(), Some(gone)),
        ToolChip {
            text: "Tool missing".to_owned(),
            title: None,
            line: "Its tool is missing. Choose a tool in the operation's Tool section.".to_owned(),
            problem: true,
        }
    );
    // The words are the operation's own (it cannot be generated for them).
    assert!(
        openchuck_app::operation::NO_TOOL == "it has no tool"
            && openchuck_app::operation::TOOL_MISSING == "its tool is missing"
    );
}

#[test]
fn req_ui_14_the_tool_tooltip_opens_on_hover_over_the_chip() {
    let mut rig = Rig::cube("tool-chip-hover", None);
    assert!(!shows_tool_tip(&rig), "no tooltip before the pointer comes");
    let chip = chip_rect(&rig);
    rig.frame(vec![Event::PointerMoved(chip.center())]);
    for _ in 0..60 {
        rig.frame(Vec::new());
    }
    assert!(shows_tool_tip(&rig), "hovering the chip shows the tool");
}

#[test]
fn req_ui_14_the_tool_tooltip_opens_while_the_operation_row_has_the_keyboard() {
    let mut rig = Rig::cube("tool-chip-focus", None);
    let id = first_operation(&rig).id;
    let row = rig
        .ctx
        .data(|d| d.get_temp::<egui::Id>(egui::Id::unique(("operation row focus", id))))
        .unwrap();
    rig.ctx.memory_mut(|m| m.request_focus(row));
    rig.frame(Vec::new());
    rig.frame(Vec::new());
    assert!(
        shows_tool_tip(&rig),
        "the focused row shows its tool, with no pointer over it"
    );
}

#[test]
fn req_ui_14_the_tool_chip_is_not_a_tab_stop_of_its_own() {
    // The owner's answer T2: the information is reached through the row's
    // own focus, so a row keeps the Tab stops it had (decision 0677 item 3
    // is met through the row).
    let mut rig = Rig::cube("tool-chip-tab", None);
    let chip = chip_rect(&rig);
    let mut seen = Vec::new();
    for _ in 0..80 {
        rig.tab(false);
        let Some(focused) = rig.focused() else {
            continue;
        };
        assert!(
            focused.rect != chip,
            "Tab reached the tool chip itself ({chip:?})"
        );
        seen.push(focused.rect);
    }
    let id = first_operation(&rig).id;
    let row = rig
        .ctx
        .data(|d| d.get_temp::<egui::Id>(egui::Id::unique(("operation row focus", id))))
        .unwrap();
    let row_rect = rig.ctx.read_response(row).unwrap().rect;
    assert!(seen.contains(&row_rect), "Tab reaches the operation row");
}

#[test]
fn req_ui_06_req_ui_11_a_chip_without_a_tool_is_outlined_in_the_error_colour() {
    // A tool missing from the tool list cannot happen in a project the
    // program accepts (a used tool cannot be removed, and loading checks
    // references), so its chip is tested through `tool_chip` above; an
    // operation with no tool is drawn here.
    let mut rig = Rig::cube("tool-chip-none-drawn", None);
    set_tool(&mut rig, None);
    assert!(rig.shows("No tool"));
    let chip = chip_rect(&rig);
    let outlined = rig.shapes().iter().any(|s| match &s.shape {
        egui::Shape::Rect(r) => {
            r.rect.expand(1.0).contains_rect(chip) && r.stroke.color == crate::theme::DARK.error
        }
        _ => false,
    });
    assert!(
        outlined,
        "the chip says what is wrong by its words and its outline"
    );
}
