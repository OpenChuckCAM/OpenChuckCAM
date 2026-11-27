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
//! proximity): each face's box, read from its whole untrimmed surface so it
//! holds every part of the trim, is compared with the box the tool's cutting
//! part sweeps over the entry's positions (on up its axis past the flutes),
//! grown by the stock to leave and the tolerance. A face
//! outside that box cannot meet the cutting part during the entry. A face
//! inside it is sampled once, at swarf's spacing (decision 0532 item 1), the
//! first time it is needed; the samples do not depend on which thread asks
//! first, so the measure is the same on any thread (NFR-06).

use std::collections::BTreeMap;
use std::sync::OnceLock;

use openchuck_types::geometry::{Aabb, Point3, UnitVec3};
use openchuck_types::tool::profile::RevolvedProfile;

use crate::cutter;
use crate::surface::{FaceRef, ParametricFace, SurfaceSource};

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
    /// A box that holds every face.
    all: Aabb,
    spacing: f64,
    pub(crate) allowance: Allowance,
}

struct Face<'a> {
    number: u32,
    face: &'a dyn ParametricFace,
    /// A box that holds the whole face: its untrimmed surface over its
    /// parameter box, on a coarse grid grown by the grid's spacing, so every
    /// part of any trim is in it.
    bounds: Aabb,
    /// The face's samples at `spacing`, and how much deeper a cut can be
    /// between them than at the nearest one (`s / sqrt 2` for samples `s`
    /// apart; OI-2078).
    samples: OnceLock<(Vec<Point3>, f64)>,
}

/// The lines each way of the grid a face's box is read from.
const BOX_GRID: usize = 65;

impl<'a> Beside<'a> {
    /// The exact faces of `source`, in face-number order, for a tool of
    /// cutting radius `radius`; `None` when it has none (a mesh: OI to
    /// come, decision 0701's open points).
    pub(crate) fn new(
        source: &'a dyn SurfaceSource,
        radius: f64,
        allowance: Allowance,
    ) -> Option<Self> {
        Self::from_faces(source.exact_faces(), radius, allowance)
    }

    /// [`new`](Self::new) from the faces `list`.
    pub(crate) fn from_faces(
        mut list: Vec<FaceRef<'a>>,
        radius: f64,
        allowance: Allowance,
    ) -> Option<Self> {
        list.sort_by_key(|f| f.number);
        list.dedup_by_key(|f| f.number);
        let spacing = crate::swarf::guard_spacing(radius);
        let faces: Vec<Face<'a>> = list
            .into_iter()
            .filter_map(|f| {
                Some(Face {
                    number: f.number,
                    face: f.face,
                    bounds: surface_box(f.face)?,
                    samples: OnceLock::new(),
                })
            })
            .collect();
        let all = faces
            .iter()
            .map(|f| f.bounds)
            .reduce(|a, b| grow_to(grow_to(a, b.min), b.max))?;
        Some(Self {
            faces,
            all,
            spacing,
            allowance,
        })
    }

