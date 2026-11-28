// SPDX-License-Identifier: AGPL-3.0-or-later
//! Sections that do not surround the rotary axis, for simultaneous
//! finishing's drive (OP-26; decision record 0363 item 3).
//!
//! Simultaneous finishing finds its drive points with rays from outside the
//! part towards the rotary axis (decision record 0036 item 1). Where the part's
//! section does not surround the axis, such as a crank pin centred off it,
//! most of those rays miss, and the side of the section facing the axis is
//! never met. At such an X the rays are aimed at the section's own centre
//! instead: the middle of the box round the points the rays meet, found
//! twice over from the box before, so that rays aimed at a round pin's
//! centre meet it all round. Every X where every ray towards the axis meets
//! the part keeps the axis, so the drive there is as before.

use openchuck_types::geometry::{Point3, UnitVec3, Vec3};

use crate::parallel;
use crate::surface::SurfaceSource;

/// How far apart the X stations are, in millimetres.
pub(crate) const STATION_LENGTH: f64 = 0.1;

/// How many rays each station casts round the axis (one a degree).
pub(crate) const STATION_RAYS: usize = 360;

/// How many times the centre is found again from the box of the hits of
/// rays aimed at the centre before.
const CENTRE_ROUNDS: usize = 2;

/// An X range of the part and the point the drive rays aim at there.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Zone {
    /// The range, `x0 < x1`.
    pub x0: f64,
    pub x1: f64,
    /// `None` on the axis; otherwise the section's centre `(x, y, z)` at
    /// each station of the zone, in X order.
    pub centres: Option<Vec<(f64, f64, f64)>>,
}

impl Zone {
    /// The centre `(y, z)` the rays aim at, at `x`: the axis, or the
    /// stations' centres interpolated, held at the ends.
    pub fn centre_at(&self, x: f64) -> (f64, f64) {
        let Some(c) = &self.centres else {
            return (0.0, 0.0);
        };
        let i = c.partition_point(|s| s.0 < x);
        match (i.checked_sub(1).and_then(|k| c.get(k)), c.get(i)) {
            (Some(a), Some(b)) if b.0 > a.0 => {
                let s = (x - a.0) / (b.0 - a.0);
                (a.1 + s * (b.1 - a.1), a.2 + s * (b.2 - a.2))
            }
            (Some(a), _) => (a.1, a.2),
            (None, Some(b)) => (b.1, b.2),
            (None, None) => (0.0, 0.0),
        }
    }
}

/// The point where a ray from `outside` mm away from `(cy, cz)`, at polar
/// angle `th` degrees (from +Y towards +Z), towards that centre first meets
/// the surface at `x`.
pub(crate) fn hit_towards(
    surface: &dyn SurfaceSource,
    x: f64,
    centre: (f64, f64),
    th: f64,
    outside: f64,
) -> Option<crate::surface::RayHit> {
    let (s, c) = th.to_radians().sin_cos();
    let origin = Point3::new(x, centre.0 + outside * c, centre.1 + outside * s);
    let direction = UnitVec3::new(Vec3::new(0.0, -c, -s)).ok()?;
    surface.cast_ray(origin, direction, outside)
}

/// How far either side, degrees, a ray that misses is tried again (issue
/// #917, the owner's answer A): a ray that grazes the edge between two faces
/// can miss both, where one a hair to the side meets the part.
const RETRY_ANGLE: f64 = 0.01;

/// The share of a station's rays towards the axis that may miss, after the
/// retry, with the station still on the axis (issue #917, the owner's answer
/// A: the axis is kept where 98% of the rays or more meet the part). A
/// feature off the axis misses far more: a pin beside it most of a turn.
const MOST_MISSED: f64 = 0.02;

/// The centre a station's rays should aim at: `None` when every ray towards
/// the axis meets the part, or nearly every one (98%, each that misses tried
/// again a hair to either side), or none does, or when the section's centre
/// is within `snap` of the axis.
fn station_centre(
    surface: &dyn SurfaceSource,
    x: f64,
    radius: f64,
    snap: f64,
) -> Option<(f64, f64)> {
    let around = |centre: (f64, f64)| -> Vec<Point3> {
        let outside = radius + centre.0.hypot(centre.1) + 1.0;
        (0..STATION_RAYS)
            .filter_map(|k| {
                let th = 360.0 * k as f64 / STATION_RAYS as f64;
                [0.0, RETRY_ANGLE, -RETRY_ANGLE]
                    .iter()
                    .find_map(|d| hit_towards(surface, x, centre, th + d, outside))
                    .map(|h| h.point)
            })
            .collect()
    };
    let hits = around((0.0, 0.0));
    let missed = STATION_RAYS - hits.len();
    if hits.is_empty() || missed as f64 <= MOST_MISSED * STATION_RAYS as f64 {
        return None;
    }
    let mut centre = middle(&hits)?;
    for _ in 0..CENTRE_ROUNDS {
        centre = middle(&around(centre)).unwrap_or(centre);
    }
    (centre.0.hypot(centre.1) > snap).then_some(centre)
}

