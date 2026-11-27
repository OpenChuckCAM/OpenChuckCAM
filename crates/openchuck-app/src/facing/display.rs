// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared, plain geometry for Facing viewport/Setup and operation-local results.
//! Display outlines approximate curves; they never replace the analytic checks.

use super::*;
use crate::scene::{SceneLabel, ScenePolyline, SceneRole};
use openchuck_path::facing::description::FacingGeometry;
use openchuck_path::facing::geometry::{CoverageRectangle, FacingPlane};
use openchuck_types::geometry::{Point3, Vec3};
use openchuck_types::ir::MoveKind;

/// One operation's shared diagram and separate completion result. Global
/// simulation deviation, including overcut, is never filtered by this value.
#[derive(Debug, Clone, PartialEq)]
pub struct FacingDisplay {
    /// Requested geometry, independently of cutter overlap.
    pub description: FacingDescription,
    /// Coverage/margin and actual cutter outlines in model coordinates.
    pub lines: Vec<ScenePolyline>,
    /// Targets, dimensions and margin with units.
    pub labels: Vec<SceneLabel>,
    /// Geometric completion status; no simulation accuracy claim.
    pub completion: String,
    /// Stock outside this operation's selected extent is intentionally retained.
    pub intentionally_unfaced: bool,
}
impl FacingVerification {
    /// Recheck actual motion and describe it without hiding global deviation.
    pub fn display(
        &self,
        group: &ToolpathGroup,
        context: &JobContext<'_>,
    ) -> Result<FacingDisplay, AppError> {
        self.validate(group, context)?;
        // Legacy Flat's unfaced spans beside a held region come from the
        // cut itself (decision 0685).
        let description = openchuck_path::facing::description::describe_path(
            &self.setup(),
            &self.0.parameters,
            group,
            context,
        )?;
        let mut display = nominal(description)?;
        // The cutter's sweep along the cutting moves only, at the tool's tip
        // (the owner's decision S-1, issue #499): the outline of the tool
        // over the face it cuts. Plunges, entries and links are not drawn,
        // nor the top of the flutes; the toolpath itself shows those.
        let r = 0.5 * self.0.tool.diameter.0;
        let circle = |tip: Point3, u: Vec3, v: Vec3| -> Vec<Point3> {
            (0..=32)
                .map(|i| {
                    let a = std::f64::consts::TAU * f64::from(i) / 32.;
                    tip + (u * a.cos() + v * a.sin()) * r
                })
                .collect()
        };
        let mut from = group.start;
        for segment in &group.segments {
            context.check_cancelled()?;
            let to = segment.end;
            if segment.kind == MoveKind::Cutting && from.a == to.a {
                let delta = to.tip - from.tip;
                let normal = from.axis.get();
                let across = normal.cross(delta);
                let u = if across.length() > 1e-12 {
                    across * (1. / across.length())
                } else {
                    Vec3::new(1., 0., 0.)
                };
                let v = normal.cross(u);
                for tip in [from.tip, to.tip] {
                    display.lines.push(ScenePolyline {
                        facing: None,
                        role: SceneRole::FacingSweep,
                        points: circle(tip, u, v),
                    });
                }
                for offset in [u * r, u * (-r)] {
                    display.lines.push(ScenePolyline {
                        facing: None,
                        role: SceneRole::FacingSweep,
                        points: vec![from.tip + offset, to.tip + offset],
                    });
                }
            }
            if segment.kind == MoveKind::Cutting && from.a != to.a {
                let count = ((to.a.0 - from.a.0).abs() / 5.).ceil();
                if !count.is_finite() || count > 72. {
                    return Err(AppError::Post(
                        "Facing rotary diagram exceeds its angular work budget".into(),
                    ));
                }
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let count = count as u32;
                let frame = openchuck_types::kinematics::RotaryFrame::of_profile(&self.0.machine);
                for i in 0..=count {
                    context.check_cancelled()?;
                    let (tip, normal, _, _) = crate::playback::between(
                        &frame,
                        &from,
                        &to,
                        f64::from(i) / f64::from(count.max(1)),
                    );
                    let u = Vec3::new(1., 0., 0.);
                    display.lines.push(ScenePolyline {
                        facing: None,
                        role: SceneRole::FacingSweep,
                        points: circle(tip, u, normal.cross(u)),
                    });
                }
            }
            if display.lines.len() > 200_000 {
                return Err(AppError::Post(
                    "Facing diagram exceeds its work budget".into(),
                ));
            }
            from = to;
        }
        // The diagram is of the path, not of a simulation (decision 0536,
        // the owner's answer of 8 Nov 2026 to (c2), refined).
        display.completion = super::completion_note(&display.description).into();
        Ok(display)
    }
}