    /// The box the tool's cutting part can sweep at `positions` (tip and
    /// axis), its flutes `top` long, grown by `grow`: from the tip on up
    /// the axis past the top of the flutes, as far as any face goes, since
    /// the cutting part goes on up at the cutting radius
    /// (`cutter::clearance_at`; CodeRabbit on #972). `None` for no
    /// positions.
    pub(crate) fn reach(
        &self,
        positions: &[(Point3, UnitVec3)],
        top: f64,
        grow: f64,
    ) -> Option<Aabb> {
        let corners = [
            self.all.min,
            self.all.max,
            Point3::new(self.all.min.x, self.all.min.y, self.all.max.z),
            Point3::new(self.all.min.x, self.all.max.y, self.all.min.z),
            Point3::new(self.all.max.x, self.all.min.y, self.all.min.z),
            Point3::new(self.all.min.x, self.all.max.y, self.all.max.z),
            Point3::new(self.all.max.x, self.all.min.y, self.all.max.z),
            Point3::new(self.all.max.x, self.all.max.y, self.all.min.z),
        ];
        bounding(positions.iter().flat_map(|&(tip, axis)| {
            let up = corners
                .iter()
                .map(|c| (*c - tip).dot(axis.get()))
                .fold(top, f64::max);
            [tip, tip + axis.get() * up]
        }))
        .map(|b| widen(b, grow))
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
            sampled(
                &crate::swarf::face_samples(f.face, self.spacing),
                self.spacing,
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

/// A box that holds all of `face`: its untrimmed surface over the whole
/// parameter box on a [`BOX_GRID`] grid, grown by the longest step between
/// neighbouring grid points. The trim is not read, so no part of it a grid
/// misses (a narrow band, a part of a trim) is left out of the box
/// (CodeRabbit on #972). `None` where the surface has no point at all.
fn surface_box(face: &dyn ParametricFace) -> Option<Aabb> {
    let (lo, hi) = face.uv_bounds();
    let n = BOX_GRID - 1;
    let at = |i: usize, j: usize| {
        face.point_normal(crate::surface::Uv {
            u: lo.u + (hi.u - lo.u) * i as f64 / n as f64,
            v: lo.v + (hi.v - lo.v) * j as f64 / n as f64,
        })
        .map(|p| p.point)
    };
    let grid: Vec<Vec<Option<Point3>>> = (0..=n)
        .map(|i| (0..=n).map(|j| at(i, j)).collect())
        .collect();
    let point = |i: usize, j: usize| grid.get(i).and_then(|r| r.get(j)).copied().flatten();
    let mut step = 0.0_f64;
    for i in 0..=n {
        for j in 0..=n {
            if let Some(p) = point(i, j) {
                for q in [point(i + 1, j), point(i, j + 1)].into_iter().flatten() {
                    step = step.max(p.distance(q));
                }
            }
        }
    }
    let b = bounding(grid.iter().flatten().flatten().copied())?;
    Some(widen(b, step))
}

/// A face's samples at `spacing` as the measure keeps them: the points, and
/// how much deeper a cut can be between them than at the nearest one.
fn sampled(
    samples: &[(Point3, openchuck_types::geometry::Vec3, f64)],
    spacing: f64,
) -> (Vec<Point3>, f64) {
    let apart = samples
        .iter()
        .map(|(_, _, apart)| *apart)
        .fold(spacing, f64::max);
    (
        samples.iter().map(|(p, _, _)| *p).collect(),
        apart * std::f64::consts::FRAC_1_SQRT_2,
    )
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::surface::{FaceClosest, FaceRef, RayHit, SurfacePoint, Uv};

    /// A plane at Z `z` over X and Y 0 to 200, trimmed to the band where
    /// `lo < x - y < hi`: with `lo` 0.5 and `hi` 1.5 a band about 0.7 mm
    /// wide along the diagonal, which no point of a 2 mm grid lies on.
    #[derive(Debug)]
    struct Band {
        z: f64,
        lo: f64,
        hi: f64,
        /// Also the square where X and Y are both 10 or less.
        square: bool,
    }

    impl ParametricFace for Band {
        fn uv_bounds(&self) -> (Uv, Uv) {
            (Uv { u: 0.0, v: 0.0 }, Uv { u: 200.0, v: 200.0 })
        }
        fn point_normal(&self, uv: Uv) -> Option<SurfacePoint> {
            ((0.0..=200.0).contains(&uv.u) && (0.0..=200.0).contains(&uv.v)).then(|| SurfacePoint {
                point: Point3::new(uv.u, uv.v, self.z),
                normal: UnitVec3::new(openchuck_types::geometry::Vec3::new(0.0, 0.0, 1.0)).unwrap(),
                position: None,
            })
        }
        fn closest(&self, _: Point3) -> Option<FaceClosest> {
            None
        }
        fn ray_hits(&self, _: Point3, _: UnitVec3, _: f64) -> Vec<RayHit> {
            Vec::new()
        }
        fn contains(&self, uv: Uv) -> bool {
            let d = uv.u - uv.v;
            (d > self.lo && d < self.hi) || (self.square && uv.u <= 10.0 && uv.v <= 10.0)
        }
    }

    fn up() -> UnitVec3 {
        UnitVec3::new(openchuck_types::geometry::Vec3::new(0.0, 0.0, 1.0)).unwrap()
    }

    /// CodeRabbit on #972: a face whose trim a coarse grid misses is kept
    /// and sampled at the measure's own spacing, as before decision 0701
    /// every face was, not taken for an empty face.
    #[test]
    fn req_op_12_a_face_the_coarse_grid_misses_is_still_measured() {
        let band = Band {
            z: 0.0,
            lo: 0.5,
            hi: 1.5,
            square: false,
        };
        // The premise: the coarse grid finds nothing on the band.
        assert!(crate::swarf::face_samples(&band, 2.0).is_empty());
        let faces = vec![FaceRef {
            number: 7,
            face: &band,
        }];
        let beside = Beside::from_faces(faces, 3.0, Allowance::Tolerance).expect("kept");
        assert_eq!(beside.number(0), 7);
        let (points, _) = beside.samples(0).unwrap();
        assert!(!points.is_empty());
        // Near the band, and not well above it.
        let at = |z: f64| Aabb {
            min: Point3::new(99.0, 98.0, z - 1.0),
            max: Point3::new(101.0, 100.0, z + 1.0),
        };
        assert_eq!(beside.near(&at(0.0)), vec![0]);
        assert!(beside.near(&at(50.0)).is_empty());
    }

    /// CodeRabbit on #972: the tool's cutting part goes on up its axis past
    /// the top of the flutes at the cutting radius (`cutter::clearance_at`),
    /// so the reach of a position covers that too, as far as any face goes,
    /// and a face above the flutes within the tool's radius is near.
    #[test]
    fn req_op_12_the_reach_covers_the_tool_above_its_flutes() {
        let band = Band {
            z: 30.0,
            lo: -200.0,
            hi: 200.0,
            square: false,
        };
        let faces = vec![FaceRef {
            number: 2,
            face: &band,
        }];
        let beside = Beside::from_faces(faces, 3.0, Allowance::PassCut).unwrap();
        // The tip at Z 0, flutes 10 mm long: the face 30 mm up the axis.
        let reach = beside
            .reach(&[(Point3::new(50.0, 50.0, 0.0), up())], 10.0, 3.0)
            .unwrap();
        assert!(reach.max.z >= 30.0, "{reach:?}");
        assert_eq!(beside.near(&reach), vec![0]);
        // Below the tip it does not reach.
        let band = Band {
            z: -30.0,
            lo: -200.0,
            hi: 200.0,
            square: false,
        };
        let faces = vec![FaceRef {
            number: 2,
            face: &band,
        }];
        let beside = Beside::from_faces(faces, 3.0, Allowance::PassCut).unwrap();
        let reach = beside
            .reach(&[(Point3::new(50.0, 50.0, 0.0), up())], 10.0, 3.0)
            .unwrap();
        assert!(beside.near(&reach).is_empty(), "{reach:?}");
    }

    /// CodeRabbit on #972: a face the coarse grid sees only in part (a
    /// square near the origin) and misses elsewhere (the band along the
    /// diagonal) is still near an entry beside the part it misses: its box
    /// holds the whole face.
    #[test]
    fn req_op_12_a_face_the_coarse_grid_sees_only_in_part_is_near_all_of_it() {
        let band = Band {
            z: 0.0,
            lo: 0.5,
            hi: 1.5,
            square: true,
        };
        // The premise: the coarse grid sees the square, and nothing of the
        // band past it.
        let coarse = crate::swarf::face_samples(&band, 2.0);
        assert!(!coarse.is_empty());
        assert!(coarse.iter().all(|(p, _, _)| p.x <= 12.0 && p.y <= 12.0));
        let faces = vec![FaceRef {
            number: 4,
            face: &band,
        }];
        let beside = Beside::from_faces(faces, 3.0, Allowance::Tolerance).unwrap();
        let near_band = Aabb {
            min: Point3::new(99.0, 98.0, -1.0),
            max: Point3::new(101.0, 100.0, 1.0),
        };
        assert_eq!(beside.near(&near_band), vec![0]);
    }

    /// CodeRabbit on #972: beside swarf's face set, a face the tool meets
    /// above the top of its flutes, within its cutting radius, is cut there
    /// and refused, as decision 0682 refused it before the reach was
    /// bounded: the measure counts the tool on up its axis at the cutting
    /// radius.
    #[test]
    fn req_op_11_req_op_12_a_face_above_the_flutes_is_refused_beside_swarf() {
        let band = Band {
            z: 30.0,
            lo: -200.0,
            hi: 200.0,
            square: false,
        };
        let faces = vec![FaceRef {
            number: 2,
            face: &band,
        }];
        let beside = Beside::from_faces(faces, 3.0, Allowance::PassCut).unwrap();
        let cutter = openchuck_types::tool::Tool {
            shape: openchuck_types::tool::ToolShape::FlatEnd,
            diameter: openchuck_types::units::Millimetres(6.0),
            flute_length: openchuck_types::units::Millimetres(10.0),
            overall_length: openchuck_types::units::Millimetres(60.0),
            shank_diameter: openchuck_types::units::Millimetres(6.0),
            flute_count: 2,
            material: None,
            centre_cutting: None,
            stick_out: None,
        }
        .profile()
        .unwrap();
        let tip = (Point3::new(50.0, 50.0, 0.0), up());
        let reach = beside.reach(&[tip], cutter.top(), 3.0).unwrap();
        let near = beside.near(&reach);
        assert_eq!(near, vec![0]);
        let cuts = beside.cuts(&near, &cutter, 3.0, tip);
        let depth = cuts.get(&0).copied().unwrap_or(0.0);
        assert!(depth > 1.0, "{depth}");
        // The pass itself does not cut the face: the move is refused.
        let found = super::super::entry::deepest_beside(
            &[cuts],
            &BTreeMap::new(),
            Allowance::PassCut,
            0.01,
            |i| beside.slack(i),
        );
        assert!(matches!(found, Some((0, 0, d, _)) if d > 1.0), "{found:?}");
    }
}
