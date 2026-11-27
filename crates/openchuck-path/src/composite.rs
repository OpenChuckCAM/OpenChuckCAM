// SPDX-License-Identifier: AGPL-3.0-or-later
//! The model's surface joined to analytic solids (requirements SET-09 and
//! OP-21; work breakdown slice 21b; decision record 0035 items 1 and 2).
//!
//! The held region of a setup and each retaining tab are part of the target:
//! a strategy that treats the target as the finished part then leaves them
//! alone, with stock to leave, gouge protection and the simulation's target
//! all unchanged. [`CompositeSurface`] is the [`SurfaceSource`] that does
//! this. It holds the model's own source and a list of [`Solid`]s and answers
//! every query as the union of its parts: a tool drop returns the highest
//! contact, a distance or projection the nearest point, a ray the first hit.
//! The port and [`StrategyInput`](crate::StrategyInput) do not change.
//!
//! Ties go to the model, then to the solids in the order given, so the answer
//! does not depend on thread count (NFR-06).

use std::sync::Arc;

use openchuck_types::geometry::{Aabb, Point3, UnitVec3};
use openchuck_types::mesh::TriangleMesh;
use openchuck_types::tool::profile::RevolvedProfile;

use crate::cutter::{Contact, ToolLine};
use crate::gap::{GapQuery, ToolGap};
use crate::holes::Hole;
use crate::solid::Solid;
use crate::surface::{FaceRef, RayHit, SurfacePoint, SurfaceSource};

/// Rounding allowed for when a solid is skipped in a drop, mm: far above the
/// rounding of a solid's drop (about 1e-13 mm on a part a metre across).
const SOLID_BOUND_SLACK: f64 = 1e-7;

/// Segments around a solid in the composite's tessellation.
const TESSELLATION_SEGMENTS: u32 = 96;

/// A model surface with analytic solids joined to it, as a
/// [`SurfaceSource`].
#[derive(Debug)]
pub struct CompositeSurface {
    model: Arc<dyn SurfaceSource>,
    solids: Vec<Solid>,
    mesh: TriangleMesh,
    bounds: Aabb,
    /// The solids' own tessellation, for the tool gap (decision record
    /// SIMUL-EDGE-1, Q2), and the largest distance from it to a solid.
    solid_mesh: TriangleMesh,
    solid_accuracy: f64,
}

impl CompositeSurface {
    /// Joins `solids` to `model`. With no solids it answers as the model does.
    pub fn new(model: Arc<dyn SurfaceSource>, solids: Vec<Solid>) -> Self {
        let mut mesh = model.tessellation().clone();
        let mut bounds = model.bounds();
        let mut solid_mesh = TriangleMesh::default();
        let mut solid_accuracy = 0.0_f64;
        for solid in &solids {
            let own = solid.mesh(TESSELLATION_SEGMENTS);
            let base = u32::try_from(solid_mesh.vertices.len()).unwrap_or(u32::MAX);
            solid_mesh.vertices.extend_from_slice(&own.vertices);
            solid_mesh.triangles.extend(
                own.triangles
                    .iter()
                    .map(|t| t.map(|i| i.saturating_add(base))),
            );
            // A round solid's mesh is inscribed: its chords lie inside by up
            // to the sagitta of one segment. A box's mesh is exact.
            if solid.as_cuboid().is_none() {
                let r = solid.base_radius().max(solid.top_radius());
                let half = std::f64::consts::PI / f64::from(TESSELLATION_SEGMENTS);
                solid_accuracy = solid_accuracy.max(r * (1.0 - half.cos()));
            }
            let offset = u32::try_from(mesh.vertices.len()).unwrap_or(u32::MAX);
            let part = solid.mesh(TESSELLATION_SEGMENTS);
            mesh.vertices.extend(part.vertices);
            mesh.triangles.extend(
                part.triangles
                    .iter()
                    .map(|t| t.map(|i| i.saturating_add(offset))),
            );
            let b = solid.bounds();
            bounds = Aabb {
                min: Point3::new(
                    bounds.min.x.min(b.min.x),
                    bounds.min.y.min(b.min.y),
                    bounds.min.z.min(b.min.z),
                ),
                max: Point3::new(
                    bounds.max.x.max(b.max.x),
                    bounds.max.y.max(b.max.y),
                    bounds.max.z.max(b.max.z),
                ),
            };
        }
        Self {
            model,
            solids,
            mesh,
            bounds,
            solid_mesh,
            solid_accuracy,
        }
    }

