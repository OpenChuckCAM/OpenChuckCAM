// SPDX-License-Identifier: AGPL-3.0-or-later
//! Machine and setups: choosing the project's machine profile, and checking
//! each setup against its stock, part and machine (SET-01, SET-03, SET-05,
//! SET-06; work breakdown slice 3, decision record 0011).
//!
//! The types a front end needs are re-exported here, so it needs no engine
//! crate of its own (architecture section 5.8).

use openchuck_types::geometry::Vec3;
use openchuck_types::kinematics::{RotaryFrame, facing_at, model_up};

pub use openchuck_project::machines::{
    MACHINE_SCHEMA, MAKERA_Z1, ProfileEntry, builtin, builtin_entry, builtin_keys,
    has_old_z1_a_limit, has_old_z1_axis_height, list_library, list_library_entries, load_profile,
    load_profile_entry, new_machine_id, save_profile, save_profile_entry,
};
pub use openchuck_types::geometry::{Aabb, Point3};
pub use openchuck_types::machine::{
    ARange, ASense, Clearance, Confirmation, MachineProfile, MaxFeeds, ProfileField, Provenance,
    RotaryAxis, ShortBlockMode,
};
pub use openchuck_types::setup::{
    ClearanceRules, DEFAULT_STOCK_MARGIN, EffectiveClearance, MinimumSource, PartExtent,
    SetupFrame, SetupWarning, SetupWarningKind, WorkOffset, WorkOffsetRegister,
    expected_work_offset,
};
pub use openchuck_types::stock::Stock;
pub use openchuck_types::units::Millimetres;
pub use openchuck_types::workholding::{
    HeldRegion, Tab, TabEnd, TabLayout, TabShape, Workholding, auto_tab_span, validate_workholding,
};

use openchuck_project::id::SetupId;

/// What checking one setup found (SET-05, SET-06).
#[derive(Debug, Clone, PartialEq)]
pub struct SetupReport {
    /// The setup.
    pub setup: SetupId,
    /// Its name.
    pub name: String,
    /// Problems with stock, clearance and travel, in a fixed order. Empty
    /// when the project has no machine (see [`SetupReport::notes`]).
    pub warnings: Vec<SetupWarning>,
    /// What the work offset register should hold (machine coordinates of
    /// the work zero at A = 0), when the project has a machine.
    pub expected_work_offset: Option<Point3>,
    /// The part's extent in the model frame, when it could be read.
    pub part: Option<PartExtent>,
    /// Other things the user should know: no machine, an unreadable part,
    /// a tab narrower than a tool; each in the one form (decision record
    /// 0644).
    pub notes: Vec<String>,
    /// The machine X of the profile's rotary reference point, the setup's
    /// X 0, when the project has a machine: the travel warnings along the
    /// rotary axis give the setup's X from it (decision record 0672).
    pub rotary_reference_x: Option<f64>,
}

impl SetupReport {
    /// Its problems in the one form, "<Topic> (setup '<name>'): <text>.",
    /// as Messages and the export checks say them (decision records 0536
    /// section 3 and 0644).
    pub fn warning_lines(&self) -> Vec<String> {
        self.warnings
            .iter()
            .map(|w| crate::overview::setup_warning_in_form(&self.name, w, self.rotary_reference_x))
            .collect()
    }
}

/// The direction of positive A in words, from the shared kinematics: where
/// the side of the part that faces up at A = 0 faces at A = +90. Shown by the
/// CLI now and in the viewport in slice 8 (work breakdown slice 3 review
/// checklist).
pub fn a_direction(profile: &MachineProfile) -> String {
    let frame = RotaryFrame::of_profile(profile);
    let facing = facing_at(
        &frame,
        model_up(&frame),
        openchuck_types::units::Degrees(90.0),
    );
    format!(
        "positive A turns the part about +{} so that the side facing up (+Z) at A 0 faces {} at A 90",
        profile.rotary_axis.name(),
        direction_name(facing)
    )
}

/// The machine direction nearest `v`, such as `-Y`.
pub fn direction_name(v: Vec3) -> &'static str {
    let candidates = [
        (Vec3::new(1.0, 0.0, 0.0), "+X"),
        (Vec3::new(-1.0, 0.0, 0.0), "-X"),
        (Vec3::new(0.0, 1.0, 0.0), "+Y"),
        (Vec3::new(0.0, -1.0, 0.0), "-Y"),
        (Vec3::new(0.0, 0.0, 1.0), "+Z"),
        (Vec3::new(0.0, 0.0, -1.0), "-Z"),
    ];
    candidates
        .iter()
        .max_by(|a, b| a.0.dot(v).total_cmp(&b.0.dot(v)))
        .map_or("?", |c| c.1)
}

/// The clearance `setup` uses on `machine` (SET-06, decision record 0495):
/// its own or the machine's, grown from the stock when Clearance from stock
/// is on, with the minimum it is checked against. `part_bounds` are the
/// setup's part bounds (in the setup's frame), which bounding-box stock
/// measures. Every consumer (toolpaths, links, the post and the checks)
/// takes the clearance from here, so they all use the same one.
pub fn effective_clearance(
    machine: &MachineProfile,
    setup: &openchuck_project::Setup,
    part_bounds: Option<&Aabb>,
) -> EffectiveClearance {
    let part = part_bounds.map(part_extent);
    openchuck_types::setup::effective_clearance(
        machine,
        setup.clearance.as_ref(),
        &setup.clearance_rules,
        setup.stock.as_ref(),
        part.as_ref(),
    )
}

