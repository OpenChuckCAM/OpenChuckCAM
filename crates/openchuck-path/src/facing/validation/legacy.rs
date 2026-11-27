// SPDX-License-Identifier: AGPL-3.0-or-later
//! Independent completion of supported restricted legacy Facing. Raised
//! composite-target regions currently refuse rather than masquerade as flats.

use super::super::coverage::{FlatSweep, verify_flat_raster};
use super::super::description::FaceDescription;
use super::*;
use crate::RotaryMapping;
use openchuck_types::ir::MoveKind;

pub(in crate::facing) fn verify(
    setup: &FacingSetup<'_>,
    parameters: &Parameters,
    group: &ToolpathGroup,
    context: &JobContext<'_>,
) -> Result<()> {
    let tolerance = parameters.number(names::TOLERANCE)?;
    verify_rapid_clearance(setup, group, 0., context)?;
    verify_protected_clearance(setup, group, tolerance, 0., context)?;
    verify_shank_clearance(setup, group, 0., context)?;
    match describe(setup, parameters)?.geometry {
        FacingGeometry::Flat { faces } => {
            let bound = flat_bound(setup, parameters)?;
            for mut face in faces {
                let sweeps = flat_sweeps(&face, group, tolerance, context)?;
                if !face.unfaced.is_empty() {
                    let segments = segments(&face, &sweeps)?;
                    super::super::held::measure_flat(&mut face, &segments, bound);
                }
                flat_completion(
                    &face,
                    &sweeps,
                    0.5 * setup.tool.diameter.0,
                    tolerance,
                    context,
                )?;
            }
        }
        FacingGeometry::Round {
            diameter,
            x_extent,
            unfaced,
        } => round(
            setup,
            group,
            diameter * 0.5,
            crate::facing::description::faced_span(x_extent, &unfaced),
            tolerance,
            context,
        )?,
        _ => return Err(refuse("expected legacy Flat/Round geometry")),
    }
    Ok(())
}

/// The at-target cutting sweeps of a legacy Flat face, each run along one
/// line merged into one sweep.
pub(in crate::facing) fn flat_sweeps(
    face: &FaceDescription,
    group: &ToolpathGroup,
    tolerance: f64,
    context: &JobContext<'_>,
) -> Result<Vec<FlatSweep>> {
    let mut sweeps: Vec<FlatSweep> = Vec::new();
    let mut from = group.start;
    for segment in &group.segments {
        context.check_cancelled()?;
        let to = segment.end;
        if segment.kind == MoveKind::Cutting
            && from.a == to.a
            && (from.axis.get() - face.target.plane.normal()).length() < 1e-5
        {
            let a = face.target.plane.coordinates(from.tip)?;
            let b = face.target.plane.coordinates(to.tip)?;
            if a[2].max(b[2]) <= face.target.offset + tolerance {
                if let Some(prior) = sweeps.last_mut().filter(|p| {
                    p.to == from && (a[1] - b[1]).abs() < 1e-10 && (a[2] - b[2]).abs() < 1e-10
                }) {
                    prior.to = to;
                } else {
                    sweeps.push(FlatSweep { from, to });
                }
            }
        }
        from = to;
    }
    Ok(sweeps)
}

/// The sweeps' ends in the face plane's (X, across) coordinates.
pub(in crate::facing) fn segments(
    face: &FaceDescription,
    sweeps: &[FlatSweep],
) -> Result<Vec<([f64; 2], [f64; 2])>> {
    sweeps
        .iter()
        .map(|s| {
            let a = face.target.plane.coordinates(s.from.tip)?;
            let b = face.target.plane.coordinates(s.to.tip)?;
            Ok(([a[0], a[1]], [b[0], b[1]]))
        })
        .collect()
}

/// How much further from the grip than its keep-out legacy Flat may be
/// finished, read from its cut: the tool's diameter, for the lift its
/// composite target gives beside the held region, and the stock to leave
/// round it (decision 0685).
pub(in crate::facing) fn flat_bound(
    setup: &FacingSetup<'_>,
    parameters: &Parameters,
) -> Result<f64> {
    Ok(setup.tool.diameter.0 + parameters.number(names::STOCK_TO_LEAVE)?)
}

#[derive(Clone, Copy)]
struct Helix {
    a0: f64,
    a1: f64,
    x0: f64,
    x1: f64,
}
impl Helix {
    fn x(self, a: f64) -> f64 {
        self.x0 + (self.x1 - self.x0) * (a - self.a0) / (self.a1 - self.a0)
    }
}

