// SPDX-License-Identifier: AGPL-3.0-or-later
//! The start card with the recent projects, and files or folders dropped
//! on the window (issue #706; UI-23, IMP-01, IMP-02, UI-14, UI-15;
//! decision record PROJUX-A-1, the owner's answers S1 to S3 A). Generated
//! folders and the public test cube only.
#![allow(clippy::unwrap_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui;

use super::checkpoint_tests::click;
use super::frame_tests::{Rig, temp};
use super::{Dialog, Shell};

/// A shell with no project open and `folders` recent project folders,
/// `job-0` the newest.
fn rig_with_recent(name: &str, folders: usize) -> (Rig, PathBuf, Vec<PathBuf>) {
    let root = temp(name);
    let made: Vec<PathBuf> = (0..folders)
        .map(|i| {
            let dir = root.join(format!("job-{i}"));
            std::fs::create_dir_all(&dir).unwrap();
            dir.canonicalize().unwrap()
        })
        .collect();
    let mut rig = Rig::new(Shell::with_settings(false, None, None), Vec::new());
    rig.shell.settings.recent = made.clone();
    rig.frame(Vec::new());
    rig.frame(Vec::new());
    (rig, root, made)
}

/// The centre of the text `label`'s rectangle, if it is drawn.
fn text_at(rig: &Rig, label: &str) -> Option<egui::Rect> {
    rig.visible_text_rects()
        .into_iter()
        .find(|(t, _)| t == label)
        .map(|(_, r)| r)
}

#[derive(Debug)]
struct Dropped(PathBuf);

impl egui::DroppedFile for Dropped {
    fn path(&self) -> &Path {
        &self.0
    }

    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|e| e.to_string())
    }
}

/// Drops `paths` on the window, in this order, and draws a frame after.
fn drop_files(rig: &mut Rig, paths: &[PathBuf]) {
    rig.raw_frame(|input| {
        input.dropped_files = paths
            .iter()
            .map(|p| Arc::new(Dropped(p.clone())) as egui::DroppedFileHandle)
            .collect();
    });
    rig.frame(Vec::new());
}

/// Drags `paths` over the window (one frame).
fn hover(rig: &mut Rig, paths: &[PathBuf]) {
    rig.raw_frame(|input| {
        input.hovered_files = paths
            .iter()
            .map(|p| egui::HoveredFile {
                path: Some(p.clone()),
                ..Default::default()
            })
            .collect();
    });
}

/// The public test cube, copied into `dir` under `name`.
fn cube_file(dir: &Path, name: &str) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let cube = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testdata/stl/cube_ascii.stl");
    let file = dir.join(name);
    std::fs::copy(cube, &file).unwrap();
    file
}

/// The messages shown, as their text.
fn messages(rig: &Rig) -> Vec<String> {
    rig.shell
        .overview
        .messages
        .iter()
        .map(|m| m.text.clone())
        .collect()
}

