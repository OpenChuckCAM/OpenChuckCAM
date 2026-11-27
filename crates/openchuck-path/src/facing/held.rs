// SPDX-License-Identifier: AGPL-3.0-or-later
//! Facing beside a held region (OP-24, SET-09; issue #500, decision
//! 0685 amending 0252). No part of the tool comes over a gripped
//! length while it cuts: its centre keeps the tool's largest radius and
//! [`HELD_CLEARANCE`](crate::HELD_CLEARANCE) from each held interval, as the
//! host's [`StrategyInput::keep_out`](crate::StrategyInput::keep_out) does
//! (decision 0161). Where the requested coverage reaches into that band,
//! Facing faces what the tool reaches, leaves the rest unfaced and says
//! where, why and how much in an information note. It never crops anything
//! else: a gap elsewhere, or nothing left to face, still refuses.

use openchuck_types::tool::Tool;

use super::geometry::FacingEnd;
use super::plan::FacingSetup;
use crate::error::{PathError, Result};
use crate::notice::Notice;
use crate::strategy::HELD_CLEARANCE;

/// A span of the requested coverage along X that Facing leaves unfaced
/// beside a held region, because no part of the tool may come over the
/// grip or within [`HELD_CLEARANCE`](crate::HELD_CLEARANCE) of it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Unfaced {
    /// The held end it lies beside.
    pub beside: FacingEnd,
    /// That held interval along X, in millimetres.
    pub held: (f64, f64),
    /// The unfaced span along X, `(low, high)`, in millimetres: from the
    /// requested coverage's edge to where the face is finished.
    pub x: (f64, f64),
}

/// The requested X span of one face, cut down to what the tool reaches.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Crop {
    /// The tool centre's allowed X span.
    pub centres: (f64, f64),
    /// The X span counted as finished.
    pub finished: (f64, f64),
    /// What is left unfaced, start end first.
    pub unfaced: Vec<Unfaced>,
}

/// The largest radius of the tool, its cutting profile or its shank: how far
/// its centre keeps from a held region, before the clearance (decision
/// 0161). The same measure the host widens its keep-out by.
pub(crate) fn reach(tool: &Tool) -> Result<f64> {
    let profile = tool.profile()?;
    Ok(profile
        .pieces()
        .iter()
        .map(|p| p.end().radius.0)
        .fold(profile.cutting_radius(), f64::max)
        .max(tool.shank_diameter.0 / 2.))
}

/// Each held interval widened by the tool's reach and the clearance: the X
/// span the tool's centre keeps out of while it cuts, start end first.
pub(crate) fn keep_out(setup: &FacingSetup<'_>) -> Result<[Option<(f64, f64)>; 2]> {
    let widen = reach(setup.tool)? + HELD_CLEARANCE;
    let mut out = [None, None];
    for (slot, held) in out.iter_mut().zip(setup.held) {
        if let Some((lo, hi)) = held {
            if !lo.is_finite() || !hi.is_finite() || hi <= lo {
                return Err(refuse(
                    "held intervals must be finite with positive length".into(),
                ));
            }
            *slot = Some((lo - widen, hi + widen));
        }
    }
    Ok(out)
}

/// Cut the requested X span of a face down to what the tool reaches beside
/// the held regions. `counted` is how far past the tool's centre the mode's
/// completion check counts the face finished at the end of its cut along X:
/// the tool's radius for Box rows across X, the radius less the Round check's
/// reserve for Round rings, zero for Flat passes along X (their round ends
/// are not counted). Refuses when nothing is left to face.
pub(crate) fn crop(setup: &FacingSetup<'_>, requested: (f64, f64), counted: f64) -> Result<Crop> {
    let mut crop = Crop {
        centres: requested,
        finished: requested,
        unfaced: Vec::new(),
    };
    let [start, far] = keep_out(setup)?;
    if let (Some((a, b)), Some(held)) = (start, setup.held[0])
        && b > requested.0
        && a < requested.1
    {
        crop.centres.0 = crop.centres.0.max(b);
        let edge = b - counted;
        if edge > requested.0 {
            crop.finished.0 = crop.finished.0.max(edge);
            crop.unfaced.push(Unfaced {
                beside: FacingEnd::Start,
                held,
                x: (requested.0, edge.min(requested.1)),
            });
        }
    }
    if let (Some((a, b)), Some(held)) = (far, setup.held[1])
        && a < requested.1
        && b > requested.0
    {
        crop.centres.1 = crop.centres.1.min(a);
        let edge = a + counted;
        if edge < requested.1 {
            crop.finished.1 = crop.finished.1.min(edge);
            crop.unfaced.push(Unfaced {
                beside: FacingEnd::Far,
                held,
                x: (edge.max(requested.0), requested.1),
            });
        }
    }
    if crop.centres.0 > crop.centres.1 || crop.finished.0 >= crop.finished.1 {
        let grips = crop
            .unfaced
            .iter()
            .map(|u| held_words(u.beside, u.held))
            .collect::<Vec<_>>()
            .join(" and ");
        return Err(refuse(format!(
            "the requested extent ({xp1} to {xp2}) lies within {grips} and the {HELD_CLEARANCE:.3} mm \
             the whole tool keeps from it, so nothing is left to face; shorten the held length or \
             face this from a setup that holds the other end",
            xp1 = crate::words::x(requested.0),
            xp2 = crate::words::n(requested.1)
        )));
    }
    Ok(crop)
}