/// The extent of a part whose bounds, in the setup's frame, are `bounds`:
/// the bounds and the radius they reach from the rotary axis.
pub fn part_extent(bounds: &Aabb) -> PartExtent {
    PartExtent {
        bounds: *bounds,
        radius: Millimetres(
            bounds
                .min
                .y
                .abs()
                .max(bounds.max.y.abs())
                .hypot(bounds.min.z.abs().max(bounds.max.z.abs())),
        ),
    }
}

/// Whether a setup check's finding is a possible collision, which must be
/// acknowledged before the program is written (decision record 0538): a safe
/// cylinder that does not enclose the stock or the part, or is inside the
/// minimum clearance, so links at the clearance may pass through material or
/// the workholding. A travel limit is not one: the controller stops at it.
pub fn is_possible_collision(kind: &openchuck_types::setup::SetupWarningKind) -> bool {
    use openchuck_types::setup::SetupWarningKind as K;
    matches!(
        kind,
        K::ClearanceInsideStock { .. }
            | K::ClearanceInsidePart { .. }
            | K::ClearanceBelowMinimum { .. }
    )
}

/// Whether `profile`, the project's machine copy whose library id is `id`,
/// is a copy of the built-in Makera Z1 (by its id, or by its name when it has
/// no id) with the built-in's Rotary reference X,
/// so its rotary reference point, X 0 of every setup, is the headstock face
/// (decision records 0649 and 0672). An older copy (reference X -180.4, in
/// front of the face) or another machine is not: what its reference point
/// is on the machine is its profile's choice, and the program does not know
/// it.
pub fn reference_is_headstock_face(id: Option<&str>, profile: &MachineProfile) -> bool {
    // Known as the old-copy checks know a Z1 copy: by its library id, or by
    // its name when it has none (decision records 0647 and 0651).
    let z1 = match id {
        Some(id) => id == openchuck_project::machines::MAKERA_Z1_ID,
        None => profile.name == openchuck_project::machines::MAKERA_Z1_NAME,
    };
    z1 && builtin(MAKERA_Z1)
        .and_then(Result::ok)
        .is_some_and(|z1| (z1.rotary_origin.x - profile.rotary_origin.x).abs() < 1e-9)
}

/// The setup checks of [`openchuck_types::setup::check_setup`], and on a
/// machine whose reference point is its headstock face
/// ([`reference_is_headstock_face`]) a stock (or part) starting behind it,
/// before the travel warnings (decision record 0672). `id` is the machine
/// copy's library id.
pub fn check_setup(
    id: Option<&str>,
    check: &openchuck_types::setup::SetupCheck<'_>,
) -> Vec<SetupWarning> {
    let mut found = openchuck_types::setup::check_setup(check);
    if reference_is_headstock_face(id, check.profile)
        && let Some(behind) = openchuck_types::setup::starts_behind_reference(check)
    {
        let at = found
            .iter()
            .position(|w| matches!(w.kind, SetupWarningKind::OutsideTravel { .. }))
            .unwrap_or(found.len());
        found.insert(at, behind);
    }
    found
}

/// The length a tool is assumed to stick out beyond its flutes, since the
/// tool library holds no stick-out (decision record 0318).
pub const STICK_OUT_BEYOND_FLUTES: f64 = 5.0;

/// A tool that would take the spindle past the top of the Z travel at the
/// safe Z (decision record 0318), in millimetres.
#[derive(Debug, Clone, PartialEq)]
pub struct SpindleHeight {
    /// The tool's assumed stick-out: flute length + 5 mm.
    pub stick_out: f64,
    /// The highest the tool's tip reaches above the axis: the spindle-to-axis
    /// distance less the stick-out. A safe Z up to this fits.
    pub highest_tip: f64,
    /// How far past the top of the travel the spindle would go.
    pub past_top: f64,
    /// The profile's recommended minimum safe Z, when the fit is under it.
    pub recommended_min: Option<f64>,
    /// The minimum clearance radius, when the tool's highest tip is inside
    /// it: no safe Z then clears both the workholding and the top of the Z
    /// travel (decision record 0495).
    pub below_minimum: Option<f64>,
}

/// Whether `tool` would take the spindle past the top of the Z travel at
/// `safe_z` above the axis on `profile` (decision record 0318), or cannot
/// rise clear of the `minimum` clearance radius within the Z travel at all
/// (decision record 0495): only on a machine that measures tool lengths and
/// gives its spindle-to-axis distance. `safe_z` is the setup's effective
/// safe Z ([`effective_clearance`]).
pub fn spindle_height(
    profile: &MachineProfile,
    safe_z: f64,
    minimum: Option<f64>,
    tool: &openchuck_types::tool::Tool,
) -> Option<SpindleHeight> {
    if !profile.automatic_tool_length {
        return None;
    }
    let distance = profile.spindle_to_axis?.0;
    let stick_out = tool.flute_length.0 + STICK_OUT_BEYOND_FLUTES;
    let highest_tip = distance - stick_out;
    // Millimetres to three places, as the G-code: a fit equal to the safe Z
    // within rounding passes.
    let past_top = safe_z - highest_tip;
    let below_minimum = minimum.filter(|m| m - highest_tip > 5e-4);
    if past_top <= 5e-4 && below_minimum.is_none() {
        return None;
    }
    Some(SpindleHeight {
        below_minimum,
        stick_out,
        highest_tip,
        past_top,
        recommended_min: profile
            .recommended_min_safe_z
            .map(|m| m.0)
            .filter(|m| highest_tip < *m),
    })
}

