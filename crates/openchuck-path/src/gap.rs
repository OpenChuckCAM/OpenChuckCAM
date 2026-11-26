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

/// What a tool gap query asks (decision record SIMUL-EDGE-1, Q2): see
/// [`SurfaceSource::tool_gap`](crate::SurfaceSource::tool_gap).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GapQuery {
    /// The straight move of the tool's tip, from the pose its line gives;
    /// zero for one pose.
    pub sweep: Vec3,
    /// The tool's axis at the end of the sweep, when the tool also turns
    /// along it; `None` for a tool that keeps its line's axis. A turning
    /// tool moves rigidly: at the fraction `s` of the move its tip is that
    /// far along the sweep and its axis is the normalised blend
    /// `(1 - s) a0 + s a1` of the two axes, so it turns in their plane.
    /// The answer is for those poses. A move along which the tool turns
    /// another way (on a rotary axis, each point on an arc about that axis)
    /// lies within a small distance of them, which the caller allows for.
    pub end_axis: Option<UnitVec3>,
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
            end_axis: None,
            limit,
            accept: None,
        }
    }

    /// The tool swept along `sweep`, the surface looked for within `limit`.
    pub fn swept(sweep: Vec3, limit: f64) -> Self {
        Self {
            sweep,
            end_axis: None,
            limit,
            accept: None,
        }
    }

    /// This query, with the tool turning along the sweep to end with its
    /// axis along `axis` (see [`end_axis`](Self::end_axis)).
    #[must_use]
    pub fn turned(self, axis: UnitVec3) -> Self {
        Self {
            end_axis: Some(axis),
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

/// The tool at its first pose, the straight move of its tip, and its axis
/// at the end of the move.
#[derive(Debug, Clone)]
pub(crate) struct Body<'a> {
    outline: &'a Outline,
    tip: Point3,
    axis: Vec3,
    sweep: Vec3,
    /// The axis at the end of the sweep; the same as `axis` unless the tool
    /// turns.
    end_axis: Vec3,
    /// The largest distance of a tool point from the tip.
    size: f64,
    /// The unit normal the axis turns about, from `axis` towards
    /// `end_axis`; any unit vector when the tool does not turn.
    pivot: Vec3,
    /// The angle the axis turns through over the move, radians.
    turn: f64,
    /// How far a pose between the two ends may lie outside their hull (see
    /// [`Span`]). Zero without a turn.
    outside: f64,
    /// A segment and a radius whose capsule holds every pose: a cheap bound
    /// for surface far from the tool.
    core: (Point3, Point3, f64),
    /// The outline's lowest and highest points along its axis and its
    /// widest radius.
    reach: (f64, f64, f64),
    /// The middle pose's axis from the height of the outline's widest
    /// radius above its lowest point (a ball's centre) to its top: the
    /// bound by the plane square to the direction from it is cheap and
    /// near exact for a round end.
    spine: (Point3, Point3),
}

impl<'a> Body<'a> {
    pub(crate) fn new(outline: &'a Outline, line: &ToolLine, query: &GapQuery) -> Self {
        let axis = line.axis.get();
        let end_axis = query.end_axis.map_or(axis, UnitVec3::get);
        let turn = axis.dot(end_axis).clamp(-1.0, 1.0).acos();
        let size = outline.top.abs().hypot(outline.radius);
        let sweep = if query.sweep.is_finite() {
            query.sweep
        } else {
            Vec3::ZERO
        };
        // Every pose lies within `outside` of the hull of the first and
        // last, and each pose lies within the outline's widest radius of
        // its own axis between the outline's lowest and highest points; so
        // the hull lies within that radius of the hull of the four ends of
        // those axis pieces, which lies within the farthest of them from
        // the middle pose's.
        let (low, high) = (-outline.support(0.0, -1.0), outline.support(0.0, 1.0));
        let wide = outline.support(1.0, 0.0);
        let middle = UnitVec3::new(axis + end_axis).map_or(axis, UnitVec3::get);
        let centre = line.origin + sweep * 0.5;
        let (c0, c1) = (centre + middle * low, centre + middle * high);
        let spread = [
            line.origin + axis * low,
            line.origin + axis * high,
            line.origin + sweep + end_axis * low,
            line.origin + sweep + end_axis * high,
        ]
        .iter()
        .map(|p| to_segment(*p, c0, c1))
        .fold(0.0, f64::max);
        let outside = size * (1.0 - (0.5 * turn).cos());
        let pivot = UnitVec3::new(axis.cross(end_axis)).map_or(across(axis), UnitVec3::get);
        Self {
            outline,
            tip: line.origin,
            axis,
            sweep,
            end_axis,
            size,
            pivot,
            turn,
            outside,
            core: (c0, c1, spread + wide + outside),
            reach: (low, high, wide),
            spine: (centre + middle * (low + wide).min(high), c1),
        }
    }

    /// A proven least signed distance over the triangle `t` from the tool
    /// at every pose, by the capsule holding them: the distance from its
    /// segment to the triangle's centre, less the triangle's reach from its
    /// centre and the capsule's radius.
    /// A proven least signed distance over the triangle `t` from the tool
    /// at every pose, by the capsule holding them: the exact distance from
    /// its segment to the triangle, less its radius.
    pub(crate) fn far_from(&self, t: &[Point3; 3]) -> f64 {
        let (c0, c1, r) = self.core;
        segment_to_triangle(c0, c1, t) - r
    }

    /// The same bound by the capsule holding the poses of `span` only,
    /// which narrows as the span does.
    fn far_over(&self, t: &[Point3; 3], span: &Span) -> f64 {
        let (low, high, wide) = self.reach;
        let middle = UnitVec3::new(span.axis + span.end_axis).map_or(span.axis, UnitVec3::get);
        let centre = span.tip + span.sweep * 0.5;
        let (c0, c1) = (centre + middle * low, centre + middle * high);
        let end = span.tip + span.sweep;
        let spread = [
            span.tip + span.axis * low,
            span.tip + span.axis * high,
            end + span.end_axis * low,
            end + span.end_axis * high,
        ]
        .iter()
        .map(|p| to_segment(*p, c0, c1))
        .fold(0.0, f64::max);
        segment_to_triangle(c0, c1, t) - spread - wide - span.outside
    }

    fn far(&self, centre: Point3, reach: f64) -> f64 {
        let (c0, c1, r) = self.core;
        to_segment(centre, c0, c1) - reach - r
    }

    /// A proven least signed distance over the triangle `t`, centred at
    /// `centre`, by the plane square to the direction from the spine.
    fn facing(&self, t: &[Point3], centre: Point3, span: &Span) -> f64 {
        let (s0, s1) = self.spine;
        let d = s1 - s0;
        let length = d.dot(d);
        let f = if length > 0.0 {
            ((centre - s0).dot(d) / length).clamp(0.0, 1.0)
        } else {
            0.0
        };
        UnitVec3::new(centre - (s0 + d * f)).map_or(f64::NEG_INFINITY, |u| self.lower(t, u.get(), span))
    }

    /// Whether the tool turns along the sweep.
    fn turns(&self) -> bool {
        self.turn > 0.0
    }

    /// The part of the move from `s0` to `s1` (fractions, 0 to 1).
    fn span(&self, s0: f64, s1: f64) -> Span {
        let (t0, t1) = (self.tip + self.sweep * s0, self.tip + self.sweep * s1);
        let (a0, a1) = (self.axis_at(s0), self.axis_at(s1));
        let outside = if self.turns() {
            // Within the part the axis is the normalised blend of its ends
            // at a fraction that differs from the tip's by at most
            // |c1 - c0| / (4 min(c0, c1)), the c being the lengths of the
            // unnormalised blends at the ends; the tip is that much of the
            // part's sweep from where the hull argument puts it.
            let blend = |s: f64| (self.axis * (1.0 - s) + self.end_axis * s).length();
            let (c0, c1) = (blend(s0), blend(s1));
            let turn = a0.dot(a1).clamp(-1.0, 1.0).acos();
            let skew = if c0.min(c1) > 0.0 {
                (c1 - c0).abs() / (4.0 * c0.min(c1))
            } else {
                1.0
            };
            self.size * (1.0 - (0.5 * turn).cos()) + (t1 - t0).length() * skew
        } else {
            0.0
        };
        Span {
            s0,
            s1,
            tip: t0,
            sweep: t1 - t0,
            axis: a0,
            end_axis: a1,
            outside,
        }
    }

    /// The angle from the first axis to the axis at the fraction `s`, about
    /// [`pivot`](Self::pivot), radians.
    fn angle_at(&self, s: f64) -> f64 {
        let a = self.axis_at(s);
        self.pivot.dot(self.axis.cross(a)).atan2(self.axis.dot(a))
    }

    /// The proven least signed distance over the points `t` (or their
    /// hull) from the tool at every pose of `span`, by the plane square to
    /// `u` at the pose `at` (a fraction) turned with the tool.
    ///
    /// The pose at `s` is the pose at `at` turned about `pivot` by
    /// `δ = angle(s) - angle(at)` and moved with the tip. Turned with it,
    /// the plane still touches the tool, so a point `p` is at least
    /// `R(δ) u . (p - tip(s)) - h(u)` from it, `h` being the tool's support
    /// along `u` at `at`, measured from the tip. With
    /// `tip(s) = tip(s0) + λ Δ` (`λ` from 0 to 1 over the span), that is at
    /// least the least of `R(δ) u . (p - tip(s0))` over the span's `δ`
    /// less the most of `R(δ) u . Δ` (if positive): each is a constant
    /// plus a cosine in `δ`, whose least and most over an interval are
    /// exact. Unlike the hull's bound this one holds the tool's turn
    /// itself, so it stays tight on the tool's flank.
    fn turning_lower(&self, t: &[Point3], u: Vec3, at: f64, span: &Span) -> f64 {
        if !self.turns() {
            return f64::NEG_INFINITY;
        }
        let a = self.axis_at(at);
        let along = a.dot(u);
        let h = self.outline.support((u - a * along).length(), along);
        let base = self.angle_at(at);
        let (d0, d1) = (self.angle_at(span.s0) - base, self.angle_at(span.s1) - base);
        let (d0, d1) = (d0.min(d1), d0.max(d1));
        let n = self.pivot;
        let un = u.dot(n);
        let up = u - n * un;
        let uq = n.cross(up);
        // The least and most of `n.u n.q + cos δ up.q + sin δ uq.q`.
        let range = |q: Vec3| {
            let (x, y) = (up.dot(q), uq.dot(q));
            let rho = x.hypot(y);
            let phi = y.atan2(x);
            let at = |d: f64| x * d.cos() + y * d.sin();
            let hits = |target: f64| {
                let k = ((d0 - target) / std::f64::consts::TAU).ceil();
                target + k * std::f64::consts::TAU <= d1
            };
            let ends = (at(d0), at(d1));
            let least = if hits(phi + std::f64::consts::PI) {
                -rho
            } else {
                ends.0.min(ends.1)
            };
            let most = if hits(phi) { rho } else { ends.0.max(ends.1) };
            let fixed = un * n.dot(q);
            (fixed + least, fixed + most)
        };
        let reach = t
            .iter()
            .map(|p| range(*p - span.tip).0)
            .fold(f64::INFINITY, f64::min);
        reach - range(span.sweep).1.max(0.0) - h
    }

    /// The angle the axis turns through over a part of the move, radians.
    fn turn_of(&self, span: &Span) -> f64 {
        if self.turns() {
            span.axis.dot(span.end_axis).clamp(-1.0, 1.0).acos()
        } else {
            0.0
        }
    }

    /// The tool's axis at the pose `s` along the sweep (0 to 1).
    fn axis_at(&self, s: f64) -> Vec3 {
        if !self.turns() {
            return self.axis;
        }
        let a = self.axis * (1.0 - s) + self.end_axis * s;
        UnitVec3::new(a).map_or(self.axis, UnitVec3::get)
    }

    /// Whether surface in the box `b` may lie within `limit` of the tool at
    /// some pose: false only where the capsule holding every pose is
    /// farther than `limit` from the box's whole extent, so surface the
    /// test leaves out can only have a gap beyond `limit`.
    ///
    /// The box is tested by the capsule, then by the plane facing it from
    /// the spine (its least value over the box is at a corner).
    pub(crate) fn near(&self, b: &Aabb, limit: f64) -> bool {
        let centre = midpoint(b.min, b.max);
        if self.far(centre, 0.5 * b.min.distance(b.max)) > limit {
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
        self.facing(&corners, centre, &self.span(0.0, 1.0)) <= limit
    }

    /// A box holding every pose of the tool, grown by `margin`.
    pub(crate) fn bounds(&self, margin: f64) -> Aabb {
        let ends = [
            self.tip,
            self.tip + self.axis * self.outline.top,
            self.tip + self.sweep,
            self.tip + self.sweep + self.end_axis * self.outline.top,
        ];
        let grow = self.outline.radius + self.outside + margin.max(0.0);
        let mut min = ends[0];
        let mut max = ends[0];
        for p in ends {
            min = Point3::new(min.x.min(p.x), min.y.min(p.y), min.z.min(p.z));
            max = Point3::new(max.x.max(p.x), max.y.max(p.y), max.z.max(p.z));
        }
        Aabb {
            min: Point3::new(min.x - grow, min.y - grow, min.z - grow),
            max: Point3::new(max.x + grow, max.y + grow, max.z + grow),
        }
    }

    /// The signed distance of `x` from the tool at the pose `s` along the
    /// sweep (0 to 1), and the outward unit normal of the tool there.
    fn at_pose(&self, x: Point3, s: f64) -> (f64, Vec3) {
        let axis = self.axis_at(s);
        let d = x - (self.tip + self.sweep * s);
        let z = d.dot(axis);
        let radial = d - axis * z;
        let r = radial.length();
        let w = if r > 1e-12 {
            radial * (1.0 / r)
        } else {
            across(axis)
        };
        let (dist, (nr, nz)) = self.outline.signed(r, z);
        (dist, w * nr + axis * nz)
    }

    /// The least signed distance of `x` from the tool over the sweep, as
    /// found (an upper bound of the true least), the normal there, and how
    /// far above the true least it may be: 0 for one pose; for a sweep, the
    /// sweep's length times the width of the search's last interval, since
    /// the distance is convex along the sweep (so the least lies in that
    /// interval) and changes no faster than the sweep's length per unit.
    /// For a turning tool the distance is not convex along the sweep, so
    /// no slack bounds it: infinity. Only the poses of `span` are searched;
    /// the last value is the fraction of the pose found.
    fn upper(&self, x: Point3, span: &Span) -> (f64, Vec3, f64, f64) {
        let length = self.sweep.length();
        if length <= 0.0 && !self.turns() {
            let (d, n) = self.at_pose(x, 0.0);
            return (d, n, 0.0, 0.0);
        }
        // The distance is convex along the sweep: golden-section search.
        let g = 0.5 * (5.0_f64.sqrt() - 1.0);
        let (mut a, mut b) = (span.s0, span.s1);
        let mut c = b - g * (b - a);
        let mut d = a + g * (b - a);
        let mut fc = self.at_pose(x, c);
        let mut fd = self.at_pose(x, d);
        for _ in 0..SWEEP_STEPS {
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
        for s in [span.s0, span.s1] {
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

    /// The proven least signed distance over the points `t` (or their
    /// hull) from the tool at every pose of `span`, by the plane square to
    /// `u`: the hull of the span's first and last poses reaches along `u`
    /// no further than the larger of their supports (for a tool that does
    /// not turn, the first pose's support plus the sweep's part along
    /// `u`), and each pose lies within the span's `outside` of that hull.
    fn lower(&self, t: &[Point3], u: Vec3, span: &Span) -> f64 {
        let support = |axis: Vec3| {
            let a = axis.dot(u);
            self.outline.support((u - axis * a).length(), a)
        };
        let first = support(span.axis);
        let last = if self.turns() {
            support(span.end_axis)
        } else {
            first
        };
        let h = u.dot(span.tip - Point3::ORIGIN) + first.max(last + u.dot(span.sweep));
        t.iter()
            .map(|p| u.dot(*p - Point3::ORIGIN))
            .fold(f64::INFINITY, f64::min)
            - h
            - span.outside
    }
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

/// A part of the move, from the fraction `s0` to `s1`: its first pose (tip
/// and axis), its sweep and its last axis, and how far a pose within it may
/// lie outside the hull of those two poses.
///
/// A tool point `w` (from the tip) at the fraction `s` is at the tip's
/// place plus `w` turned with the axis. Turned with the in-plane blend of
/// its places at the two ends instead, it moves straight between them, so
/// the blend lies in the hull; the blend is shorter than the turned point
/// by at most `|w| (1 - cos(turn / 2))` (the arc height of the turn), and
/// the tip parts agree when the fractions do. Within a part they differ
/// slightly (see [`Body::span`]), which `outside` adds.
#[derive(Debug, Clone, Copy)]
struct Span {
    s0: f64,
    s1: f64,
    tip: Point3,
    sweep: Vec3,
    axis: Vec3,
    end_axis: Vec3,
    outside: f64,
}

/// One piece of a triangle, over one part of the move, in the search queue.
#[derive(Debug, Clone, Copy)]
struct Piece {
    lower: f64,
    /// The part of the move, as fractions.
    span: (f64, f64),
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
            state.push(t, i, (0.0, 1.0));
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
        // A turning tool's hull bound holds space no pose sweeps, up to
        // about the piece's distance from the tip times the part's turn
        // away; the bound that turns with the tool does not, so the part of
        // the move is halved only once the piece is small beside that
        // wedge and the wedge could explain what the piece's bound lacks.
        // Otherwise the piece is halved.
        let (s0, s1) = piece.span;
        let part = body.span(s0, s1);
        let lever = (centroid(&piece.corners).distance(part.tip) + len).min(body.size);
        let wedge = lever * body.turn_of(&part);
        let split_move = wedge > 4.0 * len && wedge >= found - piece.lower;
        if (len < MIN_PIECE && !split_move) || pieces >= MAX_PIECES {
            break piece.lower.min(found);
        }
        pieces += 2;
        if split_move {
            let sm = 0.5 * (s0 + s1);
            state.push(piece.corners, piece.triangle, (s0, sm));
            state.push(piece.corners, piece.triangle, (sm, s1));
        } else {
            let mid = midpoint(a, b);
            state.push([a, mid, c], piece.triangle, piece.span);
            state.push([mid, b, c], piece.triangle, piece.span);
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
    fn push(&mut self, corners: [Point3; 3], triangle: u32, span: (f64, f64)) {
        let (a, b, c) = (corners[0], corners[1], corners[2]);
        let part = self.body.span(span.0, span.1);
        let centre = centroid(&corners);
        let far = self
            .body
            .far_over(&corners, &part)
            .max(self.body.facing(&corners, centre, &part));
        let lower = UnitVec3::new((b - a).cross(c - a)).map_or(far, |m| {
            let m = m.get();
            self.body
                .lower(&corners, m, &part)
                .max(self.body.lower(&corners, -m, &part))
                .max(far)
        });
        self.order += 1;
        self.queue.push(Piece {
            lower,
            span,
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
        let part = self.body.span(piece.span.0, piece.span.1);
        let (value, normal, slack, at) = self.body.upper(centre, &part);
        if self.best.is_none_or(|(b, _)| value < b) {
            self.best = Some((value, centre));
        }
        let reach = piece
            .corners
            .iter()
            .map(|p| p.distance(centre))
            .fold(0.0, f64::max);
        let lower = piece
            .lower
            .max(self.body.lower(&piece.corners, normal, &part))
            .max(self.body.turning_lower(&piece.corners, normal, at, &part))
            .max(value - slack - reach);
        Piece {
            lower,
            measured: true,
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
