# 0649: The built-in Makera Z1 takes the owner's firmware 1.1.3 measurements (SET-01, SET-06, SET-10, UI-13, GC-14, NFR-12; issues #586 and #793; lane PROFILE-1.1.3)

- **Status:** Proposed by lane PROFILE-1.1.3 on 14 Nov 2026. The owner's decisions 1 to 4 below were relayed by the coordinator the same day (decision 0049); the record itself awaits the owner's acceptance, which the coordinator relays.
- **Date:** 14 Nov 2026.
- **Evidence:** the owner's tests of 0.55.0-rc2 on the Z1, firmware 1.1.3 and ESP32 1.1.3, 11 to 14 Nov 2026, recorded in issue #586 comment 6805176778, and the owner's measurements of 14 Nov 2026 relayed by the coordinator.
- **What it amends:**
  - [Decision 0013](0013-z1-profile-consensus.md) (accepted), for the top of the Z travel: −1, not 0.
  - [Decision 0317](0317-z1-measured-axis-height-and-safe-z.md) (accepted): the default safe cylinder radius and safe Z are 45, not 40, and the recommended minimum safe Z is 45, not 30.
  - [Decision 0318](0318-z1-spindle-height-warning-switched-on.md) (accepted): the spindle-to-axis distance is measured, 72.0, not derived, 73.1.
  - [Decision 0495](0495-minimum-clearance-and-clearance-from-stock.md) (accepted) D4: the Z1's minimum clearance radius is no longer blank: 45, confirmed for the supplied chuck with its jaws reversed and fully open.
  - [Decision 0231](0231-machine-spindle-speed-range.md) (accepted) item 3: the Z1's spindle range is 4000 to 13000 rpm, confirmed.
- **Builds on:** [0647](0647-the-z1-turns-a-at-most-1800-deg-per-min.md) (the A limit, from the same `M203.1` reading), [0648](0648-the-block-rate-air-cut-runs-one-section-per-file.md) (7 ms on 1.1.3), [0007](0007-sp-12-z1-conventions.md) (the SP-12 conventions).
- **Open items:** OI-1780 and OI-982 closed; OI-1292 narrowed, still open; OI-2146 new (the user guide). OI-60 is under review with the owner (the rotary reference point, below).

Requirements and architecture are not edited by this record. No project or machine schema changes: every value changed here already has a field (machine profile schema 8). No public API of the library family changes.

## Context

The built-in Z1 profile mixed values the owner had confirmed, values from Makera's published profiles and firmware configuration, and program defaults (decision 0013). The owner's rc2 testing on firmware 1.1.3 checked several of them on the machine.

What was measured or observed (owner, 11 to 14 Nov 2026):

1. **Maximum rates.** `M203.1` prints X 20, Y 20, Z 10 (mm/s) and A 30 (deg/s): X and Y 1200 mm/min and Z 600 mm/min, as the profile has; A is decision 0647's change.
2. **Block time.** The split block-rate air cut (decision 0648): 7 ms smooth in both parts, 5 ms stuttering. The profile's `min_block_ms` 7 holds on 1.1.3.
3. **SP-12 conventions.** The rotary axis along X, the A direction and the continuous A range hold; `G93` is ignored, as on 1.1.2.
4. **Axis height.** 48.568 mm, consistent with the profile's −48.6. With G54 Z0 on the axis, the machine Z was −67.657 with a tool length offset of −19.09 and −68.180 with −19.612: each is the axis height plus the offset.
5. **Top of the Z travel.** A soft limit alert triggers at machine Z −1. The profile's top was 0. The alert's text and what the machine does at it are not yet recorded.
6. **Spindle to axis.** With no tool and the tool length offset calibrated to the collet face (offset −24.433), G54 Z0 on the axis reads machine Z −73.000: the collet face at machine Z 0 is 73.000 mm above the axis. A dummy tool 4.821 mm from the collet face (offset −19.612, machine Z −68.180) gives 73.001. The profile's 73.1 was derived (48.6 + 24.5, the probe's stick-out by caliper, decision 0318); the probe's stick-out on the tool setter is 24.433.
7. **Clearance.** A clearance radius of 45 mm is needed for the supplied Sanou K02-50 chuck with its jaws in the reverse configuration, fully open (60 mm chuck opening). The profile's default was 40 and its minimum blank.
8. **Spindle range.** With `T1 M6` and no tool, the spindle runs when S is 1000 or above, and no slower than 4000 rpm. The owner states that 13000 rpm has been tested.
9. **Tool change.** `M5`, `T1 M6`, `T2 M6`, attended with no material: the tool change is shown, the tool touches off on the measurement switch, and the machine returns to the G28 position; the offsets are as expected (OI-41, closed). `G28` goes to machine Z −3, then X −11.6 Y −14.6.
10. **Clearance at safe Z** (gate 4 step P10): about 13 mm clear.