/// The setup check's words for a tool of [`spindle_height`] (NFR-12).
/// The tool is named in the message's lead-in, "Tool length (tool '<name>',
/// setup '<name>'): " (decision record 0536 section 3), so the text names
/// only its number.
pub fn spindle_height_text(number: u32, found: &SpindleHeight) -> String {
    let mut text = if let Some(minimum) = found.below_minimum {
        // Lowering the safe Z would take links into the workholding, so the
        // 0318 advice is not given (decision record 0495).
        format!(
            "tool {number} may be too long to clear the workholding within the Z travel: \
             assuming it sticks out {:.3} mm (flute length + 5 mm), its tip can rise at most {:.3} mm \
             above the axis, inside the minimum clearance radius of {minimum:.3} mm, so no safe Z \
             clears both the chuck, jaws, headstock or tailstock and the top of the Z travel. \
             Use a shorter tool, and check the clearance over the chuck and the Z travel on your \
             machine before running.",
            found.stick_out, found.highest_tip
        )
    } else {
        format!(
            "tool {number} may take the spindle above the top of the Z travel at the safe Z: \
         assuming it sticks out {:.3} mm (flute length + 5 mm), the spindle would be {:.3} mm past the top. \
         Lower the setup's safe Z and clearance to {:.3} mm or less, or use a shorter tool, \
         and check the clearance over the chuck and the Z travel on your machine before running.",
            found.stick_out, found.past_top, found.highest_tip
        )
    };
    if let Some(minimum) = found.recommended_min {
        text.push_str(&format!(
            " The recommended minimum is {minimum} mm. This must be checked before machining."
        ));
    }
    text
}

/// A tool whose path goes lower than the spindle at the bottom of the Z
/// travel can take its tip (decision record 0673, decision record 0495 case
/// c), in millimetres from the rotary axis.
#[derive(Debug, Clone, PartialEq)]
pub struct SpindleDepth {
    /// The tool's assumed stick-out: flute length + 5 mm.
    pub stick_out: f64,
    /// The lowest the tool's tip goes in the path, measured along the tool
    /// axis from the rotary axis (negative below it).
    pub lowest_tip: f64,
    /// The lowest the tip can go: the spindle-to-axis distance, less the Z
    /// travel span, less the stick-out.
    pub lowest_reach: f64,
    /// The move of the group where the tip is lowest, by index.
    pub segment: usize,
}

/// Whether the path `group`, cut by `tool`, needs the spindle below the
/// bottom of the Z travel of `profile` (decision record 0673): only on a
/// machine that measures tool lengths and gives its spindle-to-axis
/// distance, as for the top of the travel ([`spindle_height`]). Every move
/// counts, rapids and links included. The tip's height is measured along
/// the tool axis from the rotary axis: on a 4-axis machine the tool axis
/// crosses the rotary axis at right angles, so it is the tip's component
/// along the tool axis in the model frame. To three places, as the G-code: a
/// tip within 0.0005 mm of the limit passes.
pub fn spindle_depth(
    profile: &MachineProfile,
    tool: &openchuck_types::tool::Tool,
    group: &openchuck_types::ir::ToolpathGroup,
) -> Option<SpindleDepth> {
    if !profile.automatic_tool_length {
        return None;
    }
    let distance = profile.spindle_to_axis?.0;
    let span = profile.travel.max.z - profile.travel.min.z;
    let stick_out = tool.flute_length.0 + STICK_OUT_BEYOND_FLUTES;
    let lowest_reach = distance - span - stick_out;
    let height = |pose: &openchuck_types::ir::ToolPose| {
        let (tip, axis) = (pose.tip, pose.axis.get());
        tip.y * axis.y + tip.z * axis.z
    };
    let (segment, lowest_tip) = group
        .segments
        .iter()
        .enumerate()
        .map(|(i, s)| (i, height(&s.end)))
        .fold(None, |low: Option<(usize, f64)>, (i, h)| match low {
            Some((_, l)) if l <= h => low,
            _ => Some((i, h)),
        })?;
    (lowest_reach - lowest_tip > 5e-4).then_some(SpindleDepth {
        stick_out,
        lowest_tip,
        lowest_reach,
        segment,
    })
}

