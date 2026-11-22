# 0674: The baluster example moves to the current project schema and holds the Z1's minimum clearance radius (SET-01, SET-06, NFR-12; issues #809 and #797; lane Z1-SETUP)

- **Status:** proposed by lane Z1-SETUP on 22 Nov 2026. A data and test change within decision 0649 item 11 ("the sample and example projects follow the built-in Z1"), which the owner accepted; it records what was done and how it was checked.
- **Date:** 22 Nov 2026.
- **Requirements:** SET-01 (the project keeps its own copy of the machine), SET-06 (the minimum clearance check, [decision 0495](0495-minimum-clearance-and-clearance-from-stock.md)), GC-07, NFR-12.
- **What it amends:** nothing. It carries out [decision 0649](0649-the-z1-profile-takes-the-firmware-1-1-3-measurements.md) item 11 for the one field the baluster example's schema could not hold, and settles the baluster part of OI-2149.
- **Open items:** OI-2149 (the baluster part done; the reference jobs of `testdata/projects/` stay open for the owner's choice after the first release).

## Context

The baluster example (`testdata/examples/baluster/baluster-project/`, the tutorial's finished project) was a project schema 25 folder. Its machine copy followed the built-in Makera Z1 except for `min_clearance_radius = 45.0`: schema 26 added that field (decision 0495), and a schema 25 file holding it is refused as premature. So its Setup step showed "Differs from built-in makera-z1 in: minimum clearance radius" with a blank **Minimum clearance**, and its tool 104 warned that it may take the spindle above the top of the Z travel, where a new Z1 project's warns that it may be too long to clear the workholding (OI-2149, issue #809).

Issue #797 asked for the blisk example's generator (`testdata/examples/blisk-start/generate.py`) to stop writing the Z1's old A limit of 3600 deg/min. That was already done on `main` by lane PROFILE-1.1.3 (#798, decision 0649, "Consequences": the generator writes the example's machine text byte for byte as committed); this lane found nothing left to change and adds a test that keeps it so.

## Decision

1. The baluster project is opened and saved with the current build (`openchuck-cli resave`), which runs the project migrations 25 to 28: every file's `schema` becomes 28, and the 26 to 27 step writes `exact_contact = true` into the Finish operation (helical finishing, which machined with exact contact before, decision 0526). Nothing else changes.
2. Its machine copy then takes `min_clearance_radius = 45.0` and its provenance, as in the built-in Z1, so the copy equals the built-in profile value for value (the Setup step says it is the same as the Makera Z1).
3. A test, `example_machines.rs` in `openchuck-app`, keeps every example project's copy equal to the built-in Z1 at a schema that holds the minimum, and keeps the blisk generator's embedded machine text equal to the committed `machine.toml`.

## Consequences

- **No G-code changes.** The Z1 program the example writes is byte for byte the same before and after (checked with `openchuck-cli regenerate --post makera-z1`).
- **Its tool 104 now warns as a new Z1 project's does:** "tool 104 may be too long to clear the workholding within the Z travel …", a possible collision to acknowledge before the files are written (decision 0538). The command line's `regenerate --out` then needs `--acknowledge-collision-warnings`, as for the tutorial (decision 0650).
- **The user guide's worked examples on this project** show the Setup step and Messages: they are re-captured and their text follows (see Verification).
- The reference jobs of `testdata/projects/` (schema 17) still lack the field; migrating them changes their goldens' project files and is left to the owner's choice after the first release (OI-2149).
- Generated G-code is still the user's to verify before it runs (NFR-12).

## Verification

Run by lane Z1-SETUP on 22 Nov 2026 in a Linux cloud container (4 cores), branch `lane-z1-setup` from `main` at `57b8ac1b`.

- **The migration:** `openchuck-cli resave` on the folder (schema 25 to 28: every file's `schema`, and `exact_contact = true` in the helical Finish), then the machine copy made equal to the blisk example's copy, which is the built-in Z1's, with the baluster's `id` kept; resaved again with no change. `openchuck-cli report` shows `minimum: clearance radius 45 mm (from the machine)`.
- **G-code:** `openchuck-cli regenerate --post makera-z1 --tool-numbers keep` on copies before and after: `01-setup-1.nc` identical byte for byte (after, with `--acknowledge-collision-warnings`, which the new tool-length warning needs).
- **Tests:** `crates/openchuck-app/tests/example_machines.rs`: `req_set_01_req_set_06_the_example_projects_hold_the_builtin_z1` (the three example copies equal the built-in Z1, at schema 26 or later) and `req_set_01_the_blisk_generator_writes_the_committed_machine_copy` (issue #797). The tests that read the baluster example pass: `facing_approach.rs`, `held_region.rs`, `adaptive_example.rs`, `ramp_example.rs`, `descent_feed.rs` and `lead_example.rs` (49 tests, 5 min 50 s uninstrumented on their own; their slow ones were already slow).
- **The user guide:** the six worked examples on this project re-captured at `b0e4aafa` with `user-docs/tools/capture_example.py` (Xvfb, Mesa lavapipe; two clicks of `custom_machine.py` moved, since the machine section is shorter); each figure compared with the old one (`custom_machine` figure 1 and `flutes` figure 4 changed in substance, their text followed; the rest differ in the tool 104 warning's words or in rendering only). The command-line and create-a-post examples add `--acknowledge-collision-warnings`.
- **Whole workspace:** `cargo nextest run --workspace --locked --no-fail-fast`, uninstrumented: 3424 tests, 3421 passed and 3 failed in 61 min 31 s. The three were fixed and pass on their own: `openchuck-cli` `req_gc_17_req_set_01_req_set_03_req_set_05_req_set_06_cli_sets_machine_and_setup` (its stock starts at X −2 on the Z1, so it now expects the headstock warning, decision 0672), `xtask` `registers::tests::the_repository_registers_pass` (a `|` inside an open-item cell) and `xtask` `examples_zip::tests::the_real_examples_pack_and_unzip_cleanly` (a `__pycache__` folder left by checking the blisk generator, deleted). The `openchuck-app` unit tests were run again after the last text change (282 passed). No golden program changed and none was re-blessed.
- **Checks:** `cargo fmt --all`, `cargo clippy --workspace --all-targets --locked` (no warnings), `cargo xtask deps`, `licences`, `records` and `trace` (ok; 105 of 105 P1 requirements have tests). The user guide built with mdBook 0.5.4 (`user-docs/tools/build.py`, `assemble.py`) and `user-docs/tools/check.py`: 0 errors, 0 warnings.
- **Test times:** every new test takes under 1 s uninstrumented; none is long-running (AGENTS.md).
- **Not verified:** the reference jobs of `testdata/projects/` (still schema 17, OI-2149).
