// SPDX-License-Identifier: AGPL-3.0-or-later
//! How the inspector shares its width between the labels and the entry boxes
//! (UI-07, UI-16; issue 434; decision 0440; specification section 8):
//! the label column fits its labels first, a number box stays compact, a
//! text box fills the rest.

#![allow(clippy::unwrap_used)]

use super::frame_tests::Rig;
use super::*;
use crate::theme::width;
use eframe::egui;

/// What the inspector drew at one pane width.
struct Drawn {
    left: f32,
    /// Every text in the pane with its rectangle.
    texts: Vec<(String, egui::Rect)>,
    /// The texts in the pane drawn cut short with an ellipsis.
    elided: Vec<String>,
    /// Every entry box in the pane.
    entries: Vec<egui::Rect>,
}

fn drawn(select: impl FnOnce(&mut Rig), pane: f32, name: &str) -> Drawn {
    drawn_rig(select, pane, name).1
}

/// The rig, its inspector dragged to `pane`, and what the inspector drew.
fn drawn_rig(select: impl FnOnce(&mut Rig), pane: f32, name: &str) -> (Rig, Drawn) {
    let mut rig = Rig::cube(name, None);
    crate::theme::install_style(&rig.ctx, crate::theme::Theme::Dark);
    rig.size = egui::vec2(1920.0, 1080.0);
    select(&mut rig);
    for _ in 0..8 {
        rig.frame(Vec::new());
    }
    // The pane is dragged to its width by its splitter, as a user does.
    let from = egui::pos2(rig.size.x - rig.shell.settings.inspector.size, 400.0);
    rig.drag(from, egui::pos2(rig.size.x - pane, 400.0));
    for _ in 0..8 {
        rig.frame(Vec::new());
    }
    assert!(
        (rig.shell.settings.inspector.size - pane).abs() < 1.0,
        "the pane is {} wide, not {pane}",
        rig.shell.settings.inspector.size
    );
    let left = rig.size.x - rig.shell.settings.inspector.size;
    let drawn = Drawn {
        left,
        texts: rig
            .text_rects()
            .into_iter()
            .filter(|(_, r)| r.left() >= left && r.top() > 60.0 && r.bottom() < rig.size.y - 40.0)
            .collect(),
        elided: rig
            .elided_texts()
            .into_iter()
            .filter(|(_, r)| r.left() >= left && r.top() > 60.0 && r.bottom() < rig.size.y - 40.0)
            .map(|(t, _)| t)
            .collect(),
        entries: rig
            .entry_rects()
            .into_iter()
            .filter(|r| r.left() > left && r.height() < 30.0)
            .collect(),
    };
    (rig, drawn)
}

fn operation(rig: &mut Rig) {
    let op = rig.shell.app.as_ref().unwrap().project().operations()[0].id;
    rig.shell.select(Selection::Operation(op));
}

fn setup(rig: &mut Rig) {
    let id = rig.shell.app.as_ref().unwrap().project().setups()[0].id;
    rig.shell.choose_setup(id, None);
}

/// The labels drawn cut short with an ellipsis (the galley's `elided` flag; a
/// galley's text is always the whole string, so its ending says nothing).
/// A default's hint ("0.800 \u{b7} from the tool's cutting data") and the text
/// shown in a drop-down list are cut by design and are left out.
fn cut(d: &Drawn) -> Vec<String> {
    d.elided
        .iter()
        .filter(|t| !t.contains('\u{b7}') && !t.starts_with("Setup's stock material"))
        .cloned()
        .collect()
}

#[test]
fn req_ui_07_no_inspector_label_is_cut_short_at_the_default_and_wide_widths() {
    // The operation inspector at the default and the widest pane, and the
    // setup inspector at the widest. At the default width the setup
    // inspector's longest label ("Tool length measurement") is over the 55%
    // share and is cut, with its whole text in the tooltip: the known cost
    // of decision 0440.
    for (name, select, pane) in [
        ("layout-op", operation as fn(&mut Rig), width::INSPECTOR.0),
        ("layout-op", operation as fn(&mut Rig), width::INSPECTOR.2),
        ("layout-setup", setup as fn(&mut Rig), width::INSPECTOR.2),
    ] {
        let d = drawn(select, pane, name);
        assert!(!d.entries.is_empty(), "{name} {pane}: no entry boxes");
        assert_eq!(cut(&d), Vec::<String>::new(), "{name} at {pane}");
    }
}

