// SPDX-License-Identifier: AGPL-3.0-or-later
//! The words of a user-visible text that depend on a number (decision
//! record 0644, after OWNERCHECKS2 read "in 1 layers of 1 mm").
//!
//! Every text that prints a count that can be one writes it with
//! [`counted`], so one layer reads "1 layer" and three read "3 layers";
//! `cargo xtask messages` counts the texts that put a placeholder straight
//! before a plural noun instead.

/// `n` and the noun that agrees with it: `one` when `n` is 1, `many`
/// otherwise ("0 layers", "1 layer", "3 layers").
#[must_use]
pub fn counted(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// An X span as a message writes it: "X 10.000 to 12.500", and "X 10.000"
/// when both ends are the same to three decimals (a line across the part,
/// not a range).
pub(crate) fn x_span(low: f64, high: f64) -> String {
    if format!("{low:.3}") == format!("{high:.3}") {
        format!("X {low:.3}")
    } else {
        format!("X {low:.3} to {high:.3}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn req_ui_12_a_count_of_one_takes_the_singular() {
        assert_eq!(counted(1, "layer", "layers"), "1 layer");
        assert_eq!(counted(0, "layer", "layers"), "0 layers");
        assert_eq!(counted(2, "layer", "layers"), "2 layers");
        assert_eq!(counted(1, "pass", "passes"), "1 pass");
        assert_eq!(counted(3, "entry", "entries"), "3 entries");
    }

    #[test]
    fn req_ui_12_a_span_that_is_one_place_is_not_written_as_a_range() {
        assert_eq!(x_span(10.0, 10.0), "X 10.000");
        assert_eq!(x_span(10.0, 10.0004), "X 10.000");
        assert_eq!(x_span(10.0, 12.5), "X 10.000 to 12.500");
        assert_eq!(x_span(-1.0, 0.0), "X -1.000 to 0.000");
    }
}
