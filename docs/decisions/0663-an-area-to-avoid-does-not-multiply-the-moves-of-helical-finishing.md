# 0663: An area to avoid does not multiply the moves of helical finishing (OP-02, OP-04, NFR-04, NFR-12; issue #608; lane SIM-FINISH)

- **Status:** Accepted by the project owner on 22 Nov 2026, relayed by the coordinator (decision 0049). Proposed by lane SIM-FINISH the same day; built in the lane's pull request.
- **Date:** 22 Nov 2026.
- **Builds on:** [0034](0034-slice-12-baseline-finishing.md) item 5 (region offsets replace sampling; `CONSTRAINED_SPACING` kept as the fallback for a constraint that cannot be developed), decision 0032 item 6 (a constraint is seen only where the tool is dropped).
- **Context:** OP-02 (selections, boundaries, avoid and check surfaces), OP-04 (helical finishing), NFR-04 (generation on several cores; no requirement sets a generation time). It changes no requirement, no architecture and no API.
- **Open items:** OI-2168 (the same fallback in axial finishing and the simultaneous spiral); OI-2169 (the user guide's region example, re-captured after merge).

## Problem

With an Avoid selection set (Cut short) and no Cut on set, helical finishing made about 4 times the moves of the same finish without it on the baluster tutorial (74,918 against about 19,150), and took twice as long, while cutting less. With Cut on, the same set gave 2,236 moves.

Cause: Cut on gives a selected region, which is developed and intersected with the helix, so only the intervals that can reach it are fitted, each thinned to the points the chord limit needs (decision 0034 item 5). Avoid alone has no selected region to develop, so the fallback applies: the helix is seeded no more than `CONSTRAINED_SPACING` (0.1 mm of path at the part's largest radius) apart, so the edges of the area to avoid are found. That is right for finding the edges, but every seed was then kept as a point of the path (`thin: false`), so the rest of the part was cut in moves of 0.1 mm instead of the operation's own Largest angle step. A cylinder of radius 15 mm with a 4 mm strip avoided made 33,930 cutting moves against 2,880 without it.

## Decision

Where the constraint alone made the seeds finer than the operation's own angle step, the runs of seeds between the edges of the area to avoid are thinned back to the points the chord limit needs, never further apart than the operation's own step (`crates/openchuck-path/src/helical.rs`: `thin`, and `max_dt` as the unconstrained step's share of the helix, uncapped, so each ring keeps that angle however short the part is: CodeRabbit on #837). The seeds are still dropped at 0.1 mm, so an area to avoid is found as before; each run keeps its first and last points (at the edges of the area), and the path fitter then refines every kept move at its middle and quarter points, splitting it where the constraint forbids cutting, as for Cut on. This is the thinning Cut on already uses (`wrap_fit::fit_region`), applied to the fallback.

Not a choice for the owner: the program keeps the same tolerance and the same constraint checks; only the extra points of the dense seeds go.

## Consequences

- **Changed output:** helical finishing with an Avoid or Check set, or a boundary, and no Cut on set (the fallback) makes about as many moves as without the constraint, and generates faster. Operations without a constraint, or with a Cut on set that can be developed, are unchanged.
- Axial finishing and the simultaneous spiral seed the same fallback the same way and keep every seed too; that is OI-2168, left for a later change so this one changes only the strategy the issue names.
- The user guide's *Limit an operation to a region* states the old move count with Avoid and says Avoid can lengthen the program; its figure and numbers are re-captured by a guide lane after merge (OI-2169).
- No API change; no project schema.
- Generated G-code stays the user's to verify before it runs on a machine (NFR-12).

## Verification

### Tests

- `region::req_op_02_an_avoid_surface_does_not_multiply_the_moves_of_the_rest_of_the_part` (`crates/openchuck-path/tests/region.rs`): helical finishing of a cylinder of radius 15 mm and 40 mm long with a 6 mm ball, the strip from X 18 to 22 avoided, on the spiral and in rings: the cutting moves with the strip avoided are no more than 1.1 times those without. Written first: it failed on `main` (spiral: 33,930 cutting moves against 2,880) and passes with the change (spiral 2,610 against 2,880; rings 2,988 against 3,352), 7.0 s uninstrumented.
- The existing region tests (`region::req_op_02_an_avoid_surface_is_never_cut`, `region::req_op_02_a_small_avoid_patch_between_two_default_seeds_is_not_missed`, and the rest of `region.rs`) still pass: no cut on the avoided strip, cuts up to its edges, and a patch 1 degree wide between two default seeds still found.

### Runs

- The whole `openchuck-path` suite with this change and decision 0662's (before decision 0660's note change): 932 tests passed, 1 skipped, 1,166 s (`cargo nextest run -p openchuck-path`, debug build, uninstrumented, 4 cores, Linux), at the lane's working tree on `main` 57b8ac1b.
- Not verified: the baluster tutorial's own numbers with the ball avoided (the set is made in the window; its figure and table are re-captured by a guide lane, OI-2169).
