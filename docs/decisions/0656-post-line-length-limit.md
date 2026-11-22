# 0656: A post's line-length limit, checked before export (GC-08, GC-09, NFR-12; issue #703, OI-933; lane POST-SAFETY)

- **Status:** Proposed by lane POST-SAFETY on 22 Nov 2026. The owner chose item 1 and option A on 22 Nov 2026, relayed by the coordinator. Acceptance of this record's text is pending (OI-2157). It is built in a second pull request of the lane, after the 0.55.0 crates are published, with the workspace minor version bump it needs (decision 0048), not in the first.
- **Date:** 22 Nov 2026.
- **Requirements:** GC-08 (check generated G-code before export, P2), GC-09 (the Z1 post), GC-03 and GC-04 (post definitions and options), GC-07 and NFR-06 (deterministic output), NFR-07 (clear errors), NFR-12 (the user verifies G-code).
- **Related:** [decision 0222](0222-short-moves-minimum-block-time.md) (where OI-933 was raised), [decision 0533](0533-safety-notice-post-option.md) (the notice lines), [decision 0540](0540-the-command-line-checks-before-it-writes.md) (the command line runs the export checks).
- **Open items:** OI-933 (the limit itself; this record would close its software half), OI-2157 (acceptance of this record, and the build in the second pull request).

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

To be filled in by the second pull request.