![Side view of the Z1's spindle against the rotary axis: the collet face 73.000 mm above the axis at machine Z 0, 72.0 at the soft limit at machine Z −1; the probe's tip on the axis at machine Z −48.6; the safe Z and minimum clearance radius of 45 mm](0649-the-z1-profile-takes-the-firmware-1-1-3-measurements.svg)

## Decision

The owner's decisions of 14 Nov 2026, relayed by the coordinator, one at a time:

1. **The top of the Z travel is −1, and the spindle-to-axis distance is 72.0** (owner's decision 1, option A).
   - `travel.max.z = -1.0`, still *unconfirmed*: the alert's text and behaviour are not recorded, and the other five travel extents are untested (OI-1292 stays open).
   - `spindle_to_axis = 72.0`, *confirmed* (measured): 73.000 from the collet face at machine Z 0, less 1 mm to the soft limit. The field is defined as the distance from the spindle at the top of the Z travel down to the axis (`MachineProfile::spindle_to_axis`), so the two values move together. The spindle-height check (decision 0318) now warns for a tool 1.1 mm shorter than before.
   - No warning for a project holding an old copy of the Z1: the difference is about 1 mm, and the machine's own soft limit stops it.
2. **The minimum clearance radius is 45, and the default clearance is 45** (owner's decision 2, option B, with the owner's amendment).
   - `min_clearance_radius = 45.0`, *confirmed*, for the supplied Sanou K02-50 chuck with its jaws reversed and fully open (60 mm opening) only. Other chucks, jaw configurations and a tailstock are not covered; the user sets the setup's own minimum for them (decision 0495 item 1). OI-1780 closes with that scope.
   - `clearance.radius = 45.0` and `clearance.safe_z = 45.0`, still *unconfirmed* (a program default, set to the measured minimum). The default then clears the measured worst case and adds no warning of its own. With the probe fitted, its tip is at machine Z −3.6 at safe Z 45, 2.6 mm below the top of the travel.
   - Every tool whose flute length + 5 mm is over 27.0 mm (flute over 22.0 mm) warns that it may be too long to clear the workholding within the Z travel (decision 0495 item 6b); at safe Z 45 the same tools also fail the spindle-height check.
3. **The spindle runs from 4000 to 13000 rpm** (owner's decision 3, option A). `min_spindle_rpm = 4000.0`; both ends *confirmed*: the lowest observed, the highest on the owner's statement (no run details recorded). An operation below 4000 rpm blocks Post and export (decision 0231 item 2); none of the sample projects, example projects or tool libraries is below it. OI-982 closes.
4. **The recommended minimum safe Z is 45** (owner's decision 4): advice, never a limit, from the owner's measurement for one chuck and jaw setup. It stays *unconfirmed* pending the owner's answer below.
5. **Provenance only** (no decision needed): the maximum feeds cite the `M203.1` reading for X, Y and Z; `min_block_ms` cites 1.1.3 and decision 0648; the rotary axis, A direction and A range cite the 1.1.3 recheck; the rotary origin cites the axis height of 48.568; the tool length measurement cites the 1.1.3 tool change. Every value still unconfirmed names what is missing.

**Pending with the owner** (asked by the lane, relayed one at a time; this record is updated before the release push):

- whether the default clearance (45/45) and the recommended minimum safe Z (45) are *confirmed*;
- the tutorial's tool now warns (item 2): how the tutorial should explain it;
- the rotary reference point, X and Y (OI-60): the owner's G54 Y in 4-axis mode is −123.39 against the profile's −108.8 (the published default configuration), and the owner's X0 is 83.050 from the face of the headstock body next to the chuck.

### What is still unconfirmed, and the test that settles it

| Field | Value | Missing | Test |
| --- | --- | --- | --- |
| Travel | X −200..0, Y −200..0, Z −100..−1 | The alert's text and behaviour at Z −1; the other five extents | With nothing fitted, jog each axis to its end and read the machine coordinates (G53) and any alert; with the rotary module, whether Y is held above −160 |
| Clearance | 45, 45 | A program default (pending the owner's answer) | None on the machine: the measured minimum it is set to is confirmed |
| Block window | 32 | A firmware-source value | None planned: the published planner model fails below 5 ms (decision 0648) |
| Short moves | Report | Software policy | Not machine-confirmable |
| Recommended minimum safe Z | 45 | Advice (pending the owner's answer) | None on the machine |
| Highest tool number | 1000 | The probe's tool number; whether `T1000 M6` is accepted | Read the probe's number on the controller; send `T1000 M6` with no tool |
| Rotary reference X and Y | −180.4, −108.8 | The X datum; the Y the machine uses in 4-axis mode | Under review with the owner (OI-60) |
| Rapid rates | Equal to the feed limits | G0 not timed separately | Time a long G0 X move against the same move at G1 F1200 |

## Options not chosen

- **Decision 1, option B:** the top at −1 but the spindle-to-axis distance kept at 73.1. The spindle-height check would allow the spindle 1 mm past the soft limit.
- **Decision 1, option C:** no value change. The profile would keep a top the machine does not reach.
- **The owner's formula 100 − 1 − 48.6 = 50.4** for the spindle-to-axis distance (asked and answered, 14 Nov 2026): it uses the bottom of the travel, which the distance does not involve, and is not a distance on the machine. The distance from the axis down to the bottom of the travel is 51.4 (−48.6 to −100, untested), a question for short tools near the axis (OI-1781).
- **Decision 2, option A:** the minimum 45 with the defaults kept at 40. Every new Z1 setup would also warn that its safe cylinder is inside the minimum, with the same long-tool warnings as B.
- **Decision 2, option C:** no value change. Nothing would warn about the jaws.
- **Decision 3, option B:** a lowest speed of 1000. Speeds of 1000 to 3999 would run at 4000, faster than programmed, without a warning.

## Consequences

- **New projects** take the new values. A project made with an earlier build keeps its own copy of the profile (SET-01) and gives the old results; the user chooses the Makera Z1 again in the setup's machine section to take the new ones. No warning is added for an old copy (decision 1).
- **Golden programs are unchanged.** The sample and example projects carry their own machine copies, which this record does not change; no G-code changes for them (GC-07).
- **The tutorial's tool warns.** *Z1 O-flute 3.175x25* (flute 25 mm, assumed stick-out 30 mm) reaches at most 42.0 mm above the axis, inside the 45 mm minimum. This is what decisions 1 and 2 imply under the flute + 5 mm assumption of decision 0318. The tutorial's wording is with the owner; the guide lane of OI-2146 makes the change.
- **The unconfirmed count** of the built-in Z1 drops from 6 to 4 (travel, clearance, block window, recommended minimum safe Z).
- **The user guide** (OI-2146): this change updates the Z1 tables in `reference/machine-profiles.md` and `how-to/confirm-machine-values.md` and the release notes; the figures, counts, tutorial prose and listings go to one guide lane with OI-2145.
- **`testdata/examples/blisk-start/generate.py`** writes the example's machine text byte for byte as committed (issue #797: it still had the A limit of 3600).
- Generated G-code is still the user's to verify before it runs (NFR-12). None of these values makes a program safe on a machine.

## Verification

- **Environment:** branch `claude/lane-profile-113` from `main` at `90e78107`.
- **Tests (written from the decisions first, watched failing, then passing):**
  - `crates/openchuck-project/src/machines.rs`: `req_set_01_builtin_z1_reads_and_marks_every_value` (every field marked, the minimum included; unconfirmed: travel, clearance, block window, recommended minimum safe Z), `req_set_01_req_set_06_the_z1_axis_height_is_measured_and_its_safe_z_is_in_the_travel` (45/45, top −1, safe Z at machine Z −3.6), `req_set_06_req_set_01_the_z1_minimum_clearance_is_measured_for_the_supplied_chuck` (new), `req_set_01_req_ui_13_the_z1_measures_tool_lengths_and_gives_its_spindle_to_axis_distance` (72.0, measured), `req_gc_17_req_set_01_new_z1_uses_measured_report_settings_and_old_profiles_stay_off` (1.1.3 cited), `req_gc_14_req_set_01_the_z1_spindle_range_is_confirmed_and_needs_schema_4` (renamed from `..._is_unconfirmed_...`, which decision 0231 names).
  - Older-schema fixtures in `machines.rs`, `tests/setup.rs` and `tests/z1_block_time.rs` are given a Z1 without the minimum, as those schemas had none.
- **Not verified:** the values on any other Z1, chuck or firmware; the soft limit's alert and behaviour; the G0 rates.
