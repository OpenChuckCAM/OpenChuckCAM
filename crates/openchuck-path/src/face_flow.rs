// SPDX-License-Identifier: AGPL-3.0-or-later
//! Face-flow finishing (OP-28; decision record 0525; lane FACEFLOW).
//!
//! On a STEP (B-rep) part the user selects faces, and each face is cut with
//! passes that run parallel to a **guide**, a chain of its boundary edges,
//! and step across the face from it to the opposite boundary: a blisk's
//! blade floor is cut along the shape of the floor, not the machine's axes.
//!
//! # The passes
//!
//! - **Loops and sides.** A face's outer boundary is its longest loop of
//!   edges, joined end to end at their vertices. The loop is cut into sides
//!   where it turns by more than [`CORNER_ANGLE`]; a loop with fewer than
//!   two such corners is cut at its two points furthest apart.
//! - **The guide** is the longest side, measured along the edges' exact
//!   curves, ties going to the side with the lowest edge ID; or the edges
//!   the user chose, which must be joined end to end on the face's boundary
//!   (decision 0525, owner's decisions D2 and D3).
//! - **The opposite boundary** is what is left of the loop without the guide
//!   and the two sides next to it; with only two sides left it is the point
//!   where they meet (a three-sided face), and with one it is that side.
//! - **A face that closes round on itself** (a full cylinder or cone; issue
//!   #791, decision record 0675) is bounded by a seam, an edge along which
//!   the face meets itself, listed once though the boundary runs along it
//!   twice. Its seams are not boundaries: its two ends are the guide (the
//!   longer, or the one holding the user's guide edges) and the opposite
//!   boundary, or a point where an end closes on one, and each pass is a
//!   ring round the face. A face with no end, a seam as the guide, and
//!   guide edges on no end are refused.
//! - **The blend.** Guide and opposite are both taken by arc-length fraction
//!   `s` and put on the face's (u, v); the pass at weight `w` is the face's
//!   point at `(1 - w) uv_guide(s) + w uv_opposite(s)` (owner's point 2), so
//!   every point of every pass lies on the exact face.
//! - **The spacing.** Weights are chosen so that, at every `s`, the next
//!   pass is no further than the stepover along the face and, with a
//!   scallop height and a ball or bull-nose, leaves no higher cusp; the
//!   gaps are then evened out. The last pass is the opposite boundary.
//!
//! # The tool
//!
//! The tool axis follows the face's normal in the plane across the rotary
//! axis, leaned by the lead angle towards the travel round the axis, and
//! smoothed along the pass within the tilt limit, as simultaneous finishing
//! does (OP-05, OP-06, OP-15). The tool is placed touching the pass point
//! and dropped along its axis onto the **whole part**, so faces beside the
//! selection (a blade wall beside a floor) lift it where they are in the
//! way: such a stretch is left out and reported as unreached, with where
//! and why (OP-12, NFR-07). Every straight move between two points is
//! checked at its middle and quarter points and split where it would leave
//! the face by more than the tolerance (decision records 0115 and 0133).
//!
//! # Direction and order
//!
//! Passes are cut climb, conventional or meander (decision 0525 D1, a
//! strategy-local choice until OP-25's shared one exists). Several faces are
//! cut one after another, each with its own guide, starting from the face
//! with the longest guide and then along the faces' shared edges (D2).

use crate::notice::{Level, Notice, topic};
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::TAU;
use std::sync::Arc;

use openchuck_types::brep::{BrepFace, BrepModel, EdgeId, FaceId, Surface};
use openchuck_types::geometry::{Point3, UnitVec3, Vec3};
use openchuck_types::job::JobContext;
use openchuck_types::tool::profile::RevolvedProfile;
use openchuck_types::units::Degrees;

mod outline;

use crate::assemble::{MAX_UNREACHED_POINTS, Unreached, UnreachedReason};
use crate::brep::{BrepSurface, ExactFace};
use crate::cutter::{self, ToolLine};
use crate::error::{PathError, Result};
use crate::params::{ParameterKind, ParameterSchema, ParameterSpec, Parameters, Unit, names};
use crate::region::RegionConstraint;
use crate::rotary::{RotaryMapping, nearest_turn};
use crate::scallop;
use crate::strategy::{CutPoint, Pass, Strategy, StrategyInput, StrategyOutput};
use crate::surface::{ParametricFace, SurfacePoint, SurfaceSource, Uv};

/// The registry name.
pub const NAME: &str = "face_flow";

/// Parameter names specific to face-flow finishing.
pub mod parameter {
    /// The scallop (cusp) height between passes, mm; zero for none.
    pub const SCALLOP: &str = "scallop";
    /// Lean of the tool axis from the normal towards the travel round the
    /// rotary axis, degrees.
    pub const LEAD_ANGLE: &str = "lead_angle";
    /// The most the tool axis may leave its target to smooth A, degrees.
    pub const MAX_TILT: &str = "max_tilt";
    /// The cutting direction: [`MEANDER`], [`CLIMB`] or [`CONVENTIONAL`].
    pub const DIRECTION: &str = "direction";
    /// Passes alternate direction, with no return move between them.
    pub const MEANDER: &str = "meander";
    /// Every pass climb cut (with `M3`, the uncut material on the tool's
    /// right looking along the feed from the spindle; decision 0213).
    pub const CLIMB: &str = "climb";
    /// Every pass conventional cut.
    pub const CONVENTIONAL: &str = "conventional";
    /// With Meander, the tool steps along the face from the end of one pass
    /// to the start of the next instead of lifting, where the step is clear.
    pub const STAY_DOWN: &str = "stay_down";
    /// How the passes are laid out on each face: [`AUTOMATIC`],
    /// [`ALONG_GUIDE`] or [`OUTLINE`].
    pub const PATTERN: &str = "pattern";
    /// Along the guide where those passes stay whole on the face, and along
    /// the face's outline where the face's own notches or holes would break
    /// them.
    pub const AUTOMATIC: &str = "automatic";
    /// Passes parallel to the guide, blended to the opposite side (decision
    /// 0525 item 2.2).
    pub const ALONG_GUIDE: &str = "along_guide";
    /// Passes that follow the face's outline, each the stepover inside the
    /// last, measured on the face.
    pub const OUTLINE: &str = "outline";
    /// The shortest cut kept, mm: a run (what is cut between one lift and
    /// the next) shorter than this is left out and reported; 0 keeps every
    /// run. A new operation is written with its tool's diameter.
    pub const SHORTEST_CUT: &str = "shortest_cut";
}

/// The turn between two edges, in degrees, above which their junction is a
/// corner of the face's boundary.
pub const CORNER_ANGLE: f64 = 30.0;

/// Largest lead and tilt, degrees, as simultaneous finishing's.
const LEAD_LIMIT: f64 = 30.0;
const TILT_LIMIT: f64 = 30.0;

/// Two edge ends closer than this, in millimetres, are joined when the
/// edges carry no vertices.
const JOIN: f64 = 1e-3;

/// The spacing of the samples along the guide, in millimetres, before a
/// pass is refined to the tolerance.
const SAMPLE_SPACING: f64 = 0.25;

/// Most samples along the guide (NFR-07).
const MAX_GUIDE_SAMPLES: usize = 8192;

/// Samples across the face for the pass spacing, and their spacing in
/// millimetres.
const ACROSS_SPACING: f64 = 0.02;
const MIN_ACROSS: usize = 64;
const MAX_ACROSS: usize = 4096;

/// Places along the guide at which the pass spacing is measured.
const SPACING_PLACES: usize = 49;

/// Most passes on one face (NFR-07).
const MAX_PASSES: usize = 20_000;

/// How many times a pass's chord or a move is halved.
const MAX_DEPTH: u32 = 8;

/// The step in guide fraction over which the direction of travel is read.
const TANGENT_STEP: f64 = 1e-6;

/// Fractions of a straight move at which it is checked (decision records
/// 0115 and 0133).
const MOVE_CHECKS: [f64; 3] = [0.25, 0.5, 0.75];

/// The selection a host gives the strategy: the model, the faces to cut and
/// the user's guide edges (empty for the default guides).
#[derive(Debug)]
struct Selection {
    model: Arc<BrepModel>,
    faces: Vec<FaceId>,
    guide: Vec<EdgeId>,
}

/// Face-flow finishing (OP-28). The registry's instance has no model; a host
/// builds one with [`FaceFlow::on_brep`] for each operation on a STEP part.
#[derive(Debug, Default, Clone)]
pub struct FaceFlow {
    selection: Option<Arc<Selection>>,
}

impl FaceFlow {
    /// The strategy for `faces` of `model`, with the guide made of `guide`
    /// where it bounds a face (the longest side of each face otherwise).
    pub fn on_brep(model: Arc<BrepModel>, faces: Vec<FaceId>, guide: Vec<EdgeId>) -> Self {
        Self {
            selection: Some(Arc::new(Selection {
                model,
                faces,
                guide,
            })),
        }
    }
}

// ---------------------------------------------------------------------------
// The strategy.

fn number(
    name: &str,
    label: &str,
    description: &str,
    unit: Unit,
    range: (f64, f64),
    default: f64,
) -> ParameterSpec {
    ParameterSpec {
        name: name.to_owned(),
        label: label.to_owned(),
        description: description.to_owned(),
        kind: ParameterKind::Number {
            unit,
            min: Some(range.0),
            min_exclusive: false,
            max: range.1.is_finite().then_some(range.1),
            default: Some(default),
        },
    }
}

impl Strategy for FaceFlow {
    fn name(&self) -> &str {
        NAME
    }

    fn label(&self) -> &str {
        "Face flow finishing"
    }

    fn description(&self) -> &str {
        "Finishes selected faces of a STEP model with passes that run along one edge of each \
         face, the guide, and step across the face to the opposite edge, following the face's \
         shape rather than the machine's axes: for a blade floor of a blisk, for example. The \
         guide is the longest side of the face unless you choose its edges. Passes blend \
         between the guide and the opposite side where they differ in length. Where a \
         neighbouring face, such as a blade wall, is in the way of the tool, that stretch is \
         left uncut and reported. The cutting direction is chosen here until a shared setting \
         for every strategy exists. Needs a STEP model. Check the toolpath and simulation \
         before running the program."
    }

    fn needs_brep_part(&self) -> Option<&str> {
        Some("it follows the exact faces and edges of a STEP model, and a mesh has none")
    }

    fn schema(&self) -> ParameterSchema {
        use parameter::*;
        let mut schema = ParameterSchema::with_common(vec![
            number(
                SCALLOP,
                "Scallop height",
                "The highest ridge left between neighbouring passes, for a ball-end or \
                 bull-nose tool: passes are put closer than the stepover where it needs them. \
                 Zero leaves the stepover alone to set the passes.",
                Unit::Millimetres,
                (0.0, f64::INFINITY),
                0.0,
            ),
            number(
                LEAD_ANGLE,
                "Lead angle",
                "How far the tool axis leans from the surface normal towards the travel round \
                 the rotary axis. Passes that run along the rotary axis cannot lean.",
                Unit::Degrees,
                (-LEAD_LIMIT, LEAD_LIMIT),
                0.0,
            ),
            number(
                MAX_TILT,
                "Maximum tilt",
                "How far the tool axis may leave its leaned direction to smooth the turning \
                 of the part along a pass.",
                Unit::Degrees,
                (0.0, TILT_LIMIT),
                5.0,
            ),
            ParameterSpec {
                name: DIRECTION.to_owned(),
                label: "Cutting direction".to_owned(),
                description: "Meander alternates the direction of the passes; climb and \
                              conventional cut every pass the same way relative to the uncut \
                              material and the spindle. This choice belongs to face-flow \
                              finishing until a shared cutting-direction setting exists."
                    .to_owned(),
                kind: ParameterKind::Choice {
                    options: [MEANDER, CLIMB, CONVENTIONAL]
                        .iter()
                        .map(|o| (*o).to_owned())
                        .collect(),
                    default: MEANDER.to_owned(),
                },
            },
            ParameterSpec {
                name: STAY_DOWN.to_owned(),
                label: "Stay down between passes".to_owned(),
                description: "The tool stays on the face from the end of one pass to the \
                              start of the next instead of lifting, where the way is on the face \
                              and clear of the part: with Meander, and on a face cut in rings \
                              (one that closes round on itself) in any direction, as Steps \
                              between rings sets (rings never reverse). It lifts where a wall is in the way, where the \
                              step would leave the face, or where it would be longer than twice \
                              the stepover. A new operation starts with it on; one saved before \
                              this setting keeps it off."
                    .to_owned(),
                // Off for an operation saved without it, so its program is
                // unchanged; a new operation is written with it on (as
                // decision 0545 D3 writes blended ring steps).
                kind: ParameterKind::Flag { default: false },
            },
            ParameterSpec {
                name: PATTERN.to_owned(),
                label: "Pattern".to_owned(),
                description: "How the passes are laid out on each face. Along the guide: \
                              passes parallel to the guide edge, blended to the opposite side. \
                              Follow the outline: passes that follow the face's own edges, \
                              each the stepover inside the last, for a face with fingers, \
                              notches or holes. Automatic takes the guide where its passes stay \
                              whole on the face and the outline where the face's shape would \
                              break them. A new operation starts with Automatic; one saved before \
                              this setting keeps Along the guide."
                    .to_owned(),
                kind: ParameterKind::Choice {
                    options: [AUTOMATIC, ALONG_GUIDE, OUTLINE]
                        .iter()
                        .map(|o| (*o).to_owned())
                        .collect(),
                    // Along the guide for an operation saved without it, so
                    // its program is unchanged; a new operation is written
                    // with Automatic (the owner's decision of 23 Nov 2026,
                    // decision 0545 D3's pattern).
                    default: ALONG_GUIDE.to_owned(),
                },
            },
        ]);
        schema.parameters.push(ParameterSpec {
            name: SHORTEST_CUT.to_owned(),
            label: "Shortest cut".to_owned(),
            description: "A cut from one lift to the next that is shorter than this is left \
                          out and marked on the part as not cut, as an area the tool cannot \
                          reach is, and a warning names it. Nothing inside a longer cut is \
                          removed. A new operation starts at its tool's diameter; 0 keeps \
                          every cut."
                .to_owned(),
            // 0 for an operation saved without it, so its program is
            // unchanged; a new operation is written with its tool's
            // diameter (as decision 0205 writes roughing's stock to leave).
            kind: ParameterKind::Number {
                unit: Unit::Millimetres,
                min: Some(0.0),
                min_exclusive: false,
                max: None,
                default: Some(0.0),
            },
        });
        // How the tool goes from one ring to the next on a face that closes
        // round on itself, as on the ring strategies (decision 0545's
        // parameters; the owner's decision of 23 Nov 2026).
        schema.parameters.extend(crate::helical::ring_step_specs());
        for spec in &mut schema.parameters {
            if spec.name == names::STEPOVER {
                spec.description = "The largest distance between neighbouring passes, \
                                    measured along the face."
                    .to_owned();
            }
        }
        schema
    }

