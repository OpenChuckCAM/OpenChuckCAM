// SPDX-License-Identifier: AGPL-3.0-or-later
//! One nominal coverage/target description for viewport and Setup (0252).
//! Geometry describes the supplied placed mesh; source accuracy stays explicit.
//! It is not a sweep, clearance or completion certificate.

use openchuck_types::geometry::Point3;
use openchuck_types::units::Degrees;

use super::extents::FacingExtent;
use super::geometry::{CoverageRectangle, EndTarget, FacingEnd, FacingPlane};
use super::held::{self, Unfaced};
use super::parameter;
use super::plan::FacingSetup;
use super::selection::{ResolvedSide, SideSelection};
use crate::error::{PathError, Result};
use crate::params::{Parameters, names};
use crate::rotary::RotaryMapping;

/// An indexed plane with its requested coverage, before cutter overlap.
#[derive(Debug, Clone, PartialEq)]
pub struct FaceDescription {
    /// Nominal target plane (legacy Flat can retain protected protrusions).
    pub target: ResolvedSide,
    /// Requested rectangle, already including the user-entered margin.
    pub coverage: CoverageRectangle,
    /// What of `coverage` is left unfaced beside a held region, along X;
    /// empty when the tool reaches all of it (decision FACING-GRIP-A).
    pub unfaced: Vec<Unfaced>,
}
impl FaceDescription {
    /// The rectangle Facing faces and its completion check proves:
    /// `coverage` less what is left unfaced beside a held region.
    pub fn faced(&self) -> CoverageRectangle {
        let [u0, _] = self.coverage.min();
        let [u1, _] = self.coverage.max();
        self.coverage.with_u(faced_span((u0, u1), &self.unfaced))
    }

    /// Four corners in winding order for both host diagrams. No guessed
    /// silhouette, projected AABB or tool-centre offset enters these points.
    pub fn corners(&self) -> Result<[Point3; 4]> {
        let [u0, v0] = self.coverage.min();
        let [u1, v1] = self.coverage.max();
        let point = |u, v| self.target.plane.point(u, v, self.target.offset);
        Ok([
            point(u0, v0)?,
            point(u1, v0)?,
            point(u1, v1)?,
            point(u0, v1)?,
        ])
    }
}

/// Nominal finished setting and requested coverage. Protected geometry and
/// intentional leftovers remain distinct from this surface in every mode.
#[derive(Debug, Clone, PartialEq)]
pub enum FacingGeometry {
    /// Legacy Flat nominal settings; the existing composite target may rise
    /// above these planes and must remain visible in completion reporting.
    Flat {
        /// In legacy angle-list order.
        faces: Vec<FaceDescription>,
    },
    /// Four selected box faces followed by ordered arbitrary A faces.
    Box {
        /// In effective cut order.
        faces: Vec<FaceDescription>,
    },
    /// End selected from actual workholding, never an inferred tip orientation.
    End {
        /// Which unheld end.
        end: FacingEnd,
        /// Model-derived X and length, excluding tabs and held stock.
        target: EndTarget,
        /// YZ rectangle at the target X.
        coverage: CoverageRectangle,
    },
    /// Legacy round target; coverage has full circumference in this X span.
    Round {
        /// Existing finished-diameter setting; margin never changes it.
        diameter: f64,
        /// Axial coverage including margin at both ends.
        x_extent: (f64, f64),
        /// What of `x_extent` is left unfaced beside a held region.
        unfaced: Vec<Unfaced>,
    },
}

/// `requested` less the unfaced spans at its ends.
pub(crate) fn faced_span(requested: (f64, f64), unfaced: &[Unfaced]) -> (f64, f64) {
    unfaced
        .iter()
        .fold(requested, |(lo, hi), u| match u.beside {
            FacingEnd::Start => (lo.max(u.x.1), hi),
            FacingEnd::Far => (lo, hi.min(u.x.0)),
        })
}

impl FacingGeometry {
    /// Everything left unfaced beside a held region, face by face.
    pub fn unfaced(&self) -> Vec<Unfaced> {
        match self {
            Self::Flat { faces } | Self::Box { faces } => faces
                .iter()
                .flat_map(|f| f.unfaced.iter().copied())
                .collect(),
            Self::Round { unfaced, .. } => unfaced.clone(),
            Self::End { .. } => Vec::new(),
        }
    }
}

