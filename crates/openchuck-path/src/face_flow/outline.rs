// SPDX-License-Identifier: AGPL-3.0-or-later
//! Follow the outline: passes that follow a face's own edges, each the
//! stepover inside the last, measured on the face (issue #791; the owner's
//! answers of 23 Nov 2026 to Q1 to Q4).
//!
//! The face is gridded in its own (u, v), the grid's points put on the
//! exact face. Each grid point gets its distance on the part to the nearest
//! point of the face's outline (its edges, sampled finely; a seam is not
//! part of it), found by propagating the nearest outline sample from
//! neighbour to neighbour. Inside a face the nearest point of its outline is
//! always in plain sight, so this is the distance an offset of the outline
//! is measured by. The first pass is the outline itself; the others are the
//! grid's level lines at whole stepovers (marching squares). The passes are
//! cut from the outline inwards, each started at its point nearest where
//! the last ended, and the tool steps across the face from one to the next
//! where the step is clear (Stay down between passes).

use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap};

use super::{
    Closure, Curve, Direction, FaceCut, MAX_GUIDE_SAMPLES, MAX_PASSES, PassLine, SAMPLE_SPACING,
    Stretch, Tooling, append, concat, edge_points, loops, pass_samples, split_runs, step_on,
};
use crate::brep::ExactFace;
use crate::error::{PathError, Result};
use crate::rotary::nearest_turn;
use crate::strategy::CutPoint;
use crate::surface::{ParametricFace, SurfacePoint, Uv};
use openchuck_types::brep::{BrepFace, BrepModel, EdgeId, FaceId};
use openchuck_types::geometry::{Point3, Vec3};
use openchuck_types::job::JobContext;
use openchuck_types::tool::profile::RevolvedProfile;

/// The grid's spacing as a share of the stepover, and its bounds in
/// millimetres.
const GRID_SHARE: f64 = 0.25;
const GRID_MIN: f64 = 0.05;
const GRID_MAX: f64 = 0.5;

/// Most grid points across a face in either direction, and in all (NFR-07).
const MAX_GRID_SIDE: usize = 1500;
const MAX_GRID: usize = 1_500_000;

// ---------------------------------------------------------------------------
// A pass along a polyline in the face's (u, v).

/// A pass following a polyline in the face's (u, v), taken by its length on
/// the part.
pub(super) struct UvLine<'a> {
    face: &'a ExactFace,
    uv: Vec<Uv>,
    cum: Vec<f64>,
    wrap: [Option<(f64, f64)>; 2],
}

fn wrap_back(x: f64, wrap: Option<(f64, f64)>) -> f64 {
    match wrap {
        Some((start, period)) if period > 0.0 => start + (x - start).rem_euclid(period),
        _ => x,
    }
}

impl<'a> UvLine<'a> {
    fn new(face: &'a ExactFace, uv: Vec<Uv>, wrap: [Option<(f64, f64)>; 2]) -> Self {
        let mut line = Self {
            face,
            uv,
            cum: Vec::new(),
            wrap,
        };
        let mut total = 0.0;
        let mut last: Option<Point3> = None;
        let mut cum = Vec::with_capacity(line.uv.len());
        for k in 0..line.uv.len() {
            let p = line.uv.get(k).and_then(|uv| line.point(*uv));
            if let (Some(a), Some(b)) = (last, p) {
                total += a.distance(b);
            }
            if p.is_some() {
                last = p;
            }
            cum.push(total);
        }
        line.cum = cum;
        line
    }

    fn wrapped(&self, uv: Uv) -> Uv {
        let [wu, wv] = self.wrap;
        Uv {
            u: wrap_back(uv.u, wu),
            v: wrap_back(uv.v, wv),
        }
    }

    fn point(&self, uv: Uv) -> Option<Point3> {
        self.face.point_normal(self.wrapped(uv)).map(|p| p.point)
    }

    fn length(&self) -> f64 {
        self.cum.last().copied().unwrap_or(0.0)
    }

