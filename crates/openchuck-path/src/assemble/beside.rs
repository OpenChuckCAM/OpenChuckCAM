// SPDX-License-Identifier: AGPL-3.0-or-later
//! The faces an entry or exit is measured against besides the operation's
//! own surface (OP-11, OP-12; decision records 0682 and 0701).
//!
//! Two sets, each with its own allowance:
//!
//! - **Next to the face set** (swarf's
//!   [`RegionConstraint::neighbours`](crate::RegionConstraint::neighbours),
//!   decision 0682): a lead or helix may cut each face as deep as the pass
//!   itself cuts it where the move meets the pass, plus the tolerance and the
//!   sampling slack, as swarf measures and warns of the pass's own cut
//!   (decision 0532 item 3).
//! - **Outside the selection** (every other strategy's
//!   [`RegionConstraint::outside`](crate::RegionConstraint::outside), decision
//!   0701, the owner's answers to issue #957): the tolerance and the sampling
//!   slack only, as the pass's own cut into those faces is measured nowhere
//!   else; where the pass itself cuts one, the refusal says so.
//!
//! Only the faces an entry comes near are sampled (the owner's per-entry
//! proximity): each face's box, from a coarse sampling grown by its own
//! spacing, is compared with the box the tool's cutting part sweeps over the
//! entry's positions, grown by the stock to leave and the tolerance. A face
//! outside that box cannot meet the cutting part during the entry. A face
//! inside it is sampled once, at swarf's spacing (decision 0532 item 1), the
//! first time it is needed; the samples do not depend on which thread asks
//! first, so the measure is the same on any thread (NFR-06).

use std::collections::BTreeMap;
use std::sync::OnceLock;

use openchuck_types::geometry::{Aabb, Point3, UnitVec3};
use openchuck_types::tool::profile::RevolvedProfile;

use crate::cutter;
use crate::surface::{ParametricFace, SurfaceSource};

/// How deep a face beside the operation's surface may be cut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Allowance {
    /// As deep as the pass itself cuts it where the move meets the pass
    /// (swarf's faces next to the face set, decision 0682).
    PassCut,
    /// The tolerance and the sampling slack only (the faces outside the
    /// selection, decision 0701, the owner's Q3 answer).
    Tolerance,
}

/// The faces beside the operation's surface, sampled as they are needed.
pub(crate) struct Beside<'a> {
    faces: Vec<Face<'a>>,
    spacing: f64,
    pub(crate) allowance: Allowance,
}

struct Face<'a> {
    number: u32,
    face: &'a dyn ParametricFace,
    /// A box that holds the whole face: its coarse samples, grown by their
    /// spacing.
    bounds: Aabb,
    /// The face's samples at `spacing`, and how much deeper a cut can be
    /// between them than at the nearest one (`s / sqrt 2` for samples `s`
    /// apart; OI-2078).
    samples: OnceLock<(Vec<Point3>, f64)>,
}

/// The spacing of the coarse grid a face's box is read from, mm.
const COARSE: f64 = 2.0;

impl<'a> Beside<'a> {
    /// The exact faces of `source`, in face-number order, for a tool of
    /// cutting radius `radius`; `None` when it has none (a mesh: OI to
    /// come, decision 0701's open points).
    pub(crate) fn new(
        source: &'a dyn SurfaceSource,
        radius: f64,
        allowance: Allowance,
    ) -> Option<Self> {
        let mut list = source.exact_faces();
        list.sort_by_key(|f| f.number);
        list.dedup_by_key(|f| f.number);
        let faces: Vec<Face<'a>> = list
            .into_iter()
            .filter_map(|f| {
                let coarse = crate::swarf::face_samples(f.face, COARSE);
                let apart = coarse
                    .iter()
                    .map(|(_, _, apart)| *apart)
                    .fold(COARSE, f64::max);
                let bounds = coarse
                    .iter()
                    .map(|(p, _, _)| *p)
                    .fold(None::<Aabb>, |b, p| {
                        Some(match b {
                            None => Aabb { min: p, max: p },
                            Some(b) => grow_to(b, p),
                        })
                    })?;
                Some(Face {
                    number: f.number,
                    face: f.face,
                    bounds: widen(bounds, apart),
                    samples: OnceLock::new(),
                })
            })
            .collect();
        (!faces.is_empty()).then_some(Self {
            faces,
            spacing: crate::swarf::guard_spacing(radius),
            allowance,
        })
    }

