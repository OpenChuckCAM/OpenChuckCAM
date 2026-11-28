# SIM-UX-A: Only the final cut part is coloured, its colours are prepared in the background, and a lozenge shows the wait (SIM-04, SIM-05, NFR-04; issues #454 and #538; lane SIM-UX)

- **Status:** Proposed by lane SIM-UX on 28 Nov 2026, built to the project owner's decisions of 24 and 26 Nov 2026, relayed by the coordinator (decision 0049): Q1 B with the lozenge; decision A (colour only the final cut part automatically, interim cut parts and playing frames plain); the final part's colours prepared in the background after every run, even with colours off; Q3 B where the background setting is off; the setting "Prepare deviation data in the background", on by default from 16 GB. The owner's acceptance is OI-9960, after the owner check below.
- **Date:** 28 Nov 2026.
- **Requirements:** SIM-04 (the stock at any point of the program), SIM-05 (the deviation colours), NFR-04 (never block, use the cores and the memory well).
- **Amends:** [decision 0238](0238-simulate-display-and-operation-navigation.md) item 4 (values projected onto the cut part at every playback position: now only onto the cut part at the end of a setup's program); [decision 0322](0322-simulate-playback-frame-cost.md) item 8 (the cut part, its colours and its picks unchanged: the cut part on screen is now plain before the end, and a still frame first shows plain); [decision 0514](0514-deviation-colours-cover-only-their-area.md) and decision 0322 item 6 for playing frames (a frame shown while playing was coloured by whole triangles: it is now plain). The [UI specification](../ui-design/ui-specification.md) section 11.5 (the stock, "Deviation colours", the waiting lozenge), section 13 (the setting) and section 16 (Esc) are edited in the same pull request.
- **Open items:** adds OI-9960 (the owner's acceptance of this record, after the owner check) and OI-9961 (the rest of the owner's decisions of 26 Nov 2026, built as A1b: "Generate current deviation data", points A and B, the compact form with refinement on screen and the 1 GB "Memory for deviation data" cap). Builds on [decision 0708](0708-part-colours-kept-with-their-comparison.md) (the part's colours kept while the comparison and the part are unchanged).

Numbering: provisional. The coordinator assigns the record number and the OI numbers at release, in merge order. No project schema number: the setting is a per-user settings field (read as its default from an older file), not a project format change. No library-family change.

## Context

Measured on 24 Nov 2026 (release build, 4-vCPU cloud container, public baluster tutorial project and the synthetic carving test piece `CarvedRelief::STANDARD`):
- A step to an operation's end with the cut part shown took 2.7 to 7.6 s. Of that, the cut part's shape took 0.04 to 0.05 s. The rest was its deviation colours, drawn in pieces (decision 0514), and the picker for the probe. They were made for every still frame, even with "Deviation colours" off.
- "Prepare playback" built the cut part's sample index every time: 1.46 s of its 1.5 s on the carving with playback information kept with generation, and about 150 MB on a large project (#538). The index is used only for the colours and the probe.
- The colours on a cut part before the end were not a measurement of that stock. They were the final comparison's values, painted onto the shape on screen (decision 0238 item 4). The owner found them unhelpful and chose to colour only the final cut part (decision A, 26 Nov 2026).

## Decision

1. **Only a setup's final cut part is coloured.** `Application::start_stock_surface(playback, cursor, colours)` gives the still frame at `cursor`. With `colours`, and only when the stock at `cursor` is the final one of its setup (every move of the setup's last group done), it carries the final-result colours in pieces, with the picker. Any earlier cut part is plain: it keeps the picker while colours are on, so values on the part model are still hidden behind it (decision 0492 item 5). Without `colours` the still frame is the mesh alone. A frame shown while playing (`start_playing_stock_surface`) is always plain, with no picker. `StockPlayback::is_final` says whether a cursor shows a final cut part.
2. **The final coloured cut part is made once and kept** with the playback, so turning colours off and on, or coming back to the end, costs nothing.
3. **The sample index is built when it is first needed** (#538): the first time colours are made for a setup, not by preparing playback.
4. **The new cut part shows at once** (the owner's Q1 B). When playback is still at a new point, the interface first shows the plain cut part (about 0.05 s), then asks for the still frame; with Deviation colours on at the end, its colours and probe follow.
5. **The waiting lozenge** (the owner's Q1 B, with the states and placement in specification section 11.5): "Colouring the cut part… N%", "Colouring the part… N%" or "Preparing playback… N%", after 0.3 s, at the top centre of the view, under the out-of-date banner or in its place. "Stop" (and Esc while no job runs that Esc would cancel, no tool window is open and nothing is being picked) stops the colouring and turns Deviation colours off. Preparing playback has no Stop.
6. **"Prepare deviation data in the background"** (Settings): on by default from 16 GB (the rule of decision 0333), off below. On, after each simulation and while no generation, simulation, posting or playback preparation runs, one job on one thread (`Application::start_preparing_deviation`) builds the sample index and the final coloured cut part of each setup, even with colours off. The part's own colours are then made and kept by the application (decision 0708). Anything else starting stops the work; it starts again when that ends; a new run discards it. When the user waits for those colours in the foreground, the background work stops and the foreground job uses every thread. Off, nothing is made until Deviation colours is ticked.
7. **Wording.** The colour scale's title on the cut part is "Cut part at the end · measured against the finished part". The "Deviation colours" tooltip says that the cut part at the end of the program, and the part, are coloured. While the cut part on screen is one before the end, a line under the tick box says "Colours show the finished result at the end of the program."

## Alternatives considered

- **Keep colouring every still frame, only faster.** Rejected by the owner (decision A): the colours before the end are not a measurement of that stock.
- **Build the sample index only on the first colours-on, always** (Q3 B as first answered). With background preparation on, the index is built in the background after every simulating run instead, so colours show at once; Q3 B applies when the setting is off.

## Consequences

- `openchuck-app` (not semver-checked; its changelog says so): `start_stock_surface` takes `colours` and, with `start_playing_stock_surface`, returns `Arc<StockSurface>`; `StockPlayback::is_final`, `colours_ready` and the hidden `samples_built`; `Application::start_preparing_deviation` (job "prepare deviation colours", in the Jobs tab as "Preparing deviation colours"); `SimulateSettings::deviation_in_background`.
- `openchuck-ui`: the playback panel shows the plain cut part first, follows the wait and runs the background work; the shell draws the lozenge and the setting.
- Memory: the sample index (about 110 MB on the baluster, 180 MB on the carving) and one coloured final cut part per setup (about 400 MB on the baluster in the full form) are held while the result is shown when the background setting is on, or once colours are turned on. A1b (OI-9961) adds the compact form and the cap.
- Interim colours are gone. A1b's "Generate current deviation data" measures the stock at a chosen point afresh (the owner's option Y).

## Owner check

On your laptop (a real GPU and window manager), with the public baluster tutorial project (`testdata/examples/baluster/baluster-project`, a copy), Simulate it once.
1. **Plain steps are quick.** With Deviation colours off, step to each operation's end (Shift →). Expect the new cut part within a moment each time, and no lozenge.
2. **Colours only at the end.** Tick Deviation colours. At an operation's end before the last, expect the plain cut part and the line "Colours show the finished result at the end of the program." under the tick box. Go to the end of the program (drag the playhead to the end): expect the cut part coloured, and the probe working on it.
3. **The lozenge.** Turn Settings › "Prepare deviation data in the background" off, simulate again, tick Deviation colours and go to the end. Expect "Colouring the cut part… N%" at the top of the view (under the out-of-date banner if one shows) with a Stop button, and the plain cut part shown at once underneath. Press Stop: expect the lozenge gone, Deviation colours unticked, the cut part plain. Do it once more with Esc instead of Stop.
4. **Background.** Turn the setting on (or back to the computer's default, on from 16 GB), simulate again, wait a few seconds without touching anything, then tick Deviation colours at the end: expect the colours at once, with no lozenge. The Jobs tab shows "Preparing deviation colours" while it works.
5. **Playing.** Play at 10× and at 100×. Expect the cut part to keep close behind the tool (the cut-part bar says how far), plain while playing, and coloured again when paused at the end.
6. **Pass:** each step as written. **Fail:** say which step, what you did and what happened, with the time if a wait felt long.

Generated G-code is always yours to check before running it on a machine; nothing here changes the G-code.

## Verification

Linux cloud container, 4 vCPU, no GPU; branch `claude/lane-sim-ux-a`, from `5ffbedb6` (#907's head).

**Tests, written first from the owner's decisions** (they did not compile against `main`'s API, so they failed first by not building rather than by an assertion):
- `openchuck-app` integration: `playback::req_sim_05_req_nfr_04_only_the_cut_part_at_a_setups_end_is_coloured`, `playback::req_sim_05_req_nfr_04_the_sample_index_and_the_final_colours_are_made_once_when_needed`, `playback::req_sim_05_req_nfr_04_the_final_colours_are_prepared_in_the_background` (15 to 23 s each, uninstrumented).
- `openchuck-ui`: `playback_panel::tests::req_sim_04_req_sim_05_the_new_cut_part_shows_at_once_and_its_colours_follow`, `req_sim_05_req_ui_09_the_lozenge_shows_a_wait_and_stop_turns_colours_off`, `req_sim_05_req_nfr_04_the_final_colours_are_prepared_while_idle` (17 to 30 s each).
- Changed to the new decision: `reach::req_sim_04_req_sim_05_only_a_still_frame_draws_the_cut_part_in_pieces` (a playing frame is now plain) and `playback_panel::tests::req_sim_05_the_stock_and_the_deviation_colours_come_from_jobs` (an interim cut part is plain; the end is coloured).

**Suites, timings and coverage:** to be completed before the pull request opens.

**Not verified:** the owner's projects (none used); the GPU drawing and the lozenge on a real display (the owner check).
