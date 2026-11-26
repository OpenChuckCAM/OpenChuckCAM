// SPDX-License-Identifier: AGPL-3.0-or-later
//! The tool gap: the smallest signed distance between a tool and a surface
//! (OP-12; decision record SIMUL-EDGE-1, Q2).
//!
//! The tool is its cutting part, the profile revolved about its axis and
//! capped flat at the profile's top. Every profile the tool library makes
//! (flat, ball, bull nose, tapered ball) gives a convex solid; a profile that
//! is not convex is replaced by the cylinder of its cutting radius from the
//! tip to the top, which holds it, so the answer is never less safe. The
//! tool may also be swept along a short straight move without turning: the
//! gap is then the least over every pose on the move.
//!
//! The answer is an interval. `upper` is the signed distance of the tool, at
//! one pose of the move, from a surface point it found: a real value, so the
//! true gap is no larger. `lower` is proven: no surface point lies deeper
//! inside the tool at any pose. Negative means the surface lies inside the
//! tool by that much (the overcut depth).
//!
//! **Why it is proven.** For a convex solid `K` and any unit direction `u`,
//! every point `y` has a signed distance to `K` of at least
//! `u . y - h(u)`, where `h` is `K`'s support function (the plane square to
//! `u` touching `K` holds it on one side). Over the sweep the support grows by
//! at most `max(0, u . v)`. So over a triangle (or a piece of one) the gap is
//! at least the least of that bound at its corners, the bound being linear.
//! The search takes `u` from the nearest point of the tool to the piece's
//! centre and splits the piece with the smallest bound, smallest first, until
//! the smallest bound left is within [`GAP_ACCURACY`] of the best value found
//! (decision record 0255 item 1 makes the same argument for the independent
//! checker in `openchuck_testkit::contact`, which this code does not share).
//!
//! **Determinism (NFR-06).** One query runs on one thread. Triangles are
//! taken in index order, and pieces leave the queue by their bound, then by
//! triangle index, then by the order they were made, so the same query gives
//! the same answer, bit for bit, at any thread count.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use openchuck_types::geometry::{Aabb, Point3, UnitVec3, Vec3};
use openchuck_types::mesh::TriangleMesh;
use openchuck_types::tool::profile::{ProfilePiece, ProfilePoint, RevolvedProfile};

use crate::cutter::ToolLine;

/// How close, in millimetres, the proven `lower` is to the found `upper`
/// when a search finishes within its budget: a tenth of a micrometre, far
/// below any tolerance a toolpath uses and far above rounding.
pub const GAP_ACCURACY: f64 = 1e-4;

/// The most pieces one query splits before it answers with the interval it
/// has reached.
const MAX_PIECES: usize = 200_000;

/// A piece shorter than this along every side is not split again, mm.
const MIN_PIECE: f64 = 1e-9;

/// Steps of the golden-section search for the pose along a sweep. Each step
/// keeps 0.618 of the interval, so 40 steps leave about 4e-9 of the sweep:
/// the pose only sets `upper` and the direction of the bound, never `lower`'s
/// validity.
const SWEEP_STEPS: u32 = 40;

/// The width either side of a turning tool's hinted pose that its search
/// covers, as a share of the span, and the steps it takes there: the pose
/// found only sets `upper` and the bound's direction, never its validity.
const HINT_WIDTH: f64 = 0.125;
const HINT_STEPS: u32 = 14;

/// Turns of the search for a piece's closest pair to the tool
/// ([`Body::closest`]): it only chooses the direction of a bound.
const CLOSEST_STEPS: u32 = 6;

/// A turn about a fixed line, for [`GapQuery::rotating`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rotation {
    /// A point of the line.
    pub point: Point3,
    /// The line's direction.
    pub axis: UnitVec3,
    /// The angle turned through over the whole move, radians, right-handed
    /// about `axis`.
    pub angle: f64,
}

/// What a tool gap query asks (decision record SIMUL-EDGE-1, Q2): see
/// [`SurfaceSource::tool_gap`](crate::SurfaceSource::tool_gap).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GapQuery {
    /// The straight move of the tool's tip, from the pose its line gives;
    /// zero for one pose.
    pub sweep: Vec3,
    /// A turn of the tool about a fixed line along the move, when it
    /// turns: at the fraction `f` of the move the tool is its first pose
    /// shifted by `f` times the sweep, then turned about the line by `f`
    /// times the angle. On a machine that moves X, Y, Z and A together in
    /// proportion this is the move exactly, seen from the part: the line is
    /// the rotary axis and the sweep is the machine's straight move as it
    /// is seen at the first pose. `None` for a tool that does not turn.
    pub rotation: Option<Rotation>,
    /// How far from the tool the surface is looked for, mm: a gap larger
    /// than this gives no answer, and the search stops once it finds the
    /// surface deeper than this inside the tool.
    pub limit: f64,
    /// When set, the search stops as soon as the gap is proven at least this
    /// (mm; negative for an overcut depth a caller accepts), with whatever
    /// interval it has: for a caller that needs only to know whether the
    /// gap is above a limit. When not set, the interval is narrowed to
    /// [`GAP_ACCURACY`].
    pub accept: Option<f64>,
}

impl GapQuery {
    /// One pose, the surface looked for within `limit`.
    pub fn pose(limit: f64) -> Self {
        Self {
            sweep: Vec3::ZERO,
            rotation: None,
            limit,
            accept: None,
        }
    }

