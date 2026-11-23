# 0656: A post's line-length limit, checked before export (GC-08, GC-09, NFR-12; issue #703, OI-933; lane POST-SAFETY)

- **Status:** Accepted by the project owner on 22 Nov 2026, relayed by the coordinator (decision 0049). Proposed by lane POST-SAFETY on 22 Nov 2026; the owner chose item 1 and option A the same day (relayed). It is built in a second pull request of the lane, after the 0.55.0 crates are published, with the workspace minor version bump it needs (decision 0048), not in the first.
- **Date:** 22 Nov 2026.
- **Requirements:** GC-08 (check generated G-code before export, P2), GC-09 (the Z1 post), GC-03 and GC-04 (post definitions and options), GC-07 and NFR-06 (deterministic output), NFR-07 (clear errors), NFR-12 (the user verifies G-code).
- **Related:** [decision 0222](0222-short-moves-minimum-block-time.md) (where OI-933 was raised), [decision 0533](0533-safety-notice-post-option.md) (the notice lines), [decision 0540](0540-the-command-line-checks-before-it-writes.md) (the command line runs the export checks).
- **Open items:** OI-933 (the limit itself; this record would close its software half), OI-2157 (the build; closed by the second pull request). OI-933 stays open for confirmation of the limit on the machine or from Makera.

Requirements and architecture are not edited by this record. No project or machine schema changes: the limit is a post definition option, optional with a default, like `combine_rotary_runs` (decision 0506).

## Context

OI-933: the Z1 firmware source keeps a G-code line in a 64-byte queue item (`Player.h`), so a line longer than 63 characters is cut without a warning. **This is from the firmware source only; it is not confirmed on the machine or by Makera.** LinuxCNC allows 255 characters.

Measured on `main` `57b8ac1b` over every committed `.nc` program (23 files): the longest line that is not a comment has 40 characters; the only lines over 63 are the two NFR-12 notice lines of decision 0533, 76 and 82 characters, in every Z1 program. The 0.55.0-rc3 air cuts and the first cut on the Z1 ran these programs with no visible effect (decision 0653): no air cut or cut was affected, since only comment lines were over the limit. This suggests a cut comment is harmless there; it is not confirmed. A title or operation-name comment can also pass 63 with a long project or operation name. Today nothing checks line length: the GC-08 static checks (`openchuck-gcode::static_check`) look at units, offsets, feeds, limits, tool changes and the end code, and a finding blocks export ("failed its checks") in the dialog and on the command line.

## Decision to take

1. **Where the limit lives (recommended, any option below):** a new optional post option `max_line_length` (characters per line, not counting the line end; 20 to 4096). The built-in Makera Z1 post sets 63, the LinuxCNC post 255. A post that leaves it out is not checked, as now.
2. **What happens to a line over the limit:**
   - **A (recommended):** a command line (anything that is not a comment) over the limit is a GC-08 finding, `line N has X characters; the post allows at most 63`, which blocks export through the existing path, in the dialog and on the command line. A comment longer than the limit is wrapped by the post at spaces into several comment lines that fit (a word longer than the line is cut), so nothing is lost. The NFR-12 notice is then written on the Z1 as four shorter comment lines; the notice check (decision 0533) reads the wrapped lines as one text. Z1 golden programs change in the notice lines only.
   - **B:** as A, but a long comment is cut at the limit, with the closing parenthesis kept. The end of each notice sentence is lost on the Z1.
   - **C:** as A, but comments are neither checked nor changed: only command lines are limited. Relies on the firmware's cut of a comment being harmless (not confirmed).
   - **D:** a warning only, never blocking, for every line over the limit.

## Recommendation and choice

The lane recommended item 1 and option A for item 2; the owner chose both (22 Nov 2026, relayed), and asked that the NFR-12 notice text stay word for word, only wrapped, and that the pull request show the Z1 golden diff. A cut command line changes what the machine does with no word, so it must block. Wrapping comments keeps the notice whole on the Z1 without changing its wording (which would amend decision 0533) and without relying on unconfirmed firmware behaviour. The wrap is deterministic (GC-07). Generated G-code is still the user's to verify before it is run (NFR-12).

## Consequences (option A)

