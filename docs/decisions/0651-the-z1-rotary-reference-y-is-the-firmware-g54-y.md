# 0651: The Z1's rotary reference Y is the G54 Y the firmware sets, −124.39 (SET-01, SET-03, NFR-12; issue #821; lane Z1-ORIGIN-Y)

- **Status:** Accepted by the project owner on 22 Nov 2026, relayed by the coordinator (decision 0049). Proposed by lane Z1-ORIGIN-Y on 21 Nov 2026. The owner asked for the fix (issue #821); the owner's answers A, B and C below were relayed by the coordinator on 21 Nov 2026.
- **Date:** 21 Nov 2026.
- **Evidence:** the owner's `$#` on the Z1, firmware 1.1.3, 21 Nov 2026, as given in issue #821: `G54: -118.3900, -124.3900, ...`.
- **What it amends:** [Decision 0649](0649-the-z1-profile-takes-the-firmware-1-1-3-measurements.md) (accepted), Context item 11 and Decision item 5, and the places in its Consequences and Verification that repeat the value: the rotary reference Y is −124.39, not −123.39. Its top-view drawing labels the axis "Y −123.39"; read it as −124.39. 0649's text is left as accepted.
- **Open items:** none new. OI-60 and OI-2146 (closed) note the correction.

Requirements and architecture are not edited by this record. No project or machine schema changes. No public API of the library family changes.

## Context

Decision 0649 item 11 gives the owner's G54 in 4-axis mode as X −115.34 and Y −123.39, and withdraws an earlier Y −124.39 as "possibly not in 4-axis mode". The owner now reports (issue #821) that −123.39 was a typo. G54 is not measured on the Z1: the firmware sets it in the 4-axis setup, and the owner's `$#` on firmware 1.1.3 on 21 Nov 2026 lists G54 Y as −124.3900. The withdrawal was the mistake.

A probe measurement on the same day found the axis at about Y −124.14 near the working end of a rod. The headstock had been nudged that day, so this supports −124.39 but is not the value used.

The same `$#` lists G54 X as −118.39, where 0649 item 11 read −115.34. The two readings agree with the headstock face at machine X −198.39 (derived from them, not a change; the owner's answer C, relayed): −118.39 is the face plus the firmware's default X0 offset of 80.000 mm, and 0649's −115.34 is the face plus the owner's own choice of 83.050 mm.

## Decision

1. **The built-in Makera Z1's `rotary_origin` Y is −124.39**, *confirmed*: the G54 Y the firmware sets in the 4-axis setup on firmware 1.1.3. X (−198.39) and Z (−48.6) are unchanged.
2. **The profile's comment and `source` text say where Y comes from**: the firmware's G54 Y from the owner's `$#`, with 0649's −123.39 named as a typo corrected here. The sentence withdrawing −124.39 is removed.
3. **The sample and example projects follow the built-in Z1** (0649 item 11): the 15 committed machine copies and `testdata/examples/blisk-start/generate.py` take the same value and text.
4. **No warning for a project saved with the old copy** (the owner's answer B, relayed). Such a project keeps Y −123.39 in its own copy of the profile (SET-01) and gives the same G-code, since programs are in work coordinates; it shows the expected G54 Y as −123.390 and checks the Y travel 1 mm off. Choosing the Makera Z1 again in the setup's machine section takes the new value, as in 0649 item 5. Option not chosen: an old-copy warning keyed on Y −123.39, like the one for the old axis height (`has_old_z1_axis_height`); the firmware sets G54 itself in the 4-axis setup, so the user does not type the Y shown.
5. **`rotary_origin` X stays −198.39** (the owner's answer C, relayed): the 21 Nov G54 X of −118.39 is the same face with the firmware's default 80.000 mm X0 offset (Context).

## Consequences

- **No G-code changes.** Programs are written in work coordinates, relative to G54 with Y0 on the axis, and carry only the word `G54`. No golden program changes and none is re-blessed. The gate 3 replay inputs are the same programs.
- **The expected work offset shown changes.** For a Z1 project with the new value, the setup checks and the command line's `setup` report show Y −124.390, not −123.390 (for example "set G55 to machine X −200.390 Y −124.390 Z −48.600 at A 0"). The command line's `machine` report shows the new reference point. The setup sheet does not show machine Y and does not change.
- **The travel check moves by 1 mm.** The program check (`posting.rs`) converts the Y travel to program coordinates with this point: Y −35.61 to +123.39, not −36.61 to +122.39. No sample program comes near it (the largest |Y| written is 13, swarf).
- **Project fingerprints** (decision 0207) of saved samples change with their machine copy. No golden or record holds a sample's fingerprint.
- **The user guide's** Z1 table (`reference/machine-profiles.md`) and the 0.55.0 release notes give −124.39. Screenshots are not re-captured.
- **Freeze:** the change is after the 0.55.0-rc3 pre-release, made at the owner's request for this one fix.
- Generated G-code is still the user's to verify before it runs (NFR-12).

## Verification

Run by lane Z1-ORIGIN-Y on 21 Nov 2026 in a Linux cloud container (4 cores), branch `claude/lane-z1-origin-y` from `main` at `37211de9`.

- **Before any edit: what the value reaches.** The change (profile, the 15 copies, `generate.py`, the three tests that state the value) was made in a scratch worktree and `cargo test --workspace --locked --no-fail-fast` run on it: 3421 passed, 0 failed, 3 ignored (the usual ones). No golden program differed and none was re-blessed, so the Z1 and LinuxCNC programs of every sample and reference job, first_cut and the blisk example are unchanged. Read from the code: the G-code carries only the work offset word (`G54`); the setup sheet's work offset line gives the register and model X, not machine Y (`setup_sheet.rs`); the expected work offset (`expected_work_offset`) feeds the setup checks, the command line's `setup` report and the travel check (`posting.rs`); the fingerprint hashes the saved files (`saved_files_trace`). The largest |Y| in any Z1 golden is 13 (swarf), against the program-coordinate Y limits of −35.61 and +123.39.
- **Tests** (the Z1 test changed first and watched failing, then passing):
  - `crates/openchuck-project/src/machines.rs`: `req_set_01_req_set_06_the_z1_axis_height_is_measured_and_its_safe_z_is_in_the_travel` expects Y −124.39, and its source text to cite the `$#` reading (`-124.3900`) and this record (`0651`).
  - `apps/openchuck-cli/tests/cli.rs`: `req_gc_17_req_set_01_req_set_03_req_set_05_req_set_06_cli_sets_machine_and_setup` expects "set G55 to machine X -200.390 Y -124.390 Z -48.600".
  - `crates/openchuck-app/src/facing.rs`: `req_op_24_req_gc_09_req_set_01_z1_footer_home_is_the_observed_g28_point` uses the corrected point.
  - `cargo test -p openchuck-project -p openchuck-app -p openchuck-cli --locked --no-fail-fast`: 1164 passed, 0 failed, 1 ignored (26 min, uninstrumented; no new test). Every golden comparison in these crates passes unchanged.
  - The STEP jobs: `cargo test -p openchuck-app --features kernel-tests --locked --test step_pipeline` with `openchuck-step-worker` built with the kernel (OpenCASCADE through `occt-sys`) and `OPENCHUCK_STEP_WORKER` set: 11 of 11, including the drilling, swarf and face_flow golden programs.
  - `testdata/examples/blisk-start/generate.py`: its `MACHINE` text equals the committed `blisk-project/machine.toml` byte for byte.
- **Checks:** `cargo fmt --all` (no change), `cargo clippy --workspace --all-targets --locked` (no warnings), `cargo xtask deps`, `licences`, `records` and `trace` (ok).
- **Coverage not measured:** `cargo-llvm-cov` is not installed in the container. No product code line changes (profile data, sample data, test assertions and comments only), so no line's coverage changes.
- **Not verified:** the value on the machine (it is the owner's `$#` reading); the user guide's screenshots, which are not re-captured and may show the old expected Y.