/// The check's words for a tool of [`spindle_depth`] (decision record 0673,
/// NFR-12). The lead-in names the topic and the operation
/// ("Tool length (operation '<name>'): "), so the text names only the tool's
/// number.
pub fn spindle_depth_text(number: u32, found: &SpindleDepth) -> String {
    format!(
        "tool {number} may need the spindle below the bottom of the Z travel: its tip goes down \
         to {:.3} mm from the axis, and assuming it sticks out {:.3} mm (flute length + 5 mm), the \
         spindle at the bottom of the travel takes its tip only down to {:.3} mm. Use a longer \
         tool or a longer stick-out, or keep the cut further from the axis, and check the Z \
         travel on your machine before running.",
        found.lowest_tip, found.stick_out, found.lowest_reach
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A path of one group whose moves end with the tip at `heights` mm above
    /// the axis, the tool along +Z in the model frame (A 0).
    fn path(heights: &[f64]) -> openchuck_types::ir::ToolpathGroup {
        use openchuck_types::ir::{MoveKind, PathTags, Segment, ToolPose, ToolpathGroup};
        use openchuck_types::units::{Degrees, Seconds};
        let pose = |z: f64| ToolPose {
            tip: Point3::new(10.0, 0.0, z),
            axis: openchuck_types::geometry::UnitVec3::Z,
            a: Degrees(0.0),
        };
        ToolpathGroup {
            rotary_reset_after: Vec::new(),
            tags: PathTags {
                operation: "op".into(),
                setup: "setup".into(),
                tool: "tool".into(),
            },
            start: pose(50.0),
            segments: heights
                .iter()
                .map(|&z| Segment {
                    kind: MoveKind::Rapid,
                    end: pose(z),
                    contact_feed: None,
                    duration: Seconds(0.0),
                    contact_length: Millimetres(0.0),
                })
                .collect(),
        }
    }

    /// A Z1 whose Z travel is only 80 mm (Z -81 to -1), so the spindle at
    /// the bottom is 8 mm below the axis (D - T = 72 - 80): a tool that
    /// sticks out 10 mm takes its tip down to -18 mm and no lower.
    fn short_z() -> MachineProfile {
        let mut z1 = builtin_z1();
        z1.travel.min.z = -81.0;
        z1
    }

    /// The bottom of the Z travel (decision record 0673; SET-01, GC-08,
    /// NFR-12): a short tool (flute 5, stick-out 10) whose path goes 20 mm
    /// below the axis needs the spindle 2 mm below the bottom; at -18 within
    /// the G-code's three places it fits; a longer tool fits; a machine that
    /// does not measure tool lengths gets no guess.
    #[test]
    fn req_set_01_req_gc_08_a_short_tool_deep_below_the_axis_needs_the_spindle_below_the_travel() {
        let z1 = short_z();
        let found = spindle_depth(&z1, &tool(5.0), &path(&[45.0, 2.0, -20.0, 45.0])).unwrap();
        assert!((found.stick_out - 10.0).abs() < 1e-9, "{found:?}");
        assert!((found.lowest_reach - -18.0).abs() < 1e-9, "{found:?}");
        assert!((found.lowest_tip - -20.0).abs() < 1e-9, "{found:?}");
        assert_eq!(found.segment, 2);
        assert_eq!(
            spindle_depth(&z1, &tool(5.0), &path(&[45.0, -18.0004, 45.0])),
            None
        );
        assert!(spindle_depth(&z1, &tool(5.0), &path(&[45.0, -18.001])).is_some());
        assert_eq!(spindle_depth(&z1, &tool(8.0), &path(&[-20.0])), None);
        let mut manual = z1.clone();
        manual.automatic_tool_length = false;
        assert_eq!(spindle_depth(&manual, &tool(5.0), &path(&[-20.0])), None);
        let mut unknown = z1;
        unknown.spindle_to_axis = None;
        assert_eq!(spindle_depth(&unknown, &tool(5.0), &path(&[-20.0])), None);
        let text = spindle_depth_text(7, &found);
        for words in [
            "tool 7 may need the spindle below the bottom of the Z travel",
            "down to -20.000 mm from the axis",
            "sticks out 10.000 mm (flute length + 5 mm)",
            "only down to -18.000 mm",
            "check the Z travel on your machine before running",
        ] {
            assert!(text.contains(words), "{words}: {text}");
        }
    }

    /// The tip's height is measured along the tool axis: at A 90 the tool
    /// axis is along the model's Y, so a tip at model Y -20 is 20 mm below
    /// the axis whatever its model Z (decision record 0673).
    #[test]
    fn req_set_01_req_gc_08_the_tip_height_is_along_the_tool_axis() {
        let mut group = path(&[0.0]);
        if let Some(segment) = group.segments.first_mut() {
            segment.end.tip = Point3::new(10.0, -20.0, 30.0);
            segment.end.axis = openchuck_types::geometry::UnitVec3::Y;
        }
        let found = spindle_depth(&short_z(), &tool(5.0), &group).unwrap();
        assert!((found.lowest_tip - -20.0).abs() < 1e-9, "{found:?}");
    }

    /// The built-in Z1's spindle reaches 29 mm below the axis at the bottom of
    /// its travel (D 72, Z -102 to -1), so the tutorial's tools fit a path
    /// far below the axis (decision record 0649).
    #[test]
    fn req_set_01_req_gc_08_the_builtin_z1_reaches_well_below_the_axis() {
        let z1 = builtin_z1();
        assert_eq!(spindle_depth(&z1, &tool(22.0), &path(&[0.0, -50.0])), None);
        // Flute 1 mm, stick-out 6 mm: the tip reaches 72 - 101 - 6 = -35.
        assert_eq!(spindle_depth(&z1, &tool(1.0), &path(&[0.0, -35.0])), None);
        assert!(spindle_depth(&z1, &tool(1.0), &path(&[-35.001])).is_some());
    }

    /// The stock starting behind X 0 warns only on a copy of the built-in
    /// Makera Z1 with its Rotary reference X (the headstock face): not on an
    /// older Z1 copy, whose reference is 17.99 mm in front of the face, not
    /// on a copy saved under another id or name, and not on another machine, whose
    /// reference point may be its chuck face (SET-05, SET-06, NFR-12;
    /// decision record 0672, issue #812).
    #[test]
    fn req_set_05_req_set_06_only_a_current_z1_copy_warns_of_a_stock_behind_the_headstock() {
        let z1 = builtin_z1();
        let id = Some(openchuck_project::machines::MAKERA_Z1_ID);
        let frame = SetupFrame::default();
        let rules = ClearanceRules::default();
        // The tutorial's fresh stock, 1 mm behind X 0.
        let stock = Stock::Cylinder {
            diameter: Millimetres(24.0),
            length: Millimetres(63.0),
            x_start: Millimetres(-1.0),
        };
        let behind = |id: Option<&str>, profile: &MachineProfile| {
            check_setup(
                id,
                &openchuck_types::setup::SetupCheck {
                    profile,
                    frame: &frame,
                    stock: Some(&stock),
                    clearance: None,
                    rules: &rules,
                    part: None,
                },
            )
            .into_iter()
            .filter(|w| matches!(w.kind, SetupWarningKind::StartsBehindReference { .. }))
            .count()
        };
        assert!(reference_is_headstock_face(id, &z1));
        assert_eq!(behind(id, &z1), 1);
        let text = crate::overview::setup_warning_in_form(
            "Setup 1",
            &check_setup(
                id,
                &openchuck_types::setup::SetupCheck {
                    profile: &z1,
                    frame: &frame,
                    stock: Some(&stock),
                    clearance: None,
                    rules: &rules,
                    part: None,
                },
            )[0],
            Some(z1.rotary_origin.x),
        );
        for words in [
            "Stock (setup 'Setup 1'): ",
            "he stock starts at X -1.000, 1.000 mm inside the headstock (X 0 is the headstock \
             face on the Makera Z1).",
            "set the stock's Start X to at least that",
        ] {
            assert!(text.contains(words), "{words}: {text}");
        }
        let mut old = z1.clone();
        old.rotary_origin.x = -180.4;
        assert!(!reference_is_headstock_face(id, &old));
        assert_eq!(behind(id, &old), 0);
        assert_eq!(behind(Some("another-id"), &z1), 0);
        // A copy with no id is known by its name, as the old-copy checks do.
        assert_eq!(behind(None, &z1), 1);
        let mut renamed = z1.clone();
        renamed.name = "My Z1".into();
        assert_eq!(behind(None, &renamed), 0);
        assert_eq!(behind(None, &MachineProfile::example_x_rotary()), 0);
    }

    /// A position past the X travel is given in the setup's X as typed, how
    /// far past, in machine X, and what to move: issue #820's numbers on the
    /// Makera Z1 (decision record 0672).
    #[test]
    fn req_set_05_req_set_06_the_travel_warning_along_x_says_what_to_move() {
        let z1 = builtin_z1();
        let warning = SetupWarning {
            kind: SetupWarningKind::OutsideTravel {
                what: "the start of the stock",
                axis: "X",
                value: -263.16,
                min: -207.0,
                max: -1.0,
            },
            unconfirmed: Vec::new(),
        };
        let text =
            crate::overview::setup_warning_in_form("Setup 1", &warning, Some(z1.rotary_origin.x));
        for words in [
            "Travel (setup 'Setup 1'): ",
            "he start of the stock, at X -64.770, is 56.160 mm past the machine's X travel: it \
             would be at machine X -263.160, and the machine reaches -207.000 to -1.000.",
            "Move the stock towards +X (Start X), with the part (Offset along X) and the work \
             zero (Zero at X), or shorten it.",
            "X is measured from the machine's rotary reference point, at machine X -198.390.",
        ] {
            assert!(text.contains(words), "{words}: {text}");
        }
        let at_end = |what: &'static str| SetupWarning {
            kind: SetupWarningKind::OutsideTravel {
                what,
                axis: "X",
                value: 4.0,
                min: -207.0,
                max: -1.0,
            },
            unconfirmed: Vec::new(),
        };
        let text = crate::overview::setup_warning_in_form(
            "S",
            &at_end("the end of the part"),
            Some(-198.39),
        );
        assert!(text.contains("at X 202.390, is 5.000 mm past"), "{text}");
        assert!(
            text.contains("Move the part towards -X (Offset along X)."),
            "{text}"
        );
        let text =
            crate::overview::setup_warning_in_form("S", &at_end("the work zero"), Some(-198.39));
        assert!(
            text.contains("Move the work zero towards -X (Zero at X)."),
            "{text}"
        );
        // Without the reference point, and across the axis, the words are
        // the library's.
        let plain = crate::overview::setup_warning_in_form("S", &warning, None);
        assert!(
            plain.contains("is at machine X -263.160, outside the travel"),
            "{plain}"
        );
    }

    /// A flat end mill with `flute` mm of flute, as the Z1 defaults are.
    fn tool(flute: f64) -> openchuck_types::tool::Tool {
        openchuck_types::tool::Tool {
            shape: openchuck_types::tool::ToolShape::FlatEnd,
            diameter: Millimetres(3.175),
            flute_length: Millimetres(flute),
            overall_length: Millimetres(flute + 20.0),
            shank_diameter: Millimetres(3.175),
            flute_count: 2,
            material: None,
            centre_cutting: None,
        }
    }

    /// The built-in Makera Z1.
    fn builtin_z1() -> MachineProfile {
        builtin(MAKERA_Z1)
            .and_then(Result::ok)
            .unwrap_or_else(MachineProfile::example_x_rotary)
    }

    /// The Makera Z1 as decision 0318 worked its cases: the axis at -48.6
    /// for the probe, the probe sticking out 24.5 mm, so D = 73.1, the safe
    /// Z 40, the recommended minimum 30 and no minimum clearance. The
    /// built-in Z1 has since taken the owner's firmware 1.1.3 measurements
    /// (decision 0649); these cases test the check's rule, not those values.
    fn z1() -> MachineProfile {
        let mut z1 = builtin_z1();
        z1.spindle_to_axis = Some(Millimetres(73.1));
        z1.recommended_min_safe_z = Some(Millimetres(30.0));
        z1.min_clearance_radius = None;
        z1.clearance = openchuck_types::machine::Clearance {
            radius: Millimetres(40.0),
            safe_z: Millimetres(40.0),
        };
        z1
    }

    /// The built-in Z1 on firmware 1.1.3 (decision 0649): D = 72.0 and a
    /// 45 mm minimum, so tool 204 (flute 22) just fits at the default safe
    /// Z 45, and tool 104 (flute 25), the tutorial's O-flute, rises only to
    /// 42.0 and warns that it may be too long to clear the workholding
    /// (SET-06, NFR-12, decision 0495 item 6b).
    #[test]
    fn req_set_06_req_nfr_12_the_builtin_z1_warns_a_tool_too_long_for_its_measured_minimum() {
        let z1 = builtin_z1();
        let minimum = z1.min_clearance_radius.map(|m| m.0);
        assert_eq!(minimum, Some(45.0));
        let safe_z = effective_clearance(
            &z1,
            &openchuck_project::Setup::new(SetupId::from_u128(1), "S"),
            None,
        )
        .clearance
        .safe_z
        .0;
        assert!((safe_z - 45.0).abs() < 1e-9, "{safe_z}");
        assert_eq!(spindle_height(&z1, safe_z, minimum, &tool(22.0)), None);
        let found = spindle_height(&z1, safe_z, minimum, &tool(25.0)).unwrap();
        assert_eq!(found.below_minimum, Some(45.0));
        assert!((found.highest_tip - 42.0).abs() < 1e-9, "{found:?}");
        let text = spindle_height_text(104, &found);
        for words in [
            "too long to clear the workholding within the Z travel",
            "42.000 mm",
            "45.000 mm",
            "The recommended minimum is 45 mm. This must be checked before machining.",
        ] {
            assert!(text.contains(words), "{words}: {text}");
        }
    }

    /// The owner's worked cases at the safe Z 40: tool 104 (flute 25) and 204
    /// (flute 22) fit; the 42 mm flute default does not, fits at 26.1 mm,
    /// under the recommended 30 (SET-06, NFR-12, decision 0318).
    #[test]
    fn req_set_06_req_nfr_12_the_spindle_height_follows_the_owners_worked_cases() {
        let z1 = z1();
        assert_eq!(z1.spindle_to_axis.map(|d| d.0), Some(73.1));
        assert_eq!(spindle_height(&z1, 40.0, None, &tool(25.0)), None);
        assert_eq!(spindle_height(&z1, 40.0, None, &tool(22.0)), None);
        let long = spindle_height(&z1, 40.0, None, &tool(42.0)).unwrap();
        assert!((long.stick_out - 47.0).abs() < 1e-9, "{long:?}");
        assert!((long.highest_tip - 26.1).abs() < 1e-9, "{long:?}");
        assert!((long.past_top - 13.9).abs() < 1e-9, "{long:?}");
        assert_eq!(long.recommended_min, Some(30.0));
        // At the old safe Z 50, tool 104 would have been 6.9 mm past the top.
        let gate5 = spindle_height(&z1, 50.0, None, &tool(25.0)).unwrap();
        assert!((gate5.past_top - 6.9).abs() < 1e-9, "{gate5:?}");
        assert_eq!(gate5.recommended_min, None, "43.1 is above 30");
    }

    /// The boundary: a tool that just reaches the top of the travel at the
    /// safe Z passes; 0.001 mm more warns (decision 0318).
    #[test]
    fn req_set_06_the_spindle_height_is_checked_at_its_boundary() {
        let z1 = z1();
        // D - S = safe Z: 73.1 - (28.1 + 5) = 40.
        assert_eq!(spindle_height(&z1, 40.0, None, &tool(28.1)), None);
        let over = spindle_height(&z1, 40.0, None, &tool(28.101)).unwrap();
        assert!((over.past_top - 0.001).abs() < 1e-9, "{over:?}");
    }

    /// No warning without the distance or without tool length measurement:
    /// the check cannot be made, and is not guessed (decision 0318).
    #[test]
    fn req_set_06_req_nfr_12_no_distance_or_no_measurement_gives_no_spindle_height_warning() {
        let mut unknown = z1();
        unknown.spindle_to_axis = None;
        assert_eq!(spindle_height(&unknown, 40.0, None, &tool(42.0)), None);
        let mut manual = z1();
        manual.automatic_tool_length = false;
        assert_eq!(spindle_height(&manual, 40.0, None, &tool(42.0)), None);
        assert_eq!(
            spindle_height(&MachineProfile::example_x_rotary(), 40.0, None, &tool(42.0)),
            None
        );
    }

    /// The text names the tool, its assumed stick-out, how far past the top
    /// the spindle goes and the safe Z that fits, says to check it on the
    /// machine, and adds the owner's recommended minimum when the fit is
    /// under it (NFR-12, decision 0318).
    #[test]
    fn req_nfr_12_the_spindle_height_warning_says_what_to_check() {
        let z1 = z1();
        let long = spindle_height(&z1, 40.0, None, &tool(42.0)).unwrap();
        let text = spindle_height_text(7, &long);
        for words in [
            "tool 7 may",
            "47.000 mm",
            "flute length + 5 mm",
            "13.900 mm",
            "26.100 mm",
            "on your machine",
            "The recommended minimum is 30 mm. This must be checked before machining.",
        ] {
            assert!(text.contains(words), "{words}: {text}");
        }
        let gate5 = spindle_height(&z1, 50.0, None, &tool(25.0)).unwrap();
        let text = spindle_height_text(1, &gate5);
        assert!(!text.contains("recommended minimum"), "{text}");
    }

    #[test]
    fn req_set_01_a_direction_follows_the_sense() {
        let mut profile = MachineProfile::example_x_rotary();
        assert!(a_direction(&profile).ends_with("faces -Y at A 90"));
        profile.a_sense = ASense::Negative;
        assert!(a_direction(&profile).ends_with("faces +Y at A 90"));
    }

    /// Case a of decision 0495: the check takes the setup's effective safe
    /// Z, so a tool that fits at the base safe Z warns once Clearance from
    /// stock lifts it (SET-06, NFR-12).
    #[test]
    fn req_set_06_req_nfr_12_the_spindle_height_uses_the_safe_z_grown_from_stock() {
        let z1 = z1();
        let mut setup = openchuck_project::Setup::new(SetupId::from_u128(1), "S");
        setup.stock = Some(Stock::Cylinder {
            diameter: Millimetres(100.0),
            length: Millimetres(50.0),
            x_start: Millimetres(0.0),
        });
        let base = effective_clearance(&z1, &setup, None);
        assert_eq!(base.clearance, z1.clearance);
        assert_eq!(
            spindle_height(&z1, base.clearance.safe_z.0, None, &tool(25.0)),
            None
        );
        setup.clearance_rules.from_stock = Some(DEFAULT_STOCK_MARGIN);
        let grown = effective_clearance(&z1, &setup, None);
        assert_eq!(grown.clearance.safe_z, Millimetres(55.0));
        let found = spindle_height(&z1, grown.clearance.safe_z.0, None, &tool(25.0)).unwrap();
        // 55 - 43.1
        assert!((found.past_top - 11.9).abs() < 1e-9, "{found:?}");
        assert_eq!(found.below_minimum, None);
    }

    /// Case b of decision 0495: a tool whose highest tip is inside the
    /// minimum clearance radius cannot clear both the workholding and the
    /// top of the travel; the warning says so and does not advise lowering
    /// the safe Z below the minimum (NFR-12).
    #[test]
    fn req_set_06_req_nfr_12_a_tool_too_long_for_the_minimum_clearance_warns() {
        let z1 = z1();
        // Tool 104 reaches 43.1: clear of a 40 mm minimum, inside a 45 mm one.
        assert_eq!(spindle_height(&z1, 40.0, Some(40.0), &tool(25.0)), None);
        let found = spindle_height(&z1, 40.0, Some(45.0), &tool(25.0)).unwrap();
        assert_eq!(found.below_minimum, Some(45.0));
        assert!(found.past_top < 0.0, "{found:?}");
        let text = spindle_height_text(1, &found);
        for words in [
            "tool 1 may",
            "too long to clear the workholding within the Z travel",
            "30.000 mm (flute length + 5 mm)",
            "43.100 mm",
            "45.000 mm",
            "Use a shorter tool",
            "on your machine before running",
        ] {
            assert!(text.contains(words), "{words}: {text}");
        }
        assert!(!text.contains("Lower the setup's safe Z"), "{text}");
        // Within rounding of the minimum, it fits.
        assert_eq!(spindle_height(&z1, 40.0, Some(43.1004), &tool(25.0)), None);
        // The recommended-minimum line still follows decision 0318.
        let long = spindle_height(&z1, 40.0, Some(45.0), &tool(42.0)).unwrap();
        assert!(spindle_height_text(7, &long).contains("The recommended minimum is 30 mm."),);
        // Without measurement there is still no guess (decision 0318).
        let mut manual = z1.clone();
        manual.automatic_tool_length = false;
        assert_eq!(spindle_height(&manual, 40.0, Some(80.0), &tool(42.0)), None);
    }
}

