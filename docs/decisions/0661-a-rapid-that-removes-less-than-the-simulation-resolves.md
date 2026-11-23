# 0661: A rapid that removes less than the simulation resolves (SIM-03, SIM-05, SIM-07, GC-16, UI-12, NFR-12; issue #801; lane SIM-FINISH)

- **Status:** Accepted by the project owner on 22 Nov 2026, relayed by the coordinator (decision 0049): option C. Proposed by lane SIM-FINISH the same day. Option C adds a public field to `openchuck-sim`'s `RapidCut`, so it is built in the lane's second pull request (lane SIM-FINISH, branch `claude/lane-sim-finish-rapid-depth`), after the 0.55.0 crates.io publish, with the workspace minor bump to 0.57.0 that the coordinator sequences; the lane's first pull request (#837) carried this record only.
- **Date:** 22 Nov 2026.
- **Context:** SIM-03 (material removal), SIM-07 (the simulation's cell, 0.1 mm by default), GC-16 and UI-12 (warnings acknowledged before export), NFR-12. [Decision 0025](0025-slice-7-simulation.md) item 9 (`openchuck-cli simulate` fails a run in which a rapid removes material), [decision 0536](0536-message-levels-operation-reference-and-setting-links.md) (a rapid that removes material is a warning to acknowledge, the owner's answer (e)), [decision 0022](0022-sp-6-simulation-stock.md) (the dexel stock). It changes no requirement and no architecture.
- **Open items:** OI-2166 (the owner's choice: option C), closed by the second pull request, which builds it; OI-2167 (the Periscope approach that removes 0.016 mm³, a toolpath question apart from this one).

## What the issue reports

`openchuck-cli simulate` fails a project (nightly 0.44.0) on seven rapid moves that remove material: two approaches of an axial finishing operation (0.016 and 0.001 mm³) and five rapids of a simultaneous finishing operation that each report **0.000 mm³**. Those five remove something below the printed precision and still fail the run, and nothing says why.

## How a rapid cut is found today

- The simulator cuts each rapid move on its own and reports it when it removed more than `VOLUME_EPSILON`, 1e-9 mm³ (`crates/openchuck-sim/src/simulation.rs`). That is a rounding guard, not a resolution: any overlap of the tool with the simulated stock counts.
- The stock is a grid of columns along model Z, one per cell (0.1 mm by default), each cut exactly along its own line (decision 0022). A volume of 0.0004 mm³, printed as 0.000, is the tool reaching about 0.04 mm into one column at the default cell (0.0004 / 0.01 mm²), or less spread over several.
- The command line prints the volume to three decimals and counts each rapid cut as a failure; the application lists it as a warning to acknowledge before export (decision 0536). Neither says how deep the tool went.
- How deep a rapid may go before it matters is not set anywhere. The simulated stock is itself only as good as the cell across the columns and the spacing of the poses each earlier move was sampled at, so a contact of a few hundredths of a millimetre can come from how the stock was sampled as much as from the path.

## Options

- **A. Keep counting every overlap (today), and say what it is.** Print the volume with enough digits to be non-zero ("less than 0.001 mm³") and add the depth: "move N is a rapid that reaches 0.04 mm into the stock (0.0004 mm³)". Still a failure and still a warning to acknowledge. Safest reading of a rapid in material; the issue's five rapids still fail the run.
- **B. A volume threshold of one cell.** A rapid that removes less than one cell's volume (the cell cubed: 0.001 mm³ at 0.1 mm) is listed as information ("within the simulation's resolution"), not a failure and not a warning; above it, as today. Simple, but volume is a poor measure of a contact: a long rapid grazing many columns by a hundredth adds up to more than a short one a millimetre deep in one.
- **C. A depth threshold of one cell (recommended).** The simulator records, for each rapid, the deepest it went into the stock along any column (the largest length of material removed from one column) as well as the volume. A rapid that goes no deeper than one cell (0.1 mm at the default; the user's finer cell gives a finer threshold) is reported as information, with its depth and volume, and neither fails `simulate` nor asks for an acknowledgement; deeper, it is a failure and a warning to acknowledge, as today, with its depth in the text. Messages and the command line give depth first.
- **D. A fixed depth independent of the cell** (for example 0.01 mm, or the operation's tolerance). Independent of the user's resolution, but below the cell it claims more than the simulation can show, and the operation's tolerance is not about rapids.

## Recommendation

Option C. Depth is what a user can judge a contact by, and one cell is the finest the simulation can honestly resolve across the columns (SIM-07): below it a report can come from how the stock was sampled. The rapid is still listed, with its depth, so nothing is hidden; only the failure and the acknowledgement depend on it. A user who wants to see smaller contacts sets a finer cell. Generated G-code stays the user's to verify before it runs on a machine (NFR-12), and a rapid listed as information is no statement that the move is safe.

## Option C, as built (the owner's choice)

- **`openchuck-sim`** (`dexel.rs`, `simulation.rs`). The cut of a rapid's own batch (`DexelStock::cut_deepest`, crate-private) gives the volume and the most material removed from any one column: the largest over the columns, so it does not depend on the thread count (NFR-06). `RapidCut` gains `depth` (that length, mm) and `cell` (the finer of the stock's two cells, mm), and the method `deeper_than_cell` (`depth > cell`). `GroupResult::rapid_cuts` still lists every rapid that removes more than 1e-9 mm³. This is a breaking change for a struct literal of `RapidCut`, so it takes the workspace minor bump to 0.57.0 (decision 0048; 0.56.0 went to #840, which merged first), with changelog entries in `openchuck-sim`, `openchuck-app` and `openchuck-ui`.
- **`openchuck-app`** (`pipeline.rs`, `export.rs`, `overview.rs`). Each operation's rapids are split by `deeper_than_cell`:
  - Deeper, a warning of the run, in `GroupInfo::rapid_cuts` (now `Vec<RapidCut>`), to acknowledge in Messages and the export checks as before: "Rapid cut (operation 'X'): move N is a rapid that goes D mm into the stock, deeper than the simulation's cell of C mm, and removes V mm³ of material".
  - No deeper, a note of the run (information, nothing to acknowledge): "Rapid cut (operation 'X'): move N is a rapid that touches the stock D mm deep, within the simulation's cell of C mm, and removes V mm³ of material".
  - A volume under 0.0005 mm³ reads "less than 0.001 mm³", never 0.000.
- **`openchuck-cli simulate`** counts only the deeper rapids as problems and prints every rapid with its depth and volume ("rapid move N goes D mm into the stock[, within the C mm cell] and removes V mm3, ending at (x, y, z)").
- **`openchuck-ui`**: the Results tab lists only the deeper rapids as problems and counts only those as rapid cuts on the BY OPERATION line ("Clean" otherwise); a shallower one is the run's note.
- **The user guide**: `concepts/simulation.md`, `reference/messages.md`, `reference/command-line.md` and `reference/operations.md` say what is reported and when it counts.

## The Periscope approach (OI-2167, separate)

The axial finishing approach that removes 0.016 mm³ is 1.6 mm deep on one column at the default cell, or more shallow over several: deeper than one cell under any of the options except D at a large value, so it stays a failure. Whether it still happens on a current build needs the project in the issue, which is the owner's; this lane has not opened it (the coordinator was asked). A run on the public baluster example (Rough, then axial and simultaneous finishing with the Z1 ball 3.175, `openchuck-cli simulate`) on the lane's build found no rapid that removes material.

## Verification

- Read: `crates/openchuck-sim/src/simulation.rs` (`VOLUME_EPSILON`, the rapid cut), `crates/openchuck-sim/src/dexel.rs` (the columns), `apps/openchuck-cli/src/main.rs` (`simulate`'s count and print), `crates/openchuck-app/src/export.rs` (the warning text).
- Run: `openchuck-cli simulate` on a copy of `testdata/examples/baluster/baluster-project` with its Finish replaced by an `axial_finishing` and a `simultaneous_finishing` operation (tool `c0de0001-0000-4000-8000-000000000204`, defaults), on the lane's release build on Linux: no rapid cut, 28 s.
- Not verified: the issue's own project (OI-2167; this session's permission check refused the download the owner allowed).

### Option C, as built (the lane's second pull request)

- Tests from this record's option C, written before the code; they did not build against the old API (`RapidCut` had no depth):
  - `openchuck-sim` `simulation::req_sim_03_req_sim_07_a_rapid_no_deeper_than_one_cell_is_within_the_resolution`: a rapid along X over a 30 mm bar, 0.03 mm into it at a 0.2 mm cell, is listed with a depth of at most 0.03 mm and is within the cell; one 0.4 mm deep is not.
  - `openchuck-sim` `simulation::req_sim_03_req_sim_07_a_finer_cell_gives_a_finer_threshold`: 0.15 mm into the bar is within a 0.2 mm cell and beyond a 0.1 mm one.
  - `openchuck-app` `export::tests::req_sim_03_req_gc_16_a_rapid_reads_with_its_depth_first_and_within_a_cell_as_a_contact`: the two texts, and "less than 0.001 mm³".
  - `openchuck-cli` `tests::req_sim_03_req_sim_07_only_a_rapid_deeper_than_one_cell_fails_the_simulation`.
  - Updated to the new wording: `overview::tests::req_gc_16_req_ui_12_a_rapid_that_removes_material_is_in_messages_to_acknowledge`, `export_dialog::req_gc_16_a_rapid_that_cuts_reads_as_a_possible_collision_at_writing`, the UI's `run_panel::tests::req_nfr_12_a_real_warning_is_never_merged_into_the_feed_summary`.
- `cargo nextest run -p openchuck-sim -p openchuck-app -p openchuck-cli -p openchuck-ui`: 1,782 passed, 3 skipped, 1,850 s (debug, uninstrumented, 4 cores, Linux); the reference jobs' end-to-end `simulate` runs and golden programs are unchanged. `cargo fmt`, `cargo clippy --workspace --all-targets`, `cargo xtask deps`, `licences`, `records` and `trace` pass.
- Coverage of the changed lines on the new tests: all covered but the two loops in `pipeline.rs` that turn a simulated run's rapid cuts into the run's warnings and notes (no test project makes a rapid that cuts) and two lines of `dexel.rs` (the plain `cut` and the empty batch, run by the crate's other tests).