/// Refuse unless the host keeps the tool's centre out of exactly the spans
/// Facing derives from the grip (decision 0161): Facing states where it
/// stops, and can only do so when it shares the host's limit (NFR-07).
pub(crate) fn check_host(setup: &FacingSetup<'_>, host: &[(f64, f64)]) -> Result<()> {
    let derived: Vec<(f64, f64)> = keep_out(setup)?.into_iter().flatten().collect();
    let same = |(lo, hi): (f64, f64), (a, b): (f64, f64)| {
        let tolerance = 1e-9 * lo.abs().max(hi.abs()).max(1.);
        (a - lo).abs() <= tolerance && (b - hi).abs() <= tolerance
    };
    // Each way round, in any order (issue #895 item 2): a span the grip gives
    // that the host lacks, or one the host holds that the grip does not give.
    let unmatched = derived
        .iter()
        .find(|&&span| !host.iter().any(|&h| same(span, h)))
        .or_else(|| {
            host.iter()
                .find(|&&h| !derived.iter().any(|&span| same(span, h)))
        });
    if let Some((lo, hi)) = unmatched {
        return Err(refuse(format!(
            "the held region Facing keeps the tool's centre out of ({xp1} to {xp2}) is not \
             the one this set-up gives, so Facing cannot say where it stops beside it",
            xp1 = crate::words::x(*lo),
            xp2 = crate::words::n(*hi)
        )));
    }
    Ok(())
}

/// The information notes on what is left unfaced: one for each held end
/// and unfaced span, naming every face (`faces`, as "at A 0.000°" or
/// "round") it applies to. Whole sentences with exact numbers (NFR-07).
pub(crate) fn notes(faces: &[(String, Vec<Unfaced>)], flat: bool) -> Vec<Notice> {
    let mut groups: Vec<(Unfaced, Vec<&str>)> = Vec::new();
    for (name, unfaced) in faces {
        for u in unfaced {
            match groups.iter_mut().find(|(g, _)| g == u) {
                Some((_, names)) => names.push(name),
                None => groups.push((*u, vec![name])),
            }
        }
    }
    groups
        .into_iter()
        .map(|(u, names)| {
            let on = match names.as_slice() {
                [] => String::new(),
                [one] => (*one).to_owned(),
                [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
            };
            let finished = match u.beside {
                FacingEnd::Start => format!("finished from {xp1}", xp1 = crate::words::x(u.x.1)),
                FacingEnd::Far => format!("finished up to {xp1}", xp1 = crate::words::x(u.x.0)),
            };
            let why = if flat {
                " (its passes along X end there; the tool's round ends beyond them are not counted)"
            } else {
                ""
            };
            Notice::information(format!("Facing {on} stops short of {}: no part of the tool comes within {HELD_CLEARANCE:.3} mm \
                 of it, so the face is {finished}{why}, and {xp1} to {xp2} ({:.3} mm along X) of \
                 the requested extent is left unfaced. To face it, shorten the held length or face it \
                 in a setup that holds the other end.",
held_words(u.beside, u.held),
u.x.1 - u.x.0,
xp1 = crate::words::x(u.x.0),
xp2 = crate::words::n(u.x.1)))
            .about(crate::notice::topic::HELD_REGION)
        })
        .collect()
}

fn held_words(end: FacingEnd, (lo, hi): (f64, f64)) -> String {
    let which = match end {
        FacingEnd::Start => "start",
        FacingEnd::Far => "far end",
    };
    format!(
        "the held {which} ({xp1} to {xp2})",
        xp1 = crate::words::x(lo),
        xp2 = crate::words::n(hi)
    )
}

fn refuse(reason: String) -> PathError {
    PathError::refused(crate::notice::topic::HELD_REGION, reason)
}

/// Legacy Flat beside a held region (decision 0685): its passes
/// along X are dropped onto the composite target, where the held region
/// with its stock to leave can lift their ends above the face, so where the
/// face is finished is read from the actual cut, not from the keep-out.
/// `segments` are the at-target cutting moves of `face`, each as its two
/// ends in the plane's (X, across) coordinates. Each held side's unfaced
/// span is widened to where every line along X is at the face, by at most
/// `bound`; a line that stops further from the grip is left to the
/// completion check, which refuses it.
pub(crate) fn measure_flat(
    face: &mut super::description::FaceDescription,
    segments: &[([f64; 2], [f64; 2])],
    bound: f64,
) {
    let mut lines: Vec<(f64, f64, f64)> = Vec::new();
    for (a, b) in segments {
        let v = a[1];
        let (lo, hi) = (a[0].min(b[0]), a[0].max(b[0]));
        match lines
            .iter_mut()
            .find(|(line, ..)| (line - v).abs() <= 1e-9 * v.abs().max(1.))
        {
            Some((_, l, h)) => {
                *l = l.min(lo);
                *h = h.max(hi);
            }
            None => lines.push((v, lo, hi)),
        }
    }
    let Some(start) = lines.iter().map(|l| l.1).reduce(f64::max) else {
        return;
    };
    let end = lines.iter().map(|l| l.2).fold(f64::INFINITY, f64::min);
    for u in &mut face.unfaced {
        match u.beside {
            FacingEnd::Start if start > u.x.1 && start <= u.x.1 + bound => u.x.1 = start,
            FacingEnd::Far if end < u.x.0 && end >= u.x.0 - bound => u.x.0 = end,
            _ => {}
        }
    }
}