pub(super) fn nominal(description: FacingDescription) -> Result<FacingDisplay, AppError> {
    use crate::facing_labels::FacingLabel;
    use openchuck_path::facing::extents::FacingExtent;
    let mut lines = Vec::new();
    let mut labels = Vec::new();
    let margin = description.margin;
    let label = |role: SceneRole, at: Point3, kind: FacingLabel, text: String| SceneLabel {
        role,
        at,
        text,
        facing: Some(tag(kind)),
    };
    // The extent's outline and, when the margin is not zero, the margin's
    // outline inside it; the face's own label at its far corner.
    let mut margin_at: Option<Point3> = None;
    let mut rectangle = |plane: FacingPlane,
                         extent: CoverageRectangle,
                         h: f64,
                         text: String|
     -> Result<(), AppError> {
        let lo = extent.min();
        let hi = extent.max();
        for (m, role) in [
            (0., SceneRole::FacingExtent),
            (margin, SceneRole::FacingMargin),
        ] {
            if m == 0. && role == SceneRole::FacingMargin {
                continue;
            }
            let points = [
                (lo[0] + m, lo[1] + m),
                (hi[0] - m, lo[1] + m),
                (hi[0] - m, hi[1] - m),
                (lo[0] + m, hi[1] - m),
                (lo[0] + m, lo[1] + m),
            ]
            .into_iter()
            .map(|(u, v)| plane.point(u, v, h))
            .collect::<Result<Vec<_>, _>>()?;
            if role == SceneRole::FacingMargin && margin_at.is_none() {
                margin_at = points.get(2).copied();
            }
            lines.push(ScenePolyline {
                role,
                points,
                facing: None,
            });
        }
        labels.push(label(
            SceneRole::FacingExtent,
            plane.point(hi[0], hi[1], h)?,
            FacingLabel::Face,
            text,
        ));
        Ok(())
    };
    match &description.geometry {
        FacingGeometry::Flat { faces } | FacingGeometry::Box { faces } => {
            for face in faces {
                let (a, offset) = (face.target.angle.0, face.target.offset);
                rectangle(
                    face.target.plane,
                    face.coverage,
                    offset,
                    face_text(a, offset),
                )?;
            }
            // Where each face stops beside a held region (decision 0685):
            // the line on each face, one label per held end.
            for face in faces {
                let [_, v0] = face.coverage.min();
                let [_, v1] = face.coverage.max();
                for u in &face.unfaced {
                    let x = unfaced_edge(u);
                    let h = face.target.offset;
                    lines.push(ScenePolyline {
                        role: SceneRole::FacingExtent,
                        points: vec![
                            face.target.plane.point(x, v0, h)?,
                            face.target.plane.point(x, v1, h)?,
                        ],
                        facing: Some(tag(FacingLabel::Stop)),
                    });
                    let text = unfaced_label(u);
                    if labels.iter().any(|l| l.text == text) {
                        continue;
                    }
                    labels.push(label(
                        SceneRole::FacingExtent,
                        face.target.plane.point(x, v1, h)?,
                        FacingLabel::Stop,
                        text,
                    ));
                }
            }
            // The distance between each pair of opposite faces, once per
            // pair, at a corner of its first face.
            for (i, a) in faces.iter().enumerate() {
                for b in faces.iter().skip(i + 1) {
                    if (a.target.plane.normal() + b.target.plane.normal()).length() < 1e-9 {
                        let apart = a.target.offset + b.target.offset;
                        labels.push(label(
                            SceneRole::FacingMargin,
                            a.corners()?[0],
                            FacingLabel::Separation,
                            separation_text(a.target.angle.0, b.target.angle.0, apart),
                        ));
                    }
                }
            }
        }
        FacingGeometry::End {
            end,
            target,
            coverage,
        } => {
            let plane = FacingPlane::end(*end);
            rectangle(
                plane,
                *coverage,
                plane.normal().x * target.x,
                fit(|d| format!("End face at X {:.d$} mm", target.x)),
            )?;
        }
        FacingGeometry::Round {
            diameter,
            x_extent,
            unfaced,
        } => {
            let ring = |x: f64| -> Vec<Point3> {
                (0..=64)
                    .map(|i| {
                        let a = std::f64::consts::TAU * f64::from(i) / 64.;
                        Point3::new(x, 0.5 * diameter * a.cos(), 0.5 * diameter * a.sin())
                    })
                    .collect()
            };
            for u in unfaced {
                let x = unfaced_edge(u);
                lines.push(ScenePolyline {
                    role: SceneRole::FacingExtent,
                    points: ring(x),
                    facing: Some(tag(FacingLabel::Stop)),
                });
                labels.push(label(
                    SceneRole::FacingExtent,
                    Point3::new(x, 0., 0.5 * diameter),
                    FacingLabel::Stop,
                    unfaced_label(u),
                ));
            }
            for (m, role) in [
                (0., SceneRole::FacingExtent),
                (margin, SceneRole::FacingMargin),
            ] {
                if m == 0. && role == SceneRole::FacingMargin {
                    continue;
                }
                for x in [x_extent.0 + m, x_extent.1 - m] {
                    if role == SceneRole::FacingMargin && margin_at.is_none() {
                        margin_at = Some(Point3::new(x, 0., 0.5 * diameter));
                    }
                    lines.push(ScenePolyline {
                        role,
                        points: ring(x),
                        facing: None,
                    });
                }
            }
            for (y, z) in [
                (0.5 * diameter, 0.),
                (-0.5 * diameter, 0.),
                (0., 0.5 * diameter),
                (0., -0.5 * diameter),
            ] {
                lines.push(ScenePolyline {
                    role: SceneRole::FacingExtent,
                    points: vec![Point3::new(x_extent.0, y, z), Point3::new(x_extent.1, y, z)],
                    facing: None,
                });
            }
            labels.push(label(
                SceneRole::FacingExtent,
                Point3::new(x_extent.1, 0., 0.5 * diameter),
                FacingLabel::Face,
                fit(|d| {
                    format!(
                        "Ø{diameter:.d$} mm · X {:.d$}–{:.d$}",
                        x_extent.0, x_extent.1
                    )
                }),
            ));
        }
    }
    // The margin, when it is not zero: one label, on its outline.
    if margin > 0.
        && let Some(at) = margin_at
    {
        let edges = if matches!(description.geometry, FacingGeometry::Round { .. }) {
            "ends"
        } else {
            "edges"
        };
        labels.push(label(
            SceneRole::FacingMargin,
            at,
            FacingLabel::Margin,
            fit(|d| format!("Margin: {margin:.d$} mm past the {edges}")),
        ));
    }
    Ok(FacingDisplay {
        intentionally_unfaced: description.extent != FacingExtent::WholeStock,
        description,
        lines,
        labels,
        completion: "Draft Facing extent; generate to verify completion".into(),
    })
}

