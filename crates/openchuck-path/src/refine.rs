// SPDX-License-Identifier: AGPL-3.0-or-later
//! A STEP part's tessellation refined until every triangle lies within a
//! target of its exact face (OI-9974, as the project owner answered it on
//! 26 Nov 2026: refine at load, then re-check against the exact faces only
//! where still needed; decision record SIMUL-EDGE-1, Q2).
//!
//! The importer's tessellation can lie further from the faces than its
//! stated deflection (decision 0003): on the public blisk, up to 0.043 mm
//! from its cones at a stated 0.01 mm. The tool gap takes each triangle's
//! error off its bounds, so a triangle whose error is near the tolerance
//! leaves nothing to prove a cut with. Here every triangle whose measured
//! error is over the target is split, its new points on the exact face, in
//! rounds until each is within the target.
//!
//! **No cracks.** A crack between triangles would hide part of the face from
//! the gap. Triangles are split by longest-edge bisection (Rivara's LEPP): a
//! triangle and its neighbour across its longest edge are split together at
//! that edge's midpoint, the neighbour first brought to share it as its own
//! longest edge, so the mesh stays conforming. Vertices the faces' own
//! tessellations repeat at their shared boundaries are merged first, so a
//! boundary edge is split once for both faces, at a point on both faces'
//! surfaces (found by projecting onto each in turn).
//!
//! **Determinism (NFR-06).** Triangles are split one at a time in index
//! order; only the measuring of new triangles runs in parallel, each alone
//! and in order ([`crate::parallel::map_in_order`]).

use std::collections::BTreeMap;

use openchuck_types::brep::FaceId;
use openchuck_types::geometry::Point3;
use openchuck_types::mesh::TriangleMesh;

use crate::brep::ExactFace;
use crate::gap::Band;

/// The most rounds of splitting: each round at least halves the longest
/// side of every triangle it splits, and an error shrinks with its side's
/// square, so a triangle 2^12 times over its target is within it well
/// before this.
const MAX_ROUNDS: u32 = 24;

/// A side shorter than this is not split again, mm: far below any error
/// that matters, it keeps a face the measure cannot satisfy finite.
const MIN_SIDE: f64 = 1e-5;

/// The most triangles refinement makes: a guard on memory (NFR-07). A
/// triangle left over its target keeps its measured error, which the gap
/// takes off its bounds, so stopping early costs proof, never soundness.
const MAX_TRIANGLES: usize = 4_000_000;

/// The most rounds of projecting a boundary edge's midpoint onto its faces
/// in turn.
const BOUNDARY_STEPS: u32 = 50;

/// The refined tessellation: its triangles, the face of each, how far each
/// one's face lies outside and inside it, and the largest of those.
#[derive(Debug, Clone)]
pub(crate) struct Refined {
    pub(crate) mesh: TriangleMesh,
    pub(crate) faces: Vec<FaceId>,
    pub(crate) bands: Vec<Band>,
    pub(crate) largest: f64,
}

/// The tessellation `mesh` (each triangle on the face `face_of[i]`, its face
/// lying within `bands[i]` of it) refined until every triangle's error (the
/// wider side of its band) is at most `target`, as far as [`MAX_ROUNDS`],
/// [`MAX_TRIANGLES`] and [`MIN_SIDE`] allow.
/// `fallback` is the error, both ways, of a triangle with no face.
pub(crate) fn refine(
    faces: &[ExactFace],
    mesh: &TriangleMesh,
    face_of: &[FaceId],
    bands: &[Band],
    target: f64,
    fallback: f64,
) -> Refined {
    refine_only(faces, mesh, face_of, bands, target, fallback, &|_| true)
}

/// As [`refine`], refining only the triangles `wanted` picks (by index in
/// `mesh`) and the triangles split from them; others are split only where
/// a wanted one's split reaches them, to keep the mesh conforming.
pub(crate) fn refine_only(
    faces: &[ExactFace],
    mesh: &TriangleMesh,
    face_of: &[FaceId],
    bands: &[Band],
    target: f64,
    fallback: f64,
    wanted: &dyn Fn(usize) -> bool,
) -> Refined {
    let index: BTreeMap<u32, usize> = faces
        .iter()
        .enumerate()
        .map(|(i, f)| (f.number(), i))
        .collect();
    let mut work = Work::new(mesh, face_of, bands, &index, wanted);
    for _ in 0..MAX_ROUNDS {
        let todo: Vec<usize> = (0..work.tri.len())
            .filter(|&t| work.wanted(t) && work.error(t) > target)
            .collect();
        if todo.is_empty() {
            break;
        }
        for t in todo {
            if work.tri.len() >= MAX_TRIANGLES {
                break;
            }
            // An earlier split this round may have made it a new triangle,
            // measured next round.
            if work.error(t) > target {
                work.refine(t, faces);
            }
        }
        work.measure(faces, fallback);
    }
    work.finish()
}