    fn parameter_warnings(&self, tool: &RevolvedProfile, parameters: &Parameters) -> Vec<String> {
        parameters
            .number(parameter::SCALLOP)
            .ok()
            .and_then(|height| scallop::warning(tool, height))
            .into_iter()
            .collect()
    }

    fn honours_region(&self) -> bool {
        true
    }

    fn passes_with_notes(
        &self,
        input: &StrategyInput<'_>,
        region: Option<&RegionConstraint<'_>>,
        parameters: &Parameters,
        context: &JobContext<'_>,
    ) -> Result<StrategyOutput> {
        let Some(selection) = &self.selection else {
            return Err(PathError::refused(
                crate::topic::FACE_FLOW,
                "face-flow finishing needs the exact faces of a STEP model and the faces to \
                 cut: add the part as a STEP file and select the faces"
                    .to_owned(),
            ));
        };
        let default = RegionConstraint::default();
        face_flow(
            selection,
            input,
            region.unwrap_or(&default),
            parameters,
            context,
        )
    }

    fn passes(
        &self,
        input: &StrategyInput<'_>,
        parameters: &Parameters,
        context: &JobContext<'_>,
    ) -> Result<Vec<Pass>> {
        Ok(self
            .passes_with_notes(input, None, parameters, context)?
            .passes)
    }
}

// ---------------------------------------------------------------------------
// Settings.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Meander,
    Climb,
    Conventional,
}

#[derive(Debug, Clone, Copy)]
struct Settings {
    tolerance: f64,
    stepover: f64,
    leave: f64,
    /// The cusp height and the radius of the tool's cutting arc, when a
    /// scallop height applies.
    scallop: Option<(f64, f64)>,
    lead: f64,
    tilt: f64,
    direction: Direction,
    /// Steps along the face between meander passes, where clear.
    stay_down: bool,
    pattern: Pattern,
    /// The blend angle between rings, degrees, with blended steps; `None`
    /// for sharp steps along the seam.
    ring_blend: Option<f64>,
    /// Runs shorter than this, mm, are left out; 0 keeps every run.
    shortest: f64,
}

/// How a face's passes are laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pattern {
    Automatic,
    AlongGuide,
    Outline,
}

impl Settings {
    fn read(parameters: &Parameters, tool: &RevolvedProfile) -> Result<Self> {
        use parameter::*;
        // The schema has already refused values that are not finite or are
        // out of range (a stepover of zero or less among them).
        let tolerance = parameters.number(names::TOLERANCE)?;
        let stepover = parameters.number(names::STEPOVER)?;
        let leave = parameters.number(names::STOCK_TO_LEAVE)?;
        let height = parameters.number(SCALLOP)?;
        let scallop = (height > 0.0)
            .then(|| scallop::cusp_radius(tool).map(|rho| (height, rho)))
            .flatten();
        let lead = parameters.number(LEAD_ANGLE)?;
        let tilt = parameters.number(MAX_TILT)?;
        let direction = match parameters.text(DIRECTION)? {
            CLIMB => Direction::Climb,
            CONVENTIONAL => Direction::Conventional,
            _ => Direction::Meander,
        };
        let stay_down = parameters.flag(STAY_DOWN)?;
        let pattern = match parameters.text(PATTERN)? {
            ALONG_GUIDE => Pattern::AlongGuide,
            OUTLINE => Pattern::Outline,
            _ => Pattern::Automatic,
        };
        let ring_blend = if matches!(
            parameters.get(crate::helical::parameter::RING_STEP),
            Some(crate::params::ParameterValue::Text(t)) if t == crate::helical::parameter::BLENDED
        ) {
            let phi = match parameters.get(crate::helical::parameter::RING_BLEND_ANGLE) {
                Some(crate::params::ParameterValue::Number(n)) => *n,
                None => crate::helical::parameter::DEFAULT_RING_BLEND_ANGLE,
                Some(_) => f64::NAN,
            };
            if !(phi > 0.0 && phi <= 180.0) {
                return Err(PathError::Parameter {
                    name: crate::helical::parameter::RING_BLEND_ANGLE.to_owned(),
                    reason: format!("{phi} degrees is not more than 0 and at most 180"),
                });
            }
            Some(phi)
        } else {
            None
        };
        Ok(Self {
            tolerance,
            stepover,
            leave,
            scallop,
            lead,
            tilt,
            direction,
            stay_down,
            pattern,
            ring_blend,
            shortest: parameters.number(SHORTEST_CUT)?,
        })
    }
}

// ---------------------------------------------------------------------------
// Curves by arc length.

/// A polyline taken by arc-length fraction.
#[derive(Debug, Clone)]
struct Curve {
    points: Vec<Point3>,
    /// Cumulative length at each point.
    cum: Vec<f64>,
}

impl Curve {
    fn new(points: Vec<Point3>) -> Self {
        let mut cum = Vec::with_capacity(points.len());
        let mut total = 0.0;
        let mut last: Option<Point3> = None;
        for p in &points {
            if let Some(q) = last {
                total += p.distance(q);
            }
            cum.push(total);
            last = Some(*p);
        }
        Self { points, cum }
    }

    fn length(&self) -> f64 {
        self.cum.last().copied().unwrap_or(0.0)
    }

    /// The point at fraction `s` of the length.
    fn at(&self, s: f64) -> Option<Point3> {
        let total = self.length();
        let first = *self.points.first()?;
        if total <= 0.0 {
            return Some(first);
        }
        let target = s.clamp(0.0, 1.0) * total;
        let i = self.cum.partition_point(|c| *c < target);
        if i == 0 {
            return Some(first);
        }
        let (a, b) = (*self.points.get(i - 1)?, *self.points.get(i)?);
        let (ca, cb) = (*self.cum.get(i - 1)?, *self.cum.get(i)?);
        let f = if cb > ca {
            (target - ca) / (cb - ca)
        } else {
            0.0
        };
        Some(a + (b - a) * f)
    }

    fn reversed(&self) -> Self {
        Self::new(self.points.iter().rev().copied().collect())
    }
}

// ---------------------------------------------------------------------------
// Loops, sides, the guide and the opposite boundary.

/// One edge of a loop, in the loop's direction.
#[derive(Debug, Clone)]
struct Entry {
    edge: EdgeId,
    points: Vec<Point3>,
}

/// An end of an edge: its vertex, when it has one, and its point.
#[derive(Debug, Clone, Copy)]
struct End {
    vertex: Option<u32>,
    point: Point3,
}

fn same(a: End, b: End) -> bool {
    match (a.vertex, b.vertex) {
        (Some(x), Some(y)) => x == y,
        _ => a.point.distance(b.point) < JOIN,
    }
}

/// The points of an edge from its first vertex to its last: its exact curve
/// sampled when it has one (decision 0227), its polyline otherwise.
fn edge_points(model: &BrepModel, edge: EdgeId) -> Option<(Vec<Point3>, End, End)> {
    let e = model.edges.get(edge.0 as usize)?;
    let points = e
        .curve
        .as_ref()
        .and_then(crate::edge_curve::sample)
        .filter(|p| p.len() >= 2)
        .unwrap_or_else(|| e.polyline.clone());
    let (first, last) = (*points.first()?, *points.last()?);
    let (va, vb) = match e.vertices {
        Some([a, b]) => (Some(a.0), Some(b.0)),
        None => (None, None),
    };
    Some((
        points,
        End {
            vertex: va,
            point: first,
        },
        End {
            vertex: vb,
            point: last,
        },
    ))
}

/// The face's boundary loops, without the edges in `skip` (its seams), each
/// joined end to end, the lowest edge first and each loop starting at its
/// lowest edge.
fn loops(model: &BrepModel, face: &BrepFace, skip: &BTreeSet<EdgeId>) -> Result<Vec<Vec<Entry>>> {
    let ids: BTreeSet<EdgeId> = face
        .edges
        .iter()
        .copied()
        .filter(|e| !skip.contains(e))
        .collect();
    let mut pieces = Vec::new();
    for id in ids {
        let (points, a, b) = edge_points(model, id).ok_or_else(|| {
            PathError::refused(
                crate::topic::FACE_FLOW,
                format!(
                    "edge {} of face {} has no points to follow",
                    id.0, face.id.0
                ),
            )
        })?;
        pieces.push((id, points, a, b));
    }
    let mut used = vec![false; pieces.len()];
    let mut out = Vec::new();
    while let Some(first) = used.iter().position(|u| !*u) {
        let Some((id, points, a, b)) = pieces.get(first).cloned() else {
            break;
        };
        if let Some(u) = used.get_mut(first) {
            *u = true;
        }
        let start = a;
        let mut end = b;
        let mut lp = vec![Entry { edge: id, points }];
        while !same(end, start) {
            let next = pieces.iter().enumerate().find_map(|(j, (_, _, pa, pb))| {
                if used.get(j).copied().unwrap_or(true) {
                    None
                } else if same(*pa, end) {
                    Some((j, false))
                } else if same(*pb, end) {
                    Some((j, true))
                } else {
                    None
                }
            });
            let Some((j, reversed)) = next else {
                return Err(PathError::refused(
                    crate::topic::FACE_FLOW,
                    format!(
                        "the boundary of face {} does not close: its edges are not joined end to \
                     end",
                        face.id.0
                    ),
                ));
            };
            if let Some(u) = used.get_mut(j) {
                *u = true;
            }
            let Some((jid, jpoints, ja, jb)) = pieces.get(j).cloned() else {
                break;
            };
            if reversed {
                end = ja;
                lp.push(Entry {
                    edge: jid,
                    points: jpoints.into_iter().rev().collect(),
                });
            } else {
                end = jb;
                lp.push(Entry {
                    edge: jid,
                    points: jpoints,
                });
            }
        }
        out.push(lp);
    }
    Ok(out)
}

fn entries_length(entries: &[Entry]) -> f64 {
    entries
        .iter()
        .map(|e| Curve::new(e.points.clone()).length())
        .sum()
}

/// The unit direction along `points` at its start (`true`) or end.
fn tangent(points: &[Point3], at_start: bool) -> Option<Vec3> {
    let pick = |a: Point3, b: Point3| {
        let d = b - a;
        let l = d.length();
        (l > 1e-12).then(|| d * (1.0 / l))
    };
    if at_start {
        let a = *points.first()?;
        points.iter().skip(1).find_map(|b| pick(a, *b))
    } else {
        let b = *points.last()?;
        points.iter().rev().skip(1).find_map(|a| pick(*a, b))
    }
}

/// Whether the loop turns by more than [`CORNER_ANGLE`] from `a` to `b`.
fn corner(a: &Entry, b: &Entry) -> bool {
    match (tangent(&a.points, false), tangent(&b.points, true)) {
        (Some(x), Some(y)) => x.dot(y).clamp(-1.0, 1.0).acos().to_degrees() > CORNER_ANGLE,
        _ => true,
    }
}

/// What a field of passes is built between.
#[derive(Debug, Clone)]
struct Boundaries {
    guide: Curve,
    guide_edges: Vec<EdgeId>,
    opposite: Opposite,
}

#[derive(Debug, Clone)]
enum Opposite {
    Side(Curve),
    Point(Point3),
}

impl Opposite {
    fn at(&self, s: f64) -> Option<Point3> {
        match self {
            Self::Side(c) => c.at(s),
            Self::Point(p) => Some(*p),
        }
    }
}

fn concat(entries: &[Entry]) -> Vec<Point3> {
    let mut out: Vec<Point3> = Vec::new();
    for e in entries {
        for p in &e.points {
            if out.last().is_none_or(|q| q.distance(*p) > 1e-12) {
                out.push(*p);
            }
        }
    }
    out
}

/// The sides of `entries` (in order, open), split at corners.
fn sides_of(entries: &[Entry]) -> Vec<Vec<Entry>> {
    let mut sides: Vec<Vec<Entry>> = Vec::new();
    let mut current: Vec<Entry> = Vec::new();
    for e in entries {
        if let Some(last) = current.last()
            && corner(last, e)
        {
            sides.push(std::mem::take(&mut current));
        }
        current.push(e.clone());
    }
    if !current.is_empty() {
        sides.push(current);
    }
    sides
}

/// Orient the opposite boundary so it starts near the guide's start.
fn oriented(guide: &Curve, opposite: Curve) -> Curve {
    let (Some(g0), Some(g1), Some(o0), Some(o1)) = (
        guide.points.first(),
        guide.points.last(),
        opposite.points.first(),
        opposite.points.last(),
    ) else {
        return opposite;
    };
    let keep = g0.distance(*o0) + g1.distance(*o1);
    let flip = g0.distance(*o1) + g1.distance(*o0);
    if flip < keep {
        opposite.reversed()
    } else {
        opposite
    }
}

