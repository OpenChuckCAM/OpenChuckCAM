# PROJUX-B-3: The project is saved automatically when the user turns on Auto-save, a switch in the title bar (PRJ-08, PRJ-05, UI-07, UI-14, NFR-07; issue #702; lane PROJECT-UX-B)

- **Status:** Proposed by lane PROJECT-UX-B on 28 Nov 2026. It is built as the project owner answered on 25 Nov 2026, relayed by the coordinator (decision 0049):
  - Q3 B: opt-in auto-save, off by default, with the interval in Settings.
  - The owner added an Auto-save switch in the window, like Office's.
  - The lane's two points stand: turning it on saves at once, and after a recovery it waits for one File > Save.
  - The owner agreed the specification's section 10, 3 and 13 wording.

  The owner has not yet accepted the record itself (OI-9947).
- **Date:** 28 Nov 2026.
- **Requirements:** PRJ-08 (autosave and crash recovery, P2), PRJ-05 (Git-friendly project files), UI-07 and decision 0233 (unconfirmed fields), UI-14 and decision 0677 (keyboard, tooltips on focus), NFR-07 (a clear message).
- **Builds on:** [decision 0302](0302-first-run-rough-edges.md) item 4 (the save state reads "Edited, not saved"), [decision 0243](0243-project-recovery-checkpoints.md) (recovery checkpoints; item 8: only an explicit Save writes a recovered state), [decision 0460](0460-recovery-offered-only-for-the-opened-project.md), decision PROJUX-B-1 (the title bar, the "how long ago" times; accepted).
- **Amends:** the specification's section 10 ("Nothing saves the project by itself" gains "unless the user turns on Auto-save", in the owner's agreed wording), section 3 (the title bar row) and section 13 (the interval). OI-1232 (auto-save deferred by decision 0302 item 4) is closed by this record.
- **Open items:** closes OI-1232. Adds OI-9947 (the owner's acceptance) and OI-9948 (the owner's check on a native build). Provisional numbers; the coordinator assigns the record number and the OI numbers at release, in merge order.

Numbering: provisional. No project schema number: the switch and the interval are per-user settings (`UserSettings::auto_save`, `auto_save_minutes`; a settings file without them reads as off and 5). No library-family change, so no version bump. No change to the recovery format.

## Context

Nothing saves the project by itself: the owner chose that on 30 Oct 2026 (decision 0302 item 4) and deferred auto-save "if users still lose work" (OI-1232). Recovery copies are written every 3 minutes and 15 seconds after an edit, into a hidden folder of the project, and offered after a crash (decisions 0243 and 0460). Issue #702 asks whether to build auto-save of the user's file in 2026.2. The owner chose to build it, opt-in.

## Decision

