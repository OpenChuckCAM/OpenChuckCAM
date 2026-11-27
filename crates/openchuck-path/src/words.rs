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

/// Marks the start of a position in a message not yet settled.
const OPEN: char = '\u{E000}';
/// Marks its end.
const CLOSE: char = '\u{E001}';

/// A setup-frame X as a message writes it, "X 21.203" (decision record
/// 0699): the number is converted to the work frame when the notice is
/// settled ([`settle`]), because the code that writes a message does not
/// know the setup's work zero. A message that prints a position uses this,
/// [`n`] or [`x_span`] and never `X {:.3}`, which `cargo xtask messages`
/// counts.
pub(crate) fn x(setup_x: f64) -> String {
    format!("{OPEN}x{setup_x:?}{CLOSE}")
}

/// A setup-frame X without the letter, "21.203", for the second end of
/// "from X 3.000 to 5.000" and the later items of a list.
pub(crate) fn n(setup_x: f64) -> String {
    format!("{OPEN}n{setup_x:?}{CLOSE}")
}

/// An X span as a message writes it: "X 10.000 to 12.500", and "X 10.000"
/// when both ends match to three decimals (a line across the part, not a
/// range).
pub(crate) fn x_span(low: f64, high: f64) -> String {
    format!("{OPEN}s{low:?},{high:?}{CLOSE}")
}

/// Positions as a list: "X 1.000", "X 1.000 and 2.500", "X 1.000, 2.500 and
/// 4.000" (the letter once).
pub(crate) fn x_list(positions: &[f64]) -> String {
    match positions {
        [] => String::new(),
        [only] => x(*only),
        [first, middle @ .., last] => {
            let mut text = x(*first);
            for m in middle {
                text.push_str(", ");
                text.push_str(&n(*m));
            }
            text.push_str(" and ");
            text.push_str(&n(*last));
            text
        }
    }
}

/// Writes every position in `text` as the setup's work frame reads it
/// (decision record 0699), converting the numbers marked by [`x`], [`n`] and
/// [`x_span`]. `generate` settles every notice and refusal it returns, so a
/// host reads settled text only; a marker that does not parse is dropped, so
/// no private character reaches a user (NFR-07).
pub fn settle(text: &str, frame: openchuck_types::position::WorkFrame) -> String {
    if !text.contains(OPEN) {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(OPEN) {
        out.push_str(&rest[..start]);
        let after = &rest[start + OPEN.len_utf8()..];
        let Some(end) = after.find(CLOSE) else {
            rest = "";
            break;
        };
        out.push_str(&settled_marker(&after[..end], frame));
        rest = &after[end + CLOSE.len_utf8()..];
    }
    out.push_str(rest);
    out
}

/// One marker's content (its kind and value) written in `frame`; empty when
/// it does not parse.
fn settled_marker(content: &str, frame: openchuck_types::position::WorkFrame) -> String {
    let mut chars = content.chars();
    let (Some(kind), values) = (chars.next(), chars.as_str()) else {
        return String::new();
    };
    let number = |v: &str| v.parse::<f64>().ok();
    match kind {
        'x' => number(values).map(|v| frame.x(v)),
        'n' => number(values).map(|v| frame.number(v)),
        's' => values
            .split_once(',')
            .and_then(|(a, b)| Some((number(a)?, number(b)?)))
            .map(|(a, b)| frame.x_span(a, b)),
        _ => None,
    }
    .unwrap_or_default()
}

/// `text` as a host reads it when the work zero is 0: the setup frame's own
/// numbers.
#[cfg(test)]
pub(crate) fn plain(text: &str) -> String {
    settle(text, openchuck_types::position::WorkFrame::default())
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
        assert_eq!(plain(&x_span(10.0, 10.0)), "X 10.000");
        assert_eq!(plain(&x_span(10.0, 10.0004)), "X 10.000");
        assert_eq!(plain(&x_span(10.0, 12.5)), "X 10.000 to 12.500");
        assert_eq!(plain(&x_span(-1.0, 0.0)), "X -1.000 to 0.000");
        // A value that rounds to zero is zero, whatever its sign.
        assert_eq!(plain(&x_span(-0.0001, 0.0001)), "X 0.000");
        assert_eq!(plain(&x_span(-0.0001, 2.0)), "X 0.000 to 2.000");
    }

    /// Decision record 0699: a position is written in the work frame when the
    /// text is settled, whatever the code that wrote it knew.
    #[test]
    fn req_ui_12_a_position_is_settled_in_the_work_frame() {
        use openchuck_types::position::WorkFrame;
        let frame = WorkFrame::new(17.99);
        let text = format!(
            "the stock at {} and the end face at {} to {}, and {} are left",
            x(39.193),
            x(27.99),
            n(37.99),
            x_span(20.0, 20.0004)
        );
        assert_eq!(
            settle(&text, frame),
            "the stock at X 21.203 and the end face at X 10.000 to 20.000, and X 2.010 are left"
        );
        // With a work zero of 0 the same text reads as the code computed it.
        assert_eq!(
            plain(&text),
            "the stock at X 39.193 and the end face at X 27.990 to 37.990, and X 20.000 are left"
        );
        // A text with no position is returned as it is.
        assert_eq!(settle("nothing to move", frame), "nothing to move");
    }

    #[test]
    fn req_ui_12_a_list_of_positions_writes_the_letter_once() {
        assert_eq!(plain(&x_list(&[])), "");
        assert_eq!(plain(&x_list(&[1.0])), "X 1.000");
        assert_eq!(plain(&x_list(&[1.0, 2.5])), "X 1.000 and 2.500");
        assert_eq!(plain(&x_list(&[1.0, 2.5, 4.0])), "X 1.000, 2.500 and 4.000");
        let frame = openchuck_types::position::WorkFrame::new(1.0);
        assert_eq!(
            settle(&x_list(&[1.0, 2.5, 4.0]), frame),
            "X 0.000, 1.500 and 3.000"
        );
    }

    /// NFR-07: a marker that does not parse is dropped, never shown.
    #[test]
    fn req_nfr_07_a_broken_marker_never_reaches_a_user() {
        let broken = format!("at {OPEN}x12.5 and {OPEN}qz{CLOSE} then {OPEN}xnot-a-number{CLOSE}.");
        let settled = plain(&broken);
        assert!(
            !settled.contains(OPEN) && !settled.contains(CLOSE),
            "{settled}"
        );
        assert_eq!(plain(&format!("end {OPEN}x1.5")), "end ");
    }
}
