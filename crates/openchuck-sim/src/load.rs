// SPDX-License-Identifier: AGPL-3.0-or-later
//! The load on the tool, measured as the simulation cuts the stock (SIM-16;
//! decision record TOOL-LOAD-1, Q2 A).
//!
//! For each move the simulator adds up what its poses removed
//! ([`MoveLoad`]): the volume, how far along the tool axis the material
//! removed reached, and how far the tool's tip travelled through the part.
//! [`ToolLoad::summarise`] turns that into the figures the program shows:
//!
//! - the **engaged length**: how much of the cutting edge was in the
//!   material, along the tool axis (the highest point removed less the
//!   lowest), per move;
//! - the **cut section**: the material removed per millimetre of travel, in
//!   mm², over a stretch of at least one tool diameter of travel (a single
//!   short move's volume includes the tool's own footprint where it starts,
//!   so a move alone would overstate it);
//! - the **removal rate**: the volume removed over a stretch divided by its
//!   programmed time (the program's time estimate, with the machine's feed
//!   limits), and its average over the **cutting time**, the time of the fed
//!   moves that remove material.
//!
//! Rapids are left out of every figure: a rapid that removes material is a
//! fault reported on its own ([`crate::RapidCut`]; decision record 0661).
//! None of these figures says a cut is safe for the tool or the machine
//! (NFR-12): they say how heavy it was, as simulated.

use openchuck_types::ir::{MoveKind, ToolpathGroup};

/// Removal below this, in cubic millimetres, is rounding, not a cut.
const VOLUME_EPSILON: f64 = 1e-9;

/// What one move removed, as the simulation cut it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveLoad {
    /// Volume removed, mm³.
    pub volume: f64,
    /// The lowest point of the material removed, along the tool axis from
    /// the tip, mm; infinite when nothing was removed.
    pub low: f64,
    /// The highest point of the material removed, along the tool axis from
    /// the tip, mm; minus infinity when nothing was removed.
    pub high: f64,
    /// How far the tool's tip travelled through the part during the move
    /// (in the part's frame, so a turn of A counts), mm.
    pub travel: f64,
}

impl Default for MoveLoad {
    fn default() -> Self {
        Self {
            volume: 0.0,
            low: f64::INFINITY,
            high: f64::NEG_INFINITY,
            travel: 0.0,
        }
    }
}

impl MoveLoad {
    /// Whether the move removed material.
    #[must_use]
    pub fn cuts(&self) -> bool {
        self.volume > VOLUME_EPSILON && self.high >= self.low
    }

    /// The engaged length: how much of the cutting edge was in the
    /// material along the tool axis, mm (zero when nothing was removed).
    #[must_use]
    pub fn engaged(&self) -> f64 {
        if self.cuts() {
            (self.high - self.low).max(0.0)
        } else {
            0.0
        }
    }

    pub(crate) fn add(&mut self, volume: f64, low: f64, high: f64) {
        self.volume += volume;
        self.low = self.low.min(low);
        self.high = self.high.max(high);
    }
}

/// What every move of one group removed, with the tool's diameter (the
/// stretch over which the cut section is taken).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ToolLoad {
    /// One entry per move of the group, in order; a rapid's is empty.
    pub moves: Vec<MoveLoad>,
    /// The cutting diameter of the tool, mm.
    pub diameter: f64,
}

/// One stretch of path, at least one tool diameter of travel long.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stretch {
    /// The first and last move of the stretch (indices into the group).
    pub first: usize,
    /// The last move of the stretch.
    pub last: usize,
    /// Material removed per millimetre of travel over the stretch, mm².
    pub section: f64,
    /// Material removed per minute over the stretch, mm³/min, from the
    /// programmed time of its moves; `None` when they take no time.
    pub rate: Option<f64>,
    /// The longest engaged length of a move in the stretch, mm.
    pub engaged: f64,
}

/// The tool-load figures of one operation (SIM-16).
#[derive(Debug, Clone, PartialEq)]
pub struct LoadSummary {
    /// The limit the stretches were measured against, mm².
    pub limit: f64,
    /// Stretches measured (each of at least one tool diameter of travel).
    pub stretches: usize,
    /// Stretches whose cut section passes the limit.
    pub over: usize,
    /// The stretch with the largest cut section, if any stretch removed
    /// material.
    pub heaviest: Option<Stretch>,
    /// The stretch with the highest removal rate, if any.
    pub fastest: Option<Stretch>,
    /// The longest engaged length of any move, mm, and that move.
    pub longest_engaged: Option<(f64, usize)>,
    /// Material removed by the fed moves, mm³.
    pub removed: f64,
    /// The programmed time of the fed moves that remove material, s.
    pub cutting_time: f64,
    /// The programmed time of every move, rapids included, s.
    pub total_time: f64,
}

