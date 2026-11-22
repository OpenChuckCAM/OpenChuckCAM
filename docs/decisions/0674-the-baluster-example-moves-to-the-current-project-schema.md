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

To be completed before the push.
