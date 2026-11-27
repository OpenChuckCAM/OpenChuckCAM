// SPDX-License-Identifier: AGPL-3.0-or-later
//! The tool gap: how far the tool is above the surface, each face measured
//! along its own normal (OP-12; decision record SIMUL-EDGE-1, Q2, answered
//! C by the project owner on 26 Nov 2026).
//!
//! The tool is its cutting part, the profile revolved about its axis and
//! capped flat at the profile's top. Every profile the tool library makes
//! (flat, ball, bull nose, tapered ball) gives a convex solid; a profile that
//! is not convex is replaced by the cylinder of its cutting radius from the
//! tip to the top, which holds it, so the answer is never less safe. The
//! tool may also move along a short straight move, and turn about a fixed
//! line as it does (a rotary machine's move): the gap is then the least over
//! every pose on the move.
//!
//! **The measure.** A face (a triangle, wound counter-clockwise seen from
//! outside the material) counts the tool points whose foot on its plane lies
//! on it, and their height above its plane along its outward normal. The gap
//! is the least such height over the faces: negative by the depth the tool
//! cuts below a face, along the face's normal, which is how the program's
//! tolerance measures a cut. A face counts no tool point deeper than the
//! query's limit below it: a convex tool cutting deeper passes through that
//! layer and still reads the limit, while a tool beside a thin wall, beyond
//! the layer below the wall's far face, is not in that face's material and
//! counts nothing there. At a convex edge, a tool dipping below a face's
//! extended plane beyond the face counts nothing for that face.
//!
//! The answer is an interval. `upper` is the height of a real tool point at
//! one pose over a face; `lower` is proven: no counted tool point is lower
//! at any pose.
//!
//! **Why it is proven.** Over a face's prism (its sides `n . y <= b`, `n`
//! square to the face's normal `m`), the least of `m . y` over the tool is
//! at least, for any multipliers `λ >= 0`, the least of `(m + Σ λ n) . y`
//! over the whole tool less `Σ λ b` (Lagrangian duality), which the tool's
//! support function gives; the best multipliers give the exact least. Over
//! a move the direction is fixed to the surface and the tool's support
//! along it is bounded over each part of the move (see [`Body::dual_over`]).
//! The search splits parts of the move and pieces of faces, least bound
//! first, until the least bound left is within [`GAP_ACCURACY`] of the best
//! height found (the independent checker in `openchuck_testkit::contact`,
//! which this code does not share, bounds the same measure its own way).
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

/// A part of a move shorter than this (as a fraction) is not split again.
const MIN_SPAN: f64 = 1e-9;

/// How far inside a side of a face's prism (or above the layer's floor) a
/// tool point may lie and still be taken as off the face, mm. A tool that
/// only touches a side (finishing against a wall, the drop puts it exactly
/// there) is then separated at once, where the bound would otherwise need
/// ever smaller pieces. A point that near a side is as near the face across
/// it, which counts it; at the edge of the mesh there is nothing beyond to
/// cut. It changes a reading by no more than its own size, far under
/// [`GAP_ACCURACY`].
pub const SIDE_MARGIN: f64 = 1e-7;

/// The least of `m . y` over the corners of the polytope where
/// `n . y <= b` for every `(n, b)` of `planes`, by intersecting each three
/// of them; `None` when none lies within all (the polytope is empty: it is
/// bounded by the face's prism, floor and plane).
fn polytope_least(planes: &[(Vec3, f64)], m: Vec3) -> Option<f64> {
    polytope_lowest(planes, m).map(|(v, _)| v)
}

/// As [`polytope_least`], with the corner where it is least.
fn polytope_lowest(planes: &[(Vec3, f64)], m: Vec3) -> Option<(f64, Point3)> {
    let scale = planes.iter().map(|p| p.1.abs()).fold(1.0, f64::max);
    let mut least: Option<(f64, Point3)> = None;
    let count = planes.len();
    for i in 0..count {
        for j in i + 1..count {
            for k in j + 1..count {
                let (p, q, r) = (planes[i], planes[j], planes[k]);
                let det = p.0.dot(q.0.cross(r.0));
                if det.abs() < 1e-14 {
                    continue;
                }
                let y = (q.0.cross(r.0) * p.1 + r.0.cross(p.0) * q.1 + p.0.cross(q.0) * r.1)
                    * (1.0 / det);
                if planes.iter().all(|(n, b)| n.dot(y) <= b + 1e-10 * scale) {
                    let v = m.dot(y);
                    if least.is_none_or(|(l, _)| v < l) {
                        least = Some((v, Point3::ORIGIN + y));
                    }
                }
            }
        }
    }
    least
}

/// The golden-section steps of the search for a leaning plane's angle in
/// [`Body::lower_surface`].
const LEAN_STEPS: u32 = 40;

/// The angles, in radians, a separating plane leans from a side of the
/// prism towards the layer's floor ([`Body::below_layer`]).
const LEANS: [f64; 8] = [0.0, 0.05, 0.15, 0.3, 0.5, 0.75, 1.05, 1.35];

/// The sizes the multipliers are also tried at over a part of a move
/// ([`scaled`]).
const MULTIPLIER_SCALES: [f64; 7] = [1e4, 1e3, 1e2, 30.0, 10.0, 3.0, 1.0];

/// Bisection steps for each side's multiplier in the dual's search for its
/// best multipliers ([`Body::multipliers`]): any multipliers give a proven
/// bound, the search only makes it tighter.
const MULTIPLIER_STEPS: u32 = 60;

/// The largest multiplier searched for: a side that needs more has the tool
/// all but beyond it.
const MAX_MULTIPLIER: f64 = 1e9;

/// Golden-section and bisection steps of the search along a face's normal
/// for the lowest tool point there ([`Body::column`]): it only finds a real
/// value.
const COLUMN_STEPS: u32 = 50;

/// The most a part of a move may turn for the bound over it (radians); a
/// part that turns more has no bound and is split.
const MAX_PART_TURN: f64 = 1.0;

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
    /// How far the surface is looked for, mm: a gap larger than this gives
    /// no answer, and a face counts no tool point deeper than this below
    /// it, so the gap is never below `-limit` (see the module's measure).
    pub limit: f64,
    /// When set, the search stops as soon as the gap is proven at least this
    /// (mm; negative for an overcut depth a caller accepts), with whatever
    /// interval it has: for a caller that needs only to know whether the
    /// gap is above a limit. When not set, the interval is narrowed to
    /// [`GAP_ACCURACY`].
    pub accept: Option<f64>,
    /// When set, the faces that stand in front of material thinner than
    /// this (mm) are left out: a face backed, along its inward normal, by
    /// an opposing face nearer than this. For a caller that has found the
    /// tool beside a thin wall and asks whether it cuts anything else
    /// (decision record SIMUL-EDGE-1, Terms: *Thin wall*). A source that
    /// cannot tell answers as if it were not set.
    pub skip_walls_thinner_than: Option<f64>,
}

impl GapQuery {
    /// One pose, the surface looked for within `limit`.
    pub fn pose(limit: f64) -> Self {
        Self {
            sweep: Vec3::ZERO,
            rotation: None,
            limit,
            accept: None,
            skip_walls_thinner_than: None,
        }
    }