/// The mesh being refined.
struct Work<'a> {
    pos: Vec<Point3>,
    tri: Vec<[u32; 3]>,
    /// The index into the faces of each triangle's face, if it has one.
    face: Vec<Option<usize>>,
    /// Each triangle's error, the wider side of its band; NaN until a new
    /// triangle is measured.
    err: Vec<f64>,
    /// How far each triangle's face lies outside and inside it.
    band: Vec<Band>,
    /// Whether each triangle is to be refined.
    want: Vec<bool>,
    /// The triangles on each side, by its two vertices in order.
    sides: BTreeMap<(u32, u32), Vec<usize>>,
    index: &'a BTreeMap<u32, usize>,
}

/// The sides of a triangle, each from a corner to the next.
fn sides_of(t: [u32; 3]) -> [(u32, u32); 3] {
    let [a, b, c] = t;
    [(a, b), (b, c), (c, a)]
}

impl<'a> Work<'a> {
    fn new(
        mesh: &TriangleMesh,
        face_of: &[FaceId],
        bands: &[Band],
        index: &'a BTreeMap<u32, usize>,
        wanted: &dyn Fn(usize) -> bool,
    ) -> Self {
        // Merge the vertices the faces repeat at their boundaries.
        let mut seen: BTreeMap<[u64; 3], u32> = BTreeMap::new();
        let mut pos = Vec::new();
        let mut remap = Vec::with_capacity(mesh.vertices.len());
        for p in &mesh.vertices {
            let key = [
                (p.x + 0.0).to_bits(),
                (p.y + 0.0).to_bits(),
                (p.z + 0.0).to_bits(),
            ];
            let id = *seen.entry(key).or_insert_with(|| {
                pos.push(*p);
                u32::try_from(pos.len() - 1).unwrap_or(u32::MAX)
            });
            remap.push(id);
        }
        let mut work = Self {
            pos,
            tri: Vec::new(),
            face: Vec::new(),
            err: Vec::new(),
            band: Vec::new(),
            want: Vec::new(),
            sides: BTreeMap::new(),
            index,
        };
        for (i, t) in mesh.triangles.iter().enumerate() {
            let at = |k: usize| t.get(k).and_then(|v| remap.get(*v as usize).copied());
            let (Some(a), Some(b), Some(c)) = (at(0), at(1), at(2)) else {
                continue;
            };
            if a == b || b == c || c == a {
                continue;
            }
            let face = face_of.get(i).and_then(|f| index.get(&f.0)).copied();
            let band = bands.get(i).copied();
            let id = work.add([a, b, c], face, band);
            if let Some(slot) = work.want.get_mut(id) {
                *slot = wanted(i);
            }
        }
        work
    }

    fn key(a: u32, b: u32) -> (u32, u32) {
        (a.min(b), a.max(b))
    }

    fn point(&self, v: u32) -> Option<Point3> {
        self.pos.get(v as usize).copied()
    }

    fn wanted(&self, t: usize) -> bool {
        self.want.get(t).copied().unwrap_or(false)
    }

    fn error(&self, t: usize) -> f64 {
        self.err.get(t).copied().unwrap_or(f64::NAN)
    }

    /// Adds a triangle and its sides.
    fn add(&mut self, t: [u32; 3], face: Option<usize>, band: Option<Band>) -> usize {
        let id = self.tri.len();
        self.tri.push(t);
        self.face.push(face);
        self.err.push(band.map_or(f64::NAN, Band::most));
        self.band.push(band.unwrap_or_default());
        self.want.push(true);
        for (a, b) in sides_of(t) {
            self.sides.entry(Self::key(a, b)).or_default().push(id);
        }
        id
    }

