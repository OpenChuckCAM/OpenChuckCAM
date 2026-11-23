// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tool dialogs (import, tool library, export) as windows inside the main
//! window, sized to fit it (UI-16; decision record 0164, which replaces item
//! 17 of decision 0029). A dialog is an egui window drawn in the main window's
//! frame, so it may borrow the shell's state, cannot be left behind the main
//! window, and can be dragged only within it. A dialog draws no 3D: a preview
//! stays in the main window's viewport.

use crate::focus::KeepFocus as _;
use eframe::egui;

/// What the user did to the dialog's window this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowEvent {
    /// Nothing.
    None,
    /// The user closed the dialog's window (its title-bar close button).
    CloseRequested,
}

/// Shows a dialog titled `title`, `size` points large at first, inside the
/// main window. `contents` draws it and its result is returned with what
/// happened to the window. `key` identifies the dialog across frames.
#[cfg(test)]
pub fn floating<T: Default>(
    ctx: &egui::Context,
    key: &str,
    title: &str,
    size: [f32; 2],
    contents: impl FnMut(&mut egui::Ui) -> T,
) -> (T, WindowEvent) {
    run(ctx, key, title, size, true, true, contents)
}

/// Like [`floating`], but the contents are not put in a scroll area, so a
/// dialog can pin its own footer with a bottom panel and scroll the rest.
pub fn floating_unscrolled<T: Default>(
    ctx: &egui::Context,
    key: &str,
    title: &str,
    size: [f32; 2],
    contents: impl FnMut(&mut egui::Ui) -> T,
) -> (T, WindowEvent) {
    run(ctx, key, title, size, true, false, contents)
}

/// Like [`floating_unscrolled`], but the window has a fixed size: the user
/// cannot resize it (UI-16). The contents scroll inside it, with a footer
/// pinned if the dialog wants one.
pub fn floating_fixed<T: Default>(
    ctx: &egui::Context,
    key: &str,
    title: &str,
    size: [f32; 2],
    contents: impl FnMut(&mut egui::Ui) -> T,
) -> (T, WindowEvent) {
    run(ctx, key, title, size, false, false, contents)
}

/// The id of the window of the dialog keyed `key`.
fn window_id(key: &str) -> egui::Id {
    egui::Id::unique(("dialog", key))
}

/// The layer the dialog keyed `key` is drawn on.
pub fn window_layer(key: &str) -> egui::LayerId {
    egui::LayerId::new(egui::Order::Middle, window_id(key))
}

/// The key under which a dialog's window keeps whether it waits for the first
/// Tab (it opened and no control in it has had the keyboard yet), and the
/// frame it was last drawn in.
fn armed_id(window: egui::Id) -> egui::Id {
    window.with("first tab enters")
}

fn shown_id(window: egui::Id) -> egui::Id {
    window.with("last drawn in pass")
}

/// Whether Tab or Shift Tab was pressed this frame.
fn tab_pressed(ctx: &egui::Context) -> bool {
    ctx.input(|i| i.key_pressed(egui::Key::Tab))
}

/// Marks where a dialog's body starts (UI-14, specification section 16; issue
/// #580; the owner's answer Q1 B of 23 Nov 2026). A dialog that has opened is
/// focused as a whole, with no control: the first Tab after it opens goes to
/// the first control drawn after this call, whatever egui would have given it
/// to behind the window. A dialog calls it before its body, so the first Tab
/// reaches the body's first control and not the title bar's close box or the
/// footer's buttons.
pub fn focus_starts_here(ui: &egui::Ui) {
    let armed = ui.data(|d| d.get_temp::<bool>(armed_id(ui.layer_id().id))) == Some(true);
    if !armed || ui.is_sizing_pass() || !ui.is_visible() || !tab_pressed(ui.ctx()) {
        return;
    }
    let layer = ui.layer_id();
    let inside = ui
        .memory(egui::Memory::focused)
        .and_then(|focused| ui.ctx().read_response(focused))
        .is_some_and(|r| r.layer_id == layer);
    if !inside {
        ui.memory_mut(|m| {
            m.stop_text_input();
            m.move_focus(egui::FocusDirection::Next);
        });
    }
}