/// A machine profile the user can choose from, built in or in a library
/// folder (SET-01).
#[derive(Debug, Clone, PartialEq)]
pub struct AvailableMachine {
    /// Where it comes from: `built-in makera-z1` or the file's path.
    pub source: String,
    /// Its ID, which project copies keep (OI-128); `None` if it has none.
    pub id: Option<String>,
    /// The profile, or why its file cannot be read.
    pub profile: Result<MachineProfile, String>,
}

/// The profiles in the library folder `library` (if it exists), in file
/// name order, then the built-in ones.
pub fn available_machines(library: Option<&std::path::Path>) -> Vec<AvailableMachine> {
    let mut out = Vec::new();
    let mut push = |source: String, entry: Result<ProfileEntry, String>| {
        let (id, profile) = match entry {
            Ok(e) => (e.id, Ok(e.profile)),
            Err(e) => (None, Err(e)),
        };
        out.push(AvailableMachine {
            source,
            id,
            profile,
        });
    };
    if let Some(dir) = library.filter(|d| d.is_dir())
        && let Ok(listing) = list_library_entries(dir)
    {
        for (path, entry) in listing {
            push(path.display().to_string(), entry.map_err(|e| e.to_string()));
        }
    }
    for key in builtin_keys() {
        if let Some(entry) = builtin_entry(key) {
            push(format!("built-in {key}"), entry.map_err(|e| e.to_string()));
        }
    }
    out
}