#[test]
fn req_ui_23_req_ui_15_the_start_card_lists_the_recent_projects_newest_first() {
    let (rig, root, made) = rig_with_recent("start-recent", 3);
    assert!(rig.shows("Start a project"), "{:?}", rig.texts());
    assert!(rig.shows("New project…") && rig.shows("Open project…"));
    assert!(rig.shows("Drop an STL or STEP file here to start a project with it"));
    assert!(rig.shows("RECENT PROJECTS"));
    let rows: Vec<f32> = (0..3)
        .map(|i| text_at(&rig, &format!("job-{i}")).unwrap().center().y)
        .collect();
    assert!(
        rows.windows(2).all(|w| w[0] < w[1]),
        "newest first: {rows:?}"
    );
    // Each row shows its full path, muted, beside the name.
    assert!(rig.shows(&super::checkpoints::shown(&made[1])));
    assert!(rig.shows("Clear recent list"));
    // The four steps stay below.
    assert!(rig.shows("Simulate"));
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn req_ui_23_with_nothing_recent_the_list_is_left_out() {
    let (rig, root, _) = rig_with_recent("start-none", 0);
    assert!(rig.shows("Start a project"));
    assert!(!rig.shows("RECENT PROJECTS"), "no empty heading");
    assert!(!rig.shows("Clear recent list"));
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn req_ui_23_req_prj_08_a_recent_row_opens_through_the_open_dialog() {
    let (mut rig, root, made) = rig_with_recent("start-choose", 2);
    click(&mut rig, "job-1");
    rig.frame(Vec::new());
    match &rig.shell.dialog {
        Some(Dialog::Folder { create, folder, .. }) => {
            assert!(!create);
            assert_eq!(*folder, super::checkpoints::shown(&made[1]));
        }
        other => panic!("expected the Open dialog, got {other:?}"),
    }
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn req_nfr_18_a_recent_row_on_the_card_is_logged_without_its_name() {
    let (mut rig, root, _) = rig_with_recent("start-log", 1);
    rig.shell.logged_clicks.clear();
    click(&mut rig, "job-0");
    rig.frame(Vec::new());
    assert!(
        rig.shell
            .logged_clicks
            .contains(&"click: 'Open recent project'".to_owned()),
        "{:?}",
        rig.shell.logged_clicks
    );
    assert!(
        !rig.shell.logged_clicks.iter().any(|l| l.contains("job-0")),
        "{:?}",
        rig.shell.logged_clicks
    );
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn req_ui_23_req_ui_14_each_recent_row_is_a_tab_stop_in_reading_order() {
    let (mut rig, root, _) = rig_with_recent("start-tab", 3);
    let rows: Vec<egui::Rect> = (0..3)
        .map(|i| text_at(&rig, &format!("job-{i}")).unwrap())
        .collect();
    let mut reached = Vec::new();
    let mut seen = Vec::new();
    for _ in 0..60 {
        rig.tab(false);
        let Some(focused) = rig.focused() else {
            continue;
        };
        seen.push(focused.rect);
        if let Some(i) = rows.iter().position(|r| focused.rect.contains_rect(*r))
            && !reached.contains(&i)
        {
            reached.push(i);
        }
    }
    assert_eq!(
        reached,
        [0, 1, 2],
        "Tab reaches the rows top to bottom: {rows:?} {seen:?}"
    );
    // Enter on a row opens it, as a click does.
    for _ in 0..60 {
        if rig.focused().is_some_and(|f| f.rect.contains_rect(rows[1])) {
            break;
        }
        rig.tab(false);
    }
    rig.press(egui::Key::Enter, egui::Modifiers::NONE);
    rig.frame(Vec::new());
    assert!(
        matches!(rig.shell.dialog, Some(Dialog::Folder { create: false, .. })),
        "{:?}",
        rig.shell.dialog
    );
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn req_ui_23_req_ui_15_clear_recent_list_on_the_card_empties_it() {
    let (mut rig, root, _) = rig_with_recent("start-clear", 2);
    click(&mut rig, "Clear recent list");
    rig.frame(Vec::new());
    assert!(rig.shell.settings.recent.is_empty());
    assert!(!rig.shows("RECENT PROJECTS"));
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn req_ui_23_a_gone_recent_folder_is_forgotten_from_the_card() {
    let (mut rig, root, made) = rig_with_recent("start-gone", 2);
    std::fs::remove_dir_all(&made[0]).unwrap();
    // The check runs off the UI thread; it may have run before the folder
    // went, so let it be due again.
    rig.shell.recent_checked_at = None;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while rig.shell.settings.recent.len() != 1 {
        assert!(
            std::time::Instant::now() < deadline,
            "the check never ended"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
        rig.frame(Vec::new());
    }
    rig.frame(Vec::new());
    assert!(!rig.shows("job-0"));
    assert!(rig.shows("job-1"));
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn req_ui_23_req_ui_16_the_start_card_fits_the_view_with_ten_recent_projects() {
    for (w, h) in [(1920.0, 1080.0), (1366.0, 768.0)] {
        let (mut rig, root, _) = rig_with_recent(&format!("start-fit-{w}"), 10);
        rig.size = egui::vec2(w, h);
        for _ in 0..4 {
            rig.frame(Vec::new());
        }
        // The buttons and the drop zone stay in view, inside the window.
        for text in ["Open project…", "New project…", "Clear recent list"] {
            let rect = text_at(&rig, text).unwrap_or_else(|| panic!("{text} at {w}×{h}"));
            assert!(rect.bottom() <= h && rect.top() >= 0.0, "{text} at {w}×{h}");
        }
        assert!(rig.shows("Drop an STL or STEP file here"));
        // At most 560 points wide, inside the window, holding its buttons.
        let card = rig
            .ctx
            .data(|d| d.get_temp::<egui::Rect>(egui::Id::unique("start card")))
            .unwrap();
        assert!(card.width() <= 560.5, "{card:?}");
        assert!(
            card.right() <= w && card.bottom() <= h,
            "{card:?} at {w}×{h}"
        );
        assert!(card.contains_rect(text_at(&rig, "Open project…").unwrap()));
        std::fs::remove_dir_all(root).ok();
    }
}

#[test]
fn req_ui_23_a_project_open_with_no_model_shows_start_with_a_model() {
    let root = temp("start-no-model");
    let mut rig = Rig::new(Shell::with_settings(false, None, None), Vec::new());
    rig.shell.create(root.join("empty"), "Empty");
    rig.frame(Vec::new());
    rig.frame(Vec::new());
    assert!(rig.shows("Start with a model"), "{:?}", rig.texts());
    assert!(rig.shows("Import model…"));
    assert!(rig.shows("Drop an STL or STEP file here to import it."));
    // The recent list stays, with the project just made in it.
    assert!(rig.shows("RECENT PROJECTS"));
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn req_ui_23_req_imp_01_a_model_dropped_with_no_project_starts_one_named_after_it() {
    let root = temp("start-drop-model");
    let file = cube_file(&root, "demo-shaft.stl");
    let mut rig = Rig::new(Shell::with_settings(false, None, None), Vec::new());
    drop_files(&mut rig, std::slice::from_ref(&file));
    match &rig.shell.dialog {
        Some(Dialog::Folder { create, name, .. }) => {
            assert!(*create);
            assert_eq!(name, "demo-shaft");
        }
        other => panic!("expected New project, got {other:?}"),
    }
    // OK creates the project, then the import window opens on the file.
    if let Some(Dialog::Folder { folder, .. }) = &mut rig.shell.dialog {
        *folder = root.join("projects").display().to_string();
    }
    std::fs::create_dir_all(root.join("projects")).unwrap();
    rig.shell.accept_form_dialog();
    rig.frame(Vec::new());
    assert!(rig.shell.app.is_some(), "{:?}", rig.shell.dialog);
    let import = rig.shell.import.as_ref().expect("the import window opens");
    assert_eq!(import.path, file);
    // Nothing is added until Add to project.
    assert!(
        rig.shell
            .app
            .as_ref()
            .unwrap()
            .project()
            .models()
            .is_empty()
    );
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn req_ui_23_cancelling_the_new_project_abandons_the_dropped_model() {
    let root = temp("start-drop-cancel");
    let file = cube_file(&root, "demo-shaft.stl");
    let mut rig = Rig::new(Shell::with_settings(false, None, None), Vec::new());
    drop_files(&mut rig, &[file]);
    click(&mut rig, "Cancel");
    rig.frame(Vec::new());
    assert!(rig.shell.dialog.is_none());
    assert!(rig.shell.app.is_none(), "nothing is created");
    assert!(rig.shell.start_import.is_none());
    // A later New project starts as usual.
    rig.shell.request(super::Next::New, &rig.ctx.clone());
    match &rig.shell.dialog {
        Some(Dialog::Folder { name, .. }) => assert_eq!(name, "Untitled"),
        other => panic!("{other:?}"),
    }
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn req_ui_23_req_imp_01_a_model_dropped_on_an_open_project_opens_the_import_window() {
    let mut rig = Rig::cube("start-drop-import", None);
    let file = cube_file(&rig.dirs[0].join("drops"), "part.stl");
    drop_files(&mut rig, std::slice::from_ref(&file));
    assert_eq!(rig.shell.import.as_ref().unwrap().path, file);
    // Another model replaces it in the window that is open.
    let other = cube_file(&rig.dirs[0].join("drops"), "other.STL");
    drop_files(&mut rig, std::slice::from_ref(&other));
    assert_eq!(rig.shell.import.as_ref().unwrap().path, other);
}

#[test]
fn req_ui_23_a_dropped_project_folder_or_its_project_file_opens_through_open() {
    let root = temp("start-drop-project");
    let mut rig = Rig::new(Shell::with_settings(false, None, None), Vec::new());
    rig.shell
        .create(root.join("baluster-like"), "Baluster like");
    let folder = rig.shell.app.as_ref().unwrap().location().to_path_buf();
    // With a project open, too, and through the usual Open dialog.
    drop_files(&mut rig, std::slice::from_ref(&folder));
    match &rig.shell.dialog {
        Some(Dialog::Folder {
            create, folder: f, ..
        }) => {
            assert!(!create);
            assert_eq!(*f, super::checkpoints::shown(&folder));
        }
        other => panic!("{other:?}"),
    }
    rig.shell.dialog = None;
    drop_files(&mut rig, &[folder.join("project.toml")]);
    assert!(matches!(
        &rig.shell.dialog,
        Some(Dialog::Folder { create: false, folder: f, .. }) if *f == super::checkpoints::shown(&folder)
    ));
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn req_ui_23_req_nfr_07_several_dropped_items_use_the_first_usable_and_name_the_rest() {
    let mut rig = Rig::cube("start-drop-several", None);
    let drops = rig.dirs[0].join("drops");
    let notes = drops.join("notes.txt");
    std::fs::create_dir_all(&drops).unwrap();
    std::fs::write(&notes, "a note").unwrap();
    let a = cube_file(&drops, "a.stl");
    let b = cube_file(&drops, "b.stl");
    drop_files(&mut rig, &[notes, a.clone(), b]);
    rig.settle();
    assert_eq!(rig.shell.import.as_ref().unwrap().path, a);
    assert!(
        messages(&rig).contains(
            &"Not imported: notes.txt, b.stl. Drop or import one file at a time.".to_owned()
        ),
        "{:?}",
        messages(&rig)
    );
}

#[test]
fn req_ui_23_req_nfr_07_a_file_that_cannot_be_used_says_so_and_does_nothing() {
    let root = temp("start-drop-neither");
    std::fs::create_dir_all(&root).unwrap();
    let notes = root.join("notes.txt");
    std::fs::write(&notes, "a note").unwrap();
    let mut rig = Rig::new(Shell::with_settings(false, None, None), Vec::new());
    drop_files(&mut rig, &[notes]);
    rig.frame(Vec::new());
    assert!(rig.shell.dialog.is_none() && rig.shell.app.is_none());
    assert!(
        messages(&rig).contains(
            &"Dropped file: 'notes.txt' is neither a model nor a project. OpenChuckCAM imports \
              STL and STEP files and opens project folders."
                .to_owned()
        ),
        "{:?}",
        messages(&rig)
    );
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn req_ui_23_while_files_are_dragged_over_the_window_it_says_what_the_drop_will_do() {
    let root = temp("start-hover");
    let file = cube_file(&root, "demo-shaft.step");
    let mut rig = Rig::new(Shell::with_settings(false, None, None), Vec::new());
    hover(&mut rig, std::slice::from_ref(&file));
    assert!(rig.shows("Drop to start a new project with demo-shaft.step"));
    let other = cube_file(&root, "b.stl");
    hover(&mut rig, &[file.clone(), other]);
    assert!(rig.shows(
        "Drop to start a new project with demo-shaft.step (1 of 2; the rest are not imported)"
    ));
    let notes = root.join("notes.txt");
    std::fs::write(&notes, "").unwrap();
    hover(&mut rig, &[notes]);
    assert!(rig.shows("OpenChuckCAM imports STL and STEP files and opens project folders"));
    // Nothing is said once the drag leaves.
    rig.frame(Vec::new());
    assert!(!rig.shows("OpenChuckCAM imports STL"));
    // With a project open, a model is imported.
    rig.shell.create(root.join("p"), "P");
    hover(&mut rig, &[file]);
    assert!(rig.shows("Drop to import demo-shaft.step"));
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn req_ui_23_a_drops_message_outlives_only_the_project_the_drop_made() {
    let root = temp("start-drop-note");
    let a = cube_file(&root, "a.stl");
    let b = cube_file(&root, "b.stl");
    let mut rig = Rig::new(Shell::with_settings(false, None, None), Vec::new());
    drop_files(&mut rig, &[a, b]);
    let note = "Not imported: b.stl. Drop or import one file at a time.".to_owned();
    assert!(messages(&rig).contains(&note), "{:?}", messages(&rig));
    // The project the drop starts keeps the note.
    if let Some(Dialog::Folder { folder, .. }) = &mut rig.shell.dialog {
        *folder = root.display().to_string();
    }
    rig.shell.accept_form_dialog();
    rig.settle();
    assert!(messages(&rig).contains(&note), "{:?}", messages(&rig));
    // Any later project open drops it.
    rig.shell.create(root.join("other"), "Other");
    rig.settle();
    assert!(!messages(&rig).contains(&note), "{:?}", messages(&rig));
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn req_ui_23_a_drag_the_system_does_not_name_says_what_is_taken() {
    let mut rig = Rig::new(Shell::with_settings(false, None, None), Vec::new());
    rig.raw_frame(|input| input.hovered_files = vec![egui::HoveredFile::default()]);
    assert!(rig.shows("OpenChuckCAM imports STL and STEP files and opens project folders"));
}

#[test]
fn req_ui_23_the_cards_buttons_start_open_and_import() {
    let root = temp("start-buttons");
    let mut rig = Rig::new(Shell::with_settings(false, None, None), Vec::new());
    click(&mut rig, "New project…");
    assert!(matches!(
        rig.shell.dialog,
        Some(Dialog::Folder { create: true, .. })
    ));
    rig.shell.dialog = None;
    click(&mut rig, "Open project…");
    assert!(matches!(
        rig.shell.dialog,
        Some(Dialog::Folder { create: false, .. })
    ));
    rig.shell.dialog = None;
    rig.shell.create(root.join("p"), "P");
    rig.frame(Vec::new());
    click(&mut rig, "Import model…");
    assert!(rig.shell.import.is_some());
    std::fs::remove_dir_all(root).ok();
}
