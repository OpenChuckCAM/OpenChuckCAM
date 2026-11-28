// SPDX-License-Identifier: AGPL-3.0-or-later
//! Auto-save through real frames (PRJ-08; decision PROJUX-B-3): the title
//! bar's switch, a save at once when it is turned on, waiting for a typed
//! value, waiting after a recovery, and one warning while saves fail.
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

use eframe::egui::{Key, Modifiers};
use openchuck_app::overview::Severity;
use openchuck_app::project::Project;
use openchuck_app::{Application, Command, Threads};

use super::frame_tests::{Rig, temp};
use super::{Next, Shell};

fn at(rig: &Rig, needle: &str) -> eframe::egui::Rect {
    rig.text_rects()
        .into_iter()
        .find(|(t, _)| t == needle)
        .unwrap_or_else(|| panic!("'{needle}' is not drawn: {:?}", rig.texts()))
        .1
}

fn edit(rig: &mut Rig, name: &str) {
    rig.shell
        .app
        .as_mut()
        .unwrap()
        .execute(Command::RenameProject(name.into()))
        .unwrap();
    rig.frame(Vec::new());
}

fn modified(rig: &Rig) -> bool {
    rig.shell.app.as_ref().unwrap().is_modified()
}

/// The switch: the box just right of the "Auto-save" label.
fn switch(rig: &Rig) -> eframe::egui::Pos2 {
    let label = at(rig, super::autosave::LABEL);
    eframe::egui::pos2(label.right() + 22.0, label.center().y)
}

#[test]
fn req_prj_08_auto_save_is_off_by_default_and_nothing_saves_by_itself() {
    let mut rig = Rig::cube("autosave-off", None);
    assert!(!rig.shell.settings.auto_save);
    assert!(rig.shows(super::autosave::LABEL));
    assert!(rig.shows("Off"));
    edit(&mut rig, "Changed");
    for _ in 0..5 {
        rig.frame(Vec::new());
    }
    assert!(modified(&rig));
    assert!(rig.shows("Edited, not saved"));
}

#[test]
fn req_prj_08_req_ui_14_turning_the_switch_on_saves_at_once_and_says_so() {
    let mut rig = Rig::cube("autosave-on", None);
    edit(&mut rig, "Changed");
    rig.click(switch(&rig));
    rig.frame(Vec::new());
    rig.frame(Vec::new());
    assert!(rig.shell.settings.auto_save);
    assert!(!modified(&rig), "saved at once");
    assert!(rig.shows("\u{2713} Saved automatically just now"), "{:?}", rig.texts());
    // The saved files hold the change.
    let dir = rig.shell.app.as_ref().unwrap().location().to_path_buf();
    let reopened = Application::open(&dir, Threads::Exactly(1)).unwrap();
    assert_eq!(reopened.project().name(), "Changed");
    // The File menu's tick shows it, and turns it off.
    rig.click(at(&rig, "File").center());
    rig.frame(Vec::new());
    rig.click(at(&rig, "Save automatically").center());
    rig.frame(Vec::new());
    assert!(!rig.shell.settings.auto_save);
}

#[test]
fn req_prj_08_req_ui_07_a_due_save_waits_for_a_typed_value() {
    let mut rig = super::pending_edit_tests::generated("autosave-typing");
    super::pending_edit_tests::type_unconfirmed(&mut rig, "Stepover", "1.6");
    edit(&mut rig, "Changed");
    rig.shell.set_auto_save(true);
    rig.frame(Vec::new());
    rig.frame(Vec::new());
    assert!(modified(&rig), "not saved while a value is typed");
    assert!(
        rig.shows("Edited \u{b7} next auto-save waits for the typed value"),
        "{:?}",
        rig.texts()
    );
    // Enter confirms it; the next frame saves.
    rig.press(Key::Enter, Modifiers::NONE);
    rig.frame(Vec::new());
    rig.frame(Vec::new());
    assert!(!modified(&rig));
}

#[test]
fn req_prj_08_after_a_recovery_auto_save_waits_for_one_save_by_hand() {
    let root = temp("autosave-recovered");
    let mut rig = Rig::new(Shell::with_settings(false, None, None), vec![root.clone()]);
    rig.shell.settings.auto_save = true;
    let app = Application::from_checkpoint(
        &root.join("project"),
        Project::new("Recovered"),
        Threads::Exactly(1),
    )
    .unwrap();
    rig.shell.recovery_project = Some((app, Vec::new(), true));
    let ctx = rig.ctx.clone();
    rig.shell.request(Next::Recovery, &ctx);
    rig.settle();
    for _ in 0..3 {
        rig.frame(Vec::new());
    }
    assert!(rig.shell.app.as_ref().unwrap().is_recovered_unsaved());
    assert!(
        rig.shows("Recovered, not saved \u{b7} auto-save waits for File > Save"),
        "{:?}",
        rig.texts()
    );
}

#[cfg(unix)]
#[test]
fn req_prj_08_req_nfr_07_a_failed_save_warns_once_and_the_switch_stays_on() {
    use std::os::unix::fs::PermissionsExt as _;
    let mut rig = Rig::cube("autosave-failed", None);
    let dir = rig.shell.app.as_ref().unwrap().location().to_path_buf();
    edit(&mut rig, "Changed");
    let read_only = std::fs::Permissions::from_mode(0o555);
    std::fs::set_permissions(&dir, read_only).unwrap();
    // A file in a read-only folder can still be written by root; the test
    // checks only where the folder refuses the write.
    let refused = std::fs::write(dir.join("probe"), "x").is_err();
    rig.shell.set_auto_save(true);
    rig.frame(Vec::new());
    rig.frame(Vec::new());
    rig.settle();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    if !refused {
        return;
    }
    assert!(rig.shell.settings.auto_save, "the switch stays on");
    assert!(rig.shows("\u{25b2} Auto-save failed \u{b7} see Messages"), "{:?}", rig.texts());
    let warnings: Vec<_> = rig
        .shell
        .overview
        .messages
        .iter()
        .filter(|m| m.text.starts_with("Auto-save"))
        .collect();
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].severity, Severity::Warning);
}
