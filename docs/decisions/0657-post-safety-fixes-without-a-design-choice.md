# 0657: Post and machine safety fixes that need no design choice (GC-04, GC-10, GC-16, SET-10, NFR-07, NFR-12; issues #574, #599, #645 and part of #603; lane POST-SAFETY)

- **Status:** Proposed by lane POST-SAFETY on 22 Nov 2026. It records fixes that follow accepted decisions and the verification of the lane; the design choices of the lane are in the accepted decisions [0654](0654-coolant-off-before-every-spindle-stop.md), [0655](0655-a-copied-machine-profile-keeps-its-marks-and-warns.md) and [0656](0656-post-line-length-limit.md).
- **Date:** 22 Nov 2026.
- **Requirements:** GC-04 (post options), GC-10 (the LinuxCNC post), GC-16 (the command line), SET-10 (machine profiles), NFR-07 (clear errors), NFR-12 (the user verifies G-code), GC-07 and NFR-06 (deterministic output).
- **Open items:** closes OI-2083 (raised for #574 on the never-merged branch `claude/safetynotice-followup`; the number was reserved there and is used here as raised and closed).

Requirements and architecture are not edited by this record. No project or machine schema changes. No public API of the library family changes for the items here.

## Decision

1. **The NFR-12 notice check reads whole lines at the top (issue #574, [decision 0533](0533-safety-notice-post-option.md)).** With `safety_notice` on, the post wants every line of `SAFETY_NOTICE`, in order, as whole comment lines (an `N` word allowed in front) in the comment block at the top of the program. The notice text inside a longer line, or written after the program has started, is refused as a missing notice. This is commit `80addd6b` from `claude/safetynotice-followup`, carried over unchanged. Built-in output does not change.
2. **`unwind = "rotate"` writes nothing at a boundary, as decided (issue #599).** [Decision 0241](0241-continuous-rotary-operation-boundaries.md) item 6 (accepted) says a rotate-only post must not substitute a physical full-turn return for the drop-turns bookkeeping; the post was right and its doc comment ("a rapid rotation of A by whole turns") and the user guide's "at present" were wrong. The doc comment of `UnwindMode::Rotate` and the guide now say that nothing is written and the A words carry the whole turns. No output changes.
3. **The command line prints the NFR-12 notice when it refuses for a missing acknowledgement (issue #645).** Already fixed on `main` by pull request #659 ([decision 0536](0536-message-levels-operation-reference-and-setting-links.md) step 4): both the partial-path and the possible-collision refusals print `VERIFY_GCODE_NOTICE` before they stop, as [decision 0540](0540-the-command-line-checks-before-it-writes.md) item 4 asks. Verified here; nothing changed.
4. **A corrected value clears its refusal in the machine library (issue #603, its sev3 part).** The red line at the foot of the machine library window that says why a value was refused goes as soon as the same value is set to one that is accepted, instead of staying until Save. A refusal of another value stays. Only the window's state changes (`openchuck-ui`).

## Verification

Lane POST-SAFETY, branch `claude/lane-post-safety`, from `main` `57b8ac1b`, 22 Nov 2026, Linux container, Rust 1.97.0. This section also verifies decisions 0654 (option A) and 0655 (option C), built in the same pull request. Decision 0656 is built in the lane's second pull request.

**Tests written from the requirement first** (each watched failing on the unchanged code, unless said otherwise):

| Test | Crate | Covers | Before the fix |
| --- | --- | --- | --- |
| `req_nfr_12_req_nfr_07_the_notice_must_be_whole_lines_at_the_top` | `openchuck-post` (`tests/posts.rs`) | item 1, #574 | failed (the notice inside a longer line was accepted) |
| `req_gc_04_req_gc_10_rotate_writes_nothing_at_a_boundary_and_never_turns_back` | `openchuck-post` | item 2, #599 | passed: the behaviour was right, the words were wrong; the test pins decision 0241 item 6 (two operations, LinuxCNC A words never decrease and end at A2160, no `G92`; the Z1 writes its reset twice) |
| `req_gc_04_req_nfr_12_coolant_goes_off_before_every_tool_change_and_the_end` | `openchuck-post` | decision 0654, #598 | failed (a tool change with coolant on) |
| `req_set_10_req_nfr_07_a_corrected_value_clears_its_refusal` | `openchuck-ui` (`shell/machine_library_tests.rs`) | item 4, #603 | failed (the message stayed after the correction) |
| `machine_library::tests::req_set_10_req_ui_13_a_copy_keeps_its_marks_and_warns_where_they_were_made` | `openchuck-app` | decision 0655 | did not compile (written against the new accessors) |
| `req_set_10_req_ui_13_a_copy_says_its_confirmed_marks_were_made_on_the_source` | `openchuck-ui` | decision 0655 | did not compile (written against the new accessor) |

Item 3 (#645): `req_op_04_req_nfr_12_the_cli_writes_a_partial_path_only_when_acknowledged` and `req_gc_16_cli_writes_a_possible_collision_only_when_acknowledged` (`openchuck-cli`, `tests/e2e.rs`) pass on `main` `57b8ac1b` (2 of 2, 3.9 s); both check the notice on the refusal. No code changed.

**Runs (uninstrumented, `cargo nextest run`):** `openchuck-post` and `openchuck-ui` together 804 of 804 passed, 1 skipped, 738 s (the six slow tests are the existing playback and dock tests, untouched); `openchuck-app --lib` 278 of 278 passed, 1 skipped, 52 s. Each new test takes under 1 s. The built-in golden tests pass unchanged: no built-in program changes (GC-07, NFR-06). `cargo fmt --all`, `cargo clippy --workspace --all-targets` (no warnings), `cargo xtask deps`, `licences`, `records` and `trace` (105 of 105 P1 requirements) pass.

**Coverage (instrumented, own tests, lower bounds):** `openchuck-post` all 74 tests: `render.rs` 96.9 % of lines, `options.rs` 92.1 %. The one new line not reached is the notice check's branch for a header line that starts with `N` but is not a line number (`render.rs`, `_ => line`), a defensive case: such a line is not a comment, so the check stops there either way. `openchuck-app` and `openchuck-ui`, the 27 machine library tests: no changed line of `machine_library.rs` or `machine_library_window.rs` is unreached.

**Not verified:** no program was run on a machine or in the LinuxCNC simulator for the coolant codes (no built-in post writes them); the Makera Z1's handling of `M7`/`M8`/`M9` is not known. Generated G-code is the user's to verify before it is run (NFR-12). The user guide's custom machine figures do not show the copy warning yet (OI-2158).