#[test]
fn req_ui_07_the_detector_sees_the_cut_label_of_the_setup_inspector_at_the_default_width() {
    // Proves `cut` is not blind: the label that sets the setup inspector's
    // column is over its share at 340 and is reported.
    let d = drawn(setup, width::INSPECTOR.0, "layout-setup-cut");
    assert!(
        cut(&d).iter().any(|t| t == "Tool length measurement"),
        "{:?}",
        d.elided
    );
}

#[test]
fn req_ui_07_every_compact_box_of_a_form_has_one_width_and_shared_edges() {
    for pane in [width::INSPECTOR.0, width::INSPECTOR.2] {
        let d = drawn(operation, pane, "layout-compact");
        let on_row = |label: &str, b: &egui::Rect| {
            d.texts
                .iter()
                .any(|(t, r)| t == label && (r.center().y - b.center().y).abs() < 6.0)
        };
        // The boxes with a unit in them, and the tool number's box.
        let compact: Vec<egui::Rect> = d
            .entries
            .iter()
            .filter(|b| {
                on_row("Tool number", b)
                    || d.texts.iter().any(|(t, r)| {
                        (t == "mm" || t == "mm/min" || t == "rpm" || t == "\u{b0}" || t == "deg")
                            && b.contains(r.center())
                    })
            })
            .copied()
            .collect();
        assert!(compact.len() >= 5, "{pane}: {compact:?}");
        assert!(
            compact.iter().any(|b| on_row("Tool number", b)),
            "{pane}: no tool number box"
        );
        for b in &compact {
            assert!(b.width() <= width::NUMBER_BOX + 0.5, "{pane}: {b:?}");
            assert!(
                (b.left() - compact[0].left()).abs() < 0.6,
                "left {pane}: {compact:?}"
            );
            assert!(
                (b.right() - compact[0].right()).abs() < 0.6,
                "right {pane}: {compact:?}"
            );
        }
        if pane > 450.0 {
            assert!(
                (compact[0].width() - width::NUMBER_BOX).abs() < 0.6,
                "{pane}: {compact:?}"
            );
        }
    }
}

#[test]
fn req_ui_07_widening_the_pane_gives_the_labels_room_before_the_boxes() {
    let narrow = drawn(operation, width::INSPECTOR.1, "layout-first-a");
    let wide = drawn(operation, width::INSPECTOR.2, "layout-first-b");
    // The left edge of the first number box, from the pane edge: it moves
    // right as labels need room, and never beyond the label column maximum.
    let first = |d: &Drawn| {
        d.entries
            .iter()
            .filter(|r| r.width() < 200.0)
            .map(|r| r.left() - d.left)
            .fold(f32::MAX, f32::min)
    };
    assert!(first(&wide) >= first(&narrow));
    assert!(first(&wide) <= 220.0 + 60.0, "{}", first(&wide));
}

