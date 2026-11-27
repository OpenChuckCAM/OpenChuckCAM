// SPDX-License-Identifier: AGPL-3.0-or-later
//! Lengths and feeds shown and typed in inches (UI-18; decision PROJUX-B-2,
//! the owner's answers Q2a C and Q2b A, relayed by the coordinator).
//!
//! Values are stored in millimetres (NFR-05): the project, the libraries,
//! the setup sheet's own numbers and the G-code never see an inch. Inches
//! are a per-user choice of how a number box shows and reads its value. A
//! number box gives its caller what it always gave, the entry in the field's
//! own unit (millimetres, millimetres per minute), so every caller works
//! unchanged.
//!
//! A unit may be typed after a value, or after a sum ("0.005 in", "1/8 in",
//! "3 + 0.5 mm"): a length box reads `mm`, `cm`, `in` and `"`, a feed box
//! `mm/min` and `in/min`. Any other word is refused with a message that names
//! the units the box reads, and the last good value stays (NFR-07).
//!
//! A box whose text is not edited never stores again: Enter on the shown
//! inches gives back the stored millimetres exactly, so showing a value in
//! inches, rounded, can never change it.

use eframe::egui;

use crate::arith;
use crate::field_row::{number, places};

/// Millimetres in an inch, exactly.
pub const MM_PER_INCH: f64 = 25.4;

/// How lengths and feeds are shown and read: the user's choice (UI-18).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Display {
    /// Millimetres and millimetres per minute, as stored.
    #[default]
    Millimetres,
    /// Inches and inches per minute.
    Inches,
}

impl Display {
    /// The setting's text in the user's settings file ("mm" or "in").
    pub const fn setting(self) -> &'static str {
        match self {
            Self::Millimetres => "mm",
            Self::Inches => "in",
        }
    }

    /// The choice a settings file holds; anything else reads as millimetres.
    pub fn from_setting(text: &str) -> Self {
        if text.trim().eq_ignore_ascii_case("in") {
            Self::Inches
        } else {
            Self::Millimetres
        }
    }
}

/// What kind of quantity a box holds, from its stored unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Quantity {
    Length,
    Feed,
}

fn quantity(stored_unit: &str) -> Option<Quantity> {
    match stored_unit {
        "mm" => Some(Quantity::Length),
        "mm/min" => Some(Quantity::Feed),
        _ => None,
    }
}

/// The unit a box shows for its stored unit `stored_unit`.
pub fn shown_unit(stored_unit: &str, display: Display) -> &str {
    match (quantity(stored_unit), display) {
        (Some(Quantity::Length), Display::Inches) => "in",
        (Some(Quantity::Feed), Display::Inches) => "in/min",
        _ => stored_unit,
    }
}

/// The decimal places a shown unit gets: inches to four (a ten-thousandth,
/// 0.00254 mm), inches per minute to one; the rest as
/// [`crate::field_row::places`] says.
pub fn shown_places(shown_unit: &str) -> Option<usize> {
    match shown_unit {
        "in" => Some(4),
        "in/min" => Some(1),
        other => places(other),
    }
}

/// The text a box shows for the stored text `stored` (a number in
/// `stored_unit`, or anything else, which is shown as it is).
pub fn shown_text(stored: &str, stored_unit: &str, display: Display) -> String {
    let unit = shown_unit(stored_unit, display);
    if unit == stored_unit {
        return stored.to_owned();
    }
    match stored.trim().parse::<f64>() {
        Ok(value) if value.is_finite() => number(
            arith::round_to(value / MM_PER_INCH, shown_places(unit)),
            shown_places(unit),
        ),
        _ => stored.to_owned(),
    }
}

/// A hint ("0.020 · from the tool's cutting data") with its leading number
/// shown in the user's unit; a hint that does not start with a number is
/// shown as it is.
pub fn shown_hint(hint: &str, stored_unit: &str, display: Display) -> String {
    let (head, tail) = hint
        .find(char::is_whitespace)
        .map_or((hint, ""), |at| hint.split_at(at));
    if head.parse::<f64>().is_err() {
        return hint.to_owned();
    }
    format!("{}{tail}", shown_text(head, stored_unit, display))
}

