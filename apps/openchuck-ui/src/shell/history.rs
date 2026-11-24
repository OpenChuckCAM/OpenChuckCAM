// SPDX-License-Identifier: AGPL-3.0-or-later
//! The undo history popover (UI-22; specification section 12.3; decision
//! PROJUX-B-1): opened from the title bar's history button or Ctrl H, it
//! lists every step newest first, with the step the project is at
//! highlighted, undone steps in italics marked "undone", the last save
//! marked, and the project as opened at the foot. Choosing a row undoes or
//! redoes to it, through the same guard as Undo (decision 0233).

use eframe::egui::{self, RichText};
use openchuck_app::ago;
use openchuck_app::history::UndoHistory;

use super::Shell;
use crate::focus::{KeepFocus as _, Tip as _};
use crate::theme::DARK;

/// The popover's width, as in the specification's mock-up.
const WIDTH: f32 = 360.0;

/// The tallest the list grows before it scrolls.
const LIST_HEIGHT: f32 = 400.0;

/// What a click on the history's list logs instead of the row's text, which
/// can hold a value the user typed (NFR-18, decision 0157).
pub(super) const ROW_CLICK: &str = "click: 'Undo history step'";

/// One row of the list, newest first.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Row {
    /// A step: the position it returns to, its text, the time or "undone",
    /// and whether it is undone.
    Step {
        position: usize,
        label: String,
        right: String,
        undone: bool,
    },
    /// The line under the step the project was last saved at.
    Saved(String),
}

/// The rows of `history`, newest first: each step, the saved line under the
/// step it was made at, and the project as opened last.
pub(super) fn rows(history: &UndoHistory) -> Vec<Row> {
    let mut rows = Vec::new();
    let saved_line = |rows: &mut Vec<Row>, position: usize| {
        if let Some(mark) = history.saved.filter(|m| m.position == position) {
            rows.push(Row::Saved(format!("Saved {}", ago(mark.age))));
        }
    };
    for (index, step) in history.steps.iter().enumerate().rev() {
        let position = index + 1;
        rows.push(Row::Step {
            position,
            label: step.label.clone(),
            right: if step.undone {
                "undone".to_owned()
            } else {
                ago(step.age)
            },
            undone: step.undone,
        });
        saved_line(&mut rows, position);
    }
    rows.push(Row::Step {
        position: 0,
        label: history.start.to_owned(),
        right: ago(history.start_age),
        undone: false,
    });
    saved_line(&mut rows, 0);
    rows
}

impl Shell {
    /// The title bar's history button and its popover (specification
    /// sections 3 and 12.3).
    pub(super) fn history_button(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let available = self.app.is_some();
        let button = ui
            .add_enabled(available, egui::Button::new("\u{2630}"))
            .tip(format!(
                "Undo history ({})",
                ctx.format_shortcut(&super::HISTORY)
            ));
        if button.clicked() {
            self.history_open = !self.history_open;
        }
        if !available {
            self.history_open = false;
            return;
        }
        let mut open = self.history_open;
        // Opened in this frame from the Edit menu or Ctrl H: the click that
        // chose the menu item is not a click outside the list.
        let just_opened = self.history_focus;
        let mut chosen = None;
        // A click outside the list and its button closes it. The button's
        // own click toggles it above, so egui's close-on-click-outside,
        // which would also see that click, is not used.
        let shown = egui::Popup::from_response(&button)
            .open_bool(&mut open)
            .close_behavior(egui::PopupCloseBehavior::IgnoreClicks)
            .width(WIDTH)
            .show(|ui| chosen = self.history_list(ui));
        if let Some(shown) = shown.filter(|_| !just_opened) {
            let outside = ui.input(|i| {
                i.pointer.any_click()
                    && i.pointer.interact_pos().is_some_and(|p| {
                        !shown.response.rect.contains(p) && !button.rect.contains(p)
                    })
            });
            if outside {
                open = false;
            }
        }
        self.history_open = open;
        if let Some((position, label)) = chosen {
            self.history_clicked = Some(label);
            self.go_to_step(position);
        }
    }

    /// The list. Returns the row chosen: its position and the text its
    /// button reports.
    fn history_list(&mut self, ui: &mut egui::Ui) -> Option<(usize, String)> {
        let history = self.app.as_ref()?.undo_history();
        let ctx = ui.ctx().clone();
        ui.set_width(WIDTH);
        ui.horizontal(|ui| {
            ui.label(RichText::new("UNDO HISTORY").strong().color(DARK.heading));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new("Choose a step to go back to it").color(DARK.muted));
            });
        });
        ui.separator();
        let mut chosen = None;
        let focus_current = std::mem::take(&mut self.history_focus);
        egui::ScrollArea::vertical()
            .max_height(LIST_HEIGHT)
            .show_keeping_focus(ui, |ui| {
                for row in rows(&history) {
                    match row {
                        Row::Step {
                            position,
                            label,
                            right,
                            undone,
                        } => {
                            let current = position == history.position;
                            let mut text = RichText::new(&label);
                            if undone {
                                text = text.italics().color(DARK.muted);
                            }
                            let response = ui.add_sized(
                                [ui.available_width(), 24.0],
                                egui::Button::selectable(current, text).shortcut_text(&right),
                            );
                            if current && focus_current {
                                response.request_focus();
                            }
                            if response.clicked() && !current {
                                // The button's accessible text, which the
                                // action log reads: the step and its time.
                                chosen = Some((position, format!("{label} {right}")));
                            }
                        }
                        Row::Saved(text) => {
                            ui.label(RichText::new(text).small().color(DARK.ok));
                        }
                    }
                }
            });
        ui.separator();
        ui.label(
            RichText::new(format!(
                "{} undo · {} redo. A new edit after undoing discards the undone steps.",
                ctx.format_shortcut(&super::UNDO),
                ctx.format_shortcut(&super::REDO_ALT),
            ))
            .small()
            .color(DARK.muted),
        );
        chosen
    }

    /// Returns to a step of the undo history, first resolving unconfirmed
    /// fields as Undo does (decision 0233).
    pub(super) fn go_to_step(&mut self, position: usize) {
        if self.defer(move |s| s.go_to_step(position)) {
            return;
        }
        if let Some(app) = &mut self.app {
            match app.go_to_step(position) {
                Ok(()) => {
                    self.form.clear();
                    self.rebuild_scene(false);
                }
                Err(e) => {
                    self.dialog = Some(super::Dialog::Error(crate::failure::Failure::new(
                        "Undo history",
                        e,
                    )));
                }
            }
        }
    }
}