/// The longest a Facing label in the view may be, in characters (the
/// owner's answer to issue #881).
pub(crate) const LABEL_CHARS: usize = 32;

/// A label with its numbers to three decimals, or fewer when that is what
/// keeps it within [`LABEL_CHARS`] (a face far from the axis).
fn fit(text: impl Fn(usize) -> String) -> String {
    (0..=3)
        .rev()
        .map(&text)
        .find(|t| t.chars().count() <= LABEL_CHARS)
        .unwrap_or_else(|| text(0))
}

/// A label with a length and angles: every decimal while it fits in
/// [`LABEL_CHARS`], then the length and the angles give up decimals in turn
/// (issue #965), never a word or a number. `text(d, a)` writes the length
/// to `d` decimals and the angles to at most `a`.
fn fit_angled(text: impl Fn(usize, usize) -> String) -> String {
    const STEPS: [(usize, usize); 7] = [(3, 3), (2, 3), (2, 2), (1, 2), (1, 1), (0, 1), (0, 0)];
    STEPS
        .iter()
        .map(|&(d, a)| text(d, a))
        .find(|t| t.chars().count() <= LABEL_CHARS)
        .unwrap_or_else(|| text(0, 0))
}

/// A face's label: its angle and its finished plane's distance from the
/// rotary axis.
fn face_text(angle: f64, offset: f64) -> String {
    fit_angled(|d, a| {
        format!(
            "Face A {}°: {offset:.d$} mm from axis",
            degrees_to(angle, a)
        )
    })
}