    /// The tool swept along `sweep`, the surface looked for within `limit`.
    pub fn swept(sweep: Vec3, limit: f64) -> Self {
        Self {
            sweep,
            rotation: None,
            limit,
            accept: None,
        }
    }

    /// This query, with the tool also turning about a fixed line along the
    /// move (see [`rotation`](Self::rotation)).
    #[must_use]
    pub fn rotating(self, rotation: Rotation) -> Self {
        Self {
            rotation: Some(rotation),
            ..self
        }
    }

    /// This query, ending as soon as the gap is proven at least `gap`.
    #[must_use]
    pub fn accept_above(self, gap: f64) -> Self {
        Self {
            accept: Some(gap),
            ..self
        }
    }
}

/// The smallest signed distance between a tool and a surface, as an interval
/// (decision record SIMUL-EDGE-1, Q2): see [`SurfaceSource::tool_gap`].
///
/// [`SurfaceSource::tool_gap`]: crate::SurfaceSource::tool_gap
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolGap {
    /// Proven: no surface point is nearer the tool than this, or deeper
    /// inside it, at any pose. Negative when the surface may lie inside the
    /// tool by up to that much.
    pub lower: f64,
    /// Found: the signed distance from the tool, at one pose, of the surface
    /// point [`at`](Self::at). The true gap lies between `lower` and this.
    pub upper: f64,
    /// The surface point where `upper` was found.
    pub at: Point3,
}

impl ToolGap {
    /// The deeper of two answers, for a source made of several surfaces:
    /// the smaller `lower` and the smaller `upper` with its point. Ties keep
    /// `self`, so the order the parts are asked in decides them (NFR-06).
    #[must_use]
    pub fn min(self, other: Self) -> Self {
        let lower = self.lower.min(other.lower);
        if other.upper < self.upper {
            Self { lower, ..other }
        } else {
            Self { lower, ..self }
        }
    }
}

/// The cutting part's outline in the tool's half-plane (radius, height from
/// the tip), from the tip round to the top of the axis: the profile's pieces
/// and the cap.
#[derive(Debug, Clone)]
pub(crate) struct Outline {
    pieces: Vec<ProfilePiece>,
    radius: f64,
    top: f64,
    /// Whether the outline is the profile's own (convex) or its bounding
    /// cylinder.
    own: bool,
    lower: Vec<ProfilePiece>,
}

impl Outline {
    pub(crate) fn new(profile: &RevolvedProfile) -> Self {
        let radius = profile.cutting_radius();
        let top = profile.top();
        let tip = ProfilePoint::TIP;
        if profile.is_convex() {
            let mut pieces = profile.pieces().to_vec();
            let end = profile.end();
            if pr(end) > 0.0 {
                pieces.push(ProfilePiece::Line {
                    start: end,
                    end: ProfilePoint::new(0.0, top),
                });
            }
            let lower = profile.pieces().to_vec();
            return Self {
                pieces,
                radius,
                top,
                own: true,
                lower,
            };
        }
        let corner = ProfilePoint::new(radius, 0.0);
        let rim = ProfilePoint::new(radius, top);
        let lower = vec![
            ProfilePiece::Line {
                start: tip,
                end: corner,
            },
            ProfilePiece::Line {
                start: corner,
                end: rim,
            },
        ];
        let mut pieces = lower.clone();
        pieces.push(ProfilePiece::Line {
            start: rim,
            end: ProfilePoint::new(0.0, top),
        });
        Self {
            pieces,
            radius,
            top,
            own: false,
            lower,
        }
    }

    /// Whether `(r, z)`, with `r >= 0`, lies inside the outline.
    fn contains(&self, r: f64, z: f64) -> bool {
        if !(0.0..=self.top).contains(&z) || r > self.radius {
            return false;
        }
        if !self.own {
            return true;
        }
        // The lowest point of the tool at radius r: the outline is convex
        // and monotone, so a point is inside when it is on or above it.
        let mut lowest = 0.0;
        for p in &self.lower {
            let (s, e) = (p.start(), p.end());
            if pr(e) - pr(s) > 1e-12 && r <= pr(e) {
                lowest = p.height_at(r.max(pr(s)));
                return z >= lowest;
            }
            lowest = pz(e);
        }
        z >= lowest
    }

    /// The signed distance of `(r, z)` (with `r >= 0`) from the outline and
    /// the outward unit normal (radial, axial) of the boundary at its nearest
    /// point: negative inside.
    fn signed(&self, r: f64, z: f64) -> (f64, (f64, f64)) {
        let mut best = f64::INFINITY;
        let mut near = (r, z);
        let mut piece_normal = (0.0, -1.0);
        for p in &self.pieces {
            let (q, n) = nearest_on_piece(p, r, z);
            let d = (r - q.0).hypot(z - q.1);
            if d < best {
                best = d;
                near = q;
                piece_normal = n;
            }
        }
        let inside = self.contains(r, z);
        let (dr, dz) = (r - near.0, z - near.1);
        let len = dr.hypot(dz);
        let normal = if len > 1e-12 {
            if inside {
                (-dr / len, -dz / len)
            } else {
                (dr / len, dz / len)
            }
        } else {
            piece_normal
        };
        (if inside { -best } else { best }, normal)
    }