/// Decision record 0536 section 4: "Show setting" on a message about a
/// setup's field opens the Setup step at that setup, with the field's
/// section open and the field marked; on a message about a tool, the tool
/// library at that tool's field.
#[test]
fn req_ui_12_show_setting_opens_a_setup_or_a_tool_at_its_field() {
    use openchuck_app::overview::{MessageAction, SettingRef};
    use openchuck_app::setup_form::SetupField;
    let mut rig = Rig::cube("show-setting-places", None);
    rig.size = egui::vec2(1920.0, 1080.0);
    for _ in 0..4 {
        rig.frame(Vec::new());
    }
    let project = rig.shell.app.as_ref().unwrap().project();
    let setup = project.setups()[0].id;
    let tool = project.tools()[0].id;
    rig.shell
        .message_action(MessageAction::ShowSetting(SettingRef::setup(
            setup,
            SetupField::SafeZ,
        )));
    for _ in 0..6 {
        rig.frame(Vec::new());
    }
    assert_eq!(rig.shell.step, Step::Setup);
    assert_eq!(rig.shell.selected, Some(Selection::Setup(setup)));
    assert!(rig.shows("Safe Z"), "the work offset section is open");
    assert!(
        rig.shell
            .form
            .is_shown(&inspector::setup_field_key(SetupField::SafeZ))
    );

    // A link to the setup's machine, which has no field of its own, leaves
    // no earlier mark behind (CodeRabbit on #659).
    rig.shell
        .message_action(MessageAction::ShowSetting(SettingRef::setup_machine(setup)));
    for _ in 0..3 {
        rig.frame(Vec::new());
    }
    assert!(
        !rig.shell
            .form
            .is_shown(&inspector::setup_field_key(SetupField::SafeZ))
    );

    rig.shell
        .message_action(MessageAction::ShowSetting(SettingRef::tool(
            tool,
            openchuck_app::tool_edit::fields::FLUTE_LENGTH,
        )));
    for _ in 0..6 {
        rig.frame(Vec::new());
    }
    assert!(rig.shell.tool_library.is_open());
    assert!(rig.shows("Flute length"));
    assert!(
        rig.shell
            .tool_library
            .is_shown(openchuck_app::tool_edit::fields::FLUTE_LENGTH)
    );
}

/// A set in Avoid gets its Cut short / Warn only / Ignore row beside the
/// label column; when the row wraps in a narrow pane, the wrapped part stays
/// in the value column, never left of it (issue #609; decision 0440).
#[test]
fn req_ui_07_req_ui_16_the_avoid_choice_row_wraps_inside_the_value_column() {
    use openchuck_app::selection::SelectionRole;
    let d = drawn(
        |rig| {
            let app = rig.shell.app.as_mut().unwrap();
            let op = app.project().operations()[0].id;
            let model = app.project().models()[0].id;
            let set = app
                .save_selection(
                    "Finial ball",
                    model,
                    openchuck_app::project::SelectionTarget::Triangles([0].into()),
                )
                .unwrap();
            app.set_operation_selection(op, SelectionRole::Avoid, set, true)
                .unwrap();
            rig.settle();
            operation(rig);
        },
        340.0,
        "avoid-choice-wrap",
    );
    let at = |text: &str| {
        d.texts
            .iter()
            .find(|(t, _)| t == text)
            .unwrap_or_else(|| panic!("{text} is drawn: {:?}", d.texts))
            .1
    };
    // The value column starts where the role's list does: its text is
    // inset by the list's padding.
    let column = at("Add a set\u{2026}").left() - 2.0 * crate::theme::GRID;
    assert!(
        column > d.left + 40.0,
        "{column} is right of the label column"
    );
    for choice in ["Finial ball", "Cut short", "Warn only", "Ignore"] {
        assert!(
            at(choice).left() >= column - 0.5,
            "{choice} starts at {} left of the value column at {column}",
            at(choice).left()
        );
    }
}

/// The sets of issue #759: in Avoid, Rib and Bead on Warn only; in Check,
/// Lip on Cut short and Fence on Ignore (decision 0696).
fn several_sets(rig: &mut Rig) {
    use openchuck_app::project::{OnBreak, SelectionTarget};
    use openchuck_app::selection::SelectionRole;
    let app = rig.shell.app.as_mut().unwrap();
    let op = app.project().operations()[0].id;
    let model = app.project().models()[0].id;
    for (i, (name, role, choice)) in [
        ("Rib", SelectionRole::Avoid, OnBreak::Warn),
        ("Bead", SelectionRole::Avoid, OnBreak::Warn),
        ("Lip", SelectionRole::Check, OnBreak::CutShort),
        ("Fence", SelectionRole::Check, OnBreak::Ignore),
    ]
    .into_iter()
    .enumerate()
    {
        let set = app
            .save_selection(
                name,
                model,
                SelectionTarget::Triangles([u32::try_from(i).unwrap()].into()),
            )
            .unwrap();
        app.set_operation_selection(op, role, set, true).unwrap();
        app.set_on_break(op, role, set, choice).unwrap();
    }
    rig.settle();
    // Tall enough that the whole Geometry section is in the pane.
    rig.size = egui::vec2(1920.0, 1600.0);
    operation(rig);
}