/// The label of a pair of opposite faces: their angles and the distance
/// between them.
fn separation_text(from: f64, to: f64, apart: f64) -> String {
    fit_angled(|d, a| {
        format!(
            "A {}° to A {}°: {apart:.d$} mm apart",
            degrees_to(from, a),
            degrees_to(to, a)
        )
    })
}

/// An angle in degrees as the labels write it: whole degrees without a
/// fraction, otherwise to at most `places` decimals (three at most, a
/// tenth of a millidegree), trailing zeros dropped.
fn degrees_to(a: f64, places: usize) -> String {
    let text = format!("{a:.places$}");
    // Only a fraction's trailing zeros go: "180" with no decimals stays.
    let text = if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.')
    } else {
        text.as_str()
    };
    if text == "-0" {
        "0".to_owned()
    } else {
        text.to_owned()
    }
}

/// A label's tag before its operation is known.
fn tag(kind: crate::facing_labels::FacingLabel) -> crate::facing_labels::FacingTag {
    crate::facing_labels::FacingTag {
        operation: None,
        kind,
    }
}

/// Where the face is finished up to beside a held region: the unfaced
/// span's end away from the grip.
fn unfaced_edge(u: &openchuck_path::facing::held::Unfaced) -> f64 {
    match u.beside {
        openchuck_path::facing::geometry::FacingEnd::Start => u.x.1,
        openchuck_path::facing::geometry::FacingEnd::Far => u.x.0,
    }
}

/// The overlay's label where a face stops beside a held region: the X it is
/// faced from (or up to) and which end is held (the owner's answer to issue
/// #881, Q2 A); how much is left unfaced is in the notes.
fn unfaced_label(u: &openchuck_path::facing::held::Unfaced) -> String {
    match u.beside {
        openchuck_path::facing::geometry::FacingEnd::Start => {
            fit(|d| format!("Faced from X {:.d$} (held start)", u.x.1))
        }
        openchuck_path::facing::geometry::FacingEnd::Far => {
            fit(|d| format!("Faced up to X {:.d$} (held end)", u.x.0))
        }
    }
}

/// Build draft geometry inside the existing scene worker from its immutable
/// project snapshot. Missing/invalid settings produce a visible reason.
pub(crate) fn preview(
    project: &openchuck_project::Project,
    setup: &openchuck_project::Setup,
    part: &TriangleMesh,
    projection_error: Option<f64>,
    view: &mut crate::scene::SceneView,
    context: &JobContext<'_>,
) -> Result<(), AppError> {
    use openchuck_path::{ParameterValue, Strategy};
    let registry = openchuck_path::Registry::builtin();
    for operation in project
        .operations()
        .iter()
        .filter(|o| o.setup == setup.id && o.strategy == openchuck_path::facing::NAME)
    {
        context.check_cancelled()?;
        let result = (|| -> Result<FacingDisplay, AppError> {
            let machine = project
                .machine()
                .ok_or_else(|| AppError::Post("choose a machine for the Facing preview".into()))?;
            let stock = setup
                .stock
                .as_ref()
                .ok_or_else(|| AppError::Post("define stock for the Facing preview".into()))?;
            let tool = operation
                .tool
                .and_then(|id| project.tools().iter().find(|t| t.id == id))
                .ok_or_else(|| AppError::Post("choose a tool for the Facing preview".into()))?;
            let extent = openchuck_geom::part_extent(part)
                .ok_or_else(|| AppError::Post("Facing part has no extent".into()))?;
            let protected = crate::workholding::solids(&setup.workholding, Some(stock), &extent)?;
            let mut tabs = setup.workholding.resolved(Some(stock), Some(&extent));
            tabs.start = None;
            tabs.end = None;
            for tab in &mut tabs.tabs {
                tab.auto = None;
            }
            let tabs = crate::workholding::solids(&tabs, Some(stock), &extent)?;
            let bounds = stock
                .bounds(Some(&extent))
                .ok_or_else(|| AppError::Post("Facing stock has no bounds".into()))?;
            let held = setup
                .workholding
                .held_intervals((bounds.min.x, bounds.max.x));
            let values = crate::operation::parameter_values(project, operation, &registry)?.values;
            let mut schema = Facing.schema_for(&values);
            // Only target/extent settings are required to draw a DRAFT.
            // No invented feed, stepdown or tool-size default is inserted.
            schema.parameters.retain(|s| {
                matches!(
                    s.name.as_str(),
                    "shape"
                        | "size"
                        | "angles"
                        | "extent"
                        | "extent_margin"
                        | "stock_to_leave"
                        | "arbitrary_faces"
                        | "positive_z"
                        | "positive_y"
                        | "negative_z"
                        | "negative_y"
                ) || s.name.ends_with("_offset")
                    || s.name.ends_with("_from_part")
            });
            let values: std::collections::BTreeMap<String, ParameterValue> = values
                .into_iter()
                .filter(|(name, _)| schema.get(name).is_some())
                .collect();
            let parameters = schema.resolve(&values)?;
            let shape = parameters.text(openchuck_path::facing::parameter::SHAPE)?;
            if shape == "box"
                && !matches!(stock, Stock::Box { .. } | Stock::OffsetBoundingBox { .. })
            {
                return Err(AppError::Post("Box sides requires box/block stock".into()));
            }
            if matches!(shape, "box" | "end")
                && !matches!(tool.tool.shape, openchuck_types::tool::ToolShape::FlatEnd)
            {
                return Err(AppError::Post(
                    "this Facing construction requires a flat end mill".into(),
                ));
            }
            let data = FacingSetup {
                part,
                stock,
                tool: &tool.tool,
                machine,
                tabs: &tabs,
                protected: &protected,
                held: [held.0, held.1],
                projection_error,
            };
            nominal(describe(&data, &parameters)?)
        })();
        match result {
            Ok(mut display) => {
                // The labels stay short (issue #881): the operation is in
                // the tag, which the view uses to show the selected Facing
                // operation's labels; that the extent is a draft, and that
                // its bounds are the tessellation's, are in the notes.
                place(&mut display, operation.id);
                view.notes.push(draft_note(&operation.name));
                if display.description.source_projection_error.is_none() {
                    view.notes.push(format!("{}: Facing source projection accuracy is unknown; so Facing cannot be checked against these tessellation bounds",operation.name));
                }
                view.lines.extend(display.lines);
                view.labels.extend(display.labels);
            }
            Err(error) => view.notes.push(format!(
                "{}: Facing preview unavailable: {}",
                operation.name,
                error.user_message()
            )),
        }
    }
    Ok(())
}

