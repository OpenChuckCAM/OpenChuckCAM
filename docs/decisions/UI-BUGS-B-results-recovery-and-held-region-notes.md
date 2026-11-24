# UI-BUGS-B: A refused Generate shows only its refusal, a deleted project's recovery line can be forgotten, and the held-region note is information shown in the setup too (UI-12, UI-25, PRJ-08, SET-09, NFR-07, NFR-12; issues #766, #681, #774, with #607, #874, #767, #609 and #824; lane UI-BUGS)

- **Status:** Proposed by lane UI-BUGS on 24 Nov 2026, built in the lane's second pull request as the project owner answered the lane's questions on 24 Nov 2026, relayed by the coordinator (decision 0049): Q4 option A (#766), Q5 option A (#681), Q7 option A (#774) as information, not a warning, with the setup row the owner approved with one wording edit. The owner has not yet accepted the record itself (OI-9984).
- **Date:** 24 Nov 2026.
- **Requirements:** UI-12 (results and messages), UI-25 (workholding in the interface), PRJ-08 (recovery), SET-09 (held regions), NFR-07 (clear messages), NFR-12 (generated G-code is the user's to verify), GC-07 (a run's output follows its project).
- **Amends:**
  - [decision 0519](0519-results-in-the-lower-dock.md) (accepted), for a failed run: the Results tab shows the refusal alone;
  - [decision 0460](0460-recovery-offered-only-for-the-opened-project.md) (accepted) item 1 ("Nothing is deleted. The line stays until that project is opened"): the line of a project whose folder is gone can be forgotten;
  - [decision 0536](0536-message-levels-operation-reference-and-setting-links.md) (accepted) build step 2, the owner's row P65 of the message review (`docs/message-review/path.md`, KEEP-WARNING): the held-region note is information;
  - the held-region note's words quoted in [decision 0161](0161-tools-keep-clear-of-the-gripped-length.md) items 3 and 7 and [decision 0551](0551-the-held-regions-own-end-is-not-a-shoulder.md), which stay as history.
- **Open items:** adds OI-9984 (the owner's acceptance of this record).

Numbering: provisional. The coordinator assigns the record number and the OI number at release, in merge order. No project schema number: nothing stored changes. Library family: `openchuck-path` changes the held-region note's level and words, with no public API change and no version bump (its `CHANGELOG.md` says so).

## Context

Issues of milestone 2026.2, each checked on `main` (`aa99eab`) with the synthetic cube test project or the public baluster example, never a private project.

- **#766.** After a refused Generate, Results showed the error, then "The project has changed since this run. Generate again.", then the previous run's summary (operations, moves, time) and all its warnings, as if they belonged to the refused run; the tab's count included the old warnings, and Copy all copied the old summary with the error.
- **#681.** A project deleted after an unexpected close left a Messages warning at every launch ("recovery data was kept for a project last in …, but it is not there now"), with only Locate folder. The recovery data lived in the deleted folder; only this computer's note of it was left, and nothing removed it.
- **#774.** The held-region note, "Nothing within one tool radius of the held region … is cut", understated the zone: every tool keeps its largest radius and 0.5 mm from the grip (`HELD_CLEARANCE`, decision 0161), and an inside corner beside the grip can keep more. It was a warning, so an operation that did what the setup asked was marked amber.
- **#607 and #874.** Hiding a toolpath with its eye stored the choice in the project (decision 0169 item 6), and Results and the output steps compared whole projects, so Results said the run was out of date, and Write G-code was refused with "the project changed after the files were made" (#874, sev2), although the export dialog, which compares with `Project::same_output`, called the files current.
- **#767.** The head and tail zone lines and the tab rows' zone names were worked out from the stored held regions, so a grip typed before Apply left a stale zone.
- **#609.** The Geometry section's indented rows (the Cut short / Warn only / Ignore row, the limits, warnings and notes) started with a space inside a wrapping row, so a wrapped line began left of the label column.
- **#824.** The machine profile comparison called any difference in the rotary reference point "axis height"; a profile whose Y differed was said to differ in its axis height.

![The held region section with its Kept clear row](UI-BUGS-B-held-region-kept-clear.svg)

## Decision

1. **A refused Generate shows only its refusal (#766, the owner's answer A).** The Results tab shows the refusal and nothing of the previous run: no summary, no lines, no "has changed" line, and the tab's count is the refusal alone; Copy all copies the refusal. The view keeps the previous paths, marked out of date as before.
2. **A deleted project's recovery line can be forgotten (#681, the owner's answer A).** The line's Locate dialog has **Forget this project**: this computer stops looking for that project, and its line goes for good. Only this computer's note is removed; nothing else is deleted (the recovery data was inside the project's folder). A folder that still holds that project's recovery data is not forgotten: that project is opened to recover or discard it.
3. **The held-region note is information, and says what the tool keeps (#774, the owner's answer A).** An operation whose path a held region cut short gets, at the information level: "Held region (operation 'Finish'): no part of the tool comes within 0.500 mm of the held region at the start of the stock, so the surface beside it is not cut there, and corners beside it may keep more. The setup's Workholding section shows where the tools keep clear." It does not make the operation a warning, is not counted as one, and is not acknowledged at export. The note on holes not drilled beside a held region stays a warning.
4. **Each held region says where the tools keep clear (#774, the owner's approved design).** In the setup's Workholding and tabs section, each held region's block has a read-only row after Held span, "Kept clear: to X −1.500 mm" (at the start; "from X …" at the end), from the widest tool of the setup's operations (its cutting profile or its shank) and the 0.500 mm every tool keeps, as the operations do, and under it: "No tool comes within 0.500 mm of the grip. Corners beside the grip may keep more material." It follows the draft as it is typed. With no tool yet it says "Depends on the tools".
5. **Hiding a path changes no output, everywhere (#607, #874).** Results, the G-code preview and the output steps compare a run's project with `Project::same_output`, as the export dialog already did, so hiding or showing a path neither makes the run out of date nor refuses Write G-code.
6. **The zone lines follow the draft (#767).** The head and tail zone lines and the tab rows' zone names use the draft's held regions; the default tabs offered still follow the stored held regions (they are worked out by the part's facts job).
7. **Wrapped rows stay in the value column (#609).** Every indented row of the Geometry section wraps inside the value column.
8. **The comparison names the coordinate (#824).** A difference in the rotary reference point reads "rotary reference X", "rotary reference Y" or "axis height" (its Z), as the machine editor names them.

## Questions for the owner, and the answers

Asked on 23 and 24 Nov 2026 and answered on 24 Nov 2026, relayed by the coordinator.

- **Q4 (#766).** A: show only the refusal. B: keep the previous run's results under "From the previous run". **Answer: A.**
- **Q5 (#681).** A: a Forget action on the line. B: drop the line automatically once the folder is missing. C: dismiss it for this session only. **Answer: A.**
- **Q7 (#774).** A: reword now, after #866. B: later. **Answer: A, as information rather than a warning, and shown in the setup's held region section too;** the owner approved the design with the row's note "No tool comes within 0.500 mm of the grip. Corners beside the grip may keep more material." and without per-tool distances.

## Consequences

- A project whose only warning was the held-region note now shows **Nothing to report** instead of **Needs attention**, and the Results and Messages warning counts drop by one per such operation. The user guide's pages and screenshots that showed the note as a warning, or counted it, are updated and captured again.
- Results never presents a refused run with the previous run's figures.
- The command line prints the held-region note as `note:`, not `warning:`.

## Verification

Linux container, 4 cores; tests run uninstrumented with `cargo nextest`, on the branch rebased on `main` `19de286` (after #856). The commit is the pull request's head.

**Tests written first from the issues and the owner's answers** (times from the full run):

| Test | What it checks | Seen to fail first | Time |
| --- | --- | --- | --- |
| `shell::results_tab_tests::req_ui_12_req_nfr_12_a_refused_generate_shows_only_the_refusal` | item 1 (#766): after a refused Generate, Results shows the refusal, no summary, no "has changed" line, a count of 1 | yes, on `main` | 2.5 s |
| `shell::open_recovery_tests::req_prj_08_a_deleted_projects_recovery_line_can_be_forgotten` | item 2 (#681): the Locate dialog's Forget this project removes the line for good | yes, on `main` | 0.3 s |
| `checkpoints::forget_tests::req_prj_08_forgetting_a_deleted_projects_recovery_removes_only_its_note` | item 2: only this computer's note goes; another project's stays; forgetting twice changes nothing | written with `forget` | 0.01 s |
| `checkpoints::forget_tests::req_prj_08_a_folder_that_still_holds_its_recovery_is_not_forgotten` | item 2: a folder that still holds the project's recovery data is refused | yes, with the check removed | 0.01 s |
| `strategy::tests::req_set_09_a_pass_is_cut_short_where_it_would_reach_over_a_held_region` and `req_set_09_messages_name_the_held_region_by_its_end_of_the_stock` (openchuck-path) | item 3 (#774): the note is information, with 0.500 mm and the pointer to the setup | yes (they asserted the old warning) | 0.01 s |
| `workholding_view::req_set_09_req_ui_25_the_held_regions_say_where_the_tools_keep_clear` (openchuck-app) | item 4: "to X …" and "from X …" from the widest tool's reach and 0.500 mm | written with `kept_clear` | 0.02 s |
| `workholding_panel::tests::req_set_09_req_ui_25_a_held_region_says_where_the_tools_keep_clear` | item 4: the Kept clear row and the approved note in the section | yes, on `main` | 0.3 s |
| `shell::results_tab_tests::req_ui_12_req_sim_01_hiding_a_path_does_not_mark_the_run_changed`, `export_dialog::req_gc_15_req_sim_01_hiding_a_path_after_a_run_does_not_refuse_writing`, `export_panel::write_feedback_tests::req_gc_15_req_sim_01_write_after_hiding_a_path_writes_the_files` | item 5 (#607, #874): hiding a path neither marks the run changed nor refuses Write G-code | written first | 2.6 s, 2.1 s, 0.6 s |
| `workholding_panel::tests::req_ui_25_req_set_09_the_zone_lines_follow_the_draft_before_apply` | item 6 (#767) | written first | 0.3 s |
| `shell::inspector_layout_tests::req_ui_07_req_ui_16_the_avoid_choice_row_wraps_inside_the_value_column` | item 7 (#609): no wrapped line starts left of the value column | written first | 0.6 s |
| `setup::short_block_tests::req_set_01_req_set_10_profile_comparison_names_the_rotary_reference_coordinate` | item 8 (#824) | written first | 0.03 s |

**Existing tests changed to the owner's answers**, each because it encoded the old behaviour:
- #766: `run_panel::tests::req_ui_12_the_label_count_is_warnings_and_errors_with_a_group_once` (a failed run counts its refusal alone), `req_ui_09_req_nfr_04_the_results_tab_groups_the_notes_once_per_run_and_change` (no grouping after a failed run) and `req_ui_12_req_nfr_12_the_results_tab_shows_the_run_and_project_as_they_are` (the earlier run's lines are not copied after a failure).
- #607: `export_panel::preview_tests::req_gc_15_a_preview_refused_for_a_changed_project_is_made_again_when_it_is_back` became `req_gc_15_req_gc_07_the_stock_simulation_switch_does_not_refuse_the_preview`. The switch changes no G-code, so the preview is keyed on `same_output`, as writing is.
- #774: the held-region tests in `openchuck-path` (`facing_validation`, `helical_faces`) and `openchuck-app` (`held_region`) expect the note as information.

**Suites:** `openchuck-ui`, `openchuck-app`, `openchuck-path`, `openchuck-cli` and `openchuck-project`: 2912 tests, all passing after the changes above and after the rebase (3380 s in all on 4 cores; no new test over 3 s). `cargo fmt`, `cargo clippy --workspace --all-targets` (clean) and `cargo xtask deps`, `licences`, `records` and `trace` pass.

**Coverage** (local, `cargo llvm-cov nextest --profile coverage` on the tests of the changed areas, 328 tests). Of the changed lines, those not run are:
- the Locate dialog's display of a refused Forget (`shell/checkpoints.rs`), whose rule `forget` is tested in `openchuck-app`;
- `forget`'s error for a note that cannot be removed for another reason than not being there (NFR-07);
- the Geometry section's indented rows (#609) for boundary options, refusals and reach notes, which the geometry tests of `shell::frame_tests` run, outside this run's filter.

**The real application:** the command line on a copy of the public baluster project (`testdata/examples/baluster/baluster-project`) prints the note as `note: Held region (operation 'Finish'): no part of the tool comes within 0.500 mm of the held region at the start of the stock, …`, and it is no longer in the checks before writing. The guide's screenshots were captured again from the release build (Xvfb, Mesa lavapipe, 1920 by 1080): the tutorial, the tabs and held regions example, the flute-length example and the simultaneous example. The simultaneous example was captured again after the rebase. The other three were captured before it, and each `CAPTURED.toml` records `reviewed_at` the rebased commit, because #856 changed nothing those STL screens show. In them:
- the tutorial's 2 Finish shows a green dot, with two warnings left;
- the Workholding section shows Kept clear ("Depends on the tools" before tools are added, then "to X 15.078 mm" in the tabs example);
- the head zone reads "no room for a tab" before Apply (#767);
- the simultaneous example's figure 6 shows the refusal alone (#766).

Two capture scripts were fixed: the tabs example's Grip length position below the new row; the simultaneous example's Add at its decision-0677 place, and its Lead angle and Pattern positions after #856 moved Exact STEP surface contact to Advanced. The simultaneous capture needs `openchuck-step-worker` built beside the release binary. The user guide builds with mdBook 0.5.4, and `user-docs/tools/check.py --screens strict` reports 0 errors and 0 warnings.

**Not verified here:** Windows and macOS (the change is not platform-specific); a screen reader (OI-2110).

**Open items:** OI-9984 (the owner's acceptance of this record).
