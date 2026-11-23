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

/// The order Tab takes through a window, by region (UI-14, specification
/// section 16 "in reading order"; the owner's answer Q3 A of 23 Nov 2026,
/// issue #581 F4). egui moves focus in the order the widgets are drawn, and a
/// window's panels must be drawn before its middle, so the drawing order is
/// not the reading order. Each region's drawing is wrapped in [`region`]; when
/// Tab (or Shift Tab) would take the keyboard out of a region, a stand-in
/// widget with no place on screen holds it for one frame, and the next frame
/// gives it to the first (or last) control of the next (or previous) region
/// in the scope's order, skipping a region with no control or not drawn.
/// Inside a region the drawing order stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    /// The window (or dialog) the order belongs to.
    pub scope: &'static str,
    /// The region's place in the scope's reading order.
    pub index: usize,
    /// How many regions the scope has.
    pub count: usize,
}

/// The main window's regions in reading order (Q3 A): menus, step tabs with
/// the G-code status, the health badge, the workflow actions, the project
/// panel, the view's toolbar and the view, the dock, the inspector, the
/// footer with the primary button last.
pub mod main_window {
    use super::Region;
    const fn at(index: usize) -> Region {
        Region {
            scope: "main window",
            index,
            count: 9,
        }
    }
    pub const MENUS: Region = at(0);
    pub const STEPS: Region = at(1);
    pub const BADGE: Region = at(2);
    pub const ACTIONS: Region = at(3);
    pub const PROJECT: Region = at(4);
    pub const VIEW: Region = at(5);
    pub const DOCK: Region = at(6);
    pub const INSPECTOR: Region = at(7);
    pub const FOOTER: Region = at(8);
}

