# 0701: Lead and Helix entries are measured against the faces outside an operation's selection that they come near (OP-11, OP-12, NFR-07, NFR-12, NFR-06; issue #957; lane PATH-QUALITY)

- **Status:** Proposed by lane PATH-QUALITY on 27 Nov 2026. It records the owner's answers on issue #957, relayed by the coordinator (decision 0049):
  - Q1: "accept B with per-entry proximity";
  - Q2: "refuse";
  - Q3: the tolerance-plus-slack allowance.

  Only the project owner accepts this record (decision 0049).
- **Date:** 27 Nov 2026.
- **Issue:** #957 (Lead and Helix entries on strategies other than swarf are checked against the operation's surface, not the whole part).
- **Requirements:** OP-11 (entry and exit moves), OP-12 (gouge checks where defined), OP-02 (selections), NFR-07, NFR-12, NFR-06 and GC-07 (determinism).
- **Amends:** accepted [decision 0682](0682-swarf-entries-and-exits-meet-the-faces-next-to-the-face-set.md) item 1 ("Other strategies are not given neighbours and are unchanged") and item 4 (only the faces that share an edge with the face set are checked), for strategies other than swarf. Swarf is unchanged.
- **Builds on:** [0682](0682-swarf-entries-and-exits-meet-the-faces-next-to-the-face-set.md) (the measure: face samples against the tool's cutting profile, the sampling slack), [0532](0532-swarf-keeps-off-the-faces-at-its-ends.md) item 1 (the sampling), [0527](0527-a-selection-stops-at-its-edge-with-an-unselected-face.md) (the part's faces outside a selection, which the host already gives every strategy), [0026](0026-entry-moves.md) (entry styles; an entry never changes style on its own), [0645](0645-a-swarf-ramp-follows-the-pass-along-the-face.md) and [0695](0695-finishing-edges-points-and-ramps.md) (a ramp follows its own pass).
- **Open items:** none added. OI-2078 (the sampling of a guarded face) is unchanged and also bounds this measure.

Numbering: decision 0701, the next free number across `main` and the unmerged branches on 27 Nov 2026, given by the coordinator. No project schema number is needed: nothing new is stored.

## Context

An entry or exit (a lead arc, a helix) is checked against the operation's own surface (`assemble/entry.rs`, `Entry::gouge`). For an operation limited to a selection, that surface is the selected faces. So a lead or helix that comes down beside the selection can cut into a face of the part outside it with no word.
- Swarf has been measured against the faces that share an edge with its face set since decision 0682.
- Issue #957 asked whether every strategy should be.

Probes on the public shouldered shaft of the test kit (`openchuck_testkit::brep::shouldered_shaft`, helical finishing, a 3.175 mm flat end mill):
- **Face 6 alone** (radius 8, X 26 to 40):
  - the default helix comes down 0.15 mm into the fillet beside it (face 5);
  - a helix of Helix diameter 1.5 comes down 0.4 to 0.6 mm into the fillet, the shoulder (face 4) and the larger cylinder (face 3). Faces 3 and 4 share no edge with the selection.
  - The passes themselves cut none of them. Today all of this is made with no word.
- **Face 1 alone** (radius 8, X 0 to 12): the passes themselves cut about 1.5 mm into the shoulder (face 2) and the larger cylinder (face 3) beside its end, so a lead there does too. Nothing measures either today.

## Decision

1. **Which faces (the owner's Q1: B with per-entry proximity).** For every strategy but swarf, a lead or helix is measured against the part's faces outside the operation's selection that it comes near.
   - **The candidate faces:** the host already gives every strategy limited to a selection the part's faces outside it (`RegionConstraint::outside`, decision 0527), so no new input is needed.
   - **The reach:** for each entry or exit, the box the tool's cutting part sweeps over its positions and where it meets the pass. That is the tip and the line on up the tool axis at each position, past the top of the flutes as far as any of the faces goes, since the measure counts the tool on up at its cutting radius. The box is grown by the tool radius, the stock to leave and the tolerance.
   - **The faces measured:** those whose box meets the reach. A face's box is read from its whole untrimmed surface over its parameter box, on a 65 by 65 grid grown by the grid's spacing, so it holds every part of the face's trim, however narrow. A face outside the reach cannot meet the cutting part during that entry, so this is complete for the cutting part, up to the sampling slack (OI-2078).
   - **Sampling:** each such face is sampled once, the first time an entry needs it, at swarf's spacing (decision 0532 item 1), and measured as decision 0682 measures (the deepest sample inside the tool's cutting profile).
   - **Coverage:** the faces that share an edge with the selection are among them whenever an entry comes near them, so this covers option B and the faces B misses.
