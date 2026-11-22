# 0667: Facing completion: an End slab at a sweep's edge, and a Box face left beside a grip (OP-24, NFR-07; issues #650 and #500; lane FACING-STEP)

- **Status:** Accepted by the project owner on 22 Nov 2026, relayed by the coordinator (decision 0049). Proposed by lane FACING-STEP the same day. It fixes and rewords refusals of the Facing completion checks within accepted decisions [0252](0252-facing-ends-sides-and-extents.md) and [0547](0547-end-facing-enters-from-air-and-checks-its-engagement.md); it changes neither, and no path or G-code that was already generated. The number is provisional: the Merge Manager assigns the final number at release.
- **Date:** 22 Nov 2026.
- **Requirements:** OP-24 (facing), NFR-07 (a clear refusal, naming what to change), NFR-05, GC-07, NFR-06, NFR-12.
- **Open items:** none new. OI-2132 (decision 0547) is not touched.

Numbering: from the range the coordinator reserved for lane FACING-STEP (records 0666 to 0671), checked free on `main` `57b8ac1b` on 22 Nov 2026 with no unmerged lane branches. No `OI-` number and no project schema number.

## 1. End facing on round bar refused at a slab's edge (issue #650)

**Reproduced** (`req_op_24_req_nfr_07_end_facing_on_round_bar_posts_what_it_generates`): the Baluster example's round bar, 66.3 mm long, End facing with the 3.175 mm O-flute, 1 mm stock to leave. The issue gives the slice along X as Stepdown; on `main` the slice along X is **Stepover (slice thickness along X)** and Stepdown is the band along the tool, so both readings are run. Stepover 2.2225 mm with 1 mm bands was refused at generation, not only when posted, with "the actual end sweeps leave an unfinished required region in an axial layer". The issue's other settings (slices 3.17 or 3.1 mm, bands 25 or 10 mm) were refused the same way.

**Cause** (`openchuck-path` `facing/coverage/end.rs`, `verify_end_coverage`). The check cuts the slab between the stock end and the target into sub-slabs whose boundaries are each sweep's own axial reach, `x ± r ∓ margin`. For each sub-slab it then recomputed every sweep's reach from the sub-slab's ends (`dx = max |x - c| + margin`) and dropped the sweep if `dx > r`, or if the chord left there, less the margin, was negative. With two slices 1.65 mm apart and r = 1.5875 mm, the sub-slab both slices share ends exactly at each sweep's edge, so `dx` is `r` give or take a rounding error: with no margin it rounded over and both sweeps were dropped; with the written-motion margin the chord less the margin was always negative. The material was removed; the proof did not see it.

**Fix.** Each sweep keeps the axial span its boundaries were made from. A sweep reaches a sub-slab when the sub-slab lies inside that span, the same numbers, so rounding cannot separate them; a boundary is within arithmetic roundoff of the true edge, inside the precision budget the check already enforces. `dx` is then at most `r`. At the sub-slab's farther end the flat tool still removes the centreline's across span, widened by the chord there; that span less the margin is what is counted, so a short or zero chord shrinks the span by the margin instead of dropping the sweep. This is still a lower bound of what the actual sweeps remove; nothing is sampled and no tolerance is added.

## 2. Box facing on the Baluster refused beside the held start (issue #500)

**Reproduced** (`req_op_24_req_nfr_07_baluster_box_facing_names_the_held_start_it_keeps_off`): the owner's Baluster set-up (box stock, model scaled 1.15, start held for 8 mm, X 16.990 to 24.990), Box facing of +Z, Whole stock and Part extents. Both were refused with "an unfinished strip remains inside the requested extent".

**Diagnosis: the refusal is correct; the message is improved.** Issue #500 stays open for the owner to decide (the pull request refers to it, it does not close it). The raster's first strip starts at X 26.37 (Whole stock) and 27.36 (Part), and the requested extent starts at X 16.99 and 17.99: the part starts inside the grip, and Facing keeps off the held region and the tool's radius beside it. Decision 0252 says Box coverage that meets held geometry is refused, never cropped silently. Without the grip the same faces generate (the same test).

**Change.** Only the words. `verify_flat_raster` (public) is unchanged; its proof now lives in a crate-private `flat_raster_gap`, which returns the first strip it cannot prove finished. The Box check says where along X, and, when the strip is a held region and the tool's radius beside it, names it and what to change: "the Box faces cannot be finished from X a to b: that is the held start (X 16.990 to 24.990) and the tool's radius (1.588 mm) beside it, which Facing keeps clear of", then for Whole stock "Use the Part only extent so the faces stop at the part, with the part clear of the grip by the tool's radius", and for Part or Part plus tabs "Shorten the held length, or move the part along X away from the grip, so the part clears it by the tool's radius". A gap elsewhere keeps the old words with "from X a to b" added.

Not done: refusing earlier, when the extent overlaps a grip. A grip narrower than the stock (a held diameter) can leave room to face above it, so overlap alone is not a reason; the refusal stays on the actual sweeps.

## Consequences

- `openchuck-path` `facing/coverage/end.rs`, `facing/coverage.rs` and `facing/validation.rs`; no public API change, no version bump. No path or G-code changes: settings that were refused now generate and post, and refusals that stay read better.
- Determinism (GC-07, NFR-06): unchanged; the checks only accept or refuse.
- The user guide is not changed: it did not describe these refusals.

## Verification

On `main` `57b8ac1b` with this lane's changes, Linux container (4 cores, 15 GB), debug build, uninstrumented `cargo nextest` times.

- `facing_approach::req_op_24_req_nfr_07_end_facing_on_round_bar_posts_the_issue_settings` and `..._posts_thick_slices` (`openchuck-app` `tests/facing_approach.rs`; one test split in two to keep each short): five settings on the 66.3 mm bar generate and post through the Makera Z1 and LinuxCNC posts. Before the fix the stepover 2.2225 mm, 1 mm band case was refused at generation with the issue's words (the end sweeps leave an unfinished required region). 25.2 s and 3.4 s alone; the unsplit test took 43.6 s in the full suite run.
- `facing_approach::req_op_24_req_nfr_05_whole_number_of_slices_posts_with_both_posts` (existing, now through the shared `round_bar` helper): passes, 9.1 s.
- `facing_approach::req_op_24_req_nfr_07_baluster_box_facing_names_the_held_start_it_keeps_off`: both extents are refused naming the held start (X 16.990 to 24.990) with their advice; without the grip the Box faces generate. Before the change both gave only "an unfinished strip remains inside the requested extent". 5.0 s.
- The `openchuck-path` Facing tests (`facing_box_plan`, `facing_coverage` and the others) pass unchanged in the full suite: 1899 of 1900 tests of `openchuck-path`, `openchuck-app` and `openchuck-import` pass, the one failure being decision 0666's test, corrected.
- Not verified: anything on a machine.

Nothing here was run on a machine. Generated G-code remains the user's to verify before running it on a machine (NFR-12).