/// The guide and opposite boundary of `face`, the guide made of `chosen`
/// where any of them bound it.
fn boundaries(
    model: &BrepModel,
    face: &BrepFace,
    chosen: &BTreeSet<EdgeId>,
    closed: &Closure,
    exact: &ExactFace,
) -> Result<Boundaries> {
    if !closed.seams.is_empty() {
        return ring_boundaries(model, face, chosen, closed, exact);
    }
    let mut all = loops(model, face, &BTreeSet::new())?;
    if all.is_empty() {
        return Err(PathError::refused(
            crate::topic::FACE_FLOW,
            format!("face {} has no boundary edges to follow", face.id.0),
        ));
    }
    // The outer loop is the longest; the others are holes, cut round by
    // the face's trim test.
    let mut best = 0;
    let mut best_len = f64::NEG_INFINITY;
    for (i, l) in all.iter().enumerate() {
        let len = entries_length(l);
        if len > best_len * (1.0 + 1e-9) {
            best = i;
            best_len = len;
        }
    }
    let outer = all.swap_remove(best);
    let n = outer.len();
    let mine: Vec<usize> = outer
        .iter()
        .enumerate()
        .filter(|(_, e)| chosen.contains(&e.edge))
        .map(|(i, _)| i)
        .collect();
    let corners: Vec<usize> = (0..n)
        .filter(|&i| match (outer.get(i), outer.get((i + 1) % n)) {
            (Some(a), Some(b)) => n > 1 && corner(a, b),
            _ => false,
        })
        .collect();
    if mine.is_empty() && corners.len() < 2 {
        return Ok(halves(&outer));
    }
    // The guide's entries as a range of the loop, from `g0` for `len`.
    let (g0, len) = if mine.is_empty() {
        // Rotate so the loop starts after a corner, then take its sides.
        let start = corners.first().map_or(0, |c| (c + 1) % n);
        let rotated: Vec<Entry> = (0..n)
            .filter_map(|k| outer.get((start + k) % n).cloned())
            .collect();
        let sides = sides_of(&rotated);
        let mut pick: Option<(usize, f64, EdgeId, usize)> = None;
        let mut offset = 0;
        for side in &sides {
            let length = entries_length(side);
            let lowest = side
                .iter()
                .map(|e| e.edge)
                .min()
                .unwrap_or(EdgeId(u32::MAX));
            let better = match pick {
                None => true,
                Some((_, l, id, _)) => {
                    length > l * (1.0 + 1e-6) || (length >= l * (1.0 - 1e-6) && lowest < id)
                }
            };
            if better {
                pick = Some((offset, length, lowest, side.len()));
            }
            offset += side.len();
        }
        let (at, _, _, count) = pick.unwrap_or((0, 0.0, EdgeId(0), 1));
        ((start + at) % n, count)
    } else {
        contiguous(&mine, n).ok_or_else(|| {
            PathError::refused(
                crate::topic::FACE_FLOW,
                format!(
                    "the guide edges {} of face {} are not joined end to end into one chain",
                    list(mine.iter().filter_map(|i| outer.get(*i)).map(|e| e.edge.0)),
                    face.id.0
                ),
            )
        })?
    };
    if len >= n {
        return Err(PathError::refused(
            crate::topic::FACE_FLOW,
            format!(
                "the guide of face {} is its whole boundary: choose fewer edges, so there is an \
             opposite side to step towards",
                face.id.0
            ),
        ));
    }
    let guide_entries: Vec<Entry> = (0..len)
        .filter_map(|k| outer.get((g0 + k) % n).cloned())
        .collect();
    let rest: Vec<Entry> = (len..n)
        .filter_map(|k| outer.get((g0 + k) % n).cloned())
        .collect();
    let guide = Curve::new(concat(&guide_entries));
    let sides = sides_of(&rest);
    let opposite = match sides.len() {
        0 | 1 => Opposite::Side(oriented(&guide, Curve::new(concat(&rest)))),
        2 => {
            let apex = sides
                .first()
                .and_then(|s| s.last())
                .and_then(|e| e.points.last())
                .copied()
                .ok_or_else(|| {
                    PathError::refused(
                        crate::topic::FACE_FLOW,
                        format!("face {} has an empty side", face.id.0),
                    )
                })?;
            Opposite::Point(apex)
        }
        k => {
            let middle: Vec<Entry> = sides
                .get(1..k - 1)
                .unwrap_or_default()
                .iter()
                .flatten()
                .cloned()
                .collect();
            Opposite::Side(oriented(&guide, Curve::new(concat(&middle))))
        }
    };
    Ok(Boundaries {
        guide,
        guide_edges: guide_entries.iter().map(|e| e.edge).collect(),
        opposite,
    })
}

// ---------------------------------------------------------------------------
// A face that closes round on itself (issue #791; decision record 0675).

/// How a face closes round on itself: its seams, the edges along which it
/// meets itself, and the parameter period in u and in v in each direction
/// it closes, from the start of its parameter box.
#[derive(Debug, Clone, Default)]
struct Closure {
    seams: BTreeSet<EdgeId>,
    /// (start, period) in u and in v.
    period: [Option<(f64, f64)>; 2],
}

/// The number of points of an edge or loop read for a seam or a turn.
const CLOSURE_SAMPLES: usize = 64;

/// The points a ring is measured at to judge whether it is shorter than the
/// tool.
const RING_SAMPLES: usize = 64;

/// How far short of a point end, as a share of the way across, the face is
/// read where the end has no normal.
const POINT_END_STEP: f64 = 1e-6;

/// At most `n` points of `points`, evenly spread, the first and last kept.
fn spread(points: &[Point3], n: usize) -> Vec<Point3> {
    if points.len() <= n {
        return points.to_vec();
    }
    (0..n)
        .filter_map(|k| points.get(k * (points.len() - 1) / (n - 1)).copied())
        .collect()
}

/// Whether and how `face` closes round on itself (decision record 0675):
/// it closes in u where the two ends of its parameter box are the same
/// curve on the part, and a seam is an edge of no other face, not a point,
/// lying along that curve.
fn closure(model: &BrepModel, face: &BrepFace, exact: &ExactFace) -> Closure {
    let (lo, hi) = exact.uv_bounds();
    let closes = |dir: usize| -> bool {
        // Inside the box only: a cone's apex or a sphere's pole has no
        // normal.
        (1..8).all(|k| {
            let t = f64::from(k) / 8.0;
            let (a, b) = if dir == 0 {
                let v = lo.v + (hi.v - lo.v) * t;
                (Uv { u: lo.u, v }, Uv { u: hi.u, v })
            } else {
                let u = lo.u + (hi.u - lo.u) * t;
                (Uv { u, v: lo.v }, Uv { u, v: hi.v })
            };
            match (exact.point_normal(a), exact.point_normal(b)) {
                (Some(a), Some(b)) => a.point.distance(b.point) < JOIN,
                _ => false,
            }
        })
    };
    let mut out = Closure::default();
    for dir in 0..2 {
        if !closes(dir) {
            continue;
        }
        if let Some(slot) = out.period.get_mut(dir) {
            *slot = Some(if dir == 0 {
                (lo.u, hi.u - lo.u)
            } else {
                (lo.v, hi.v - lo.v)
            });
        }
        for id in &face.edges {
            let Some(edge) = model.edges.get(id.0 as usize) else {
                continue;
            };
            if edge.faces.as_slice() != [face.id] {
                continue;
            }
            let Some((points, _, _)) = edge_points(model, *id) else {
                continue;
            };
            if Curve::new(points.clone()).length() <= JOIN {
                continue;
            }
            let (start, end) = if dir == 0 { (lo.u, hi.u) } else { (lo.v, hi.v) };
            let near = 1e-7 * (1.0 + (end - start).abs());
            // Each point is on the face, at the start or end of the box
            // across the seam, or on the part where the start would be.
            let on_seam = spread(&points, 17).iter().all(|p| {
                exact.closest(*p).is_some_and(|c| {
                    let x = if dir == 0 { c.uv.u } else { c.uv.v };
                    let at = if dir == 0 {
                        Uv { u: lo.u, v: c.uv.v }
                    } else {
                        Uv { u: c.uv.u, v: lo.v }
                    };
                    c.distance < JOIN
                        && ((x - start).abs() <= near
                            || (x - end).abs() <= near
                            || exact
                                .point_normal(at)
                                .is_some_and(|q| q.point.distance(*p) < JOIN))
                })
            });
            if on_seam {
                out.seams.insert(*id);
            }
        }
    }
    out
}

/// Whether the closed loop `points` goes once round a face that closes with
/// `period` in the parameter `dir`: an end of the face, not a hole in it.
fn goes_round(points: &[Point3], exact: &ExactFace, dir: usize, period: f64) -> bool {
    let mut total = 0.0;
    let mut last: Option<f64> = None;
    for p in spread(points, CLOSURE_SAMPLES) {
        let Some(c) = exact.closest(p) else {
            return false;
        };
        let x = if dir == 0 { c.uv.u } else { c.uv.v };
        if let Some(prev) = last {
            let x = unwrap_to(x, prev, Some(period));
            total += x - prev;
            last = Some(x);
        } else {
            last = Some(x);
        }
    }
    total.abs() > 0.5 * period
}

/// The closed polyline `points` started at its point nearest `at`.
fn started_at(points: &[Point3], at: &[Point3]) -> Vec<Point3> {
    let mut ring: Vec<Point3> = points.to_vec();
    if ring.len() > 1
        && let (Some(a), Some(b)) = (ring.first(), ring.last())
        && a.distance(*b) < JOIN
    {
        ring.pop();
    }
    let nearest = |p: &Point3| {
        at.iter()
            .map(|q| p.distance(*q))
            .fold(f64::INFINITY, f64::min)
    };
    let start = ring
        .iter()
        .enumerate()
        .min_by(|a, b| nearest(a.1).total_cmp(&nearest(b.1)))
        .map_or(0, |(i, _)| i);
    let n = ring.len();
    let mut out: Vec<Point3> = (0..n)
        .filter_map(|k| ring.get((start + k) % n).copied())
        .collect();
    if let Some(first) = out.first().copied() {
        out.push(first);
    }
    out
}

/// The guide and opposite end of a face that closes round on itself
/// (decision record 0675, option A): its seams are not boundaries; its two
/// ends are the guide (the longer, or the one holding the user's guide
/// edges) and the opposite boundary, or a point where an end closes on one.
/// Each pass is then a ring round the face.
fn ring_boundaries(
    model: &BrepModel,
    face: &BrepFace,
    chosen: &BTreeSet<EdgeId>,
    closed: &Closure,
    exact: &ExactFace,
) -> Result<Boundaries> {
    let refuse = |text: String| PathError::refused(crate::topic::FACE_FLOW, text);
    if let Some(seam) = chosen.iter().find(|e| closed.seams.contains(e)) {
        return Err(refuse(format!(
            "edge {} is the seam of face {}, where the face meets itself, not one of its ends: \
             choose an end edge as the guide, or leave the Guide edge set empty",
            seam.0, face.id.0
        )));
    }
    let no_end = || {
        refuse(format!(
            "face {} closes round on itself with no end edge to cut from: choose a face with \
             an end, or cut it with simultaneous or helical finishing",
            face.id.0
        ))
    };
    // The direction the face closes in: across the seams.
    let Some((dir, (_, period))) = closed
        .period
        .iter()
        .enumerate()
        .find_map(|(d, p)| p.map(|p| (d, p)))
    else {
        return Err(no_end());
    };
    if closed.period.iter().all(Option::is_some) {
        return Err(no_end());
    }
    let mut ends: Vec<Vec<Entry>> = Vec::new();
    let mut points: Vec<Point3> = Vec::new();
    for l in loops(model, face, &closed.seams)? {
        let ring = concat(&l);
        if entries_length(&l) <= JOIN {
            points.extend(ring.first().copied());
        } else if goes_round(&ring, exact, dir, period) {
            ends.push(l);
        }
    }
    if ends.is_empty() || ends.len() > 2 {
        return Err(no_end());
    }
    // The user's guide edges on this face: another face's guide is that
    // face's, and this face then takes its own default end.
    let local: BTreeSet<EdgeId> = face
        .edges
        .iter()
        .copied()
        .filter(|e| chosen.contains(e))
        .collect();
    // An end is the guide only when every local guide edge is on it.
    let holds = |l: &Vec<Entry>| local.iter().all(|g| l.iter().any(|e| e.edge == *g));
    let guide_at = if local.is_empty() {
        let mut best = 0;
        let mut key = (f64::NEG_INFINITY, EdgeId(u32::MAX));
        for (i, l) in ends.iter().enumerate() {
            let length = entries_length(l);
            let lowest = l.iter().map(|e| e.edge).min().unwrap_or(EdgeId(u32::MAX));
            if length > key.0 * (1.0 + 1e-6) || (length >= key.0 * (1.0 - 1e-6) && lowest < key.1) {
                best = i;
                key = (length, lowest);
            }
        }
        best
    } else {
        let at: Vec<usize> = (0..ends.len())
            .filter(|i| ends.get(*i).is_some_and(holds))
            .collect();
        match at.as_slice() {
            [i] => *i,
            _ => {
                return Err(refuse(format!(
                    "the guide edges {} of face {} are not all on one of its ends: face {} closes \
                     round on itself, and its passes run round it from one end",
                    list(local.iter().map(|e| e.0)),
                    face.id.0,
                    face.id.0
                )));
            }
        }
    };
    let guide_entries = ends.swap_remove(guide_at);
    let seam_ends: Vec<Point3> = closed
        .seams
        .iter()
        .filter_map(|e| edge_points(model, *e))
        .flat_map(|(_, a, b)| [a.point, b.point])
        .collect();
    let guide = Curve::new(started_at(&concat(&guide_entries), &seam_ends));
    let opposite = match (ends.first(), points.first()) {
        (Some(other), _) => {
            let curve = Curve::new(started_at(&concat(other), &seam_ends));
            // The same way round as the guide.
            let reversed = curve.reversed();
            let near = |c: &Curve| match (guide.at(0.25), c.at(0.25)) {
                (Some(a), Some(b)) => a.distance(b),
                _ => f64::INFINITY,
            };
            Opposite::Side(if near(&reversed) < near(&curve) {
                reversed
            } else {
                curve
            })
        }
        (None, Some(p)) => Opposite::Point(*p),
        (None, None) => return Err(no_end()),
    };
    Ok(Boundaries {
        guide,
        guide_edges: guide_entries.iter().map(|e| e.edge).collect(),
        opposite,
    })
}

/// The start and count of `indices` as one run of the cyclic range `0..n`,
/// or `None` when they are not one run.
fn contiguous(indices: &[usize], n: usize) -> Option<(usize, usize)> {
    let set: BTreeSet<usize> = indices.iter().copied().collect();
    let len = set.len();
    if len == 0 {
        return None;
    }
    // The run starts at an index whose predecessor is not in the set.
    let starts: Vec<usize> = set
        .iter()
        .copied()
        .filter(|i| len == n || !set.contains(&((i + n - 1) % n)))
        .collect();
    match starts.as_slice() {
        [start] => Some((*start, len)),
        _ if len == n => Some((0, n)),
        _ => None,
    }
}

