# 0700: Ends that do not reach round the rotary axis are faced in arcs, with a Cutting direction (OP-04, OP-09, OP-21, OP-25, SET-09, GC-16, NFR-07, NFR-12, GC-07; issue #834; lane PATH-QUALITY)

- **Status:** Proposed by lane PATH-QUALITY on 27 Nov 2026. It records the owner's answers on issue #834, relayed by the coordinator (decision 0049): build "B-improved", "the arcs join should be based on the climb / conventional / meander options" (J1, with climb the default), no note for the arcs at tabbed ends, "only count it as tabbed if the tab meets the face", the tab cases (a) to (c) below approved with the coordinator's five conditions, and WORKHOLD-TABS in its own lane. Only the project owner accepts this record (decision 0049).
- **Date:** 27 Nov 2026.
- **Issue:** #834 (a free end that does not reach round the rotary axis is skipped with no word).
- **Requirements:** OP-04 and OP-09 (finishing, end faces), OP-21 (tabs), OP-25 (cutting direction), OP-11 (entry), SET-09 (held regions), GC-16 (acknowledgement before writing), NFR-07, NFR-12, GC-07 and NFR-06 (determinism).
- **Amends:** accepted [decision 0344](0344-finishing-stepdown-and-free-ends.md) items 4, 5, 8 and 9 (as set out under *Decision*). Builds on [0297](0297-helix-strategies-turn-at-faces-across-the-axis.md) (the turn at a face across the axis, unchanged), [0213](0213-requirement-proposal-cutting-direction.md) (climb and conventional made by the feed direction with the spindle on `M3`), [0538](0538-a-possible-collision-is-a-warning-to-acknowledge.md) (a possible collision is a warning to acknowledge) and [0695](0695-finishing-edges-points-and-ramps.md) (one rule for ramp entries).
- **Related lane:** WORKHOLD-TABS (issue #963) gives the setup-level warning about a tab that does not meet the part. It is separate from this record and is not built here; the operation-level warnings of case (c) below are this record's.
- **Open items:** none added. It depends on no open item. OI-1393 (a check of whether a cut-free piece is joined to anything) stays open and applies to the arcs as it does to the rings.

Numbering: decision 0700, the next free number across `main` and the unmerged branches on 27 Nov 2026, given by the coordinator. No project schema number is needed: the new setting is stored in the operation's free parameter map, as decisions 0390, 0543 and 0545 did. No open item number was needed (OI-2271 was offered and not taken).

## Context

Decision 0344 faces a free end (an end of the part with nothing past it) with one whole turn of A in each layer, down to the rotary axis. It finds a free end only where, on every line of A, the drop finds the part on one side of an X and nothing on the other. An end that does not reach round the axis, such as the end of a pin whose axis is off the rotary axis or a crankshaft's web, has lines of A that miss it, so it was not found, and it was skipped with no word (#834). Its stock was left as the earlier operations left it.

Probes on public and synthetic parts (a pin 15 mm across with its axis 10 mm off the rotary axis, the CrankShaft STEP model) showed:

- With no tab, the end was skipped silently.
- With a tab on the axis that meets the end face (radius 4 mm against the end's nearest point at 2.5 mm), decision 0297's turn at the face across the axis already faces it down to the tab.
- With a tab that misses the face (radius 2 mm), 0297's turn faces it down to the tab and cuts the part free of the tab, with no warning.
- A Facing operation with Shape End faces such an end (2268.3 mm³ removed on the CrankShaft), with notes only.

## Decision

1. **Finding the end (amends 0344 item 4).** A free end may now be found where some lines of A miss it, as long as:
   - no line that misses it finds the part within reach of the face (so the part does not carry on past it on those angles), and
   - its face is square to the rotary axis: the edge measured along each line that finds it, with a thin probe, spreads by no more than one sample step. An end that leans is not faced (item 5).
   An end found on every line, within one sample as before, is a *round* end and is faced as 0344 says, byte for byte. Grazing lines see the edge of an off-axis end up to one tool radius later than square-on lines, so for an end not found on every line the lines are matched within one tool radius (plus a sample) of each other.
2. **The arcs (B-improved; amends 0344 item 5).** An end that does not reach round the axis is faced in nested, layered arcs down to its nearest point to the axis (the owner's "a whole turn where the arcs cover nearly a turn" is item 2's last point):
   - The nearest radius at each angle is measured with a thin probe (a flat end 0.02 mm across) one tool radius past the face, on the part alone.
   - Layers are at whole Stepdowns, as 0344's, from the highest below the stock beside the end to the last above the end's nearest point. Each angle is cut in each layer at or above its own lowest layer, so each layer's arc lies within the arc above it.
   - Each arc runs over the angles where the face is at that layer, widened by one sample at each end; an end in two pieces (two lugs) takes an arc over each.
   - Where the face is found at every sampled angle in a layer, a whole turn is cut there instead of an arc. Because each arc is widened by one sample at each end, a gap of one or two samples is cut over as well, so arcs that cover nearly a turn are cut as one.
   - The tool stands beside the face as for 0344's turn, a tenth of the tolerance clear of it.
3. **Cutting direction (J1).** Helical finishing, axial finishing and rest machining have a new setting under Face free ends, **Cutting direction**, with Meander, Climb and Conventional, **Climb by default**:
   - It acts only on the arcs. Meander alternates their direction; Climb and Conventional cut every arc the same way relative to the end face and the spindle (`M3`), by the feed direction, as decision 0213 says.
   - Whole turns, 0344's rings at round ends, keep the strategy's own direction, so every round end is cut as before and no golden program changes.
   - A saved operation without the setting takes the default, Climb. It affects only ends that were skipped before, so a saved project's program changes only where an end is now faced.
4. **Entry.** Each arc is its own pass, so the shared services make its entry and link, as for 0344's rings (decision 0695's one rule). Measured on the arcs alone: Plunge, Lead and Ramp (3°) enter them; Helix is refused, as it is at whole turns. The main passes over an off-axis part already refuse Lead, Helix and Ramp at their defaults, with or without the arcs; that is not changed here.
5. **An end that still cannot be faced.** An end found on some lines whose face is not square to the axis (it leans) is not faced, and a **warning** says so and what to do: "the end face at X … has nothing beyond it but is not faced: it is not square to the rotary axis, so the tool cannot turn beside it, and it keeps what earlier operations left. Face it with a Facing operation, and check the path before you run it". Where the end is the one opposite the setup's only held end, it says "a Facing operation, Shape End". An end a turn already faces (0297) is not warned.
6. **Tabs (the coordinator's cases, approved).**
   - **(a) A tab meets the end face.** The end is left to 0297's existing turn, which faces it down to the tab, unchanged. No arcs and no new note.
   - **(b) No tab.** The end gets the arcs (item 2) and decision 0344's two Part cut free warnings (item 7).
   - **(c) A tab past the end that does not meet the face.** 0297's turn is unchanged; it faces the end down to the tab, which cuts the part free of the tab. Only the Part cut free warnings are added: the stock past it, where nothing grips that stock, and the part itself, where nothing in the setup holds the part on its other side. The part's warning is a possible collision to acknowledge before the files are written (decision 0538). These warnings are given whether or not Face free ends is on, as the turn is 0297's.
   - **This is a visible change for saved projects** with such a tab: such a project made again gives the warning, and writing its files then waits for the acknowledgement (on the command line, `--acknowledge-collision-warnings`).
7. **Warnings (amends 0344 item 8).** The two Part cut free warnings are given for the arcs as for the rings, with "in arcs down to … mm from the rotary axis" for "down to the rotary axis" and "beside the whole face, in arcs" for "a ring". A round end's wording is unchanged.
8. **Notes (amends 0344 item 9).** An end faced in arcs says so: "the end face at X … has nothing beyond it and does not reach round the rotary axis (its nearest point is … mm from it): the tool goes down beside it in N layers of … mm to … mm from the axis, in arcs over the angles where the face is". There is no note for an end of case (a) or (c). A round end's note is unchanged.
9. **Generated G-code stays the user's to verify** before it is run on a machine (NFR-12). Nothing here says a path is safe.

## Alternatives considered

- **A whole turn at every off-axis end** (the first option B): it cuts air over most of the turn and, on a crankshaft, cuts into the webs either side. Rejected for B-improved's arcs.
- **One direction for every arc, with no setting.** The owner chose J1: the arcs follow climb, conventional or meander.
- **A note for the arcs at a tabbed end.** The owner chose none (case (a) is 0297's turn, unchanged).
- **A Lead entry special to the arcs** (an earlier provisional build, "L2"). Measurement showed the arcs take Lead as whole turns do; the refusals came from the main passes, so it was removed.

## Consequences

- **Library.** The `openchuck-path` output changes where an end does not reach round the axis (it is now faced, or warned) and where a tab misses an end (warnings). There is no public API change: the new setting is a parameter and the new types are crate-private. Under decision 0048 the library family takes its own minor version, provisional **0.74.0** (by merge order).
- **Saved projects and goldens.** No golden program and no shipped example changes. A saved project changes only where an end was skipped before (now faced, or warned), or where a tab misses an end (case (c), which asks for acknowledgement at writing).
- **Determinism.** The ends, arcs and warnings depend only on the part, the stock, the tool and the parameters, not on the thread count (GC-07, NFR-06). Tested at 1, 2 and N threads.

## Open points

- Decision 0297's turn at a face across the axis is a single full-depth turn; it is not layered by this record (cases (a) and (c) keep it as it is).
- The setup-level warning about a tab that does not meet the part is lane WORKHOLD-TABS (#963).
- Option C, facing such an end by circling its own centre (or along its edge), with a facing strategy on a selected face, is kept for later in #962 (milestone Future).

## Verification

Written from issue #834 and the owner's answers before the code; each was seen failing first on `main` `b99af9b8` (the end skipped with no word, no warning for the tab case).

`openchuck-path`, `tests/helical_faces.rs`:

- `req_op_04_req_op_09_an_end_that_does_not_reach_round_the_axis_is_faced_to_its_lowest_radius` (helical, axial and rest: what is left is within the tolerance, the lowest layer is 2.0 mm from the axis, no whole turn).
- `req_op_04_an_off_axis_end_that_the_first_line_of_a_misses_is_found` (the pin at 0°, 45°, 180° and 300°).
- `req_op_04_req_set_09_an_off_axis_end_is_named_and_its_cut_free_warnings_given`.
- `req_op_25_the_arcs_follow_the_cutting_direction` (the face on the cutter's climb side at both ends; the default is Climb).
- `req_op_04_each_layer_s_arc_lies_within_the_arc_above_it`.
- `req_op_04_an_end_in_two_pieces_takes_an_arc_over_each`.
- `req_nfr_06_req_gc_07_off_axis_end_arcs_are_identical_at_1_2_and_n_threads`.
- `req_op_04_req_nfr_12_an_end_that_cannot_be_faced_is_warned_and_named_with_facing` (a leaning end; Shape End where the far end is the only one held).
- `req_op_11_the_arcs_take_the_operations_entry_as_whole_turns_do` (Plunge, Lead and Ramp face the end; Helix is refused).
- `req_nfr_12_an_off_axis_end_on_the_setups_own_stock_is_faced_and_said`.
- `req_set_09_an_off_axis_end_with_the_part_held_beyond_it_keeps_only_the_stock_warning`.
- `req_op_04_the_crankshafts_ends_are_faced` (the public CrankShaft model).
- `req_op_21_an_off_axis_end_whose_tab_meets_it_is_faced_down_to_the_tab_as_before` (case (a)).
- `req_set_09_req_op_21_an_end_whose_tab_misses_it_is_warned_as_cut_free` (case (c)).

`openchuck-app`, `tests/off_axis_ends.rs` (case (c) in a saved project):

- `req_set_09_req_gc_16_a_saved_project_whose_tab_misses_an_end_asks_for_acknowledgement`: a project saved with no Cutting direction, opened again and made, lists the tab's possible collision; writing is refused until it is acknowledged, then writes.
- `req_set_09_the_tab_warning_is_given_with_face_free_ends_off`.

Both app tests were seen failing with the case (c) warnings switched off, and passing with them.

**Results** on the draft head, from `main` `b99af9b8` (0.73.0), version 0.74.0 provisional, in this container (Linux, Rust 1.97.0):

- `cargo test --release --locked -p openchuck-path`: 153 unit, 939 integration and 1 doc test pass.
- `-p openchuck-app`: 317 unit (1 ignored) and 636 integration tests pass.
- `cargo fmt --all`, `cargo clippy --workspace --all-targets --locked`, `cargo check --locked` in `fuzz/`, and `cargo xtask deps` (with messages and topics), `licences`, `records` and `trace` pass, as does `changelog check-pr`.
- `cargo-semver-checks` and `cargo-nextest` are not installed here; CI runs them. Each new test was timed uninstrumented with the dev-profile test binary: the 14 path tests take 1.3 s to 8.8 s each, and the two app tests about 2 s each in release. None is long-running.

**Coverage** (`openchuck-path`, the 14 new path tests alone, instrumented): of the changed lines, `faces.rs` runs 172 of 180 and `finishing.rs` 295 of 332; the one-line caller changes in `helical.rs`, `axial.rs` and `rest.rs` are run. The lines not reached in that run are:

- the round-end turn and its note, which the existing free-end tests run (not part of the instrumented set), with the ring wording of `faced_words` and `cut_words`;
- the whole turn inside `arcs` where the face is found at every sampled angle in a layer (item 2): on an end that does not reach round the axis some angles miss the face, so the synthetic and CrankShaft cases do not reach it; it is kept so that such a layer is not cut as one arc of 360° with an overlap;
- defensive branches (NFR-07): an empty or unbounded surface, a candidate with no line near it, a line whose section is not found, no sampled angle finding the face, a stock of no known X extent, and the cancellation check.


Not done: no air or wood cut. Generated G-code remains the user's to verify before it is run on a machine (NFR-12).