    /// The (u, v) at fraction `s` of the length.
    fn uv_at(&self, s: f64) -> Option<Uv> {
        let total = self.length();
        let first = *self.uv.first()?;
        if total <= 0.0 {
            return Some(first);
        }
        let target = s.clamp(0.0, 1.0) * total;
        let i = self.cum.partition_point(|c| *c < target);
        if i == 0 {
            return Some(first);
        }
        let (a, b) = (*self.uv.get(i - 1)?, *self.uv.get(i)?);
        let (ca, cb) = (*self.cum.get(i - 1)?, *self.cum.get(i)?);
        let f = if cb > ca {
            (target - ca) / (cb - ca)
        } else {
            0.0
        };
        Some(Uv {
            u: a.u + (b.u - a.u) * f,
            v: a.v + (b.v - a.v) * f,
        })
    }
}

impl PassLine for UvLine<'_> {
    fn m(&self) -> usize {
        ((self.length() / SAMPLE_SPACING).ceil() as usize).clamp(16, MAX_GUIDE_SAMPLES)
    }

    fn at(&self, s: f64) -> Option<SurfacePoint> {
        let uv = self.uv_at(s)?;
        self.face.point_normal(self.wrapped(uv))
    }
}

/// A straight line in (u, v) between two points of the face, which must
/// stay on the face: the step from one offset to the next.
struct UvStep<'a> {
    face: &'a ExactFace,
    a: Uv,
    b: Uv,
    wrap: [Option<(f64, f64)>; 2],
}

impl PassLine for UvStep<'_> {
    fn m(&self) -> usize {
        1
    }

    fn at(&self, t: f64) -> Option<SurfacePoint> {
        let [wu, wv] = self.wrap;
        let unwrap = |x: f64, near: f64, w: Option<(f64, f64)>| match w {
            Some((_, p)) if p > 0.0 => x + ((near - x) / p).round() * p,
            _ => x,
        };
        let b = Uv {
            u: unwrap(self.b.u, self.a.u, wu),
            v: unwrap(self.b.v, self.a.v, wv),
        };
        let uv = Uv {
            u: wrap_back(self.a.u + (b.u - self.a.u) * t, wu),
            v: wrap_back(self.a.v + (b.v - self.a.v) * t, wv),
        };
        if !self.face.contains(uv) {
            return None;
        }
        self.face.point_normal(uv)
    }
}

// ---------------------------------------------------------------------------
// The distance to the outline over a grid.

/// The face gridded in its (u, v): `ni` by `nj` points, each its point on the
/// face (or `None` off the face) and its distance to the outline.
struct Grid {
    ni: usize,
    nj: usize,
    /// Cells across in u and v (equal to the points in a direction the face
    /// closes in, one less otherwise).
    nu: usize,
    nv: usize,
    /// The spacing of the grid's points on the part, about.
    h: f64,
    wrap_u: bool,
    wrap_v: bool,
    lo: Uv,
    hi: Uv,
    points: Vec<Option<Point3>>,
    distance: Vec<f64>,
}

/// A grid point waiting in the propagation: its distance and index.
#[derive(Debug, Clone, Copy)]
struct Queued {
    d: f64,
    node: usize,
}

impl PartialEq for Queued {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Queued {}

impl PartialOrd for Queued {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Queued {
    // Smallest distance first, then lowest index: a deterministic order.
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .d
            .total_cmp(&self.d)
            .then_with(|| other.node.cmp(&self.node))
    }
}

impl Grid {
    fn index(&self, i: usize, j: usize) -> usize {
        j * self.ni + i
    }

    /// The (u, v) at grid coordinates (x, y), not wrapped.
    fn uv(&self, x: f64, y: f64) -> Uv {
        Uv {
            u: self.lo.u + (self.hi.u - self.lo.u) * x / self.nu as f64,
            v: self.lo.v + (self.hi.v - self.lo.v) * y / self.nv as f64,
        }
    }

    /// The grid coordinates of (u, v).
    fn coords(&self, uv: Uv) -> (f64, f64) {
        let x = (uv.u - self.lo.u) / (self.hi.u - self.lo.u) * self.nu as f64;
        let y = (uv.v - self.lo.v) / (self.hi.v - self.lo.v) * self.nv as f64;
        (x, y)
    }