    /// The lowest height `z0` from which the capsule of radius `wide` about
    /// the axis up to the top holds the whole outline: no higher than any
    /// point's `z + sqrt(wide^2 - r^2)` (below `z0` a point must be within
    /// `wide` of the segment's end). Along a straight piece that is concave,
    /// so its ends bound it; a convex arc is held whole when its circle,
    /// radius `c` about `(rc, zc)`, is, which is from
    /// `zc + sqrt((wide - c)^2 - rc^2)` down; otherwise the capsule starts at
    /// the tip. A ball end's capsule starts at its centre, so it is the
    /// ball itself there.
    fn capsule_base(&self, low: f64, high: f64, wide: f64) -> f64 {
        let room = |r: f64| (wide * wide - r * r).max(0.0).sqrt();
        let mut base = high;
        for p in &self.pieces {
            match *p {
                ProfilePiece::Arc {
                    centre, radius, ..
                } => {
                    let free = wide - radius;
                    base = base.min(if free >= pr(centre) {
                        pz(centre) + (free * free - pr(centre) * pr(centre)).max(0.0).sqrt()
                    } else {
                        low
                    });
                }
                _ => {
                    for q in [p.start(), p.end()] {
                        base = base.min(pz(q) + room(pr(q)));
                    }
                }
            }
        }
        base.clamp(low, high)
    }

    /// The support of the outline (mirrored about the axis) in the
    /// half-plane direction `(a, b)` with `a >= 0`: the largest
    /// `a * r + b * z` over the outline.
    fn support(&self, a: f64, b: f64) -> f64 {
        let mut best = 0.0_f64; // the tip, (0, 0)
        for p in &self.pieces {
            for q in [p.start(), p.end()] {
                best = best.max(a * pr(q) + b * pz(q));
            }
            if let ProfilePiece::Arc {
                start,
                end,
                centre,
                radius,
                convex: true,
            } = *p
            {
                let len = a.hypot(b);
                if len > 0.0 {
                    let angle = b.atan2(a);
                    let (ta, tb) = arc_range(start, end, centre);
                    if angle >= ta && angle <= tb {
                        best = best.max(a * pr(centre) + b * pz(centre) + radius * len);
                    }
                }
            }
        }
        best
    }
}

/// The angles of a convex arc's ends about its centre, lower first.
fn arc_range(start: ProfilePoint, end: ProfilePoint, centre: ProfilePoint) -> (f64, f64) {
    let ta = (pz(start) - pz(centre)).atan2(pr(start) - pr(centre));
    let tb = (pz(end) - pz(centre)).atan2(pr(end) - pr(centre));
    (ta.min(tb), ta.max(tb))
}