/// Part `index` of the `count` parts of a dialog, in reading order: its
/// body (and a header or list before it), then its footer last (Q3 A). Tab
/// goes round inside the dialog.
pub const fn dialog_part(scope: &'static str, index: usize, count: usize) -> Region {
    Region {
        scope,
        index,
        count,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Way {
    Next,
    Previous,
}

/// What the router keeps from frame to frame.
#[derive(Debug, Clone, Default)]
struct Router {
    /// The region the focused control was in at the end of the last frame.
    focused_in: Option<(&'static str, usize)>,
    /// The way Tab went this frame, and the region it left from.
    crossing: Option<(Way, Region)>,
    /// The region that is to take the keyboard, and how many regions are left
    /// to try.
    pending: Option<(Way, Region, usize)>,
    /// The regions drawn this frame, with their rectangles and layers.
    drawn: Vec<(Region, egui::Rect, egui::LayerId)>,
}

fn router_id() -> egui::Id {
    egui::Id::unique("focus router")
}

/// The stand-in that holds the keyboard between two regions for a frame.
fn stand_in() -> egui::Id {
    egui::Id::unique("focus router stand-in")
}

fn load(ctx: &egui::Context) -> Router {
    ctx.data(|d| d.get_temp::<Router>(router_id()))
        .unwrap_or_default()
}

fn store(ctx: &egui::Context, router: Router) {
    ctx.data_mut(|d| d.insert_temp(router_id(), router));
}

/// The way Tab goes this frame, if it was pressed.
fn tab_way(ctx: &egui::Context) -> Option<Way> {
    ctx.input(|i| {
        i.events.iter().find_map(|e| match e {
            egui::Event::Key {
                key: egui::Key::Tab,
                pressed: true,
                modifiers,
                ..
            } if !modifiers.any() => Some(Way::Next),
            egui::Event::Key {
                key: egui::Key::Tab,
                pressed: true,
                modifiers,
                ..
            } if modifiers.shift_only() => Some(Way::Previous),
            _ => None,
        })
    })
}

/// The region after or before `region` in its scope, going round.
fn step(region: Region, way: Way) -> Region {
    let index = match way {
        Way::Next => (region.index + 1) % region.count,
        Way::Previous => (region.index + region.count - 1) % region.count,
    };
    Region { index, ..region }
}

/// Starts the frame's routing: called once, before anything is drawn. If the
/// stand-in holds the keyboard, the region after (or before) the one Tab
/// left is to take it.
pub fn begin_frame(ctx: &egui::Context) {
    let mut router = load(ctx);
    router.drawn.clear();
    if ctx.memory(egui::Memory::focused) == Some(stand_in())
        && let Some((way, from)) = router.crossing.take()
    {
        router.pending = Some((way, step(from, way), from.count));
    }
    router.crossing = None;
    // Shift Tab with nothing focused starts at the end of the reading order:
    // the main window's last control (the primary button), not the control
    // egui drew last. A dialog that has just opened takes its own first Tab.
    if tab_way(ctx) == Some(Way::Previous) && ctx.memory(egui::Memory::focused).is_none() {
        let last = main_window::FOOTER;
        router.pending = Some((Way::Previous, last, last.count));
    }
    store(ctx, router);
}

/// Draws a region of a window with `contents`, in the reading order of its
/// scope (see [`Region`]).
pub fn region<R>(
    ui: &mut egui::Ui,
    region: Region,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let ctx = ui.ctx().clone();
    let layer = ui.layer_id();
    let mut router = load(&ctx);
    let way = tab_way(&ctx);
    let here = Some((region.scope, region.index));
    // Shift Tab from the region's first control: the stand-in, registered as
    // the region begins, is the control before it.
    if way == Some(Way::Previous) && router.focused_in == here {
        ctx.memory_mut(|m| m.interested_in_focus(stand_in(), layer));
        router.crossing = Some((Way::Previous, region));
        // Draw again at once (with no input), so the hand-off is made in the
        // frame of the key: the stand-in is never what the user sees.
        ctx.request_discard("Tab left a region");
    }
    let taking = router.pending.filter(|(_, target, _)| *target == region);
    if let Some((Way::Next, ..)) = taking {
        ctx.memory_mut(|m| {
            m.stop_text_input();
            m.move_focus(egui::FocusDirection::Next);
        });
    }
    if let Some((Way::Previous, ..)) = taking {
        // An empty region leaves the stand-in as the last control before it.
        ctx.memory_mut(|m| m.interested_in_focus(stand_in(), layer));
    }
    store(&ctx, router);
    // In a scope of its own, so the region's rectangle is its own and not the
    // whole of a parent it shares with another region.
    let shown = ui.scope(contents);
    let inner = shown.inner;
    let mut router = load(&ctx);
    let rect = shown.response.rect;
    router.drawn.push((region, rect, layer));
    let focused = ctx.memory(egui::Memory::focused);
    match taking {
        Some((Way::Next, _, left)) => {
            ctx.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
            router.pending = if focused.is_some() {
                None
            } else {
                // No control here: the region after it, next frame.
                (left > 1).then(|| (Way::Next, step(region, Way::Next), left - 1))
            };
        }
        Some((Way::Previous, _, left)) => {
            // The control registered last in the region takes the keyboard.
            ctx.memory_mut(|m| {
                m.stop_text_input();
                m.move_focus(egui::FocusDirection::Previous);
                m.interested_in_focus(stand_in(), layer);
            });
            let empty = ctx.memory(egui::Memory::focused) == Some(stand_in());
            router.pending =
                (empty && left > 1).then(|| (Way::Previous, step(region, Way::Previous), left - 1));
            if empty && left <= 1 {
                ctx.memory_mut(egui::Memory::stop_text_input);
            }
        }
        None => {}
    }
    // Tab from the region's last control: egui would give the keyboard to the
    // next control drawn, wherever it is; the stand-in takes it instead.
    if way == Some(Way::Next) && router.focused_in == here && focused.is_none() {
        ctx.memory_mut(|m| m.interested_in_focus(stand_in(), layer));
        router.crossing = Some((Way::Next, region));
        ctx.request_discard("Tab left a region");
    }
    store(&ctx, router);
    inner
}

/// Ends the frame's routing: called once, after everything is drawn. Notes
/// the region the focused control is in, and gives up a pending region that
/// was not drawn (a collapsed panel) for the one after it.
pub fn end_frame(ctx: &egui::Context) {
    let mut router = load(ctx);
    let focused = ctx
        .memory(egui::Memory::focused)
        .filter(|id| *id != stand_in())
        .and_then(|id| ctx.read_response(id));
    let region_of = focused.as_ref().and_then(|r| {
        router
            .drawn
            .iter()
            .filter(|(_, rect, layer)| *layer == r.layer_id && rect.contains(r.rect.center()))
            // The innermost: a region drawn inside another comes later.
            .next_back()
            .map(|(region, ..)| *region)
    });
    let was_in = router.focused_in;
    router.focused_in = match (focused.is_some(), region_of) {
        (_, Some(region)) => Some((region.scope, region.index)),
        // A control drawn outside every region (an overlay over the view, a
        // pop-up) belongs to the region the keyboard was last in.
        (true, None) => was_in,
        (false, None) => None,
    };
    // Tab from such a control, with nothing drawn after it: egui would hand
    // the keyboard to the first control of the next frame. The stand-in takes
    // it, and the region after the one it was in follows.
    if tab_way(ctx) == Some(Way::Next)
        && ctx.memory(egui::Memory::focused).is_none()
        && router.crossing.is_none()
        && router.pending.is_none()
        && let Some((scope, index)) = was_in
        && let Some((region, ..)) = router
            .drawn
            .iter()
            .find(|(r, ..)| r.scope == scope && r.index == index)
            .copied()
    {
        ctx.memory_mut(|m| m.interested_in_focus(stand_in(), egui::LayerId::background()));
        router.crossing = Some((Way::Next, region));
        ctx.request_discard("Tab left a region");
    }
    if let Some((way, target, left)) = router.pending
        && !router.drawn.iter().any(|(region, ..)| *region == target)
    {
        router.pending = (left > 1).then(|| (way, step(target, way), left - 1));
    }
    if router.pending.is_some() {
        // A region with no control was passed: the next one takes the
        // keyboard in the next frame, without waiting for input.
        ctx.request_repaint();
    }
    if router.pending.is_none()
        && ctx.memory(egui::Memory::focused) == Some(stand_in())
        && router.crossing.is_none()
    {
        ctx.memory_mut(egui::Memory::stop_text_input);
    }
    store(ctx, router);
}

/// A row of controls at the right end of `ui`, drawn left to right so Tab
/// reads them in reading order (the owner's answer Q3 A), where a right-to-left
/// layout would reverse them. The row's width is measured as it is drawn and
/// kept for the next frame; a change asks egui to draw the frame again, so the
/// row is never seen out of place. Returns what `contents` returns.
pub fn row_at_right<R>(
    ui: &mut egui::Ui,
    key: &str,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let width_id = egui::Id::unique(("row at right", key));
    let width = ui.data(|d| d.get_temp::<f32>(width_id)).unwrap_or(0.0);
    let full = ui.available_rect_before_wrap();
    let left = (full.right() - width).max(full.left());
    let place = egui::Rect::from_min_max(egui::pos2(left, full.top()), full.max);
    let shown = ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(place)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        contents,
    );
    let drawn = shown.response.rect.width();
    if (drawn - width).abs() > 0.5 {
        ui.data_mut(|d| d.insert_temp(width_id, drawn));
        ui.ctx().request_discard("a row at the right was measured");
    }
    shown.inner
}

/// The region the keyboard is in, for tests.
#[cfg(test)]
pub fn focused_region(ctx: &egui::Context) -> Option<(&'static str, usize)> {
    load(ctx).focused_in
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