pub(crate) fn add_generated(
    view: &mut crate::scene::SceneView,
    run: &crate::Regeneration,
    project: &openchuck_project::Project,
    context: &JobContext<'_>,
) -> Result<(), AppError> {
    // Each operation's sweep shows while its own path is current and its eye
    // is on (the owner's decision S-1, issue #499), not only while the whole
    // project is exactly the one the run was made from.
    let current = crate::export::current_operations(project, run);
    for (group, info) in run.toolpath.groups.iter().zip(&run.groups) {
        if Some(info.setup) != view.setup
            || !current.contains(&info.operation)
            || !project.path_shown(info.operation)
        {
            continue;
        }
        if let Some(proof) = &info.facing {
            let mut display = proof.display(group, context)?;
            place(&mut display, info.operation);
            view.lines.extend(
                display
                    .lines
                    .iter()
                    .filter(|l| l.role == SceneRole::FacingSweep)
                    .cloned(),
            );
            // Where the face stops is read from the cut once it is made
            // (legacy Flat, decision 0685): the generated stop marker takes
            // the draft's place, so the view shows one, where the notes say
            // (issue #895 item 1).
            let is_stop = |t: &Option<crate::facing_labels::FacingTag>| {
                t.is_some_and(|t| {
                    t.operation == Some(info.operation)
                        && t.kind == crate::facing_labels::FacingLabel::Stop
                })
            };
            view.lines.retain(|l| !is_stop(&l.facing));
            view.labels.retain(|l| !is_stop(&l.facing));
            view.lines
                .extend(display.lines.into_iter().filter(|l| is_stop(&l.facing)));
            view.labels
                .extend(display.labels.into_iter().filter(|l| is_stop(&l.facing)));
            // Generated: no longer a draft.
            if let Some(operation) = project.operation(info.operation) {
                let draft = draft_note(&operation.name);
                view.notes.retain(|n| *n != draft);
            }
        }
    }
    Ok(())
}

/// The note that says an operation's Facing outlines and labels are a
/// draft, said once instead of on every label (issue #881).
fn draft_note(operation: &str) -> String {
    format!("{operation}: the Facing outlines and labels are a draft until you generate")
}