    /// Replaces triangle `id` with `t` (same face, unmeasured).
    fn replace(&mut self, id: usize, t: [u32; 3]) {
        let Some(old) = self.tri.get(id).copied() else {
            return;
        };
        for (a, b) in sides_of(old) {
            if let Some(list) = self.sides.get_mut(&Self::key(a, b)) {
                list.retain(|&x| x != id);
            }
        }
        if let Some(slot) = self.tri.get_mut(id) {
            *slot = t;
        }
        if let Some(slot) = self.err.get_mut(id) {
            *slot = f64::NAN;
        }
        for (a, b) in sides_of(t) {
            self.sides.entry(Self::key(a, b)).or_default().push(id);
        }
    }

    /// The longest side of triangle `t`, as its two vertices in order, and
    /// its length (ties to the earlier side); `None` for no such triangle.
    fn longest(&self, t: usize) -> Option<((u32, u32), f64)> {
        let v = self.tri.get(t).copied()?;
        let mut best: Option<((u32, u32), f64)> = None;
        for (a, b) in sides_of(v) {
            let len = self.point(a)?.distance(self.point(b)?);
            if best.is_none_or(|(_, l)| len > l) {
                best = Some(((a, b), len));
            }
        }
        best
    }

    /// Splits `t0` at its longest side, first splitting the neighbours
    /// across that side until they share it as their own longest (LEPP).
    fn refine(&mut self, t0: usize, faces: &[ExactFace]) {
        let mut stack = vec![t0];
        let mut steps = 0_u32;
        while let Some(&t) = stack.last() {
            steps += 1;
            if steps > 10_000 {
                return;
            }
            let Some(((a, b), len)) = self.longest(t) else {
                stack.pop();
                continue;
            };
            if len < MIN_SIDE {
                stack.pop();
                continue;
            }
            let others: Vec<usize> = self
                .sides
                .get(&Self::key(a, b))
                .map(|l| l.iter().copied().filter(|&x| x != t).collect())
                .unwrap_or_default();
            if let Some(&n) = others.iter().find(|&&n| {
                self.longest(n)
                    .is_some_and(|((p, q), _)| Self::key(p, q) != Self::key(a, b))
            }) {
                stack.push(n);
                continue;
            }
            self.split(a, b, faces);
            stack.pop();
        }
    }

    /// Splits every triangle on the side `a b` at its midpoint, placed on
    /// the exact face of the lowest-numbered of their faces.
    fn split(&mut self, a: u32, b: u32, faces: &[ExactFace]) {
        let users: Vec<usize> = self
            .sides
            .get(&Self::key(a, b))
            .cloned()
            .unwrap_or_default();
        let (Some(pa), Some(pb)) = (self.point(a), self.point(b)) else {
            return;
        };
        let mid = Point3::new(
            0.5 * (pa.x + pb.x),
            0.5 * (pa.y + pb.y),
            0.5 * (pa.z + pb.z),
        );
        // On the face of each triangle using the side: one face, its closest
        // point; two or more (a boundary edge), the point on all of them by
        // projecting onto each in turn (alternating projections converge to
        // the faces' meeting curve where they cross; a midpoint on one face
        // only would stay on the chord, off the other).
        let mut on_faces: Vec<&ExactFace> = users
            .iter()
            .filter_map(|&t| self.face.get(t).copied().flatten())
            .filter_map(|k| faces.get(k))
            .collect();
        on_faces.sort_by_key(|f| f.number());
        on_faces.dedup_by_key(|f| f.number());
        let mut on = mid;
        let rounds = if on_faces.len() > 1 {
            BOUNDARY_STEPS
        } else {
            1
        };
        for _ in 0..rounds {
            let before = on;
            for f in &on_faces {
                if let Some(p) = f.on_surface(on)
                    && p.is_finite()
                    && p.distance(mid) <= pa.distance(pb)
                {
                    on = p;
                }
            }
            if on.distance(before) < 1e-12 {
                break;
            }
        }
        self.pos.push(on);
        let m = u32::try_from(self.pos.len() - 1).unwrap_or(u32::MAX);
        for t in users {
            let Some([v0, v1, v2]) = self.tri.get(t).copied() else {
                continue;
            };
            // The triangle as [p, q, r] with p q the side, in its winding.
            let turns = [(v0, v1, v2), (v1, v2, v0), (v2, v0, v1)];
            let Some(&(p, q, r)) = turns
                .iter()
                .find(|(p, q, _)| Self::key(*p, *q) == Self::key(a, b))
            else {
                continue;
            };
            let face = self.face.get(t).copied().flatten();
            let want = self.wanted(t);
            self.replace(t, [p, m, r]);
            let id = self.add([m, q, r], face, None);
            if let Some(slot) = self.want.get_mut(id) {
                *slot = want;
            }
        }
    }