/// A smooth loop cut at its two points furthest apart: the longer half is
/// the guide, the other the opposite side.
fn halves(outer: &[Entry]) -> Boundaries {
    let mut points: Vec<(Point3, EdgeId)> = Vec::new();
    for e in outer {
        for p in &e.points {
            if points.last().is_none_or(|(q, _)| q.distance(*p) > 1e-12) {
                points.push((*p, e.edge));
            }
        }
    }
    if points.len() > 1
        && let (Some(a), Some(b)) = (points.first(), points.last())
        && a.0.distance(b.0) < JOIN
    {
        points.pop();
    }
    let step = (points.len() / 400).max(1);
    let candidates: Vec<usize> = (0..points.len()).step_by(step).collect();
    let mut far = (0, 0, f64::NEG_INFINITY);
    for (k, &i) in candidates.iter().enumerate() {
        for &j in candidates.iter().skip(k + 1) {
            if let (Some(a), Some(b)) = (points.get(i), points.get(j)) {
                let d = a.0.distance(b.0);
                if d > far.2 {
                    far = (i, j, d);
                }
            }
        }
    }
    let (i, j, _) = far;
    let n = points.len();
    let first: Vec<(Point3, EdgeId)> = (i..=j).filter_map(|k| points.get(k).copied()).collect();
    let second: Vec<(Point3, EdgeId)> = (j..=i + n)
        .filter_map(|k| points.get(k % n.max(1)).copied())
        .collect();
    let curve = |half: &[(Point3, EdgeId)]| Curve::new(half.iter().map(|p| p.0).collect());
    let (a, b) = (curve(&first), curve(&second));
    let (guide, opposite, edges) = if a.length() >= b.length() {
        (a, b, &first)
    } else {
        (b, a, &second)
    };
    let guide_edges: BTreeSet<EdgeId> = edges.iter().map(|p| p.1).collect();
    let opposite = oriented(&guide, opposite);
    Boundaries {
        guide,
        guide_edges: guide_edges.into_iter().collect(),
        opposite: Opposite::Side(opposite),
    }
}

fn list(ids: impl Iterator<Item = u32>) -> String {
    ids.map(|i| i.to_string()).collect::<Vec<_>>().join(", ")
}

// ---------------------------------------------------------------------------
// The field of passes on one face.

/// The face's parameter period in u and in v, for surfaces that close round
/// an axis.
fn periods(surface: &Surface) -> (Option<f64>, Option<f64>) {
    match surface {
        Surface::Cylinder { .. } | Surface::Cone { .. } | Surface::Sphere { .. } => {
            (Some(TAU), None)
        }
        Surface::Torus { .. } => (Some(TAU), Some(TAU)),
        _ => (None, None),
    }
}

fn unwrap_to(x: f64, near: f64, period: Option<f64>) -> f64 {
    match period {
        Some(p) => x + ((near - x) / p).round() * p,
        None => x,
    }
}

/// One face's passes in its own terms: the face, its boundaries in (u, v)
/// by guide fraction, and the evaluated grid used for the spacing.
struct Field<'a> {
    id: FaceId,
    face: &'a ExactFace,
    boundaries: Boundaries,
    /// (u, v) of the guide and of the opposite boundary at `s = j / m`.
    guide_uv: Vec<Uv>,
    opposite_uv: Vec<Uv>,
    /// (start, period) in u and in v where the face closes round on itself,
    /// to bring a blended (u, v) back into its parameter box.
    wrap: [Option<(f64, f64)>; 2],
}

impl Field<'_> {
    fn m(&self) -> usize {
        self.guide_uv.len().saturating_sub(1)
    }

    /// The (u, v) at guide fraction `s` and weight `w`.
    fn uv(&self, s: f64, w: f64) -> Option<Uv> {
        let m = self.m();
        let x = s.clamp(0.0, 1.0) * m as f64;
        let j = (x.floor() as usize).min(m.saturating_sub(1));
        let f = x - j as f64;
        let lerp = |v: &[Uv]| -> Option<Uv> {
            let (a, b) = (v.get(j)?, v.get((j + 1).min(m))?);
            Some(Uv {
                u: a.u + (b.u - a.u) * f,
                v: a.v + (b.v - a.v) * f,
            })
        };
        let (g, o) = (lerp(&self.guide_uv)?, lerp(&self.opposite_uv)?);
        let back = |x: f64, wrap: Option<(f64, f64)>| match wrap {
            Some((start, period)) if period > 0.0 => start + (x - start).rem_euclid(period),
            _ => x,
        };
        let [wu, wv] = self.wrap;
        Some(Uv {
            u: back(g.u + (o.u - g.u) * w, wu),
            v: back(g.v + (o.v - g.v) * w, wv),
        })
    }

    /// The face's point and normal at (s, w); on the face's boundary by
    /// construction where `s` or `w` is 0 or 1, otherwise only inside its
    /// trim.
    fn at(&self, s: f64, w: f64) -> Option<SurfacePoint> {
        let uv = self.uv(s, w)?;
        let edge = s <= 0.0 || s >= 1.0 || w <= 0.0 || w >= 1.0;
        if !edge && !self.face.contains(uv) {
            return None;
        }
        // A point end with no normal there (a cone's apex) is read just
        // short of it, so the spacing across the face reaches it.
        self.face.point_normal(uv).or_else(|| {
            (w >= 1.0 && matches!(self.boundaries.opposite, Opposite::Point(_)))
                .then(|| self.at(s, 1.0 - POINT_END_STEP))
                .flatten()
        })
    }
}

/// Puts the boundary samples on the face.
fn field<'a>(
    model: &BrepModel,
    face_id: FaceId,
    face: &'a ExactFace,
    boundaries: Boundaries,
    closed: &Closure,
) -> Result<Field<'a>> {
    let brep = model.faces.get(face_id.0 as usize).ok_or_else(|| {
        PathError::refused(
            crate::topic::FACE_FLOW,
            format!("the model has no face {}", face_id.0),
        )
    })?;
    let (pu, pv) = periods(&brep.geometry);
    // A face that closes round with no period of its own (a closed B-spline)
    // takes its parameter box's as its period.
    let [cu, cv] = closed.period;
    let (pu, pv) = (pu.or(cu.map(|c| c.1)), pv.or(cv.map(|c| c.1)));
    let length = boundaries.guide.length().max(match &boundaries.opposite {
        Opposite::Side(c) => c.length(),
        Opposite::Point(_) => 0.0,
    });
    let m = ((length / SAMPLE_SPACING).ceil() as usize).clamp(16, MAX_GUIDE_SAMPLES);
    let place = |p: Point3, seed: Option<Uv>| -> Result<Uv> {
        let found = match seed {
            Some(seed) => face.closest_from(p, seed, f64::INFINITY),
            None => face.closest(p),
        };
        found.map(|c| c.uv).ok_or_else(|| {
            PathError::refused(
                crate::topic::FACE_FLOW,
                format!(
                    "the boundary of face {} could not be put on its surface",
                    face_id.0
                ),
            )
        })
    };
    let mut guide_uv: Vec<Uv> = Vec::with_capacity(m + 1);
    let mut opposite_uv: Vec<Uv> = Vec::with_capacity(m + 1);
    for j in 0..=m {
        let s = j as f64 / m as f64;
        let g = boundaries.guide.at(s).ok_or_else(|| {
            PathError::refused(
                crate::topic::FACE_FLOW,
                format!("the guide of face {} is empty", face_id.0),
            )
        })?;
        let o = boundaries.opposite.at(s).ok_or_else(|| {
            PathError::refused(
                crate::topic::FACE_FLOW,
                format!("the opposite side of face {} is empty", face_id.0),
            )
        })?;
        let mut gu = place(g, guide_uv.last().copied())?;
        let mut ou = match (&boundaries.opposite, opposite_uv.last()) {
            (Opposite::Point(_), Some(prev)) => *prev,
            (_, prev) => place(o, prev.copied())?,
        };
        // Keep each boundary continuous across a seam, and the opposite
        // boundary on the same turn as the guide.
        if let Some(prev) = guide_uv.last() {
            gu.u = unwrap_to(gu.u, prev.u, pu);
            gu.v = unwrap_to(gu.v, prev.v, pv);
        }
        if let Some(prev) = opposite_uv.last() {
            ou.u = unwrap_to(ou.u, prev.u, pu);
            ou.v = unwrap_to(ou.v, prev.v, pv);
        } else {
            ou.u = unwrap_to(ou.u, gu.u, pu);
            ou.v = unwrap_to(ou.v, gu.v, pv);
        }
        // On a face that closes round on itself, a point end turns with the
        // guide in the direction the face closes in, so every ring goes once
        // round (CodeRabbit on #854); its other parameter stays the point's.
        if matches!(boundaries.opposite, Opposite::Point(_)) {
            let [cu, cv] = closed.period;
            if cu.is_some() {
                ou.u = gu.u;
            }
            if cv.is_some() {
                ou.v = gu.v;
            }
        }
        guide_uv.push(gu);
        opposite_uv.push(ou);
    }
    Ok(Field {
        id: face_id,
        face,
        boundaries,
        guide_uv,
        opposite_uv,
        wrap: closed.period,
    })
}

/// The face sampled across, at a few places along the guide, for the pass
/// spacing.
struct Across {
    /// At each place: the points and normals at `w = i / n`, and the length
    /// along the face from `w = 0`.
    places: Vec<Vec<Option<(Point3, Vec3, f64)>>>,
    n: usize,
}

impl Across {
    fn new(field: &Field<'_>) -> Self {
        let mut width: f64 = 0.0;
        for k in 0..SPACING_PLACES {
            let s = k as f64 / (SPACING_PLACES - 1) as f64;
            if let (Some(a), Some(b)) = (field.at(s, 0.0), field.at(s, 1.0)) {
                width = width.max(a.point.distance(b.point));
            }
        }
        let n = ((width / ACROSS_SPACING).ceil() as usize).clamp(MIN_ACROSS, MAX_ACROSS);
        let places = (0..SPACING_PLACES)
            .map(|k| {
                let s = k as f64 / (SPACING_PLACES - 1) as f64;
                let mut along = 0.0;
                let mut last: Option<Point3> = None;
                (0..=n)
                    .map(|i| {
                        let w = i as f64 / n as f64;
                        let sp = field.at(s, w)?;
                        if let Some(q) = last {
                            along += sp.point.distance(q);
                        }
                        last = Some(sp.point);
                        Some((sp.point, sp.normal.get(), along))
                    })
                    .collect()
            })
            .collect();
        Self { places, n }
    }

    /// The point, normal and length along at place `k` and weight `w`,
    /// interpolated.
    fn at(&self, k: usize, w: f64) -> Option<(Point3, Vec3, f64)> {
        let row = self.places.get(k)?;
        let x = w.clamp(0.0, 1.0) * self.n as f64;
        let i = (x.floor() as usize).min(self.n.saturating_sub(1));
        let f = x - i as f64;
        let (a, b) = ((*row.get(i)?)?, (*row.get(i + 1)?)?);
        let n = a.1 + (b.1 - a.1) * f;
        let len = n.length();
        Some((
            a.0 + (b.0 - a.0) * f,
            if len > 0.0 { n * (1.0 / len) } else { a.1 },
            a.2 + (b.2 - a.2) * f,
        ))
    }

    /// How far the step from `wa` to `wb` uses the allowance, at its worst
    /// place: 1 is the stepover or the scallop height exactly.
    fn ratio(&self, wa: f64, wb: f64, settings: &Settings) -> f64 {
        let mut worst: f64 = 0.0;
        for k in 0..self.places.len() {
            let (Some(a), Some(b)) = (self.at(k, wa), self.at(k, wb)) else {
                continue;
            };
            worst = worst.max((b.2 - a.2).abs() / settings.stepover);
            if let Some((height, rho)) = settings.scallop
                && let Some(m) = self.at(k, 0.5 * (wa + wb))
            {
                let (ca, cb) = (a.0 + a.1 * rho, b.0 + b.1 * rho);
                let d = ca.distance(cb);
                let cusp = if d >= 2.0 * rho {
                    f64::INFINITY
                } else {
                    let down = -(a.1 + b.1);
                    let dl = down.length();
                    if dl <= 1e-12 {
                        0.0
                    } else {
                        let mid = ca + (cb - ca) * 0.5;
                        let q = mid + down * ((rho * rho - 0.25 * d * d).sqrt() / dl);
                        (q - m.0).dot(m.1)
                    }
                };
                worst = worst.max(cusp / height);
            }
        }
        worst
    }

    /// The width of the face across at guide fraction `s`, along the face.
    fn width(&self, s: f64) -> f64 {
        let k = (s.clamp(0.0, 1.0) * (self.places.len().saturating_sub(1)) as f64).round() as usize;
        self.at(k, 1.0).map_or(f64::INFINITY, |p| p.2)
    }
}