/// What was typed into a number box, read.
#[derive(Debug, Clone, PartialEq)]
pub struct Read {
    /// The value in the box's stored unit, as the text to apply.
    pub stored: String,
    /// The unit typed with the value, when one was: the box then says what
    /// was stored ("Stored as 0.127 mm").
    pub typed_unit: Option<&'static str>,
}

/// The units a length box reads, with millimetres per one.
const LENGTH_UNITS: [(&str, &str, f64); 4] = [
    ("mm", "mm", 1.0),
    ("cm", "cm", 10.0),
    ("in", "in", MM_PER_INCH),
    ("\"", "in", MM_PER_INCH),
];

/// The units a feed box reads, with millimetres per minute per one.
const FEED_UNITS: [(&str, &str, f64); 2] =
    [("mm/min", "mm/min", 1.0), ("in/min", "in/min", MM_PER_INCH)];

/// Splits a unit typed at the end of `typed` from the sum before it: the
/// sum, and the unit's text as typed. No unit gives the whole entry.
fn split_unit(typed: &str) -> (&str, Option<&str>) {
    let text = typed.trim_end();
    if let Some(sum) = text.strip_suffix('"') {
        return (sum, Some("\""));
    }
    let start = text
        .char_indices()
        .rev()
        .take_while(|(_, c)| c.is_ascii_alphabetic() || *c == '/')
        .last()
        .map(|(at, _)| at);
    match start {
        Some(at) if at > 0 => (&text[..at], Some(&text[at..])),
        // The whole entry is letters: no number before it.
        Some(_) => ("", Some(text)),
        None => (text, None),
    }
}

/// Reads an entry typed in a number box whose value is stored in
/// `stored_unit`, shown in `display`'s unit: a number or a sum, with or
/// without a unit at its end. A plain number in the stored unit is given
/// back as typed, so what is stored never changes for it (as before).
///
/// # Errors
/// A plain message for an entry that cannot be read: the sum's own message,
/// or one naming the units the box reads.
pub fn read(typed: &str, stored_unit: &str, display: Display) -> Result<Read, String> {
    let typed = typed.trim();
    let Some(quantity) = quantity(stored_unit) else {
        // Not a length or a feed: no unit is read (UI-26 as before).
        return arith::entry_text(typed, places(stored_unit)).map(|stored| Read {
            stored,
            typed_unit: None,
        });
    };
    let (units, words): (&[(&str, &str, f64)], &str) = match quantity {
        Quantity::Length => (&LENGTH_UNITS, "mm, cm, in or \""),
        Quantity::Feed => (&FEED_UNITS, "mm/min or in/min"),
    };
    let (sum, unit) = split_unit(typed);
    let (factor, typed_unit) = match unit {
        Some(word) => {
            let lower = word.to_ascii_lowercase();
            let Some((_, name, factor)) = units.iter().find(|(text, _, _)| *text == lower) else {
                return Err(format!(
                    "Cannot read '{typed}': this box reads {words} after the value."
                ));
            };
            (*factor, Some(*name))
        }
        None => match display {
            Display::Millimetres => {
                return arith::entry_text(typed, places(stored_unit)).map(|stored| Read {
                    stored,
                    typed_unit: None,
                });
            }
            Display::Inches => (MM_PER_INCH, None),
        },
    };
    let sum = sum.trim();
    if sum.is_empty() {
        return Err(format!(
            "Cannot read '{typed}'; expected a number or a sum before the unit."
        ));
    }
    let value = arith::evaluate(sum).map_err(|e| e.to_string())?;
    if factor == 1.0 {
        // Typed in the stored unit: as a plain entry of that unit.
        let stored = arith::entry_text(sum, places(stored_unit))?;
        return Ok(Read { stored, typed_unit });
    }
    // Converted exactly, then kept to a billionth to drop float noise
    // (0.1 in is 2.54 mm, not 2.5400000000000005).
    let stored = number(
        arith::round_to(value * factor, Some(9)),
        places(stored_unit),
    );
    Ok(Read { stored, typed_unit })
}

