// SPDX-License-Identifier: AGPL-3.0-or-later
//! What a file or folder dropped on the window is, and what dropping it
//! does (UI-23, IMP-01, IMP-02; issue #706, decision record 0697).
//!
//! A dropped STL or STEP file is imported, or starts a new project with it
//! when no project is open. A project folder, or its `project.toml`, opens
//! through Open. Anything else is neither. With several items, the first
//! usable one is used and the rest are named as not imported. The words
//! here are shown while the items are dragged over the window and in
//! Messages after the drop.

use std::path::{Path, PathBuf};

use crate::lead_in::Context;
use crate::overview::{Message, Severity};
use openchuck_project::file::PROJECT_FILE;
use openchuck_types::model::ModelFormat;

/// What one dropped item is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dropped {
    /// An STL or STEP file, to import.
    Model(PathBuf),
    /// A project folder (given as the folder or as its `project.toml`), to
    /// open.
    Project(PathBuf),
    /// Anything else.
    Other(PathBuf),
}

impl Dropped {
    /// What `path` is. A folder is a project when it holds `project.toml`;
    /// a model is told by its file name's extension, as import tells it.
    pub fn of(path: &Path) -> Self {
        if path.is_dir() {
            if path.join(PROJECT_FILE).is_file() {
                return Self::Project(path.to_path_buf());
            }
            return Self::Other(path.to_path_buf());
        }
        if path.file_name().is_some_and(|n| n == PROJECT_FILE)
            && let Some(folder) = path.parent()
        {
            return Self::Project(folder.to_path_buf());
        }
        let name = path.file_name().map(|n| n.to_string_lossy());
        if name.is_some_and(|n| ModelFormat::from_file_name(&n).is_some()) {
            Self::Model(path.to_path_buf())
        } else {
            Self::Other(path.to_path_buf())
        }
    }
}

/// What a drop of one or more items does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DropPlan {
    /// The first usable item, if any.
    pub used: Option<Dropped>,
    /// The names of every other item, in the order given.
    pub rest: Vec<String>,
    /// The names of the items that are neither a model nor a project, in
    /// the order given (when nothing is usable, these are what is named).
    pub unusable: Vec<String>,
    /// How many items were dropped.
    pub count: usize,
}

/// The name of a dropped item as shown: its file or folder name.
pub fn item_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// What dropping `paths`, in the order the system gives them, does.
pub fn plan(paths: &[PathBuf]) -> DropPlan {
    let kinds: Vec<Dropped> = paths.iter().map(|p| Dropped::of(p)).collect();
    let used_at = kinds.iter().position(|k| !matches!(k, Dropped::Other(_)));
    let rest = paths
        .iter()
        .enumerate()
        .filter(|(i, _)| Some(*i) != used_at)
        .map(|(_, p)| item_name(p))
        .collect();
    let unusable = kinds
        .iter()
        .filter_map(|k| match k {
            Dropped::Other(p) => Some(item_name(p)),
            _ => None,
        })
        .collect();
    DropPlan {
        used: used_at.and_then(|i| kinds.get(i).cloned()),
        rest,
        unusable,
        count: paths.len(),
    }
}

/// What the program does with dropped files: the line for anything that
/// is neither a model nor a project.
pub const WHAT_IS_TAKEN: &str = "OpenChuckCAM imports STL and STEP files and opens project folders";

/// The project name a new project started with `model` gets: the file's
/// name without its extension ("demo-shaft.step" gives "demo-shaft").
pub fn project_name(model: &Path) -> String {
    model.file_stem().map_or_else(
        || "Untitled".to_owned(),
        |s| s.to_string_lossy().into_owned(),
    )
}

/// The one line shown in the middle of the window while `plan`'s items are
/// dragged over it.
pub fn hover_text(plan: &DropPlan, project_open: bool) -> String {
    let line = match &plan.used {
        None => return WHAT_IS_TAKEN.to_owned(),
        Some(Dropped::Model(p)) if project_open => format!("Drop to import {}", item_name(p)),
        Some(Dropped::Model(p)) => {
            format!("Drop to start a new project with {}", item_name(p))
        }
        Some(Dropped::Project(folder)) => {
            format!("Drop to open the project {}", item_name(folder))
        }
        Some(Dropped::Other(_)) => unreachable!("an unusable item is never used"),
    };
    if plan.count > 1 {
        format!("{line} (1 of {}; the rest are not imported)", plan.count)
    } else {
        line
    }
}

/// The Messages line naming what a drop left out, if anything: every item
/// but the one used ("Not imported: b.stl, c.step. Drop or import one file
/// at a time."), or, when nothing could be used, that it is neither and
/// what the program takes.
pub fn message(plan: &DropPlan) -> Option<Message> {
    if plan.used.is_none() {
        let names = quoted_list(&plan.unusable);
        let verb = if plan.unusable.len() == 1 {
            "is"
        } else {
            "are"
        };
        return Some(Message::about(
            Severity::Warning,
            "Dropped file",
            &Context::default(),
            &format!("{names} {verb} neither a model nor a project. {WHAT_IS_TAKEN}"),
            None,
            None,
        ));
    }
    if plan.rest.is_empty() {
        return None;
    }
    Some(Message::about(
        Severity::Warning,
        "Not imported",
        &Context::default(),
        &format!(
            "{}. Drop or import one file at a time",
            plan.rest.join(", ")
        ),
        None,
        None,
    ))
}