fn round(
    setup: &FacingSetup<'_>,
    group: &ToolpathGroup,
    target: f64,
    extent: (f64, f64),
    tolerance: f64,
    context: &JobContext<'_>,
) -> Result<()> {
    let rotary = RotaryMapping::of_profile(setup.machine);
    // The flute must reach the selected radial tolerance shell. Restrict
    // interpolation error explicitly instead of trusting rounded endpoints.
    let reserve = super::ROUND_RESERVE;
    if tolerance < 8. * reserve || setup.tool.flute_length.0 < tolerance {
        return Err(refuse(
            "Round completion has no supported interpolation/flute precision budget at this tolerance",
        ));
    }
    let r = 0.5 * setup.tool.diameter.0 - reserve;
    if r <= 0. {
        return Err(refuse(
            "Round cutter is below its interpolation precision budget",
        ));
    }
    let mut lines = Vec::new();
    let mut phases = vec![0., 360.];
    let mut from = group.start;
    for segment in &group.segments {
        context.check_cancelled()?;
        let to = segment.end;
        if segment.kind == MoveKind::Cutting {
            let a = rotary.to_axis(from.tip, from.a);
            let b = rotary.to_axis(to.tip, to.a);
            if a.height.min(b.height) < target - tolerance {
                return Err(refuse("Round cut overcuts the nominal radius"));
            }
            if a.height.max(b.height) <= target + 0.25 * tolerance {
                if a.across.abs() > reserve
                    || b.across.abs() > reserve
                    || (a.height - b.height).abs() > reserve
                {
                    return Err(refuse(
                        "raised/off-axis Round composite-target completion is not checked",
                    ));
                }
                let (low, high, x0, x1) = if from.a.0 < to.a.0 {
                    (from.a.0, to.a.0, from.tip.x, to.tip.x)
                } else {
                    (to.a.0, from.a.0, to.tip.x, from.tip.x)
                };
                if high > low {
                    if high - low > 360. {
                        return Err(refuse("Round segment exceeds one supported physical turn"));
                    }
                    let source = Helix {
                        a0: low,
                        a1: high,
                        x0,
                        x1,
                    };
                    let base = (low / 360.).floor() * 360.;
                    for shift in [base, base + 360.] {
                        let lo = low.max(shift);
                        let hi = high.min(shift + 360.);
                        if hi > lo {
                            let line = Helix {
                                a0: lo - shift,
                                a1: hi - shift,
                                x0: source.x(lo),
                                x1: source.x(hi),
                            };
                            phases.extend([line.a0, line.a1]);
                            lines.push(line);
                        }
                    }
                }
            }
        }
        from = to;
    }
    phases.sort_by(f64::total_cmp);
    phases.dedup();
    if phases.len().saturating_mul(lines.len()) > 20_000_000 {
        return Err(refuse("Round coverage work budget exceeded"));
    }
    for pair in phases.windows(2) {
        let [lo, hi] = *pair else {
            return Err(refuse("invalid Round phase slab"));
        };
        context.check_cancelled()?;
        // Each interval is present for EVERY physical angle in this slab,
        // because X is affine in A. Their union must cover the entire X span.
        let mut spans = lines
            .iter()
            .filter(|l| l.a0 <= lo && l.a1 >= hi)
            .map(|l| {
                let a = l.x(lo);
                let b = l.x(hi);
                (a.max(b) - r, a.min(b) + r)
            })
            .filter(|(a, b)| a <= b)
            .collect::<Vec<_>>();
        spans.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut x = extent.0;
        for (a, b) in spans {
            if b < x {
                continue;
            }
            if a > x {
                break;
            }
            x = x.max(b);
        }
        if x < extent.1 {
            return Err(refuse(
                "Round has an unfinished axial/angular strip; full circumference is required",
            ));
        }
    }
    if lines.is_empty() {
        return Err(refuse("Round has no checked full-circumference finish"));
    }
    Ok(())
}