/// The line under a box after an entry typed with a unit: what was stored,
/// and how it reads in the box's own unit when that is another.
pub fn stored_note(stored: &str, stored_unit: &str, display: Display) -> String {
    let shown = shown_unit(stored_unit, display);
    if shown == stored_unit {
        format!("Stored as {stored} {stored_unit}")
    } else {
        format!(
            "Stored as {stored} {stored_unit} ({} {shown})",
            shown_text(stored, stored_unit, display)
        )
    }
}

/// A drag box for a length stored in millimetres in `value`, shown and
/// dragged in `display`'s unit with `suffix` after it (" mm" or " in") when
/// asked for. What it stores is millimetres either way (UI-18).
pub fn length_drag<'a>(
    value: &'a mut f64,
    display: Display,
    speed_mm: f64,
    suffix: bool,
) -> egui::DragValue<'a> {
    let factor = match display {
        Display::Millimetres => 1.0,
        Display::Inches => MM_PER_INCH,
    };
    let unit = shown_unit("mm", display);
    let drag = egui::DragValue::from_get_set(move |new| {
        if let Some(new) = new {
            *value = new * factor;
        }
        *value / factor
    })
    .speed(speed_mm / factor)
    .max_decimals(shown_places(unit).unwrap_or(3) + 1);
    if suffix {
        drag.suffix(format!(" {unit}"))
    } else {
        drag
    }
}

/// A length stored in millimetres as the interface shows it, with its unit:
/// "1.200 mm" or "0.0472 in".
pub fn length_text(mm: f64, display: Display) -> String {
    let unit = shown_unit("mm", display);
    format!(
        "{} {unit}",
        shown_text(&number(mm, places("mm")), "mm", display)
    )
}

/// A length stored in millimetres as a bare number in `display`'s unit, to
/// `places_mm` places in millimetres and one more in inches (a readout).
pub fn length_number(mm: f64, display: Display, places_mm: usize) -> String {
    match display {
        Display::Millimetres => format!("{mm:.places_mm$}"),
        Display::Inches => {
            let places = places_mm + 1;
            format!("{:.places$}", mm / MM_PER_INCH)
        }
    }
}

/// The context's key for the display choice.
fn key() -> egui::Id {
    egui::Id::unique("openchuck display units")
}

/// The display choice the shell set for this context (millimetres until it
/// sets one).
pub fn display(ctx: &egui::Context) -> Display {
    ctx.data(|d| d.get_temp(key())).unwrap_or_default()
}