/// Coverage choices and source-bound status accompanying nominal geometry.
#[derive(Debug, Clone, PartialEq)]
pub struct FacingDescription {
    /// Requested geometry; cutter sweep is a separate generated result.
    pub geometry: FacingGeometry,
    /// The selected meaning of the bounding geometry.
    pub extent: FacingExtent,
    /// User-entered margin, separate from extension and source error.
    pub margin: f64,
    /// Bound from the placed mesh to the original source; None is unknown.
    /// Hosts must not present an unknown STEP projection as certified exact.
    pub source_projection_error: Option<f64>,
}

/// [`describe`], for a generated path: where legacy Flat passes along X
/// are lifted off the face beside a held region, its unfaced spans are read
/// from the cut in `group`, as its completion check reads them (decision
/// FACING-GRIP-A). The other modes are described as [`describe`] does.
pub fn describe_path(
    setup: &FacingSetup<'_>,
    parameters: &Parameters,
    group: &openchuck_types::ir::ToolpathGroup,
    context: &openchuck_types::job::JobContext<'_>,
) -> Result<FacingDescription> {
    super::validation::describe_path(setup, parameters, group, context)
}

/// Describe the stored settings using whole placed geometry. This neither
/// changes targets nor grants permission to cut tabs in Part plus tabs.
pub fn describe(setup: &FacingSetup<'_>, parameters: &Parameters) -> Result<FacingDescription> {
    setup.part.validate()?;
    setup.stock.validate()?;
    setup.machine.validate()?;
    if setup
        .projection_error
        .is_some_and(|e| !e.is_finite() || e < 0.)
    {
        return Err(PathError::parameter(
            "projection error",
            "must be finite and nonnegative, or explicitly unknown",
        ));
    }
    let extent = FacingExtent::from_name(parameters.text(parameter::EXTENT)?)?;
    let margin = parameters.number(parameter::MARGIN)?;
    let rotary = RotaryMapping::of_profile(setup.machine);
    let coverage = |plane| extent.planar(plane, setup.stock, setup.part, setup.tabs, margin);
    // How far past the tool's centre each mode's completion check counts the
    // face finished at the end of its cut along X (decision FACING-GRIP-A).
    let radius = 0.5 * setup.tool.diameter.0;
    let counted = match parameters.text(parameter::SHAPE)? {
        parameter::BOX => radius,
        parameter::ROUND => radius - super::validation::ROUND_RESERVE,
        _ => 0.,
    };
    let face = |target: ResolvedSide| -> Result<FaceDescription> {
        let coverage = coverage(target.plane)?;
        let unfaced = held::crop(setup, (coverage.min()[0], coverage.max()[0]), counted)?.unfaced;
        Ok(FaceDescription {
            coverage,
            target,
            unfaced,
        })
    };
    let geometry = match parameters.text(parameter::SHAPE)? {
        parameter::ROUND => {
            let x_extent = extent.axial(setup.stock, setup.part, setup.tabs, margin)?;
            FacingGeometry::Round {
                diameter: parameters.number(parameter::SIZE)?,
                x_extent,
                unfaced: held::crop(setup, x_extent, counted)?.unfaced,
            }
        }
        parameter::END => {
            let end = FacingEnd::opposite_held(setup.held[0].is_some(), setup.held[1].is_some())?;
            let part_x = FacingExtent::Part.axial(setup.stock, setup.part, &[], 0.)?;
            let target = end.target(part_x, parameters.number(names::STOCK_TO_LEAVE)?)?;
            FacingGeometry::End {
                end,
                target,
                coverage: coverage(FacingPlane::end(end))?,
            }
        }
        parameter::FLAT => {
            let offset = parameters.number(parameter::SIZE)?;
            let faces = parameters
                .angles(parameter::ANGLES)?
                .into_iter()
                .map(|angle| {
                    let angle = Degrees(angle);
                    face(ResolvedSide {
                        angle,
                        offset,
                        plane: FacingPlane::side(rotary, angle)?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            FacingGeometry::Flat { faces }
        }
        parameter::BOX => {
            let selection = SideSelection::read(parameters)?;
            let vertices = setup
                .part
                .triangles
                .iter()
                .flatten()
                .filter_map(|i| {
                    usize::try_from(*i)
                        .ok()
                        .and_then(|i| setup.part.vertices.get(i))
                        .copied()
                })
                .collect::<Vec<_>>();
            let faces = selection
                .resolve(rotary, &vertices, parameters.number(names::STOCK_TO_LEAVE)?)?
                .into_iter()
                .map(face)
                .collect::<Result<Vec<_>>>()?;
            FacingGeometry::Box { faces }
        }
        _ => {
            return Err(PathError::parameter(
                parameter::SHAPE,
                "unknown facing mode",
            ));
        }
    };
    Ok(FacingDescription {
        geometry,
        extent,
        margin,
        source_projection_error: setup.projection_error,
    })
}
