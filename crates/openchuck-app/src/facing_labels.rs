// SPDX-License-Identifier: AGPL-3.0-or-later
//! The Facing overlay's labels in the view, which of their kinds the user
//! shows, and the one chip that says what the view hides (issue #881;
//! decision 0702, the owner's answers of 25 and 26 Nov 2026).
//! Display only: the G-code, the checks, the simulation and the notes never
//! read any of it.

use serde::{Deserialize, Serialize};

use crate::pathview::ShownMoves;
use openchuck_project::id::OperationId;

/// The kinds of label the Facing overlay draws. End and Round facing's
/// labels are faces too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FacingLabel {
    /// A face: where its finished plane is ("Face A 0°: 13.650 mm from
    /// axis"), an end face's X, or a round face's diameter and X range.
    Face,
    /// The extent margin, when it is not zero.
    Margin,
    /// The distance between two opposite faces.
    Separation,
    /// Where a face stops beside a held region.
    Stop,
}

impl FacingLabel {
    /// The kinds in the order the menus list them.
    pub const ORDER: [Self; 4] = [Self::Face, Self::Margin, Self::Separation, Self::Stop];

    /// The kind's name in the Show menu and View > Facing labels.
    pub const fn menu_name(self) -> &'static str {
        match self {
            Self::Face => "Faces",
            Self::Margin => "Margin",
            Self::Separation => "Opposite faces",
            Self::Stop => "Stops beside a grip",
        }
    }

    /// The kind's name in the chip that says what is hidden.
    pub const fn chip_name(self) -> &'static str {
        match self {
            Self::Face => "Facing faces",
            Self::Margin => "Facing margin",
            Self::Separation => "Facing separations",
            Self::Stop => "Facing stops",
        }
    }
}

/// The name of the four kinds together, in the menus and the chip.
pub const FACING_LABELS_NAME: &str = "Facing labels";

/// What a Facing label or line belongs to: its operation and its kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FacingTag {
    /// The Facing operation it was drawn for; `None` while the shared
    /// drawing has not been placed in a setup's view yet.
    pub operation: Option<OperationId>,
    /// The kind of label (a stop line is tagged [`FacingLabel::Stop`]).
    pub kind: FacingLabel,
}

/// Which kinds of Facing label the view draws. Kept per user on the
/// computer (UI-15), never in the project. A settings file without it, or
/// without one of its kinds, shows that kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct ShownFacingLabels {
    /// Faces (End and Round facing's labels among them).
    pub faces: bool,
    /// The extent margin.
    pub margin: bool,
    /// The distances between opposite faces.
    pub separations: bool,
    /// Where faces stop beside a held region.
    pub stops: bool,
}

impl Default for ShownFacingLabels {
    fn default() -> Self {
        Self::ALL
    }
}

impl ShownFacingLabels {
    /// Every kind shown.
    pub const ALL: Self = Self {
        faces: true,
        margin: true,
        separations: true,
        stops: true,
    };

    /// No kind shown.
    pub const NONE: Self = Self {
        faces: false,
        margin: false,
        separations: false,
        stops: false,
    };

    fn field(&mut self, kind: FacingLabel) -> &mut bool {
        match kind {
            FacingLabel::Face => &mut self.faces,
            FacingLabel::Margin => &mut self.margin,
            FacingLabel::Separation => &mut self.separations,
            FacingLabel::Stop => &mut self.stops,
        }
    }

    /// Whether labels of `kind` are drawn.
    pub fn shows(self, kind: FacingLabel) -> bool {
        let mut copy = self;
        *copy.field(kind)
    }

    /// Shows or hides labels of `kind`.
    pub fn set(&mut self, kind: FacingLabel, shown: bool) {
        *self.field(kind) = shown;
    }

    /// Whether every kind is shown.
    pub fn all_shown(self) -> bool {
        self == Self::ALL
    }

    /// How many kinds are shown.
    pub fn count_shown(self) -> usize {
        FacingLabel::ORDER
            .into_iter()
            .filter(|k| self.shows(*k))
            .count()
    }

    /// The parent box's press: ticked or partly ticked hides all four, empty
    /// shows all four (as Entry and exit's box does).
    pub fn toggle_all(&mut self) {
        *self = if self.count_shown() == 0 {
            Self::ALL
        } else {
            Self::NONE
        };
    }

    /// The chip names of the kinds hidden, in menu order; the four hidden
    /// read as one name.
    pub fn hidden_names(self) -> Vec<&'static str> {
        if self.count_shown() == 0 {
            return vec![FACING_LABELS_NAME];
        }
        FacingLabel::ORDER
            .into_iter()
            .filter(|k| !self.shows(*k))
            .map(FacingLabel::chip_name)
            .collect()
    }
}