/// The nearest point of a piece to `(r, z)` and the piece's outward normal
/// there.
fn nearest_on_piece(p: &ProfilePiece, r: f64, z: f64) -> ((f64, f64), (f64, f64)) {
    match *p {
        ProfilePiece::Line { start, end } => {
            let (dr, dz) = (pr(end) - pr(start), pz(end) - pz(start));
            let len2 = dr * dr + dz * dz;
            let t = if len2 > 0.0 {
                (((r - pr(start)) * dr + (z - pz(start)) * dz) / len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let len = len2.sqrt();
            let normal = if len > 0.0 {
                (dz / len, -dr / len)
            } else {
                (0.0, -1.0)
            };
            ((pr(start) + t * dr, pz(start) + t * dz), normal)
        }
        ProfilePiece::Arc {
            start,
            end,
            centre,
            radius,
            convex,
        } => {
            let (ta, tb) = if convex {
                arc_range(start, end, centre)
            } else {
                // Not reached: a profile with a concave arc is not convex and
                // is replaced by its bounding cylinder. Kept as a chord.
                let q = ((pr(start) + pr(end)) * 0.5, (pz(start) + pz(end)) * 0.5);
                return (q, (0.0, -1.0));
            };
            let angle = (z - pz(centre)).atan2(r - pr(centre)).clamp(ta, tb);
            let (c, s) = (angle.cos(), angle.sin());
            ((pr(centre) + radius * c, pz(centre) + radius * s), (c, s))
        }
    }
}

/// The tool at its first pose and its move: a straight shift `sweep`, and a
/// turn about a fixed line (see [`GapQuery::rotation`]).
///
/// The search works in the tool's frame, the frame of its first pose. There
/// the tool stands still and each surface point moves: at the fraction `f`
/// a point `p` is at `R(-f θ) p - f Δ`, `R` the turn about the line and `Δ`
/// the sweep. Along a direction `u` fixed in that frame, the point's
/// distance from the plane touching the tool is a constant plus a cosine in
/// `f θ` less `f (u . Δ)`, whose least over an interval is found exactly
/// ([`least`]).
#[derive(Debug, Clone)]
pub(crate) struct Body<'a> {
    outline: &'a Outline,
    tip: Point3,
    axis: Vec3,
    sweep: Vec3,
    /// A point of the line turned about, and its unit direction.
    pivot: (Point3, Vec3),
    /// The turn over the whole move, radians; zero without one.
    angle: f64,
    /// The capsule holding the tool: the height its segment starts at (see
    /// [`Outline::capsule_base`]), the outline's highest point and widest
    /// radius.
    reach: (f64, f64, f64),
    /// The axis from the height of the outline's widest radius above its
    /// lowest point (a ball's centre) to its top: the bound by the plane
    /// square to the direction from it is cheap and near exact for a round
    /// end.
    spine: (Point3, Point3),
}

impl<'a> Body<'a> {
    pub(crate) fn new(outline: &'a Outline, line: &ToolLine, query: &GapQuery) -> Self {
        let axis = line.axis.get();
        let sweep = if query.sweep.is_finite() {
            query.sweep
        } else {
            Vec3::ZERO
        };
        let (pivot, angle) = match query.rotation {
            Some(r) if r.angle.is_finite() && r.point.is_finite() && r.angle != 0.0 => {
                ((r.point, r.axis.get()), r.angle)
            }
            _ => ((line.origin, axis), 0.0),
        };
        let (low, high) = (-outline.support(0.0, -1.0), outline.support(0.0, 1.0));
        let wide = outline.support(1.0, 0.0);
        Self {
            outline,
            tip: line.origin,
            axis,
            sweep,
            pivot,
            angle,
            reach: (outline.capsule_base(low, high, wide), high, wide),
            spine: (
                line.origin + axis * (low + wide).min(high),
                line.origin + axis * high,
            ),
        }
    }

    /// Whether the tool turns along the move.
    fn turns(&self) -> bool {
        self.angle != 0.0
    }

    /// The point `p` turned about the line by `a` radians.
    fn turned(&self, p: Point3, a: f64) -> Point3 {
        if a == 0.0 {
            return p;
        }
        let (c, n) = self.pivot;
        c + turn_vector(p - c, n, a)
    }

    /// Where the surface point `x` is in the tool's frame at the fraction
    /// `f` of the move.
    fn frame(&self, x: Point3, f: f64) -> Point3 {
        self.turned(x, -f * self.angle) + self.sweep * (-f)
    }

    /// How far a point `p` of the surface moves in the tool's frame over
    /// the fractions `s0` to `s1`, at most, from where it is at their
    /// middle: the shift's part plus the arc of its turn.
    fn moved(&self, p: Point3, s0: f64, s1: f64) -> f64 {
        let (c, n) = self.pivot;
        let q = p - c;
        let off = (q - n * q.dot(n)).length();
        0.5 * (s1 - s0) * (self.sweep.length() + self.angle.abs() * off)
    }

    /// The proven least signed distance from the tool over the points `t`
    /// (or their hull) over the fractions `s0` to `s1`, by the plane square
    /// to `u` (in the tool's frame) touching the tool: for each point, the
    /// exact least of its distance from that plane as it moves, the least
    /// over the hull being at a corner at every fraction.
    fn lower(&self, t: &[Point3], u: Vec3, s0: f64, s1: f64) -> f64 {
        let along = self.axis.dot(u);
        let h = u.dot(self.tip - Point3::ORIGIN)
            + self.outline.support((u - self.axis * along).length(), along);
        let k = u.dot(self.sweep);
        let (c, n) = self.pivot;
        let un = n * u.dot(n);
        let up = u - un;
        let uq = n.cross(up);
        let base = u.dot(c - Point3::ORIGIN);
        t.iter()
            .map(|p| {
                let q = *p - c;
                least(
                    base + un.dot(q),
                    up.dot(q),
                    uq.dot(q),
                    self.angle,
                    k,
                    (s0, s1),
                )
            })
            .fold(f64::INFINITY, f64::min)
            - h
    }

    /// A proven least signed distance over the triangle `t` over the
    /// fractions `s0` to `s1`, by the capsule holding the tool: the exact
    /// distance from its segment to the triangle where it is at the middle
    /// fraction, less its radius and less how far the triangle moves.
    fn far_over(&self, t: &[Point3; 3], s0: f64, s1: f64) -> f64 {
        let (low, high, wide) = self.reach;
        let fm = 0.5 * (s0 + s1);
        let moved = t
            .iter()
            .map(|p| self.moved(*p, s0, s1))
            .fold(0.0, f64::max);
        let at = t.map(|p| self.frame(p, fm));
        segment_to_triangle(self.tip + self.axis * low, self.tip + self.axis * high, &at)
            - wide
            - moved
    }

    /// The same bound over the whole move.
    pub(crate) fn far_from(&self, t: &[Point3; 3]) -> f64 {
        self.far_over(t, 0.0, 1.0)
    }

    /// A proven least signed distance over the points `t`, centred at
    /// `centre`, over the fractions `s0` to `s1`, by the plane square to
    /// the direction from the spine to the centre at the middle fraction.
    fn facing(&self, t: &[Point3], centre: Point3, s0: f64, s1: f64) -> f64 {
        let (a, b) = self.spine;
        let centre = self.frame(centre, 0.5 * (s0 + s1));
        let d = b - a;
        let length = d.dot(d);
        let f = if length > 0.0 {
            ((centre - a).dot(d) / length).clamp(0.0, 1.0)
        } else {
            0.0
        };
        UnitVec3::new(centre - (a + d * f))
            .map_or(f64::NEG_INFINITY, |u| self.lower(t, u.get(), s0, s1))
    }

    /// Whether surface in the box `b` may lie within `limit` of the tool at
    /// some pose: false only where a proven bound puts all of it farther.
    /// The box is tested by the capsule, then by the plane facing it from
    /// the spine (its least value over the box is at a corner).
    pub(crate) fn near(&self, b: &Aabb, limit: f64) -> bool {
        let (low, high, wide) = self.reach;
        let centre = midpoint(b.min, b.max);
        let reach = 0.5 * b.min.distance(b.max);
        let (c, n) = self.pivot;
        let q = centre - c;
        let off = (q - n * q.dot(n)).length() + reach;
        let moved = 0.5 * (self.sweep.length() + self.angle.abs() * off);
        let at = self.frame(centre, 0.5);
        if to_segment(at, self.tip + self.axis * low, self.tip + self.axis * high)
            - reach
            - wide
            - moved
            > limit
        {
            return false;
        }
        let corners = [
            b.min,
            Point3::new(b.max.x, b.min.y, b.min.z),
            Point3::new(b.min.x, b.max.y, b.min.z),
            Point3::new(b.max.x, b.max.y, b.min.z),
            Point3::new(b.min.x, b.min.y, b.max.z),
            Point3::new(b.max.x, b.min.y, b.max.z),
            Point3::new(b.min.x, b.max.y, b.max.z),
            b.max,
        ];
        self.facing(&corners, centre, 0.0, 1.0) <= limit
    }

    /// A box holding every pose of the tool, grown by `margin`.
    pub(crate) fn bounds(&self, margin: f64) -> Aabb {
        let top = self.tip + self.axis * self.outline.top;
        let ends = [self.tip, top, self.tip + self.sweep, top + self.sweep];
        let grow = self.outline.radius + margin.max(0.0);
        let mut min = ends[0];
        let mut max = ends[0];
        for p in ends {
            min = Point3::new(min.x.min(p.x), min.y.min(p.y), min.z.min(p.z));
            max = Point3::new(max.x.max(p.x), max.y.max(p.y), max.z.max(p.z));
        }
        let (min, max) = (
            Point3::new(min.x - grow, min.y - grow, min.z - grow),
            Point3::new(max.x + grow, max.y + grow, max.z + grow),
        );
        // A turn moves each point by at most its distance from the line
        // times the angle.
        let (c, _) = self.pivot;
        let far = [
            min,
            Point3::new(max.x, min.y, min.z),
            Point3::new(min.x, max.y, min.z),
            Point3::new(max.x, max.y, min.z),
            Point3::new(min.x, min.y, max.z),
            Point3::new(max.x, min.y, max.z),
            Point3::new(min.x, max.y, max.z),
            max,
        ]
        .iter()
        .map(|p| p.distance(c))
        .fold(0.0, f64::max);
        let more = self.angle.abs() * far;
        Aabb {
            min: Point3::new(min.x - more, min.y - more, min.z - more),
            max: Point3::new(max.x + more, max.y + more, max.z + more),
        }
    }

    /// The signed distance of `x` from the tool at the fraction `f` of the
    /// move, and the outward unit normal of the tool there, in the tool's
    /// frame.
    fn at_pose(&self, x: Point3, f: f64) -> (f64, Vec3) {
        self.at_first(self.frame(x, f))
    }

    /// The signed distance of the point `y` of the tool's frame from the
    /// tool at its first pose, and the outward unit normal there.
    fn at_first(&self, y: Point3) -> (f64, Vec3) {
        let d = y - self.tip;
        let z = d.dot(self.axis);
        let radial = d - self.axis * z;
        let r = radial.length();
        let w = if r > 1e-12 {
            radial * (1.0 / r)
        } else {
            across(self.axis)
        };
        let (dist, (nr, nz)) = self.outline.signed(r, z);
        (dist, w * nr + self.axis * nz)
    }

    /// The least signed distance of `x` from the tool over the fractions
    /// `s0` to `s1`, as found (an upper bound of the true least), the
    /// normal there, how far above the true least it may be, and the
    /// fraction found. Without a turn the distance is convex along the
    /// move, so a golden-section search brackets the least, and the slack is
    /// the move's length times the last bracket; with one it is not, so the
    /// search starts from `hint` (where a larger piece holding this one
    /// found its pose, when known), searches near it, and gives no slack.
    fn upper(&self, x: Point3, s0: f64, s1: f64, hint: f64) -> (f64, Vec3, f64, f64) {
        let length = self.sweep.length();
        if length <= 0.0 && !self.turns() {
            let (d, n) = self.at_pose(x, 0.0);
            return (d, n, 0.0, 0.0);
        }
        let g = 0.5 * (5.0_f64.sqrt() - 1.0);
        let (mut a, mut b, steps) = if self.turns() && hint.is_finite() {
            let h = hint.clamp(s0, s1);
            let w = HINT_WIDTH * (s1 - s0);
            ((h - w).max(s0), (h + w).min(s1), HINT_STEPS)
        } else {
            (s0, s1, SWEEP_STEPS)
        };
        let mut c = b - g * (b - a);
        let mut d = a + g * (b - a);
        let mut fc = self.at_pose(x, c);
        let mut fd = self.at_pose(x, d);
        for _ in 0..steps {
            if fc.0 <= fd.0 {
                b = d;
                d = c;
                fd = fc;
                c = b - g * (b - a);
                fc = self.at_pose(x, c);
            } else {
                a = c;
                c = d;
                fc = fd;
                d = a + g * (b - a);
                fd = self.at_pose(x, d);
            }
        }
        let mut best = if fc.0 <= fd.0 { (fc, c) } else { (fd, d) };
        for s in [s0, s1] {
            let end = self.at_pose(x, s);
            if end.0 < best.0.0 {
                best = (end, s);
            }
        }
        let slack = if self.turns() {
            f64::INFINITY
        } else {
            length * (b - a)
        };
        (best.0.0, best.0.1, slack, best.1)
    }

    /// Where the point `y` of the tool's frame at the fraction `f` is on the
    /// surface: the inverse of [`frame`](Self::frame).
    fn unframe(&self, y: Point3, f: f64) -> Point3 {
        self.turned(y + self.sweep * f, f * self.angle)
    }

    /// The closest pair of the tool and the triangle `t` at the fraction
    /// `f`, approached by turns: the tool's nearest boundary point to a
    /// point of the triangle, then the triangle's closest point to that,
    /// from `start` ([`CLOSEST_STEPS`] times; for two convex sets each turn
    /// comes nearer). The last point of the triangle (on the surface), its
    /// signed distance from the tool and the tool's normal there: the
    /// direction of the plane that separates the two best.
    fn closest(&self, t: &[Point3; 3], f: f64, start: Point3) -> (Point3, f64, Vec3) {
        let tri = t.map(|p| self.frame(p, f));
        let mut y = self.frame(start, f);
        let (mut d, mut n) = self.at_first(y);
        for _ in 0..CLOSEST_STEPS {
            let next = crate::surface::closest_on_triangle(y + n * (-d), &tri);
            let (nd, nn) = self.at_first(next);
            if nd >= d {
                break;
            }
            (y, d, n) = (next, nd, nn);
        }
        (self.unframe(y, f), d, n)
    }

    /// A surface direction `m` (fixed to the surface) as seen in the tool's
    /// frame at the fraction `f`.
    fn direction(&self, m: Vec3, f: f64) -> Vec3 {
        if self.turns() {
            turn_vector(m, self.pivot.1, -f * self.angle)
        } else {
            m
        }
    }

    /// The turn over the fractions `s0` to `s1`, radians.
    fn turn_over(&self, s0: f64, s1: f64) -> f64 {
        self.angle.abs() * (s1 - s0)
    }
}

/// The vector `v` turned by `a` radians about the unit vector `n`
/// (Rodrigues).
fn turn_vector(v: Vec3, n: Vec3, a: f64) -> Vec3 {
    let along = n * v.dot(n);
    let (s, c) = a.sin_cos();
    along + (v - along) * c + n.cross(v) * s
}

/// The exact least of `g(f) = k0 + x cos(f θ) + y sin(f θ) - f k` over `f`
/// from `span.0` to `span.1`: at an end, or where `g' = 0`, that is where
/// `sin(f θ - φ) = -k / (θ ρ)` with `x = ρ cos φ`, `y = ρ sin φ`.
fn least(k0: f64, x: f64, y: f64, theta: f64, k: f64, span: (f64, f64)) -> f64 {
    let g = |f: f64| {
        let (s, c) = (f * theta).sin_cos();
        k0 + x * c + y * s - f * k
    };
    let mut best = g(span.0).min(g(span.1));
    let rho = x.hypot(y);
    if theta != 0.0 && rho > 0.0 {
        let r = -k / (theta * rho);
        if r.abs() <= 1.0 {
            let phi = y.atan2(x);
            let beta = r.asin();
            let (a0, a1) = (span.0 * theta, span.1 * theta);
            let (lo, hi) = (a0.min(a1), a0.max(a1));
            let tau = std::f64::consts::TAU;
            for start in [phi + beta, phi + std::f64::consts::PI - beta] {
                let mut a = start + tau * ((lo - start) / tau).ceil();
                while a <= hi {
                    best = best.min(g(a / theta));
                    a += tau;
                }
            }
        }
    }
    best
}

/// The distance from `p` to the segment from `a` to `b`.
fn to_segment(p: Point3, a: Point3, b: Point3) -> f64 {
    let d = b - a;
    let length = d.dot(d);
    let s = if length > 0.0 {
        ((p - a).dot(d) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.distance(a + d * s)
}

/// The least distance between the segment from `a` to `b` and the triangle
/// `t`: zero where the segment passes through it, else the least of its
/// ends' distances to the triangle and its distances to the triangle's
/// sides.
fn segment_to_triangle(a: Point3, b: Point3, t: &[Point3; 3]) -> f64 {
    let normal = (t[1] - t[0]).cross(t[2] - t[0]);
    let (da, db) = (normal.dot(a - t[0]), normal.dot(b - t[0]));
    if (da <= 0.0) != (db <= 0.0) || (da == 0.0 && db == 0.0) {
        let f = if da == db { 0.0 } else { da / (da - db) };
        let x = a + (b - a) * f;
        // Inside the triangle when the closest point of it is the point (a
        // hair outside counts as inside, the safe side).
        if crate::surface::closest_on_triangle(x, t).distance(x) <= 1e-9 * (1.0 + x.distance(t[0])) {
            return 0.0;
        }
    }
    let ends = crate::surface::closest_on_triangle(a, t)
        .distance(a)
        .min(crate::surface::closest_on_triangle(b, t).distance(b));
    [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])]
        .iter()
        .map(|(p, q)| segment_to_segment(a, b, *p, *q))
        .fold(ends, f64::min)
}

/// The least distance between the segments `p1 q1` and `p2 q2` (Ericson,
/// Real-Time Collision Detection, 5.1.9).
fn segment_to_segment(p1: Point3, q1: Point3, p2: Point3, q2: Point3) -> f64 {
    let (d1, d2, r) = (q1 - p1, q2 - p2, p1 - p2);
    let (a, e, f) = (d1.dot(d1), d2.dot(d2), d2.dot(r));
    let (s, t) = if a <= f64::EPSILON && e <= f64::EPSILON {
        (0.0, 0.0)
    } else if a <= f64::EPSILON {
        (0.0, (f / e).clamp(0.0, 1.0))
    } else {
        let c = d1.dot(r);
        if e <= f64::EPSILON {
            ((-c / a).clamp(0.0, 1.0), 0.0)
        } else {
            let b = d1.dot(d2);
            let denom = a * e - b * b;
            let mut s = if denom > 0.0 {
                ((b * f - c * e) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let mut t = (b * s + f) / e;
            if t < 0.0 {
                t = 0.0;
                s = (-c / a).clamp(0.0, 1.0);
            } else if t > 1.0 {
                t = 1.0;
                s = ((b - c) / a).clamp(0.0, 1.0);
            }
            (s, t)
        }
    };
    (p1 + d1 * s).distance(p2 + d2 * t)
}

/// A direction square to `axis`, for a point on the axis.
fn across(axis: Vec3) -> Vec3 {
    let pick = if axis.x.abs() < 0.9 {
        Vec3::new(1.0, 0.0, 0.0)
    } else {
        Vec3::new(0.0, 1.0, 0.0)
    };
    UnitVec3::new(pick - axis * pick.dot(axis)).map_or(Vec3::new(0.0, 0.0, 1.0), UnitVec3::get)
}

/// One piece of a triangle, over one part of the move, in the search queue.
#[derive(Debug, Clone, Copy)]
struct Piece {
    lower: f64,
    /// The part of the move, as fractions.
    span: (f64, f64),
    /// The pose (a fraction) where the piece, or the piece it was split
    /// from, was measured closest; NaN before any.
    hint: f64,
    /// Whether the piece's centre has been measured and its bound narrowed
    /// by it; a piece is queued first with its plane's bound only.
    measured: bool,
    triangle: u32,
    order: u64,
    corners: [Point3; 3],
}

impl PartialEq for Piece {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Piece {}

impl PartialOrd for Piece {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Piece {
    // A max-heap: the smallest bound comes out first, then the lowest
    // triangle, then the earliest made.
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .lower
            .total_cmp(&self.lower)
            .then_with(|| other.triangle.cmp(&self.triangle))
            .then_with(|| other.order.cmp(&self.order))
    }
}

/// The tool gap over `triangles` (index and corners, in index order), within
/// `limit`: `None` when no surface lies within `limit` of the tool. The
/// search stops early once it finds the surface deeper than `limit` inside
/// the tool; the interval then holds the true gap but may be wider than
/// [`GAP_ACCURACY`].
pub(crate) fn search(
    body: &Body<'_>,
    triangles: impl IntoIterator<Item = (u32, [Point3; 3])>,
    limit: f64,
    accept: Option<f64>,
) -> Option<ToolGap> {
    let accept = accept.filter(|a| a.is_finite()).unwrap_or(f64::INFINITY);
    let mut state = Search {
        body,
        queue: BinaryHeap::new(),
        order: 0,
        best: None,
    };
    for (i, t) in triangles {
        if t.iter().all(|p| p.is_finite()) {
            state.push(t, i, (0.0, 1.0), f64::NAN);
        }
    }
    let mut pieces = 0_usize;
    let close = GAP_ACCURACY;
    let lower = loop {
        let found = state.best.map_or(f64::INFINITY, |b| b.0);
        let Some(piece) = state.queue.pop() else {
            break found;
        };
        if piece.lower >= found - close
            || piece.lower >= accept
            || piece.lower > limit
            || found < -limit
        {
            if state.best.is_none() && piece.lower <= limit {
                state.measure(piece);
            }
            break piece.lower.min(found);
        }
        if !piece.measured {
            let piece = state.measure(piece);
            state.queue.push(piece);
            continue;
        }
        let (a, b, c, len) = longest_side(&piece.corners);
        // Over a part of the move the piece turns in the tool's frame, so
        // one plane can bound it only as well as about the tool's radius
        // and the piece's size times that turn allow: where that could
        // explain half of what the piece's bound lacks, the part of the
        // move is halved; otherwise the piece is.
        let (s0, s1) = piece.span;
        let slack = body.turn_over(s0, s1) * (body.outline.radius + len);
        let split_move = 2.0 * slack >= found - piece.lower;
        if (len < MIN_PIECE && !split_move) || pieces >= MAX_PIECES {
            break piece.lower.min(found);
        }
        pieces += 2;
        if split_move {
            let sm = 0.5 * (s0 + s1);
            state.push(piece.corners, piece.triangle, (s0, sm), piece.hint);
            state.push(piece.corners, piece.triangle, (sm, s1), piece.hint);
        } else {
            let mid = midpoint(a, b);
            state.push([a, mid, c], piece.triangle, piece.span, piece.hint);
            state.push([mid, b, c], piece.triangle, piece.span, piece.hint);
        }
    };
    let (upper, at) = state.best?;
    (lower <= limit).then_some(ToolGap {
        lower: lower.min(upper),
        upper,
        at,
    })
}


/// The state of one search: the queue of pieces and the best value found.
struct Search<'a, 'b> {
    body: &'a Body<'b>,
    queue: BinaryHeap<Piece>,
    order: u64,
    best: Option<(f64, Point3)>,
}

impl Search<'_, '_> {
    /// Bounds a piece by its plane, by the capsule holding the tool and by
    /// the plane facing it from the tool's spine, and queues it. The piece lies in its triangle's plane: the gap is at
    /// least the gap between the tool and that plane, from either side,
    /// which is exact at once where the tool rests on the triangle's face.
    /// Neither costs a distance to the tool.
    fn push(&mut self, corners: [Point3; 3], triangle: u32, span: (f64, f64), hint: f64) {
        let (a, b, c) = (corners[0], corners[1], corners[2]);
        let (s0, s1) = span;
        let centre = centroid(&corners);
        let far = self
            .body
            .far_over(&corners, s0, s1)
            .max(self.body.facing(&corners, centre, s0, s1));
        let lower = UnitVec3::new((b - a).cross(c - a)).map_or(far, |m| {
            let m = self.body.direction(m.get(), 0.5 * (s0 + s1));
            self.body
                .lower(&corners, m, s0, s1)
                .max(self.body.lower(&corners, -m, s0, s1))
                .max(far)
        });
        self.order += 1;
        self.queue.push(Piece {
            lower,
            span,
            hint,
            measured: false,
            triangle,
            order: self.order,
            corners,
        });
    }

    /// Measures a piece's centre, notes its value, and narrows the piece's
    /// bound by the better of two more proven ones: the plane square to the
    /// normal at the centre's nearest tool point, and the centre's least
    /// distance less the piece's reach from it (the distance changes no
    /// faster than the point moves) and less the search's own slack.
    fn measure(&mut self, piece: Piece) -> Piece {
        let centre = centroid(&piece.corners);
        let (s0, s1) = piece.span;
        let (value, normal, slack, at) = self.body.upper(centre, s0, s1, piece.hint);
        if self.best.is_none_or(|(b, _)| value < b) {
            self.best = Some((value, centre));
        }
        let reach = piece
            .corners
            .iter()
            .map(|p| p.distance(centre))
            .fold(0.0, f64::max);
        // The closest pair at the pose found gives the plane that best
        // separates the piece from the tool there, and a nearer point.
        let (near, gap, across) = self.body.closest(&piece.corners, at, centre);
        if gap < value && self.best.is_none_or(|(b, _)| gap < b) {
            self.best = Some((gap, near));
        }
        let lower = piece
            .lower
            .max(self.body.lower(&piece.corners, normal, s0, s1))
            .max(self.body.lower(&piece.corners, across, s0, s1))
            .max(value - slack - reach);
        Piece {
            lower,
            measured: true,
            hint: at,
            ..piece
        }
    }
}

fn centroid(t: &[Point3; 3]) -> Point3 {
    Point3::new(
        (t[0].x + t[1].x + t[2].x) / 3.0,
        (t[0].y + t[1].y + t[2].y) / 3.0,
        (t[0].z + t[1].z + t[2].z) / 3.0,
    )
}

fn midpoint(a: Point3, b: Point3) -> Point3 {
    Point3::new(0.5 * (a.x + b.x), 0.5 * (a.y + b.y), 0.5 * (a.z + b.z))
}

/// The corners of `t` reordered so that `a` to `b` is its longest side, with
/// that side's length.
fn longest_side(t: &[Point3; 3]) -> (Point3, Point3, Point3, f64) {
    let [p, q, r] = *t;
    let sides = [(p, q, r), (q, r, p), (r, p, q)];
    let mut best = sides[0];
    let mut len = p.distance(q);
    for s in &sides[1..] {
        let l = s.0.distance(s.1);
        if l > len {
            len = l;
            best = *s;
        }
    }
    (best.0, best.1, best.2, len)
}

/// Whether two boxes overlap.
pub(crate) fn overlaps(a: &Aabb, b: &Aabb) -> bool {
    a.min.x <= b.max.x
        && b.min.x <= a.max.x
        && a.min.y <= b.max.y
        && b.min.y <= a.max.y
        && a.min.z <= b.max.z
        && b.min.z <= a.max.z
}

/// The tool gap over every triangle of `mesh`, without a search structure:
/// the answer of [`SurfaceSource::tool_gap`](crate::SurfaceSource::tool_gap)
/// for a source that has none of its own.
pub(crate) fn on_mesh(
    mesh: &TriangleMesh,
    profile: &RevolvedProfile,
    line: &ToolLine,
    query: &GapQuery,
) -> Option<ToolGap> {
    if !line.origin.is_finite() || !query.limit.is_finite() {
        return None;
    }
    let outline = Outline::new(profile);
    let body = Body::new(&outline, line, query);
    let near = body.bounds(query.limit);
    let triangles = (0..mesh.triangles.len()).filter_map(|i| {
        let t = mesh.triangle(i)?;
        let index = u32::try_from(i).ok()?;
        let b = triangle_bounds(&t);
        (overlaps(&near, &b) && body.near(&b, query.limit) && body.far_from(&t) <= query.limit)
            .then_some((index, t))
    });
    search(&body, triangles, query.limit, query.accept)
}

/// The box of a triangle.
pub(crate) fn triangle_bounds(t: &[Point3; 3]) -> Aabb {
    Aabb {
        min: Point3::new(
            t[0].x.min(t[1].x).min(t[2].x),
            t[0].y.min(t[1].y).min(t[2].y),
            t[0].z.min(t[1].z).min(t[2].z),
        ),
        max: Point3::new(
            t[0].x.max(t[1].x).max(t[2].x),
            t[0].y.max(t[1].y).max(t[2].y),
            t[0].z.max(t[1].z).max(t[2].z),
        ),
    }
}

/// A profile point's radius.
fn pr(p: ProfilePoint) -> f64 {
    p.radius.0
}

/// A profile point's height.
fn pz(p: ProfilePoint) -> f64 {
    p.height.0
}
