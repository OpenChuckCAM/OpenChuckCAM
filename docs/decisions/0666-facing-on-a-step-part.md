# 0666: Facing on a STEP part: what bounds the part, and the plain refusal until it is built (OP-23, OP-24; issue #800; lane FACING-STEP)

- **Status:** Accepted by the project owner on 22 Nov 2026, relayed by the coordinator (decision 0049). Proposed by lane FACING-STEP the same day; the owner chose option C (conservative bounds from the exact faces), also relayed. Items 1 and 2 of *Decision* (the plain refusal and the import notice) are built in the lane's first pull request (#836) as an interim that changes no G-code. Item 3, option C, is built in the lane's second pull request, after the 0.55.0 crates publish, with the shared 0.56.0 bump; that removes items 1 and 2 (OI-2176, closed by it). See *Built: option C*. The number is provisional: the Merge Manager assigns the final number at release.
- **Date:** 22 Nov 2026.
- **Requirements:** OP-23 (every strategy that works on a mesh also works on a STEP part; one that keeps the tessellation says so in its notes, with the reason), OP-24 (facing), NFR-05, NFR-07, NFR-12, GC-07, NFR-06. Verification plan 4.1 (the 0.005 mm tessellation share of the 0.01 mm toolpath tolerance).
- **Builds on:** [decision 0252](0252-facing-ends-sides-and-extents.md) section 4 (accepted): "STEP bounds must identify their geometry source and have a conservative projection-error bound compatible with operation tolerance; a tessellation setting alone is not proof. If the required geometry or accuracy cannot be established, explain and refuse the affected mode rather than label approximate bounds exact." [Decision 0211](0211-step-accuracy-measured.md) M1 (accepted): the import's tessellation measured against the exact faces of `blisk.step`. [Decision 0209](0209-step-geometry-for-every-strategy.md) (accepted, OP-23).
- **Open items:** adds OI-2175 (the owner's choice of option), closed by the owner's choice of C, and OI-2176 (building C). Depends on OI-81 and OI-839 without closing them.

Numbering: the coordinator reserved records 0666 to 0671 and OI-2175 to OI-2184 for lane FACING-STEP, checked free on `main` `57b8ac1b` on 22 Nov 2026 with no unmerged lane branches. This record takes 0666, OI-2175 and OI-2176. No project schema number: nothing stored changes.

## Context

Issue #800 (sev2): every Facing operation that checks its cut against the part is refused on a STEP part with "the whole source has no established projection bound". The same project with the part as an STL generates.

The cause, on `main` `57b8ac1b`:

- `openchuck-app` `operation.rs` gives the Facing checks `projection_error: placed.brep.is_none().then_some(0.0)`: `Some(0)` for an STL (its triangles are the source), `None` for a STEP part, with the comment "A STEP tessellation's nominal deflection alone is not a proved projection bound."
- `openchuck-path` `facing/validation.rs` `validate_setup` refuses `None`, and also refuses any bound above zero: "nominal Facing completion currently requires exact placed triangles; nonzero source projection uncertainty is not checked".
- `facing/plan.rs` and `facing/end.rs` already accept a nonzero bound up to a quarter of the operation's tolerance and move the End cut out by it; `facing/motion.rs` and `motion/protection.rs` widen the part's bounds by it. The written-motion check (`openchuck-app` `facing/written.rs`) requires `Some(0)` for its Z1 footer clearance.

Which modes are affected: only those that build the whole-setup Facing checks (`openchuck-app` `facing::needs_verification`): the End and Box shapes, a Part or Part plus tabs extent, and a nonzero extent margin. Flat or Round over the whole stock with no margin (the legacy defaults) does not read the part and already runs on a STEP part.

**What Facing reads from the part.** Every use in `openchuck-path` `facing/` is a bound, never a drop or a contact: the part's axial extent (`FacingExtent::Part.axial`), its projected rectangle on an end or side plane (`planar`), its height above a side plane at an angle A (`support`), and the largest radius and coarse box of its vertices (`description.rs`, `plan.rs`, `motion/protection.rs`). So a STEP part needs, for each of those directions, a value that is never inside the exact part.

**Which way the tessellation errs.** The import's tessellation has its vertices on the exact faces. On a convex region its triangles lie inside the exact surface, so the part's true extent in a direction can be larger than the triangles' extent. An End cut placed at the triangles' extreme can then reach into the part by up to that difference, less the stock to leave. On a planar end face the difference is zero.

**How big it is.** The import's setting is 0.01 mm and 0.1 rad (decision 0027 item 7). Decision 0211 M1 measured, on the blisk at that setting, up to 0.0095 mm on the B-spline blades and **0.0429 mm on the cones** (p99 0.0247): the setting is not a bound. The Facing planner's budget for a source bound is a quarter of the operation tolerance, 0.0025 mm at the default 0.01 mm (`Tolerance::DEFAULT`).

## Options

| | Option | Safety | Accuracy and cost | Fits the accepted records |
| --- | --- | --- | --- | --- |
| A | **The import's nominal deflection (0.01 mm) as the bound.** `projection_error: Some(0.01)` for a STEP part, `validate_setup` and the written check accept a nonzero bound. | **Not proved.** M1 measured 0.0429 mm on cones at this setting, so a Facing cut checked as clear can reach up to about 0.03 mm into a curved part end beyond what the check allows for. | 0.01 mm is four times the planner's 0.0025 mm budget at the default tolerance, so it is refused anyway unless the tolerance is 0.04 mm or more, or the budget rule changes. Small code change. | **No.** Decision 0252 section 4 says a tessellation setting alone is not proof; adopting it amends an accepted record. |
| B | **A measured bound per part.** At import (or on first Facing), sample the tessellation against the exact faces as M1 does, take the largest distance, add a safety factor, and use that. | Better grounded than A, still a **sampled maximum, not a bound** (0211's own limits section says so of M1). Some faces (B-spline, near cone apexes) are the hard cases. | Needs a finer tessellation to fit 0.0025 mm, and costs import time on large parts (M1: 0.23 s at 0.01 mm, 0.43 s at 0.005 mm on the blisk). Medium change in `openchuck-app` and the import. | Only with a recorded amendment of 0252 section 4 saying a measured bound is enough. |
| C | **Conservative bounds from the exact faces (recommended).** Each bound Facing reads (axial extent, projected rectangle, height above a side plane, radius) is computed from the exact faces so that it is never inside the part: closed form for planes, cylinders, cones, spheres and tori over the face's UV range; the convex hull of the control net (split to the face's UV range) for B-spline and rational faces with positive weights; edges the same way. Trims are bounded by their UV box, which can only enlarge the bound. | **Proved by construction**: each value is an upper bound of the exact face, independent of the tessellation. A cut is never placed inside the part on its account. | Exact for planar and analytic part ends (the common case, including issue #800's). On a B-spline extreme the hull can lie outside the surface, so Facing stops short there and leaves a little more stock; the operation's notes say so (OP-23) and by how much at most. Larger change: the Facing checks read the part through a bound source instead of `&TriangleMesh` alone, which is a library API change in `openchuck-path` (a workspace minor bump under decision 0048, after the 0.55.0 crates publish). | **Yes**: it is the "conservative projection-error bound compatible with operation tolerance" 0252 section 4 asks for; no record is amended. |
| D | **Keep refusing** (the interim of item 1, with no fix). | Safe. | None. | Fits 0252, but not OP-23, which asks that every strategy that works on a mesh also works on a STEP part. |

Not considered further: dropping or projecting onto the exact faces with `SurfaceSource::project` and Newton refinement. It would be tight, but a local optimiser finds a maximum, not a bound, without interval arithmetic, so it has B's safety problem at C's cost.

## Recommendation

*The owner chose C on 22 Nov 2026 (relayed); the recommendation is kept as written.*

**Option C.** It is the only option that is a proof, so it needs no change to the accepted decision 0252, and it is exact for the part ends people face most (planes, cylinders, cones). Its one cost to the user, stopping a little short of a B-spline extreme, errs on the side of leaving stock and is stated in the notes. Its code cost is a library API change, so it is built after the 0.55.0 crates publish, in this lane or a follow-up, as the owner chooses.

If the owner prefers a smaller change now, B with a recorded amendment of 0252 is the fallback; A is not recommended, because the measured 0.0429 mm shows the nominal setting is exceeded on ordinary cone faces.

## Decision

1. **Until the fix is built, the refusal says plainly what is not supported and what to do (built in this lane, interim).** On a STEP part, Facing with the End or Box shape, a Part or Part plus tabs extent, or an extent margin is refused before anything is generated, with: "Facing with the End or Box shape, a Part or Part plus tabs extent, or an extent margin does not work on a STEP part yet: these check the cut against a proved bound of the part, and a STEP part's tessellation is not one. For now, use the part as an STL for this setup, or cut the stock to length so its end is flush with the part's end, or use the Flat or Round shape over the whole stock with no margin." The check reads the project only (`openchuck-app` `operation::facing_on_step_part`, beside `brep_only`), so the operation is in error in the overview at once, the parameter form says it first, and generation refuses with the same words instead of "the whole source has no established projection bound" (NFR-07). Flat and Round over the whole stock with no margin are not refused.
2. **The STEP import notice names Facing (built, interim).** `inspect` and the import dialog say: "Facing works on a STEP part only as Flat or Round over the whole stock with no margin for now: its other modes check the cut against a proved bound of the part, which the tessellation is not."
3. **The fix: option C, the owner's choice (22 Nov 2026, relayed; OI-2175 closed), built in the lane's second pull request (OI-2176).** Each bound Facing's checks read of the part (axial extent, projected rectangle on an end or side plane, height above a side plane at an angle A, radius) is computed from the exact faces so that it is never inside the part: closed form for planes, cylinders, cones, spheres and tori over the face's UV range; the convex hull of the control net, split to the face's UV range, for B-spline and rational faces with positive weights; trims bounded by their UV box. The Facing setup reads the part through that bound source instead of the triangles alone, a breaking `openchuck-path` change: the workspace minor bump (0.56.0, shared, as the coordinator sequences it) and a changelog entry (decision 0048), only after the 0.55.0 crates publish. When built, items 1 and 2 are removed; the operation's notes say Facing bounded the STEP part from its exact faces and, where an extreme lies on a B-spline face, the most it can stop short there (OP-23). Tests from OP-23, written failing first, on public STEP parts only: a flat end (exact, no stock left short), a cylindrical end and side (exact), and a B-spline extreme where the notes say it may stop short and by at most how much; the tests of item 1 change from refused to generated, and the output is the same at any thread count (GC-07, NFR-06).

## Consequences

- **Code (items 1 and 2):** `openchuck-app` (`operation.rs`, `overview.rs`, `form.rs`) and `openchuck-import` (`step.rs`). No library-family change, no version bump, no schema change, no G-code change.
- **Code (item 3, option C, second pull request):** the Facing setup's part source in `openchuck-path` (a breaking library change: workspace minor bump and changelog under decision 0048, after the 0.55.0 publish), the two refusals in `facing/validation.rs` and the written check's `Some(0)` requirement, and `openchuck-app` supplying the exact-face bounds for a STEP part. Not in the first pull request.
- **Documents:** the user guide's Facing page and known limitations say Facing's checked modes do not yet work on a STEP part (no record or open-item numbers there). `docs/message-review/setup.md` S93 quotes the notice of item 2. No change to `docs/requirements.md` or `docs/architecture.md`.
- **Determinism:** unchanged (GC-07, NFR-06); items 1 and 2 add text only.

## Built: option C (lane FACING-STEP, second pull request)

How item 3 was built, and where it is narrower or wider than the words above. Each difference errs outward, never into the part.

1. **One enclosure, read through the existing checks.** Every bound Facing's checks take of the part is the largest value of a convex function over the part's referenced vertices:
   - the axial extent;
   - the projected rectangle;
   - the height above a side plane at an angle A;
   - the radius.
   
   So `openchuck-path` `enclose` (new, `brep/enclosure.rs`) builds points whose convex hull holds every exact face, and the application passes them to Facing as the part, with `projection_error: Some(0)`. No Facing check changed.
2. **Planes** are bounded by their edges' exact curves (lines, circles, ellipses, B-splines). A plane with an edge that has no exact curve falls back to its parameter box. That is wider than its edges, but it still holds the face, because the plane's parametrisation is affine.
3. **Analytic faces** are not bounded by exact support functions. They are bounded by circumscribed arcs, which is also closed form.
   - Each arc (a circle, an ellipse, the meridian of a sphere or torus, a parallel swept round the axis) is held by its end points and the corners where the tangents at the ends of equal sub-arcs meet.
   - Each corner lies at most 0.0005 mm (`ANALYTIC_EXCESS`) from the face.
   - A flat end is exact along its normal. A round face's bound can be up to 0.0005 mm outside it.
   
   The record's "exact for planar and analytic part ends" therefore holds exactly along X for flat ends, and to within 0.0005 mm outward otherwise. The operation's note says so.
4. **B-spline faces** start from the control-net hull of the record, split exactly at the knots and the face's parameter box (the existing `brep/hull.rs` code). The piece furthest from its surface is split first, until every control point lies within 0.001 mm (`SPLINE_TARGET`) of its Greville point or 16,384 pieces are made. The largest distance reached is the most a bound can lie outside the face, and the note states it as how far Facing may stop short. On the corpus vase (`revolved_spline.step`), whose interpolated profile has control points up to 64 mm from the axis, it is 0.0056 mm.
5. **What Facing is given** is the prism over the points' outline seen along X (their Y-Z convex hull) between their X extremes (`Enclosure::mesh`). It is exact in every direction Facing reads on an X-rotary machine (along X, across X, the radius) and wider in oblique ones. On the vase it is 5,450 points instead of 245,760; Facing's checks read the part on every move, and with every point the B-spline test took 58 s.
6. **Refusals.** A face that cannot be bounded (a weight that is not positive, geometry that is not finite, a flat face with neither exact edges nor a finite parameter box) refuses the operation, naming the face and suggesting an STL. Nothing falls back to the tessellation.
7. **Items 1 and 2 are removed.** The STEP import notice now says Facing takes its bounds of the part from the exact faces (the listing #800 asked for), and the scene's Facing preview draws from the same enclosure.
8. **Version.** `openchuck-path` gains `enclose`, `Enclosure`, `ANALYTIC_EXCESS` and `SPLINE_TARGET`; no existing signature changes. It ships with the shared workspace bump to 0.56.0 (decision 0048).

## Verification

On `main` `57b8ac1b` with this lane's changes, Linux container (4 cores, 15 GB), debug build, OpenCASCADE kernel built locally (`kernel-tests`).

- `operation::req_op_23_req_op_24_req_nfr_07_checked_facing_on_a_step_part_is_refused_plainly_before_generating` (`openchuck-app` `tests/operation.rs`, kernel-free: the part is only named as a STEP file). Run first against the unchanged source: it failed (the operation was not in error before generating). With the change it passes: the overview error, the form's first note and the generation error carry the plain words and what to do, never "projection bound"; End, Box, a Part extent and an extent margin are refused; Flat and Round over the whole stock with no margin are not. 0.01 s.
- `step_pipeline::req_op_23_req_op_24_req_nfr_07_facing_the_end_of_a_step_part_is_refused_plainly` (kernel, in the STEP kernel CI job): issue #800's set-up on the corpus rod `rotary_blank_drilled.step` (End facing, held start 10 mm, stock running 9 mm past the part). Failed against the unchanged source; passes with the change. Round facing over the whole stock generates cutting moves on the same STEP part. 6.0 s.
- `step::tests::req_op_23_req_imp_02_req_nfr_12_the_notice_says_which_strategies_use_the_tessellation` (`openchuck-import`): the notice names Facing.
- Suites: `cargo nextest run -p openchuck-path -p openchuck-app -p openchuck-import --features openchuck-app/kernel-tests`: 1899 of 1900 passed; the one failure was this record's first test asserting the generation error's case, corrected and rerun with the other new tests (7 of 7 pass). `cargo fmt --all`, `cargo clippy --workspace --all-targets -D warnings` with and without `openchuck-app/kernel-tests`, and `cargo xtask deps`, `licences`, `records` and `trace` (105 of 105 P1) pass.
- Not verified (first pull request): the fix itself (item 3), built in the second pull request (OI-2176); anything on a machine.

Generated G-code remains the user's to verify before running it on a machine (NFR-12).

### Verification of option C (second pull request)

Branch `claude/lane-facing-step-option-c`, stacked on the first pull request (#836), from `main` `57b8ac1b`. Linux container (4 cores, 15 GB), debug build, OpenCASCADE kernel built locally; uninstrumented `cargo nextest` times.

- **`openchuck-path` `tests/brep_enclosure.rs`** (kernel-free, closed forms and a rational Bezier patch evaluated in the test). All seven failed against a stub before the code. The bounds are checked against the faces' own points, never the enclosure's arithmetic.

  | Test | What it checks | Time |
  | --- | --- | --- |
  | `req_op_23_req_op_24_the_enclosure_holds_each_analytic_face_within_its_stated_excess` | Cylinder, boss, cone, sphere band, torus and a partial cylinder: the support in 206 directions is never below the face's and at most the stated excess above it | 2.4 s |
  | `req_op_23_req_op_24_a_flat_end_is_bounded_exactly_and_the_round_side_within_the_excess` | A rod with exact circle edges: X extremes exact, radius within 0.0005 mm, the disc ends not widened by their parameter squares | 0.03 s |
  | `req_op_23_req_nfr_07_a_flat_face_without_exact_edges_is_bounded_by_its_parameter_box` | The parameter-box fallback | 0.04 s |
  | `req_op_23_req_op_24_a_b_spline_extreme_is_bounded_and_the_most_it_can_stop_short_is_stated` | Never inside the patch, at most the stated excess (≤ 0.001 mm) outside | 0.25 s |
  | `req_op_23_req_nfr_07_a_patch_it_cannot_bound_is_refused_with_the_face` | A refusal names the face | <0.01 s |
  | `req_gc_07_req_nfr_06_the_enclosure_is_the_same_every_time` | Determinism | <0.01 s |
  | `req_op_23_req_op_24_facing_reads_the_same_bounds_from_the_mesh_as_from_the_points` | The prism's bounds equal the points' along X, in 720 directions across X and in radius, and are never smaller elsewhere | 0.2 s |

- **`openchuck-app` `tests/step_pipeline.rs`** (kernel, STEP kernel CI job). Each test is issue #800's set-up: End facing, start held 10 mm, stock past the part. Each generates and posts through LinuxCNC with byte-identical G-code at 1 and 2 threads, and the notes say the part was bounded from its exact faces.

  | Test | Part | Time |
  | --- | --- | --- |
  | `req_op_23_req_op_24_facing_the_flat_end_of_a_step_part_bounds_it_from_its_exact_faces` | The corpus rod; was refused before (#800) | 1.7 s |
  | `req_op_23_req_op_24_facing_the_round_end_of_a_step_part_bounds_it_from_its_exact_faces` | The corpus cylinder lying across the axis; the note gives at most 0.0005 mm | 3.6 s |
  | `req_op_23_req_op_24_facing_a_b_spline_end_says_how_far_it_may_stop_short` | The corpus vase; the note gives the B-spline figure | 7.0 s |

- **`operation::req_op_23_req_op_24_checked_facing_on_a_step_part_is_not_refused_before_generating`** (kernel-free) replaces the first pull request's refusal test.
- **`step::tests::req_op_23_req_imp_02_req_nfr_12_the_notice_says_which_strategies_use_the_tessellation`**: the notice names Facing's exact bounds.
- **Suites, coverage and checks:** in the second pull request's description.
- **Not verified:** anything on a machine. A part whose bounds Facing reads in an oblique direction (a rotary axis other than X) would get the prism's wider bound; no such machine exists in the project.

Generated G-code remains the user's to verify before running it on a machine (NFR-12).