1. **A switch in the title bar, before the project's name.** It reads "Auto-save", then the switch, then "On" or "Off" (section 3). It is off by default and remembered per user for every project. It is a real control: in the title bar's Tab order after the menus, Space toggles it, it draws the focus ring, and its tooltip opens on focus as well as hover ("Save the project every 5 minutes while it has changes (File > Settings)"; decision 0677). **File > Save automatically** is the same choice as a ticked menu item. No shortcut, so a stray key cannot turn it off.
2. **When it saves.** It saves the project's own files, as File > Save does (`Application::save`):
   - at once when it is turned on with changes (the lane's point a);
   - then the interval after the first change not yet saved, the interval being 1 to 30 minutes in Settings, default 5;
   - a save by hand restarts the clock.

   It never commits to Git or stages files (PRJ-05): the working files change as with Save.
3. **When it waits.** A save that is due waits, and goes ahead on the next frame that frees it:
   - while a box holds unconfirmed text (decision 0233): the save state reads "Edited · next auto-save waits for the typed value";
   - while a dialog is open: "… waits for the open dialog";
   - after a recovery, until the user has done File > Save once (decision 0243 item 8, the lane's point b): "Recovered, not saved · auto-save waits for File > Save".
4. **What the title bar says.**
   - After an automatic save: "✓ Saved automatically 3 min ago". This is how long ago, as the undo history says it (decision PROJUX-B-1, accepted, whose item 3 the owner confirmed). The agreed wording's example, "14:32", was a clock time, which would need a time-zone library.
   - While a change waits for its interval: "Edited, saved automatically soon".
   - The tooltip while it is on: "Auto-save is on: the project is saved every 5 minutes while it has changes."
   - With the switch off, everything is as before: "Edited, not saved" and its tooltip.
5. **A save that fails** (a read-only folder, a full disk, a locked file):
   - The switch stays on.
   - The title bar reads in amber "Auto-save failed · see Messages", after the warning mark the tree uses, with the reason in its tooltip.
   - Messages gets one warning naming the folder and the reason, not one per try, until a save works.
   - It is tried again at each interval. Recovery copies carry on.
6. **The application decides; the window acts.** `openchuck-app` has `autosave::Schedule`, which takes the frame's facts (the switch, modified, recovered, typing, dialog) and monotonic time, and says Off, Idle, Waiting or Save. It is unit-tested without a window. The window saves, records "Saved automatically …" in the Jobs list (decision 0536 section 8), and tells the schedule how the save went.

Not chosen (Q3):
- **A, keep it deferred.**
- **C, on by default:** a user's experiment would overwrite the last saved file without their asking.

## Verification

Tests written first from PRJ-08 and the owner's answers, named by requirement. Generated projects only.

- `openchuck-app`, `src/autosave.rs` unit tests:
  - `req_prj_08_off_never_saves`;
  - `req_prj_08_turning_it_on_with_changes_saves_at_once`;
  - `req_prj_08_it_saves_an_interval_after_the_first_unsaved_change`;
  - `req_prj_08_req_ui_07_a_due_save_waits_for_typing_a_dialog_and_a_recovery`;
  - `req_prj_08_req_nfr_07_a_failure_is_said_once_and_tried_again_after_the_interval`;
  - `req_prj_08_a_save_by_hand_restarts_the_clock_and_ends_a_failure`;
  - `req_prj_08_the_interval_is_held_to_one_to_thirty_minutes`.
- `openchuck-ui`, `shell/autosave_tests.rs`, through real frames:
  - `req_prj_08_auto_save_is_off_by_default_and_nothing_saves_by_itself`;
  - `req_prj_08_req_ui_14_turning_the_switch_on_saves_at_once_and_says_so`: the saved files hold the change, and the File menu's tick turns it off;
  - `req_prj_08_req_ui_07_a_due_save_waits_for_a_typed_value`;
  - `req_prj_08_after_a_recovery_auto_save_waits_for_one_save_by_hand`;
  - `req_prj_08_a_due_save_waits_for_an_open_dialog`;
  - `req_prj_08_req_nfr_07_a_failed_save_warns_once_and_the_switch_stays_on`: a folder where `project.toml` goes makes every save fail, for any user. One warning stays one, and a save by hand that works removes it.

### Results (lane PROJECT-UX-B, 28 Nov 2026, on PR #904's local head `a115377c`)

Environment: managed Linux x86_64 container, Rust 1.97.0, debug builds. Tests were timed uninstrumented with `cargo nextest run`.
- `cargo nextest run -p openchuck-ui -p openchuck-app`: 1694 of 1695 passed, 2 skipped, in 1634 s. The one failure was `theme::tests::req_nfr_09_req_ui_16_every_character_the_interface_draws_has_a_glyph`: the failed state began with "▲", which the bundled fonts do not draw. The title bar now draws the tree's warning mark (a shape, so the state does not rest on colour alone) before the text, and the autosave tests and the glyph test then passed, 6 of 6 (0.14 to 1.6 s each).
- `cargo fmt --all`, `cargo clippy` on the changed crates, `cargo xtask deps`, `licences`, `records` and `trace` pass.
- Coverage, on the auto-save tests alone (`cargo llvm-cov nextest --profile coverage -p openchuck-app -p openchuck-ui -E 'test(~autosave) | test(~req_prj_08)'`), a lower bound:
  - `crates/openchuck-app/src/autosave.rs`: 100% of lines.
  - `apps/openchuck-ui/src/shell/autosave.rs`: 93.7%. The first run found the dialog wait's text not reached, and `req_prj_08_a_due_save_waits_for_an_open_dialog` was written for it.
  - Two parts stay uncovered, and why:
    - The switch's focus ring, drawn only while the switch has keyboard focus; no auto-save test tabs to it. Whether the rest of the suite reaches it is not checked here.
    - Clearing the failure warning after an automatic retry that works. A retry comes an interval after the failure, at least a minute of real time, which the frame tests do not wait. The schedule's side of it (`saved` ends a failure) is covered by `req_prj_08_a_save_by_hand_restarts_the_clock_and_ends_a_failure`, and the window's side is the owner's check (OI-9948).

This branch sits on top of PR #904 (B1), because both change the title bar, and is rebased once #904 has merged.

Not verified here: native drawing, keys and file locking on Windows, and Git status in a repository folder, which are the owner's check (OI-9948).

## Owner check

What to check, and the result expected:
1. The title bar shows "Auto-save" with the switch and "Off" before the project's name. Tab reaches it, Space turns it on, and its tooltip shows on focus.
2. With a change made, turning it on saves at once: "✓ Saved automatically just now", and the folder's files change.
3. While a value is typed in a box and not confirmed, it does not save, and says it waits for the typed value. After Enter it saves.
4. With the project folder made read-only (folder Properties, Read-only, or another program holding `project.toml` open), a save fails: amber "Auto-save failed · see Messages", one warning in Messages, and the switch stays on.
5. In a project kept in Git, `git status` shows the changed files after an automatic save, and no commit is made.

Steps, on a nightly build of the pull request (Windows):
1. Open a copy of `testdata/examples/baluster/baluster-project`. Look at the title bar (check 1).
2. Rename an operation (F2), then click the switch (check 2).
3. Settings: set Auto-save interval to 1. In an operation's Stepover type `0.5` and wait a minute without pressing Enter, then press Enter (check 3).
4. Make the folder read-only, rename another operation, and wait a minute (check 4). Make it writable again.
5. In a copy of the folder under Git (`git init`, `git add .`, `git commit`), make a change and let it save, then run `git status` and `git log` (check 5).

A pass is all five as expected. A fail is anything else: report the step, what was shown (a screenshot helps) and the version from Help > About.
