// SPDX-License-Identifier: AGPL-3.0-or-later
//! Positions as a message writes them (decision record 0699; requirements
//! UI-12, NFR-07 and NFR-12).
//!
//! Every position a message gives along the rotary axis is in the **work
//! frame**: measured from the setup's work zero (its **Zero at X**), the
//! origin the viewport marks and the G-code and the simulation's readout
//! use. The toolpaths and the setup's stock, held regions and tabs are given
//! in the **setup frame**, measured from the machine's rotary reference
//! point (decision record 0201). The two differ by the work zero, so a
//! message converts a setup-frame number once, here, when it writes it.
//!
//! A position in another frame is written with one of the [`label`]s, and
//! only those; a length, a distance, a delta and an angle have no frame and
//! take no label.

use crate::setup::SetupFrame;

/// The labels a position in another frame carries (decision record 0699,
/// question 3). `cargo xtask messages` refuses a message that writes a
/// position and uses neither [`WorkFrame`] nor one of these.
pub mod label {
    /// A position in the machine's own coordinates: a travel limit, the
    /// register's value, **Rotary reference X**.
    pub const MACHINE_X: &str = "machine X";
    /// **Zero at X** itself: it defines the work frame, so it is measured
    /// from the origin the work frame is moved from.
    pub const FROM_ROTARY_REFERENCE_POINT: &str = "from the rotary reference point";
    /// A position on the model, before any setup: an import's extent, the
    /// placement, a face's place, and the cursor with no setup chosen.
    pub const MODEL_X: &str = "model X";

    /// Every label, in the order above.
    pub const ALL: [&str; 3] = [MACHINE_X, FROM_ROTARY_REFERENCE_POINT, MODEL_X];
}

/// How a setup's positions along the rotary axis are written in a message
/// (decision record 0699): the work zero, as a setup-frame X.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WorkFrame {
    zero_x: f64,
}

impl WorkFrame {
    /// The work frame whose zero is at setup-frame X `zero_x`. A zero that
    /// is not finite is taken as 0, so a bad value never writes "NaN" into a
    /// message (NFR-07).
    #[must_use]
    pub fn new(zero_x: f64) -> Self {
        Self {
            zero_x: if zero_x.is_finite() { zero_x } else { 0.0 },
        }
    }

    /// The work frame of `setup`.
    #[must_use]
    pub fn of_setup(setup: &SetupFrame) -> Self {
        Self::new(setup.work_offset.x.0)
    }

    /// The work zero, as a setup-frame X.
    #[must_use]
    pub fn zero_x(self) -> f64 {
        self.zero_x
    }

    /// The work-frame X of the setup-frame X `setup_x`.
    #[must_use]
    pub fn to_work(self, setup_x: f64) -> f64 {
        setup_x - self.zero_x
    }

    /// The setup-frame X of the work-frame X `work_x`.
    #[must_use]
    pub fn to_setup(self, work_x: f64) -> f64 {
        work_x + self.zero_x
    }

    /// The number of the setup-frame X `setup_x` as a message writes it
    /// in the work frame: "21.203", three decimals, a value that rounds to
    /// zero written "0.000" and never "-0.000".
    #[must_use]
    pub fn number(self, setup_x: f64) -> String {
        written(self.to_work(setup_x))
    }

    /// A position: "X 21.203".
    #[must_use]
    pub fn x(self, setup_x: f64) -> String {
        format!("X {}", self.number(setup_x))
    }

    /// A span: "X 10.000 to 12.500", and "X 10.000" when both ends are the
    /// same to three decimals (a line across the part, not a range).
    #[must_use]
    pub fn x_span(self, setup_low: f64, setup_high: f64) -> String {
        let (low, high) = (self.number(setup_low), self.number(setup_high));
        if low == high {
            format!("X {low}")
        } else {
            format!("X {low} to {high}")
        }
    }
}

/// A position in the machine's own coordinates, labelled: "machine X
/// -263.160".
#[must_use]
pub fn machine_x(x: f64) -> String {
    format!("{} {}", label::MACHINE_X, written(x))
}

/// A position on the model, labelled: "model X 12.500".
#[must_use]
pub fn model_x(x: f64) -> String {
    format!("{} {}", label::MODEL_X, written(x))
}

/// A setup-frame X on both sides, for a message about the machine's travel
/// (decision record 0699, question 3): "X -82.760 (machine X -100.150)". The
/// machine X is the machine's **Rotary reference X** (where the setup's X 0
/// is on the machine) plus the setup-frame X.
#[must_use]
pub fn x_and_machine_x(frame: WorkFrame, setup_x: f64, rotary_reference_x: f64) -> String {
    format!(
        "{} ({})",
        frame.x(setup_x),
        machine_x(rotary_reference_x + setup_x)
    )
}