    /// The model's own surface.
    pub fn model(&self) -> &Arc<dyn SurfaceSource> {
        &self.model
    }

    /// The solids joined to it, in the order given.
    pub fn solids(&self) -> &[Solid] {
        &self.solids
    }
}

impl SurfaceSource for CompositeSurface {
    fn bounds(&self) -> Aabb {
        self.bounds
    }

    fn project(&self, point: Point3) -> Option<SurfacePoint> {
        let mut best = self.model.project(point);
        for solid in &self.solids {
            if let Some(c) = solid.project(point)
                && best.is_none_or(|b| c.point.distance(point) < b.point.distance(point))
            {
                best = Some(c);
            }
        }
        best
    }

    fn cast_ray(&self, origin: Point3, direction: UnitVec3, max_distance: f64) -> Option<RayHit> {
        let mut best = self.model.cast_ray(origin, direction, max_distance);
        for solid in &self.solids {
            if let Some(h) = solid.cast_ray(origin, direction, max_distance)
                && best.is_none_or(|b| h.distance < b.distance)
            {
                best = Some(h);
            }
        }
        best
    }

    fn tessellation(&self) -> &TriangleMesh {
        &self.mesh
    }

    fn holes(&self) -> &[Hole] {
        self.model.holes()
    }

    fn exact_faces(&self) -> Vec<FaceRef<'_>> {
        self.model.exact_faces()
    }

    fn drop_accuracy(&self) -> f64 {
        self.model.drop_accuracy()
    }

    fn tool_gap(
        &self,
        profile: &RevolvedProfile,
        line: &ToolLine,
        query: &GapQuery,
    ) -> Option<ToolGap> {
        let model = self.model.tool_gap(profile, line, query);
        let chord = self.solid_accuracy;
        let solids = crate::gap::on_mesh(
            &self.solid_mesh,
            profile,
            line,
            query,
            &|_| crate::gap::Band::even(chord),
            chord,
        );
        match (model, solids) {
            (Some(m), Some(s)) => Some(m.min(s)),
            (m, s) => m.or(s),
        }
    }

    fn tool_gap_accuracy(&self) -> f64 {
        self.model.tool_gap_accuracy().max(self.solid_accuracy)
    }

    fn drop_on_part(&self, profile: &RevolvedProfile, line: &ToolLine) -> Option<Contact> {
        self.model.drop_tool(profile, line)
    }

    fn drop_tool(&self, profile: &RevolvedProfile, line: &ToolLine) -> Option<Contact> {
        let mut best = self.model.drop_tool(profile, line);
        for solid in &self.solids {
            // A solid that lies wholly below the contact found, less a tie,
            // cannot replace it, so it is not dropped onto: the answer is the
            // same, bit for bit, and a solid far below the part costs nothing
            // (issue #382).
            if best.is_some_and(|b| {
                solid.highest_along(line) + SOLID_BOUND_SLACK <= b.offset + crate::cutter::TIP_TIE
            }) {
                continue;
            }
            // A solid higher by less than a tie keeps the model's contact, so
            // rounding cannot choose (decision record 0113).
            if let Some(c) = solid.drop_tool(profile, line)
                && best.is_none_or(|b| c.offset > b.offset + crate::cutter::TIP_TIE)
            {
                best = Some(c);
            }
        }
        best
    }
}