/// Where a project's machine copy stands against the profile it came from
/// (OI-54; decision record 0029 item 16).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum MachineSource {
    /// The project has no machine.
    NoMachine,
    /// The copy records no source: it was set before profiles had IDs, or
    /// from a profile that never had one.
    Unrecorded,
    /// No available profile has the copy's ID.
    NotFound {
        /// The copy's ID.
        id: String,
    },
    /// The copy matches its source.
    Same {
        /// Where the source is.
        source: String,
    },
    /// The source has changed since the copy was taken, or the copy was
    /// edited. The project's copy always wins until the user updates it.
    Differs {
        /// Where the source is.
        source: String,
        /// The source's ID, to keep with the updated copy.
        id: String,
        /// The source profile, to update the copy from.
        profile: Box<MachineProfile>,
        /// The fields that differ, in the user's words.
        fields: Vec<&'static str>,
    },
}

/// Compares the project's machine copy (`copy`, recorded as coming from the
/// profile with `copy_id`) with the available profiles. The first profile
/// with that ID is its source; library profiles come before built-in ones,
/// so a library profile the user derived from a built-in one, keeping its
/// ID, is the one compared.
pub fn compare_machine(
    copy: Option<&MachineProfile>,
    copy_id: Option<&str>,
    available: &[AvailableMachine],
) -> MachineSource {
    let Some(copy) = copy else {
        return MachineSource::NoMachine;
    };
    let Some(id) = copy_id else {
        return MachineSource::Unrecorded;
    };
    let found = available.iter().find_map(|a| {
        a.profile
            .as_ref()
            .ok()
            .filter(|_| a.id.as_deref() == Some(id))
            .map(|p| (a.source.clone(), p))
    });
    match found {
        None => MachineSource::NotFound { id: id.to_owned() },
        Some((source, profile)) => {
            let fields = differing_fields(copy, profile);
            if fields.is_empty() {
                MachineSource::Same { source }
            } else {
                MachineSource::Differs {
                    source,
                    id: id.to_owned(),
                    profile: Box::new(profile.clone()),
                    fields,
                }
            }
        }
    }
}