/// Issue #759 (decision 0696 items 1 to 3, 5 and 6): with several sets
/// in each role, each set is named once, on its own line in the value
/// column with × at its right end, its choice just under its name; the
/// role's line holds its name and "Add a set…"; Warn only's note is said
/// once for the role; nothing starts left of the value column. At the
/// narrowest, the default and the widest inspector.
#[test]
fn req_ui_07_req_ui_16_each_set_is_named_once_on_its_own_line_with_its_choice_under_it() {
    for pane in [width::INSPECTOR.1, width::INSPECTOR.0, width::INSPECTOR.2] {
        let d = drawn(several_sets, pane, "geometry-several-sets");
        let all = |text: &str| -> Vec<egui::Rect> {
            d.texts
                .iter()
                .filter(|(t, _)| t == text)
                .map(|(_, r)| *r)
                .collect()
        };
        let one = |text: &str| {
            let found = all(text);
            assert_eq!(
                found.len(),
                1,
                "{pane}: {text} is drawn once: {:?}",
                d.texts
            );
            found[0]
        };
        let lists = all("Add a set\u{2026}");
        assert!(
            lists.len() >= 2,
            "{pane}: each role offers its list: {:?}",
            d.texts
        );
        let column = lists[0].left() - 2.0 * crate::theme::GRID;
        for (set, role) in [
            ("Rib", "Avoid"),
            ("Bead", "Avoid"),
            ("Lip", "Check"),
            ("Fence", "Check"),
        ] {
            let name = one(set);
            let label = one(role);
            assert!(name.top() > label.top(), "{pane}: {set} is under {role}");
            assert!(
                name.left() >= column - 0.5,
                "{pane}: {set} is in the value column"
            );
            // × at the right end of the set's own line.
            let cross = d
                .texts
                .iter()
                .filter(|(t, r)| t == "\u{d7}" && (r.center().y - name.center().y).abs() < 4.0)
                .map(|(_, r)| *r)
                .next()
                .unwrap_or_else(|| panic!("{pane}: {set} has × on its line: {:?}", d.texts));
            assert!(cross.left() > name.right(), "{pane}: × is right of {set}");
            assert!(
                cross.right() > d.left + pane - 40.0,
                "{pane}: × of {set} is at the right end, {} of {}",
                cross.right(),
                d.left + pane
            );
        }
        // No set is drawn as a "name ×" button any more.
        assert!(
            !d.texts.iter().any(|(t, _)| t.ends_with(" \u{d7}")),
            "{pane}: {:?}",
            d.texts
        );
        // Each set's choice is just under its name: its Cut short is the
        // first below the name and above the next set.
        let mut names: Vec<(&str, egui::Rect)> = ["Rib", "Bead", "Lip", "Fence"]
            .iter()
            .map(|s| (*s, one(s)))
            .collect();
        names.sort_by(|a, b| a.1.top().total_cmp(&b.1.top()));
        let cuts = all("Cut short");
        assert_eq!(cuts.len(), 4, "{pane}: one choice per set: {:?}", d.texts);
        for (i, (set, name)) in names.iter().enumerate() {
            let next = names.get(i + 1).map_or(f32::INFINITY, |n| n.1.top());
            assert!(
                cuts.iter()
                    .any(|c| c.top() > name.top() && c.top() < next && c.left() >= column - 0.5),
                "{pane}: {set}'s choice is under it"
            );
        }
        // Warn only's sampling note once, for Avoid's two sets on it.
        let notes = d
            .texts
            .iter()
            .filter(|(t, _)| t.starts_with("Warn only samples the path"))
            .count();
        assert_eq!(notes, 1, "{pane}: {:?}", d.texts);
        // Nothing of the section starts left of the value column but the
        // roles' names and the section's own lines.
        for (t, r) in &d.texts {
            if [
                "Rib",
                "Bead",
                "Lip",
                "Fence",
                "Cut short",
                "Warn only",
                "Ignore",
                "\u{d7}",
            ]
            .contains(&t.as_str())
            {
                assert!(
                    r.left() >= column - 0.5,
                    "{pane}: {t} at {} is left of {column}",
                    r.left()
                );
            }
        }
        // A set on Ignore says nothing under it (the owner's change).
        assert!(
            !d.texts.iter().any(|(t, _)| t.contains("limits nothing")),
            "{pane}: {:?}",
            d.texts
        );
    }
}