/// Tags a display's Facing labels and stop lines with the operation it was
/// drawn for.
fn place(display: &mut FacingDisplay, operation: openchuck_project::id::OperationId) {
    for tag in display
        .labels
        .iter_mut()
        .map(|l| &mut l.facing)
        .chain(display.lines.iter_mut().map(|l| &mut l.facing))
        .flatten()
    {
        tag.operation = Some(operation);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        LABEL_CHARS, degrees_to, face_text, fit, separation_text, unfaced_edge, unfaced_label,
    };
    use openchuck_path::facing::geometry::FacingEnd;
    use openchuck_path::facing::held::Unfaced;

    /// Issue #881, the owner's answer Q2 A: beside the held start a face is
    /// "faced from" the unfaced span's far edge; beside the held end it is
    /// "faced up to" its near edge, and the stop is drawn at that X.
    #[test]
    fn req_ui_03_req_op_24_a_stop_label_names_the_x_and_the_held_end() {
        let start = Unfaced {
            beside: FacingEnd::Start,
            held: (16.99, 24.99),
            x: (16.99, 25.49),
        };
        assert_eq!(unfaced_label(&start), "Faced from X 25.490 (held start)");
        assert!((unfaced_edge(&start) - 25.49).abs() < 1e-12);
        let end = Unfaced {
            beside: FacingEnd::Far,
            held: (86.49, 94.49),
            x: (85.99, 94.49),
        };
        assert_eq!(unfaced_label(&end), "Faced up to X 85.990 (held end)");
        assert!((unfaced_edge(&end) - 85.99).abs() < 1e-12);
        let far = Unfaced {
            beside: FacingEnd::Far,
            held: (1000.0, 1010.0),
            x: (1234.5678, 1300.0),
        };
        assert!(unfaced_label(&far).chars().count() <= LABEL_CHARS);
    }

    /// Issue #881: a label keeps its three decimals while it fits in 32
    /// characters, and gives up decimals, never words, when it would not (a
    /// face 100 mm or more from the axis).
    #[test]
    fn req_ui_03_a_facing_label_gives_up_decimals_to_stay_within_32_characters() {
        let face = |offset: f64| fit(|d| format!("Face A 270°: {offset:.d$} mm from axis"));
        assert_eq!(face(13.65), "Face A 270°: 13.650 mm from axis");
        assert_eq!(face(123.456), "Face A 270°: 123.46 mm from axis");
        for offset in [0.0, 9.999, 99.9995, 1234.5678, 123_456.789] {
            assert!(
                face(offset).chars().count() <= LABEL_CHARS,
                "{}",
                face(offset)
            );
        }
        assert_eq!(degrees_to(90.0, 3), "90");
        assert_eq!(degrees_to(22.5, 3), "22.5");
        assert_eq!(degrees_to(-0.0, 3), "0");
        assert_eq!(degrees_to(12.3456, 3), "12.346");
        assert_eq!(degrees_to(180.0, 0), "180");
        assert_eq!(degrees_to(202.125, 1), "202.1");
    }

    /// Issue #965 (CodeRabbit on #950): a label with angles stays within 32
    /// characters for any angle, giving up the angles' decimals too, never
    /// a word or a number; it keeps every decimal while it fits.
    #[test]
    fn req_ui_03_a_label_with_arbitrary_angles_stays_within_32_characters() {
        assert_eq!(
            separation_text(0.0, 180.0, 27.3),
            "A 0° to A 180°: 27.300 mm apart"
        );
        let odd = separation_text(22.125, 202.125, 27.0);
        assert!(odd.chars().count() <= LABEL_CHARS, "{odd}");
        assert!(
            odd.starts_with("A 22") && odd.contains("° to A 202") && odd.contains(" mm apart"),
            "{odd}"
        );
        assert_eq!(face_text(270.0, 13.65), "Face A 270°: 13.650 mm from axis");
        for (a, offset) in [
            (22.125, 13.65),
            (202.125, 123.456),
            (359.999, 999.999),
            (-179.875, 0.5),
        ] {
            let face = face_text(a, offset);
            assert!(face.chars().count() <= LABEL_CHARS, "{face}");
            assert!(
                face.starts_with("Face A ") && face.ends_with(" mm from axis"),
                "{face}"
            );
        }
        for (from, to, apart) in [
            (22.125, 202.125, 999.999),
            (359.875, 179.875, 27.3),
            (-179.875, 0.125, 54.0),
        ] {
            let text = separation_text(from, to, apart);
            assert!(text.chars().count() <= LABEL_CHARS, "{text}");
        }
    }
}
