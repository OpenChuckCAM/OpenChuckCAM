# PROJUX-B-1: The undo history is listed in the title bar's popover and any step can be returned to (UI-22, UI-08, UI-14, NFR-18, NFR-07; issue #705; lane PROJECT-UX-B)

- **Status:** Proposed by lane PROJECT-UX-B on 25 Nov 2026 and built in the lane's first pull request. The project owner answered the lane's question on 25 Nov 2026, relayed by the coordinator (decision 0049): Q1 option A, mark the last save in the list. The owner has not yet accepted the record itself (OI-9941).
- **Date:** 25 Nov 2026.
- **Requirements:** UI-22 (show the undo history and return to any step, P2), UI-08 (undo and redo for all edits), UI-14 (keyboard), NFR-18 (the action log names no value), NFR-07 (a clear refusal, never a crash).
- **Builds on:** [decision 0233](0233-unconfirmed-edits-before-leaving-a-form.md) (accepted; one undoable command per applied value, and the departure guard), decision UI-BUGS-A (proposed, #891: values of one setup kept together by Keep are one command), [decision 0157](0157-user-action-log.md) (the action log), [decision 0536](0536-message-levels-operation-reference-and-setting-links.md) section 8 (times said as "how long ago" from the session's own clock), [decision 0677](0677-keyboard-focus-tooltips-and-tool-windows.md) (keyboard focus and tooltips).
- **Amends:** the specification's section 12.3 (*Rows*: the time is said as how long ago; a "Saved" line), section 3 (the title bar's menus take only their own width), and section 16's key list (Ctrl H is built). No requirement, architecture or project-format change.
- **Open items:** adds OI-9941 (the owner's acceptance of this record) and OI-9942 (the owner's check on a native build). Provisional numbers; the coordinator assigns the record number and the OI numbers at release, in merge order.

Numbering: provisional. No project schema number: the history lives in memory and nothing stored changes. No library-family API change: `openchuck-app` (`history` module, `Application::undo_history` and `go_to_step`, `AppError::NoHistoryStep`) and `openchuck-ui` change, and neither is checked by `cargo-semver-checks` (decision 0048), so the workspace version is not bumped.

## Context

Undo and Redo (UI-08) work one step at a time. The program kept every step since the project was opened, but only the command that reverses it, with no name and no time, so nothing could be listed. The specification already describes the list (section 12.3, board 12): a popover from the title bar (Ctrl H), newest first, the current step highlighted, undone steps in italics marked "undone", a row returns the project to that step, and a line at the foot gives the keys.

While building it, the lane found that the title bar's own Undo and Redo buttons, the project's name and its save state had not been drawn on screen. egui 0.37's menu bar takes all the width it is offered (`MenuBar::ui` sets its minimum width to the available width), so everything after the menus was laid out past the window's right edge and clipped. Undo and Redo still worked from the Edit menu and the keys. The history button goes in the same row, so the row is fixed here.

## Decision