impl LoadSummary {
    /// The average removal rate over the cutting time, mm³/min; `None`
    /// when no time was spent cutting.
    #[must_use]
    pub fn average_rate(&self) -> Option<f64> {
        (self.cutting_time > 0.0).then(|| self.removed / self.cutting_time * 60.0)
    }

    /// The average removal rate over the whole programmed time, rapids
    /// included, mm³/min; `None` for a path that takes no time.
    #[must_use]
    pub fn overall_rate(&self) -> Option<f64> {
        (self.total_time > 0.0).then(|| self.removed / self.total_time * 60.0)
    }
}

impl ToolLoad {
    /// The figures of `group` (the group these loads were measured on)
    /// against `limit`, the largest cut section allowed before a warning,
    /// mm². Stretches run over consecutive fed moves and start again after a
    /// rapid; each is the shortest run of moves ending at a move whose
    /// travel is at least one tool diameter, so every stretch is measured
    /// once, at its end.
    #[must_use]
    pub fn summarise(&self, group: &ToolpathGroup, limit: f64) -> LoadSummary {
        let diameter = self.diameter.max(1e-6);
        let mut out = LoadSummary {
            limit,
            stretches: 0,
            over: 0,
            heaviest: None,
            fastest: None,
            longest_engaged: None,
            removed: 0.0,
            cutting_time: 0.0,
            total_time: 0.0,
        };
        // The current run of fed moves: its first move, and its sums.
        let mut start = 0;
        let mut volume = 0.0;
        let mut travel = 0.0;
        let mut time = 0.0;
        for (i, segment) in group.segments.iter().enumerate() {
            let duration = segment.duration.0.max(0.0);
            out.total_time += duration;
            let load = self.moves.get(i).copied().unwrap_or_default();
            if segment.kind == MoveKind::Rapid {
                start = i + 1;
                volume = 0.0;
                travel = 0.0;
                time = 0.0;
                continue;
            }
            if load.cuts() {
                out.removed += load.volume;
                out.cutting_time += duration;
                let engaged = load.engaged();
                if out.longest_engaged.is_none_or(|(e, _)| engaged > e) {
                    out.longest_engaged = Some((engaged, i));
                }
            }
            volume += load.volume;
            travel += load.travel;
            time += duration;
            // Drop moves from the front while the run stays at least one
            // diameter long without them.
            while start < i {
                let front = self.moves.get(start).copied().unwrap_or_default();
                let front_time = group
                    .segments
                    .get(start)
                    .map_or(0.0, |s| s.duration.0.max(0.0));
                if travel - front.travel < diameter {
                    break;
                }
                volume -= front.volume;
                travel -= front.travel;
                time -= front_time;
                start += 1;
            }
            if travel < diameter {
                continue;
            }
            out.stretches += 1;
            let section = volume.max(0.0) / travel;
            let rate = (time > 0.0).then(|| volume.max(0.0) / time * 60.0);
            let engaged = (start..=i)
                .filter_map(|k| self.moves.get(k))
                .map(MoveLoad::engaged)
                .fold(0.0, f64::max);
            let stretch = Stretch {
                first: start,
                last: i,
                section,
                rate,
                engaged,
            };
            if section > limit {
                out.over += 1;
            }
            if volume > VOLUME_EPSILON {
                if out.heaviest.is_none_or(|h| section > h.section) {
                    out.heaviest = Some(stretch);
                }
                if let Some(r) = rate
                    && out.fastest.is_none_or(|f| f.rate.is_none_or(|fr| r > fr))
                {
                    out.fastest = Some(stretch);
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openchuck_types::geometry::{Point3, UnitVec3};
    use openchuck_types::ir::{PathTags, Segment, ToolPose};
    use openchuck_types::units::{Degrees, FeedRate, Millimetres, Seconds};

    fn group(kinds: &[(MoveKind, f64)]) -> ToolpathGroup {
        let pose = ToolPose {
            tip: Point3::new(0.0, 0.0, 10.0),
            axis: UnitVec3::Z,
            a: Degrees(0.0),
        };
        ToolpathGroup {
            rotary_reset_after: Vec::new(),
            tags: PathTags {
                operation: "op".into(),
                setup: "setup".into(),
                tool: "tool".into(),
            },
            start: pose,
            segments: kinds
                .iter()
                .map(|&(kind, seconds)| Segment {
                    kind,
                    end: pose,
                    contact_feed: (kind != MoveKind::Rapid).then_some(FeedRate(600.0)),
                    duration: Seconds(seconds),
                    contact_length: Millimetres(1.0),
                })
                .collect(),
        }
    }

    fn cut(volume: f64, low: f64, high: f64, travel: f64) -> MoveLoad {
        MoveLoad {
            volume,
            low,
            high,
            travel,
        }
    }

    #[test]
    fn req_sim_16_a_steady_slot_gives_its_section_and_rate() {
        // Ten moves of 1 mm, each removing a slot 2 mm deep and 4 mm wide
        // (8 mm³ per mm) in 0.1 s: 8 mm² and 4800 mm³/min.
        let g = group(&[(MoveKind::Cutting, 0.1); 10]);
        let load = ToolLoad {
            moves: vec![cut(8.0, 0.0, 2.0, 1.0); 10],
            diameter: 4.0,
        };
        let s = load.summarise(&g, 10.0);
        assert_eq!(s.stretches, 7, "stretches end at moves 4 to 10");
        assert_eq!(s.over, 0);
        let h = s.heaviest.expect("a heaviest stretch");
        assert!((h.section - 8.0).abs() < 1e-9);
        assert!((h.rate.unwrap() - 4800.0).abs() < 1e-6);
        assert!((h.engaged - 2.0).abs() < 1e-12);
        assert!((s.average_rate().unwrap() - 4800.0).abs() < 1e-6);
        assert!((s.removed - 80.0).abs() < 1e-9);
        let over = load.summarise(&g, 7.0);
        assert_eq!(over.over, 7);
    }

    #[test]
    fn req_sim_16_rapids_and_air_moves_are_left_out_of_the_rates() {
        // A cut of 1 s removing 60 mm³, an air move of 1 s, a rapid of 1 s:
        // the average over the cutting time is 3600 mm³/min, over the whole
        // time 1200.
        let g = group(&[
            (MoveKind::Cutting, 1.0),
            (MoveKind::Cutting, 1.0),
            (MoveKind::Rapid, 1.0),
        ]);
        let load = ToolLoad {
            moves: vec![
                cut(60.0, 0.0, 1.0, 5.0),
                cut(0.0, f64::INFINITY, f64::NEG_INFINITY, 5.0),
                MoveLoad::default(),
            ],
            diameter: 3.0,
        };
        let s = load.summarise(&g, 100.0);
        assert!((s.average_rate().unwrap() - 3600.0).abs() < 1e-9);
        assert!((s.overall_rate().unwrap() - 1200.0).abs() < 1e-9);
        assert!((s.cutting_time - 1.0).abs() < 1e-12);
    }

    #[test]
    fn req_sim_16_a_stretch_starts_again_after_a_rapid() {
        // Two cuts of 2 mm either side of a rapid, with a 3 mm tool: no run
        // reaches 3 mm of travel, so nothing is measured.
        let g = group(&[
            (MoveKind::Cutting, 0.1),
            (MoveKind::Rapid, 0.1),
            (MoveKind::Cutting, 0.1),
        ]);
        let load = ToolLoad {
            moves: vec![
                cut(10.0, 0.0, 1.0, 2.0),
                MoveLoad::default(),
                cut(10.0, 0.0, 1.0, 2.0),
            ],
            diameter: 3.0,
        };
        let s = load.summarise(&g, 1.0);
        assert_eq!(s.stretches, 0);
        assert!(s.heaviest.is_none());
        assert!((s.removed - 20.0).abs() < 1e-12);
    }

    #[test]
    fn req_sim_16_the_longest_engaged_length_is_reported_with_its_move() {
        let g = group(&[(MoveKind::Cutting, 0.1); 3]);
        let load = ToolLoad {
            moves: vec![
                cut(1.0, 0.0, 1.0, 1.0),
                cut(0.01, 0.5, 12.0, 1.0),
                cut(1.0, 0.0, 2.0, 1.0),
            ],
            diameter: 2.0,
        };
        let s = load.summarise(&g, 10.0);
        let (length, at) = s.longest_engaged.unwrap();
        assert!((length - 11.5).abs() < 1e-12);
        assert_eq!(at, 1);
    }
}
