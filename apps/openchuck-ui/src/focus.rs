// SPDX-License-Identifier: AGPL-3.0-or-later
//! Keyboard focus that the user can see (UI-14; specification section 16):
//! a control that Tab reaches is scrolled into view (issue #577), and its
//! tooltip opens on focus as well as on hover (issue #578). egui 0.37 does
//! neither by itself, so every scroll area in the application is shown
//! through [`KeepFocus::show_keeping_focus`], and every tooltip is given with
//! [`Tip::tip`] or [`Tip::tip_ui`].

use eframe::egui;

/// Scrolls the control that has just taken the keyboard into view, when it is
/// drawn in the scroll area whose contents `ui` holds. Called after the
/// contents, so the control's place is known. The scroll is not animated:
/// egui makes it when the next frame starts, and the control is drawn in view
/// from the frame after that.
pub fn keep_focus_in_view(ui: &egui::Ui) {
    let ctx = ui.ctx();
    let Some(id) = ctx.memory(egui::Memory::focused) else {
        return;
    };
    let Some(response) = ctx.read_response(id) else {
        return;
    };
    if !response.gained_focus() {
        return;
    }
    if response.layer_id != ui.layer_id() || !ui.min_rect().contains_rect(response.rect) {
        return;
    }
    ui.scroll_to_rect_animation(response.rect, None, egui::style::ScrollAnimation::none());
}

/// A scroll area that keeps the control with the keyboard in view.
pub trait KeepFocus {
    /// As [`egui::ScrollArea::show`], then [`keep_focus_in_view`].
    fn show_keeping_focus<R>(
        self,
        ui: &mut egui::Ui,
        contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::scroll_area::ScrollAreaOutput<R>;
}

impl KeepFocus for egui::ScrollArea {
    fn show_keeping_focus<R>(
        self,
        ui: &mut egui::Ui,
        contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::scroll_area::ScrollAreaOutput<R> {
        self.show(ui, |ui| {
            let inner = contents(ui);
            keep_focus_in_view(ui);
            inner
        })
    }
}

/// A tooltip that opens on hover and on keyboard focus (specification section
/// 16: "Tooltips and help bubbles open on focus as well as hover").
pub trait Tip: Sized {
    /// As [`egui::Response::on_hover_text`], and shown at once while the
    /// control has the keyboard.
    fn tip(self, text: impl Into<egui::WidgetText>) -> Self;
    /// As [`egui::Response::on_hover_ui`], and shown at once while the
    /// control has the keyboard.
    fn tip_ui(self, contents: impl FnOnce(&mut egui::Ui)) -> Self;
}

impl Tip for egui::Response {
    fn tip(self, text: impl Into<egui::WidgetText>) -> Self {
        let text = text.into();
        self.tip_ui(|ui| {
            // As egui's own hover text: a tooltip as wide as egui's.
            ui.set_max_width(ui.spacing().tooltip_width);
            ui.add(egui::Label::new(text));
        })
    }

    fn tip_ui(self, contents: impl FnOnce(&mut egui::Ui)) -> Self {
        // One or the other, so the two never stack. A text field clicked
        // into has the keyboard too, and shows its tooltip at once.
        if self.has_focus() {
            self.show_tooltip_ui(contents);
            self
        } else {
            self.on_hover_ui(contents)
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;

    /// UI-14 (issue #577): Tab to a control below the visible part of a
    /// scroll area scrolls it into view, in the frame of the key.
    #[test]
    fn req_ui_14_tab_scrolls_the_focused_control_into_view() {
        let ctx = egui::Context::default();
        let mut rects = Vec::new();
        let run = |events: Vec<egui::Event>, rects: &mut Vec<egui::Rect>| {
            let mut viewport = egui::Rect::NOTHING;
            let mut focused = None;
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(400.0, 300.0),
                )),
                events,
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| {
                let out = egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show_keeping_focus(ui, |ui| {
                        rects.clear();
                        for i in 0..40 {
                            rects.push(ui.button(format!("Button {i}")).rect);
                        }
                    });
                viewport = out.inner_rect;
                // Read in the pass: after it, egui answers from the pass before.
                focused = ctx
                    .memory(egui::Memory::focused)
                    .and_then(|id| ctx.read_response(id))
                    .map(|r| r.rect);
            });
            output.textures_delta.clear();
            (viewport, focused)
        };
        let tab = || {
            let key = |pressed| egui::Event::Key {
                key: egui::Key::Tab,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            };
            vec![key(true), key(false)]
        };
        run(Vec::new(), &mut rects);
        let mut viewport = egui::Rect::NOTHING;
        for _ in 0..30 {
            run(tab(), &mut rects);
            run(Vec::new(), &mut rects);
            let focused;
            (viewport, focused) = run(Vec::new(), &mut rects);
            let focused = focused.unwrap();
            assert!(
                viewport.contains_rect(focused),
                "{focused:?} is out of view in {viewport:?}"
            );
        }
        // It did scroll: the first button has left the view.
        assert!(!viewport.contains_rect(rects[0]));
    }