2. **What is allowed (the owner's Q3).** A face outside the selection may be cut no deeper than the tolerance plus the sampling slack. The pass's own cut is **not** allowed, as decision 0682 allows for swarf: the pass's cut into those faces is measured nowhere else. So the point where the move meets the pass is measured too, as the last position the move sends the tool to.
3. **What happens (the owner's Q2: refuse).** A lead or helix that cuts such a face deeper than allowed is refused, as decision 0682 does for swarf. The operation is not made, and the message names the pass, the move, the face, the depth and what to do. Two wordings:
   - **The entry cuts the face:** "the tool would cut 0.621 mm into face 5, outside the operation's selection, at position 421 of its path (the tolerance is 0.010 mm); set Entry to Plunge or Ramp, or use a smaller size".
   - **The pass itself cuts the face where the move meets it**, so no lead or helix there can keep clear: "the pass itself cuts 1.555 mm into face 3, outside the operation's selection, where the lead-out meets it, so the lead-out would cut it too (…). Set Entry to Plunge or Ramp, or change the selection or the stock to leave so that the pass keeps clear of face 3".
4. **Unchanged:**
   - **Plunge and Ramp** are not measured against these faces. A plunge goes down above its pass's first point, and a ramp follows its pass (decisions 0645, 0695).
   - **Swarf** keeps decision 0682 whole: the faces next to its face set, with the pass's own cut allowed (its pass cut is measured and warned of, 0532 item 3), even though the host also gives it the faces outside its face set.
   - **An operation on the whole part**, or with no selection, has no faces outside it and is unchanged.
   - **An operation on a mesh (STL) selection** is unchanged: its faces outside are triangles, which carry no face numbers to measure and name. This is an open point.
5. **NFR-12.** Generated G-code remains the user's to verify before it is run on a machine. The check covers the tool's cutting part against the part's faces near each lead and helix. The shank, the holder, the stock, a mesh part's selection and a cut between samples are not covered. Nothing here says an entry is safe.

## Consequences

- **Existing projects.** A STEP operation limited to a selection whose Entry is Lead or Helix, and whose entry or exit cuts a face of the part outside the selection by more than the tolerance, is now refused where it was made, with the message above. That includes every place where the pass itself cuts such a face. The fix is Entry Plunge or Ramp, or a smaller size, or a selection or stock to leave that keeps the pass clear. Operations on the whole part, on STL selections, with Plunge or Ramp, and swarf operations are unchanged.
- **Shipped examples and goldens.** See *Verification*. Any example that became refused would be a finding reported to the coordinator, not adjusted.
- **Library (decision 0048).** There is no public API change. The library family takes its own minor version, provisional **0.75.0**, after #968's 0.74.0. It is an output change: a refusal where a program was made.
- **Determinism.** A face's samples are made once, the same whichever thread asks first, and the measure reads the faces in number order, so the result is the same at 1, 2 and N threads (tested).
- **Cost.** At the operation's start, each face outside the selection is read on a 65 by 65 grid of its untrimmed surface for its box. Each face an entry comes near is then sampled once, at the tool radius over 8 (at least 0.05 mm, at most 400 by 400 a face). Faces no entry comes near are never sampled.

## Open points

- A mesh (STL) selection's faces outside it are not measured (item 4). A measure of triangles near each entry would cover it. Raise an open item if the owner wants it.
- The pass's own cut into faces outside a selection is still not measured or warned of in itself (the #572 family). This record only refuses the leads and helices it would catch, and says that the pass itself cuts the face.

## Verification

Tests written first, from the issue and the owner's answers, in `crates/openchuck-path/tests/entry_proximity.rs` (on the public shouldered shaft). The four refusal tests were seen failing before the code, where the operation was made; the two that are made, and the threads test, pass on the old code too and guard the rule:
- `req_op_11_req_op_12_a_helix_that_cuts_a_face_outside_the_selection_is_refused`: a helix of Helix diameter 1.5 on face 6 is made without the faces outside the selection, and refused with them, naming a face and saying "Plunge or Ramp".
- `req_op_11_req_op_12_faces_that_share_no_edge_with_the_selection_are_measured`: with only faces that share no edge with face 6 given (faces 0 to 4), it is still refused, naming face 3 or 4.
- `req_op_11_the_default_helix_into_the_fillet_is_refused`: the default helix on face 6 is refused, naming face 5.
- `req_op_11_req_nfr_12_where_the_pass_itself_cuts_a_face_the_refusal_says_so`: the default lead on face 1 is refused, saying the pass itself cuts the face, with "Plunge or Ramp" and "change the selection".
- `req_op_11_an_entry_that_keeps_clear_of_them_is_made`: the default lead on face 6 is made.
- `req_op_11_plunge_and_ramp_are_not_measured_against_them`: on face 1, where a lead is refused, Plunge and Ramp are made.
- `req_nfr_06_req_gc_07_the_measure_is_the_same_at_1_2_and_n_threads`.

In `crates/openchuck-path/tests/swarf.rs`: `req_op_11_req_op_12_swarf_keeps_its_own_allowance_beside_the_faces_outside_its_selection` (swarf given both sets makes the same program as with its faces next to the face set alone). This one passes on the old code too: it guards the rule rather than showing a fault.

In `src/assemble/entry.rs`, the unit test `req_op_12_a_large_face_next_to_the_face_set_takes_the_slack_of_its_own_samples` is ported to the new sampler and passes.

**Results** on the draft head, from `main` `7947ed78` (0.74.0), version 0.75.0 (provisional then), in this container (Linux, Rust 1.97.0):
- `cargo test --release --locked -p openchuck-path`: 153 unit, 954 integration and 1 doc test pass.
- `-p openchuck-app`: 318 unit (1 ignored) and 636 integration tests pass. The app already gives every strategy limited to a selection the faces outside it, so no app code changed, and no shipped example or golden test is refused or changes.
- `cargo fmt --all`, `cargo clippy --workspace --all-targets --locked` (no findings), `cargo check --locked` in `fuzz/`, and `cargo xtask deps`, `licences`, `records` and `trace` pass.
- The new tests take under 4 s together in release.

**Coverage** (`openchuck-path`, the 8 new tests alone, instrumented): `beside.rs` runs 112 of its 113 instrumented lines, and the changed lines of `entry.rs` 65 of 69, `lead.rs` 3 of 4 and `strategy.rs` 7 of 7. Not reached in that run:
- a defensive branch (a face with no samples, NFR-07);
- the wording for the faces next to swarf's face set and for the finished surface, run by the existing swarf and entry tests;
- the pass-cut refusal of a lead on a pass that turns A, which the shaft's helical passes do not reach.


**CodeRabbit on the draft (7313593c), three Majors, fixed in the release push.** Each has a unit test, written before the fix:
- `assemble::beside::tests::req_op_12_a_face_the_coarse_grid_misses_is_still_measured`: a face trimmed to a band about 0.7 mm wide along a 200 mm diagonal, which the 2 mm grid misses (the test asserts that). Before, the face was dropped, where the measure before this record sampled every face at its own spacing. It is now kept and sampled at the measure's own spacing; since the second push its box comes from its untrimmed surface.
- `assemble::beside::tests::req_op_12_the_reach_covers_the_tool_above_its_flutes`: a face 30 mm up the axis of a tool with 10 mm of flutes, within its radius, is near; one 30 mm below the tip is not. Before, the reach stopped at the top of the flutes, though the measure counts the tool on up at its cutting radius, so a swarf entry could lose a refusal decision 0682 gave.
- `assemble::entry::tests::req_op_11_req_op_12_the_join_is_measured_against_the_faces_outside_the_selection`: where only the join cuts a face outside the selection deeper than allowed, it is refused, as the pass's own cut; beside swarf's face set the join stays the allowance; a deeper move is the one named.

**CodeRabbit on the release head (71ccb4cb): one more Major, fixed in a second push with the coordinator's leave.** A face the coarse grid saw in one part of its trim and missed in another (a square and a narrow band) took its box from the part it saw, so an entry near the other part left the face out. Each face's box is now read from its untrimmed surface (item 1). Test: `assemble::beside::tests::req_op_12_a_face_the_coarse_grid_sees_only_in_part_is_near_all_of_it`, which failed before the change. On the same review, CodeRabbit asked for the refusal itself above the flutes, not only the proximity: `assemble::beside::tests::req_op_11_req_op_12_a_face_above_the_flutes_is_refused_beside_swarf` shows a face 30 mm up the axis of a tool with 10 mm of flutes cut more than 1 mm and refused under swarf's allowance (it passes on the first fix: it guards it).

**Release head** (`main` `bfbcac75` merged, library family 0.75.0, confirmed by the coordinator):
- `cargo test --release --locked -p openchuck-path`: 157 unit, 954 integration and 1 doc test pass.
- `-p openchuck-app`: 318 unit (1 ignored) and 637 integration tests pass. No shipped example or golden test is refused or changes, now that the join is measured too.
- `cargo clippy --workspace --all-targets --locked` gives no findings, and `cargo check --locked` in `fuzz/` and `cargo xtask deps`, `licences`, `records` and `trace` pass.

Not done: no air or wood cut. Generated G-code remains the user's to verify before it is run on a machine (NFR-12).
