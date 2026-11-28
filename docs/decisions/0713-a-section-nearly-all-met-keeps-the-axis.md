# 0713: A station whose rays nearly all meet the part keeps the rotary axis (OP-26, OP-05, NFR-06, GC-07, NFR-12; issue #917; lane PATH-QUALITY)

- **Status:** Proposed by lane PATH-QUALITY on 28 Nov 2026. It records the owner's answer on issue #917 (OI-2234), "A with investigation of B", and the answers of 28 Nov 2026 to keep A as a guard after the importer fix (decision 0712, option B's F1), relayed by the coordinator (decision 0049). Only the project owner accepts this record (decision 0049).
- **Date:** 28 Nov 2026.
- **Issue:** #917 (STEP blisk: spurious off-axis zones in 0.05 mm strips).
- **Requirements:** OP-26 (off-axis sections), OP-05 (simultaneous finishing), NFR-06 and GC-07 (determinism), NFR-12.
- **Amends:** accepted [decision 0363](0363-simultaneous-finishing-off-axis-sections-walls-and-held-ends.md) item 3 ("A station where some [rays] meet the part and some do not has a section that does not surround the axis").
- **Open items:** none added. OI-1431 (a section in separate pieces driven round its box middle) is unchanged.

Numbering: decision 0713, provisional: the coordinator gives the number at the release push. No project schema number is needed.

## Context

Simultaneous finishing finds where the part's section does not surround the rotary axis by casting 360 rays towards the axis at stations 0.1 mm apart (decision 0363 item 3). One ray in 360 that missed made a station off-axis, and the station was then driven round its box middle.

On the STEP blisk, a few rays at a station grazed the edge between two faces and missed: five at one station, one a blade, 72° apart. The station was taken off the axis, round a box middle 1.1 to 1.5 mm away (a five-blade section's box is not centred on the axis). The consequences:
- simultaneous finishing's spiral was split into pieces, with extra retracts;
- about 120 lines in each 0.05 mm strip went round the false centre;
- a note said the section does not surround the axis.

The STL blisk had none of this. The rays a hair to either side met the part.

The importer fix of decision 0712 narrows the cases but does not remove them. On the blisk, partial misses at 2 stations became 1 (5 rays, one a blade), and a retry 0.01° to either side met the part on all 5.

## Decision

1. **A ray that misses is tried again 0.01° either side** (`section::RETRY_ANGLE`) before it counts as a miss. That costs nothing where every ray meets the part, and three casts a missed ray where the section is off the axis.
2. **A station is off the axis only where more than 2% of its rays still miss** (`section::MOST_MISSED`, the owner's 98%). Otherwise it keeps the axis. A section that does not surround the axis misses far more: a pin beside the axis, most of a turn.
3. Everything else in decision 0363 item 3 is unchanged: the stations, the centre rounds and the snap to the axis.

## Alternatives and risks

- **A span rule** (the misses must cover a real angle) was the other half of the owner's option A. The 98% rule covers it: an off-axis feature large enough to matter misses far more than 7° of rays.
- **Risk:** a real feature off the axis that blocks no more than 2% of the rays (about 7°) at a station is now driven round the axis. The drive's own rays still meet the surface there.
- **Risk, the owner's condition:** removing the false strips once exposed a 2.0 mm overcut in the simulation at the shoulder rim. That came from decision 0695's weak move check (lane SIMUL-EDGE), which the strips hid. Before this change is released, a simulation of the blisk's simultaneous finish must show that the overcut does not return. That needs SIMUL-EDGE's rebuilt move check on `main`; see Verification.

## Verification

Tests were written first, each seen failing on the old code:
- `openchuck-path`, `section::tests` (unit), on a synthetic pentagon prism centred on the axis (its box middle 1.34 mm off it, as the blisk's):
  - `req_op_26_req_nfr_06_an_isolated_ray_that_grazes_an_edge_keeps_the_axis`: five rays 72° apart that miss keep the axis. Before, the station was taken 1.34 mm off it.
  - `req_op_26_a_section_nearly_all_met_keeps_the_axis`: a 3° gap that the retry cannot cross keeps the axis, and a 9° gap is still off it. The 3° case failed before.
  - `req_op_26_an_off_axis_section_is_still_found`: a pin off the axis keeps its own centre, unchanged.
- `openchuck-app` (kernel tests), `step_strategies::req_op_26_req_op_05_simultaneous_finishing_on_the_step_blisk_keeps_the_axis`: simultaneous finishing on the public STEP blisk, held either side of ten stations round X 48, on the whole part's station grid (10 s).
  - Before: "the part's section does not surround the rotary axis at X 47.973 to 48.023".
  - Now: no such note.

Still to do before release: the simulation check of the owner's condition above, once SIMUL-EDGE's rebuilt move check is on `main`.

Not verified: no air or wood cut. Generated G-code remains the user's to verify before it is run on a machine (NFR-12).
