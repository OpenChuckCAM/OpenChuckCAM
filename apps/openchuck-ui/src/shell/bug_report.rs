// SPDX-License-Identifier: AGPL-3.0-or-later
//! Help > Save files for a bug report… (NFR-20; decision PROJUX-B-4): what
//! the file will hold, ticked by the user, listed in full before it is
//! written, then one zip on this computer. Nothing is sent anywhere (NFR-09).

use std::path::PathBuf;

use eframe::egui;
use openchuck_app::bug_report::{self, Choice, Sources};

use super::Shell;
use crate::field_row;
use crate::focus::{KeepFocus as _, Tip as _};
use crate::theme::DARK;

/// The dialog's state: what is ticked, and what the last save said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Draft {
    pub(super) choice: Choice,
    /// The last save's result: the file written, or why it was not.
    pub(super) result: Option<Result<PathBuf, String>>,
}

impl Default for Draft {
    /// The project's own files ticked; model and program files not (the
    /// owner's answer Q4b A, relayed).
    fn default() -> Self {
        Self {
            choice: Choice {
                project: true,
                models: false,
                programs: false,
            },
            result: None,
        }
    }
}

/// The name the file is offered under.
const FILE_NAME: &str = "openchuckcam-bug-report.zip";

impl Shell {
    /// What the file would be made from now: this run's build, the log files
    /// in the log folder, and the open project with its models and the
    /// program files last written.
    pub(super) fn bug_report_sources(&self) -> Sources {
        let logs = crate::logging::log_folder()
            .map(|folder| {
                (0..=crate::logging::KEEP)
                    .map(|i| crate::logging::file_name(&folder, i))
                    .filter(|p| p.is_file())
                    .collect()
            })
            .unwrap_or_default();
        let (project, models) = self.app.as_ref().map_or((None, Vec::new()), |app| {
            let root = app.location().to_path_buf();
            let models = app
                .project()
                .models()
                .iter()
                .map(|m| m.path.resolve(&root))
                .collect();
            (Some(root), models)
        });
        Sources {
            build: openchuck_buildinfo::current_text().to_owned(),
            logs,
            project,
            models,
            programs: self.export.written_files(),
            home: bug_report::home_folder(),
        }
    }

    /// Draws the dialog. Returns true when it is to close.
    pub(super) fn bug_report_dialog(&mut self, ui: &mut egui::Ui, draft: &mut Draft) -> bool {
        let mut close = false;
        ui.heading("Save files for a bug report");
        field_row::note_line(
            ui,
            "One .zip file on this computer. Nothing is sent anywhere; attach it to your report \
             yourself. Issues on GitHub are public.",
            DARK.muted,
        );
        let sources = self.bug_report_sources();
        let has_project = sources.project.is_some();
        ui.add_enabled(false, egui::Checkbox::new(&mut true, "Build information"))
            .tip("Always included: the version and build of the program.");
        ui.add_enabled(
            false,
            egui::Checkbox::new(&mut true, "Log and action history"),
        )
        .tip("Always included: start-up, warnings, errors, and the names of the actions you took, never the values you typed.");
        ui.add_enabled(
            has_project,
            egui::Checkbox::new(
                &mut draft.choice.project,
                "Project settings and operations (no model files)",
            ),
        );
        ui.add_enabled(
            has_project,
            egui::Checkbox::new(&mut draft.choice.models, "Model files"),
        );
        let has_programs = !sources.programs.is_empty();
        ui.add_enabled(
            has_programs,
            egui::Checkbox::new(&mut draft.choice.programs, "Program files last written"),
        );
        let choice = Choice {
            project: draft.choice.project && has_project,
            models: draft.choice.models && has_project,
            programs: draft.choice.programs && has_programs,
        };
        let entries = bug_report::entries(&sources, choice);
        field_row::note_line(
            ui,
            "Folder names show your home folder as \"~\". Do not include a project you do not \
             want to share publicly.",
            DARK.muted,
        );
        egui::CollapsingHeader::new("What the file will hold")
            .id_salt("bug report contents")
            .default_open(true)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(180.0)
                    .show_keeping_focus(ui, |ui| {
                        for (name, size) in bug_report::preview(&entries) {
                            ui.monospace(format!("{name}  ({size} bytes)"));
                        }
                    });
            });
        match &draft.result {
            Some(Ok(path)) => field_row::note_line(
                ui,
                &format!(
                    "Saved {}.",
                    bug_report::without_home(&path.display().to_string(), sources.home.as_deref())
                ),
                DARK.ok,
            ),
            Some(Err(reason)) => field_row::note_line(ui, reason, DARK.error),
            None => {}
        }
        ui.horizontal(|ui| {
            if ui.button("Save…").clicked()
                && let Some(path) = self.bug_report_target()
            {
                draft.result = Some(
                    bug_report::write_zip(&path, &entries, bug_report::FIXED_TIME).map(|()| path),
                );
            }
            if ui.button("Close").clicked() {
                close = true;
            }
        });
        close
    }

    /// Where to save the file: the platform's save dialog, or in tests the
    /// path a test set.
    fn bug_report_target(&mut self) -> Option<PathBuf> {
        #[cfg(test)]
        if let Some(path) = self.bug_report_path.take() {
            return Some(path);
        }
        rfd::FileDialog::new()
            .set_title("Save files for a bug report")
            .add_filter("Zip file", &["zip"])
            .set_file_name(FILE_NAME)
            .save_file()
    }
}