- `openchuck-post`: the option, its validation, and comment wrapping in the renderer (all comments come from the template's `line` macro or the notice, so the wrap is applied to the comment text the template is given, and to the rendered lines as a final check). `openchuck-gcode`: a `max_line_length` in `CheckConfig` and a `LineLength` finding. `openchuck-app`: passes the post's limit to the check. A new public field of `PostOptions` (and the `CheckConfig`/`FindingKind` changes) is breaking for `cargo-semver-checks` (a struct built by literal gains a field), so the change takes the workspace minor bump (decision 0048) in the second pull request, claimed in the work breakdown when the coordinator says the 0.55.0 crates are published.
- Z1 golden programs re-blessed for the notice lines only; every other line byte-identical.
- The user guide's post reference (`writing-a-post.md`) documents `max_line_length`; the G-code reference says the Z1 post wraps long comments and refuses a longer command line.
- OI-933: the check closes its software half; confirming the limit on the machine or with Makera stays open.

## Verification

Lane POST-SAFETY, second pull request, branch `claude/lane-post-safety-line-length` on `main` `4a381f3a` (#835 merged), 22 Nov 2026, Linux container, Rust 1.97.0. Library family 0.56.0 (decision 0048).

**As built, where it differs in detail from Consequences above (no change of rule):** the G-code check is a separate function, `openchuck_gcode::line_length_check(source, max_chars)`, with the finding kind `FindingKind::LineTooLong`; `CheckConfig` is unchanged. `openchuck-app` runs it on each file before the static checks, so its finding comes first in the file's report and in the export's "failed its checks" line (`line N: the line has X characters; the post allows at most 63`). Comments are wrapped once, on the rendered program, after the NFR-12 notice check (so the notice is checked as the template wrote it). A comment line is a whole `(...)` line after an optional `N` word; the `N` word stays on its first line. Any other line is never changed.

**Tests written from the requirement first** (each failed before the change, or did not compile against the new API):

| Test | Crate | Covers |
| --- | --- | --- |
| `req_gc_08_req_nfr_12_the_z1_wraps_long_comments_and_keeps_the_notice_word_for_word` | `openchuck-post` | Z1 63, LinuxCNC 255; every Z1 line fits; the notice joined is word for word; LinuxCNC unchanged |
| `req_gc_08_req_gc_07_long_comments_are_wrapped_to_fit_and_lose_nothing` | `openchuck-post` | a long title with an 80-character word, with and without line numbers; deterministic; nothing lost |
| `req_gc_08_command_lines_are_never_wrapped` | `openchuck-post` | command lines identical with and without a tight limit; a word of exactly two lines |
| `req_gc_04_req_nfr_07_bad_options_are_refused_naming_the_option` (two cases added) | `openchuck-post` | 19 and 4097 refused at load, naming `max_line_length` |
| `req_gc_08_a_line_over_the_posts_limit_is_a_finding` | `openchuck-gcode` | the finding, its line and message; the line end (LF or CRLF) not counted |
| `posting::req_gc_08_a_line_longer_than_the_posts_limit_fails_the_files_checks` | `openchuck-app` | a post folder with limit 20 fails its file checks naming the first long command line; 4096 is clean |

**Also in this pull request (issue #839 item 3, the coordinator's instruction, 22 Nov 2026):** the NFR-12 notice check's opening comment block takes only lines that are one comment and nothing else, so a line such as `(title) G90` ends it and a notice written after it is refused (decision 0533; a CodeRabbit Minor on #835). Test `req_nfr_12_req_nfr_07_a_mixed_comment_and_command_line_ends_the_opening_block`, which failed before the change (the post was accepted). Built-in output does not change.

**After review (CodeRabbit on #840, two Minors, folded into the merge push):** a long `N` word narrows only the first line of a wrapped comment; the lines after it use the whole width, and a word too long for the first line but not for a whole one moves intact to the next line, leaving the numbered first line an empty comment. A long last comment with no line end is wrapped onto separate lines too. Tests `req_gc_08_req_nfr_12_a_long_line_number_narrows_only_the_first_wrapped_line` and `req_gc_08_a_long_last_comment_without_a_line_end_is_wrapped_onto_lines`, both failing first. Programs without line numbers (both built-in posts) do not change.

The NFR-12 notice tests of decision 0533 now read the notice block as wrapped lines (`notice_block`, `notice_text` in `tests/posts.rs`): word for word on both posts, whole lines on LinuxCNC.

**Golden programs:** the 23 Makera Z1 `.nc` goldens under `testdata/golden/` and `post_makera_z1_small_part.ngc` change in the notice lines only, each sentence wrapped onto two lines; every other byte is unchanged. The LinuxCNC goldens do not change. The `.nc` goldens that only the STEP kernel tests produce (`drilling`, `face_flow`, `swarf` and others in `testdata/golden/reference/`) could not be run here (no OpenCASCADE in this container): they were changed with the same text replacement, which the non-kernel goldens confirm byte for byte; the kernel job in CI checks them.

```text
-(This program has not been run on your machine. Check it before you run it.)
-(Simulate it and air cut it first. You are responsible for what the machine does.)
+(This program has not been run on your machine. Check it)
+(before you run it.)
+(Simulate it and air cut it first. You are responsible for)
+(what the machine does.)
```

**Runs (uninstrumented, `cargo nextest run`):** `openchuck-app`, `openchuck-cli` and `openchuck-gcode` 998 of 999 passed, 1 skipped, 1,425 s (the one failure was the new app test, written before the built-in posts carried the key; corrected, then `posting::` 32 of 32); `openchuck-post` and `openchuck-gcode` 114 of 114; `openchuck-ui` 730 of 730, 1 skipped, 566 s (unchanged code; run for the bump). Each new test takes under 1 s. `cargo fmt --all`, `cargo clippy --workspace --all-targets` (no warnings), `cargo xtask deps`, `licences`, `records` and `trace` pass. `cargo semver-checks` against `4a381f3a`: the four library crates pass at 0.56.0; as a patch release `openchuck-post` and `openchuck-gcode` each fail one major check (the new `PostOptions` field and the new `FindingKind` variant), so the minor bump is needed.

**Coverage (instrumented, own tests):** `render.rs` 96.1 % of lines, `options.rs` 92.6 %, `check.rs` 95.7 %. New lines not reached: a program whose last line has no line end (`wrap_comments`; templates always end the program with one) and a long line that is not a comment, passed through unchanged (reached by `req_gc_08_command_lines_are_never_wrapped` after the coverage run).

**Not verified:** the 63-character limit on the machine (OI-933 stays open); no program was run on the Z1 or in the LinuxCNC simulator for this change. Generated G-code is the user's to verify before it is run (NFR-12).