/// Whether the text `text` is drawn struck through, in the pane right of
/// `left`.
fn struck(rig: &Rig, text: &str, left: f32) -> Option<bool> {
    fn walk(shape: &egui::Shape, text: &str, left: f32) -> Option<bool> {
        match shape {
            egui::Shape::Text(t) if t.galley.text() == text && t.pos.x >= left => Some(
                t.galley
                    .job
                    .sections
                    .iter()
                    .any(|s| s.format.strikethrough.width > 0.0),
            ),
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| walk(s, text, left)),
            _ => None,
        }
    }
    rig.shapes.iter().find_map(|s| walk(&s.shape, text, left))
}

/// Decision 0696 items 4 and 7: a set on Ignore is greyed and struck
/// through, with no warning line; its tooltip, on hover and on keyboard
/// focus, says "Ignored: this set limits nothing", and it gives its state
/// in its accessible name. A set on another choice is not struck through.
#[test]
fn req_ui_07_req_op_21_a_set_on_ignore_is_struck_through_and_says_so_in_words() {
    let (mut rig, d) = drawn_rig(several_sets, width::INSPECTOR.0, "geometry-ignored");
    assert_eq!(struck(&rig, "Fence", d.left), Some(true));
    for set in ["Rib", "Bead", "Lip"] {
        assert_eq!(struck(&rig, set, d.left), Some(false), "{set}");
    }
    let tip = "Ignored: this set limits nothing";
    assert!(!rig.shows(tip));
    // On hover.
    let fence = d.texts.iter().find(|(t, _)| t == "Fence").unwrap().1;
    rig.frame(vec![egui::Event::PointerMoved(fence.center())]);
    for _ in 0..60 {
        rig.frame(Vec::new());
    }
    assert!(rig.shows(tip), "the tooltip shows on hover");
    rig.frame(vec![egui::Event::PointerMoved(egui::pos2(10.0, 10.0))]);
    for _ in 0..10 {
        rig.frame(Vec::new());
    }
    // On keyboard focus, with its state in its accessible name: Tab from
    // the first field of the inspector until the line has the keyboard.
    let named = "Fence, ignored: this set limits nothing";
    let mut reached = false;
    for _ in 0..200 {
        rig.press(egui::Key::Tab, egui::Modifiers::NONE);
        if rig.events.iter().any(|e| {
            matches!(e, egui::output::OutputEvent::FocusGained(info)
                if info.label.as_deref() == Some(named))
        }) {
            reached = true;
            break;
        }
    }
    assert!(reached, "Tab reaches the ignored set's line, named {named}");
    for _ in 0..60 {
        rig.frame(Vec::new());
    }
    assert!(rig.shows(tip), "the tooltip shows on keyboard focus");
}

