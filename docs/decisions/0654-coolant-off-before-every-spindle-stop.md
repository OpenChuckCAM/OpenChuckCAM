# 0654: A post that turns coolant on turns it off before every spindle stop (GC-03, GC-04, NFR-12; issue #598; lane POST-SAFETY)

- **Status:** Proposed by lane POST-SAFETY on 22 Nov 2026. The owner chose option A on 22 Nov 2026, relayed by the coordinator; built in lane POST-SAFETY's pull request. Acceptance of this record's text is pending (OI-2155).
- **Date:** 22 Nov 2026.
- **Requirements:** GC-03 (template posts), GC-04 (post options), GC-07 and NFR-06 (deterministic output), NFR-12 (the user verifies G-code). Related: decisions [0019](0019-sp-10-template-engine.md) item 4 (the typed options) and [0020](0020-slice-6-post-and-round-trip.md) (the built-in posts).
- **Open items:** OI-2155 (acceptance of this record).

Requirements and architecture are not edited by this record. No project or machine schema changes.

## Context

The post option `coolant` has three values, `off`, `flood` and `mist`. Its doc comment (`crates/openchuck-post/src/options.rs`) says flood is "M8, off with M9" and mist "M7, off with M9". Read on `main` `57b8ac1b`, both built-in templates (`posts/makera-z1/main.post` and `posts/linuxcnc/main.post`, which are identical):

- write `M8` (flood) or `M7` (mist) on the line after every `S<rpm> M3`;
- never write `M9`: not before the `M5` of a tool change, not before the footer (`M5 ... M2` on the Z1, `M5 M30` on LinuxCNC).

So a custom post with `coolant = "flood"` or `"mist"` leaves coolant running through every tool change and past the end of the program, unless the user's footer happens to contain `M9`. On LinuxCNC `M2`/`M30` turn coolant off; on the Z1 it is not known (no Makera document checked). Both built-in posts ship `coolant = "off"`, which writes no coolant code, so no built-in output and no golden file is affected. The user guide's post reference already tells readers to leave it off (issue #598).

## Options

- **A (recommended): finish it.** When coolant has been turned on, the template writes `M9` before the next tool change (before its `M5`, when `tool_change.stop_spindle_first` is on) and before the footer. `M8`/`M7` stay on the line after each `S<rpm> M3`, so coolant comes back on with the spindle after a tool change. The template tracks whether coolant is on, so no `M9` is written before the first tool change, when nothing has turned it on. The footer is unchanged (a user footer with its own `M9` gives two, which is harmless). Built-in output is unchanged (coolant off). Tests `req_gc_04_*` on a copy of each built-in post with flood and with mist: every `M8`/`M7` is followed by an `M9` before the next `M5` and before the end code, and nothing else changes.
- **B: remove `flood` and `mist`.** `Coolant` keeps only `off`; a post file with another value is refused when it is loaded, naming the key (as an unknown value is today). The doc comment and the guide say coolant codes are not written. A breaking change to the library family's public enum (decision 0048: workspace minor bump, after the 0.55.0 crates publish).
- **C: keep the values, correct the words.** The doc comment and the guide say the post turns coolant on and nothing turns it off; the user writes `M9` in the footer. Leaves a machine state on that a user would not expect.

## Recommendation and choice

The lane recommended option A, and the owner chose it (22 Nov 2026, relayed). It makes the option do what its comment and its name promise, changes no built-in output, needs no public API change (a template change and a doc comment) and closes the gap without taking a feature away. The written program is still the user's to verify before it is run (NFR-12).

## Decision (option A)

1. When coolant has been turned on (`M8` or `M7` after an `S<rpm> M3`), the template writes `M9` before the next tool change, before its `M5`, and before the footer. The template tracks the state, so an `M9` is written only while coolant is on: none before the first tool change of a program, when nothing has turned coolant on yet.
2. `M8`/`M7` are written after every `S<rpm> M3`, as before, so coolant comes back on with the spindle after each tool change.
3. With `coolant = "off"` nothing changes.

## Consequences

- Both built-in templates change identically (a test keeps them the same). Golden files do not change.
- The doc comment of `Coolant` says where `M9` is written; the user guide's post reference (`writing-a-post.md`, `coolant` row) describes the three values as working, with where each code is written.
- Decision 0533's notice and every other line are unchanged; output stays byte-identical for the same project, post and version (GC-07, NFR-06).

## Verification

See [decision 0657](0657-post-safety-fixes-without-a-design-choice.md), *Verification*: test `req_gc_04_req_nfr_12_coolant_goes_off_before_every_tool_change_and_the_end` (`openchuck-post`, both built-in posts with flood and with mist, a two-tool program), watched failing on the unchanged templates; the built-in golden tests pass unchanged.