/// The weights of the passes with each step using at most `limit` of the
/// allowance; `None` when there are more than [`MAX_PASSES`].
fn greedy(across: &Across, settings: &Settings, limit: f64) -> Result<Option<Vec<f64>>> {
    let mut weights = vec![0.0];
    let mut w = 0.0;
    while w < 1.0 {
        if weights.len() > MAX_PASSES {
            return Ok(None);
        }
        if across.ratio(w, 1.0, settings) <= limit {
            weights.push(1.0);
            break;
        }
        let (mut lo, mut hi) = (w, 1.0);
        for _ in 0..40 {
            let mid = 0.5 * (lo + hi);
            if across.ratio(w, mid, settings) <= limit {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        if lo - w < 1e-12 {
            return Err(PathError::parameter(
                names::STEPOVER,
                "the stepover (or scallop height) is too small for this face to be cut",
            ));
        }
        weights.push(lo);
        w = lo;
    }
    Ok(Some(weights))
}

/// The weights of the passes, as few as the allowance needs and evened out.
fn weights(across: &Across, settings: &Settings, face: FaceId) -> Result<Vec<f64>> {
    let too_many = || {
        PathError::refused(
            crate::topic::FACE_FLOW,
            format!(
                "face {} would need more than {MAX_PASSES} passes: raise the stepover",
                face.0
            ),
        )
        .with_setting(crate::params::names::STEPOVER)
    };
    // At least the widest place's width over the stepover: refuse at once
    // what cannot be cut in the limit, rather than search for it.
    let widest = (0..across.places.len())
        .filter_map(|k| across.at(k, 1.0).map(|p| p.2))
        .fold(0.0, f64::max);
    if widest / settings.stepover > MAX_PASSES as f64 {
        return Err(too_many());
    }
    let first = greedy(across, settings, 1.0)?.ok_or_else(too_many)?;
    let gaps = first.len() - 1;
    if gaps < 2 {
        return Ok(first);
    }
    // The smallest share of the allowance that still needs no more passes:
    // the gaps come out nearly equal.
    let (mut lo, mut hi) = (0.9 * (gaps - 1) as f64 / gaps as f64, 1.0);
    let mut best = first;
    for _ in 0..14 {
        let mid = 0.5 * (lo + hi);
        match greedy(across, settings, mid)? {
            Some(w) if w.len() - 1 <= gaps => {
                hi = mid;
                best = w;
            }
            _ => lo = mid,
        }
    }
    Ok(best)
}

// ---------------------------------------------------------------------------
// The tool along a pass.

/// One placed point of a pass.
#[derive(Debug, Clone, Copy)]
struct Pose {
    s: f64,
    point: CutPoint,
}

/// What became of one sample of a pass.
#[derive(Debug, Clone, Copy)]
enum Sample {
    Cut(Pose),
    /// The tool cannot reach the point without gouging the part elsewhere.
    Blocked {
        point: Point3,
        a: f64,
        s: f64,
    },
    /// The region (avoid or check set) forbids a cut here, or the face has
    /// no point here.
    Skipped,
}

/// What the tool needs that does not change along a pass.
struct Tooling<'a> {
    input: &'a StrategyInput<'a>,
    region: &'a RegionConstraint<'a>,
    part: &'a BrepSurface,
    profile: &'a RevolvedProfile,
    settings: Settings,
    sense: f64,
}

impl Tooling<'_> {
    fn axis(&self, phi: f64) -> (Degrees, UnitVec3) {
        let a = Degrees(self.sense * (90.0 - phi));
        (a, self.input.rotary.tool_axis(a))
    }

    /// The tip for the tool's point with outward normal `-n` to sit on `p`.
    fn seed(&self, p: Point3, n: Vec3, axis: UnitVec3) -> Point3 {
        let t = axis.get();
        let d = -n;
        let along = d.dot(t);
        let radial = d - t * along;
        let b = radial.length();
        let angle = along.atan2(b).min(0.0);
        let Some(q) = self.profile.point_for_normal(angle) else {
            return p;
        };
        let e = if b > 1e-12 {
            radial * (1.0 / b)
        } else {
            Vec3::ZERO
        };
        p + (-(t * q.height.0 + e * q.radius.0))
    }

    /// The tool touching `p` (normal `n`) at angle `phi`.
    fn place(&self, s: f64, p: Point3, n: Vec3, phi: f64) -> Sample {
        let (a, axis) = self.axis(phi);
        let seed = self.seed(p, n, axis);
        let line = ToolLine { origin: seed, axis };
        let Some(contact) = self.part.drop_tool(self.profile, &line) else {
            return Sample::Skipped;
        };
        let lift = (contact.tip - seed).dot(axis.get());
        if lift > self.settings.tolerance {
            return Sample::Blocked {
                point: p,
                a: a.0,
                s,
            };
        }
        if !self
            .region
            .allows(contact.contact, contact.tip, self.profile, &line)
        {
            return Sample::Skipped;
        }
        Sample::Cut(Pose {
            s,
            point: self.cut_point(contact.tip, contact.contact, a, axis),
        })
    }

    fn cut_point(
        &self,
        grown_tip: Point3,
        contact: Point3,
        a: Degrees,
        axis: UnitVec3,
    ) -> CutPoint {
        let leave = self.settings.leave;
        let tip = grown_tip + axis.get() * leave;
        let contact = if leave > 0.0 {
            cutter::cutter_contact(self.input.tool, tip, axis, contact)
        } else {
            contact
        };
        CutPoint { tip, a, contact }
    }

    /// How far the straight machine move from `a` to `b` leaves the part at
    /// its middle and quarter points, along the tool axis: positive into it.
    fn move_error(&self, a: &CutPoint, b: &CutPoint) -> f64 {
        let rotary = self.input.rotary;
        let pa = rotary.to_axis(a.tip, a.a);
        let pb = rotary.to_axis(b.tip, b.a);
        let bb = nearest_turn(b.a, a.a);
        let mut worst: f64 = 0.0;
        for f in MOVE_CHECKS {
            let at = Degrees(a.a.0 + (bb.0 - a.a.0) * f);
            let pos = crate::rotary::AxisPosition {
                along: pa.along + (pb.along - pa.along) * f,
                across: pa.across + (pb.across - pa.across) * f,
                height: pa.height + (pb.height - pa.height) * f,
            };
            let tip = rotary.to_model(pos, at);
            let axis = rotary.tool_axis(at);
            let grown = tip + axis.get() * -self.settings.leave;
            let line = ToolLine {
                origin: grown,
                axis,
            };
            if let Some(c) = self.part.drop_tool(self.profile, &line) {
                let e = (c.tip - grown).dot(axis.get());
                if e.abs() > worst.abs() {
                    worst = e;
                }
            }
        }
        worst
    }
}

/// The polar angle, degrees, of the normal's part across the rotary axis;
/// `None` along it.
fn polar(n: Vec3) -> Option<f64> {
    (n.y.abs() > 1e-12 || n.z.abs() > 1e-12).then(|| n.z.atan2(n.y).to_degrees())
}

/// The angle equal to `x` by whole turns nearest `near`.
fn near(x: f64, near: f64) -> f64 {
    x + ((near - x) / 360.0).round() * 360.0
}

/// A line across a face that a pass follows, taken by fraction `s` from 0
/// to 1: the blend between the guide and the opposite boundary at one
/// weight, or an offset of the face's outline.
trait PassLine {
    /// How many base samples the line is first taken at.
    fn m(&self) -> usize;
    /// The face's point and normal at fraction `s`, or `None` where the line
    /// leaves the face.
    fn at(&self, s: f64) -> Option<SurfacePoint>;
}

/// The pass of a field at weight `w` (decision 0525 item 2.2).
struct Blend<'a, 'f> {
    field: &'a Field<'f>,
    w: f64,
}

impl PassLine for Blend<'_, '_> {
    fn m(&self) -> usize {
        self.field.m()
    }

    fn at(&self, s: f64) -> Option<SurfacePoint> {
        self.field.at(s, self.w)
    }
}

/// One pass's samples, in guide order.
fn pass_samples(line: &dyn PassLine, tooling: &Tooling<'_>) -> Vec<Sample> {
    // The base samples: the face's points along the pass, refined where the
    // chord leaves the face by more than the tolerance.
    let m = line.m();
    let mut base: Vec<(f64, Option<SurfacePoint>)> = Vec::new();
    for j in 0..=m {
        let s = j as f64 / m as f64;
        let sp = line.at(s);
        if let (Some((s0, Some(a))), Some(b)) = (base.last().copied(), sp) {
            refine_chord(
                line,
                (s0, a),
                (s, b),
                tooling.settings.tolerance,
                0,
                &mut base,
            );
        }
        base.push((s, sp));
    }
    // Travel round the axis, for the lead, and the axis plan.
    let points: Vec<Option<Point3>> = base.iter().map(|(_, sp)| sp.map(|p| p.point)).collect();
    let mut phi_n: Vec<Option<f64>> = Vec::with_capacity(base.len());
    let mut last: Option<f64> = None;
    for (_, sp) in &base {
        let phi = sp.and_then(|p| polar(p.normal.get())).map(|x| match last {
            Some(prev) => near(x, prev),
            None => x,
        });
        if phi.is_some() {
            last = phi;
        }
        phi_n.push(phi);
    }
    // Where the normal is along the rotary axis the tool cannot face it: it
    // takes the angle of the nearest point that it can face, or, on a pass
    // with none, comes from outside the point (the drop then says whether
    // it reaches).
    let known: Vec<(usize, f64)> = phi_n
        .iter()
        .enumerate()
        .filter_map(|(i, p)| p.map(|p| (i, p)))
        .collect();
    for (i, slot) in phi_n.iter_mut().enumerate() {
        if slot.is_some() {
            continue;
        }
        let nearest = known
            .iter()
            .min_by_key(|(k, _)| k.abs_diff(i))
            .map(|(_, p)| *p);
        let outside = base
            .get(i)
            .and_then(|(_, sp)| sp.as_ref())
            .and_then(|p| polar(p.point.to_vec()));
        *slot = nearest.or(outside);
    }
    let settings = tooling.settings;
    let mut target: Vec<Option<f64>> = Vec::with_capacity(base.len());
    for (i, phi) in phi_n.iter().enumerate() {
        let Some(phi) = *phi else {
            target.push(None);
            continue;
        };
        // The direction of travel, from the face itself a hair either side.
        let s = base.get(i).map_or(0.0, |b| b.0);
        let travel = match (
            line.at((s - TANGENT_STEP).max(0.0)),
            line.at((s + TANGENT_STEP).min(1.0)),
        ) {
            (Some(a), Some(b)) => b.point - a.point,
            _ => Vec3::ZERO,
        };
        let len = travel.length();
        let r = phi.to_radians();
        let round = Vec3::new(0.0, -r.sin(), r.cos());
        let share = if len > 0.0 {
            travel.dot(round) / len
        } else {
            0.0
        };
        target.push(Some(phi + settings.lead * share));
    }
    // Smooth A within the tilt band, along the length of each stretch of
    // the pass that has points (OP-15; as simultaneous finishing).
    let phi = smooth(&base, &points, &target, settings.tilt);
    let mut out = Vec::with_capacity(base.len());
    for (i, (s, sp)) in base.iter().enumerate() {
        match (sp, phi.get(i).copied().flatten()) {
            (Some(p), Some(phi)) => out.push(tooling.place(*s, p.point, p.normal.get(), phi)),
            (Some(p), None) => out.push(Sample::Blocked {
                point: p.point,
                a: 0.0,
                s: *s,
            }),
            _ => out.push(Sample::Skipped),
        }
    }
    refine_moves(line, tooling, &base, &phi, out)
}

fn refine_chord(
    line: &dyn PassLine,
    a: (f64, SurfacePoint),
    b: (f64, SurfacePoint),
    tolerance: f64,
    depth: u32,
    out: &mut Vec<(f64, Option<SurfacePoint>)>,
) {
    if depth >= MAX_DEPTH {
        return;
    }
    let s = 0.5 * (a.0 + b.0);
    let Some(m) = line.at(s) else {
        return;
    };
    let chord = a.1.point + (b.1.point - a.1.point) * 0.5;
    if chord.distance(m.point) <= tolerance {
        return;
    }
    refine_chord(line, a, (s, m), tolerance, depth + 1, out);
    out.push((s, Some(m)));
    refine_chord(line, (s, m), b, tolerance, depth + 1, out);
}

/// The smoothed angle at each base sample.
fn smooth(
    base: &[(f64, Option<SurfacePoint>)],
    points: &[Option<Point3>],
    target: &[Option<f64>],
    tilt: f64,
) -> Vec<Option<f64>> {
    let mut out: Vec<Option<f64>> = vec![None; base.len()];
    let mut i = 0;
    while i < base.len() {
        if target.get(i).copied().flatten().is_none() {
            i += 1;
            continue;
        }
        let start = i;
        while i < base.len() && target.get(i).copied().flatten().is_some() {
            i += 1;
        }
        // The stretch `start..i`: x by length along it.
        let mut x = Vec::with_capacity(i - start);
        let mut lo = Vec::with_capacity(i - start);
        let mut hi = Vec::with_capacity(i - start);
        let mut idx = Vec::with_capacity(i - start);
        let mut along = 0.0;
        let mut last: Option<Point3> = None;
        for k in start..i {
            let (Some(p), Some(t)) = (
                points.get(k).copied().flatten(),
                target.get(k).copied().flatten(),
            ) else {
                continue;
            };
            if let Some(q) = last {
                along += p.distance(q);
            }
            if x.last().is_some_and(|l: &f64| along <= *l) {
                last = Some(p);
                continue;
            }
            last = Some(p);
            x.push(along);
            lo.push(t - tilt);
            hi.push(t + tilt);
            idx.push(k);
        }
        let (Some(first), Some(end)) = (
            idx.first().and_then(|k| target.get(*k).copied().flatten()),
            idx.last().and_then(|k| target.get(*k).copied().flatten()),
        ) else {
            continue;
        };
        if x.len() < 2 {
            for k in &idx {
                if let Some(slot) = out.get_mut(*k) {
                    *slot = target.get(*k).copied().flatten();
                }
            }
            continue;
        }
        let string = crate::simultaneous::taut_string(&x, &lo, &hi, first, end);
        for (n, k) in idx.iter().enumerate() {
            let Some(xi) = x.get(n) else { continue };
            let v = interpolate(&string, *xi);
            let (Some(l), Some(h)) = (lo.get(n), hi.get(n)) else {
                continue;
            };
            if let Some(slot) = out.get_mut(*k) {
                *slot = v.map(|v| v.clamp(*l, *h));
            }
        }
        // Samples dropped as coincident take their neighbour's angle.
        for k in start..i {
            if out.get(k).copied().flatten().is_none() {
                let v = (start..k).rev().find_map(|j| out.get(j).copied().flatten());
                if let Some(slot) = out.get_mut(k) {
                    *slot = v.or_else(|| target.get(k).copied().flatten());
                }
            }
        }
    }
    out
}

fn interpolate(vertices: &[(f64, f64)], x: f64) -> Option<f64> {
    let i = vertices.partition_point(|v| v.0 < x);
    match (
        i.checked_sub(1).and_then(|k| vertices.get(k)),
        vertices.get(i),
    ) {
        (Some(a), Some(b)) if b.0 > a.0 => Some(a.1 + (b.1 - a.1) * (x - a.0) / (b.0 - a.0)),
        (_, Some(b)) => Some(b.1),
        (Some(a), None) => Some(a.1),
        _ => None,
    }
}

/// The smoothed angle at guide fraction `s`, from the base samples.
fn phi_at(base: &[(f64, Option<SurfacePoint>)], phi: &[Option<f64>], s: f64) -> Option<f64> {
    let i = base.partition_point(|b| b.0 < s);
    let a = i
        .checked_sub(1)
        .and_then(|k| Some((base.get(k)?.0, phi.get(k).copied().flatten()?)));
    let b = base
        .get(i)
        .and_then(|x| Some((x.0, phi.get(i).copied().flatten()?)));
    match (a, b) {
        (Some(a), Some(b)) if b.0 > a.0 => {
            let bb = near(b.1, a.1);
            Some(a.1 + (bb - a.1) * (s - a.0) / (b.0 - a.0))
        }
        (Some(a), _) => Some(a.1),
        (_, Some(b)) => Some(b.1),
        _ => None,
    }
}

