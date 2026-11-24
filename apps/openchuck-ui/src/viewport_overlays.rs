// SPDX-License-Identifier: AGPL-3.0-or-later
//! The viewport's overlays (interface specification 7.2; UI-02, SIM-01,
//! SIM-05, UI-25): at most five dark boxes in fixed places, never
//! overlapping, dark in both themes. Top left is the selection tools box,
//! which the selection panel draws; under it the path legend (Operations);
//! top right the view cube; bottom left the axes gadget, without a box, and
//! above it on Simulate the colour scale; bottom right the readout
//! (Operations, Simulate).
//! Held regions and tabs join the legend of the step (7.5).

use crate::focus::Tip as _;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, pos2, vec2};
use openchuck_app::pathview::{DeviationBand, ShownMoves, move_name};
use openchuck_app::playback::MoveKind;
use openchuck_app::scene::{SceneRole, SceneView};
use openchuck_viewport::{Camera, StandardView};

use crate::results_panel::{BandFilter, SOME_HIDDEN, Shown};
use crate::roles::{self, to_color32};
use crate::viewport::{ViewStep, Viewport};

/// The gap between an overlay and the view's edge.
const MARGIN: f32 = 12.0;
/// The view cube's box.
const CUBE: f32 = 84.0;
/// The height of the caption under the view cube, and its gap from the cube.
const CUBE_CAPTION: f32 = 18.0;
const CUBE_CAPTION_GAP: f32 = 4.0;
/// The size of the view cube's labels, in pixels at a face seen square-on.
const CUBE_LABEL: f32 = 8.0;
/// How squarely a cube face must face the eye for its label to show: the
/// sine of 15 degrees.
const LABEL_FACING: f32 = 0.258_819;
/// The least a cube face must face the eye to be drawn (about 1.1 degrees).
const MIN_FACING: f64 = 0.02;
/// The axes gadget's size.
const TRIAD: f32 = 78.0;
/// Below this height the path legend collapses to a button (7.2).
pub const LEGEND_MIN_HEIGHT: f32 = 480.0;
/// A legend row's height.
const ROW: f32 = 18.0;

/// Where the boxes are this frame.
#[derive(Debug, Default)]
pub(crate) struct Boxes {
    cube: Option<Rect>,
    /// The caption naming the frame the cube's faces refer to (issue 531).
    cube_caption: Option<Rect>,
    triad: Option<Rect>,
    legend: Option<Rect>,
    legend_button: Option<Rect>,
    readout: Option<Rect>,
    scale: Option<Rect>,
}

impl Boxes {
    /// Every box shown.
    pub(crate) fn all(&self) -> Vec<Rect> {
        [
            self.cube,
            self.triad,
            self.legend,
            self.legend_button,
            self.readout,
            self.scale,
            self.cube_caption,
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}

/// What the path legend lists (7.2), with the held region and tab roles
/// present after "Avoid" (7.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegendEntry {
    /// Cut moves.
    Cut,
    /// Link moves.
    Link,
    /// Rapid moves.
    Rapid,
    /// Entry and exit moves: the tick box of the three below.
    Entry,
    /// Plunges, under Entry and exit (issue #789).
    Plunge,
    /// Lead-ins and ramps, under Entry and exit (issue #789).
    LeadIn,
    /// Lead-outs, under Entry and exit (issue #789).
    LeadOut,
    /// The working selection.
    Selected,
    /// Avoid surfaces.
    Avoid,
    /// The Facing cutter sweep (decision 0507).
    CutterSweep,
    /// A held region.
    HeldRegion,
    /// A tab.
    Tab,
}

impl LegendEntry {
    /// Its name in the legend.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Cut => "Cut",
            Self::Link => "Link",
            Self::Rapid => "Rapid",
            Self::Entry => openchuck_app::pathview::ENTRY_NAME,
            Self::Plunge => move_name(MoveKind::Plunge),
            Self::LeadIn => move_name(MoveKind::LeadIn),
            Self::LeadOut => move_name(MoveKind::LeadOut),
            Self::Selected => "Selected",
            Self::Avoid => "Avoid",
            Self::CutterSweep => "Cutter sweep",
            Self::HeldRegion => "Held region",
            Self::Tab => "Tab",
        }
    }
}

impl LegendEntry {
    /// The kinds of toolpath move the entry's tick box shows or hides (issue
    /// #789; decision record PROJUX-A-2), or `None` for an entry with no
    /// tick box.
    pub fn moves(self) -> Option<&'static [MoveKind]> {
        match self {
            Self::Cut => Some(&[MoveKind::Cutting]),
            Self::Link => Some(&[MoveKind::Linking]),
            Self::Rapid => Some(&[MoveKind::Rapid]),
            Self::Entry => Some(&ShownMoves::ENTRY),
            Self::Plunge => Some(&[MoveKind::Plunge]),
            Self::LeadIn => Some(&[MoveKind::LeadIn]),
            Self::LeadOut => Some(&[MoveKind::LeadOut]),
            _ => None,
        }
    }

    /// Whether the row sits under Entry and exit.
    fn indented(self) -> bool {
        matches!(self, Self::Plunge | Self::LeadIn | Self::LeadOut)
    }
}

/// How a legend tick box reads: every kind it covers shown, some, or none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tick {
    /// Every kind shown.
    On,
    /// Some shown, some hidden (Entry and exit only).
    Partly,
    /// Every kind hidden.
    Off,
}

/// The tick of `kinds` in `moves`.
pub fn tick(moves: ShownMoves, kinds: &[MoveKind]) -> Tick {
    let shown = kinds.iter().filter(|k| moves.shows(**k)).count();
    match shown {
        0 => Tick::Off,
        n if n == kinds.len() => Tick::On,
        _ => Tick::Partly,
    }
}

/// What a press on a legend tick box does: a ticked or partly ticked box
/// hides its kinds, an empty one shows them.
pub fn toggle(moves: &mut ShownMoves, kinds: &[MoveKind]) {
    let show = tick(*moves, kinds) == Tick::Off;
    for kind in kinds {
        moves.set(*kind, show);
    }
}

/// The id of a legend tick box, so it is a Tab stop of its own and its focus
/// ring can be drawn (UI-14).
pub(crate) fn tick_id(entry: LegendEntry) -> egui::Id {
    egui::Id::unique(("path legend tick", entry.name()))
}

/// Where legend row `i` of the box `r` draws its tick box, if it has one,
/// and its swatch: a row of moves has a tick box before its swatch, and the
/// three kinds under Entry and exit are indented (issue #789).
fn row_parts(r: Rect, i: usize, entry: LegendEntry) -> (Option<Rect>, Rect) {
    let y = r.top() + 6.0 + ROW * i as f32;
    if entry.moves().is_none() {
        return (
            None,
            Rect::from_min_size(pos2(r.left() + 8.0, y + 3.0), vec2(18.0, 10.0)),
        );
    }
    let indent = if entry.indented() { TICK_INDENT } else { 0.0 };
    let t = Rect::from_min_size(pos2(r.left() + 8.0 + indent, y + 3.0), vec2(TICK, TICK));
    let swatch = Rect::from_min_size(pos2(t.right() + 6.0, y + 4.0), vec2(18.0, 10.0));
    (Some(t), swatch)
}

/// The rest of a legend tick box's tooltip, after the row's name.
pub const MOVES_TIP: &str = "moves: shown or hidden in the view only. The G-code, the simulation \
     and the checks do not change.";

/// The size of a legend tick box, and the indent of the rows under Entry
/// and exit.
const TICK: f32 = 12.0;
const TICK_INDENT: f32 = 14.0;

/// The room a legend row of moves needs before its label: the margin, the
/// indent, the tick box, the swatch and their gaps.
fn move_row_lead(entry: LegendEntry) -> f32 {
    8.0 + if entry.indented() { TICK_INDENT } else { 0.0 } + TICK + 6.0 + 18.0 + 8.0
}

/// A legend tick box: filled with a tick when every kind it covers is shown,
/// a bar when some are, empty when none, so the state is not by colour
/// alone; with the focus ring while it has the keyboard.
fn paint_tick(painter: &egui::Painter, r: Rect, state: Tick, focused: bool) {
    let accent = Color32::from_rgb(0x3d, 0x8b, 0xff);
    match state {
        Tick::On => {
            painter.rect_filled(r, 2.0, accent);
            painter.add(egui::Shape::line(
                vec![
                    r.left_center() + vec2(2.5, 0.0),
                    pos2(r.center().x - 1.0, r.bottom() - 3.0),
                    r.right_top() + vec2(-2.5, 3.0),
                ],
                Stroke::new(1.6, Color32::WHITE),
            ));
        }
        Tick::Partly => {
            painter.rect_filled(r, 2.0, accent);
            painter.line_segment(
                [
                    r.left_center() + vec2(3.0, 0.0),
                    r.right_center() - vec2(3.0, 0.0),
                ],
                Stroke::new(1.8, Color32::WHITE),
            );
        }
        Tick::Off => {
            painter.rect_stroke(
                r,
                2.0,
                Stroke::new(1.0, roles::OVERLAY_TEXT),
                egui::StrokeKind::Inside,
            );
        }
    }
    if focused {
        painter.rect_stroke(
            r.expand(2.0),
            3.0,
            Stroke::new(2.0, crate::theme::DARK.accent),
            egui::StrokeKind::Outside,
        );
    }
}

/// The held region and tab roles present in `view` (7.5).
pub fn workholding_entries(view: &SceneView) -> Vec<LegendEntry> {
    let has = |role| view.meshes.iter().any(|m| m.role == role);
    let mut out = Vec::new();
    if has(SceneRole::HeldRegion) {
        out.push(LegendEntry::HeldRegion);
    }
    if has(SceneRole::Tab) {
        out.push(LegendEntry::Tab);
    }
    out
}