/// Sets the display choice every number box of this context reads.
pub fn set_display(ctx: &egui::Context, display: Display) {
    ctx.data_mut(|d| d.insert_temp(key(), display));
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::float_cmp)]
    use super::*;

    fn stored(typed: &str, unit: &str, display: Display) -> String {
        read(typed, unit, display)
            .unwrap_or_else(|e| panic!("{typed}: {e}"))
            .stored
    }

    #[test]
    fn req_ui_18_req_nfr_05_inch_display_round_trips_to_millimetres() {
        // Shown in inches, rounded to four places.
        assert_eq!(shown_text("1.2", "mm", Display::Inches), "0.0472");
        assert_eq!(shown_text("3.175", "mm", Display::Inches), "0.1250");
        assert_eq!(shown_text("1500", "mm/min", Display::Inches), "59.1");
        assert_eq!(shown_unit("mm", Display::Inches), "in");
        assert_eq!(shown_unit("mm/min", Display::Inches), "in/min");
        // Degrees, speeds and plain numbers are never converted.
        assert_eq!(shown_text("12.5", "°", Display::Inches), "12.5");
        assert_eq!(shown_unit("rpm", Display::Inches), "rpm");
        // Millimetres show as stored; an empty box stays empty.
        assert_eq!(shown_text("1.2", "mm", Display::Millimetres), "1.2");
        assert_eq!(shown_text("", "mm", Display::Inches), "");
        // Typed in inches, stored in millimetres exactly.
        assert_eq!(stored("0.125", "mm", Display::Inches), "3.175");
        assert_eq!(stored("0.1", "mm", Display::Inches), "2.540");
        assert_eq!(stored("10", "mm/min", Display::Inches), "254");
    }

    #[test]
    fn req_ui_18_a_value_typed_with_a_unit_converts_exactly() {
        for display in [Display::Millimetres, Display::Inches] {
            let r = read("0.005 in", "mm", display).unwrap();
            assert_eq!(r.stored, "0.127");
            assert_eq!(r.typed_unit, Some("in"));
            assert_eq!(stored("1/8 in", "mm", display), "3.175");
            assert_eq!(stored("1/8\"", "mm", display), "3.175");
            assert_eq!(stored("0.2 mm", "mm", display), "0.2");
            assert_eq!(stored("3 + 0.5 mm", "mm", display), "3.500");
            assert_eq!(stored("1.5cm", "mm", display), "15.000");
            assert_eq!(stored("0.5 IN", "mm", display), "12.700");
            assert_eq!(stored("1000 mm/min", "mm/min", display), "1000");
            assert_eq!(stored("40 in/min", "mm/min", display), "1016");
        }
        // A plain number in millimetres is kept as typed (UI-26 as before).
        assert_eq!(stored("1.2", "mm", Display::Millimetres), "1.2");
        assert_eq!(
            read("1.2", "mm", Display::Millimetres).unwrap().typed_unit,
            None
        );
    }

    #[test]
    fn req_ui_18_req_nfr_07_an_unknown_unit_is_refused_with_a_message() {
        let err = read("5 ft", "mm", Display::Millimetres).unwrap_err();
        assert_eq!(
            err,
            "Cannot read '5 ft': this box reads mm, cm, in or \" after the value."
        );
        // A length unit in a feed box, and a feed unit in a length box.
        assert!(
            read("5 in", "mm/min", Display::Inches)
                .unwrap_err()
                .contains("mm/min or in/min")
        );
        assert!(
            read("5 in/min", "mm", Display::Inches)
                .unwrap_err()
                .contains("mm, cm, in")
        );
        // A unit with no number before it.
        assert!(
            read("in", "mm", Display::Inches)
                .unwrap_err()
                .contains("expected a number")
        );
        // A box that is not a length or a feed reads no unit, as before.
        assert!(read("5 deg", "°", Display::Inches).is_err());
        assert!(read("5 in", "rpm", Display::Inches).is_err());
        // A sum that cannot be read says so, as before.
        assert!(read("12* in", "mm", Display::Inches).is_err());
    }

    #[test]
    fn req_ui_18_the_note_says_what_was_stored() {
        assert_eq!(
            stored_note("0.127", "mm", Display::Millimetres),
            "Stored as 0.127 mm"
        );
        assert_eq!(
            stored_note("0.2", "mm", Display::Inches),
            "Stored as 0.2 mm (0.0079 in)"
        );
    }

    #[test]
    fn req_ui_18_hints_show_their_number_in_the_users_unit() {
        assert_eq!(
            shown_hint("2.54 · from the tool's cutting data", "mm", Display::Inches),
            "0.1000 · from the tool's cutting data"
        );
        assert_eq!(
            shown_hint("Machine setting", "mm", Display::Inches),
            "Machine setting"
        );
        assert_eq!(shown_hint("2.54", "mm", Display::Millimetres), "2.54");
    }

    #[test]
    fn req_ui_18_readouts_and_read_only_lengths_follow_the_unit() {
        assert_eq!(length_text(1.2, Display::Millimetres), "1.200 mm");
        assert_eq!(length_text(25.4, Display::Inches), "1.0000 in");
        assert_eq!(length_number(12.7, Display::Millimetres, 3), "12.700");
        assert_eq!(length_number(12.7, Display::Inches, 3), "0.5000");
    }

    #[test]
    fn req_ui_18_the_setting_reads_mm_or_in_and_anything_else_as_mm() {
        assert_eq!(Display::from_setting("in"), Display::Inches);
        assert_eq!(Display::from_setting(" IN "), Display::Inches);
        assert_eq!(Display::from_setting("mm"), Display::Millimetres);
        assert_eq!(Display::from_setting("furlongs"), Display::Millimetres);
        assert_eq!(Display::Inches.setting(), "in");
    }
}
