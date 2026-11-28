# 0708: The part's deviation colours are kept while the comparison and the part are unchanged (SIM-05, NFR-04, NFR-06; issue #699; lane SIM-UX)

- **Status:** Accepted by the project owner on 26 Nov 2026, relayed by the coordinator (decision 0049). Proposed by lane SIM-UX on 24 Nov 2026; the coordinator released the pull request on the owner's answers of 24 Nov 2026 as one that closes on its tests (no owner check).
- **Date:** 24 Nov 2026.
- **Requirements:** SIM-05 (the deviation colours), NFR-04 (never block, use the cores well), NFR-06 (the same result whatever the history).
- **Amends:** nothing. It builds what [decision 0217](0217-band-visibility-and-the-kept-comparison.md)'s Consequences left as OI-881. [Decision 0492](0492-deviation-colours-on-the-part-model.md) (a part file changed on disk is refused) holds unchanged.
- **Open items:** closes OI-881; adds OI-2279 (the owner's acceptance of this record), closed by the acceptance of 26 Nov 2026.

Numbering: decision 0708 and OI-2279, assigned by the coordinator at release (drafted as "SIM-UX-B" and OI-9952). No project schema number and no library-family change.

## Context

With "Deviation colours" on and Part shown, Simulate colours the part model by its measured final result (decision 0492). The job that works the colours out, `Application::start_deviation_bands`, ran again for every new run: it read and placed the part, matched every part triangle to the comparison's samples (`triangle_worst`, `triangle_samples`) and split the triangles that hold more than one band into pieces (decision 0514). A run that changes nothing the comparison depends on takes the comparison kept with the cached final stock (decision 0217 item 2), so the colours came out the same each time.

Measured on `main` `6b039e5` (release build, 4-vCPU cloud container, uninstrumented): 0.28 s each run on the baluster tutorial project (`testdata/examples/baluster/baluster-project`, 1.58 M counted samples) and 0.55 to 0.61 s each run on the synthetic carving test piece (`CarvedRelief::STANDARD`, a 454k-triangle part, 2.61 M samples).

## Decision

1. **The comparison is shared, not copied.** The comparison kept with a cached stock (`SetupState`, decision 0217) is held behind a shared pointer, and the run's report (`SetupSimulation::deviation`) holds that same pointer. A run that takes the kept comparison therefore holds the identical comparison, which also saves copying every sample into each run's report.
2. **The colours are kept with what they were worked out from.** For each setup, the application keeps the last colours with: the comparison, by identity, held weakly so that kept colours never keep an old comparison alive; the part's model and the setup as the run had them, which fix the part's triangles and its held regions and tabs; and the cell. A request whose comparison, part and cell are all the same gets the kept colours; any other request works them out again and replaces them.
3. **A part file changed on disk is still refused.** Before anything kept is used, the part file's content hash is read and compared with the model's, so the refusal of decision 0492 is unchanged.
4. **Nothing shown changes.** The colours given from the kept entry are the ones working them out again would give, because they come from the same comparison, part and cell.

## Alternatives considered

- **Key the colours by a hash of the comparison's samples.** This is exact without sharing, but hashing 2.6 M samples costs a large part of what it saves. Rejected.
- **Keep the colours of every run.** This needs more memory for no gain: one entry a setup covers rerunning with nothing changed.

## Consequences

- `openchuck-app`: `SetupSimulation::deviation` is `Option<Arc<Deviation>>` (not a library-family crate, so no version bump; its changelog says so). `Application::start_deviation_bands` keeps and reuses the colours; `Application::part_colours_made` (hidden) counts the times they are worked out, for tests.
- `openchuck-ui`: the Results panel reads the comparison through the shared pointer (two lines).
- Memory: one set of part colours a setup, which was already held by the Simulate panel for the run on screen. Sharing the comparison saves one copy of every sample for each run that reuses it.

## Verification

Linux cloud container, 4 vCPU, no GPU; branch `claude/lane-sim-ux-b`, rebased on `main` `361613e`.

**Test, written first from SIM-05 and NFR-04 and watched failing.** With the kept colours turned off, it failed at "the colours were kept" (worked out 2 times, not 1). With them on, it passes.
- `playback::req_sim_05_req_nfr_04_part_colours_are_kept_while_the_comparison_and_part_are_unchanged` (`openchuck-app` integration; 19 to 32 s alone, 52 s in the full run with every other test). It checks:
  - a second run with nothing changed shares the first run's comparison and gets equal colours without working them out;
  - a coarser cell (a new comparison) works them out again;
  - the part placed elsewhere works them out again, with other values;
  - an older run asked again gets its own colours, not the newer run's.
- The existing guards pass unchanged: `playback::req_ui_11_deviation_bands_refuse_a_part_file_changed_since_the_run` (a file edited on disk is still refused, kept colours included) and the UI's `playback_panel::tests::req_sim_05_a_part_that_cannot_be_read_leaves_the_cut_part_coloured`.
  - The UI test found a fault in the first version: a deleted part file was reported as changed rather than unreadable. Now a file that cannot be read is left to the normal reading, which says why, and nothing kept is used.

**Suites.**
- `cargo nextest run -p openchuck-app -p openchuck-ui -p openchuck-cli` before the rebase: 1,746 tests, 1,745 passed. The one failure was the UI test above, fixed in this pull request.
- After the fix, the 84 colour and playback tests of both crates passed.
- After the rebase on `361613e`: the simulation, colour and playback tests of the three crates; `cargo fmt --all --check`; `cargo clippy --workspace --all-targets` with no warnings; `cargo xtask deps`, `licences`, `records` and `trace`, all ok.

**Timings, release build, uninstrumented.** "Part colours" is `start_deviation_bands` on each run's result.

| Project | `main` `6b039e5`, every run | This branch, first run | This branch, a run with nothing changed |
| --- | --- | --- | --- |
| Baluster tutorial project | 0.28 s | 0.262 s | 0.000 s |
| Carving test piece | 0.55 to 0.61 s | 0.572 s | 0.009 to 0.010 s (reading the 23 MB part file's hash) |

**Coverage.** `cargo llvm-cov nextest --profile coverage -p openchuck-app` on the three colour tests, then `cargo llvm-cov clean --workspace`. The new lines are covered except three:
- the error branch of making the key (serialising the model and the setup cannot fail for these types; NFR-07 defensive);
- the existing re-check of the hash after the part is read (a file changed between the two reads);
- the `Debug` impl of the kept colours.

**Not verified:** the owner's projects (none used) and the GPU drawing (unchanged).