/// The path legend's entries: the moves, the selection roles, then the held
/// region and tab roles of `view`.
pub fn path_legend(view: &SceneView) -> Vec<LegendEntry> {
    let mut out = vec![
        LegendEntry::Cut,
        LegendEntry::Link,
        LegendEntry::Rapid,
        LegendEntry::Entry,
        LegendEntry::Plunge,
        LegendEntry::LeadIn,
        LegendEntry::LeadOut,
        LegendEntry::Selected,
        LegendEntry::Avoid,
    ];
    // The Facing cutter sweep, when one is shown (issue #499, decision 0507).
    if view.lines.iter().any(|l| l.role == SceneRole::FacingSweep) {
        out.push(LegendEntry::CutterSweep);
    }
    out.extend(workholding_entries(view));
    out
}

/// The rows of moves alone: the path legend on Simulate (issue #789; the
/// owner's answer P2b B), where the selection roles do not apply and the
/// held regions, tabs, the shown set and the unreachable areas are listed in
/// the colour scale's box.
pub const MOVE_ENTRIES: [LegendEntry; 7] = [
    LegendEntry::Cut,
    LegendEntry::Link,
    LegendEntry::Rapid,
    LegendEntry::Entry,
    LegendEntry::Plunge,
    LegendEntry::LeadIn,
    LegendEntry::LeadOut,
];

/// Whether the path legend is shown on this step: always on Operations; on
/// Simulate only while Toolpaths is ticked in its Show choices (the owner's
/// answer P2b B), since with the toolpaths off there are no lines to name.
pub fn has_path_legend(viewport: &Viewport) -> bool {
    match viewport.step() {
        ViewStep::Operations => true,
        ViewStep::Simulate => viewport.simulate_show().toolpath,
        _ => false,
    }
}

/// The path legend's entries on the view's step: on Operations
/// [`path_legend`]; on Simulate the rows of moves only.
pub fn legend_entries(viewport: &Viewport) -> Vec<LegendEntry> {
    if viewport.step() == ViewStep::Simulate {
        MOVE_ENTRIES.to_vec()
    } else {
        path_legend(viewport.view())
    }
}

/// Whether the legend box also lists the shown set and the unreachable
/// areas: on Operations; on Simulate those are in the colour scale's box.
fn legend_lists_areas(viewport: &Viewport) -> bool {
    viewport.step() != ViewStep::Simulate
}

/// The path legend's rows: its entries, and the shown set and unreachable
/// areas when the legend lists them.
fn legend_rows(viewport: &Viewport) -> f32 {
    let areas = if legend_lists_areas(viewport) {
        f32::from(u8::from(viewport.legend_set.is_some()))
            + viewport.unreached_legend().len() as f32
    } else {
        0.0
    };
    legend_entries(viewport).len() as f32 + areas
}

/// A legend swatch for the surfaces the tool could not reach: the part's
/// grey with two diagonal lines, a core in a casing, as the view draws them
/// (decision 0515 item 4 as amended for issue 637).
pub fn hatch_swatch(painter: &egui::Painter, r: Rect) {
    painter.rect_filled(r, 0.0, to_color32(roles::PART));
    let clipped = painter.with_clip_rect(r);
    for x in [r.left() + r.width() * 0.3, r.left() + r.width() * 0.7] {
        let from = pos2(x - r.height() / 2.0, r.bottom());
        let to = pos2(x + r.height() / 2.0, r.top());
        clipped.line_segment(
            [from, to],
            Stroke::new(3.0, to_color32(roles::AREA_HATCH_CASING)),
        );
        clipped.line_segment(
            [from, to],
            Stroke::new(1.2, to_color32(roles::AREA_HATCH_CORE)),
        );
    }
}

/// A shown selection set's legend swatch, drawn the way the set is drawn
/// (decision 0216 items 4 and 5): its colour at 40% over the part grey,
/// framed by its outline, dark outside and in its colour inside.
pub fn set_swatch(painter: &egui::Painter, r: Rect, [red, green, blue]: [u8; 3]) {
    let colour = Color32::from_rgb(red, green, blue);
    let part = to_color32(roles::PART);
    let mix = |a: u8, b: u8| {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let v = (f32::from(a) * 0.4 + f32::from(b) * 0.6).round() as u8;
        v
    };
    painter.rect_filled(
        r,
        0.0,
        Color32::from_rgb(
            mix(red, part.r()),
            mix(green, part.g()),
            mix(blue, part.b()),
        ),
    );
    painter.rect_stroke(
        r,
        0.0,
        Stroke::new(2.0, to_color32(roles::SET_FRAME)),
        egui::StrokeKind::Outside,
    );
    painter.rect_stroke(
        r,
        0.0,
        Stroke::new(roles::SET_OUTLINE_WIDTH / 2.0, colour),
        egui::StrokeKind::Inside,
    );
}

/// Whether the path legend collapses to a button: the view is under 480
/// pixels high (7.2).
pub fn legend_collapsed(height: f32) -> bool {
    height < LEGEND_MIN_HEIGHT
}

/// The readout's lines (7.2): X, Y and Z in mm to 3 places and A in degrees
/// to 2, wrapped into 0 to 360 (issue 207), then the source and the frame
/// ("G54 work coordinates").
pub fn readout_lines(work: [f64; 4], source: &str, register: Option<&str>) -> Vec<String> {
    let frame = match register {
        Some(r) => format!("{r} work coordinates"),
        None => "work coordinates".to_owned(),
    };
    vec![
        format!("X {:>10.3} mm", work[0]),
        format!("Y {:>10.3} mm", work[1]),
        format!("Z {:>10.3} mm", work[2]),
        format!(
            "A {:>10} deg",
            openchuck_app::playback::wrapped_a_text(work[3], 2)
        ),
        format!("{source}, {frame}"),
    ]
}

/// What the readout shows, if anything: on Simulate the tool at its move,
/// else the point of the surface under the pointer, in work coordinates
/// (the work zero is where the work axes start).
fn readout(viewport: &Viewport) -> Option<([f64; 4], String)> {
    let step = viewport.step();
    if !matches!(step, ViewStep::Operations | ViewStep::Simulate) {
        return None;
    }
    if step == ViewStep::Simulate
        && let Some(tool) = viewport.tool_readout
    {
        return Some((tool.work, format!("Tool at move {}", tool.move_number)));
    }
    let p = viewport.cursor_point?;
    let zero = viewport
        .view()
        .lines
        .iter()
        .find(|l| l.role == SceneRole::WorkX)
        .and_then(|l| l.points.first())
        .map_or([0.0; 3], |z| [z.x, z.y, z.z]);
    Some((
        [
            p[0] - zero[0],
            p[1] - zero[1],
            p[2] - zero[2],
            viewport.part_turn(),
        ],
        "Cursor on surface".to_owned(),
    ))
}

/// One face of the view cube as seen from the eye.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CubeFace {
    /// The standard view the face gives.
    pub view: StandardView,
    /// Its label.
    pub label: &'static str,
    /// Its corners on screen about the cube's centre, in units of the cube's
    /// half size, y down.
    pub corners: [[f32; 2]; 4],
    /// How squarely it faces the eye, 0 to 1.
    pub facing: f32,
    /// The screen direction (y down, in the same units) the label's text runs
    /// in on the face, so the label lies on the face and reads from outside.
    pub right: [f32; 2],
    /// The screen direction of the label's upright.
    pub up: [f32; 2],
}

/// The faces of the view cube that face the eye, back to front (7.2; issue
/// 158: the labels lie flat on the faces).
pub fn cube_faces(camera: &Camera) -> Vec<CubeFace> {
    let (right, up, back) = camera.basis();
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let screen = |p: [f64; 3]| [dot(p, right) as f32, -dot(p, up) as f32];
    let faces: [(StandardView, &str, [f64; 3]); 6] = [
        (StandardView::Front, "FRONT", [0.0, -1.0, 0.0]),
        (StandardView::Back, "BACK", [0.0, 1.0, 0.0]),
        (StandardView::Left, "LEFT", [-1.0, 0.0, 0.0]),
        (StandardView::Right, "RIGHT", [1.0, 0.0, 0.0]),
        (StandardView::Top, "TOP", [0.0, 0.0, 1.0]),
        (StandardView::Bottom, "BOTTOM", [0.0, 0.0, -1.0]),
    ];
    let mut out: Vec<CubeFace> = faces
        .iter()
        .filter_map(|&(view, label, n)| {
            let facing = dot(n, back);
            // A face within a degree of edge-on is under a pixel thick: its
            // neighbours' edges already draw it, and tessellating such a
            // sliver gave long spikes across the cube (issue 176).
            if facing < MIN_FACING {
                return None;
            }
            // Two directions across the face, so its corners are n ± u ± v.
            let side = n[2].abs() <= 0.5;
            let u = if side {
                [0.0, 0.0, 1.0]
            } else {
                [1.0, 0.0, 0.0]
            };
            let v = [
                n[1] * u[2] - n[2] * u[1],
                n[2] * u[0] - n[0] * u[2],
                n[0] * u[1] - n[1] * u[0],
            ];
            let corner = |a: f64, b: f64| {
                screen([
                    n[0] + a * u[0] + b * v[0],
                    n[1] + a * u[1] + b * v[1],
                    n[2] + a * u[2] + b * v[2],
                ])
            };
            // Read from outside, a side face's text runs along u × n with
            // the vertical up; the top and bottom faces' run along X with Y
            // (or its opposite, below) up, so each label is upright from the
            // front view and from above.
            let (text_right, text_up) = if side {
                ([-v[0], -v[1], -v[2]], u)
            } else {
                (u, v)
            };
            Some(CubeFace {
                view,
                label,
                corners: [
                    corner(-1.0, -1.0),
                    corner(1.0, -1.0),
                    corner(1.0, 1.0),
                    corner(-1.0, 1.0),
                ],
                facing: facing as f32,
                right: screen(text_right),
                up: screen(text_up),
            })
        })
        .collect();
    out.sort_by(|a, b| a.facing.total_cmp(&b.facing));
    out
}