    /// The point index at (i, j), wrapping where the face closes; `None`
    /// off the grid.
    fn at(&self, i: isize, j: isize) -> Option<usize> {
        let fold = |k: isize, n: usize, wrap: bool| -> Option<usize> {
            if wrap {
                Some(k.rem_euclid(n as isize) as usize)
            } else if k >= 0 && (k as usize) < n {
                Some(k as usize)
            } else {
                None
            }
        };
        Some(self.index(
            fold(i, self.ni, self.wrap_u)?,
            fold(j, self.nj, self.wrap_v)?,
        ))
    }

    /// The distance at grid coordinates (x, y), bilinear over the points on
    /// the face; `None` off it.
    fn distance_at(&self, x: f64, y: f64) -> Option<f64> {
        let (i, j) = (x.floor() as isize, y.floor() as isize);
        let (fx, fy) = (x - x.floor(), y - y.floor());
        let mut sum = 0.0;
        let mut weight = 0.0;
        for (di, dj, w) in [
            (0, 0, (1.0 - fx) * (1.0 - fy)),
            (1, 0, fx * (1.0 - fy)),
            (0, 1, (1.0 - fx) * fy),
            (1, 1, fx * fy),
        ] {
            let Some(k) = self.at(i + di, j + dj) else {
                continue;
            };
            let d = *self.distance.get(k)?;
            if d.is_finite() {
                sum += d * w;
                weight += w;
            }
        }
        (weight > 1e-9).then(|| sum / weight)
    }
}

/// The length on the face of the line of constant `v` (or `u`, `along_u`
/// false) through the face's box at `t` of the way across.
fn line_length(face: &ExactFace, lo: Uv, hi: Uv, along_u: bool, t: f64) -> f64 {
    let mut total = 0.0;
    let mut last: Option<Point3> = None;
    for k in 0..=64 {
        let f = f64::from(k) / 64.0;
        let uv = if along_u {
            Uv {
                u: lo.u + (hi.u - lo.u) * f,
                v: lo.v + (hi.v - lo.v) * t,
            }
        } else {
            Uv {
                u: lo.u + (hi.u - lo.u) * t,
                v: lo.v + (hi.v - lo.v) * f,
            }
        };
        let p = face.point_normal(uv).map(|p| p.point);
        if let (Some(a), Some(b)) = (last, p) {
            total += a.distance(b);
        }
        last = p;
    }
    total
}

/// The outline's points, no more than `h` apart: every edge of the face but
/// its seams.
fn outline_samples(model: &BrepModel, face: &BrepFace, closed: &Closure, h: f64) -> Vec<Point3> {
    let mut out = Vec::new();
    for id in &face.edges {
        if closed.seams.contains(id) {
            continue;
        }
        let Some((points, _, _)) = edge_points(model, *id) else {
            continue;
        };
        for w in points.windows(2) {
            let (a, b) = (w[0], w[1]);
            let n = ((a.distance(b) / h).ceil() as usize).max(1);
            for k in 0..n {
                out.push(a + (b - a) * (k as f64 / n as f64));
            }
        }
        out.extend(points.last().copied());
    }
    out
}

fn grid(
    model: &BrepModel,
    brep: &BrepFace,
    face: &ExactFace,
    closed: &Closure,
    stepover: f64,
    context: &JobContext<'_>,
) -> Result<Grid> {
    let (lo, hi) = face.uv_bounds();
    let lu = [0.25, 0.5, 0.75]
        .iter()
        .map(|t| line_length(face, lo, hi, true, *t))
        .fold(0.0, f64::max);
    let lv = [0.25, 0.5, 0.75]
        .iter()
        .map(|t| line_length(face, lo, hi, false, *t))
        .fold(0.0, f64::max);
    let mut h = (GRID_SHARE * stepover).clamp(GRID_MIN, GRID_MAX);
    let cells = |len: f64, h: f64| ((len / h).ceil() as usize).clamp(8, MAX_GRID_SIDE);
    while cells(lu, h) * cells(lv, h) > MAX_GRID {
        h *= 1.25;
    }
    let (nu, nv) = (cells(lu, h), cells(lv, h));
    let [cu, cv] = closed.period;
    let (wrap_u, wrap_v) = (cu.is_some(), cv.is_some());
    let ni = if wrap_u { nu } else { nu + 1 };
    let nj = if wrap_v { nv } else { nv + 1 };
    let mut g = Grid {
        ni,
        nj,
        nu,
        nv,
        h,
        wrap_u,
        wrap_v,
        lo,
        hi,
        points: Vec::with_capacity(ni * nj),
        distance: vec![f64::INFINITY; ni * nj],
    };
    for j in 0..nj {
        context.check_cancelled()?;
        for i in 0..ni {
            let uv = g.uv(i as f64, j as f64);
            let p = if face.contains(uv) {
                face.point_normal(uv).map(|p| p.point)
            } else {
                None
            };
            g.points.push(p);
        }
    }
    // Seed the points beside each outline sample, then pass the nearest
    // sample on from neighbour to neighbour.
    let samples = outline_samples(model, brep, closed, 0.5 * h);
    let mut seed: Vec<usize> = vec![usize::MAX; ni * nj];
    let mut heap: BinaryHeap<Queued> = BinaryHeap::new();
    let mut last_uv: Option<Uv> = None;
    for (k, q) in samples.iter().enumerate() {
        let found = match last_uv {
            Some(prev) => face.closest_from(*q, prev, f64::INFINITY),
            None => face.closest(*q),
        };
        let Some(c) = found else {
            continue;
        };
        last_uv = Some(c.uv);
        let (x, y) = g.coords(c.uv);
        let (i0, j0) = (x.floor() as isize, y.floor() as isize);
        for di in -1..=2 {
            for dj in -1..=2 {
                let Some(node) = g.at(i0 + di, j0 + dj) else {
                    continue;
                };
                let Some(p) = g.points.get(node).copied().flatten() else {
                    continue;
                };
                let d = p.distance(*q);
                if let Some(slot) = g.distance.get_mut(node)
                    && d < *slot
                {
                    *slot = d;
                    if let Some(s) = seed.get_mut(node) {
                        *s = k;
                    }
                    heap.push(Queued { d, node });
                }
            }
        }
    }
    let mut settled = vec![false; ni * nj];
    while let Some(Queued { d, node }) = heap.pop() {
        if settled.get(node).copied().unwrap_or(true)
            || d > g.distance.get(node).copied().unwrap_or(0.0)
        {
            continue;
        }
        if let Some(s) = settled.get_mut(node) {
            *s = true;
        }
        let (i, j) = ((node % ni) as isize, (node / ni) as isize);
        let from = seed.get(node).copied().unwrap_or(usize::MAX);
        let Some(q) = samples.get(from).copied() else {
            continue;
        };
        for (di, dj) in [
            (-1, 0),
            (1, 0),
            (0, -1),
            (0, 1),
            (-1, -1),
            (1, -1),
            (-1, 1),
            (1, 1),
        ] {
            let Some(nb) = g.at(i + di, j + dj) else {
                continue;
            };
            let Some(p) = g.points.get(nb).copied().flatten() else {
                continue;
            };
            let candidate = p.distance(q);
            if let Some(slot) = g.distance.get_mut(nb)
                && candidate < *slot - 1e-12
            {
                *slot = candidate;
                if let Some(s) = seed.get_mut(nb) {
                    *s = from;
                }
                heap.push(Queued {
                    d: candidate,
                    node: nb,
                });
            }
        }
    }
    // Off the face, and on it where no outline sample reached: no distance.
    for (k, d) in g.distance.iter_mut().enumerate() {
        if g.points.get(k).copied().flatten().is_none() || !d.is_finite() {
            *d = f64::NAN;
        }
    }
    Ok(g)
}

// ---------------------------------------------------------------------------
// The level lines.

/// One path of a pass, in grid coordinates, and whether it closes.
#[derive(Debug, Clone)]
struct Path {
    xy: Vec<(f64, f64)>,
    closed: bool,
    level: usize,
}

/// The level lines of the grid's distance at `value` (marching squares):
/// each a polyline in grid coordinates, closed or open.
fn level_lines(g: &Grid, value: f64, level: usize) -> Vec<Path> {
    let v = |i: isize, j: isize| -> f64 {
        g.at(i, j)
            .and_then(|k| g.distance.get(k).copied())
            .filter(|d| d.is_finite())
            .unwrap_or(-1.0)
    };
    let ci = if g.wrap_u {
        g.ni
    } else {
        g.ni.saturating_sub(1)
    };
    let cj = if g.wrap_v {
        g.nj
    } else {
        g.nj.saturating_sub(1)
    };
    // Each crossing is keyed by the two grid points of its edge.
    type Key = (usize, usize);
    let mut points: BTreeMap<Key, (f64, f64)> = BTreeMap::new();
    let mut segments: Vec<(Key, Key)> = Vec::new();
    for j in 0..cj {
        for i in 0..ci {
            let (ii, jj) = (i as isize, j as isize);
            let corners = [(ii, jj), (ii + 1, jj), (ii + 1, jj + 1), (ii, jj + 1)];
            let vals: Vec<f64> = corners.iter().map(|(a, b)| v(*a, *b)).collect();
            let inside: Vec<bool> = vals.iter().map(|x| *x >= value).collect();
            let case = inside
                .iter()
                .enumerate()
                .fold(0u8, |c, (k, on)| c | (u8::from(*on) << k));
            if case == 0 || case == 15 {
                continue;
            }
            // The crossing on the edge from corner a to corner b.
            let mut crossing = |a: usize, b: usize| -> Option<Key> {
                let (pa, pb) = (*corners.get(a)?, *corners.get(b)?);
                let (ka, kb) = (g.at(pa.0, pa.1)?, g.at(pb.0, pb.1)?);
                let (va, vb) = (*vals.get(a)?, *vals.get(b)?);
                let t = if (vb - va).abs() > 1e-15 {
                    (value - va) / (vb - va)
                } else {
                    0.5
                };
                let key = (ka.min(kb), ka.max(kb));
                points.entry(key).or_insert((
                    pa.0 as f64 + (pb.0 - pa.0) as f64 * t,
                    pa.1 as f64 + (pb.1 - pa.1) as f64 * t,
                ));
                Some(key)
            };
            // Edges: 0 bottom (0-1), 1 right (1-2), 2 top (2-3), 3 left (3-0).
            let edges_of = |e: usize| match e {
                0 => (0, 1),
                1 => (1, 2),
                2 => (2, 3),
                _ => (3, 0),
            };
            let crossed: Vec<usize> = (0..4)
                .filter(|e| {
                    let (a, b) = edges_of(*e);
                    inside.get(a) != inside.get(b)
                })
                .collect();
            let pairs: Vec<(usize, usize)> = match crossed.as_slice() {
                [a, b] => vec![(*a, *b)],
                [_, _, _, _] => {
                    // A saddle: joined by the centre's side.
                    let centre = vals.iter().sum::<f64>() / 4.0;
                    let corner0_in = inside.first().copied().unwrap_or(false);
                    if (centre >= value) == corner0_in {
                        vec![(0, 1), (2, 3)]
                    } else {
                        vec![(0, 3), (1, 2)]
                    }
                }
                _ => Vec::new(),
            };
            for (ea, eb) in pairs {
                let (a0, a1) = edges_of(ea);
                let (b0, b1) = edges_of(eb);
                if let (Some(ka), Some(kb)) = (crossing(a0, a1), crossing(b0, b1))
                    && ka != kb
                {
                    segments.push((ka, kb));
                }
            }
        }
    }
    // Chain the segments end to end.
    let mut by_key: BTreeMap<Key, Vec<usize>> = BTreeMap::new();
    for (k, (a, b)) in segments.iter().enumerate() {
        by_key.entry(*a).or_default().push(k);
        by_key.entry(*b).or_default().push(k);
    }
    let mut used = vec![false; segments.len()];
    let mut out = Vec::new();
    for start in 0..segments.len() {
        if used.get(start).copied().unwrap_or(true) {
            continue;
        }
        if let Some(u) = used.get_mut(start) {
            *u = true;
        }
        let Some(&(a, b)) = segments.get(start) else {
            continue;
        };
        let mut chain: Vec<Key> = vec![a, b];
        // Forward from b, then backward from a.
        for forward in [true, false] {
            loop {
                let end = if forward {
                    *chain.last().unwrap_or(&b)
                } else {
                    *chain.first().unwrap_or(&a)
                };
                let next = by_key.get(&end).and_then(|list| {
                    list.iter()
                        .copied()
                        .find(|k| !used.get(*k).copied().unwrap_or(true))
                });
                let Some(k) = next else { break };
                if let Some(u) = used.get_mut(k) {
                    *u = true;
                }
                let Some(&(p, q)) = segments.get(k) else {
                    break;
                };
                let other = if p == end { q } else { p };
                if forward {
                    chain.push(other);
                } else {
                    chain.insert(0, other);
                }
            }
        }
        let closed = chain.len() > 2 && chain.first() == chain.last();
        if closed {
            chain.pop();
        }
        let mut xy: Vec<(f64, f64)> = chain
            .iter()
            .filter_map(|k| points.get(k).copied())
            .collect();
        unwrap_path(g, &mut xy);
        if xy.len() >= 2 {
            out.push(Path { xy, closed, level });
        }
    }
    out
}

/// Makes a path continuous across the face's seams, in grid coordinates.
fn unwrap_path(g: &Grid, xy: &mut [(f64, f64)]) {
    let (nu, nv) = (g.nu as f64, g.nv as f64);
    for k in 1..xy.len() {
        let Some(prev) = xy.get(k - 1).copied() else {
            continue;
        };
        if let Some(p) = xy.get_mut(k) {
            if g.wrap_u {
                p.0 += ((prev.0 - p.0) / nu).round() * nu;
            }
            if g.wrap_v {
                p.1 += ((prev.1 - p.1) / nv).round() * nv;
            }
        }
    }
}

/// Orients `path` with the inside of the face (the larger distance) on its
/// left in (u, v).
fn orient(g: &Grid, path: &mut Path) {
    let n = path.xy.len();
    if n < 3 {
        return;
    }
    let m = n / 2;
    let (Some(a), Some(b), Some(c)) = (path.xy.get(m - 1), path.xy.get(m), path.xy.get(m + 1))
    else {
        return;
    };
    let (tx, ty) = (c.0 - a.0, c.1 - a.1);
    let len = tx.hypot(ty);
    if len <= 1e-12 {
        return;
    }
    let (lx, ly) = (-ty / len * 0.75, tx / len * 0.75);
    let left = g.distance_at(b.0 + lx, b.1 + ly);
    let right = g.distance_at(b.0 - lx, b.1 - ly);
    if let (Some(l), Some(r)) = (left, right)
        && l < r
    {
        path.xy.reverse();
    }
}

// ---------------------------------------------------------------------------
// The face cut along its outline.

/// The face cut along its outline: the outline itself, then its offsets
/// inwards at the stepover, in that order, each run directed and, with
/// Stay down between passes, joined to the next where the step is clear.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) fn cut_outline(
    model: &BrepModel,
    brep: &BrepFace,
    id: FaceId,
    face: &ExactFace,
    closed: &Closure,
    tooling: &Tooling<'_>,
    profile: &RevolvedProfile,
    context: &JobContext<'_>,
) -> Result<FaceCut> {
    let settings = tooling.settings;
    let step = match settings.scallop {
        // The spacing that leaves the scallop height on a flat, for the
        // ball or bull-nose's cutting arc.
        Some((height, rho)) if height < rho => settings
            .stepover
            .min(2.0 * (2.0 * rho * height - height * height).sqrt()),
        _ => settings.stepover,
    };
    let g = grid(model, brep, face, closed, step, context)?;
    let wrap = closed.period;
    let to_uv = |xy: &[(f64, f64)]| -> Vec<Uv> { xy.iter().map(|(x, y)| g.uv(*x, *y)).collect() };
    // The outline itself first: the face's loops of edges, seams left out.
    let mut paths: Vec<Path> = Vec::new();
    for l in loops(model, brep, &closed.seams)? {
        let points = concat(&l);
        let mut xy: Vec<(f64, f64)> = Vec::with_capacity(points.len());
        let mut last: Option<Uv> = None;
        for q in &points {
            let found = match last {
                Some(prev) => face.closest_from(*q, prev, f64::INFINITY),
                None => face.closest(*q),
            };
            if let Some(c) = found {
                last = Some(c.uv);
                xy.push(g.coords(c.uv));
            }
        }
        unwrap_path(&g, &mut xy);
        let closes = matches!((points.first(), points.last()), (Some(a), Some(b)) if a.distance(*b) < super::JOIN);
        if closes && xy.len() > 2 {
            xy.pop();
        }
        if xy.len() >= 2 {
            paths.push(Path {
                xy,
                closed: closes,
                level: 0,
            });
        }
    }
    let deepest = g
        .distance
        .iter()
        .copied()
        .filter(|d| d.is_finite())
        .fold(0.0, f64::max);
    // Each offset is taken half a grid spacing short of its whole stepover:
    // between two grid points the distance can peak (a ridge, where offsets
    // from both sides meet) up to that much above what the points hold, so
    // the last offset is not lost there and no pass is more than a stepover
    // from the last.
    let level_at = |level: usize| level as f64 * step - 0.5 * g.h;
    let mut level = 1;
    while level_at(level) < deepest {
        context.check_cancelled()?;
        paths.extend(level_lines(&g, level_at(level), level));
        if paths.len() > MAX_PASSES {
            return Err(PathError::refused(
                crate::topic::FACE_FLOW,
                format!(
                    "face {} would need more than {MAX_PASSES} passes along its outline: raise \
                     the stepover",
                    id.0
                ),
            )
            .with_setting(crate::params::names::STEPOVER));
        }
        level += 1;
    }
    for p in &mut paths {
        orient(&g, p);
    }
    // The order: level by level from the outline in, each next path the one
    // nearest where the last ended, a closed one started at that point.
    let point_of = |xy: (f64, f64)| -> Option<Point3> {
        let uv = g.uv(xy.0, xy.1);
        let line = UvLine::new(face, vec![uv], wrap);
        line.point(uv)
    };
    let mut ordered: Vec<Path> = Vec::with_capacity(paths.len());
    let mut left: Vec<Path> = paths;
    left.sort_by(|a, b| {
        a.level
            .cmp(&b.level)
            .then_with(|| b.xy.len().cmp(&a.xy.len()))
    });
    let mut here: Option<Point3> = None;
    while !left.is_empty() {
        let lowest = left.iter().map(|p| p.level).min().unwrap_or(0);
        let mut best: Option<(usize, usize, f64)> = None;
        for (k, p) in left.iter().enumerate().filter(|(_, p)| p.level == lowest) {
            let candidates: Vec<usize> = if p.closed {
                (0..p.xy.len()).collect()
            } else {
                vec![0]
            };
            for c in candidates {
                let d = match (here, p.xy.get(c).copied().and_then(point_of)) {
                    (Some(h), Some(q)) => h.distance(q),
                    _ => 0.0,
                };
                if best.is_none_or(|b| d < b.2 - 1e-12) {
                    best = Some((k, c, d));
                }
                if here.is_none() {
                    break;
                }
            }
            if here.is_none() {
                break;
            }
        }
        let Some((k, c, _)) = best else { break };
        let mut p = left.remove(k);
        if p.closed && c > 0 {
            p.xy.rotate_left(c);
            unwrap_path(&g, &mut p.xy);
        }
        if p.closed
            && let Some(first) = p.xy.first().copied()
        {
            // A closed path ends where it starts.
            let n = (g.nu as f64, g.nv as f64);
            let last = p.xy.last().copied().unwrap_or(first);
            let mut end = first;
            if g.wrap_u {
                end.0 += ((last.0 - end.0) / n.0).round() * n.0;
            }
            if g.wrap_v {
                end.1 += ((last.1 - end.1) / n.1).round() * n.1;
            }
            p.xy.push(end);
        }
        here = p.xy.last().copied().and_then(point_of);
        ordered.push(p);
    }
    // The direction: the inside of the face on the left in (u, v) to start
    // with; Climb and Conventional keep the material on the side they ask
    // for, and Meander alternates.
    let inside_right = ordered
        .first()
        .and_then(|p| inside_on_right(&g, face, p, wrap));
    let flip_all = match (settings.direction, inside_right) {
        (Direction::Climb, Some(right)) => !right,
        (Direction::Conventional, Some(right)) => right,
        _ => false,
    };
    let diameter = 2.0 * profile.cutting_radius();
    let mut passes: Vec<Vec<Vec<CutPoint>>> = Vec::with_capacity(ordered.len());
    let mut lines: Vec<UvLine<'_>> = Vec::with_capacity(ordered.len());
    let mut spans_of: Vec<Vec<(f64, f64)>> = Vec::with_capacity(ordered.len());
    let mut stretches: Vec<Stretch> = Vec::new();
    for (k, p) in ordered.iter().enumerate() {
        context.check_cancelled()?;
        let mut uv = to_uv(&p.xy);
        let alternate = settings.direction == Direction::Meander && k % 2 == 1;
        if flip_all != alternate {
            uv.reverse();
        }
        let line = UvLine::new(face, uv, wrap);
        let samples = pass_samples(&line, tooling);
        let (runs, spans) = split_runs(
            samples,
            id,
            k,
            step,
            &|_| step.max(diameter),
            diameter,
            &mut stretches,
        );
        passes.push(runs);
        spans_of.push(spans);
        lines.push(line);
    }
    // The runs in order, joined across from one path to the next.
    let mut joined: Vec<Vec<CutPoint>> = Vec::new();
    let mut open: Option<Uv> = None;
    for ((runs, spans), line) in passes.iter().zip(&spans_of).zip(&lines) {
        for (i, (points, (s0, s1))) in runs.iter().zip(spans).enumerate() {
            let start = line.uv_at(*s0);
            let steps = match (
                settings.stay_down,
                i,
                open,
                start,
                joined.last().and_then(|r| r.last()),
                points.first(),
            ) {
                (true, 0, Some(a), Some(b), Some(pa), Some(pb)) => {
                    let across = UvStep { face, a, b, wrap };
                    let end_a = nearest_turn(pb.a, pa.a);
                    step_on(
                        &across,
                        tooling,
                        0.0,
                        pa,
                        pb,
                        2.0 * settings.stepover,
                        end_a,
                    )
                    .map(|steps| (steps, end_a))
                }
                _ => None,
            };
            match (steps, joined.last_mut()) {
                (Some((steps, end_a)), Some(run)) => append(run, steps, points, end_a),
                _ => joined.push(points.clone()),
            }
            open = None;
            if i + 1 == runs.len() {
                open = line.uv_at(*s1);
            }
        }
        if runs.is_empty() {
            open = None;
        }
    }
    let outline_edges: Vec<EdgeId> = brep
        .edges
        .iter()
        .copied()
        .filter(|e| !closed.seams.contains(e))
        .collect();
    let length: f64 = outline_edges
        .iter()
        .filter_map(|e| edge_points(model, *e))
        .map(|(points, _, _)| Curve::new(points).length())
        .sum();
    Ok(FaceCut {
        id,
        passes,
        joined: Some(joined),
        stretches,
        guide_edges: outline_edges,
        guide_length: length,
        point_opposite: false,
        dropped_short: 0,
        spacing: step,
        outline: true,
        ring_words: String::new(),
    })
}

