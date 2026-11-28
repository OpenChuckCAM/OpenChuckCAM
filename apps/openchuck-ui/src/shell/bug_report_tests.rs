// SPDX-License-Identifier: AGPL-3.0-or-later
//! Help > Save files for a bug report… through real frames (NFR-20;
//! decision PROJUX-B-4): the dialog lists what the file will hold, and saves
//! it with the defaults or with more ticked.
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

use super::frame_tests::{Rig, temp};

fn at(rig: &Rig, needle: &str) -> eframe::egui::Rect {
    rig.text_rects()
        .into_iter()
        .find(|(t, _)| t == needle)
        .unwrap_or_else(|| panic!("'{needle}' is not drawn: {:?}", rig.texts()))
        .1
}

/// The names in a zip, read from its central directory.
fn zip_names(bytes: &[u8]) -> Vec<String> {
    let u16_at = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]) as usize;
    let u32_at = |at: usize| {
        u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]) as usize
    };
    let end = bytes.len() - 22;
    let mut at = u32_at(end + 16);
    (0..u16_at(end + 10))
        .map(|_| {
            let len = u16_at(at + 28);
            let name = String::from_utf8(bytes[at + 46..at + 46 + len].to_vec()).unwrap();
            at += 46 + len + u16_at(at + 30) + u16_at(at + 32);
            name
        })
        .collect()
}

/// The dialog's list of what the file will hold: its lines end in "bytes)".
fn listed(rig: &Rig) -> Vec<String> {
    rig.text_rects()
        .into_iter()
        .map(|(t, _)| t)
        .filter(|t| t.ends_with(" bytes)"))
        .collect()
}

/// A folder to save into.
fn out(name: &str) -> std::path::PathBuf {
    let dir = temp(name);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("report.zip")
}

fn open(rig: &mut Rig) {
    rig.click(at(rig, "Help").center());
    rig.frame(Vec::new());
    rig.click(at(rig, "Save files for a bug report…").center());
    rig.frame(Vec::new());
    assert!(rig.shows("Save files for a bug report"));
}

#[test]
fn req_nfr_20_help_lists_what_the_file_holds_and_saves_it_with_the_defaults() {
    let mut rig = Rig::cube("bug-report-defaults", None);
    open(&mut rig);
    // Always in, and the project ticked by default; nothing sent anywhere.
    assert!(rig.shows("Nothing is sent anywhere"));
    let list = listed(&rig);
    assert!(
        list.iter().any(|l| l.starts_with("build-information.txt")),
        "{list:?}"
    );
    assert!(
        list.iter().any(|l| l.starts_with("project/project.toml")),
        "{list:?}"
    );
    assert!(
        list.iter().any(|l| l.starts_with("contents.txt")),
        "{list:?}"
    );
    assert!(!list.iter().any(|l| l.starts_with("models/")), "{list:?}");
    let out = out("bug-report-defaults-out");
    rig.shell.bug_report_path = Some(out.clone());
    rig.click(at(&rig, "Save…").center());
    rig.frame(Vec::new());
    assert!(rig.shows("Saved"), "{:?}", rig.texts());
    let names = zip_names(&std::fs::read(&out).unwrap());
    assert_eq!(
        names.first().map(String::as_str),
        Some("build-information.txt")
    );
    assert!(
        names.iter().any(|n| n == "project/project.toml"),
        "{names:?}"
    );
    assert!(!names.iter().any(|n| n.starts_with("models/")), "{names:?}");
    assert_eq!(names.last().map(String::as_str), Some("contents.txt"));
}

#[test]
fn req_nfr_20_ticking_model_files_adds_them_to_the_list_and_the_file() {
    let mut rig = Rig::cube("bug-report-models", None);
    open(&mut rig);
    rig.click(at(&rig, "Model files").center());
    rig.frame(Vec::new());
    let list = listed(&rig);
    assert!(list.iter().any(|l| l.starts_with("models/")), "{list:?}");
    let out = out("bug-report-models-out");
    rig.shell.bug_report_path = Some(out.clone());
    rig.click(at(&rig, "Save…").center());
    rig.frame(Vec::new());
    let names = zip_names(&std::fs::read(&out).unwrap());
    assert!(names.iter().any(|n| n.starts_with("models/")), "{names:?}");
}

#[test]
fn req_nfr_20_req_nfr_07_a_file_that_cannot_be_written_says_why() {
    let mut rig = Rig::cube("bug-report-refused", None);
    open(&mut rig);
    rig.shell.bug_report_path = Some("/nowhere/at/all/report.zip".into());
    rig.click(at(&rig, "Save…").center());
    rig.frame(Vec::new());
    assert!(rig.shows("Could not write"), "{:?}", rig.texts());
    // Close closes it.
    rig.click(at(&rig, "Close").center());
    rig.frame(Vec::new());
    assert!(!rig.shows("Save files for a bug report"));
}