/// Whether `p` is inside the convex polygon `corners`.
fn inside(p: [f32; 2], corners: &[[f32; 2]; 4]) -> bool {
    let mut side: Option<bool> = None;
    for (a, b) in corners.iter().zip(corners.iter().cycle().skip(1)) {
        let cross = (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
        if cross.abs() < 1e-9 {
            continue;
        }
        let left = cross > 0.0;
        if *side.get_or_insert(left) != left {
            return false;
        }
    }
    true
}

/// The face of the view cube in `cube` under the point `p`, front-most
/// first.
fn cube_face_at(camera: &Camera, cube: Rect, p: Pos2) -> Option<StandardView> {
    let half = cube.width() * 0.25;
    let local = [
        (p.x - cube.center().x) / half,
        (p.y - cube.center().y) / half,
    ];
    cube_faces(camera)
        .iter()
        .rev()
        .find(|face| inside(local, &face.corners))
        .map(|face| face.view)
}

/// Where the overlays go in the view `rect` this frame (7.2).
/// The width a box needs to show each "Unreachable (operation name)" row
/// whole: the swatch with its margins and the longest label in the legend's
/// font (decision record 0515 item 4), no wider than the view allows.
fn area_rows_width(ui: &egui::Ui, viewport: &Viewport, rect: Rect) -> f32 {
    let font = egui::FontId::proportional(12.0);
    let label = viewport
        .unreached_legend()
        .iter()
        .map(|name| {
            ui.fonts_mut(|f| f.layout_no_wrap(name.clone(), font.clone(), roles::OVERLAY_TEXT))
                .size()
                .x
        })
        .fold(0.0_f32, f32::max);
    if label <= 0.0 {
        return 0.0;
    }
    // Left margin, swatch, gap, the label, right margin; and a point to
    // spare, so rounding never cuts the last letter.
    (8.0 + 18.0 + 8.0 + label + 8.0 + 1.0).min(rect.width() - 2.0 * MARGIN)
}

/// How wide the legend's rows of moves need the box: the longest of them,
/// with its tick box, swatch and margins (issue #789).
fn move_rows_width(ui: &egui::Ui) -> f32 {
    let font = egui::FontId::proportional(12.0);
    [
        LegendEntry::Cut,
        LegendEntry::Link,
        LegendEntry::Rapid,
        LegendEntry::Entry,
        LegendEntry::Plunge,
        LegendEntry::LeadIn,
        LegendEntry::LeadOut,
    ]
    .into_iter()
    .map(|e| {
        let label = ui
            .fonts_mut(|f| f.layout_no_wrap(e.name().to_owned(), font.clone(), roles::OVERLAY_TEXT))
            .size()
            .x;
        move_row_lead(e) + label + 8.0 + 1.0
    })
    .fold(0.0_f32, f32::max)
}

/// The legend's box and the view cube's, for tests of where they stand.
#[cfg(test)]
pub(crate) fn legend_and_cube(
    ui: &egui::Ui,
    viewport: &Viewport,
    rect: Rect,
) -> (Option<Rect>, Option<Rect>) {
    let b = boxes(ui, viewport, rect);
    (b.legend, b.cube)
}

pub(crate) fn boxes(ui: &egui::Ui, viewport: &Viewport, rect: Rect) -> Boxes {
    let mut b = Boxes::default();
    let step = viewport.step();
    let right = rect.right() - MARGIN;
    b.cube = Some(Rect::from_min_size(
        pos2(right - CUBE, rect.top() + MARGIN),
        vec2(CUBE, CUBE),
    ));
    if let Some(cube) = b.cube {
        b.cube_caption = Some(Rect::from_min_size(
            pos2(cube.left(), cube.bottom() + CUBE_CAPTION_GAP),
            vec2(CUBE, CUBE_CAPTION),
        ));
    }
    // The axes gadget: bottom left of every 3D view, drawn without a box
    // (decision 0149).
    b.triad = Some(Rect::from_min_size(
        pos2(rect.left() + MARGIN, rect.bottom() - MARGIN - TRIAD),
        vec2(TRIAD, TRIAD),
    ));
    if readout(viewport).is_some() {
        b.readout = Some(Rect::from_min_size(
            pos2(right - 212.0, rect.bottom() - MARGIN - 96.0),
            vec2(212.0, 96.0),
        ));
    }
    if has_path_legend(viewport) {
        // Under the selection tools box while it shows (Operations); on
        // Simulate at the top left, while the toolpaths are shown.
        let top = if viewport.is_picking() {
            ui.ctx()
                .memory(|m| m.area_rect(egui::Id::unique("selection tools")))
                .map_or(rect.top() + MARGIN, |r| r.bottom() + 8.0)
        } else {
            rect.top() + MARGIN
        };
        let left = rect.left() + MARGIN;
        // As wide as its longest area row, but short of the view cube at the
        // top right, with a gap; a longer row ends in "…" (`fitted_label`).
        let legend_width = area_rows_width(ui, viewport, rect)
            .max(move_rows_width(ui))
            .min(right - CUBE - 8.0 - left)
            .max(124.0);
        if legend_collapsed(rect.height()) {
            b.legend_button = Some(Rect::from_min_size(pos2(left, top), vec2(84.0, 26.0)));
            if viewport.legend_open {
                let rows = legend_rows(viewport);
                b.legend = Some(Rect::from_min_size(
                    pos2(left, top + 30.0),
                    vec2(legend_width, rows * ROW + 12.0),
                ));
            }
        } else {
            let rows = legend_rows(viewport);
            b.legend = Some(Rect::from_min_size(
                pos2(left, top),
                vec2(legend_width, rows * ROW + 12.0),
            ));
        }
    }
    let held = workholding_entries(viewport.view()).len() as f32
        + f32::from(u8::from(viewport.legend_set.is_some()))
        + viewport.unreached_legend().len() as f32;
    if step == ViewStep::Simulate
        && (viewport.scale_bands().is_some()
            || viewport.part_changed()
            || !viewport.unreached_legend().is_empty())
    {
        let notes = scale_note_height(&ui.painter_at(rect), viewport);
        let height = scale_held_top(viewport, notes) + 4.0 + held * ROW;
        b.scale = Some(Rect::from_min_size(
            pos2(
                rect.left() + MARGIN,
                rect.bottom() - MARGIN - TRIAD - 8.0 - height,
            ),
            vec2(area_rows_width(ui, viewport, rect).max(236.0), height),
        ));
    }
    if step == ViewStep::Simulate {
        share_left_side(&mut b, viewport.legend_open);
    }
    b
}

/// On Simulate the path legend (top left) and the colour scale (bottom left)
/// share the left side of the view. Where they would meet, in a short view
/// or under a tall scale, the legend collapses to its button; opened, it
/// takes the scale's place until it is closed, so no two boxes overlap (7.2;
/// issue #789).
fn share_left_side(b: &mut Boxes, legend_open: bool) {
    let (Some(legend), Some(scale)) = (b.legend, b.scale) else {
        return;
    };
    if !legend.intersects(scale) {
        return;
    }
    let button = Rect::from_min_size(legend.min, vec2(84.0, 26.0));
    b.legend_button = Some(button);
    if legend_open {
        b.legend = Some(Rect::from_min_size(
            pos2(legend.left(), button.bottom() + 4.0),
            legend.size(),
        ));
        b.scale = None;
    } else {
        b.legend = None;
    }
}

/// Takes the overlays' clicks: a face of the view cube gives that standard
/// view, and the collapsed legend's button opens it. Returns true when the
/// pointer is over a box, so the view does not pick through it.
pub fn interact(ui: &egui::Ui, viewport: &mut Viewport, rect: Rect) -> bool {
    let b = boxes(ui, viewport, rect);
    legend_ticks(ui, viewport, b.legend);
    let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) else {
        return false;
    };
    let clicked = ui.input(|i| i.pointer.primary_clicked());
    let over = b.all().iter().any(|r| r.contains(pointer));
    if clicked {
        if let Some(cube) = b.cube
            && cube.contains(pointer)
            && let Some(view) = cube_face_at(&viewport.camera, cube, pointer)
        {
            viewport.set_view(view);
        }
        if b.legend_button.is_some_and(|r| r.contains(pointer)) {
            viewport.legend_open = !viewport.legend_open;
        }
    }
    over
}

/// The path legend's tick boxes (issue #789; decision record PROJUX-A-2, the
/// owner's answer P2 A): each row of moves is a control of its own, which a
/// click, or Space or Enter while it has the keyboard, turns on or off. The
/// whole row takes the click; Tab reaches each row in turn (UI-14).
fn legend_ticks(ui: &egui::Ui, viewport: &mut Viewport, legend: Option<Rect>) {
    let Some(r) = legend else {
        return;
    };
    let mut moves = viewport.shown_moves();
    for (i, entry) in legend_entries(viewport).into_iter().enumerate() {
        let Some(kinds) = entry.moves() else {
            continue;
        };
        let y = r.top() + 6.0 + ROW * i as f32;
        let row = Rect::from_min_max(pos2(r.left() + 4.0, y), pos2(r.right() - 4.0, y + ROW));
        let response = ui.interact(row, tick_id(entry), egui::Sense::click());
        let response = response.tip(format!("{} {MOVES_TIP}", entry.name()));
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::Role::CheckBox,
                true,
                tick(moves, kinds) == Tick::On,
                entry.name(),
            )
        });
        if response.clicked() {
            toggle(&mut moves, kinds);
        }
    }
    viewport.set_shown_moves(moves);
}