/// The middle of the box round `points` across X.
fn middle(points: &[Point3]) -> Option<(f64, f64)> {
    let first = points.first()?;
    let (mut y0, mut y1, mut z0, mut z1) = (first.y, first.y, first.z, first.z);
    for p in points {
        y0 = y0.min(p.y);
        y1 = y1.max(p.y);
        z0 = z0.min(p.z);
        z1 = z1.max(p.z);
    }
    Some((0.5 * (y0 + y1), 0.5 * (z0 + z1)))
}

/// The zones of `x0` to `x1`, in X order: runs of stations whose rays aim
/// at the axis or at the section's own centre. One zone on the axis when
/// every station's rays towards the axis meet the part.
pub(crate) fn zones(
    surface: &dyn SurfaceSource,
    x0: f64,
    x1: f64,
    radius: f64,
    snap: f64,
) -> Vec<Zone> {
    let n = ((x1 - x0) / STATION_LENGTH).ceil().max(1.0) as usize;
    let step = (x1 - x0) / n as f64;
    // Stations at the middles of the steps, off the part's end faces.
    let xs: Vec<f64> = (0..n).map(|i| x0 + (i as f64 + 0.5) * step).collect();
    let centres = parallel::map_in_order(&xs, |x| station_centre(surface, *x, radius, snap));
    let mut out: Vec<Zone> = Vec::new();
    for (k, (x, c)) in xs.iter().zip(&centres).enumerate() {
        let off = c.is_some();
        // Between stations, a quarter of a step into the off-axis zone: the
        // middle is where a face across the axis usually is, and a drive
        // sample on a face's plane meets its edge.
        let boundary = if k == 0 {
            x0
        } else if off {
            x - 0.25 * step
        } else {
            x - 0.75 * step
        };
        match out.last_mut() {
            Some(z) if z.centres.is_some() == off => {
                if let (Some(list), Some((cy, cz))) = (&mut z.centres, c) {
                    list.push((*x, *cy, *cz));
                }
            }
            _ => {
                if let Some(z) = out.last_mut() {
                    z.x1 = boundary;
                }
                out.push(Zone {
                    x0: boundary,
                    x1,
                    centres: c.map(|(cy, cz)| vec![(*x, cy, cz)]),
                });
            }
        }
    }
    if out.is_empty() {
        out.push(Zone {
            x0,
            x1,
            centres: None,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]
    use super::*;

    #[test]
    fn req_op_26_a_centre_between_stations_is_interpolated_and_held_at_the_ends() {
        let z = Zone {
            x0: 0.0,
            x1: 10.0,
            centres: Some(vec![(1.0, 0.0, 10.0), (3.0, 2.0, 12.0)]),
        };
        assert_eq!(z.centre_at(2.0), (1.0, 11.0));
        assert_eq!(z.centre_at(0.0), (0.0, 10.0));
        assert_eq!(z.centre_at(9.0), (2.0, 12.0));
        let axis = Zone {
            x0: 0.0,
            x1: 1.0,
            centres: None,
        };
        assert_eq!(axis.centre_at(0.5), (0.0, 0.0));
    }

    #[test]
    fn req_op_26_the_box_middle_of_points() {
        let pts = [
            Point3::new(0.0, -1.0, 2.0),
            Point3::new(0.0, 3.0, 4.0),
            Point3::new(0.0, 1.0, -2.0),
        ];
        assert_eq!(middle(&pts), Some((1.0, 1.0)));
        assert_eq!(middle(&[]), None);
    }

    /// A prism along X whose section is a regular pentagon, of circumradius
    /// `radius` about `centre`, its first corner at polar angle 90°: an
    /// on-axis five-blade section's box middle is off the axis by about a
    /// tenth of its radius, as the blisk's is. A ray whose origin's polar
    /// angle (about the axis) lies in one of `gaps` (degrees) misses, as a
    /// ray grazing a trim edge did (issue #917).
    #[derive(Debug)]
    struct Pentagon {
        centre: (f64, f64),
        radius: f64,
        gaps: Vec<(f64, f64)>,
        mesh: openchuck_types::mesh::TriangleMesh,
    }

    impl Pentagon {
        fn new(centre: (f64, f64), radius: f64, gaps: Vec<(f64, f64)>) -> Self {
            Self {
                centre,
                radius,
                gaps,
                mesh: openchuck_types::mesh::TriangleMesh::default(),
            }
        }

        fn corners(&self) -> Vec<(f64, f64)> {
            (0..5)
                .map(|k| {
                    let a = (90.0 + 72.0 * f64::from(k)).to_radians();
                    (
                        self.centre.0 + self.radius * a.cos(),
                        self.centre.1 + self.radius * a.sin(),
                    )
                })
                .collect()
        }
    }

    impl SurfaceSource for Pentagon {
        fn bounds(&self) -> openchuck_types::geometry::Aabb {
            let r = self.radius + self.centre.0.hypot(self.centre.1);
            openchuck_types::geometry::Aabb {
                min: Point3::new(-100.0, -r, -r),
                max: Point3::new(100.0, r, r),
            }
        }
        fn project(&self, _: Point3) -> Option<crate::surface::SurfacePoint> {
            None
        }
        fn cast_ray(
            &self,
            origin: Point3,
            direction: UnitVec3,
            max_distance: f64,
        ) -> Option<crate::surface::RayHit> {
            let th = origin.z.atan2(origin.y).to_degrees().rem_euclid(360.0);
            if self.gaps.iter().any(|&(lo, hi)| th >= lo && th <= hi) {
                return None;
            }
            let d = direction.get();
            let corners = self.corners();
            let mut best: Option<f64> = None;
            for k in 0..5 {
                let (a, b) = (corners[k], corners[(k + 1) % 5]);
                // origin + t d = a + u (b - a), in (y, z).
                let (ey, ez) = (b.0 - a.0, b.1 - a.1);
                let det = d.y * (-ez) - d.z * (-ey);
                if det.abs() < 1e-12 {
                    continue;
                }
                let (ry, rz) = (a.0 - origin.y, a.1 - origin.z);
                let t = (ry * (-ez) - rz * (-ey)) / det;
                let u = (d.y * rz - d.z * ry) / det;
                if (0.0..=1.0).contains(&u) && t >= 0.0 && t <= max_distance {
                    best = Some(best.map_or(t, |x: f64| x.min(t)));
                }
            }
            let t = best?;
            Some(crate::surface::RayHit {
                distance: t,
                point: origin + d * t,
                normal: UnitVec3::new(-d).ok()?,
                position: None,
            })
        }
        fn tessellation(&self) -> &openchuck_types::mesh::TriangleMesh {
            &self.mesh
        }
        fn drop_tool(
            &self,
            _: &openchuck_types::tool::profile::RevolvedProfile,
            _: &crate::cutter::ToolLine,
        ) -> Option<crate::cutter::Contact> {
            None
        }
    }

    #[test]
    fn req_op_26_req_nfr_06_an_isolated_ray_that_grazes_an_edge_keeps_the_axis() {
        // Issue #917, the owner's answer A: on the STEP blisk five rays, one
        // a blade, grazed a trim edge and missed, and the station was taken
        // off the axis, round its box middle 1.3 mm away. A missed ray is
        // tried again 0.01° either side, so the station keeps the axis.
        let gaps = [8.0, 80.0, 152.0, 224.0, 296.0]
            .iter()
            .map(|&a| (a - 0.001, a + 0.001))
            .collect();
        let blisk = Pentagon::new((0.0, 0.0), 14.0, gaps);
        assert_eq!(station_centre(&blisk, 0.0, 14.0, 0.01), None);
    }

    #[test]
    fn req_op_26_a_section_nearly_all_met_keeps_the_axis() {
        // The owner's 98% rule: a few rays that miss whatever the retry (a
        // 3° gap: 3 or 4 of 360) do not take the station off the axis.
        let blisk = Pentagon::new((0.0, 0.0), 14.0, vec![(100.0, 103.0)]);
        assert_eq!(station_centre(&blisk, 0.0, 14.0, 0.01), None);
        // Over 2% missed (a 9° gap) is still taken round the section's centre.
        let gapped = Pentagon::new((0.0, 0.0), 14.0, vec![(100.0, 109.0)]);
        assert!(station_centre(&gapped, 0.0, 14.0, 0.01).is_some());
    }

    #[test]
    fn req_op_26_an_off_axis_section_is_still_found() {
        // A pin off the axis, which most rays towards the axis miss, keeps
        // its own centre (decision 0363 item 3).
        let pin = Pentagon::new((0.0, 8.0), 3.0, Vec::new());
        let (cy, cz) = station_centre(&pin, 0.0, 12.0, 0.01).unwrap();
        assert!(cy.abs() < 0.5 && (cz - 8.0).abs() < 0.5, "{cy} {cz}");
    }
}