    /// The faces whose box meets `reach`, by index, in face-number order.
    pub(crate) fn near(&self, reach: &Aabb) -> Vec<usize> {
        self.faces
            .iter()
            .enumerate()
            .filter(|(_, f)| overlaps(&f.bounds, reach))
            .map(|(i, _)| i)
            .collect()
    }

    fn samples(&self, i: usize) -> Option<&(Vec<Point3>, f64)> {
        let f = self.faces.get(i)?;
        Some(f.samples.get_or_init(|| {
            let samples = crate::swarf::face_samples(f.face, self.spacing);
            let apart = samples
                .iter()
                .map(|(_, _, apart)| *apart)
                .fold(self.spacing, f64::max);
            (
                samples.into_iter().map(|(p, _, _)| p).collect(),
                apart * std::f64::consts::FRAC_1_SQRT_2,
            )
        }))
    }

    /// How much deeper the tool can cut face `i` between its samples than at
    /// the nearest one.
    pub(crate) fn slack(&self, i: usize) -> f64 {
        self.samples(i).map_or(0.0, |s| s.1)
    }

    /// The face's number, for a refusal.
    pub(crate) fn number(&self, i: usize) -> u32 {
        self.faces.get(i).map_or(u32::MAX, |f| f.number)
    }

    /// The deepest cut into each of the faces `near` (indices) by the tool
    /// `cutter` of radius `radius` with its tip at `tip` along `axis`; a
    /// face it does not cut is left out. In index order, so the same on any
    /// thread (NFR-06).
    pub(crate) fn cuts(
        &self,
        near: &[usize],
        cutter: &RevolvedProfile,
        radius: f64,
        (tip, axis): (Point3, UnitVec3),
    ) -> BTreeMap<usize, f64> {
        let mut out = BTreeMap::new();
        for &i in near {
            let Some((points, _)) = self.samples(i) else {
                continue;
            };
            let mut deepest = 0.0_f64;
            for p in points {
                // Outside the tool's radius or below its tip: not in the
                // tool.
                if cutter::bound_at(radius, tip, axis, *p) >= 0.0 {
                    continue;
                }
                deepest = deepest.max(-cutter::clearance_at(cutter, tip, axis, *p));
            }
            if deepest > 0.0 {
                out.insert(i, deepest);
            }
        }
        out
    }
}

/// `b` grown to hold `p`.
fn grow_to(b: Aabb, p: Point3) -> Aabb {
    Aabb {
        min: Point3::new(b.min.x.min(p.x), b.min.y.min(p.y), b.min.z.min(p.z)),
        max: Point3::new(b.max.x.max(p.x), b.max.y.max(p.y), b.max.z.max(p.z)),
    }
}

/// `b` grown by `d` on every side.
pub(crate) fn widen(b: Aabb, d: f64) -> Aabb {
    Aabb {
        min: Point3::new(b.min.x - d, b.min.y - d, b.min.z - d),
        max: Point3::new(b.max.x + d, b.max.y + d, b.max.z + d),
    }
}

/// The box holding `points`, or `None` for none.
pub(crate) fn bounding(points: impl IntoIterator<Item = Point3>) -> Option<Aabb> {
    points.into_iter().fold(None, |b, p| {
        Some(match b {
            None => Aabb { min: p, max: p },
            Some(b) => grow_to(b, p),
        })
    })
}

fn overlaps(a: &Aabb, b: &Aabb) -> bool {
    a.min.x <= b.max.x
        && b.min.x <= a.max.x
        && a.min.y <= b.max.y
        && b.min.y <= a.max.y
        && a.min.z <= b.max.z
        && b.min.z <= a.max.z
}