/// Before a dialog's window is drawn: if it was not drawn in the frame
/// before, it has just opened. Nothing behind it keeps the keyboard (a field
/// that had it sees it lost and applies its value, as on any focus loss), and
/// the dialog waits for the first Tab.
fn begin_opening(ctx: &egui::Context, window: egui::Id) {
    let pass = ctx.cumulative_pass_nr();
    let previous = ctx.data_mut(|d| {
        let previous = d.get_temp::<u64>(shown_id(window));
        d.insert_temp(shown_id(window), pass);
        previous
    });
    // Not drawn in the frame before: closed until now.
    if previous.is_none_or(|p| p.saturating_add(1) < pass) {
        ctx.memory_mut(egui::Memory::stop_text_input);
        ctx.data_mut(|d| d.insert_temp(armed_id(window), true));
    }
}

/// After a dialog's window is drawn: it stops waiting for the first Tab once
/// a control in it has the keyboard, or once the user has given the keyboard
/// to something else without Tab (a click behind the window).
fn end_opening(ctx: &egui::Context, window: egui::Id) {
    let id = armed_id(window);
    if ctx.data(|d| d.get_temp::<bool>(id)) != Some(true) {
        return;
    }
    let tab = tab_pressed(ctx);
    if tab {
        // The Tab is not carried past the dialog's body.
        ctx.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
    }
    let focused = ctx
        .memory(egui::Memory::focused)
        .and_then(|focused| ctx.read_response(focused));
    let done = match focused {
        Some(r) => r.layer_id.id == window || !tab,
        None => false,
    };
    if done {
        ctx.data_mut(|d| d.insert_temp(id, false));
    }
}

/// The key of what Esc found at the end of the last frame: whether a text
/// field had the keyboard, and whether a pop-up was open.
fn esc_state_id() -> egui::Id {
    egui::Id::unique("dialog esc state")
}

/// Whether Esc this frame closes the dialog on `layer` (UI-14, specification
/// section 16; the owner's answer Q6 A, issue #581 F11): Esc does what the
/// dialog's Close or Cancel does, through the same guard. Not when a text
/// field had the keyboard at the end of the last frame (that Esc only ends
/// the edit; egui lets go of the field before this frame is drawn), not when
/// a pop-up was open (that Esc closes the pop-up), not while a question is
/// open in front of the dialog, and only for the dialog on top.
fn esc_closes(ctx: &egui::Context, layer: egui::LayerId) -> bool {
    if !ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        return false;
    }
    let (text, popup) = ctx
        .data(|d| d.get_temp::<(bool, bool)>(esc_state_id()))
        .unwrap_or_default();
    let (modal, top) = ctx.memory(|m| {
        (
            m.top_modal_layer(),
            m.areas().top_layer_id(egui::Order::Middle),
        )
    });
    !text && !popup && modal.is_none_or(|m| m == layer) && top.is_none_or(|t| t == layer)
}

/// Notes, after a dialog is drawn, what the next frame's Esc has to know.
fn note_esc_state(ctx: &egui::Context) {
    let state = (ctx.text_edit_focused(), egui::Popup::is_any_open(ctx));
    ctx.data_mut(|d| d.insert_temp(esc_state_id(), state));
}

/// The room a dialog leaves round it inside the main window, in points.
const MARGIN: f32 = 24.0;

/// The size a dialog asking for `size` gets in a main window whose inner area is `inner`, and
/// where its top left goes so that it is centred on the main window's `outer` area. A dialog
/// never opens larger than the window it belongs to (UI-16), and never below the smallest a
/// resizable dialog may be; with an unknown window it keeps the size asked for and the system
/// places it.
pub fn fitted(
    size: [f32; 2],
    inner: Option<egui::Rect>,
    outer: Option<egui::Rect>,
) -> ([f32; 2], Option<egui::Pos2>) {
    let Some(inner) = inner else {
        return (size, None);
    };
    // Never larger than the window, however small it is (UI-16).
    let fit = [
        size[0].min((inner.width() - 2.0 * MARGIN).max(1.0)),
        size[1].min((inner.height() - 2.0 * MARGIN).max(1.0)),
    ];
    let centre = outer.unwrap_or(inner).center();
    (fit, Some(centre - egui::vec2(fit[0], fit[1]) / 2.0))
}