/// Decision 0696 items 2 to 4: a set's warning that it reaches a held
/// region or a tab (11.7) is said once, under that set's own line, with its links;
/// Warn only under a set chooses it for that set; × at the end of a set's
/// line takes the set out of its role.
#[test]
fn req_ui_07_req_set_09_a_sets_line_carries_its_reach_warning_choice_and_remove() {
    use openchuck_app::project::{Item, OnBreak, SelectionTarget};
    use openchuck_app::selection::SelectionRole;
    use openchuck_app::setup::{Millimetres, Tab, TabLayout, TabShape};
    let select = |rig: &mut Rig| {
        let app = rig.shell.app.as_mut().unwrap();
        let op = app.project().operations()[0].id;
        let model = app.project().models()[0].id;
        // A tab the length of the stock and wider than the cube: every
        // corner of the set that lies in the stock is inside it.
        let mut setup = app.project().setups()[0].clone();
        setup.workholding.tabs = vec![Tab {
            auto: None,
            x_start: Millimetres(-7.0),
            length: Millimetres(14.0),
            shape: TabShape::Cylinder {
                diameter: Millimetres(20.0),
            },
            layout: TabLayout::OnAxis,
        }];
        app.execute(Command::Replace(Item::Setup(setup))).unwrap();
        let whole = app
            .save_selection(
                "Whole",
                model,
                SelectionTarget::Triangles((0..12).collect()),
            )
            .unwrap();
        let bead = app
            .save_selection("Bead", model, SelectionTarget::Triangles([1].into()))
            .unwrap();
        app.set_operation_selection(op, SelectionRole::Machine, whole, true)
            .unwrap();
        app.set_operation_selection(op, SelectionRole::Avoid, bead, true)
            .unwrap();
        rig.settle();
        rig.size = egui::vec2(1920.0, 1600.0);
        operation(rig);
    };
    let (mut rig, d) = drawn_rig(select, width::INSPECTOR.0, "geometry-line-actions");
    let at = |d: &Drawn, text: &str| -> Vec<egui::Rect> {
        d.texts
            .iter()
            .filter(|(t, _)| t == text)
            .map(|(_, r)| *r)
            .collect()
    };
    // The reach warning, once, under Whole's line and above Contain within.
    let whole = at(&d, "Whole")[0];
    let contain = at(&d, "Contain within")[0];
    let warnings: Vec<egui::Rect> = d
        .texts
        .iter()
        .filter(|(t, _)| t.contains("'Whole'"))
        .map(|(_, r)| *r)
        .collect();
    assert_eq!(warnings.len(), 1, "{:?}", d.texts);
    assert!(warnings[0].top() > whole.top() && warnings[0].top() < contain.top());
    let links = at(&d, "Show in view");
    assert_eq!(links.len(), 1, "{:?}", d.texts);
    assert_eq!(at(&d, "Edit workholding").len(), 1);
    // Warn only under Bead chooses it for Bead.
    let bead = at(&d, "Bead")[0];
    let warn = d
        .texts
        .iter()
        .filter(|(t, r)| t == "Warn only" && r.top() > bead.top())
        .map(|(_, r)| *r)
        .next()
        .expect("Warn only under Bead");
    rig.click(warn.center());
    rig.settle();
    let app = rig.shell.app.as_ref().unwrap();
    let operation = app.project().operations()[0].clone();
    let id = operation.avoid[0];
    assert_eq!(
        operation.restriction_options.avoid(id).on_break,
        OnBreak::Warn
    );
    // × at the end of Whole's line takes it out of Cut on.
    for _ in 0..4 {
        rig.frame(Vec::new());
    }
    let left = rig.size.x - rig.shell.settings.inspector.size;
    let whole = rig
        .text_rects()
        .into_iter()
        .find(|(t, r)| t == "Whole" && r.left() >= left)
        .unwrap()
        .1;
    let cross = rig
        .text_rects()
        .into_iter()
        .find(|(t, r)| t == "\u{d7}" && (r.center().y - whole.center().y).abs() < 4.0)
        .expect("× on Whole's line")
        .1;
    rig.click(cross.center());
    rig.settle();
    let app = rig.shell.app.as_ref().unwrap();
    assert!(app.project().operations()[0].selections.is_empty());
}