1. **Each step keeps a name and a time.** When a command runs, its name is worked out against the project as it was just before, so a later rename or delete does not change it. The name says the change in words with its item, and for a parameter the value with its unit: "Set stepover 1.2 mm (Roughing)", "Reset stepover (Roughing)", "Add operation "Roughing"", "Rename setup "Setup 1" to "Flip"", "Edit setup "Flip"", "Delete operation "Roughing"", "Import model "shaft.stl"", "Turn off operation "Roughing"", "Hide the toolpath of "Roughing"". The parameter's label is the one its form shows. The form stores a cutting value (stepover, feed, plunge feed) together with the whole operation, because setting one drops the operation's material preset (decision 0230). A replaced operation whose only change is one parameter (and the material) is therefore named by that parameter, as setting it alone would be; anything more is "Edit operation". The owner-check pack of #705 found this, built from the first push. Several commands on one item applied as one step (Keep, decision UI-BUGS-A) are one row named "Edit <item>"; one command in a batch is named as itself. Values are in the project's units (millimetres and degrees). Showing them in inches is lane PROJECT-UX-B's later change (#704). Nothing new is stored in the project or written to the log.
2. **The list (specification 12.3).** A button at the right of Undo and Redo in the title bar (tooltip "Undo history (Ctrl+H)"), **Edit > Undo history** and Ctrl H open it, and Ctrl H or Esc closes it, as does a click outside it. Rows run newest first: the undone steps in italics with "undone", then the applied steps with how long ago each was made ("just now", "3 min ago"), and last the project as it was opened ("Opened project", "Created project" or "Recovered project"). The step the project is at is highlighted. The foot reads "Ctrl+Z undo · Ctrl+Shift+Z redo. A new edit after undoing discards the undone steps.", naming Redo as the Edit menu and the Redo button do (Ctrl+Y works too). The list scrolls past 400 pixels.
3. **The time is how long ago (deviation from board 12's clock times).** Board 12 shows clock times ("14:32"). The program has no time-zone library, and the Jobs tab already says times as how long ago from the session's own clock (decision 0536 section 8), so the history does the same. A clock time would need a new dependency (NFR-14). This is the one difference from the mock-up and the owner's Q1 picture ("Saved at 14:21" reads "Saved 3 min ago").
4. **The last save is marked (Q1, the owner's answer A).** A green line "Saved 3 min ago" sits under the step at which the project was last saved in this session. It stays while that state can be reached by undo or redo, and goes when a new edit discards it (the save was made at a step that is now undone). Nothing is marked before the first save of the session: the opened project is the saved files.
5. **Choosing a row returns to that step**, by undoing or redoing as many steps as that takes, exactly as pressing Undo or Redo that many times would (verification plan, UI-22). The step is one revision per undo or redo, as before, and the scene is rebuilt once. It goes through the same guard as Undo (decision 0233): unconfirmed text in a form is resolved first. A step that does not exist is refused with "there is no step 5 in the undo history; it has 4", and nothing changes (NFR-07). The list stays open, so the user can go back and forth.
6. **Keyboard (UI-14, decision 0677).** Opening the list with Ctrl H or the menu puts the keyboard on the current row. Tab and Shift Tab move between rows, and Enter or Space chooses one. The rows are buttons with their full text, so a screen reader reads them.
7. **The action log names no value (NFR-18, decision 0157).** A row's text can hold a value the user typed. A click on a row is logged as "click: 'Undo history step'", and the undo and redo lines the application already writes name each step without values.
8. **The title bar's menus take only their own width.** The menu bar is given the width its six titles need, so the project's name, its save state, Undo, Redo and the history button are on screen again, in the order section 3 gives.

Not chosen:
- A dock tab for the history: the specification puts it in a popover, and the dock's tabs are for jobs and results.
- Naming every command from its log text: the log text has no values and uses command names ("SetParameter"), not the user's words.
- Clock times: see item 3.

## Verification

Tests written first from UI-22 and section 12.3, named by requirement. They use generated projects only. Times are uninstrumented (`cargo nextest run`).

- `openchuck-app`, `tests/undo_history.rs` (in `tests/integration.rs`):
  - `undo_history::req_ui_22_steps_are_named_in_the_users_words_with_their_item`: twelve kinds of step, each name exact (0.01 s);
  - `undo_history::req_ui_22_values_kept_together_are_one_step_named_by_their_item`: a batch on one operation is one "Edit operation" row, and a one-command batch is named as its command (0.01 s);
  - `undo_history::req_ui_22_undone_steps_are_marked_and_a_new_edit_discards_them` (0.01 s);
  - `undo_history::req_ui_22_returning_to_a_step_gives_the_same_project_as_that_many_undos_or_redos`: every step back and forward (0.01 s);
  - `undo_history::req_ui_22_req_nfr_07_a_step_that_does_not_exist_is_refused_and_changes_nothing` (0.01 s);
  - `undo_history::req_ui_22_the_last_save_is_marked_and_goes_when_its_state_is_discarded` (0.02 s);
  - `undo_history::req_ui_22_the_history_starts_at_the_project_as_opened` (0.02 s);
  - `undo_history::req_ui_22_a_value_set_through_the_form_is_named_with_its_setting_and_value`: through `set_parameter`, the inspector's path. It failed on the first push ("Edit operation" for stepover and plunge feed) and passes with the fix. It also checks that two settings, or one with the tool, stay "Edit operation";
  - `undo_history::req_ui_22_project_settings_and_several_items_are_named_too`: project-wide settings, all toolpaths hidden, a model's units in words, and a batch over several items named "<first> and 1 more" (0.01 s).
- `openchuck-app`, `tests/undo.rs`: `undo::req_ui_22_returning_to_a_step_equals_that_many_undos_or_redos`, a property test (256 cases) with random edits of every kind and random jumps, checked against the project read by single undos (0.6 s).
- `openchuck-app`, `src/history.rs` unit tests: numbers without trailing zeros, labels inside a sentence, values with their unit (`req_ui_22_*`).
- `openchuck-ui`, `shell/history_tests.rs`, through real frames:
  - `req_ui_22_req_ui_14_ctrl_h_lists_the_steps_newest_first`: order, foot text, the keyboard on the current row, Esc closes;
  - `req_ui_22_choosing_a_row_goes_back_to_it_and_forward_again`;
  - `req_ui_22_the_last_save_is_marked_under_its_step`;
  - `req_ui_22_req_nfr_18_a_chosen_row_is_logged_without_its_text`;
  - `req_ui_22_the_title_bar_button_opens_and_closes_it`, with a click outside;
  - `req_ui_22_the_edit_menu_opens_it`;
  - `req_ui_22_req_ui_07_a_value_typed_in_the_inspector_is_named_with_its_value`: Stepover typed and confirmed with Enter in the operation inspector, twice;
  - `req_ui_22_req_ui_07_setup_values_kept_together_are_one_edit_setup_step`: Keep on a setup's unconfirmed Diameter is one "Edit setup" step;
  - `req_ui_22_req_ui_14_the_foot_names_the_keys_the_edit_menu_shows`: the foot names Redo as the Edit menu does (Ctrl+Shift+Z; Ctrl+Y still works). The owner-check pack found that the first push named Ctrl+Y;
  - `req_ui_08_req_ui_22_the_title_bar_shows_the_name_undo_redo_and_history`: the regression for item 8. On `main` the name, Undo and Redo were not drawn.

### Results (lane PROJECT-UX-B, 25 Nov 2026)

Environment: managed Linux x86_64 container, Rust 1.97.0, debug builds. The branch `claude/lane-projux-b-1-undo-history` was first built on `main` `6b039e5`, then rebased onto `361613e` (FACEFLOW #854) before the push. Every test was timed uninstrumented with `cargo nextest run`.

- Before the rebase: `cargo nextest run -p openchuck-ui`: 760 of 760 passed, 1 skipped, in 651 s. The new tests take 0.2 to 0.4 s. The slowest tests are the existing playback tests (about 157 s each), already classified. `cargo nextest run -p openchuck-app -p openchuck-project`: 1119 of 1119 passed, 1 skipped, in 1374 s. The new tests take 0.01 to 1.8 s; the 1.8 s one is the property test. No new test is near a minute, so none is added to `.config/nextest.toml`.
- The first full UI run found two failures, both fixed before the push. The source scan `focus::tests::req_ui_14_every_scroll_area_keeps_the_focused_control_in_view` needed the history list to use `show_keeping_focus`. `shell::model_copy_tests::req_prj_02_not_now_keeps_the_link_marked_with_a_way_to_copy_it` looked for the text "linked" anywhere on screen, and its project, named "linked", is now in the title bar (item 8). It now looks below the title bar.
- After the rebase onto `361613e`: `cargo nextest run -p openchuck-ui`: 761 of 761 passed, 1 skipped, in 598 s. `cargo nextest run -p openchuck-app`: 918 of 918 passed, 1 skipped, in 1292 s.
- `cargo fmt --all`, `cargo clippy --workspace --all-targets` (no warnings), `cargo xtask deps`, `licences`, `records` and `trace` (105 of 105 P1 requirements have tests), and `cargo xtask changelog check-pr` all pass.
- Coverage, report-only under decision 0102. The instrumented run used the tests that reach the changed code: `cargo llvm-cov nextest --profile coverage -p openchuck-app -p openchuck-ui -E 'test(~req_ui_22) | test(~req_ui_08) | test(~undo) | test(~history) | test(~req_nfr_18) | test(~model_copy_tests) | test(~open_recovery) | test(~checkpoint)'` ran 135 tests, all passing, in 363 s. Line coverage of the changed files: `openchuck-app/src/history.rs` 92.1%, `openchuck-ui/src/shell/history.rs` 93.8%, `openchuck-ui/src/shell/actions.rs` 97.7%. The application file's uncovered names (project-wide settings and mixed batches) then got the last test above. Two UI paths stay uncovered: the guard's deferral of a jump while a field holds unconfirmed text, which is the same `defer` as Undo and is tested there (decision 0233), and the error dialog for a refused jump, which the list cannot cause because it offers only steps that exist (NFR-07, defensive). The whole-crate figures were not run instrumented, because both suites take over half an hour uninstrumented. The nightly coverage run measures them. `cargo llvm-cov clean --workspace` was run afterwards.

Not verified here: native Windows and macOS drawing, keys and screen readers; that is the owner's check (OI-9942).

## Owner check

What to check, and the result expected:
1. The title bar shows the project name, its save state, Undo, Redo and the history button, all inside the window.
2. Ctrl H opens the history, newest first, with the current step highlighted.
3. Choosing a row returns the project to that step, and the undone steps read "undone".
4. After File > Save the list shows "Saved … ago" under the step that was saved.
5. The keyboard alone can open the list, move along it, choose a row and close it.

Steps, on a nightly build of this pull request (Windows):
1. **File > Open project…**, choose `testdata/examples/baluster/baluster-project` from a copy of the repository (the tutorial's public project, with its own model). Work on a copy of that folder if you do not want it changed.
2. Check the title bar (check 1).
3. In the project panel select the first operation. In its form, set **Stepover** to `0.5` and press Enter, then `0.8` and Enter. Untick the operation's eye in the project panel.
4. Press Ctrl H. Expect, newest first, "Hide the toolpath of "…"", "Set stepover 0.8 mm (…)", "Set stepover 0.5 mm (…)", then "Opened project" (check 2).
5. Click "Set stepover 0.5 mm". Expect the stepover field to read 0.5, the eye shown again, and the two newer rows in italics reading "undone" (check 3). Click the top row: everything is back.
6. Press Ctrl S, then Ctrl H. Expect "Saved just now" under the top row (check 4).
7. Press Esc. Press Ctrl H again, then use Tab, Shift Tab, Enter and Esc only (check 5).

A pass is all five as expected. A fail is anything else: report the step number, what was shown (a screenshot helps) and the build's version from Help > About.