/// Draws a legend row's label right of its swatch, on one line ending
/// before `right`; a longer one, such as a long set name, ends in "…"
/// rather than running over the scene.
fn fitted_label(
    painter: &egui::Painter,
    swatch: Rect,
    text: &str,
    font: &egui::FontId,
    right: f32,
) {
    let galley = fit_row(painter, text, font, right - (swatch.right() + 8.0));
    let at = pos2(
        swatch.right() + 8.0,
        swatch.center().y - galley.size().y / 2.0,
    );
    painter.galley(at, galley, roles::OVERLAY_TEXT);
}

/// Lays `text` out as one row at most `width` wide.
fn fit_row(
    painter: &egui::Painter,
    text: &str,
    font: &egui::FontId,
    width: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::single_section(
        text.to_owned(),
        egui::TextFormat::simple(font.clone(), roles::OVERLAY_TEXT),
    );
    job.wrap = egui::text::TextWrapping {
        max_width: width.max(0.0),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    painter.layout_job(job)
}

fn frame(painter: &egui::Painter, r: Rect) {
    painter.rect(
        r,
        4.0,
        roles::OVERLAY_FILL,
        Stroke::new(1.0, roles::OVERLAY_BORDER),
        egui::StrokeKind::Inside,
    );
}

/// Draws `text` centred on `centre` so that it lies on a cube face: its
/// width runs along `right` and its upright along `up` (screen vectors, y
/// down, one unit of length for one pixel when the face is square-on), so it
/// foreshortens with the face and never faces the user (issue 158).
fn flat_text(painter: &egui::Painter, text: &str, centre: Pos2, right: egui::Vec2, up: egui::Vec2) {
    let galley = painter.layout_no_wrap(
        text.to_owned(),
        egui::FontId::proportional(CUBE_LABEL),
        roles::OVERLAY_TEXT,
    );
    let size = galley.size();
    let uv = painter.fonts(|f| f.font_image_size());
    let normalise = vec2(1.0 / uv[0] as f32, 1.0 / uv[1] as f32);
    let mut mesh = egui::Mesh::default();
    for row in &galley.rows {
        let base = mesh.vertices.len() as u32;
        mesh.indices
            .extend(row.visuals.mesh.indices.iter().map(|i| i + base));
        mesh.vertices
            .extend(row.visuals.mesh.vertices.iter().map(|v| {
                // Pixels right of and below the text's middle.
                let local = row.pos.to_vec2() + v.pos.to_vec2() - size / 2.0;
                egui::epaint::Vertex {
                    pos: centre + right * local.x - up * local.y,
                    uv: (v.uv.to_vec2() * normalise).to_pos2(),
                    color: v.color,
                }
            }));
    }
    painter.add(egui::Shape::mesh(mesh));
}

/// Draws the faces of the view cube in `cube` as the camera sees them.
fn paint_cube(painter: &egui::Painter, cube: Rect, camera: &Camera) {
    let half = cube.width() * 0.25;
    for face in cube_faces(camera) {
        let points: Vec<Pos2> = face
            .corners
            .iter()
            .map(|c| cube.center() + vec2(c[0], c[1]) * half)
            .collect();
        let shade = (0x2b as f32 + 0x18 as f32 * face.facing) as u8;
        // The fill has no outline of its own: a face seen almost edge-on is a
        // sliver whose sharp corners, outlined as one path, grow long spikes
        // (issue 176). Each edge is a line of its own.
        painter.add(egui::Shape::convex_polygon(
            points.clone(),
            Color32::from_rgb(shade, shade + 4, shade + 9),
            Stroke::NONE,
        ));
        let edge = Stroke::new(1.0, Color32::from_rgb(0x7b, 0x84, 0x90));
        for (a, b) in points.iter().zip(points.iter().cycle().skip(1)) {
            painter.line_segment([*a, *b], edge);
        }
        let centre = points.iter().fold(Pos2::ZERO, |a, p| a + p.to_vec2()) / 4.0;
        // A face within 15 degrees of edge-on is too thin to read.
        if face.facing >= LABEL_FACING {
            flat_text(
                painter,
                face.label,
                centre,
                vec2(face.right[0], face.right[1]),
                vec2(face.up[0], face.up[1]),
            );
        }
    }
}

/// Draws the overlays.
pub fn paint(ui: &egui::Ui, viewport: &Viewport, rect: Rect) {
    let painter = ui.painter_at(rect);
    let b = boxes(ui, viewport, rect);
    let font = egui::FontId::proportional(12.0);
    if let Some(cube) = b.cube {
        frame(&painter, cube);
        paint_cube(&painter, cube, &viewport.camera);
    }
    if let Some(caption) = b.cube_caption {
        let frame_of_cube = viewport.cube_frame();
        frame(&painter, caption);
        painter.text(
            caption.center(),
            egui::Align2::CENTER_CENTER,
            crate::orient::cube_caption(frame_of_cube),
            font.clone(),
            roles::OVERLAY_TEXT,
        );
        // A Tab stop that cannot be clicked, so the tooltip opens on focus too.
        let response = ui.interact(
            caption,
            ui.scope_id().with("cube caption"),
            crate::focus::info_sense(),
        );
        crate::focus::focus_ring(ui, &response);
        response.tip(crate::orient::cube_tooltip(frame_of_cube));
    }
    if let Some(triad) = b.triad {
        let (right, up, _) = viewport.camera.basis();
        let centre = triad.center();
        for (axis, name, colour) in [
            ([1.0, 0.0, 0.0], "X", roles::WORK_X),
            ([0.0, 1.0, 0.0], "Y", roles::WORK_Y),
            ([0.0, 0.0, 1.0], "Z", roles::WORK_Z),
        ] {
            let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
            let tip = centre + vec2(dot(axis, right) as f32, -dot(axis, up) as f32) * 26.0;
            let colour = to_color32(colour);
            painter.line_segment([centre, tip], Stroke::new(2.0, colour));
            arrowhead(&painter, centre, tip, colour);
            let away = (tip - centre).normalized() * 9.0;
            painter.text(
                tip + away,
                egui::Align2::CENTER_CENTER,
                name,
                egui::FontId::monospace(12.0),
                colour,
            );
        }
    }
    if let (Some(r), Some((work, source))) = (b.readout, readout(viewport)) {
        frame(&painter, r);
        // The continuous A, as the G-code gives it, on hovering the box
        // (issue 207).
        let response = ui.interact(r, ui.scope_id().with("readout"), crate::focus::info_sense());
        crate::focus::focus_ring(ui, &response);
        response.tip(crate::playback_panel::continuous_a_tip(work[3], 2));
        let colours = [
            roles::WORK_X,
            roles::WORK_Y,
            roles::WORK_Z,
            roles::ROTARY_AXIS,
        ];
        let register = viewport.view().work_register.as_deref();
        for (i, line) in readout_lines(work, &source, register).iter().enumerate() {
            let at = r.left_top() + vec2(10.0, 8.0 + 16.0 * i as f32);
            let colour = colours
                .get(i)
                .map_or(roles::OVERLAY_MUTED, |c| to_color32(*c));
            if i < 4 {
                // The axis letter in its colour, the value in the text colour.
                painter.text(
                    at,
                    egui::Align2::LEFT_TOP,
                    &line[..1],
                    egui::FontId::monospace(12.0),
                    colour,
                );
                painter.text(
                    at + vec2(14.0, 0.0),
                    egui::Align2::LEFT_TOP,
                    &line[1..],
                    egui::FontId::monospace(12.0),
                    roles::OVERLAY_TEXT,
                );
            } else {
                painter.text(
                    at,
                    egui::Align2::LEFT_TOP,
                    line,
                    egui::FontId::proportional(11.0),
                    colour,
                );
            }
        }
    }
    if let Some(button) = b.legend_button {
        frame(&painter, button);
        painter.line_segment(
            [
                button.left_center() + vec2(8.0, 0.0),
                button.left_center() + vec2(24.0, 0.0),
            ],
            Stroke::new(2.0, to_color32(roles::CUT)),
        );
        painter.text(
            button.left_center() + vec2(30.0, 0.0),
            egui::Align2::LEFT_CENTER,
            "Legend",
            font.clone(),
            roles::OVERLAY_TEXT,
        );
    }
    if let Some(r) = b.legend {
        frame(&painter, r);
        let moves = viewport.shown_moves();
        let entries = legend_entries(viewport);
        for (i, entry) in entries.iter().copied().enumerate() {
            let (tick_box, swatch) = row_parts(r, i, entry);
            if let (Some(t), Some(kinds)) = (tick_box, entry.moves()) {
                let focused = ui.ctx().memory(|m| m.has_focus(tick_id(entry)));
                paint_tick(&painter, t, tick(moves, kinds), focused);
            }
            swatch_for(&painter, swatch, entry, false);
            painter.text(
                pos2(swatch.right() + 8.0, swatch.center().y),
                egui::Align2::LEFT_CENTER,
                entry.name(),
                font.clone(),
                roles::OVERLAY_TEXT,
            );
        }
        if let Some((name, rgb)) = viewport
            .legend_set
            .as_ref()
            .filter(|_| legend_lists_areas(viewport))
        {
            let y = r.top() + 6.0 + ROW * entries.len() as f32;
            let swatch = Rect::from_min_size(pos2(r.left() + 8.0, y + 3.0), vec2(18.0, 10.0));
            set_swatch(&painter, swatch, *rgb);
            fitted_label(&painter, swatch, name, &font, r.right() - 8.0);
        }
        let first = entries.len() + usize::from(viewport.legend_set.is_some());
        let unreached = if legend_lists_areas(viewport) {
            viewport.unreached_legend()
        } else {
            Vec::new()
        };
        for (i, name) in unreached.iter().enumerate() {
            let y = r.top() + 6.0 + ROW * (first + i) as f32;
            let swatch = Rect::from_min_size(pos2(r.left() + 8.0, y + 3.0), vec2(18.0, 10.0));
            hatch_swatch(&painter, swatch);
            fitted_label(&painter, swatch, name, &font, r.right() - 8.0);
        }
    }
    if let Some(r) = b.scale {
        frame(&painter, r);
        let notes = scale_notes(&painter, viewport);
        let note_height = notes.iter().map(|(g, _)| g.size().y).sum::<f32>();
        let held_top = scale_held_top(viewport, note_height);
        if let Some(bands) = viewport.scale_bands() {
            // What the colours are on, then the scale (NFR-15; decision 0492).
            painter.text(
                r.left_top() + vec2(8.0, 6.0),
                egui::Align2::LEFT_TOP,
                viewport.scale_title(),
                egui::FontId::proportional(12.0),
                roles::OVERLAY_TEXT,
            );
            let scale = Rect::from_min_max(r.min + vec2(0.0, SCALE_TITLE), r.max);
            colour_scale(
                &painter,
                scale,
                bands.tolerance,
                &viewport.band_filter,
                unreachable_row(viewport),
            );
        }
        let mut y = r.top() + held_top - note_height;
        for (galley, colour) in notes {
            let height = galley.size().y;
            painter.galley(pos2(r.left() + 8.0, y), galley, colour);
            y += height;
        }
        for (i, entry) in workholding_entries(viewport.view()).into_iter().enumerate() {
            let y = r.top() + held_top + ROW * i as f32;
            let swatch = Rect::from_min_size(pos2(r.left() + 8.0, y + 2.0), vec2(14.0, 12.0));
            swatch_for(&painter, swatch, entry, true);
            let name = match entry {
                LegendEntry::HeldRegion => "Held region (kept)",
                _ => "Tab (kept)",
            };
            painter.text(
                pos2(swatch.right() + 8.0, swatch.center().y),
                egui::Align2::LEFT_CENTER,
                name,
                font.clone(),
                roles::OVERLAY_TEXT,
            );
        }
        if let Some((name, rgb)) = &viewport.legend_set {
            let i = workholding_entries(viewport.view()).len() as f32;
            let y = r.top() + held_top + ROW * i;
            let swatch = Rect::from_min_size(pos2(r.left() + 8.0, y + 2.0), vec2(14.0, 12.0));
            set_swatch(&painter, swatch, *rgb);
            let label = format!("Selection set “{name}”");
            fitted_label(&painter, swatch, &label, &font, r.right() - 8.0);
        }
        let first =
            workholding_entries(viewport.view()).len() + usize::from(viewport.legend_set.is_some());
        for (i, name) in viewport.unreached_legend().iter().enumerate() {
            let y = r.top() + held_top + ROW * (first + i) as f32;
            let swatch = Rect::from_min_size(pos2(r.left() + 8.0, y + 2.0), vec2(18.0, 12.0));
            hatch_swatch(&painter, swatch);
            fitted_label(&painter, swatch, name, &font, r.right() - 8.0);
        }
    }
    if viewport.shows_unapplied() {
        let text = "Showing unapplied workholding";
        let galley = painter.layout_no_wrap(text.to_owned(), font.clone(), roles::OVERLAY_TEXT);
        let chip = Rect::from_center_size(
            pos2(rect.center().x, rect.top() + MARGIN + 12.0),
            galley.size() + vec2(16.0, 8.0),
        );
        frame(&painter, chip);
        painter.galley(chip.min + vec2(8.0, 4.0), galley, roles::OVERLAY_TEXT);
    }
}

/// The height of the colour scale's title line (decision 0492).
const SCALE_TITLE: f32 = 18.0;
/// The width the scale box's notes wrap at.
const SCALE_NOTE_WIDTH: f32 = 220.0;

/// The notes under the scale's own lines, laid out, with their colours.
fn scale_notes(
    painter: &egui::Painter,
    viewport: &Viewport,
) -> Vec<(std::sync::Arc<egui::Galley>, Color32)> {
    viewport
        .scale_notes()
        .into_iter()
        .map(|text| {
            let colour = if text == crate::viewport::PART_CHANGED_NOTE {
                roles::OVERLAY_TEXT
            } else {
                roles::OVERLAY_MUTED
            };
            (
                painter.layout(
                    text.to_owned(),
                    egui::FontId::proportional(11.0),
                    colour,
                    SCALE_NOTE_WIDTH,
                ),
                colour,
            )
        })
        .collect()
}

/// The height the scale box's notes take.
fn scale_note_height(painter: &egui::Painter, viewport: &Viewport) -> f32 {
    scale_notes(painter, viewport)
        .iter()
        .map(|(g, _)| g.size().y)
        .sum()
}

/// Where the rows under the scale begin in its box, given the height of its
/// notes: after the title and the scale's lines when there are bands, else
/// at the top.
fn scale_held_top(viewport: &Viewport, notes: f32) -> f32 {
    if viewport.scale_bands().is_some() {
        let row = if unreachable_row(viewport) {
            UNREACHABLE_ROW
        } else {
            0.0
        };
        70.0 + row + SCALE_TITLE + notes
    } else {
        6.0 + notes
    }
}

/// The height of the legend's "Unreachable (enclosed)" row (issue 478).
const UNREACHABLE_ROW: f32 = 16.0;

/// Whether the legend lists "Unreachable (enclosed)": while the colours
/// shown hold it, or its tick box hides it (then dimmed).
fn unreachable_row(viewport: &Viewport) -> bool {
    viewport.band_filter.hide_unreachable
        || viewport.scale_bands().is_some_and(|b| {
            b.bands
                .iter()
                .any(|(band, set)| *band == DeviationBand::Unreachable && !set.is_empty())
        })
}

/// The bands of the colour scale in order, with the labels at its ends
/// (11.5; D12: decision 0040's bands in the colours of 11.5).
pub const SCALE: [DeviationBand; 5] = [
    DeviationBand::Overcut,
    DeviationBand::OnTarget,
    DeviationBand::Thin,
    DeviationBand::Medium,
    DeviationBand::Thick,
];

/// The cell of the scale that is on the surface, under which "On" is
/// written.
fn on_target_cell() -> usize {
    SCALE
        .iter()
        .position(|b| *b == DeviationBand::OnTarget)
        .unwrap_or(0)
}

/// Draws the colour scale: a bar of the bands, its labelled ends, the hatched
/// "not measured" swatch and the band of "on the surface". A band the band
/// filter hides is dimmed, one it shows in part is dimmed in its lower half,
/// and while anything is hidden the box says so (SIM-15; decision 0163).
fn colour_scale(
    painter: &egui::Painter,
    r: Rect,
    tolerance: f64,
    filter: &BandFilter,
    unreachable: bool,
) {
    let bar = Rect::from_min_size(r.left_top() + vec2(8.0, 8.0), vec2(r.width() - 16.0, 10.0));
    let step = bar.width() / SCALE.len() as f32;
    for (i, band) in SCALE.iter().enumerate() {
        let cell = Rect::from_min_size(
            bar.left_top() + vec2(step * i as f32, 0.0),
            vec2(step, bar.height()),
        );
        let colour = to_color32(roles::band_colour(*band));
        match filter.band_shown(*band, tolerance) {
            Shown::All => {
                painter.rect_filled(cell, 0.0, colour);
            }
            Shown::Part => {
                let (top, bottom) = cell.split_top_bottom_at_fraction(0.5);
                painter.rect_filled(top, 0.0, colour);
                painter.rect_filled(bottom, 0.0, colour.gamma_multiply(HIDDEN_OPACITY));
            }
            Shown::Hidden => {
                painter.rect_filled(cell, 0.0, colour.gamma_multiply(HIDDEN_OPACITY));
            }
        }
    }
    let font = egui::FontId::proportional(11.0);
    painter.text(
        bar.left_bottom() + vec2(0.0, 4.0),
        egui::Align2::LEFT_TOP,
        "Overcut",
        font.clone(),
        roles::OVERLAY_TEXT,
    );
    painter.text(
        bar.left_bottom() + vec2(step * (on_target_cell() as f32 + 0.5), 4.0),
        egui::Align2::CENTER_TOP,
        "On",
        font.clone(),
        roles::OVERLAY_TEXT,
    );
    painter.text(
        bar.right_bottom() + vec2(0.0, 4.0),
        egui::Align2::RIGHT_TOP,
        "Left > 1 mm",
        font.clone(),
        roles::OVERLAY_TEXT,
    );
    // The two patterned bands, as the part draws them (decision 0513):
    // "Not measured", then "Unreachable (enclosed)", dimmed when its own
    // tick box hides it.
    let swatch = |y: f32, band: DeviationBand| {
        let at = Rect::from_min_size(r.left_top() + vec2(8.0, y), vec2(14.0, 10.0));
        let dim = if filter.band_shown(band, tolerance) == Shown::Hidden {
            HIDDEN_OPACITY
        } else {
            1.0
        };
        painter.rect_filled(
            at,
            0.0,
            to_color32(roles::band_colour(band)).gamma_multiply(dim),
        );
        if let Some(h) = roles::band_hatch(band) {
            hatch(
                painter,
                at,
                to_color32(h.colour).gamma_multiply(dim),
                matches!(h.kind, openchuck_viewport::HatchKind::Cross),
            );
        }
        painter.text(
            at.right_center() + vec2(8.0, 0.0),
            egui::Align2::LEFT_CENTER,
            band.name(),
            egui::FontId::proportional(11.0),
            roles::OVERLAY_TEXT,
        );
        at
    };
    let unmeasured = swatch(38.0, DeviationBand::Unmeasured);
    let row = if unreachable {
        swatch(38.0 + UNREACHABLE_ROW, DeviationBand::Unreachable);
        UNREACHABLE_ROW
    } else {
        0.0
    };
    painter.text(
        r.left_top() + vec2(8.0, 52.0 + row),
        egui::Align2::LEFT_TOP,
        format!("On the surface: within {tolerance:.3} mm"),
        font.clone(),
        roles::OVERLAY_MUTED,
    );
    if filter.hides_any() {
        // On the "Not measured" row, clear of the labels under the bar.
        painter.text(
            pos2(r.right() - 8.0, unmeasured.center().y),
            egui::Align2::RIGHT_CENTER,
            SOME_HIDDEN,
            font,
            roles::OVERLAY_TEXT,
        );
    }
}

/// How strongly a band the filter hides is drawn on the colour scale.
const HIDDEN_OPACITY: f32 = 0.25;

/// Diagonal lines 6 pixels apart across `r`, clipped to it: one direction,
/// or both.
pub fn hatch(painter: &egui::Painter, r: Rect, colour: Color32, cross: bool) {
    let clipped = painter.with_clip_rect(r);
    let stroke = Stroke::new(1.0, colour);
    let span = r.width() + r.height();
    let mut k = 0.0;
    while k <= span {
        clipped.line_segment(
            [
                r.left_top() + vec2(k, 0.0),
                r.left_top() + vec2(k - span, span),
            ],
            stroke,
        );
        if cross {
            clipped.line_segment(
                [
                    r.right_top() - vec2(k, 0.0),
                    r.right_top() + vec2(span - k, span),
                ],
                stroke,
            );
        }
        k += 6.0;
    }
}

/// A legend swatch drawn the way the role is drawn (7.5): `kept` is the
/// Simulate form, an outline in the role colour.
pub fn swatch_for(painter: &egui::Painter, r: Rect, entry: LegendEntry, kept: bool) {
    let line = |colour: openchuck_viewport::Colour, width: f32, dotted: bool| {
        let y = r.center().y;
        let stroke = Stroke::new(width, to_color32(colour));
        if dotted {
            painter.add(egui::Shape::dashed_line(
                &[pos2(r.left(), y), pos2(r.right(), y)],
                stroke,
                2.0,
                3.0,
            ));
        } else {
            painter.line_segment([pos2(r.left(), y), pos2(r.right(), y)], stroke);
        }
    };
    match entry {
        LegendEntry::Cut => line(roles::CUT, roles::PATH_WIDTH, false),
        LegendEntry::Link => line(roles::LINK, roles::LINE_WIDTH, false),
        LegendEntry::Rapid => line(roles::RAPID, roles::PATH_WIDTH, true),
        LegendEntry::Entry | LegendEntry::Plunge | LegendEntry::LeadIn | LegendEntry::LeadOut => {
            line(roles::ENTRY, roles::PATH_WIDTH, false);
        }
        LegendEntry::CutterSweep => line(roles::SWEEP, roles::LINE_WIDTH, false),
        LegendEntry::Selected => {
            painter.rect_filled(r, 0.0, to_color32(roles::SELECTED).gamma_multiply(0.4));
        }
        LegendEntry::Avoid => {
            hatch(painter, r, to_color32(roles::AVOID), false);
            painter.rect_stroke(
                r,
                0.0,
                Stroke::new(1.0, to_color32(roles::AVOID)),
                egui::StrokeKind::Inside,
            );
        }
        LegendEntry::HeldRegion => {
            let violet = to_color32(roles::HELD_LINE);
            if !kept {
                painter.rect_filled(r, 0.0, violet.gamma_multiply(0.35));
                hatch(painter, r, violet, true);
            }
            painter.rect_stroke(
                r,
                0.0,
                Stroke::new(roles::OUTLINE_WIDTH, violet),
                egui::StrokeKind::Inside,
            );
        }
        LegendEntry::Tab => {
            let sand = to_color32(roles::TAB);
            if kept {
                painter.rect_stroke(
                    r,
                    0.0,
                    Stroke::new(roles::OUTLINE_WIDTH, sand),
                    egui::StrokeKind::Inside,
                );
            } else {
                painter.rect_filled(r, 0.0, sand);
                painter.rect_stroke(
                    r,
                    0.0,
                    Stroke::new(roles::OUTLINE_WIDTH, to_color32(roles::TAB_LINE)),
                    egui::StrokeKind::Inside,
                );
            }
        }
    }
}

/// An arrowhead at `tip`, pointing away from `from`.
pub fn arrowhead(painter: &egui::Painter, from: Pos2, tip: Pos2, colour: Color32) {
    let along = tip - from;
    if along.length() < 1.0 {
        return;
    }
    let d = along.normalized() * 7.0;
    let side = vec2(-d.y, d.x) * 0.5;
    painter.add(egui::Shape::convex_polygon(
        vec![tip + d * 0.5, tip - d * 0.5 + side, tip - d * 0.5 - side],
        colour,
        Stroke::NONE,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Draws the overlays of an Operations view of 1000 by 700 with `events`,
    /// as the view does: the clicks first, then the paint.
    fn legend_frame(
        ctx: &egui::Context,
        viewport: &mut Viewport,
        events: Vec<egui::Event>,
    ) -> Vec<egui::epaint::ClippedShape> {
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 700.0));
        let input = egui::RawInput {
            screen_rect: Some(screen),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            interact(ui, viewport, screen);
            paint(ui, viewport, screen);
        });
        output.textures_delta.clear();
        output.shapes
    }

    fn operations_view() -> Viewport {
        let mut viewport = Viewport::new(true);
        viewport.set_step(Some(ViewStep::Operations));
        viewport
    }

    /// The middle of legend row `entry`.
    fn row_centre(viewport: &Viewport, entry: LegendEntry) -> Pos2 {
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 700.0));
        let mut legend = None;
        let input = egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        };
        let mut output = egui::Context::default().run_ui(input, |ui| {
            legend = legend_and_cube(ui, viewport, screen).0;
        });
        // No renderer here takes the font texture.
        output.textures_delta.clear();
        let r = legend.unwrap();
        let i = path_legend(viewport.view())
            .iter()
            .position(|e| *e == entry)
            .unwrap();
        pos2(r.center().x, r.top() + 6.0 + ROW * (i as f32 + 0.5))
    }

    /// Issue #789 (decision record PROJUX-A-2, the owner's answers P1 B and P2
    /// A): each row of moves in the path legend is a tick box. A click on
    /// the row turns its kind off and on; Entry and exit sets its three kinds
    /// together and reads partly ticked when only some are shown.
    #[test]
    fn req_sim_01_a_click_on_a_legend_row_hides_and_shows_its_kind() {
        let ctx = egui::Context::default();
        let mut viewport = operations_view();
        legend_frame(&ctx, &mut viewport, Vec::new());
        let click = |viewport: &mut Viewport, at: Pos2| {
            let button = |pressed| egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            };
            legend_frame(
                &ctx,
                viewport,
                vec![egui::Event::PointerMoved(at), button(true)],
            );
            legend_frame(&ctx, viewport, vec![button(false)]);
            legend_frame(&ctx, viewport, Vec::new());
        };
        let rapid = row_centre(&viewport, LegendEntry::Rapid);
        click(&mut viewport, rapid);
        assert_eq!(viewport.shown_moves().hidden(), [MoveKind::Rapid]);
        click(&mut viewport, rapid);
        assert!(viewport.shown_moves().all_shown());
        // One kind under Entry and exit: the parent reads partly ticked.
        let lead_out = row_centre(&viewport, LegendEntry::LeadOut);
        click(&mut viewport, lead_out);
        assert_eq!(viewport.shown_moves().hidden(), [MoveKind::LeadOut]);
        assert_eq!(
            tick(viewport.shown_moves(), &ShownMoves::ENTRY),
            Tick::Partly
        );
        // A press on a partly ticked parent hides all three; again, shows them.
        let entry = row_centre(&viewport, LegendEntry::Entry);
        click(&mut viewport, entry);
        assert_eq!(viewport.shown_moves().hidden(), ShownMoves::ENTRY);
        click(&mut viewport, entry);
        assert!(viewport.shown_moves().all_shown());
    }

    /// UI-14: each legend tick box is a Tab stop; Space while it has the
    /// keyboard turns it, and its focus ring is drawn.
    #[test]
    fn req_ui_14_a_legend_tick_box_works_from_the_keyboard() {
        let ctx = egui::Context::default();
        let mut viewport = operations_view();
        legend_frame(&ctx, &mut viewport, Vec::new());
        ctx.memory_mut(|m| m.request_focus(tick_id(LegendEntry::Link)));
        let shapes = legend_frame(&ctx, &mut viewport, Vec::new());
        let ring = shapes.iter().any(|s| match &s.shape {
            egui::Shape::Rect(r) => r.stroke.color == crate::theme::DARK.accent,
            _ => false,
        });
        assert!(ring, "the focused tick box shows its focus ring");
        let space = vec![egui::Event::Key {
            key: egui::Key::Space,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }];
        legend_frame(&ctx, &mut viewport, space);
        assert_eq!(viewport.shown_moves().hidden(), [MoveKind::Linking]);
    }

    /// The owner's answer P2b B (issue #789): Simulate shows the path legend
    /// at the top left, with the rows of moves only, while Toolpaths is
    /// ticked; with the toolpaths off there is no legend. Its tick boxes set
    /// the same choice as on Operations.
    #[test]
    fn req_sim_01_simulate_shows_the_move_legend_only_with_the_toolpaths_on() {
        let ctx = egui::Context::default();
        let mut viewport = Viewport::new(true);
        viewport.set_step(Some(ViewStep::Simulate));
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 700.0));
        let legend = |viewport: &Viewport| {
            let mut legend = None;
            let input = egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            };
            let mut output = egui::Context::default().run_ui(input, |ui| {
                legend = legend_and_cube(ui, viewport, screen).0;
            });
            output.textures_delta.clear();
            legend
        };
        // Cut part only, the default: no toolpaths, no legend.
        assert!(!viewport.simulate_show().toolpath);
        assert!(!has_path_legend(&viewport));
        assert_eq!(legend(&viewport), None);
        let mut show = viewport.simulate_show();
        show.toolpath = true;
        viewport.set_simulate_show(show);
        assert!(has_path_legend(&viewport));
        let r = legend(&viewport).expect("the legend shows with the toolpaths");
        assert!(
            r.left() < screen.center().x && r.top() < screen.center().y,
            "top left"
        );
        let names: Vec<&str> = legend_entries(&viewport).iter().map(|e| e.name()).collect();
        assert_eq!(
            names,
            [
                "Cut",
                "Link",
                "Rapid",
                "Entry and exit",
                "Plunge",
                "Lead-in and ramp",
                "Lead-out"
            ]
        );
        // Its tick boxes work as on Operations.
        legend_frame_in(&ctx, &mut viewport, screen, Vec::new());
        ctx.memory_mut(|m| m.request_focus(tick_id(LegendEntry::Cut)));
        legend_frame_in(&ctx, &mut viewport, screen, Vec::new());
        let space = vec![egui::Event::Key {
            key: egui::Key::Space,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }];
        legend_frame_in(&ctx, &mut viewport, screen, space);
        assert_eq!(viewport.shown_moves().hidden(), [MoveKind::Cutting]);
    }

    /// Specification 7.2: the boxes never overlap. On Simulate with the path
    /// legend, for views from the 1920 by 1080 window's down to the 1366 by
    /// 768 window's (the owner's direction of 25 Nov 2026: 1920 by 1080 is
    /// the target, 1366 by 768 best effort), and below, where the legend
    /// collapses to its button.
    #[test]
    fn req_ui_16_the_simulate_legend_never_overlaps_another_box() {
        let mut viewport = Viewport::new(true);
        viewport.set_step(Some(ViewStep::Simulate));
        let mut show = viewport.simulate_show();
        show.toolpath = true;
        viewport.set_simulate_show(show);
        // The colour scale's box shows too, with its bands and its "part
        // changed" note, on the same side.
        viewport.set_bands(Some((
            openchuck_app::project::id::ModelId::new(),
            openchuck_app::pathview::DeviationBands {
                tolerance: 0.05,
                bands: Vec::new(),
                values: std::sync::Arc::from(Vec::new()),
                pieces: None,
            },
        )));
        viewport.set_part_changed(true);
        for open in [false, true] {
            viewport.legend_open = open;
            for (w, h) in [
                (1300.0, 760.0),
                (1100.0, 600.0),
                (760.0, 480.0),
                (700.0, 420.0),
                (600.0, 330.0),
            ] {
                let rect = Rect::from_min_size(pos2(0.0, 40.0), vec2(w, h));
                let input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(w, h + 40.0))),
                    ..Default::default()
                };
                let mut all = Vec::new();
                let mut output = egui::Context::default().run_ui(input, |ui| {
                    all = boxes(ui, &viewport, rect).all();
                });
                output.textures_delta.clear();
                for (i, a) in all.iter().enumerate() {
                    assert!(rect.contains_rect(*a), "{w}x{h}: {a:?} leaves the view");
                    for b in &all[i + 1..] {
                        assert!(!a.intersects(*b), "{w}x{h}: {a:?} overlaps {b:?}");
                    }
                }
            }
        }
    }

    /// Where the legend and a tall colour scale would meet on Simulate, the
    /// legend is a button; opened, it shows in the scale's place.
    #[test]
    fn req_ui_16_a_legend_that_would_meet_the_scale_collapses_to_its_button() {
        let legend = Rect::from_min_size(pos2(12.0, 52.0), vec2(160.0, 138.0));
        let scale = Rect::from_min_size(pos2(12.0, 150.0), vec2(236.0, 200.0));
        let shown = |open| {
            let mut b = Boxes {
                legend: Some(legend),
                scale: Some(scale),
                ..Boxes::default()
            };
            share_left_side(&mut b, open);
            b
        };
        let closed = shown(false);
        assert_eq!(closed.legend, None);
        assert_eq!(closed.scale, Some(scale));
        let button = closed.legend_button.unwrap();
        assert!(!button.intersects(scale));
        let open = shown(true);
        assert_eq!(
            open.scale, None,
            "the opened legend takes the scale's place"
        );
        assert!(!open.legend.unwrap().intersects(button));
        // Apart, nothing changes.
        let mut apart = Boxes {
            legend: Some(legend),
            scale: Some(scale.translate(vec2(0.0, 200.0))),
            ..Boxes::default()
        };
        share_left_side(&mut apart, false);
        assert_eq!(apart.legend, Some(legend));
        assert_eq!(apart.legend_button, None);
    }

    fn legend_frame_in(
        ctx: &egui::Context,
        viewport: &mut Viewport,
        screen: Rect,
        events: Vec<egui::Event>,
    ) {
        let input = egui::RawInput {
            screen_rect: Some(screen),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            interact(ui, viewport, screen);
            paint(ui, viewport, screen);
        });
        output.textures_delta.clear();
    }

    /// Entry and exit reads ticked, partly ticked or empty by its three
    /// kinds; a press on an empty box shows its kinds, on a ticked or partly
    /// ticked one hides them.
    #[test]
    fn req_sim_01_entry_and_exit_reads_and_sets_its_three_kinds() {
        assert_eq!(tick(ShownMoves::ALL, &ShownMoves::ENTRY), Tick::On);
        let mut none = ShownMoves::ALL;
        for k in ShownMoves::ENTRY {
            none.set(k, false);
        }
        assert_eq!(tick(none, &ShownMoves::ENTRY), Tick::Off);
        let mut moves = none;
        toggle(&mut moves, &ShownMoves::ENTRY);
        assert_eq!(moves, ShownMoves::ALL, "an empty box shows its kinds");
        toggle(&mut moves, &[MoveKind::Cutting]);
        assert!(
            !moves.shows(MoveKind::Cutting),
            "a ticked box hides its kind"
        );
    }

    #[test]
    fn req_ui_05_a_long_set_name_stays_inside_its_legend_row() {
        // Decision 0216 item 5: the shown set's legend row; a long name ends
        // in "…" within the box (CodeRabbit review of pull request 282).
        let ctx = egui::Context::default();
        let input = || egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 200.0))),
            ..Default::default()
        };
        ctx.run_ui(input(), |_| {}).textures_delta.clear();
        let mut output = ctx.run_ui(input(), |ui| {
            let font = egui::FontId::proportional(12.0);
            let long = "Selection set “The pocket floor and both of its side walls”";
            let galley = fit_row(ui.painter(), long, &font, 90.0);
            assert!(galley.size().x <= 90.0, "{}", galley.size().x);
            assert_eq!(galley.rows.len(), 1);
            let drawn = |g: &egui::Galley| -> String {
                g.rows
                    .iter()
                    .flat_map(|r| r.glyphs.iter().map(|c| c.chr))
                    .collect()
            };
            assert!(drawn(&galley).ends_with('…'), "{}", drawn(&galley));
            let short = fit_row(ui.painter(), "Pocket", &font, 90.0);
            assert_eq!(drawn(&short), "Pocket");
        });
        output.textures_delta.clear();
    }

    /// Issue 176 (F-21): in axis-aligned views and near them nothing of the
    /// cube is drawn outside its box (a nearly edge-on face made a long
    /// horizontal spike across it).
    #[test]
    fn req_ui_02_req_ui_16_the_cube_never_draws_outside_its_box() {
        let cube = Rect::from_min_size(pos2(100.0, 20.0), vec2(CUBE, CUBE));
        let mut camera = Camera::default();
        let mut views = Vec::new();
        for view in [
            StandardView::Front,
            StandardView::Back,
            StandardView::Left,
            StandardView::Right,
            StandardView::Top,
            StandardView::Bottom,
            StandardView::Isometric,
        ] {
            camera.set_view(view);
            views.push((camera.yaw, camera.pitch));
        }
        // And a sweep through the angles the faces turn edge-on at.
        for yaw in [-90.0, -45.0, -0.4, -0.05, 0.0, 0.05, 0.4, 45.0, 90.0, 180.0] {
            for pitch in [
                -90.0, -89.95, -89.0, -0.4, -0.05, 0.0, 0.05, 0.4, 89.0, 89.95, 90.0,
            ] {
                views.push((yaw, pitch));
            }
        }
        for (yaw, pitch) in views {
            camera.yaw = yaw;
            camera.pitch = pitch;
            let ctx = egui::Context::default();
            let input = || egui::RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(300.0, 200.0))),
                ..Default::default()
            };
            ctx.run_ui(input(), |_| {}).textures_delta.clear();
            let mut output = ctx.run_ui(input(), |ui| {
                paint_cube(ui.painter(), cube, &camera);
            });
            output.textures_delta.clear();
            let shapes = std::mem::take(&mut output.shapes);
            for primitive in ctx.tessellate(shapes, 1.0) {
                if let egui::epaint::Primitive::Mesh(mesh) = primitive.primitive {
                    for vertex in &mesh.vertices {
                        assert!(
                            cube.expand(2.0).contains(vertex.pos),
                            "a vertex at {:?} lies outside the cube {cube:?} at yaw {yaw}, pitch {pitch}",
                            vertex.pos
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn req_ui_02_the_view_cube_shows_the_faces_toward_the_eye_and_gives_their_view() {
        let mut camera = Camera::default();
        // Isometric, from +X, -Y and +Z: front, right and top face the eye.
        camera.set_view(StandardView::Isometric);
        let faces: Vec<StandardView> = cube_faces(&camera).iter().map(|f| f.view).collect();
        assert_eq!(faces.len(), 3);
        for view in [StandardView::Front, StandardView::Right, StandardView::Top] {
            assert!(faces.contains(&view), "{faces:?}");
        }
        // Looking from the front only the front faces the eye, and the
        // middle of the cube gives the front view.
        camera.set_view(StandardView::Front);
        let faces = cube_faces(&camera);
        assert_eq!(faces.len(), 1);
        let cube = Rect::from_min_size(pos2(0.0, 0.0), vec2(84.0, 84.0));
        assert_eq!(
            cube_face_at(&camera, cube, cube.center()),
            Some(StandardView::Front)
        );
        // From the top, the point above the centre on screen is on the top
        // face; outside the cube is nothing.
        camera.set_view(StandardView::Top);
        assert_eq!(
            cube_face_at(&camera, cube, cube.center()),
            Some(StandardView::Top)
        );
        assert_eq!(cube_face_at(&camera, cube, pos2(1.0, 1.0)), None);
    }

    #[test]
    fn req_ui_02_the_cube_labels_lie_flat_on_their_faces_and_read_from_outside() {
        // Issue 158: a label is drawn with its face, so it is there whenever
        // the face is, and it is never mirrored. From the front the front
        // label runs left to right with the upright up; from above so does
        // the top label.
        let mut camera = Camera::default();
        camera.set_view(StandardView::Front);
        let front = cube_faces(&camera)[0];
        assert_eq!((front.view, front.label), (StandardView::Front, "FRONT"));
        let near = |a: [f32; 2], b: [f32; 2]| (a[0] - b[0]).hypot(a[1] - b[1]) < 1e-5;
        assert!(near(front.right, [1.0, 0.0]) && near(front.up, [0.0, -1.0]));
        camera.set_view(StandardView::Top);
        let top = cube_faces(&camera)[0];
        assert!(near(top.right, [1.0, 0.0]) && near(top.up, [0.0, -1.0]));
        // Every face seen from any direction: a right-handed pair on screen
        // (y is down there, so the cross product is negative), and the text
        // direction lies along the face's own edges.
        for yaw in (0..360).step_by(25) {
            for pitch in (-80..=80).step_by(20) {
                camera.yaw = f64::from(yaw);
                camera.pitch = f64::from(pitch);
                for face in cube_faces(&camera) {
                    let cross = face.right[0] * face.up[1] - face.right[1] * face.up[0];
                    assert!(cross < 0.0, "{:?} mirrored at {yaw}, {pitch}", face.view);
                    let edge = |a: usize, b: usize| {
                        [
                            face.corners[b][0] - face.corners[a][0],
                            face.corners[b][1] - face.corners[a][1],
                        ]
                    };
                    let parallel =
                        |p: [f32; 2], q: [f32; 2]| (p[0] * q[1] - p[1] * q[0]).abs() < 1e-4;
                    assert!(
                        parallel(face.right, edge(0, 3)) || parallel(face.right, edge(0, 1)),
                        "{:?}",
                        face.view
                    );
                }
            }
        }
    }

    #[test]
    fn req_ui_02_req_sim_01_the_readout_gives_places_and_units() {
        let lines = readout_lines(
            [142.3184, -11.9, 3.45, -53.2],
            "Cursor on surface",
            Some("G54"),
        );
        assert_eq!(lines[0], "X    142.318 mm");
        assert_eq!(lines[1], "Y    -11.900 mm");
        // Issue 207: A reads 0 to 360 degrees.
        assert_eq!(lines[3], "A     306.80 deg");
        assert_eq!(
            readout_lines([0.0, 0.0, 0.0, 720.0], "", None)[3],
            "A       0.00 deg"
        );
        assert_eq!(lines[4], "Cursor on surface, G54 work coordinates");
        let lines = readout_lines([0.0; 4], "Tool at move 12", None);
        assert_eq!(lines[4], "Tool at move 12, work coordinates");
    }

    #[test]
    fn req_ui_25_req_sim_01_the_path_legend_lists_moves_then_held_regions_and_tabs() {
        let view = SceneView::default();
        let names: Vec<&str> = path_legend(&view).iter().map(|e| e.name()).collect();
        assert_eq!(
            names,
            [
                "Cut",
                "Link",
                "Rapid",
                "Entry and exit",
                "Plunge",
                "Lead-in and ramp",
                "Lead-out",
                "Selected",
                "Avoid"
            ]
        );
        // Below 480 pixels high it collapses to a button (7.2).
        assert!(legend_collapsed(479.0));
        assert!(!legend_collapsed(480.0));
    }

    /// Issue #499 (the owner's decision S-1): the Facing cutter sweep is a
    /// diagram, not a cut, so it is drawn in its own colour, not the Cut
    /// amber, and named in the path legend while one is shown (SIM-01).
    #[test]
    #[allow(clippy::float_cmp)] // Exact colour constants, not computed values.
    fn req_sim_01_req_ui_25_the_facing_sweep_has_its_own_colour_and_legend_entry() {
        assert_ne!(roles::SWEEP, roles::CUT);
        assert_eq!(
            crate::viewport::style(SceneRole::FacingSweep).0,
            roles::SWEEP
        );
        let mut view = SceneView::default();
        view.lines.push(openchuck_app::scene::ScenePolyline {
            role: SceneRole::FacingSweep,
            points: Vec::new(),
        });
        let names: Vec<&str> = path_legend(&view).iter().map(|e| e.name()).collect();
        assert_eq!(
            names,
            [
                "Cut",
                "Link",
                "Rapid",
                "Entry and exit",
                "Plunge",
                "Lead-in and ramp",
                "Lead-out",
                "Selected",
                "Avoid",
                "Cutter sweep"
            ]
        );
    }

    #[test]
    fn req_ui_05_req_nfr_15_a_shown_set_joins_the_path_legend_framed() {
        // Decision 0216 item 5: the legend shows the set the way it is drawn.
        let mut viewport = Viewport::new(false);
        viewport.set_step(Some(ViewStep::Operations));
        let ctx = egui::Context::default();
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(1200.0, 800.0));
        let mut heights = Vec::new();
        let mut shapes = Vec::new();
        for set in [None, Some(("Pocket".to_owned(), [0x23, 0x96, 0x88]))] {
            viewport.set_legend_set(set);
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                heights.push(boxes(ui, &viewport, rect).legend.unwrap().height());
                set_swatch(
                    ui.painter(),
                    Rect::from_min_size(pos2(10.0, 10.0), vec2(18.0, 10.0)),
                    [0x23, 0x96, 0x88],
                );
            });
            output.textures_delta.clear();
            shapes = output.shapes;
        }
        assert!((heights[1] - heights[0] - ROW).abs() < 1e-3, "{heights:?}");
        // The fill and two strokes: the dark frame and the set's colour.
        let strokes: Vec<Color32> = shapes
            .iter()
            .filter_map(|c| match &c.shape {
                egui::Shape::Rect(r) if r.stroke.width > 0.0 => Some(r.stroke.color),
                _ => None,
            })
            .collect();
        assert!(
            strokes.contains(&to_color32(roles::SET_FRAME)),
            "{strokes:?}"
        );
        assert!(
            strokes.contains(&Color32::from_rgb(0x23, 0x96, 0x88)),
            "{strokes:?}"
        );
    }

    #[test]
    fn req_ui_02_req_sim_01_operations_reads_the_pointer_not_the_old_tool_move() {
        // Issue 535: a tool position kept from a simulation is not the
        // readout of any other step.
        let mut viewport = Viewport::new(true);
        viewport.set_tool_readout(Some(crate::viewport::ToolReadout {
            work: [1.0, 2.0, 3.0, 4.0],
            move_number: 7,
        }));
        viewport.set_step(Some(ViewStep::Operations));
        assert!(readout(&viewport).is_none(), "no pointer on the surface");
        viewport.cursor_point = Some([5.0, 0.0, 0.0]);
        assert_eq!(readout(&viewport).unwrap().1, "Cursor on surface");
        viewport.set_step(Some(ViewStep::Simulate));
        assert_eq!(readout(&viewport).unwrap().1, "Tool at move 7");
    }

    #[test]
    fn req_ui_02_inside_tests_a_convex_polygon() {
        let square = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]];
        assert!(inside([0.0, 0.0], &square));
        assert!(!inside([2.0, 0.0], &square));
    }
}