/// Splits each move that leaves the part by more than the tolerance at its
/// middle or quarter points, by placing the tool between its ends; a move
/// that still cuts in after [`MAX_DEPTH`] halvings is broken.
fn refine_moves(
    line: &dyn PassLine,
    tooling: &Tooling<'_>,
    base: &[(f64, Option<SurfacePoint>)],
    phi: &[Option<f64>],
    samples: Vec<Sample>,
) -> Vec<Sample> {
    let mut out: Vec<Sample> = Vec::with_capacity(samples.len());
    for sample in samples {
        if let (Some(Sample::Cut(a)), Sample::Cut(b)) = (out.last().copied(), sample) {
            split_move(line, tooling, base, phi, a, b, 0, &mut out);
        }
        out.push(sample);
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn split_move(
    line: &dyn PassLine,
    tooling: &Tooling<'_>,
    base: &[(f64, Option<SurfacePoint>)],
    phi: &[Option<f64>],
    a: Pose,
    b: Pose,
    depth: u32,
    out: &mut Vec<Sample>,
) {
    let error = tooling.move_error(&a.point, &b.point);
    if error.abs() <= tooling.settings.tolerance {
        return;
    }
    if depth >= MAX_DEPTH {
        if error > tooling.settings.tolerance {
            // Still cutting in: break the pass here rather than gouge.
            out.push(Sample::Skipped);
        }
        return;
    }
    let s = 0.5 * (a.s + b.s);
    let (Some(sp), Some(angle)) = (line.at(s), phi_at(base, phi, s)) else {
        out.push(Sample::Skipped);
        return;
    };
    let middle = tooling.place(s, sp.point, sp.normal.get(), angle);
    match middle {
        Sample::Cut(m) => {
            split_move(line, tooling, base, phi, a, m, depth + 1, out);
            out.push(middle);
            split_move(line, tooling, base, phi, m, b, depth + 1, out);
        }
        other => out.push(other),
    }
}

// ---------------------------------------------------------------------------
// The whole operation.

/// A stretch of a pass the tool cannot reach.
#[derive(Debug, Clone)]
struct Stretch {
    face: FaceId,
    pass: usize,
    s: (f64, f64),
    points: Vec<Point3>,
    a: Vec<f64>,
    reason: UnreachedReason,
    /// Its area: its length times the width it stands for across.
    area: f64,
}

/// One face's cut: its passes in guide order, each as its runs, and what
/// cannot be reached.
struct FaceCut {
    id: FaceId,
    passes: Vec<Vec<Vec<CutPoint>>>,
    /// The runs in cutting order, already directed, when Meander stays down
    /// between passes; `None` when the passes are directed and linked one
    /// by one.
    joined: Option<Vec<Vec<CutPoint>>>,
    stretches: Vec<Stretch>,
    guide_edges: Vec<EdgeId>,
    guide_length: f64,
    point_opposite: bool,
    dropped_short: usize,
    spacing: f64,
    /// Cut along the face's outline rather than from its guide.
    outline: bool,
    /// What the face note says of the joins between rings, if anything.
    ring_words: String,
    /// Its passes are already directed (rings, which never reverse).
    one_way: bool,
}

#[allow(clippy::too_many_lines)]
fn face_flow(
    selection: &Selection,
    input: &StrategyInput<'_>,
    region: &RegionConstraint<'_>,
    parameters: &Parameters,
    context: &JobContext<'_>,
) -> Result<StrategyOutput> {
    let model = selection.model.as_ref();
    let settings = Settings::read(parameters, input.tool)?;
    let faces: BTreeSet<FaceId> = selection.faces.iter().copied().collect();
    if faces.is_empty() {
        return Err(PathError::refused(
            crate::topic::FACE_FLOW,
            "face-flow finishing needs at least one face to cut: select the faces".to_owned(),
        ));
    }
    if let Some(bad) = faces.iter().find(|f| f.0 as usize >= model.faces.len()) {
        return Err(PathError::refused(
            crate::topic::FACE_FLOW,
            format!("the model has no face {}", bad.0),
        ));
    }
    let guide: BTreeSet<EdgeId> = selection.guide.iter().copied().collect();
    if let Some(bad) = guide.iter().find(|e| e.0 as usize >= model.edges.len()) {
        return Err(PathError::refused(
            crate::topic::FACE_FLOW,
            format!("the model has no edge {} to use as the guide", bad.0),
        ));
    }
    if let Some(bad) = guide.iter().find(|e| {
        !faces.iter().any(|f| {
            model
                .faces
                .get(f.0 as usize)
                .is_some_and(|face| face.edges.contains(e))
        })
    }) {
        return Err(PathError::refused(
            crate::topic::FACE_FLOW,
            format!(
                "guide edge {} is not on the boundary of the selected faces: choose edges of the \
             faces to cut",
                bad.0
            ),
        ));
    }
    if !input.tool.is_convex() {
        return Err(PathError::refused(
            crate::topic::FACE_FLOW,
            "face-flow finishing needs a tool whose profile is convex".to_owned(),
        ));
    }
    let part = BrepSurface::new(model)?.with_exact_drops(true);
    let profile = cutter::grow_profile(input.tool, settings.leave)?;
    let tooling = Tooling {
        input,
        region,
        part: &part,
        profile: &profile,
        settings,
        sense: input.rotary.sense.sign(),
    };
    // The faces' evaluators, in ID order.
    let exact: Vec<(FaceId, ExactFace)> = faces
        .iter()
        .map(|id| {
            let face = model.faces.get(id.0 as usize).ok_or_else(|| {
                PathError::refused(
                    crate::topic::FACE_FLOW,
                    format!("the model has no face {}", id.0),
                )
            })?;
            Ok((*id, ExactFace::new(face)?))
        })
        .collect::<Result<_>>()?;
    let mut fields = Vec::with_capacity(exact.len());
    let mut closures = Vec::with_capacity(exact.len());
    for (id, face) in &exact {
        context.check_cancelled()?;
        let brep = model.faces.get(id.0 as usize).ok_or_else(|| {
            PathError::refused(
                crate::topic::FACE_FLOW,
                format!("the model has no face {}", id.0),
            )
        })?;
        let closed = closure(model, brep, face);
        let b = boundaries(model, brep, &guide, &closed, face)?;
        fields.push(field(model, *id, face, b, &closed)?);
        closures.push(closed);
    }
    let order = face_order(model, &fields);
    let total = fields.len();
    let mut cuts: Vec<FaceCut> = Vec::with_capacity(total);
    for (done, index) in order.iter().enumerate() {
        context.check_cancelled()?;
        context.report("face-flow finishing", done as u64, Some(total as u64));
        let Some(f) = fields.get(*index) else {
            continue;
        };
        let (Some(closed), Some(brep)) = (closures.get(*index), model.faces.get(f.id.0 as usize))
        else {
            continue;
        };
        let along_outline = match settings.pattern {
            Pattern::AlongGuide => false,
            Pattern::Outline => true,
            Pattern::Automatic => notched(f, closed),
        };
        cuts.push(if along_outline {
            outline::cut_outline(
                model, brep, f.id, f.face, closed, &tooling, &profile, context,
            )?
        } else {
            cut_face(f, &tooling, &profile, context)?
        });
    }
    context.report("face-flow finishing", total as u64, Some(total as u64));
    assemble(cuts, &tooling, input, context)
}

/// Whether the passes from a face's guide would leave it in places (its
/// outline has notches or fingers the blend between its guide and the
/// opposite boundary runs across), so Automatic follows its outline instead
/// (the owner's answer of 23 Nov 2026 to Q1). A face that closes round on
/// itself keeps its rings.
fn notched(f: &Field<'_>, closed: &Closure) -> bool {
    if closed.period.iter().any(Option::is_some) {
        return false;
    }
    const N: usize = 40;
    let mut off = 0;
    let mut all = 0;
    for i in 1..N {
        for j in 1..N {
            let (s, w) = (i as f64 / N as f64, j as f64 / N as f64);
            all += 1;
            if f.at(s, w).is_none() {
                off += 1;
            }
        }
    }
    off * 50 > all
}

/// The order the faces are cut in (decision 0525 D2): the face with the
/// longest guide first (the lowest ID on a tie), then a face sharing an
/// edge with the face just cut, then with any face cut, lowest ID first.
fn face_order(model: &BrepModel, fields: &[Field<'_>]) -> Vec<usize> {
    let edges_of = |f: &Field<'_>| -> BTreeSet<EdgeId> {
        model
            .faces
            .get(f.id.0 as usize)
            .map(|face| face.edges.iter().copied().collect())
            .unwrap_or_default()
    };
    let edge_sets: Vec<BTreeSet<EdgeId>> = fields.iter().map(edges_of).collect();
    let shares = |i: usize, j: usize| match (edge_sets.get(i), edge_sets.get(j)) {
        (Some(a), Some(b)) => a.intersection(b).next().is_some(),
        _ => false,
    };
    let start = |left: &BTreeSet<usize>| -> Option<usize> {
        let mut best: Option<(usize, f64)> = None;
        for &i in left {
            let len = fields.get(i).map_or(0.0, |f| f.boundaries.guide.length());
            if best.is_none_or(|(_, l)| len > l * (1.0 + 1e-6)) {
                best = Some((i, len));
            }
        }
        best.map(|b| b.0)
    };
    let mut left: BTreeSet<usize> = (0..fields.len()).collect();
    let mut order: Vec<usize> = Vec::with_capacity(fields.len());
    while !left.is_empty() {
        let next = order
            .last()
            .and_then(|&last| left.iter().copied().find(|&i| shares(last, i)))
            .or_else(|| {
                left.iter()
                    .copied()
                    .find(|&i| order.iter().any(|&o| shares(o, i)))
            })
            .or_else(|| start(&left));
        let Some(next) = next else { break };
        left.remove(&next);
        order.push(next);
    }
    order
}

fn cut_face(
    f: &Field<'_>,
    tooling: &Tooling<'_>,
    profile: &RevolvedProfile,
    context: &JobContext<'_>,
) -> Result<FaceCut> {
    let across = Across::new(f);
    let weights = weights(&across, &tooling.settings, f.id)?;
    let diameter = 2.0 * profile.cutting_radius();
    let mut passes: Vec<Vec<Vec<CutPoint>>> = Vec::with_capacity(weights.len());
    // Each kept pass's weight and its runs' spans along it.
    let mut kept: Vec<(f64, Vec<(f64, f64)>)> = Vec::with_capacity(weights.len());
    let mut stretches: Vec<Stretch> = Vec::new();
    let mut dropped_short = 0;
    let point_opposite = matches!(f.boundaries.opposite, Opposite::Point(_));
    let spacing = weights
        .windows(2)
        .filter_map(|w| match w {
            [a, b] => Some(
                (0..across.places.len())
                    .filter_map(|k| Some((across.at(k, *b)?.2 - across.at(k, *a)?.2).abs()))
                    .fold(0.0, f64::max),
            ),
            _ => None,
        })
        .fold(0.0, f64::max);
    // On a face that closes round on itself, with Stay down between passes,
    // every ring runs one way and is joined to the next, unless Meander
    // steps along the seam with each ring back the other way (the owner's
    // decision of 23 Nov 2026; decision 0545's two shapes).
    // Rings never reverse: every ring runs the same way round, for every
    // direction and either join (the owner's correction of 23 Nov 2026).
    let settings = tooling.settings;
    let closed_face = f.wrap.iter().any(Option::is_some);
    let rings_joined = closed_face && settings.stay_down;
    let ring_dir = if closed_face {
        ring_direction(f, &weights, settings.direction)
    } else {
        1.0
    };
    let turn = if rings_joined {
        settings.ring_blend.unwrap_or(0.0) / 360.0
    } else {
        0.0
    };
    let mut kept_k: Vec<usize> = Vec::with_capacity(weights.len());
    for (k, &w) in weights.iter().enumerate() {
        context.check_cancelled()?;
        let samples = if closed_face {
            // Ring k starts where the blend before it ends, φ further round.
            let line = Ring {
                field: f,
                w,
                s0: round01(ring_dir * kept.len() as f64 * turn),
                dir: ring_dir,
            };
            pass_samples(&line, tooling)
        } else {
            pass_samples(&Blend { field: f, w }, tooling)
        };
        // A pass towards a point opposite shorter than the tool is left out:
        // on a face that closes round, a ring by its length round (its ends
        // meet); otherwise by the distance between its ends.
        if point_opposite && w > 0.0 {
            let short = if f.wrap.iter().any(Option::is_some) {
                let ring: Vec<Point3> = (0..=RING_SAMPLES)
                    .filter_map(|k| f.at(k as f64 / RING_SAMPLES as f64, w))
                    .map(|p| p.point)
                    .collect();
                Curve::new(ring).length() < diameter
            } else {
                let pts: Vec<Point3> = [0.0, 1.0]
                    .iter()
                    .filter_map(|s| f.at(*s, w))
                    .map(|p| p.point)
                    .collect();
                matches!(pts.as_slice(), [a, b] if a.distance(*b) < diameter)
            };
            if short {
                dropped_short += 1;
                continue;
            }
        }
        // The width each pass stands for across the face.
        let gap = |i: Option<usize>| i.and_then(|i| weights.get(i)).copied();
        let below = gap(k.checked_sub(1)).map_or(0.0, |x| w - x);
        let above = gap(Some(k + 1)).map_or(0.0, |x| x - w);
        let share = 0.5 * (below + above);
        let (runs, spans) = split_runs(
            samples,
            f.id,
            k,
            share,
            &|s| across.width(s),
            diameter,
            &mut stretches,
        );
        kept.push((w, spans));
        kept_k.push(k);
        passes.push(runs);
    }
    // With Meander and Stay down between passes, the runs are joined across
    // the passes' ends by steps along the face, where clear (the owner's
    // answer of 23 Nov 2026 to Q2).
    let joined = if rings_joined {
        Some(join_rings(
            f,
            tooling,
            &passes,
            &kept,
            &kept_k,
            ring_dir,
            settings.ring_blend,
        ))
    } else {
        (!closed_face && settings.direction == Direction::Meander && settings.stay_down)
            .then(|| join_meander(f, tooling, &passes, &kept))
    };
    // The ring note's words for the joins (decision 0545 item 6's form):
    // the blend and the steepest angle it makes with the ring, at the
    // largest radius cut.
    let mut ring_words = String::new();
    if rings_joined && let Some(phi) = settings.ring_blend {
        let radius = passes
            .iter()
            .flatten()
            .flatten()
            .map(|c| c.contact.y.hypot(c.contact.z))
            .fold(0.0, f64::max);
        ring_words.push_str(&crate::helical::blend_words(Some(phi), spacing, radius));
    }
    if closed_face && settings.direction == Direction::Meander {
        ring_words.push_str("; with Meander every ring still runs the same way round");
    }
    Ok(FaceCut {
        id: f.id,
        passes,
        joined,
        stretches,
        guide_edges: f.boundaries.guide_edges.clone(),
        guide_length: f.boundaries.guide.length(),
        point_opposite,
        dropped_short,
        spacing,
        outline: false,
        ring_words,
        one_way: closed_face,
    })
}

/// A pass's samples split into runs (stretches of at least two points the
/// tool cuts without lifting), with where each starts and ends along the
/// pass, and its unreached stretches added to `stretches`: too narrow where
/// the face there (`width` at the stretch's middle) is narrower than the
/// tool, blocked otherwise, each standing for `share` of the face across.
fn split_runs(
    samples: Vec<Sample>,
    face: FaceId,
    k: usize,
    share: f64,
    width: &dyn Fn(f64) -> f64,
    diameter: f64,
    stretches: &mut Vec<Stretch>,
) -> (Vec<Vec<CutPoint>>, Vec<(f64, f64)>) {
    let mut runs: Vec<Vec<CutPoint>> = Vec::new();
    let mut run: Vec<CutPoint> = Vec::new();
    // Where along the pass each run starts and ends.
    let mut spans: Vec<(f64, f64)> = Vec::new();
    let mut run_s: (f64, f64) = (0.0, 0.0);
    let mut blocked: Vec<(Point3, f64, f64)> = Vec::new();
    let flush_blocked = |blocked: &mut Vec<(Point3, f64, f64)>, stretches: &mut Vec<Stretch>| {
        if blocked.is_empty() {
            return;
        }
        let pts: Vec<Point3> = blocked.iter().map(|b| b.0).collect();
        let length = Curve::new(pts.clone()).length();
        let (s0, s1) = (
            blocked.first().map_or(0.0, |b| b.2),
            blocked.last().map_or(0.0, |b| b.2),
        );
        let across = width(0.5 * (s0 + s1));
        let reason = if across < diameter {
            UnreachedReason::TooNarrow
        } else {
            UnreachedReason::Blocked
        };
        stretches.push(Stretch {
            face,
            pass: k,
            s: (s0, s1),
            points: pts,
            a: blocked.iter().map(|b| b.1).collect(),
            reason,
            area: length.max(SAMPLE_SPACING) * share * across,
        });
        blocked.clear();
    };
    for sample in samples {
        match sample {
            Sample::Cut(pose) => {
                flush_blocked(&mut blocked, stretches);
                if run.is_empty() {
                    run_s.0 = pose.s;
                }
                run_s.1 = pose.s;
                run.push(pose.point);
            }
            Sample::Blocked { point, a, s } => {
                if run.len() >= 2 {
                    runs.push(std::mem::take(&mut run));
                    spans.push(run_s);
                } else {
                    run.clear();
                }
                blocked.push((point, a, s));
            }
            Sample::Skipped => {
                flush_blocked(&mut blocked, stretches);
                if run.len() >= 2 {
                    runs.push(std::mem::take(&mut run));
                    spans.push(run_s);
                } else {
                    run.clear();
                }
            }
        }
    }
    flush_blocked(&mut blocked, stretches);
    if run.len() >= 2 {
        runs.push(run);
        spans.push(run_s);
    }
    (runs, spans)
}

/// The runs of a face's passes in meander order, each pass after the first
/// reversed in turn, with the end of one pass joined to the start of the
/// next by a step along the face where [`step`] finds it clear. A run that
/// does not reach the end of its pass, or whose next pass does not start at
/// that end, is not joined: the tool lifts there as before.
fn join_meander(
    f: &Field<'_>,
    tooling: &Tooling<'_>,
    passes: &[Vec<Vec<CutPoint>>],
    kept: &[(f64, Vec<(f64, f64)>)],
) -> Vec<Vec<CutPoint>> {
    let mut out: Vec<Vec<CutPoint>> = Vec::new();
    let mut forward = true;
    // The weight and guide fraction where the last pass's last run ended,
    // when that is an end of the pass.
    let mut open: Option<(f64, f64)> = None;
    for (runs, (w, spans)) in passes.iter().zip(kept) {
        if runs.is_empty() {
            open = None;
            continue;
        }
        let mut pass: Vec<(Vec<CutPoint>, (f64, f64))> =
            runs.iter().cloned().zip(spans.iter().copied()).collect();
        if !forward {
            pass.reverse();
            for (points, span) in &mut pass {
                points.reverse();
                *span = (span.1, span.0);
            }
        }
        let last = pass.len().saturating_sub(1);
        for (i, (points, (s0, s1))) in pass.into_iter().enumerate() {
            let steps = match (i, open, out.last().and_then(|r| r.last()), points.first()) {
                (0, Some((w0, s)), Some(a), Some(b)) if (s0 - s).abs() < 1e-12 => {
                    step(f, tooling, (w0, *w), s, a, b)
                }
                _ => None,
            };
            match (steps, out.last_mut()) {
                (Some(steps), Some(run)) => {
                    let end_a = run
                        .last()
                        .zip(points.first())
                        .map_or(Degrees(0.0), |(a, b)| nearest_turn(b.a, a.a));
                    append(run, steps, &points, end_a);
                }
                _ => out.push(points),
            }
            if i == last {
                open = (s1 <= 0.0 || s1 >= 1.0).then_some((*w, s1));
            }
        }
        forward = !forward;
    }
    out
}

/// The pass end of a field at guide fraction `s`, from weight `w0` to `w1`:
/// the line a step between two meander passes follows.
struct EndStep<'a, 'f> {
    field: &'a Field<'f>,
    s: f64,
    w0: f64,
    w1: f64,
}

impl PassLine for EndStep<'_, '_> {
    fn m(&self) -> usize {
        1
    }

    fn at(&self, t: f64) -> Option<SurfacePoint> {
        self.field.at(self.s, self.w0 + (self.w1 - self.w0) * t)
    }
}

/// The points of a step along the face at guide fraction `s` (an end of the
/// passes) from weight `ws.0`, where the run ends at `a`, to `ws.1`, where
/// the next starts at `b`; `None` where the tool should lift instead.
fn step(
    f: &Field<'_>,
    tooling: &Tooling<'_>,
    ws: (f64, f64),
    s: f64,
    a: &CutPoint,
    b: &CutPoint,
) -> Option<Vec<CutPoint>> {
    let line = EndStep {
        field: f,
        s,
        w0: ws.0,
        w1: ws.1,
    };
    step_on(
        &line,
        tooling,
        s,
        a,
        b,
        2.0 * tooling.settings.stepover,
        nearest_turn(b.a, a.a),
    )
}

/// Adds `steps` and then the run `next` to `run`, `next`'s A moved by whole
/// turns so it goes on from `end_a`, where the steps end.
fn append(run: &mut Vec<CutPoint>, steps: Vec<CutPoint>, next: &[CutPoint], end_a: Degrees) {
    let shift = next.first().map_or(0.0, |b| end_a.0 - b.a.0);
    run.extend(steps);
    run.extend(next.iter().map(|c| CutPoint {
        a: Degrees(c.a.0 + shift),
        ..*c
    }));
}

/// A fraction of the way round brought into 0 to 1.
fn round01(s: f64) -> f64 {
    let x = s.rem_euclid(1.0);
    if x >= 1.0 { 0.0 } else { x }
}

/// A ring round a face that closes round on itself, at weight `w`, from
/// guide fraction `s0` once round the way `dir` (+1 with the guide, -1
/// against it).
struct Ring<'a, 'f> {
    field: &'a Field<'f>,
    w: f64,
    s0: f64,
    dir: f64,
}

impl PassLine for Ring<'_, '_> {
    fn m(&self) -> usize {
        self.field.m()
    }

    fn at(&self, t: f64) -> Option<SurfacePoint> {
        // Within 0 to 1 as it is, so a ring from the seam ends on it.
        let s = self.s0 + self.dir * t;
        let s = if (0.0..=1.0).contains(&s) {
            s
        } else {
            round01(s)
        };
        self.field.at(s, self.w)
    }
}

/// A blend from a ring's end at guide fraction `s0` and weight `w0` to the
/// next ring's start, `ds` further round, at `w1`: round and across both
/// linear (decision 0545 item 2's shape on the face).
struct RingBlend<'a, 'f> {
    field: &'a Field<'f>,
    s0: f64,
    ds: f64,
    w0: f64,
    w1: f64,
}

impl PassLine for RingBlend<'_, '_> {
    fn m(&self) -> usize {
        1
    }

    fn at(&self, t: f64) -> Option<SurfacePoint> {
        self.field.at(
            round01(self.s0 + self.ds * t),
            self.w0 + (self.w1 - self.w0) * t,
        )
    }
}

/// The way the rings of a face that closes round on itself run, when every
/// ring runs the same way (blended steps, or Climb and Conventional): +1
/// with the guide, -1 against it. Meander takes the guide's way; Climb and
/// Conventional the way that puts the uncut material, towards the next
/// ring, on the side they ask for (decision 0213, as `material_on_right`).
fn ring_direction(f: &Field<'_>, weights: &[f64], direction: Direction) -> f64 {
    if direction == Direction::Meander {
        return 1.0;
    }
    let (Some(&w0), Some(&w1)) = (weights.first(), weights.get(1)) else {
        return 1.0;
    };
    let s = 0.25;
    let (Some(p), Some(q), Some(next)) = (f.at(s, w0), f.at(s + 1e-3, w0), f.at(s, w1)) else {
        return 1.0;
    };
    let right = (q.point - p.point).cross(p.normal.get());
    let side = right.dot(next.point - p.point);
    if (side > 0.0) == (direction == Direction::Climb) {
        1.0
    } else {
        -1.0
    }
}

/// The runs of a face cut in rings, every ring the same way `dir`, joined
/// from each ring's end to the next ring's start: by a step along the seam
/// with A held (sharp), or by a blend over `blend` degrees (decision 0545's
/// two shapes; the owner's decision of 23 Nov 2026). A ring's end joins the
/// next ring's start only where the ring was cut to its end and the next
/// from its start, and no ring between was left out; elsewhere the tool
/// lifts.
#[allow(clippy::too_many_arguments)]
fn join_rings(
    f: &Field<'_>,
    tooling: &Tooling<'_>,
    passes: &[Vec<Vec<CutPoint>>],
    kept: &[(f64, Vec<(f64, f64)>)],
    kept_k: &[usize],
    dir: f64,
    blend: Option<f64>,
) -> Vec<Vec<CutPoint>> {
    let turn = blend.unwrap_or(0.0) / 360.0;
    let mut out: Vec<Vec<CutPoint>> = Vec::new();
    // The ring the last run closed, when it ran to its end: its index among
    // the kept rings, its weight and its place among all the passes.
    let mut open: Option<(usize, f64, usize)> = None;
    for (idx, ((runs, (w, spans)), k)) in passes.iter().zip(kept).zip(kept_k).enumerate() {
        if runs.is_empty() {
            open = None;
            continue;
        }
        let last = runs.len() - 1;
        for (i, (points, (t0, t1))) in runs.iter().zip(spans).enumerate() {
            let joined = match (i, open, out.last().and_then(|r| r.last()), points.first()) {
                (0, Some((i0, w0, k0)), Some(a), Some(b)) if *t0 <= 1e-12 && *k == k0 + 1 => {
                    let s_end = round01(dir * i0 as f64 * turn);
                    match blend {
                        None => {
                            let end_a = nearest_turn(b.a, a.a);
                            let line = EndStep {
                                field: f,
                                s: s_end,
                                w0,
                                w1: *w,
                            };
                            step_on(
                                &line,
                                tooling,
                                s_end,
                                a,
                                b,
                                2.0 * tooling.settings.stepover,
                                end_a,
                            )
                            .map(|steps| (steps, end_a))
                        }
                        Some(phi) => {
                            let line = RingBlend {
                                field: f,
                                s0: s_end,
                                ds: dir * turn,
                                w0,
                                w1: *w,
                            };
                            // A turns on the way the ring turned, by about φ.
                            let travel = runs_turn(passes.get(idx.wrapping_sub(1)));
                            let end_a = nearest_turn(b.a, Degrees(a.a.0 + travel * phi));
                            let reach =
                                1.25 * planned_length(&line) + 2.0 * tooling.settings.tolerance;
                            step_on(&line, tooling, s_end, a, b, reach, end_a)
                                .map(|steps| (steps, end_a))
                        }
                    }
                }
                _ => None,
            };
            match (joined, out.last_mut()) {
                (Some((steps, end_a)), Some(run)) => append(run, steps, points, end_a),
                _ => out.push(points.clone()),
            }
            if i == last {
                open = (*t1 >= 1.0 - 1e-12).then_some((idx, *w, *k));
            }
        }
    }
    out
}

/// The way A turned along a ring's runs: +1 up, -1 down, 0 where it did
/// not turn.
fn runs_turn(runs: Option<&Vec<Vec<CutPoint>>>) -> f64 {
    let Some(runs) = runs else { return 0.0 };
    let (Some(a), Some(b)) = (
        runs.first().and_then(|r| r.first()),
        runs.last().and_then(|r| r.last()),
    ) else {
        return 0.0;
    };
    let d = b.a.0 - a.a.0;
    if d.abs() < 1e-9 { 0.0 } else { d.signum() }
}

/// The length on the part of a line, from 32 of its points.
fn planned_length(line: &dyn PassLine) -> f64 {
    let points: Vec<Point3> = (0..=32)
        .filter_map(|k| line.at(f64::from(k) / 32.0))
        .map(|p| p.point)
        .collect();
    Curve::new(points).length()
}

/// The points of a step from `a` to `b` along `line` (taken from 0 at `a`
/// to 1 at `b`), A going linearly from `a`'s to `end_a` (`b`'s, whole turns
/// added so the step turns the way it should); `None` where the tool should
/// lift instead: the step is longer than `reach`, leaves the face, needs the tool to tilt
/// beyond Maximum tilt, is lifted by the part (a wall in the way), or a move
/// of it would cut into the part. A goes linearly from `a`'s to `b`'s; each
/// point is placed by the same drop on the whole part as the passes, and
/// every move is checked at its quarter and middle points (decisions 0115
/// and 0133). `label` is the fraction recorded with the points.
fn step_on(
    line: &dyn PassLine,
    tooling: &Tooling<'_>,
    label: f64,
    a: &CutPoint,
    b: &CutPoint,
    reach: f64,
    end_a: Degrees,
) -> Option<Vec<CutPoint>> {
    let settings = tooling.settings;
    let span = a.contact.distance(b.contact);
    if span > reach {
        return None;
    }
    let n = ((span / SAMPLE_SPACING).ceil() as usize).max(1);
    let mut out: Vec<CutPoint> = Vec::with_capacity(n);
    let mut prev = *a;
    let mut length = 0.0;
    for i in 1..n {
        let t = i as f64 / n as f64;
        let sp = line.at(t)?;
        let deg = a.a.0 + (end_a.0 - a.a.0) * t;
        let phi = 90.0 - deg / tooling.sense;
        let normal = sp.normal.get();
        let target = polar(normal)?;
        if (near(target, phi) - phi).abs() > settings.tilt + settings.lead.abs() + 1e-9 {
            return None;
        }
        let Sample::Cut(pose) = tooling.place(label, sp.point, normal, phi) else {
            return None;
        };
        let point = pose.point;
        if tooling.move_error(&prev, &point) > settings.tolerance {
            return None;
        }
        length += prev.contact.distance(point.contact);
        out.push(point);
        prev = point;
    }
    if tooling.move_error(&prev, b) > settings.tolerance {
        return None;
    }
    length += prev.contact.distance(b.contact);
    (length <= reach + settings.tolerance).then_some(out)
}

/// Whether the uncut material, towards `next`, is on the tool's right
/// looking along `run`'s feed from the spindle.
fn material_on_right(run: &[CutPoint], next: Vec3, rotary: &RotaryMapping) -> Option<bool> {
    let i = run.len() / 2;
    let (a, b) = (run.get(i.checked_sub(1)?)?, run.get(i)?);
    let feed = b.contact - a.contact;
    let right = feed.cross(rotary.tool_axis(b.a).get());
    let side = next.dot(right);
    (side.abs() > 1e-12).then_some(side > 0.0)
}

fn centre(runs: &[Vec<CutPoint>]) -> Option<Point3> {
    let pts: Vec<Point3> = runs.iter().flatten().map(|c| c.contact).collect();
    let n = pts.len();
    (n > 0).then(|| {
        let s = pts.iter().fold(Vec3::ZERO, |s, p| s + p.to_vec());
        Point3::ORIGIN + s * (1.0 / n as f64)
    })
}

fn reverse(runs: &mut [Vec<CutPoint>]) {
    runs.reverse();
    for r in runs.iter_mut() {
        r.reverse();
    }
}

fn assemble(
    cuts: Vec<FaceCut>,
    tooling: &Tooling<'_>,
    input: &StrategyInput<'_>,
    context: &JobContext<'_>,
) -> Result<StrategyOutput> {
    let settings = tooling.settings;
    let mut out_passes: Vec<Pass> = Vec::new();
    let mut notes: Vec<Notice> = Vec::new();
    let mut stretches: Vec<Stretch> = Vec::new();
    let mut face_notes: Vec<String> = Vec::new();
    // Passes that cut something, one per line across a face however many
    // pieces its unreached stretches split it into (issue #631).
    let mut cutting = 0;
    for mut cut in cuts {
        let geometric = cut.passes.len();
        let reaching = cut.passes.iter().filter(|runs| !runs.is_empty()).count();
        cutting += reaching;
        // Directions (decision 0525 D1), unless Meander already joined the
        // runs in their cutting order.
        let centres: Vec<Option<Point3>> = cut.passes.iter().map(|p| centre(p)).collect();
        let mut forward = true;
        let directed = if cut.joined.is_some() || cut.one_way {
            0
        } else {
            cut.passes.len()
        };
        for k in 0..directed {
            let here = centres.get(k).copied().flatten();
            let towards = centres
                .get(k + 1..)
                .and_then(|rest| rest.iter().find_map(|c| *c))
                .zip(here)
                .map(|(n, h)| n - h)
                .or_else(|| {
                    let before = centres
                        .get(..k)
                        .and_then(|b| b.iter().rev().find_map(|c| *c));
                    before.zip(here).map(|(b, h)| h - b)
                });
            let Some(runs) = cut.passes.get_mut(k) else {
                continue;
            };
            if runs.is_empty() {
                continue;
            }
            match settings.direction {
                Direction::Meander => {
                    if !forward {
                        reverse(runs);
                    }
                    forward = !forward;
                }
                Direction::Climb | Direction::Conventional => {
                    let want = settings.direction == Direction::Climb;
                    let longest = runs
                        .iter()
                        .max_by_key(|r| r.len())
                        .cloned()
                        .unwrap_or_default();
                    if let Some(next) = towards
                        && let Some(right) = material_on_right(&longest, next, &input.rotary)
                        && right != want
                    {
                        reverse(runs);
                    }
                }
            }
        }
        if cut.outline {
            face_notes.push(format!(
                "face {} follows its outline, edges {} ({:.1} mm long), inwards in {} up to {:.3} \
                 mm apart along the face",
                cut.id.0,
                list(cut.guide_edges.iter().map(|e| e.0)),
                cut.guide_length,
                crate::counted(geometric, "pass", "passes"),
                cut.spacing
            ));
            let runs = cut.joined.take().unwrap_or_default();
            for points in runs {
                out_passes.push(Pass {
                    low_link: !out_passes.is_empty(),
                    points,
                    ..Pass::default()
                });
            }
            stretches.append(&mut cut.stretches);
            continue;
        }
        let edges = list(cut.guide_edges.iter().map(|e| e.0));
        let plural = if cut.guide_edges.len() == 1 { "" } else { "s" };
        let towards = if cut.point_opposite {
            "the opposite point"
        } else {
            "the opposite side"
        };
        let mut note = format!(
            "face {} is cut from its guide, edge{plural} {edges} ({:.1} mm long), towards {towards} \
             in {} up to {:.3} mm apart along the face",
            cut.id.0,
            cut.guide_length,
            crate::counted(geometric, "pass", "passes"),
            cut.spacing
        );
        if reaching < geometric {
            note.push_str(&format!(
                "; {reaching} of them cut and the rest are left out: along them the tool \
                 reaches no part of the face it may cut"
            ));
        }
        note.push_str(&cut.ring_words);
        if cut.dropped_short > 0 {
            note.push_str(&format!(
                "; {} nearest the point {} left out, being shorter than the tool's \
                 diameter",
                crate::counted(cut.dropped_short, "pass", "passes"),
                if cut.dropped_short == 1 { "is" } else { "are" }
            ));
        }
        face_notes.push(note);
        let runs: Vec<Vec<CutPoint>> = match cut.joined.take() {
            Some(joined) => joined,
            None => cut.passes.into_iter().flatten().collect(),
        };
        for points in runs {
            out_passes.push(Pass {
                low_link: !out_passes.is_empty(),
                points,
                ..Pass::default()
            });
        }
        stretches.append(&mut cut.stretches);
    }
    let mut unreached = areas(&stretches, context)?;
    // Shortest cut, applied last, to whole runs (the owner's answer of 23
    // Nov 2026 to Q3): a run shorter than it is left out and reported as an
    // area left uncut; nothing inside a longer run is removed.
    let mut short: Vec<Unreached> = Vec::new();
    if settings.shortest > 0.0 {
        out_passes.retain(|pass| {
            let length: f64 = pass
                .points
                .windows(2)
                .map(|w| w[0].contact.distance(w[1].contact))
                .sum();
            if length < settings.shortest {
                short.push(short_area(pass, length, settings.stepover));
                false
            } else {
                true
            }
        });
        for (i, pass) in out_passes.iter_mut().enumerate() {
            pass.low_link = i > 0;
        }
    }
    let diameter = 2.0 * input.tool.cutting_radius();
    if out_passes.is_empty() {
        let why = if unreached.is_empty() {
            "nothing on them is left to cut after the avoid and check sets".to_owned()
        } else {
            unreached_list(&unreached, diameter)
        };
        return Err(PathError::refused(
            crate::topic::FACE_FLOW,
            format!("face-flow finishing cannot reach any of the selected faces: {why}"),
        ));
    }
    notes.push(
        Notice::information(format!(
            "cuts {} on {}",
            crate::counted(cutting, "pass", "passes"),
            crate::counted(face_notes.len(), "face", "faces")
        ))
        .about(topic::FACE_FLOW),
    );
    notes.extend(
        face_notes
            .into_iter()
            .map(|note| Notice::information(note).about(topic::FACE_FLOW)),
    );
    let mut warnings = Vec::new();
    if !unreached.is_empty() {
        let plural = if unreached.len() == 1 { "" } else { "s" };
        warnings.push(Notice {
            level: Level::Warning,
            topic: Some(topic::UNREACHED_AREAS.to_owned()),
            segment: None,
            text: format!(
                "face-flow finishing cannot reach {} area{plural} of the selected faces, which \
                 stay uncut: {}; check them in the simulation before running the program",
                unreached.len(),
                unreached_list(&unreached, diameter)
            ),
            setting: None,
            acknowledgement: None,
        });
    }
    if !short.is_empty() {
        warnings.push(
            Notice {
                level: Level::Warning,
                topic: Some(topic::UNREACHED_AREAS.to_owned()),
                segment: None,
                text: format!(
                    "face-flow finishing leaves out {} shorter than the Shortest cut ({:.3} \
                     mm), which {} uncut: {}; lower the Shortest cut to keep {} (0 keeps every \
                     cut), and check {} in the simulation before running the program",
                    crate::counted(short.len(), "cut", "cuts"),
                    settings.shortest,
                    if short.len() == 1 { "stays" } else { "stay" },
                    unreached_list(&short, diameter),
                    if short.len() == 1 { "it" } else { "them" },
                    if short.len() == 1 { "it" } else { "them" },
                ),
                setting: None,
                acknowledgement: None,
            }
            .with_setting(parameter::SHORTEST_CUT),
        );
        unreached.extend(short);
    }
    Ok(StrategyOutput {
        passes: out_passes,
        notices: crate::strategy::notices(warnings, notes),
        unreached,
        shank_breaks: Vec::new(),
    })
}

/// The unreachable stretches joined into areas: stretches of neighbouring
/// passes of one face whose spans along the guide overlap, of one reason.
fn areas(stretches: &[Stretch], context: &JobContext<'_>) -> Result<Vec<Unreached>> {
    // Union-find over the stretches.
    let n = stretches.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while let Some(&p) = parent.get(i) {
            if p == i {
                break;
            }
            let gp = parent.get(p).copied().unwrap_or(p);
            if let Some(slot) = parent.get_mut(i) {
                *slot = gp;
            }
            i = p;
        }
        i
    }
    // Only stretches of one face on neighbouring passes can join: compare
    // each pass's stretches with the next pass's, not every pair.
    let mut by_pass: BTreeMap<(FaceId, usize), Vec<usize>> = BTreeMap::new();
    for (i, s) in stretches.iter().enumerate() {
        by_pass.entry((s.face, s.pass)).or_default().push(i);
    }
    for ((face, pass), here) in &by_pass {
        context.check_cancelled()?;
        let Some(next) = by_pass.get(&(*face, pass + 1)) else {
            continue;
        };
        for &i in here {
            for &j in next {
                let (Some(a), Some(b)) = (stretches.get(i), stretches.get(j)) else {
                    continue;
                };
                let overlap = a.s.0 <= b.s.1 + 1e-9 && b.s.0 <= a.s.1 + 1e-9;
                if overlap && a.reason == b.reason {
                    let (ri, rj) = (root(&mut parent, i), root(&mut parent, j));
                    if ri != rj
                        && let Some(slot) = parent.get_mut(ri.max(rj))
                    {
                        *slot = ri.min(rj);
                    }
                }
            }
        }
    }
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..n {
        let r = root(&mut parent, i);
        groups.entry(r).or_default().push(i);
    }
    let mut out: Vec<Unreached> = groups
        .values()
        .filter_map(|members| {
            let parts: Vec<&Stretch> = members.iter().filter_map(|i| stretches.get(*i)).collect();
            let first = parts.first()?;
            let points: Vec<Point3> = parts
                .iter()
                .flat_map(|s| s.points.iter().copied())
                .collect();
            let a: Vec<f64> = parts.iter().flat_map(|s| s.a.iter().copied()).collect();
            let x = points
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
                    (lo.min(p.x), hi.max(p.x))
                });
            let a0 = a.first().copied().unwrap_or(0.0);
            let a = a
                .iter()
                .map(|v| near(*v, a0))
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                    (lo.min(v), hi.max(v))
                });
            let step = points.len().div_ceil(MAX_UNREACHED_POINTS).max(1);
            let points: Vec<Point3> = points
                .into_iter()
                .step_by(step)
                .take(MAX_UNREACHED_POINTS)
                .collect();
            Some(Unreached {
                x,
                a,
                area: parts.iter().map(|s| s.area).sum(),
                reason: first.reason,
                points,
            })
        })
        .collect();
    out.sort_by(|a, b| b.area.total_cmp(&a.area).then(a.x.0.total_cmp(&b.x.0)));
    Ok(out)
}

