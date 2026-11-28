// SPDX-License-Identifier: AGPL-3.0-or-later
//! How the main window opens and how its state is read back (UI-15, UI-16;
//! specification section 3, "Opening the window").
//!
//! The window opens maximised the first time and when it was closed
//! maximised; otherwise it opens at the normal size it had, centred on the
//! screen (eframe's `centered`, on the primary monitor). Its position is never
//! kept, so it cannot open off the screen. Wayland does not let an
//! application place its window; there the compositor places it.

use eframe::egui::{ViewportBuilder, ViewportInfo};
use openchuck_app::settings::{WindowStart, WindowState};

/// The window's builder for `start`. A maximised window is also given its
/// normal size, so un-maximising goes back to it, centred.
pub(crate) fn opening(viewport: ViewportBuilder, start: WindowStart) -> ViewportBuilder {
    match start {
        WindowStart::Maximised { restore } => {
            let viewport = viewport.with_maximized(true);
            match restore {
                Some(size) => viewport.with_inner_size(size),
                None => viewport,
            }
        }
        WindowStart::Centred(size) => viewport.with_inner_size(size),
    }
}

/// The window's state as the platform reports it. A state the platform does
/// not report reads as a normal window.
pub(crate) fn state(info: &ViewportInfo) -> WindowState {
    if info.minimized == Some(true) {
        WindowState::Minimised
    } else if info.fullscreen == Some(true) {
        WindowState::Fullscreen
    } else if info.maximized == Some(true) {
        WindowState::Maximised
    } else {
        WindowState::Normal
    }
}

#[cfg(test)]
mod tests {
    use eframe::egui;

    use super::*;

    /// The first start, and a window closed maximised, open maximised; a
    /// window closed at a normal size opens at it (UI-15, UI-16).
    #[test]
    fn req_ui_15_req_ui_16_the_window_opens_maximised_or_at_its_size() {
        let first = opening(
            ViewportBuilder::default(),
            WindowStart::Maximised { restore: None },
        );
        assert_eq!(first.maximized, Some(true));
        assert_eq!(first.inner_size, None);

        let again = opening(
            ViewportBuilder::default(),
            WindowStart::Maximised {
                restore: Some([1200.0, 800.0]),
            },
        );
        assert_eq!(again.maximized, Some(true));
        assert_eq!(again.inner_size, Some(egui::vec2(1200.0, 800.0)));

        let normal = opening(
            ViewportBuilder::default(),
            WindowStart::Centred([1400.0, 900.0]),
        );
        assert_ne!(normal.maximized, Some(true));
        assert_eq!(normal.inner_size, Some(egui::vec2(1400.0, 900.0)));
        // No position is given: eframe centres the window (`centered`).
        assert_eq!(normal.position, None);
    }

    /// Minimised and full screen win over maximised; nothing reported is a
    /// normal window (UI-15).
    #[test]
    fn req_ui_15_the_window_state_is_read_from_the_platform() {
        let info = |minimized, maximized, fullscreen| ViewportInfo {
            minimized,
            maximized,
            fullscreen,
            ..ViewportInfo::default()
        };
        assert_eq!(state(&info(None, None, None)), WindowState::Normal);
        assert_eq!(
            state(&info(Some(false), Some(true), Some(false))),
            WindowState::Maximised
        );
        assert_eq!(
            state(&info(Some(true), Some(true), None)),
            WindowState::Minimised
        );
        assert_eq!(
            state(&info(None, Some(true), Some(true))),
            WindowState::Fullscreen
        );
    }
}