/// The chip beside Show while anything is hidden in the view: the hidden
/// kinds of toolpath move, then the hidden kinds of Facing label, in menu
/// order (decision 0702's length rule).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HiddenChip {
    /// Every hidden name, in order.
    pub names: Vec<String>,
}

/// How many names the long form of the chip spells out.
pub const CHIP_NAMES: usize = 2;

impl HiddenChip {
    /// The chip for these choices, or `None` when nothing is hidden.
    pub fn of(moves: ShownMoves, labels: ShownFacingLabels) -> Option<Self> {
        let mut names: Vec<String> = moves
            .hidden_names()
            .into_iter()
            .map(str::to_owned)
            .collect();
        if !labels.all_shown() {
            names.extend(labels.hidden_names().into_iter().map(str::to_owned));
        }
        (!names.is_empty()).then_some(Self { names })
    }

    /// The long form: at most [`CHIP_NAMES`] names, then how many more,
    /// "Hidden: Rapid, Lead-in and ramp +2 more".
    pub fn long(&self) -> String {
        let shown: Vec<&str> = self
            .names
            .iter()
            .take(CHIP_NAMES)
            .map(String::as_str)
            .collect();
        let more = self.names.len().saturating_sub(CHIP_NAMES);
        if more == 0 {
            format!("Hidden: {}", shown.join(", "))
        } else {
            format!("Hidden: {} +{more} more", shown.join(", "))
        }
    }

    /// The short form, for a strip without room for the long one:
    /// "Hidden (4)".
    pub fn short(&self) -> String {
        format!("Hidden ({})", self.names.len())
    }

    /// Every hidden name, for the Show menu's "Show everything" tooltip:
    /// "Hidden in the view: Rapid, Facing margin."
    pub fn full(&self) -> String {
        format!("Hidden in the view: {}.", self.names.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openchuck_types::ir::MoveKind;

    #[test]
    fn req_ui_15_req_sim_01_nothing_hidden_gives_no_chip() {
        assert_eq!(
            HiddenChip::of(ShownMoves::ALL, ShownFacingLabels::ALL),
            None
        );
    }

    /// Issue #881 (the owner's answers of 25 Nov 2026): one chip names what
    /// the view hides, moves first, then Facing labels; at most two names,
    /// then a count, and a short form when the strip has no room.
    #[test]
    fn req_ui_14_the_hidden_chip_names_two_and_counts_the_rest() {
        let mut moves = ShownMoves::ALL;
        moves.set(MoveKind::Rapid, false);
        moves.set(MoveKind::LeadIn, false);
        let mut labels = ShownFacingLabels::ALL;
        labels.set(FacingLabel::Margin, false);
        labels.set(FacingLabel::Stop, false);
        let chip = HiddenChip::of(moves, labels).unwrap();
        assert_eq!(chip.long(), "Hidden: Rapid, Lead-in and ramp +2 more");
        assert_eq!(chip.short(), "Hidden (4)");
        assert_eq!(
            chip.full(),
            "Hidden in the view: Rapid, Lead-in and ramp, Facing margin, Facing stops."
        );
        assert!(chip.long().chars().count() <= 40, "{}", chip.long());
        // Two names or fewer: no count.
        let chip = HiddenChip::of(ShownMoves::ALL, labels).unwrap();
        assert_eq!(chip.long(), "Hidden: Facing margin, Facing stops");
        // The four kinds hidden read as one name.
        let chip = HiddenChip::of(ShownMoves::ALL, ShownFacingLabels::NONE).unwrap();
        assert_eq!(chip.long(), "Hidden: Facing labels");
        assert_eq!(chip.short(), "Hidden (1)");
    }

    #[test]
    fn req_ui_14_the_facing_labels_box_sets_all_four() {
        let mut labels = ShownFacingLabels::ALL;
        labels.set(FacingLabel::Separation, false);
        assert_eq!(labels.count_shown(), 3);
        labels.toggle_all();
        assert_eq!(labels, ShownFacingLabels::NONE);
        labels.toggle_all();
        assert_eq!(labels, ShownFacingLabels::ALL);
        for kind in FacingLabel::ORDER {
            assert!(labels.shows(kind));
            assert!(!kind.menu_name().is_empty() && kind.chip_name().starts_with("Facing "));
        }
    }

    #[test]
    fn req_ui_15_a_settings_file_without_a_kind_shows_it() {
        let labels: ShownFacingLabels = toml::from_str("margin = false").unwrap();
        assert!(labels.faces && labels.separations && labels.stops && !labels.margin);
        let all: ShownFacingLabels = toml::from_str("").unwrap();
        assert_eq!(all, ShownFacingLabels::ALL);
    }
}