/// The user's words for a machine profile value (specification section 15),
/// never its name in the profile file.
pub fn profile_field_label(field: ProfileField) -> &'static str {
    match field {
        ProfileField::RotaryAxis => "rotary axis",
        ProfileField::ASense => "A+ direction",
        ProfileField::RotaryOrigin => "axis height",
        ProfileField::ARange => "A range",
        ProfileField::Travel => "travel",
        ProfileField::MaxFeeds => "maximum feeds",
        ProfileField::Clearance => "clearance",
        ProfileField::SpindleRange => "spindle speed range",
        ProfileField::MinBlockMs => "minimum block time",
        ProfileField::BlockWindow => "block window",
        ProfileField::ShortBlockMode => "short-block mode",
        ProfileField::ToolLengthMeasurement => "tool length measurement",
        ProfileField::SpindleToAxis => "spindle-to-axis distance",
        ProfileField::RecommendedMinSafeZ => "recommended minimum safe Z",
        ProfileField::MinClearanceRadius => "minimum clearance radius",
    }
}

/// The fields of two profiles that differ, in the user's words. The
/// destructuring is exhaustive, so a field added to the profile fails to
/// build here until it is compared.
pub(crate) fn differing_fields(a: &MachineProfile, b: &MachineProfile) -> Vec<&'static str> {
    let MachineProfile {
        name,
        rotary_axis,
        a_sense,
        rotary_origin,
        a_range,
        travel,
        max_feeds,
        clearance,
        max_tool_number,
        min_spindle_rpm,
        max_spindle_rpm,
        min_block_ms,
        block_window,
        short_block_mode,
        automatic_tool_length,
        spindle_to_axis,
        recommended_min_safe_z,
        min_clearance_radius,
        provenance,
    } = a;
    let checks = [
        ("name", *name == b.name),
        ("rotary axis", *rotary_axis == b.rotary_axis),
        ("A+ direction", *a_sense == b.a_sense),
        ("axis height", *rotary_origin == b.rotary_origin),
        ("A range", *a_range == b.a_range),
        ("travel", *travel == b.travel),
        ("maximum feeds", *max_feeds == b.max_feeds),
        ("clearance", *clearance == b.clearance),
        ("highest tool number", *max_tool_number == b.max_tool_number),
        (
            "spindle speed range",
            *min_spindle_rpm == b.min_spindle_rpm && *max_spindle_rpm == b.max_spindle_rpm,
        ),
        (
            "minimum block time",
            min_block_ms.to_bits() == b.min_block_ms.to_bits(),
        ),
        ("block window", *block_window == b.block_window),
        ("short-block mode", *short_block_mode == b.short_block_mode),
        (
            "tool length measurement",
            *automatic_tool_length == b.automatic_tool_length,
        ),
        (
            "spindle-to-axis distance",
            *spindle_to_axis == b.spindle_to_axis,
        ),
        (
            "recommended minimum safe Z",
            *recommended_min_safe_z == b.recommended_min_safe_z,
        ),
        (
            "minimum clearance radius",
            *min_clearance_radius == b.min_clearance_radius,
        ),
        ("which values are confirmed", *provenance == b.provenance),
    ];
    checks
        .into_iter()
        .filter(|(_, same)| !same)
        .map(|(name, _)| name)
        .collect()
}

#[cfg(test)]
mod short_block_tests {
    use super::*;

    #[test]
    fn req_gc_17_req_set_01_profile_comparison_names_short_move_settings() {
        let source = MachineProfile::example_x_rotary();
        let mut copy = source.clone();
        copy.min_block_ms = 7.0;
        assert_eq!(differing_fields(&source, &copy), ["minimum block time"]);
        copy.block_window = 32;
        copy.short_block_mode = ShortBlockMode::Report;
        assert_eq!(
            differing_fields(&source, &copy),
            ["minimum block time", "block window", "short-block mode"]
        );
        assert_eq!(source.min_block_ms.to_bits(), 0.0f64.to_bits());
    }
}