/// Whether, along `path` as oriented (the inside of the face on its left in
/// (u, v)), the inside is on the tool's right looking along the feed from
/// the spindle; `None` where it cannot be told.
fn inside_on_right(
    g: &Grid,
    face: &ExactFace,
    path: &Path,
    wrap: [Option<(f64, f64)>; 2],
) -> Option<bool> {
    let m = path.xy.len() / 2;
    let a = *path.xy.get(m.checked_sub(1)?)?;
    let b = *path.xy.get(m)?;
    let c = *path.xy.get(m + 1)?;
    let (tx, ty) = (c.0 - a.0, c.1 - a.1);
    let len = tx.hypot(ty);
    if len <= 1e-12 {
        return None;
    }
    let left = (b.0 - ty / len * 0.75, b.1 + tx / len * 0.75);
    let line = UvLine::new(face, Vec::new(), wrap);
    let p = |xy: (f64, f64)| face.point_normal(line.wrapped(g.uv(xy.0, xy.1)));
    let (pa, pb, pc, pl) = (p(a)?, p(b)?, p(c)?, p(left)?);
    let feed: Vec3 = pc.point - pa.point;
    // The tool axis points out of the material, along the face's normal.
    let right = feed.cross(pb.normal.get());
    Some(right.dot(pl.point - pb.point) > 0.0)
}