/// `'a'`, `'a' and 'b'`, `'a', 'b' and 'c'`.
fn quoted_list(names: &[String]) -> String {
    let quoted: Vec<String> = names.iter().map(|n| format!("'{n}'")).collect();
    match quoted.split_last() {
        None => String::new(),
        Some((last, [])) => last.clone(),
        Some((last, others)) => format!("{} and {last}", others.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    struct Dir(PathBuf);
    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A folder with a project in `project`, an empty folder `plain`, and
    /// the files `a.stl`, `b.STEP`, `c.stp` and `notes.txt`.
    fn place(name: &str) -> Dir {
        let dir =
            std::env::temp_dir().join(format!("openchuck-dropped-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("project")).unwrap();
        std::fs::create_dir_all(dir.join("plain")).unwrap();
        std::fs::write(dir.join("project").join(PROJECT_FILE), "").unwrap();
        for f in ["a.stl", "b.STEP", "c.stp", "notes.txt"] {
            std::fs::write(dir.join(f), "").unwrap();
        }
        Dir(dir)
    }

    #[test]
    fn req_ui_23_req_imp_01_req_imp_02_each_dropped_item_is_told_by_what_it_is() {
        let d = place("kinds");
        let p = |n: &str| d.0.join(n);
        assert_eq!(Dropped::of(&p("a.stl")), Dropped::Model(p("a.stl")));
        assert_eq!(Dropped::of(&p("b.STEP")), Dropped::Model(p("b.STEP")));
        assert_eq!(Dropped::of(&p("c.stp")), Dropped::Model(p("c.stp")));
        assert_eq!(Dropped::of(&p("project")), Dropped::Project(p("project")));
        assert_eq!(
            Dropped::of(&p("project").join(PROJECT_FILE)),
            Dropped::Project(p("project"))
        );
        assert_eq!(Dropped::of(&p("plain")), Dropped::Other(p("plain")));
        assert_eq!(Dropped::of(&p("notes.txt")), Dropped::Other(p("notes.txt")));
    }

    #[test]
    fn req_ui_23_the_first_usable_item_is_used_and_the_rest_named() {
        let d = place("plan");
        let p = |n: &str| d.0.join(n);
        let plan = plan(&[p("notes.txt"), p("a.stl"), p("c.stp")]);
        assert_eq!(plan.used, Some(Dropped::Model(p("a.stl"))));
        assert_eq!(plan.rest, ["notes.txt", "c.stp"]);
        assert_eq!(
            message(&plan).unwrap().text,
            "Not imported: notes.txt, c.stp. Drop or import one file at a time."
        );
        assert_eq!(
            hover_text(&plan, true),
            "Drop to import a.stl (1 of 3; the rest are not imported)"
        );
    }

    #[test]
    fn req_ui_23_one_usable_item_has_no_message() {
        let d = place("single");
        let plan = plan(&[d.0.join("a.stl")]);
        assert!(message(&plan).is_none());
    }

    #[test]
    fn req_ui_23_req_nfr_07_an_unusable_drop_says_what_is_taken() {
        let d = place("neither");
        let one = plan(&[d.0.join("notes.txt")]);
        assert_eq!(one.used, None);
        assert_eq!(
            message(&one).unwrap().text,
            "Dropped file: 'notes.txt' is neither a model nor a project. OpenChuckCAM imports \
             STL and STEP files and opens project folders."
        );
        assert_eq!(hover_text(&one, false), WHAT_IS_TAKEN);
        let two = plan(&[d.0.join("notes.txt"), d.0.join("plain")]);
        assert!(
            message(&two)
                .unwrap()
                .text
                .starts_with("Dropped file: 'notes.txt' and 'plain' are neither a model nor")
        );
        let none = plan(&[]);
        assert_eq!(none.used, None);
    }

    #[test]
    fn req_ui_23_the_drag_line_says_what_the_drop_will_do() {
        let d = place("hover");
        let model = plan(&[d.0.join("a.stl")]);
        assert_eq!(hover_text(&model, true), "Drop to import a.stl");
        assert_eq!(
            hover_text(&model, false),
            "Drop to start a new project with a.stl"
        );
        let project = plan(&[d.0.join("project")]);
        assert_eq!(
            hover_text(&project, false),
            "Drop to open the project project"
        );
        assert_eq!(
            hover_text(&project, true),
            "Drop to open the project project"
        );
    }

    #[test]
    fn req_ui_23_a_new_project_is_named_after_the_dropped_model() {
        assert_eq!(project_name(Path::new("/x/demo-shaft.step")), "demo-shaft");
        assert_eq!(project_name(Path::new("/x/part.v2.stl")), "part.v2");
    }
}
