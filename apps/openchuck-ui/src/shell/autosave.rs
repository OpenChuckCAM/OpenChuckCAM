// SPDX-License-Identifier: AGPL-3.0-or-later
//! Auto-save in the window (PRJ-08; decision PROJUX-B-3, the owner's answer
//! Q3 B with the title-bar switch, relayed): the switch before the project's
//! name, the save-state text beside it, the File menu's tick, the save each
//! frame asks for, and the one Messages warning while saves fail.

use std::time::Instant;

use eframe::egui::{self, Color32, RichText, Sense};
use openchuck_app::autosave::{Frame, Schedule, Step, Wait};
use openchuck_app::lead_in::Context;
use openchuck_app::overview::{Message, MessageAction, Severity};

use super::Shell;
use crate::focus::Tip as _;
use crate::theme::DARK;

/// What the window keeps for auto-save: its clock, measured from when the
/// window started, and what the last frame decided.
#[derive(Debug)]
pub(super) struct State {
    pub(super) schedule: Schedule,
    start: Instant,
    /// What the last frame decided, for the save-state text.
    pub(super) step: Step,
}

impl Default for State {
    fn default() -> Self {
        Self {
            schedule: Schedule::default(),
            start: Instant::now(),
            step: Step::Off,
        }
    }
}

impl State {
    fn now(&self) -> std::time::Duration {
        self.start.elapsed()
    }
}

/// The switch's label.
pub(super) const LABEL: &str = "Auto-save";

/// The save state while saves fail, after the warning mark (the shape the
/// tree uses, so it does not rest on colour alone).
pub(super) const FAILED: &str = "Auto-save failed \u{b7} see Messages";

impl Shell {
    /// Saves the project if auto-save says so this frame (PRJ-08).
    pub(super) fn autosave_frame(&mut self, ctx: &egui::Context) {
        let Some(app) = &self.app else {
            self.autosave.step = Step::Off;
            return;
        };
        let frame = Frame {
            on: self.settings.auto_save,
            modified: app.is_modified(),
            recovered: app.is_recovered_unsaved(),
            typing: !crate::pending_edits::entries(ctx).is_empty(),
            dialog: self.dialog.is_some()
                || self.edit_guard.active()
                || ctx.memory(|m| m.top_modal_layer().is_some()),
        };
        let now = self.autosave.now();
        let minutes = self.settings.auto_save_minutes;
        let step = self.autosave.schedule.step(now, minutes, frame);
        self.autosave.step = step;
        if step != Step::Save {
            return;
        }
        match app.save() {
            Ok(()) => {
                self.recovery.saved(app.revision());
                self.jobs
                    .record(&format!("Saved automatically {}", app.location().display()));
                if self.autosave.schedule.saved(now) {
                    self.overview_dirty = true;
                }
            }
            Err(e) => {
                if self.autosave.schedule.failed(now, minutes, e.to_string()) {
                    self.overview_dirty = true;
                }
            }
        }
        // The title bar's text follows at once.
        self.autosave.step = Step::Idle;
        ctx.request_repaint();
    }

    /// The warning while automatic saves fail: one line, kept until a save
    /// works, not one per try.
    pub(super) fn autosave_message(&self) -> Option<Message> {
        let reason = self.autosave.schedule.failure()?;
        let folder = self
            .app
            .as_ref()
            .map(|a| a.location().display().to_string())
            .unwrap_or_default();
        Some(Message::about(
            Severity::Warning,
            "Auto-save",
            &Context::default(),
            &format!(
                "the project could not be saved automatically in {folder} ({reason}). It is \
                 tried again at each interval; your changes are in memory and in the recovery \
                 copies. Save the project to another folder, or free the folder, to keep them"
            ),
            None,
            Some(MessageAction::Save),
        ))
    }

    /// The title bar's switch, before the project's name (specification
    /// section 3): a real control in the Tab order, Space toggles it, its
    /// tooltip opens on focus too (decision 0677).
    pub(super) fn autosave_switch(&mut self, ui: &mut egui::Ui) {
        let on = self.settings.auto_save;
        ui.label(RichText::new(LABEL).color(DARK.text));
        let size = egui::vec2(34.0, 18.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        response.widget_info(|| egui::WidgetInfo::selected(egui::Role::CheckBox, true, on, LABEL));
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let (fill, knob_x) = if on {
                (DARK.accent, rect.right() - 9.0)
            } else {
                (DARK.field, rect.left() + 9.0)
            };
            painter.rect(
                rect,
                9.0,
                fill,
                egui::Stroke::new(1.0, if on { DARK.accent } else { DARK.muted }),
                egui::StrokeKind::Inside,
            );
            painter.circle_filled(
                egui::pos2(knob_x, rect.center().y),
                6.0,
                if on { Color32::WHITE } else { DARK.muted },
            );
            if response.has_focus() {
                painter.rect_stroke(
                    rect.expand(2.0),
                    10.0,
                    egui::Stroke::new(2.0, DARK.accent),
                    egui::StrokeKind::Outside,
                );
            }
        }
        ui.label(
            RichText::new(if on { "On" } else { "Off" })
                .small()
                .color(DARK.muted),
        );
        let minutes = self.settings.auto_save_minutes;
        let response = response.tip(format!(
            "Save the project every {minutes} minutes while it has changes (File > Settings)"
        ));
        if response.clicked() {
            self.set_auto_save(!on);
        }
    }

    /// Turns auto-save on or off, for every project; remembered per user.
    pub(super) fn set_auto_save(&mut self, on: bool) {
        self.settings.auto_save = on;
        self.settings_touched();
    }

    /// The save-state text beside the project's name, with its colour and
    /// tooltip, as auto-save stands (specification section 10).
    pub(super) fn save_state_text(&self, modified: bool) -> (String, Color32, Option<String>) {
        if self.settings.auto_save {
            let now = self.autosave.now();
            if let Some(reason) = self.autosave.schedule.failure() {
                return (
                    FAILED.to_owned(),
                    DARK.warning,
                    Some(format!("The last automatic save failed: {reason}.")),
                );
            }
            if modified {
                let text = match self.autosave.step {
                    Step::Waiting(Wait::Recovered) => {
                        "Recovered, not saved \u{b7} auto-save waits for File > Save"
                    }
                    Step::Waiting(Wait::Typing) => {
                        "Edited \u{b7} next auto-save waits for the typed value"
                    }
                    Step::Waiting(Wait::Dialog) => {
                        "Edited \u{b7} next auto-save waits for the open dialog"
                    }
                    _ => "Edited, saved automatically soon",
                };
                return (text.to_owned(), DARK.muted, Some(self.autosave_tip()));
            }
            if let Some(last) = self.autosave.schedule.last() {
                return (
                    format!(
                        "\u{2713} Saved automatically {}",
                        openchuck_app::ago(now.saturating_sub(last))
                    ),
                    DARK.muted,
                    Some(self.autosave_tip()),
                );
            }
        }
        let (text, tip) = crate::window_title::save_state(modified);
        (text.to_owned(), DARK.muted, tip.map(str::to_owned))
    }

    fn autosave_tip(&self) -> String {
        format!(
            "Auto-save is on: the project is saved every {} minutes while it has changes.",
            self.settings.auto_save_minutes
        )
    }
}