/// The size a dialog asking for `size` gets in the main window `ctx` draws,
/// and the position that centres it.
fn place(ctx: &egui::Context, size: [f32; 2]) -> ([f32; 2], egui::Pos2) {
    let screen = ctx.content_rect();
    let (fit, position) = fitted(size, Some(screen), Some(screen));
    (fit, position.unwrap_or(screen.center()))
}

fn run<T: Default>(
    ctx: &egui::Context,
    key: &str,
    title: &str,
    size: [f32; 2],
    resizable: bool,
    scroll: bool,
    mut contents: impl FnMut(&mut egui::Ui) -> T,
) -> (T, WindowEvent) {
    let (fit, position) = place(ctx, size);
    begin_opening(ctx, window_id(key));
    let mut open = true;
    let window = egui::Window::new(title)
        .id(window_id(key))
        .open(&mut open)
        .collapsible(false)
        .default_pos(position);
    let window = if resizable {
        window
            .default_size(fit)
            .min_size([fit[0].min(240.0), fit[1].min(160.0)])
            .resizable(true)
    } else {
        window.fixed_size(fit).resizable(false)
    };
    let shown = window.show(ctx, |ui| {
        crate::pending_edits::form(ctx, egui::Id::unique(("dialog form", key)), || {
            if scroll {
                egui::ScrollArea::vertical()
                    .show_keeping_focus(ui, &mut contents)
                    .inner
            } else {
                contents(ui)
            }
        })
    });
    end_opening(ctx, window_id(key));
    let esc = esc_closes(ctx, window_layer(key));
    note_esc_state(ctx);
    let value = shown.and_then(|r| r.inner).unwrap_or_default();
    let event = if open && !esc {
        WindowEvent::None
    } else {
        WindowEvent::CloseRequested
    };
    (value, event)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp, clippy::unwrap_used)]

    use super::*;

    /// UI-16: a dialog never opens larger than the window it belongs to, and is centred on it.
    #[test]
    fn req_ui_16_a_dialog_is_fitted_inside_the_window_and_centred_on_it() {
        let window = egui::Rect::from_min_size(egui::pos2(100.0, 50.0), egui::vec2(1366.0, 768.0));
        let (size, position) = fitted([1200.0, 760.0], Some(window), Some(window));
        assert_eq!(size, [1200.0, 720.0], "the height leaves the margin");
        let at = position.unwrap();
        let dialog = egui::Rect::from_min_size(at, egui::vec2(size[0], size[1]));
        assert!(window.contains_rect(dialog), "{dialog:?} in {window:?}");
        assert_eq!(dialog.center(), window.center());
        // A small dialog keeps its size and is centred.
        let (size, position) = fitted([460.0, 520.0], Some(window), Some(window));
        assert_eq!(size, [460.0, 520.0]);
        assert_eq!(
            egui::Rect::from_min_size(position.unwrap(), egui::vec2(460.0, 520.0)).center(),
            window.center()
        );
        // A tiny window still holds the dialog whole.
        let tiny = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(200.0, 100.0));
        let (size, position) = fitted([1200.0, 760.0], Some(tiny), None);
        let dialog = egui::Rect::from_min_size(position.unwrap(), egui::vec2(size[0], size[1]));
        assert!(tiny.contains_rect(dialog), "{dialog:?} in {tiny:?}");
        // An unknown window leaves the size and the place to the system.
        assert_eq!(fitted([300.0, 200.0], None, None), ([300.0, 200.0], None));
    }

    #[test]
    fn req_ui_01_floating_dialog_draws_its_contents_inside_the_main_window() {
        // The dialog is an egui window in the main window's own frame
        // (decision 0164): it is drawn, and closing it is left to its
        // buttons and its close box.
        let ctx = egui::Context::default();
        let mut drawn = 0;
        let mut event = None;
        for _ in 0..2 {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                let (value, e) = floating(ui.ctx(), "test dialog", "Test", [300.0, 200.0], |ui| {
                    ui.label("inside");
                    7
                });
                drawn += value;
                event = Some(e);
            });
            // No renderer here takes the font texture; egui asks for it to
            // be dropped on purpose.
            output.textures_delta.clear();
        }
        assert_eq!(drawn, 14);
        assert_eq!(event, Some(WindowEvent::None));
    }
}