// The final completion boundary after collecting actual sweeps. Keep error
// policy here so cancellation arriving at this stage can be checked directly.
fn flat_completion(
    face: &super::super::description::FaceDescription,
    sweeps: &[FlatSweep],
    radius: f64,
    tolerance: f64,
    context: &JobContext<'_>,
) -> Result<()> {
    // Read from the cut beside grips at both ends, the faced span can close;
    // nothing is then left to face, which refuses (decision 0685, NFR-07).
    let ([u0, _], [u1, _]) = (face.faced().min(), face.faced().max());
    if u0 >= u1 {
        let grips = face
            .unfaced
            .iter()
            .map(|u| {
                format!(
                    "{xp1} to {xp2}",
                    xp1 = crate::words::x(u.held.0),
                    xp2 = crate::words::n(u.held.1)
                )
            })
            .collect::<Vec<_>>()
            .join(" and ");
        return Err(refuse(&format!(
            "nothing is left to face on the Flat face at A {:.3}°: where its passes reach the face \
             beside the held regions ({grips}), the spans left unfaced meet",
            face.target.angle.0
        )));
    }
    verify_flat_raster(face.faced(), face.target.plane, face.target.offset,
        sweeps, radius, tolerance, context)
        .map_err(|error| match error {
            PathError::Cancelled => error,
            _ => refuse("restricted legacy Flat has an unfinished or raised composite-target region; complete coverage of that composite surface is not checked"),
        })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::facing::description::FaceDescription;
    use crate::facing::geometry::FacingPlane;
    use crate::facing::selection::ResolvedSide;
    use openchuck_types::geometry::Point3;
    use openchuck_types::job::{CancelToken, NoProgress};
    use openchuck_types::machine::MachineProfile;
    use openchuck_types::units::Degrees;

    /// Decision 0685: where nothing is left to face beside the grips, Facing
    /// refuses. Flat's faced span is read from its cut; grips at both ends
    /// can move its edges past each other (start edge X 6, far edge X 5), and
    /// a sweep across the whole face would then "prove" an empty span. The
    /// completion check refuses it, naming the held regions (NFR-07).
    #[test]
    fn req_op_24_req_nfr_07_flat_with_its_faced_span_closed_by_both_grips_refuses() {
        use crate::facing::geometry::FacingEnd;
        use crate::facing::held::Unfaced;
        use openchuck_types::geometry::UnitVec3;
        use openchuck_types::ir::ToolPose;
        let plane = FacingPlane::side(
            RotaryMapping::of_profile(&MachineProfile::example_x_rotary()),
            Degrees(0.),
        )
        .unwrap();
        let mut face = FaceDescription {
            target: ResolvedSide {
                angle: Degrees(0.),
                offset: 7.,
                plane,
            },
            coverage: plane
                .bounds(
                    [Point3::new(0., 0., 7.), Point3::new(10., 10., 7.)],
                    &[],
                    0.,
                )
                .unwrap(),
            unfaced: Vec::new(),
        };
        let token = CancelToken::new();
        let context = JobContext::new(&token, &NoProgress);
        let [_, v0] = face.coverage.min();
        let [_, v1] = face.coverage.max();
        let at = |x: f64| ToolPose {
            tip: plane.point(x, 0.5 * (v0 + v1), 7.).unwrap(),
            axis: UnitVec3::new(plane.normal()).unwrap(),
            a: Degrees(0.),
        };
        let sweep = [FlatSweep {
            from: at(0.),
            to: at(10.),
        }];
        // One sweep wide enough to cover the face: proved complete.
        flat_completion(&face, &sweep, 20., 0.01, &context).unwrap();
        face.unfaced = vec![
            Unfaced {
                beside: FacingEnd::Start,
                held: (-5., -1.),
                x: (0., 6.),
            },
            Unfaced {
                beside: FacingEnd::Far,
                held: (11., 15.),
                x: (5., 10.),
            },
        ];
        let error = flat_completion(&face, &sweep, 20., 0.01, &context)
            .unwrap_err()
            .to_string();
        assert!(error.contains("nothing is left to face"), "{error}");
    }

    #[test]
    fn req_ui_09_req_op_24_cancellation_during_legacy_flat_completion_stays_cancelled() {
        let plane = FacingPlane::side(
            RotaryMapping::of_profile(&MachineProfile::example_x_rotary()),
            Degrees(0.),
        )
        .unwrap();
        let face = FaceDescription {
            target: ResolvedSide {
                angle: Degrees(0.),
                offset: 7.,
                plane,
            },
            coverage: plane
                .bounds(
                    [Point3::new(0., 0., 7.), Point3::new(10., 10., 7.)],
                    &[],
                    0.,
                )
                .unwrap(),
            unfaced: Vec::new(),
        };
        let token = CancelToken::new();
        // Simulate a request arriving after sweep collection but before the
        // completion proof; early clearance checks cannot mask this boundary.
        token.cancel();
        assert!(matches!(
            flat_completion(&face, &[], 1., 0.01, &JobContext::new(&token, &NoProgress)),
            Err(PathError::Cancelled)
        ));
    }
}