    /// The tool swept along `sweep`, the surface looked for within `limit`.
    pub fn swept(sweep: Vec3, limit: f64) -> Self {
        Self {
            sweep,
            rotation: None,
            limit,
            accept: None,
            skip_walls_thinner_than: None,
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

    /// This query, leaving out the faces in front of material thinner than
    /// `wall` (see [`skip_walls_thinner_than`](Self::skip_walls_thinner_than)).
    #[must_use]
    pub fn skip_walls_thinner_than(self, wall: f64) -> Self {
        Self {
            skip_walls_thinner_than: Some(wall),
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

/// How far the tool is above the surface, each face measured along its own
/// normal over itself, as an interval (decision record SIMUL-EDGE-1, Q2):
/// see [`SurfaceSource::tool_gap`].
///
/// [`SurfaceSource::tool_gap`]: crate::SurfaceSource::tool_gap
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolGap {
    /// Proven: no tool point counted by a face is lower above it than this
    /// at any pose. Negative when the tool may cut that deep below a face.
    pub lower: f64,
    /// Found: the height above a face, at one pose, of a real tool point
    /// over the face point [`at`](Self::at). The true gap lies between
    /// `lower` and this.
    pub upper: f64,
    /// The face point under the tool point where `upper` was found.
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
                ProfilePiece::Arc { centre, radius, .. } => {
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

    /// The outline point farthest in the half-plane direction `(a, b)`,
    /// `a >= 0`: where [`support`](Self::support) is reached.
    fn support_point(&self, a: f64, b: f64) -> (f64, f64) {
        let mut best = (0.0_f64, (0.0, 0.0));
        for p in &self.pieces {
            for q in [p.start(), p.end()] {
                let v = a * pr(q) + b * pz(q);
                if v > best.0 {
                    best = (v, (pr(q), pz(q)));
                }
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
                    let (ta, tb) = arc_range(start, end, centre);
                    let angle = b.atan2(a);
                    if angle >= ta && angle <= tb {
                        let q = (pr(centre) + radius * a / len, pz(centre) + radius * b / len);
                        let v = a * q.0 + b * q.1;
                        if v > best.0 {
                            best = (v, q);
                        }
                    }
                }
            }
        }
        best.1
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
            + self
                .outline
                .support((u - self.axis * along).length(), along);
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
        let moved = t.iter().map(|p| self.moved(*p, s0, s1)).fold(0.0, f64::max);
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
    /// some pose: false only where the capsule holding the tool, over the
    /// whole move, is proven farther than `limit` from the box. Cheap, for
    /// the boxes of a search structure; a triangle that passes is bounded
    /// more closely when it is queued.
    pub(crate) fn near(&self, b: &Aabb, limit: f64) -> bool {
        let (low, high, wide) = self.reach;
        let centre = midpoint(b.min, b.max);
        let reach = 0.5 * b.min.distance(b.max);
        let (c, n) = self.pivot;
        let q = centre - c;
        let off = (q - n * q.dot(n)).length() + reach;
        let moved = 0.5 * (self.sweep.length() + self.angle.abs() * off);
        let at = self.frame(centre, 0.5);
        to_segment(at, self.tip + self.axis * low, self.tip + self.axis * high)
            - reach
            - wide
            - moved
            <= limit
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

    /// Where the point `y` of the tool's frame at the fraction `f` is on the
    /// surface: the inverse of [`frame`](Self::frame).
    fn unframe(&self, y: Point3, f: f64) -> Point3 {
        self.turned(y + self.sweep * f, f * self.angle)
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

    /// The tool's support along `v` measured from its tip, at its first
    /// pose: never negative, the tip being part of the tool.
    fn reach_along(&self, v: Vec3) -> f64 {
        let along = self.axis.dot(v);
        self.outline
            .support((v - self.axis * along).length(), along)
            .max(0.0)
    }

    /// The point of the tool at its first pose farthest along `v`.
    fn support_point(&self, v: Vec3) -> Point3 {
        let along = self.axis.dot(v);
        let radial = v - self.axis * along;
        let r = radial.length();
        let (pr_, pz_) = self.outline.support_point(r, along);
        let e = if r > 1e-15 {
            radial * (1.0 / r)
        } else {
            across(self.axis)
        };
        self.tip + self.axis * pz_ + e * pr_
    }

    /// The least of `w . y` over the tool at its first pose.
    fn lowest_first(&self, w: Vec3) -> f64 {
        w.dot(self.tip - Point3::ORIGIN) - self.reach_along(-w)
    }

    /// The face `face` seen in the tool's frame at the fraction `f`.
    fn face_at(&self, face: &Face, f: f64) -> Face {
        let corners = face.corners.map(|p| self.frame(p, f));
        let clips = face
            .clips
            .iter()
            .map(|(n, b)| {
                let on = self.frame(Point3::ORIGIN + *n * *b, f);
                let n = self.direction(*n, f);
                (n, n.dot(on - Point3::ORIGIN))
            })
            .collect();
        Face::new(corners, self.direction(face.m, f)).with_clips(clips)
    }

    /// The best multipliers for the dual bound of the lowest point of the
    /// tool at the fraction `f` over the face's prism (see [`Face`]); with
    /// `highest`, of the highest point (the same problem on the face turned
    /// over, see [`Face::dual_top`]).
    ///
    /// The lowest point over the prism is the tool's free lowest point when
    /// its foot is on the face (no multipliers); otherwise it lies against
    /// one side, where the one multiplier is the root of the dual's slope
    /// (how far the dual's lowest point lies beyond the side), or on the
    /// column at a corner, where the tool's normal there gives the two. The
    /// candidate whose dual bound is highest is taken: every candidate's
    /// multipliers give a proven bound, the right one the exact least.
    fn multipliers(&self, face: &Face, f: f64, highest: bool) -> Vec<f64> {
        let mut at = self.face_at(face, f);
        if highest {
            at = at.turned_over();
        }
        let count = at.count();
        let value = |lam: &[f64]| {
            let (w, c) = at.dual(lam);
            self.lowest_first(w) - c
        };
        let beyond = |y: Point3, i: usize| {
            let (n, b) = at.constraint(i);
            n.dot(y - Point3::ORIGIN) - b
        };
        let low = self.support_point(-at.m);
        if (0..count).all(|i| beyond(low, i) <= 0.0) {
            return vec![0.0; count];
        }
        let mut best = (vec![0.0; count], value(&vec![0.0; count]));
        let consider = |lam: Vec<f64>, best: &mut (Vec<f64>, f64)| {
            if lam.iter().all(|l| l.is_finite() && *l >= 0.0) {
                let v = value(&lam);
                if v > best.1 {
                    *best = (lam, v);
                }
            }
        };
        // Against one side: the slope of the dual in its multiplier is how
        // far the dual's lowest point lies beyond the side, and it falls as
        // the multiplier grows (the dual is concave).
        for i in 0..count {
            let n = at.constraint(i).0;
            if n.length() <= 0.0 {
                continue;
            }
            let slope = |l: f64| beyond(self.support_point(-(at.m + n * l)), i);
            if slope(0.0) <= 0.0 {
                continue;
            }
            let mut hi = 1e-3_f64;
            while slope(hi) > 0.0 && hi < MAX_MULTIPLIER {
                hi *= 4.0;
            }
            let mut lo = 0.0_f64;
            for _ in 0..MULTIPLIER_STEPS {
                let mid = 0.5 * (lo + hi);
                if slope(mid) > 0.0 {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            // The dual is flat across the kink where the lowest point
            // jumps; either end of the last bracket may be the better.
            for l in [lo, hi] {
                let mut lam = vec![0.0; count];
                lam[i] = l;
                consider(lam, &mut best);
            }
        }
        // The best so far is exact when its dual's lowest point has its
        // foot on the face. Otherwise the lowest point lies on the column
        // at a corner whose two sides it crosses: the tool's outward normal
        // there, written as -(m + λi ni + λj nj) scaled, gives both
        // multipliers.
        let (w, _) = at.dual(&best.0);
        let y = self.support_point(-w);
        let crossed = [beyond(y, 0) > 0.0, beyond(y, 1) > 0.0, beyond(y, 2) > 0.0];
        if !crossed.iter().any(|c| *c) {
            return best.0;
        }
        for k in 0..3 {
            let (i, j) = ((k + 2) % 3, k);
            if !(crossed[i] || crossed[j]) {
                continue;
            }
            let x = at.corners[k];
            if let Some((_, y)) = self.column_first(&at, x) {
                let normal = self.at_first(y).1;
                if let Some(l) = corner_multipliers(-normal, at.m, at.sides[i].0, at.sides[j].0) {
                    let mut lam = vec![0.0; count];
                    lam[i] = l.0;
                    lam[j] = l.1;
                    consider(lam, &mut best);
                }
            }
        }
        best.0
    }

    /// The lowest point of the tool at its first pose on the line through
    /// the point `x` along the face's normal (both in the tool's frame), and
    /// its height: as [`column`](Self::column) without the layer.
    fn column_first(&self, at: &Face, x: Point3) -> Option<(f64, Point3)> {
        let m = at.m;
        let sd = |t: f64| self.at_first(x + m * t).0;
        let reach = x.distance(self.tip) + self.outline.top.abs() + self.outline.radius + 1.0;
        let golden = 0.5 * (5.0_f64.sqrt() - 1.0);
        let (mut a, mut b) = (-reach, reach);
        let mut c = b - golden * (b - a);
        let mut d = a + golden * (b - a);
        let (mut fc, mut fd) = (sd(c), sd(d));
        for _ in 0..COLUMN_STEPS {
            if fc <= fd {
                b = d;
                d = c;
                fd = fc;
                c = b - golden * (b - a);
                fc = sd(c);
            } else {
                a = c;
                c = d;
                fc = fd;
                d = a + golden * (b - a);
                fd = sd(d);
            }
        }
        let (inner, deepest) = if fc <= fd { (c, fc) } else { (d, fd) };
        if deepest > 0.0 {
            return None;
        }
        let (mut out, mut inn) = (-reach, inner);
        for _ in 0..COLUMN_STEPS {
            let mid = 0.5 * (out + inn);
            if sd(mid) <= 0.0 {
                inn = mid;
            } else {
                out = mid;
            }
        }
        Some((inn, x + m * inn))
    }

    /// The constant and the three parts (`k0 + x cos(f θ) + y sin(f θ)`) of
    /// `w_f . v`, `w_f` being the surface direction `w` seen in the tool's
    /// frame at the fraction `f`.
    fn seen(&self, w: Vec3, v: Vec3) -> (f64, f64, f64) {
        let n = self.pivot.1;
        let along = n * v.dot(n);
        (v.dot(n) * w.dot(n), w.dot(v - along), w.dot(n.cross(v)))
    }

    /// A proven least of `min over the tool of w . y`, less `c`, over the
    /// fractions `s0` to `s1`, `w` fixed to the surface. Seen from the
    /// tool, `w` turns: the tool's support along it over the part is at
    /// most the larger at its ends times `sec(α / 2)`, plus `sec(α / 2) - 1`
    /// times its support along the part of `w` on the turn's axis, `α` the
    /// part's turn (the turned direction is a blend of the two ends' with
    /// weights summing to at least 1 and at most `sec(α / 2)`).
    fn dual_over(&self, w: Vec3, c: f64, s0: f64, s1: f64) -> f64 {
        if !self.turns() {
            let k = w.dot(self.sweep);
            return self.lowest_first(w) + (s0 * k).min(s1 * k) - c;
        }
        let alpha = self.turn_over(s0, s1);
        if alpha >= MAX_PART_TURN {
            return f64::NEG_INFINITY;
        }
        let (cl, n) = self.pivot;
        let span = (s0, s1);
        let (k0, x, y) = self.seen(w, self.tip - cl);
        let first = least(k0, x, y, self.angle, 0.0, span);
        let (k0, x, y) = self.seen(w, self.sweep);
        let lo = least(k0, x, y, self.angle, 0.0, span);
        let hi = -least(-k0, -x, -y, self.angle, 0.0, span);
        let shift = (s0 * lo).min(s0 * hi).min(s1 * lo).min(s1 * hi);
        let sec = 1.0 / (0.5 * alpha).cos();
        let ends = self
            .reach_along(self.direction(-w, s0))
            .max(self.reach_along(self.direction(-w, s1)));
        let on_axis = n * w.dot(n);
        let axial = self.reach_along(on_axis).max(self.reach_along(-on_axis));
        w.dot(cl - Point3::ORIGIN) + first + shift - sec * ends - (sec - 1.0) * axial - c
    }

    /// A proven most of the tool's height above the face's plane over the
    /// fractions `s0` to `s1`: below `-limit`, the whole tool is deeper than
    /// the layer the face counts.
    fn top_over(&self, face: &Face, s0: f64, s1: f64) -> f64 {
        -self.dual_over(-face.m, -face.d, s0, s1)
    }

    /// Whether the tool lies wholly beyond one side of the face's prism at
    /// every pose from `s0` to `s1`, so no tool point has its foot on the
    /// face: proven by the least of the side's outward distance over the
    /// tool and the motion. Where the tool only touches a side the dual's
    /// multipliers grow without end; this settles the empty prisms among
    /// them at once.
    fn beside(&self, face: &Face, s0: f64, s1: f64) -> bool {
        face.constraints()
            .any(|(n, b)| n.length() > 0.0 && self.dual_over(*n, *b, s0, s1) > -SIDE_MARGIN)
            || face.around.iter().chain(self.toward(face).iter()).any(|u| {
                // u is square to the face's normal: over the prism u . y is
                // at most its most at a corner.
                let c = face
                    .corners
                    .iter()
                    .map(|p| u.dot(*p - Point3::ORIGIN))
                    .fold(f64::NEG_INFINITY, f64::max);
                self.dual_over(*u, c, s0, s1) > -SIDE_MARGIN
            })
    }

    /// The direction across the face's normal from the piece towards the
    /// tool's nearest point to its centre, at the first pose (the tool's
    /// outward normal there, reversed): a place to look for a plane parting
    /// them that the tool's axis may not give (a flat end overhanging an
    /// edge lies beside the face across from where its axis is).
    fn toward(&self, face: &Face) -> Option<Vec3> {
        let c = centroid(&face.corners);
        let n = -self.at_first(c).1;
        UnitVec3::new(n - face.m * n.dot(face.m))
            .ok()
            .map(UnitVec3::get)
    }

    /// Whether no tool point with its foot on the face lies above the
    /// layer, `limit` below it, at any pose from `s0` to `s1`: a plane
    /// leaning from one side of the prism towards the layer's floor
    /// (outward normal `n cos t - m sin t`) with the whole tool beyond it
    /// and the prism above the floor behind it. A tool resting on the plane
    /// of a side, below the layer (a face beside the one the tool touches),
    /// is separated so only by a leaning plane: the side alone touches it.
    fn below_layer(&self, face: &Face, limit: f64, s0: f64, s1: f64) -> bool {
        let floor = face.corners.map(|p| p + face.m * (-limit));
        // Across the face's normal: away from each side, and from the piece
        // towards the tool's spine (the tool may lie beyond a corner, or
        // beyond the side of a neighbouring face).
        let centre = centroid(&face.corners);
        let (a, b) = self.spine;
        let ab = b - a;
        let k = if ab.length() > 0.0 {
            ((centre - a).dot(ab) / ab.dot(ab)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let to = (a + ab * k) - centre;
        let across = UnitVec3::new(to - face.m * to.dot(face.m)).map(UnitVec3::get);
        let directions: Vec<Vec3> = face
            .sides
            .iter()
            .map(|(n, _)| *n)
            .chain(face.around.iter().copied())
            .chain(across)
            .chain(self.toward(face))
            .filter(|n| n.length() > 0.0)
            .collect();
        directions.iter().any(|n| {
            LEANS.iter().any(|t| {
                let w = *n * t.cos() - face.m * t.sin();
                // The prism above the floor lies behind the plane at its
                // floor corners, as w leans away from the face's normal.
                let c = floor
                    .iter()
                    .map(|p| w.dot(*p - Point3::ORIGIN))
                    .fold(f64::NEG_INFINITY, f64::max);
                self.dual_over(w, c, s0, s1) > -SIDE_MARGIN
            })
        })
    }

    /// The lowest tool point at the fraction `f` with its foot on the face
    /// and no deeper than `limit` below it, as found from the multipliers
    /// `lam`: its height above the face's plane and its foot. The dual's
    /// lowest point is the constrained one when the multipliers are best;
    /// otherwise the column through the face's centre is measured.
    fn found(&self, face: &Face, lam: &[f64], f: f64, limit: f64) -> Option<(f64, Point3)> {
        let (w, _) = face.dual(lam);
        let y = self.unframe(self.support_point(-self.direction(w, f)), f);
        let height = face.m.dot(y - Point3::ORIGIN) - face.d;
        let foot = y + face.m * (-height);
        if height >= -limit && face.holds(foot) && face.clear(y) {
            return Some((height, foot));
        }
        // The foot is off the face (the multipliers are not quite the best,
        // or rounding) or the point is below the layer: measure the column
        // through the face point nearest the foot, then the face's centre.
        let near = crate::surface::closest_on_triangle(foot, &face.corners);
        let at_near = self.column(face, near, f, limit);
        let at_centre = self.column(face, centroid(&face.corners), f, limit);
        match (at_near, at_centre) {
            (Some(a), Some(b)) => Some(if b.0 < a.0 { b } else { a }),
            (a, b) => a.or(b),
        }
    }

    /// The lowest tool point at the fraction `f` on the line through the
    /// face point `x` along the face's normal, no deeper than `limit` below
    /// it: the tool meets the line in an interval (it is convex), found by
    /// a golden-section search for its deepest point and a bisection for
    /// its lower end.
    fn column(&self, face: &Face, x: Point3, f: f64, limit: f64) -> Option<(f64, Point3)> {
        let base = self.frame(x, f);
        let m = self.direction(face.m, f);
        let sd = |t: f64| self.at_first(base + m * t).0;
        let reach = base.distance(self.tip) + self.outline.top.abs() + self.outline.radius + 1.0;
        let golden = 0.5 * (5.0_f64.sqrt() - 1.0);
        let (mut a, mut b) = (-reach, reach);
        let mut c = b - golden * (b - a);
        let mut d = a + golden * (b - a);
        let (mut fc, mut fd) = (sd(c), sd(d));
        for _ in 0..COLUMN_STEPS {
            if fc <= fd {
                b = d;
                d = c;
                fd = fc;
                c = b - golden * (b - a);
                fc = sd(c);
            } else {
                a = c;
                c = d;
                fc = fd;
                d = a + golden * (b - a);
                fd = sd(d);
            }
        }
        let (inner, deepest) = if fc <= fd { (c, fc) } else { (d, fd) };
        if deepest > 0.0 {
            return None;
        }
        let (mut out, mut inn) = (-reach, inner);
        for _ in 0..COLUMN_STEPS {
            let mid = 0.5 * (out + inn);
            if sd(mid) <= 0.0 {
                inn = mid;
            } else {
                out = mid;
            }
        }
        // The heights the face counts along the column: no deeper than the
        // layer, and on the inner side of every clip.
        let (mut lo, mut hi) = (-limit, f64::INFINITY);
        for (n, b) in &face.clips {
            let k = n.dot(face.m);
            let r = b - n.dot(x - Point3::ORIGIN);
            if k > 1e-12 {
                hi = hi.min(r / k);
            } else if k < -1e-12 {
                lo = lo.max(r / k);
            } else if r < 0.0 {
                return None;
            }
        }
        let height = inn.max(lo);
        if height > hi || sd(height) > 0.0 {
            return None;
        }
        Some((height, x))
    }

    /// Planes touching the tool from below at a single pose, each `(n, k)`
    /// with the tool where `n . y >= k` and `n . m > 0`: tangent where the
    /// columns through the piece's centre and corners first meet the tool,
    /// and leaning from the face's normal towards the piece from under the
    /// tool, towards the tool's nearest point and across each side, each at
    /// the angle a golden-section search finds best at the piece's corners.
    fn cheap_planes(&self, face: &Face) -> Vec<(Vec3, f64)> {
        let m = face.m;
        let mut planes = Vec::new();
        let centre = centroid(&face.corners);
        for x in std::iter::once(centre).chain(face.corners) {
            if let Some((_, y)) = self.column_first(face, x) {
                let n = -self.at_first(y).1;
                if n.dot(m) > 1e-9 {
                    planes.push((n, n.dot(y - Point3::ORIGIN)));
                }
            }
        }
        let (a, b) = self.spine;
        let middle = Point3::new(0.5 * (a.x + b.x), 0.5 * (a.y + b.y), 0.5 * (a.z + b.z));
        let to = centre - middle;
        let leans: Vec<Vec3> = UnitVec3::new(to - m * to.dot(m))
            .ok()
            .map(UnitVec3::get)
            .into_iter()
            .chain(self.toward(face))
            .chain(face.sides.iter().map(|(n, _)| *n))
            .filter(|e| e.length() > 0.0)
            .collect();
        let golden = 0.5 * (5.0_f64.sqrt() - 1.0);
        for e in leans {
            let plane = |t: f64| {
                let w = m * t.cos() + e * t.sin();
                (w, self.lowest_first(w))
            };
            let score = |t: f64| {
                let (n, k) = plane(t);
                face.corners
                    .iter()
                    .map(|x| (k - n.dot(*x - Point3::ORIGIN)) / n.dot(m))
                    .fold(f64::INFINITY, f64::min)
            };
            let (mut lo, mut hi) = (0.0_f64, std::f64::consts::FRAC_PI_2 - 1e-6);
            for _ in 0..LEAN_STEPS {
                let (c, d) = (hi - golden * (hi - lo), lo + golden * (hi - lo));
                if score(c) >= score(d) {
                    hi = d;
                } else {
                    lo = c;
                }
            }
            planes.push(plane(0.5 * (lo + hi)));
            // Square to the face: a tool lying beside it.
            let side = (e, self.lowest_first(e));
            planes.push(side);
        }
        planes
    }

    /// The best bound `planes` give on the least height above the face of
    /// a tool point in `region` (as `n . y <= b` sides), no deeper than
    /// `limit` below the face and no higher than `cap` above it: the least
    /// height over the corners of the polytope of the region and each
    /// plane; `INFINITY` when for some plane it is empty.
    fn polytope_bound(
        &self,
        face: &Face,
        region: &[(Vec3, f64)],
        planes: &[(Vec3, f64)],
        limit: f64,
        cap: f64,
    ) -> (f64, Option<Point3>) {
        let m = face.m;
        let mut base: Vec<(Vec3, f64)> = region.to_vec();
        base.push((-m, limit - face.d));
        base.push((m, face.d + cap));
        let mut best = (f64::NEG_INFINITY, None);
        for (n, k) in planes {
            let mut with = base.clone();
            with.push((-*n, -*k));
            let (v, at) = polytope_lowest(&with, m)
                .map_or((f64::INFINITY, None), |(v, at)| (v - face.d, Some(at)));
            if v > best.0 {
                best = (v, at);
            }
            if best.0 == f64::INFINITY {
                break;
            }
        }
        best
    }

    /// A bound, at a single pose (no shift or turn), on the least height
    /// above the face of a tool point in `region` (the face's prism, with
    /// or without its clips, as `n . y <= b` sides), counted no deeper than
    /// `limit` below the face and no higher than `cap` above it, from the
    /// tool's lower surface along the face's normal (the owner's answer of
    /// 27 Nov 2026). Each plane touching the tool (the tool on its side
    /// `n . y >= k`) bounds it by the least height over the corners of the
    /// polytope of the region and the plane, or shows it empty
    /// (`INFINITY`): planes tangent where the columns through the piece's
    /// centre and corners first meet the tool, and planes leaning from the
    /// face's normal by an angle searched up to square to it, towards the
    /// piece from under the tool, towards the tool's nearest point, and
    /// across each side and neighbour. Where the tool runs along the edge
    /// of the face's region, or lies beside it, the dual's multipliers grow
    /// without end; these do not.
    fn lower_surface(&self, face: &Face, region: &[(Vec3, f64)], limit: f64, cap: f64) -> f64 {
        let m = face.m;
        let mut base: Vec<(Vec3, f64)> = region.to_vec();
        base.push((-m, limit - face.d));
        base.push((m, face.d + cap));
        let least = |n: Vec3, k: f64| {
            let mut with = base.clone();
            with.push((-n, -k));
            polytope_least(&with, m).map_or(f64::INFINITY, |v| v - face.d)
        };
        let mut best = f64::NEG_INFINITY;
        let centre = centroid(&face.corners);
        for x in std::iter::once(centre).chain(face.corners) {
            if let Some((_, y)) = self.column_first(face, x) {
                // The tool's outward normal there, turned to face into it.
                let n = -self.at_first(y).1;
                best = best.max(least(n, n.dot(y - Point3::ORIGIN)));
                if best == f64::INFINITY {
                    return best;
                }
            }
        }
        let (a, b) = self.spine;
        let middle = Point3::new(0.5 * (a.x + b.x), 0.5 * (a.y + b.y), 0.5 * (a.z + b.z));
        let to = centre - middle;
        let leans: Vec<Vec3> = UnitVec3::new(to - m * to.dot(m))
            .ok()
            .map(UnitVec3::get)
            .into_iter()
            .chain(self.toward(face))
            .chain(face.sides.iter().map(|(n, _)| *n))
            .chain(face.around.iter().copied())
            .filter(|e| e.length() > 0.0)
            .collect();
        let golden = 0.5 * (5.0_f64.sqrt() - 1.0);
        for e in leans {
            let at = |t: f64| {
                let w = m * t.cos() + e * t.sin();
                least(w, self.lowest_first(w))
            };
            let (mut lo, mut hi) = (0.0_f64, std::f64::consts::FRAC_PI_2);
            for _ in 0..LEAN_STEPS {
                let (c, d) = (hi - golden * (hi - lo), lo + golden * (hi - lo));
                if at(c) >= at(d) {
                    hi = d;
                } else {
                    lo = c;
                }
            }
            for t in [0.0, 0.5 * (lo + hi), std::f64::consts::FRAC_PI_2] {
                best = best.max(at(t));
            }
            if best == f64::INFINITY {
                return best;
            }
        }
        best
    }

    /// The turn over the fractions `s0` to `s1`, radians.
    fn turn_over(&self, s0: f64, s1: f64) -> f64 {
        self.angle.abs() * (s1 - s0)
    }
}

/// A face of the surface for the gap (decision record SIMUL-EDGE-1, Q2 as
/// answered C): a triangle, or a piece of one, with the outward unit normal
/// `m` of the triangle it is part of, its plane `m . y = d`, and its sides,
/// each `n . y <= b` with `n` square to `m` pointing out of the face.
///
/// The gap over a face is the least height `m . y - d` of a tool point `y`
/// whose foot on the plane lies on the face, counting no point deeper than
/// the query's limit below it. Over a face's prism (its sides), the least
/// of `m . y` over the tool is at least, for any multipliers `λ >= 0`,
/// the least of `(m + Σ λ n) . y` over the whole tool less `Σ λ b`
/// (Lagrangian duality): every multiplier gives a proven bound, the best
/// ones the exact least.
///
/// **Convex neighbours** (the owner's answer of 26 Nov 2026, option A: the
/// depth into the material). A face also counts a tool point only where it
/// lies at least the query's acceptance below the plane of each face that
/// meets it at a convex edge or corner (its `clips`, each `n . y <= b`). A
/// point within that of a neighbour's plane is, the material being below
/// both planes there, within that of the surface: no cut deeper than the
/// acceptance. Without the rule a tool resting on one face beside a sharp
/// edge would read, along the other face's normal, as cut as far below it
/// as it is past the edge. The clips are more sides for the same dual.
#[derive(Debug, Clone)]
struct Face {
    corners: [Point3; 3],
    m: Vec3,
    d: f64,
    sides: [(Vec3, f64); 3],
    clips: Vec<(Vec3, f64)>,
    /// Unit directions square to `m` from the normals of every face meeting
    /// this one at a corner: places to look for a plane parting the tool
    /// from the prism (a tool against a wall that meets the face only at a
    /// corner is parted from it by the wall's plane, not by a side).
    around: Vec<Vec3>,
}

impl Face {
    fn new(corners: [Point3; 3], m: Vec3) -> Self {
        let side = |i: usize| {
            let (a, b) = (corners[i], corners[(i + 1) % 3]);
            let n = UnitVec3::new((b - a).cross(m)).map_or(Vec3::ZERO, UnitVec3::get);
            (n, n.dot(a - Point3::ORIGIN))
        };
        Self {
            corners,
            m,
            d: m.dot(corners[0] - Point3::ORIGIN),
            sides: [side(0), side(1), side(2)],
            clips: Vec::new(),
            around: Vec::new(),
        }
    }

    /// This face with the planes of its convex neighbours as more sides.
    fn with_clips(self, clips: Vec<(Vec3, f64)>) -> Self {
        Self { clips, ..self }
    }

    /// This face with directions to part the tool from it (see `around`).
    fn with_around(self, normals: &[Vec3]) -> Self {
        let m = self.m;
        let around = normals
            .iter()
            .filter_map(|n| UnitVec3::new(*n - m * n.dot(m)).ok().map(UnitVec3::get))
            .collect();
        Self { around, ..self }
    }

    /// Every side of the face's region: the triangle's three, then its
    /// clips.
    fn constraints(&self) -> impl Iterator<Item = &(Vec3, f64)> {
        self.sides.iter().chain(self.clips.iter())
    }

    /// How many sides the region has, so how many multipliers.
    fn count(&self) -> usize {
        3 + self.clips.len()
    }

    /// The side `i` of the region.
    fn constraint(&self, i: usize) -> (Vec3, f64) {
        if i < 3 {
            self.sides[i]
        } else {
            self.clips.get(i - 3).copied().unwrap_or((Vec3::ZERO, 0.0))
        }
    }

    /// The dual direction and constant for the multipliers `lam`.
    fn dual(&self, lam: &[f64]) -> (Vec3, f64) {
        let mut w = self.m;
        let mut c = self.d;
        for (l, (n, b)) in lam.iter().zip(self.constraints()) {
            w = w + *n * *l;
            c += l * b;
        }
        (w, c)
    }

    /// Whether the point `y` is on the inner side of every clip, a hair
    /// outside counting as on it.
    fn clear(&self, y: Point3) -> bool {
        let scale = 1e-9 * (1.0 + (y - Point3::ORIGIN).length());
        self.clips
            .iter()
            .all(|(n, b)| n.dot(y - Point3::ORIGIN) <= b + scale)
    }

    /// The face turned over: the highest point over it is the lowest over
    /// this, its normal and plane negated, its sides the same.
    fn turned_over(&self) -> Self {
        Self {
            m: -self.m,
            d: -self.d,
            ..self.clone()
        }
    }

    /// The direction and constant for the multipliers `mu` of the bound on
    /// the highest point: over the face's prism, the most of `m . y` over
    /// the tool is at most the most of `(m - Σ μ n) . y` over the whole
    /// tool plus `Σ μ b`, so the height is at most that less `d`: the most
    /// of `v . y` less `c`.
    fn dual_top(&self, mu: &[f64]) -> (Vec3, f64) {
        let mut v = self.m;
        let mut c = self.d;
        for (l, (n, b)) in mu.iter().zip(self.constraints()) {
            v = v - *n * *l;
            c -= l * b;
        }
        (v, c)
    }

    /// Whether the plane point `x` lies on the face, a hair outside
    /// counting as on it.
    fn holds(&self, x: Point3) -> bool {
        let scale = 1e-9 * (1.0 + (x - Point3::ORIGIN).length());
        self.sides
            .iter()
            .all(|(n, b)| n.dot(x - Point3::ORIGIN) <= b + scale)
    }
}

/// The multipliers `(λi, λj)` with `v` along `m + λi ni + λj nj` (`ni`,
/// `nj` square to `m`): `v`'s parts along the three, from the 3 by 3 system,
/// scaled to `m`'s; `None` where `v` does not face along `m`.
fn corner_multipliers(v: Vec3, m: Vec3, ni: Vec3, nj: Vec3) -> Option<(f64, f64)> {
    let det = m.dot(ni.cross(nj));
    if det.abs() < 1e-12 {
        return None;
    }
    let a = v.dot(ni.cross(nj)) / det;
    let b = m.dot(v.cross(nj)) / det;
    let c = m.dot(ni.cross(v)) / det;
    (a > 1e-12).then(|| (b / a, c / a))
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
        if crate::surface::closest_on_triangle(x, t).distance(x) <= 1e-9 * (1.0 + x.distance(t[0]))
        {
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
    /// The outward unit normal of the triangle the piece is part of.
    normal: Vec3,
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
    // A max-heap: the smallest bound comes out first, then the earliest
    // made (so among pieces held at the layer every triangle's turn comes,
    // where by triangle one whose pieces never settle would keep it), then
    // the lowest triangle.
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .lower
            .total_cmp(&self.lower)
            .then_with(|| other.order.cmp(&self.order))
            .then_with(|| other.triangle.cmp(&self.triangle))
    }
}

/// The tool gap over `triangles` (index and corners, in index order, each
/// wound counter-clockwise seen from outside the material), within `limit`:
/// `None` when no face has the tool within `limit` above it. A face counts
/// no tool point deeper than `limit` below it, so the gap is never below
/// `-limit`.
pub(crate) fn search(
    body: &Body<'_>,
    triangles: impl IntoIterator<Item = (u32, [Point3; 3])>,
    limit: f64,
    accept: Option<f64>,
    error: &dyn Fn(u32) -> f64,
    neighbours: &dyn Fn(u32) -> Vec<(Vec3, f64, bool)>,
) -> Option<ToolGap> {
    let accept = accept.filter(|a| a.is_finite()).unwrap_or(f64::INFINITY);
    let inside = if accept.is_finite() {
        (-accept).max(GAP_ACCURACY)
    } else {
        GAP_ACCURACY
    };
    let mut state = Search {
        body,
        queue: BinaryHeap::new(),
        order: 0,
        best: None,
        limit,
        error,
        neighbours,
        inside,
        clips: std::collections::HashMap::new(),
    };
    for (i, t) in triangles {
        if !t.iter().all(|p| p.is_finite()) {
            continue;
        }
        if let Ok(m) = UnitVec3::new((t[1] - t[0]).cross(t[2] - t[0])) {
            state.push(t, m.get(), i, (0.0, 1.0), f64::NAN);
        }
    }
    let moving = body.sweep.length() > 0.0 || body.turns();
    let mut pieces = 0_usize;
    let lower = loop {
        let found = state.best.map_or(f64::INFINITY, |b| b.0);
        let Some(piece) = state.queue.pop() else {
            break found;
        };
        if piece.lower >= found - GAP_ACCURACY || piece.lower >= accept || piece.lower > limit {
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
        // The bound is exact for one pose; over a part of a move it is
        // taken with the multipliers of one of its poses, so the part is
        // halved while the tool moves across the piece by more than a
        // tenth of what the bound lacks, or while nothing has been found
        // yet. Otherwise the piece is halved.
        let (s0, s1) = piece.span;
        let centre = centroid(&piece.corners);
        let travel = (s1 - s0)
            * (body.sweep.length()
                + body.angle.abs() * (centre.distance(body.pivot.0) + body.outline.top.abs()));
        // With nothing found yet there is no deficit to measure against:
        // the larger of the move's travel over the piece and the piece is
        // halved, so neither is halved for ever while the other stays whole.
        let split_move = moving
            && s1 - s0 > MIN_SPAN
            && if found.is_finite() {
                10.0 * travel >= found - piece.lower
            } else {
                travel >= len
            };
        if (len < MIN_PIECE && !split_move) || pieces >= MAX_PIECES {
            break piece.lower.min(found);
        }
        pieces += 2;
        if split_move {
            let sm = 0.5 * (s0 + s1);
            state.push(
                piece.corners,
                piece.normal,
                piece.triangle,
                (s0, sm),
                piece.hint,
            );
            state.push(
                piece.corners,
                piece.normal,
                piece.triangle,
                (sm, s1),
                piece.hint,
            );
        } else {
            let mid = midpoint(a, b);
            state.push(
                [a, mid, c],
                piece.normal,
                piece.triangle,
                piece.span,
                piece.hint,
            );
            state.push(
                [mid, b, c],
                piece.normal,
                piece.triangle,
                piece.span,
                piece.hint,
            );
        }
    };
    if std::env::var("GSTAT").is_ok() {
        use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
        static Q: AtomicU64 = AtomicU64::new(0);
        static P: AtomicU64 = AtomicU64::new(0);
        static CAP: AtomicU64 = AtomicU64::new(0);
        let q = Q.fetch_add(1, Relaxed) + 1;
        P.fetch_add(pieces as u64, Relaxed);
        if pieces >= MAX_PIECES {
            CAP.fetch_add(1, Relaxed);
            let mut ps: Vec<&Piece> = state.queue.iter().collect();
            ps.sort_by(|a, b| a.lower.total_cmp(&b.lower));
            eprintln!(
                "CAPPED lower {lower} found {:?} accept {accept} tip {:?} axis {:?} sweep {:?} angle {} queue {}",
                state.best.map(|b| b.0),
                body.tip,
                body.axis,
                body.sweep,
                body.angle,
                ps.len()
            );
            for p in ps.iter().take(4) {
                eprintln!(
                    "   piece lower {} tri {} span {:?} size {:.2e} normal {:?} corners {:?}",
                    p.lower,
                    p.triangle,
                    p.span,
                    longest_side(&p.corners).3,
                    p.normal,
                    p.corners
                );
                let f = Face::new(p.corners, p.normal);
                let c = centroid(&p.corners);
                eprintln!(
                    "     lower_surface {:?} column centre {:?} beside {} below {} top {}",
                    body.lower_surface(&f, &f.sides, limit, 1.0),
                    body.column_first(&f, c),
                    body.beside(&f, 0.0, 1.0),
                    body.below_layer(&f, limit, 0.0, 1.0),
                    body.top_over(&f, 0.0, 1.0)
                );
            }
            let mut tris: Vec<u32> = ps.iter().map(|p| p.triangle).collect();
            tris.sort();
            tris.dedup();
            eprintln!(
                "   triangles {} : {:?}",
                tris.len(),
                tris.iter().take(12).collect::<Vec<_>>()
            );
        }
        if q % 500 == 0 {
            eprintln!(
                "GSTAT queries {q} pieces/q {:.1} capped {}",
                P.load(Relaxed) as f64 / q as f64,
                CAP.load(Relaxed)
            );
        }
    }
    // The best was kept less its triangle's error, as the bounds are; the
    // exact face's value is at most the triangle's plus that error.
    // A search that ran out of pieces with nothing found has not shown the
    // tool clear: it answers its bound, with no real value above it.
    if state.best.is_none() && pieces >= MAX_PIECES && lower <= limit {
        return Some(ToolGap {
            lower,
            upper: f64::INFINITY,
            at: body.tip,
        });
    }
    let (found, at, e) = state.best?;
    (lower <= limit).then_some(ToolGap {
        lower: lower.min(found),
        upper: found + 2.0 * e,
        at,
    })
}

/// The state of one search: the queue of pieces and the best value found.
struct Search<'a, 'b> {
    body: &'a Body<'b>,
    queue: BinaryHeap<Piece>,
    order: u64,
    /// The best value found less its triangle's error, its point and that
    /// error.
    best: Option<(f64, Point3, f64)>,
    limit: f64,
    /// Each triangle's largest distance from the exact surface it stands
    /// for (zero for a mesh, which is its own surface): taken off every
    /// bound and value over the triangle, so the answer holds the exact
    /// surface's gap.
    error: &'a dyn Fn(u32) -> f64,
    /// The planes of the faces meeting each triangle at a corner, and
    /// whether each meets it at a convex edge or corner (see [`Face`]).
    neighbours: &'a dyn Fn(u32) -> Vec<(Vec3, f64, bool)>,
    /// How far below a convex neighbour's plane a point must be for a face
    /// to count it: the query's acceptance, at least [`GAP_ACCURACY`].
    inside: f64,
    /// Each triangle's clips, its convex neighbours' planes moved in by
    /// `inside`, and every neighbour's normal.
    clips: std::collections::HashMap<u32, (Vec<(Vec3, f64)>, Vec<Vec3>)>,
}

impl Search<'_, '_> {
    /// The face of `corners` on triangle `triangle`, with the clips that
    /// matter there: a clip every point of the piece's prism from its face
    /// down to the layer already satisfies cuts nothing off what the face
    /// counts below itself (the only part its clips are used for, see
    /// [`Search::measure`]), so it is left out.
    fn face(&mut self, corners: [Point3; 3], m: Vec3, triangle: u32) -> Face {
        let (neighbours, inside, limit) = (self.neighbours, self.inside, self.limit);
        let (clips, around) = self.clips.entry(triangle).or_insert_with(|| {
            let all = neighbours(triangle);
            (
                all.iter()
                    .filter(|(_, _, convex)| *convex)
                    .map(|(n, d, _)| (*n, d - inside))
                    .collect(),
                all.iter().map(|(n, _, _)| *n).collect(),
            )
        });
        let floor = corners.map(|p| p + m * (-limit));
        let slack = 1e-9 * (1.0 + (corners[0] - Point3::ORIGIN).length());
        let relevant = clips
            .iter()
            .filter(|(n, b)| {
                corners
                    .iter()
                    .chain(floor.iter())
                    .any(|p| n.dot(*p - Point3::ORIGIN) > b - slack)
            })
            .copied()
            .collect();
        Face::new(corners, m)
            .with_clips(relevant)
            .with_around(around)
    }

    /// Bounds a piece and queues it, unless it cannot count. A tool point
    /// the face counts is as far from the face as its height, so a piece
    /// the tool is proven farther than `limit` from (by the capsule holding
    /// the tool, or the plane facing it from the tool's spine) has no gap
    /// within `limit`; nor has one the whole tool is deeper than `limit`
    /// below. Otherwise the bound is the dual's with no multipliers: the
    /// lowest the tool reaches below the face's plane anywhere.
    fn push(&mut self, corners: [Point3; 3], m: Vec3, triangle: u32, span: (f64, f64), hint: f64) {
        let (s0, s1) = span;
        let e = (self.error)(triangle);
        let centre = centroid(&corners);
        let far = self
            .body
            .far_over(&corners, s0, s1)
            .max(self.body.facing(&corners, centre, s0, s1));
        if far - e > self.limit {
            return;
        }
        // The open region: a tool beyond a clip may still be above the face
        // (see [`Search::measure`]).
        let face = Face {
            clips: Vec::new(),
            ..self.face(corners, m, triangle)
        };
        if self.body.top_over(&face, s0, s1) < -self.limit || self.body.beside(&face, s0, s1) {
            return;
        }
        let lower = self.body.dual_over(face.m, face.d, s0, s1).max(-self.limit) - e;
        self.order += 1;
        self.queue.push(Piece {
            lower,
            span,
            hint,
            measured: false,
            triangle,
            order: self.order,
            corners,
            normal: m,
        });
    }

    /// Measures a piece at the ends and middle of its part of the move
    /// (and its hint): the best multipliers at each pose give a proven
    /// bound over the whole part, the best of which is kept, and the lowest
    /// Measures a piece at a single pose (no shift or turn) by the tool's
    /// lower surface along the face's normal and its columns, without the
    /// dual (the owner's confirmation of 27 Nov 2026: at the move check's
    /// single poses the dual's multipliers grow without end where the tool
    /// runs along a face's edge or lies beside it). The bound is the best
    /// of planes touching the tool from below, each at the piece's corners
    /// ([`Body::cheap_planes`]), and where that is short of what the caller
    /// accepts, over the corners of the polytope of the region and the
    /// plane, which may show it empty; real values are the columns through
    /// the piece's centre and corners. The open and clipped regions are
    /// counted as in [`Self::measure`].
    fn measure_single(&mut self, piece: Piece) -> Piece {
        let clipped = self.face(piece.corners, piece.normal, piece.triangle);
        let open = Face {
            clips: Vec::new(),
            ..clipped.clone()
        };
        let e = (self.error)(piece.triangle);
        let empty = Piece {
            lower: f64::INFINITY,
            measured: true,
            ..piece
        };
        if self.body.beside(&open, 0.0, 0.0) {
            return empty;
        }
        let top = self.body.top_over(&open, 0.0, 0.0);
        if top < -self.limit {
            return empty;
        }
        let planes = self.body.cheap_planes(&open);
        // At the corners only a plane facing along the face's normal bounds
        // the height; one square to it (a tool beside the piece) counts in
        // the polytope's bound, where it is a side.
        let at_corners = |(n, k): &(Vec3, f64)| {
            let along = n.dot(open.m);
            if along <= 1e-9 {
                return f64::NEG_INFINITY;
            }
            open.corners
                .iter()
                .map(|x| (k - n.dot(*x - Point3::ORIGIN)) / along)
                .fold(f64::INFINITY, f64::min)
        };
        let mut open_lower = planes
            .iter()
            .map(at_corners)
            .fold(self.body.dual_over(open.m, open.d, 0.0, 0.0), f64::max);
        // The foot of a polytope's lowest corner: a place for a real value.
        let foot =
            |face: &Face, y: Point3| y + face.m * (-(face.m.dot(y - Point3::ORIGIN) - face.d));
        let mut probes: Vec<(bool, Point3)> = Vec::new();
        if open_lower < -self.inside {
            let region: Vec<(Vec3, f64)> = open.sides.to_vec();
            let (v, at) =
                self.body
                    .polytope_bound(&open, &region, &planes, self.limit, top.max(0.0));
            if v == f64::INFINITY {
                return empty;
            }
            open_lower = open_lower.max(v);
            probes.extend(at.map(|y| (false, foot(&open, y))));
        }
        let centre = centroid(&open.corners);
        let columns = |face: &Face| {
            std::iter::once(centre)
                .chain(face.corners)
                .filter_map(|x| self.body.column(face, x, 0.0, self.limit))
                .fold(None, |best: Option<(f64, Point3)>, c| match best {
                    Some(b) if b.0 <= c.0 => Some(b),
                    _ => Some(c),
                })
        };
        // And the tool's own lowest point along the face's normal, with the
        // column nearest its foot.
        let free = self.body.found(&open, &[0.0; 3], 0.0, self.limit);
        let probe = |face: &Face, list: &[(bool, Point3)], clipped: bool| {
            list.iter()
                .filter(|(c, _)| *c == clipped)
                .filter_map(|(_, x)| self.body.column(face, *x, 0.0, self.limit))
                .fold(None, |best: Option<(f64, Point3)>, c| match best {
                    Some(b) if b.0 <= c.0 => Some(b),
                    _ => Some(c),
                })
        };
        let lowest = |a: Option<(f64, Point3)>, b: Option<(f64, Point3)>| match (a, b) {
            (Some(a), Some(b)) => Some(if b.0 < a.0 { b } else { a }),
            (a, b) => a.or(b),
        };
        let near = match (lowest(columns(&open), probe(&open, &probes, false)), free) {
            (Some(a), Some(b)) => Some(if b.0 < a.0 { b } else { a }),
            (a, b) => a.or(b),
        };
        let has_clips = !clipped.clips.is_empty();
        let mut bound = open_lower;
        let mut value = near;
        if has_clips && open_lower < 0.0 {
            // Below the face only the clipped region counts; its value is
            // at most zero where the open region's is below the face.
            let clipped_lower = if open_lower >= -self.inside {
                0.0_f64.min(open_lower.max(-self.inside))
            } else if self.body.beside(&clipped, 0.0, 0.0) {
                f64::INFINITY
            } else {
                let region: Vec<(Vec3, f64)> = clipped.constraints().copied().collect();
                let cplanes = self.body.cheap_planes(&clipped);
                let (v, at) = self
                    .body
                    .polytope_bound(&clipped, &region, &cplanes, self.limit, 0.0);
                probes.extend(at.map(|y| (true, foot(&clipped, y))));
                v.max(open_lower)
            };
            bound = clipped_lower.min(0.0).max(open_lower);
            value = match near {
                Some((h, at)) if h >= 0.0 => Some((h, at)),
                Some((_, at)) => Some(
                    match lowest(columns(&clipped), probe(&clipped, &probes, true)) {
                        Some((hc, atc)) if hc < 0.0 => (hc, atc),
                        _ => (0.0, at),
                    },
                ),
                None => lowest(columns(&clipped), probe(&clipped, &probes, true))
                    .map(|(h, at)| (h.min(0.0), at)),
            };
        }
        if let Some((h, at)) = value
            && self.best.is_none_or(|(b, _, _)| h - e < b)
        {
            self.best = Some((h - e, at, e));
        }
        if std::env::var("MDBG").is_ok() {
            eprintln!(
                "MS tri {} size {:.3} open_lower {open_lower} bound {bound} near {near:?} value {value:?} planes {} top {top}",
                piece.triangle,
                longest_side(&piece.corners).3,
                planes.len()
            );
        }
        Piece {
            lower: piece.lower.max(bound.max(-self.limit) - e),
            measured: true,
            ..piece
        }
    }

    /// tool point on the face there a real value.
    fn measure(&mut self, piece: Piece) -> Piece {
        // The face's region without its clips, which counts the tool above
        // the face (its clearance) and bounds what the clipped region
        // counts, and with them, which counts the tool below it (see
        // [`Face`]): over the piece the gap is the open region's where that
        // is not below the face, else the clipped region's, or zero.
        if !self.body.turns() && self.body.sweep.length() <= 0.0 {
            return self.measure_single(piece);
        }
        let clipped = self.face(piece.corners, piece.normal, piece.triangle);
        let open = Face {
            clips: Vec::new(),
            ..clipped.clone()
        };
        let (s0, s1) = piece.span;
        let fm = 0.5 * (s0 + s1);
        let e = (self.error)(piece.triangle);
        let mut hint = piece.hint;
        let mut poses = vec![s0, fm, s1];
        if !self.body.turns() && self.body.sweep.length() <= 0.0 {
            poses.truncate(1);
        }
        if piece.hint.is_finite() && piece.hint > s0 && piece.hint < s1 {
            poses.push(piece.hint);
        }
        let empty = Piece {
            lower: f64::INFINITY,
            measured: true,
            ..piece
        };
        if self.body.beside(&open, s0, s1) {
            return empty;
        }
        let has_clips = !clipped.clips.is_empty();
        let (mut open_lower, mut clipped_lower) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        let mut near: Option<(f64, Point3, f64)> = None;
        let mut near_clipped: Option<(f64, Point3)> = None;
        for &f in &poses {
            // Nothing counts when the whole tool over the face is deeper
            // than the layer.
            let top_with = |mu: Vec<f64>| {
                scaled(&mu)
                    .map(|mu| {
                        let (v, c) = open.dual_top(&mu);
                        -self.body.dual_over(-v, -c, s0, s1)
                    })
                    .fold(f64::INFINITY, f64::min)
            };
            let mut top = top_with(self.body.multipliers(&open, f, true));
            if top >= -self.limit && self.body.below_layer(&open, self.limit, s0, s1) {
                top = f64::NEG_INFINITY;
            }
            if top < -self.limit {
                return empty;
            }
            let bound_of = |face: &Face, lam: &[f64]| {
                scaled(lam)
                    .map(|lam| {
                        let (w, c) = face.dual(&lam);
                        self.body.dual_over(w, c, s0, s1)
                    })
                    .fold(f64::NEG_INFINITY, f64::max)
            };
            let lam = self.body.multipliers(&open, f, false);
            open_lower = open_lower.max(bound_of(&open, &lam));
            if let Some((h, at)) = self.body.found(&open, &lam, f, self.limit)
                && near.is_none_or(|(b, _, _)| h < b)
            {
                near = Some((h, at, f));
            }
        }
        // At a single pose, where the dual leaves the piece short, the
        // tool's lower surface may bound it better.
        let single = !self.body.turns() && self.body.sweep.length() <= 0.0;
        if single && open_lower < -self.inside {
            let top = self.body.top_over(&open, s0, s1);
            let region: Vec<(Vec3, f64)> = open.sides.to_vec();
            let v = self
                .body
                .lower_surface(&open, &region, self.limit, top.max(0.0));
            if v == f64::INFINITY {
                return empty;
            }
            open_lower = open_lower.max(v);
        }
        // Below the face only the clipped region counts. Its bound is only
        // ever higher than the open region's, so it is needed only where
        // that is short of what the caller accepts.
        // And only where a real point below the face is found: where the open
        // bound is short only because it is loose (the piece is large, or
        // its part of the move long), halving settles it, and the clipped
        // bound is no better.
        let short = open_lower < -self.inside;
        let wanted = short && near.is_some_and(|(h, _, _)| h < 0.0);
        // Whether the clipped region is empty is cheap to ask: ask it
        // wherever the open bound is short.
        let clipped_beside = has_clips && short && self.body.beside(&clipped, s0, s1);
        if clipped_beside {
            clipped_lower = f64::INFINITY;
        }
        if has_clips && wanted && !clipped_beside && single {
            let region: Vec<(Vec3, f64)> = clipped.constraints().copied().collect();
            clipped_lower =
                clipped_lower.max(self.body.lower_surface(&clipped, &region, self.limit, 0.0));
        }
        if has_clips && wanted && !clipped_beside {
            for &f in &poses {
                let lam = self.body.multipliers(&clipped, f, false);
                let bound = scaled(&lam)
                    .map(|lam| {
                        let (w, c) = clipped.dual(&lam);
                        self.body.dual_over(w, c, s0, s1)
                    })
                    .fold(f64::NEG_INFINITY, f64::max);
                clipped_lower = clipped_lower.max(bound);
                if let Some((h, at)) = self.body.found(&clipped, &lam, f, self.limit)
                    && near_clipped.is_none_or(|(b, _)| h < b)
                {
                    near_clipped = Some((h, at));
                }
            }
        }
        let bound = if !has_clips || open_lower >= 0.0 || !(wanted || clipped_beside) {
            open_lower
        } else {
            clipped_lower.min(0.0).max(open_lower)
        };
        let lower = piece.lower.max(bound.max(-self.limit) - e);
        // A real value: the open region's lowest point where it is not
        // below the face; else the clipped region's, at most zero (the
        // piece's gap is at most the open region's, held at zero).
        let value = match near {
            Some((h, at, f)) if !has_clips || h >= 0.0 => Some((h, at, f)),
            Some((_, at, f)) => Some(match near_clipped {
                Some((hc, atc)) if hc < 0.0 => (hc, atc, f),
                _ => (0.0, at, f),
            }),
            None => near_clipped.map(|(h, at)| (h.min(0.0), at, fm)),
        };
        if let Some((h, at, f)) = value {
            hint = f;
            if self.best.is_none_or(|(b, _, _)| h - e < b) {
                self.best = Some((h - e, at, e));
            }
        }
        Piece {
            lower,
            measured: true,
            hint,
            ..piece
        }
    }
}

/// The multipliers best for one pose, then the same direction scaled down
/// to sizes from [`MULTIPLIER_SCALES`]: every choice of multipliers gives a
/// proven bound, and over a part of a move the slack for the motion grows
/// with their size, so where the best for one pose is large (the tool
/// touching a side of the prism, where they grow without end) a smaller
/// choice can bound the whole part better.
fn scaled(lam: &[f64]) -> impl Iterator<Item = Vec<f64>> + '_ {
    let size = lam.iter().map(|l| l.abs()).fold(0.0, f64::max);
    std::iter::once(lam.to_vec()).chain(
        MULTIPLIER_SCALES
            .iter()
            .filter(move |k| **k < size)
            .map(move |k| lam.iter().map(|l| l * (k / size)).collect()),
    )
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
    error: &dyn Fn(u32) -> f64,
    largest: f64,
) -> Option<ToolGap> {
    if !line.origin.is_finite() || !query.limit.is_finite() {
        return None;
    }
    let outline = Outline::new(profile);
    let body = Body::new(&outline, line, query);
    let near = body.bounds(query.limit + largest);
    let triangles = (0..mesh.triangles.len()).filter_map(|i| {
        let t = mesh.triangle(i)?;
        let index = u32::try_from(i).ok()?;
        let b = triangle_bounds(&t);
        let reach = query.limit + largest;
        (overlaps(&near, &b) && body.near(&b, reach) && body.far_from(&t) <= reach)
            .then_some((index, t))
    });
    let neighbours = convex_neighbours(mesh);
    search(&body, triangles, query.limit, query.accept, error, &|i| {
        neighbours.get(i as usize).cloned().unwrap_or_default()
    })
}

/// The planes `(n, d)` (`n . y = d`, `n` the outward unit normal) of the
/// faces that meet each triangle of `mesh` at a corner, not in its plane,
/// each with whether it meets it at a convex edge or corner (see [`Face`]):
/// each lying on or below the other's plane. Corners are shared where their
/// coordinates are the same. In each triangle's list the neighbours are in
/// index order (NFR-06). A mesh with cracks there finds fewer convex ones,
/// and so counts more: never less.
pub(crate) fn convex_neighbours(mesh: &TriangleMesh) -> Vec<Vec<(Vec3, f64, bool)>> {
    let key = |p: Point3| {
        [
            (p.x + 0.0).to_bits(),
            (p.y + 0.0).to_bits(),
            (p.z + 0.0).to_bits(),
        ]
    };
    let mut ids: std::collections::HashMap<[u64; 3], usize> = std::collections::HashMap::new();
    let tris: Vec<Option<([usize; 3], [Point3; 3], Vec3, f64)>> = (0..mesh.triangles.len())
        .map(|i| {
            let t = mesh.triangle(i)?;
            let n = UnitVec3::new((t[1] - t[0]).cross(t[2] - t[0])).ok()?.get();
            let v = t.map(|p| {
                let next = ids.len();
                *ids.entry(key(p)).or_insert(next)
            });
            Some((v, t, n, n.dot(t[0] - Point3::ORIGIN)))
        })
        .collect();
    let mut at: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
    for (i, t) in tris.iter().enumerate() {
        if let Some((v, ..)) = t {
            for k in v {
                at.entry(*k).or_default().push(i);
            }
        }
    }
    let scale = mesh
        .vertices
        .iter()
        .map(|p| p.x.abs().max(p.y.abs()).max(p.z.abs()))
        .fold(1.0, f64::max);
    let eps = 1e-9 * scale;
    tris.iter()
        .enumerate()
        .map(|(i, t)| {
            let Some((v, pts, n, d)) = t else {
                return Vec::new();
            };
            let mut others: Vec<usize> = v
                .iter()
                .filter_map(|k| at.get(k))
                .flatten()
                .copied()
                .filter(|j| *j != i)
                .collect();
            others.sort_unstable();
            others.dedup();
            others
                .into_iter()
                .filter_map(|j| {
                    let (_, q, m, e) = tris.get(j)?.as_ref()?;
                    if m.dot(*n) > 1.0 - 1e-12 {
                        return None;
                    }
                    let below = |pts: &[Point3; 3], n: Vec3, d: f64| {
                        pts.iter().all(|p| n.dot(*p - Point3::ORIGIN) - d <= eps)
                    };
                    Some((*m, *e, below(q, *n, *d) && below(pts, *m, *e)))
                })
                .collect()
        })
        .collect()
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