/// A run left out for being shorter than the Shortest cut, as an area left
/// uncut: its X and A ranges, its length times the stepover, and its
/// contacts.
fn short_area(pass: &Pass, length: f64, stepover: f64) -> Unreached {
    let range = |v: &mut dyn Iterator<Item = f64>| {
        v.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), x| {
            (lo.min(x), hi.max(x))
        })
    };
    let x = range(&mut pass.points.iter().map(|c| c.contact.x));
    let a = range(&mut pass.points.iter().map(|c| c.a.0));
    let n = pass.points.len();
    let take = n.div_ceil(MAX_UNREACHED_POINTS).max(1);
    Unreached {
        x,
        a,
        area: length * stepover,
        reason: UnreachedReason::TooShort { length },
        points: pass
            .points
            .iter()
            .step_by(take)
            .map(|c| c.contact)
            .collect(),
    }
}

/// The areas for a message: the largest five, each with where, how large
/// and why, then how many more.
fn unreached_list(areas: &[Unreached], diameter: f64) -> String {
    const LISTED: usize = 5;
    let listed: Vec<String> = areas
        .iter()
        .take(LISTED)
        .map(|u| {
            let why = match u.reason {
                UnreachedReason::TooNarrow => {
                    format!("the face is too narrow for this {diameter:.3} mm tool there")
                }
                UnreachedReason::TooShort { length } => format!("a cut of {length:.1} mm"),
                _ => "a neighbouring face is in the way of the tool".to_owned(),
            };
            format!(
                "X {:.3} to {:.3} near A {:.0} ({:.1} mm2), {why}",
                u.x.0,
                u.x.1,
                (0.5 * (u.a.0 + u.a.1)).rem_euclid(360.0),
                u.area
            )
        })
        .collect();
    let more = areas.len().saturating_sub(LISTED);
    let more = if more > 0 {
        format!("; and {more} smaller")
    } else {
        String::new()
    };
    format!("{}{more}", listed.join("; "))
}