    /// A tooltip given with `tip` is shown while its control has the
    /// keyboard, without the pointer anywhere near (issue #578).
    #[test]
    fn req_ui_14_a_tip_opens_on_keyboard_focus() {
        let ctx = egui::Context::default();
        let mut shown = false;
        for frame in 0..4 {
            let events = if frame == 1 {
                let key = |pressed| egui::Event::Key {
                    key: egui::Key::Tab,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                };
                vec![key(true), key(false)]
            } else {
                Vec::new()
            };
            let input = egui::RawInput {
                events,
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| {
                ui.button("Fit").tip("Fit the view to the part (F)");
            });
            output.textures_delta.clear();
            shown = output.shapes.iter().any(|s| {
                fn has(shape: &egui::Shape) -> bool {
                    match shape {
                        egui::Shape::Text(t) => t.galley.text() == "Fit the view to the part (F)",
                        egui::Shape::Vec(v) => v.iter().any(has),
                        _ => false,
                    }
                }
                has(&s.shape)
            });
            if frame == 0 {
                assert!(!shown, "no tooltip before the control has the keyboard");
            }
        }
        assert!(shown, "the tooltip opened on focus");
    }

    /// Every tooltip in the application opens on focus too: none is given
    /// with egui's hover-only calls. A disabled control takes no focus in
    /// egui, so its hover text (`on_disabled_hover_text`) is not counted.
    #[test]
    fn req_ui_14_every_tooltip_also_opens_on_focus() {
        let mut hover_only = Vec::new();
        for (name, text) in sources() {
            for call in [".on_hover_text(", ".on_hover_ui("] {
                for (at, _) in text.match_indices(call) {
                    let line = text[..at].lines().count();
                    hover_only.push(format!("{name}:{line}"));
                }
            }
        }
        assert!(
            hover_only.is_empty(),
            "hover-only tooltips: {hover_only:#?}"
        );
    }

    /// The application's source files, by name, without the test files and
    /// this one.
    fn sources() -> Vec<(String, String)> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut stack = vec![root];
        let mut files = Vec::new();
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                if name.ends_with(".rs") && !name.ends_with("_tests.rs") && name != "focus.rs" {
                    let text = std::fs::read_to_string(&path).unwrap();
                    // Unit tests at the foot of a file are not application code.
                    let code = text
                        .split("#[cfg(test)]\nmod tests")
                        .next()
                        .unwrap_or_default();
                    files.push((name, code.to_owned()));
                }
            }
        }
        files
    }

    /// Every scroll area in the application keeps focus in view: none is
    /// shown with egui's plain `show`.
    #[test]
    fn req_ui_14_every_scroll_area_keeps_the_focused_control_in_view() {
        let mut plain = Vec::new();
        for (name, text) in sources() {
            for (at, _) in text.match_indices("ScrollArea::") {
                let rest = &text[at..];
                let show = [
                    ".show(",
                    ".show_keeping_focus(",
                    ".show_rows(",
                    ".show_viewport(",
                ]
                .iter()
                .filter_map(|call| rest.find(call))
                .min()
                .unwrap();
                if !rest[show..].starts_with(".show_keeping_focus(") {
                    plain.push(format!("{name}: {}", &rest[..show.min(60)]));
                }
            }
        }
        assert!(
            plain.is_empty(),
            "scroll areas shown without keeping focus: {plain:#?}"
        );
    }
}