    /// Measures every new triangle's error against its face, in parallel,
    /// each alone and in index order.
    fn measure(&mut self, faces: &[ExactFace], fallback: f64) {
        let new: Vec<usize> = (0..self.tri.len())
            .filter(|&t| self.error(t).is_nan())
            .collect();
        let errs = crate::parallel::map_in_order(&new, |&t| {
            let corners = self.tri.get(t).and_then(|v| {
                let [a, b, c] = *v;
                Some([self.point(a)?, self.point(b)?, self.point(c)?])
            });
            let even = Band::even(fallback);
            match (corners, self.face.get(t).copied().flatten()) {
                (Some(c), Some(k)) => faces.get(k).map_or(even, |f| f.triangle_band(&c)),
                _ => even,
            }
        });
        for (t, b) in new.into_iter().zip(errs) {
            if let Some(slot) = self.err.get_mut(t) {
                *slot = b.most();
            }
            if let Some(slot) = self.band.get_mut(t) {
                *slot = b;
            }
        }
    }

    fn finish(self) -> Refined {
        let numbers: BTreeMap<usize, u32> = self.index.iter().map(|(n, k)| (*k, *n)).collect();
        let faces = self
            .face
            .iter()
            .map(|f| FaceId(f.and_then(|k| numbers.get(&k).copied()).unwrap_or(u32::MAX)))
            .collect();
        let largest = self
            .err
            .iter()
            .copied()
            .filter(|e| e.is_finite())
            .fold(0.0, f64::max);
        Refined {
            mesh: TriangleMesh {
                vertices: self.pos,
                triangles: self.tri,
            },
            faces,
            bands: self.band,
            largest,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(
        model: &openchuck_types::brep::BrepModel,
    ) -> (Vec<ExactFace>, TriangleMesh, Vec<FaceId>, Vec<Band>) {
        let faces: Vec<ExactFace> = model
            .faces
            .iter()
            .map(|f| ExactFace::new(f).unwrap())
            .collect();
        let (mesh, face_of) = model.joined_tessellation();
        let bands = (0..mesh.triangles.len())
            .map(|i| {
                let f = faces.iter().find(|f| f.number() == face_of[i].0).unwrap();
                f.triangle_band(&mesh.triangle(i).unwrap())
            })
            .collect();
        (faces, mesh, face_of, bands)
    }

    /// The sides used by one triangle only, with their total length.
    fn open_sides(mesh: &TriangleMesh) -> (usize, f64) {
        let mut count: BTreeMap<[u64; 6], usize> = BTreeMap::new();
        let key = |a: Point3, b: Point3| {
            let (p, q) = if (a.x, a.y, a.z) <= (b.x, b.y, b.z) {
                (a, b)
            } else {
                (b, a)
            };
            [p.x, p.y, p.z, q.x, q.y, q.z].map(|v| (v + 0.0).to_bits())
        };
        let mut length = BTreeMap::new();
        for t in &mesh.triangles {
            for i in 0..3 {
                let (a, b) = (
                    mesh.vertices[t[i] as usize],
                    mesh.vertices[t[(i + 1) % 3] as usize],
                );
                *count.entry(key(a, b)).or_insert(0) += 1;
                length.insert(key(a, b), a.distance(b));
            }
        }
        let open: Vec<_> = count
            .iter()
            .filter(|(_, n)| **n == 1)
            .map(|(k, _)| *k)
            .collect();
        (open.len(), open.iter().map(|k| length[k]).sum())
    }

    #[test]
    fn req_op_12_refined_triangles_are_within_the_target_and_the_mesh_has_no_new_cracks() {
        // A shaft of cylinders, planes and a fillet (a torus), tessellated
        // coarsely: every refined triangle is within a quarter of a 0.01 mm
        // tolerance of its face, and the open boundary is the same length as
        // before (a crack would add open sides), so the refined mesh covers
        // the same surface.
        let model = openchuck_testkit::brep::shouldered_shaft(4);
        let (faces, mesh, face_of, errors) = setup(&model);
        let target = 0.0025;
        assert!(
            errors.iter().any(|e| e.most() > target),
            "the fixture needs refining"
        );
        let r = refine(&faces, &mesh, &face_of, &errors, target, 0.0);
        assert!(r.largest <= target, "{} over {target}", r.largest);
        assert!(r.mesh.triangles.len() > mesh.triangles.len());
        // The faces' own tessellations repeat their shared corners, so the
        // original's open sides are compared after merging (as refine does).
        let merged = refine(&faces, &mesh, &face_of, &errors, f64::INFINITY, 0.0);
        let (before, before_len) = open_sides(&merged.mesh);
        let (after, after_len) = open_sides(&r.mesh);
        assert!(after >= before);
        assert!(
            (after_len - before_len).abs() < 1e-6 * before_len.max(1.0),
            "{before_len} -> {after_len}"
        );
        // Each triangle keeps the winding of the one it came from: its
        // normal faces the same way as its face's normal at its centre.
        for (i, t) in r.mesh.triangles.iter().enumerate() {
            let c = t.map(|v| r.mesh.vertices[v as usize]);
            let n = (c[1] - c[0]).cross(c[2] - c[0]);
            let f = faces.iter().find(|f| f.number() == r.faces[i].0).unwrap();
            let centre = Point3::new(
                (c[0].x + c[1].x + c[2].x) / 3.0,
                (c[0].y + c[1].y + c[2].y) / 3.0,
                (c[0].z + c[1].z + c[2].z) / 3.0,
            );
            let near = f.closest_within(centre, f64::MAX).unwrap();
            assert!(
                n.dot(near.point.normal.get()) > 0.0,
                "triangle {i} turned over"
            );
        }
    }

    #[test]
    fn req_op_12_refining_chosen_triangles_leaves_the_rest_as_they_were() {
        // The recheck of a failed pose refines only the triangles that
        // decide it: the chosen one comes within the target, and only
        // triangles its splits reach to keep the mesh conforming are split.
        let model = openchuck_testkit::brep::shouldered_shaft(4);
        let (faces, mesh, face_of, bands) = setup(&model);
        let target = 0.0005;
        let chosen = (0..mesh.triangles.len())
            .find(|&i| bands[i].most() > 4.0 * target)
            .expect("a triangle to refine");
        let all = refine(&faces, &mesh, &face_of, &bands, target, 0.0);
        let some = refine_only(&faces, &mesh, &face_of, &bands, target, 0.0, &|i| {
            i == chosen
        });
        assert!(some.mesh.triangles.len() > mesh.triangles.len());
        assert!(
            some.mesh.triangles.len() * 4 < all.mesh.triangles.len(),
            "{} of {}",
            some.mesh.triangles.len(),
            all.mesh.triangles.len()
        );
        // Every triangle lying inside the chosen one is within the target.
        let t = mesh.triangle(chosen).unwrap();
        let n = (t[1] - t[0]).cross(t[2] - t[0]);
        let inside = |p: Point3| {
            [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])]
                .iter()
                .all(|&(a, b)| (b - a).cross(p - a).dot(n) >= -1e-9)
        };
        let mut seen = 0;
        for (i, tri) in some.mesh.triangles.iter().enumerate() {
            let c = tri.map(|v| some.mesh.vertices[v as usize]);
            let centre = Point3::new(
                (c[0].x + c[1].x + c[2].x) / 3.0,
                (c[0].y + c[1].y + c[2].y) / 3.0,
                (c[0].z + c[1].z + c[2].z) / 3.0,
            );
            let off = (centre - t[0]).dot(n) / n.length();
            if off.abs() < 4.0 * bands[chosen].most() && inside(centre + n * (-off / n.length())) {
                seen += 1;
                assert!(some.bands[i].most() <= target, "{:?}", some.bands[i]);
            }
        }
        assert!(seen > 1, "the chosen triangle was split");
    }

    #[test]
    fn req_gc_07_req_nfr_06_refinement_is_the_same_every_time() {
        let model = openchuck_testkit::brep::shouldered_shaft(4);
        let (faces, mesh, face_of, errors) = setup(&model);
        let a = refine(&faces, &mesh, &face_of, &errors, 0.0025, 0.0);
        let b = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap()
            .install(|| refine(&faces, &mesh, &face_of, &errors, 0.0025, 0.0));
        assert_eq!(a.mesh, b.mesh);
        assert_eq!(
            a.bands
                .iter()
                .map(|e| (e.above.to_bits(), e.below.to_bits()))
                .collect::<Vec<_>>(),
            b.bands
                .iter()
                .map(|e| (e.above.to_bits(), e.below.to_bits()))
                .collect::<Vec<_>>()
        );
    }
}