/// Three decimals; a value that rounds to zero is "0.000", never "-0.000".
fn written(x: f64) -> String {
    let text = format!("{x:.3}");
    if text == "-0.000" {
        "0.000".to_owned()
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Decision record 0699: the same point, written in the work frame, is
    /// the setup-frame number less Zero at X (the flute-length example's
    /// warning "at X 39.193" with Zero at X 17.99 is X 21.203).
    #[test]
    fn req_ui_12_a_position_is_written_from_the_work_zero() {
        let frame = WorkFrame::new(17.99);
        assert_eq!(frame.x(39.193), "X 21.203");
        assert_eq!(frame.number(39.193), "21.203");
        assert_eq!(frame.x(17.99), "X 0.000");
        assert_eq!(frame.x(0.0), "X -17.990");
        assert_eq!(frame.x_span(27.99, 37.99), "X 10.000 to 20.000");
        assert!((frame.to_work(39.193) - 21.203).abs() < 1e-9);
        assert!((frame.to_setup(21.203) - 39.193).abs() < 1e-9);
    }

    /// Decision record 0699: with Zero at X 0 every message reads as it did
    /// before, so a project that never moved its zero changes nothing.
    #[test]
    fn req_ui_12_a_zero_at_zero_changes_nothing() {
        let frame = WorkFrame::default();
        assert_eq!(frame, WorkFrame::new(0.0));
        assert_eq!(frame.x(39.193), "X 39.193");
        assert_eq!(frame.x_span(10.0, 12.5), "X 10.000 to 12.500");
        assert_eq!(frame.x(-1.0), "X -1.000");
    }

    /// A span that is one place is not a range, and a value that rounds to
    /// zero is zero whatever its sign (issue #612).
    #[test]
    fn req_ui_12_a_span_that_is_one_place_is_not_a_range() {
        let frame = WorkFrame::new(5.0);
        assert_eq!(frame.x_span(15.0, 15.0), "X 10.000");
        assert_eq!(frame.x_span(15.0, 15.0004), "X 10.000");
        assert_eq!(frame.x_span(4.9999, 5.0001), "X 0.000");
        assert_eq!(frame.x_span(4.9999, 7.0), "X 0.000 to 2.000");
        assert_eq!(frame.x(5.0 - 0.0001), "X 0.000");
    }

    /// NFR-07: a zero that is not a number is taken as 0, never written.
    #[test]
    #[allow(clippy::float_cmp)] // A non-finite zero is replaced by exactly 0.
    fn req_nfr_07_a_zero_that_is_not_finite_is_taken_as_zero() {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let frame = WorkFrame::new(bad);
            assert_eq!(frame.zero_x(), 0.0);
            assert_eq!(frame.x(1.5), "X 1.500");
        }
    }

    /// Decision record 0699, question 3: the labels are exactly these three,
    /// and a labelled position is written beside its label.
    #[test]
    fn req_ui_12_the_labels_are_exactly_the_three_the_owner_accepted() {
        assert_eq!(
            label::ALL,
            ["machine X", "from the rotary reference point", "model X"]
        );
        assert_eq!(machine_x(-263.16), "machine X -263.160");
        assert_eq!(model_x(12.5), "model X 12.500");
        assert_eq!(machine_x(-0.0001), "machine X 0.000");
    }

    /// Decision record 0699, question 3 (ii): a travel message gives the
    /// work-frame position and the machine position, each labelled.
    #[test]
    fn req_ui_12_a_travel_position_is_written_in_both_frames() {
        // Zero at X 181.0 and a rotary reference X of -198.39: setup X 98.24
        // is X -82.760 from the zero and machine X -100.150.
        let frame = WorkFrame::new(181.0);
        assert_eq!(
            x_and_machine_x(frame, 98.24, -198.39),
            "X -82.760 (machine X -100.150)"
        );
    }

    #[test]
    fn req_ui_12_the_work_frame_is_the_setups_zero_at_x() {
        let mut setup = SetupFrame::default();
        setup.work_offset.x = crate::units::Millimetres(18.0);
        assert_eq!(WorkFrame::of_setup(&setup), WorkFrame::new(18.0));
        assert_eq!(WorkFrame::of_setup(&setup).x(79.0), "X 61.000");
    }
}
