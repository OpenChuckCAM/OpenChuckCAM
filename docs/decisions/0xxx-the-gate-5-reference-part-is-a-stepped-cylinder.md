# 0xxx: The gate 5 reference part is a stepped cylinder, compared with the simulated part (issue #826; lane STEPPED-CYL)

- **Status:** proposed (23 Nov 2026) by lane STEPPED-CYL, phase 1 (documents only). Nothing is built until the project owner accepts it. The owner answers questions Q1 to Q7 below (OI-9901). Each question gives options and a recommendation. The coordinator relays the answers (decision 0049). The number is provisional: the coordinator assigns the final record and `OI-` numbers when the Merge Manager releases the pull request.
- **What it would amend (on acceptance):** [decision 0221](0221-gate-5-first-cut-project-and-limit.md) item 2 (the part: a cone becomes stepped cylinders) and the comparison behind its ±0.1 mm limit (the model becomes the simulated part); [verification plan](../verification-plan.md) section 4.1, row "First-cut part (gate 5)", and section 5, gate 5 row; the method of OI-2152 for setting X0 (Q4). Decision 0221's ±0.1 mm limit is **not** changed by this proposal unless the owner chooses so in Q2.
- **Date:** 23 Nov 2026.
- **Requirements:** none covers a gate 5 reference part (decision 0221 said the same of the cone). Q7 proposes wording. The phase 2 tests trace to the IDs the cone's tests already use: GC-07, NFR-06 and NFR-08 (golden programs), GC-10 (gate 3), SIM-03 and SIM-05 (simulation), SET-09 and OP-21 (held regions and tabs), GC-12 (setup sheet). NFR-12 throughout: generated G-code is the user's to verify before it is run, and nothing here says a program is safe or verified.
- **Builds on:** [decision 0221](0221-gate-5-first-cut-project-and-limit.md) (accepted; the first-cut project, tooling board, the ±0.1 mm limit); [0297](0297-helix-strategies-turn-at-faces-across-the-axis.md) (accepted; a helix turns once at each face across the axis); [0161](0161-tools-keep-clear-of-the-gripped-length.md) and [0160](0160-held-region-is-the-stock-section.md) (held regions); [0244](0244-bounded-first-cut-linuxcnc-replay.md) (bounded gate 3 replay of the first-cut program); [0649](0649-the-z1-profile-takes-the-firmware-1-1-3-measurements.md) and [0651](0651-the-z1-rotary-reference-y-is-the-firmware-g54-y.md) (the Z1 reference setup); [0653](0653-verification-report-for-release-0-55-0.md) (the 0.55.0 verification report and the waiver of the partial rc3 first cut, OI-2151).
- **Open items:** adds OI-9901 (the owner's answers to Q1 to Q7) and OI-9902 (phase 2: the build). Depends on OI-2152 (the repeat first cut; Q4 refines its X0 method and leaves the decision to the owner) and OI-2154 (the gates 4 and 5 instructions are stale; phase 2 rewrites their section 5, and the rest of OI-2154 stays with its docs lane unless the owner asks this lane to take it). Neither is closed here.

Numbering: provisional ("0xxx"; OI-9901 and OI-9902 are numeric placeholders, because `cargo xtask records` refuses an `OI-` number that is not a number), on the owner's rule of 23 Nov 2026 (reference numbers are monotonic in merge order; no reserved blocks). No project schema number: the phase 2 project is written at the current schema and nothing stored changes format. No spike.

## Context

The rc3 first cut (issue #586, [rc3 test record part 3](../archive/0653-release-0-55-verification/rc3-test-record-part-3.md), owner's measurements of 22 Nov 2026) cut the cone of decision 0221: 20 mm diameter at X 73, falling to 10 mm at X 98, between the 10 mm tab (X 68 to 73) and the 2 mm tailstock piece (X 98 to 100), on 35 x 35 x 100 mm tooling board. The cut ran with no unexpected event and matched the simulation, but the diameters could not test the ±0.1 mm limit:

- A cone loses 0.4 mm of diameter per mm of X. The shoulder was measured at X 74.43 against 73.0: the part sat about 1.4 mm toward the tailstock, which reads as about 0.56 mm of diameter. The five E-26 differences were +0.14 to +0.49 mm.
- The residuals about the best-fit line were about ±0.12 mm, more than the limit under test. A generic digital vernier (0.01 mm) with flat jaws also reads high on a taper.
- With the shift allowed for, the radii were under by about 0.05 to 0.15 mm (tab 9.8 mm against 10). This could not be separated from the position error.
- E-26 compared with the model, but the simulation expects material left: up to 0.889 mm at the unfinished tail (fed moves stop at X 95.9 beside the held end, decision 0161).
- Removing the cone from the stock to measure it removed part of the tab.

The record's own proposals: compare the measured radius with the simulated radius at the same X and report simulation against model separately; set X0 by probing the stock's end face; measure in the machine with the probe (top touches at A0 and A180); use a stepped-cylinder part. Issue #826 asks for that part.

**What a stepped cylinder changes.** On a cylinder the diameter does not depend on where along X the instrument sits, so a micrometer or vernier reads size alone. The faces between steps (square to the axis) give X positions directly. A work-zero error then shows as a position error, separate from a size error. Steps of several diameters also separate a radial error that is the same at every radius (tool length measurement, axis height) from one that grows with radius (a Z scale error).

## What this lane measured before proposing (scratch, not committed)

To ground the options, the lane built `openchuck-cli` in release mode at `main` `8f61706` and made scratch projects with the CLI. Each was made as `build_first_cut` in `apps/openchuck-cli/tests/e2e.rs` makes the first-cut project: the same stock, part offset, work X, held regions, tab, material and tools, and the committed project's two operations (rotary roughing with tool 104; helical finishing with tool 204, 5 degree steps). Only the part changed: an exactly revolved STL written by a small script, with walls square to the axis. Programs come from `regenerate --post makera-z1 --grouping operation`; material comes from `simulate` at the default 0.1 mm cell. None of this is committed; phase 2 builds the chosen part with tests. The scratch cone reproduces the rc3 simulation (largest material left 0.889 mm at the tail, deepest overcut 0.049 mm; rc3 record E-10), so the geometry and held regions match the committed first-cut project. Its roughing estimate does not: 41:03 against the committed program's 55:52 (rc3 air cut, C-02). The cause was not looked into: the committed project was written at schema 17, the scratch ones at today's schema 28. So the table gives relative costs only, and phase 2's golden program gives the real estimate.

The times are the program's own estimate (the post's "cut" time). Each is compared with the scratch cone built the same way, not with the committed golden program: the rc3 air cut's setup sheet gave 55:52 and 12:00 for the committed project. These are estimates, not machine times.

| Part (program X 73 to 98) | Roughing | Finishing (helical, ball 204) | Total | Against the cone | Face turns (decision 0297) |
| --- | --- | --- | --- | --- | --- |
| Cone 20 to 10 mm (today's part) | 41:03 | 11:41 | 52:44 | — | 1 (the face beside the tab) |
| **A. Ø24 x 7, Ø17 x 8, Ø10 x 10** | 45:41 | 12:07 | 57:48 | +9.6 % | 3, at X 71.41, 81.59, 89.59 |
| B. Ø24 x 7, Ø18 x 8, Ø12 x 10 | 44:09 | 12:07 | 56:15 | +6.7 % | 3 |
| C. Ø24 x 12, Ø12 x 13 | 40:11 | 11:54 | 52:05 | −1.2 % | 2 |
| D. Ø24 x 6, Ø20 x 6, Ø15 x 6, Ø10 x 7 | 48:04 | 12:20 | 60:24 | +14.5 % | 4 |

Finishing alternatives on part A (Q5):

| Finishing of part A | Time | What it leaves |
| --- | --- | --- |
| Helical, ball 204 (as today) | 12:07 | Cylinders: scallop about 0.016 mm at the 0.4536 mm pitch (calculated: r − √(r² − (s/2)²), r 1.5875). Walls: cut by the side of the tool on the turn at each face (decision 0297); the root of each wall keeps the ball's 1.59 mm radius |
| Helical, flat end 104 (the roughing tool) | 3:01 | Square roots; one tool, no tool change; pitch 2.22 mm |
| Axial, ball 204 (default angle step) | 30:03 | Walls cut on every pass; scallops between passes grow with radius; 20,337 moves |

Simulation of part A (0.1 mm cell): largest material left 0.502 mm, at the tail end of the Ø10 step (program X 98, beside the held end, where fed moves stop at X 95.9, as for the cone). Deepest overcut 0.045 mm, on the Ø24 step (program X 77.85), within one cell; the cone's was 0.049 mm. At a 0.05 mm cell the same part gives 0.023 mm (program X 74.46) and 0.500 mm left at the tail, so the overcut follows the cell size: a resolution effect of the simulation, not a cut into the part. A 0.025 mm cell is refused on this stock (more than the 64,000,000 surface samples supported). Phase 2 reads the simulated radius at the stations at the finest cell the stock allows, and states it.

## Questions for the owner (most important first)

### Q1. The part: number, diameters and lengths of the steps

All options keep decision 0221's layout and the #586 choices: stock 35 x 35 x 100 mm tooling board, chuck at X 0, the 68 mm held region at the start, the 10 mm tab at X 68 to 73, the part at X 73 to 98, and the tailstock piece (held end, 2 mm at 10 mm diameter) at X 98 to 100. Steps get smaller toward the tailstock, as the cone did, so the tailstock body sits beside the smallest diameter.

Where each step can be measured: the ball leaves a 1.59 mm radius at the root of each wall, on the smaller step, and fed moves stop at X 95.9, so the last 2.1 mm before the tailstock piece is not finished (as on the cone).

| Option | Steps (program X) | Clean length per step for a micrometer | Wall heights (radial) | Notes |
| --- | --- | --- | --- | --- |
| **A (recommended)** | Ø24 X 73–80; Ø17 X 80–88; Ø10 X 88–98 | 7.0, 6.4, 6.3 mm | 7 (tab to Ø24, facing the chuck), 3.5, 3.5 mm | Radii 12, 8.5 and 5 mm: a 2.4 : 1 spread. The last step meets the tailstock piece flush, as the cone's small end did. All three fit a 0–25 mm micrometer. Each tailstock-facing wall has about 1.9 mm of flat face above the root radius for a probe touch. +9.6 % cutting time |
| B | Ø24 X 73–80; Ø18 X 80–88; Ø12 X 88–98 | 7.0, 6.4, 6.3 mm | 7, 3, 3 mm | Radii 12, 9 and 6. About 1.4 mm of flat face per wall, tight for a probe touch. +6.7 % |
| C | Ø24 X 73–85; Ø12 X 85–98 | 12.0, 9.3 mm | 7, 6 mm | Longest flats and one tall wall, but only two radii: cannot tell a constant radial error from one that grows with radius. −1.2 % |
| D | Ø24, Ø20, Ø15, Ø10, X 73–79–85–91–98 | 6.0, 4.4, 4.4, 3.3 mm | 7, 2, 2.5, 2.5 mm | Most data, but the flats are shorter than a 6.35 mm micrometer anvil and the walls are too low for a probe touch beside the root radius. +14.5 % |

**Recommendation: A.** Three diameters are the fewest that check a straight-line fit of radius error against radius. Each flat is longer than a standard micrometer anvil, and each wall leaves room for a probe touch. The cost is about 5 minutes of roughing.

### Q2. The criterion and the limit (decision 0221)

What is compared: **the measured radius against the simulated radius at the same X** (as the rc3 record proposed), with the simulation against the model reported separately as the program's known deviation. On the cylinders of part A the two nearly coincide: the simulated part is the model plus the scallop (about 0.016 mm) inside the clean lengths. Positions: each wall's X against the simulated wall (the model's, since the turn cuts each face to the stock to leave, which is 0 here). The step lengths (wall to wall) are also compared.

The limit: decision 0221 sets ±0.1 mm "on every measurement", and the cone's E-26 applied it to diameters. A radius limit is not the same number:

| Option | On a diameter (micrometer) | On a radius (probe) | Positions and lengths | Change to decision 0221 |
| --- | --- | --- | --- | --- |
| **L1 (recommended)** | ±0.1 mm | ±0.05 mm | ±0.1 mm | None to the number. The reference becomes the simulated part |
| L2 | ±0.2 mm | ±0.1 mm | ±0.1 mm | Looser: the rc3 record's wording "radius within ±0.1 mm" taken literally |
| L3 | Pass ±0.1 mm; between 0.1 and 0.2 mm is "partial, finding recorded" | Pass ±0.05; partial to ±0.1 | ±0.1 mm | Adds a partial band |

**Recommendation: L1.** It keeps the owner's number as it was applied to the cone. The stepped part removes the X sensitivity that made it untestable. A micrometer (about 0.002 to 0.01 mm) and three probe touches (0.005 to 0.02 mm each, rc3 record part 2) are well inside it. The rc3 radial undersize (0.05 to 0.15 mm, not separable there) may well fail L1. If it does, the three radii say whether it is constant (tool length or axis height, about ±0.02 mm from the tool length measurement alone, rc3 record) or grows with radius, which is what the gate is for. Known contributions are listed beside the results, as the rc3 record proposed: Z tilt after re-setting G54 (about 0.004 mm over this length), tool length measurement (about ±0.02 mm), yaw (second order on a round section).

### Q3. Measuring in the machine with the probe

| Option | What | Notes |
| --- | --- | --- |
| **M1 (recommended): probe first, then micrometer** | Before the part leaves the machine: top touches with the Makera 3D Probe at A0 and A180 at three stations per step. The radius is the mean of the two tops in G54 Z, with G54 Z0 on the axis as set by the probe in the C2P procedure. X touches (`G38.2 X-…`) on the two tailstock-facing walls and on the stock's tail end face. Then, with the stock out of the chuck and the part still on it, a 0–25 mm micrometer across each step at A0 and A90 | The probe reads radius about the machine's own axis, so no axis or runout question enters. The A0 and A180 mean cancels an offset of the part. Same-direction X touches cancel the probe tip's diameter: each wall's distance from the tail end face is a difference of two touches. The micrometer reads diameter with no axis at all and cross-checks the probe. Neither needs the part cut off the stock: a micrometer frame reaches each step from the side, so the tab is not cut through (the rc3 complaint) |
| M2: probe only | As M1 without the micrometer | Quicker; no independent check of the axis Z the radii are measured from |
| M3: micrometer and vernier only | Diameters by micrometer, step lengths by vernier depth rod from the tail end face | No probe procedure to write; positions to about ±0.05 mm, from the end face rather than the machine's X |

Stations for part A (program X, inside the clean lengths): Ø24 at 74.5, 76.5, 78.5; Ø17 at 83, 85, 87; Ø10 at 91, 93, 95. Wall touches (probe moving −X): the wall at X 80 at radius about 10.5 (between 8.5 + 1.59 and 12), the wall at X 88 at about 7.2 (between 5 + 1.59 and 8.5), and the tail end face (program X 100) at a radius clear of the tailstock centre. The wall at X 73 faces the chuck. It sits in a groove about 4.5 mm wide (the held region's uncut face is at about X 68.5) and is not touched by default.

The probe commands would follow the rc3 C2P conventions: `G38.2` only, F 20 to 60, relative distances of at most 5 mm, `M119` before each touch. They come from the firmware source and are unverified on a Z1 until the owner runs them, as the C2P notes say.

**Recommendation: M1.** It is what the rc3 record proposed, plus the independent micrometer check, and it adds about 20 minutes of measuring.

### Q4. Setting X0 for the repeat first cut (OI-2152)

| Option | Method | Notes |
| --- | --- | --- |
| X1: OI-2152 as written | Probe the tail end face (off the tailstock centre); X0 = that machine X − tip radius − **measured** stock length; put the measured length in the project | Each stock blank changes the project, so the program must be regenerated, and air-cut again before it is cut (gate 5 F1, rc3 E-17) |
| **X2 (recommended)** | The same touch; X0 = machine X − tip radius − the **project's** stock length (100 mm) | The part keeps its place against the tailstock end, where it is. A short stock (rc3: 99.5 mm) loses length only from the 58 mm of uncut free stock at the chuck end, and the air-cut program is cut unchanged. The stock must not be longer than the project's (rc3 E-11) and no more than about 1 mm shorter. The real grip depth is measured and recorded (W1) |
| X3: as rc3 | Makera's routine from the headstock face plus an offset | The method whose result was 1.4 mm off (cause not recorded) |

The tip radius is the probe's own calibration value. If the probe will not fit between the end face and the tailstock centre with the tailstock up, touch the face before bringing the tailstock up. The wall touches after the cut (Q3) then show any shift the tailstock caused, as a position error separate from size, which is the point of the part.

**Recommendation: X2.** It satisfies OI-2152's aim, a probe-set X0 from the stock end, without a per-blank regeneration. If the owner chooses X2, OI-2152's method text is updated in the phase 2 pull request.

### Q5. The finishing strategy at the shoulders

| Option | Time on part A | Notes |
| --- | --- | --- |
| **F1 (recommended): helical with ball 204, as today** | 12:07 | The owner's tool choice for #586 (OI-921) and the air-cut-tested tool change kept. Decision 0297 turns once at each face, so the walls are cut by the side of the tool and stay at the stock to leave. Each wall's root keeps the ball's 1.59 mm radius, which Q1 and Q3 allow for |
| F2: helical with flat end 104 | 3:01 | Square roots and full-height flat walls; one tool and no tool change; finishing with a single-flute O-flute may leave a rougher surface on tooling board; gate 5 then no longer tests a ball finishing drop |
| F3: axial with ball 204 | 30:03 | Two and a half times as long, many more A indexes; no measurement advantage |

**Recommendation: F1.**

### Q6. The project folder, golden programs and gate 1 to 3 coverage

| Option | What | Notes |
| --- | --- | --- |
| **P1 (recommended): replace the cone in place** | `testdata/projects/first_cut/` and `testdata/golden/first_cut/` keep their names; the part becomes part A; the golden programs are re-blessed | Every document, test and the gate 3 chunking (decision 0244, which splits whatever program the file holds) already name `first_cut`. CI cost stays about the same: the first-cut golden tests are already among the longest (decision 0516, OI-2036), and part A has about 3 % fewer moves than the cone (15,397 against 15,929). The cone stays in history: v0.55.0, the rc3 record and decision 0653 name its commit |
| P2: a new project beside the cone | `first_cut_steps` added; the cone kept | Doubles the first-cut test time and the gate 3 replay in CI; more code to generalise (`xtask` names `first_cut`) |
| P3: a new project; the cone removed | As P1 under a new name | The same CI cost as P1, but every reference to `first_cut` changes |

**Recommendation: P1.**

### Q7. Requirement wording

| Option | Wording |
| --- | --- |
| **R1 (recommended): no new requirement; the verification plan carries it** | Verification plan 4.1, row "First-cut part (gate 5)": "±0.1 mm on a diameter and on lengths and positions, ±0.05 mm on a radius (decision 0221; [this record]), each measured against the simulated part at the same X, with the simulation against the model reported separately. The part is a stepped cylinder whose diameters are measured on cylinders and whose faces square to the axis give positions along X. Software tolerance cannot beat what the machine holds." Section 5, gate 5 row, pass criterion: "Part matches the simulated part within the limit in 4.1; the simulation's deviation from the model and the findings recorded". The gate 5 row's "Check" column reads "(wax, foam or tooling board)" |
| R2: a new requirement (next free NFR number at merge) | "The project keeps a first-cut reference project for its reference machine: a part whose sizes are measured independently of position along the rotary axis (cylinders of stated diameters, separated by faces square to the axis that give positions along it), its generated programs held as golden files, and the simulated part's radius at each measurement station stated beside the model's, so that a first cut is compared with the simulated part." Priority P2 |

**Recommendation: R1.** Gate 5 is a verification method, not something the software does. Decision 0221 made the cone with no requirement for the same reason, and the phase 2 tests trace to existing IDs. `docs/requirements.md` is not edited either way until the owner agrees.

## Phase 2: what would be built after acceptance (OI-9902)

On the recommended answers; changed to suit the owner's choices. One code and test pull request, as a draft, from `main`:

1. **The part.** A stepped-cylinder test part in `openchuck-testkit` with an exact radius function and its own mesh, rings at each wall at the same X so the faces are square to the axis. The shared `radial_mesh` spaces rings evenly and would slope the walls. `first_cut_part` in `apps/openchuck-cli/tests/common/sample_projects.rs` returns it, and `sample_projects` writes `models/part.stl` for the owner to open and cut from stock. Tests first, from Q1: the radius at each step and wall, a closed mesh with no inside-out pieces, and walls exactly square to the axis.
2. **The project and golden programs** (P1). `testdata/projects/first_cut/` rebuilt with `OPENCHUCK_BLESS_PROJECTS=1`; `testdata/golden/first_cut/` re-blessed (both posts). The existing `first_cut` tests (gates 1 and 2 at 1, 2 and N threads; simulation without overcut; the setup sheet) keep their names. Gate 3 re-chunks the LinuxCNC program (decision 0244), and the chunk count is recorded.
3. **New tests from Q2 and Q3**, named by requirement ID. The simulated radius at each station of Q3 is within the scallop of the model's (SIM-03). The face turns fall at each wall (decision 0297's note: X 71.41, 81.59 and 89.59). No fed move below X 70.1 or above X 95.9 (SET-09). The setup sheet states the part (GC-12). Each test is timed uninstrumented; any over about a minute is listed in `.config/nextest.toml`, and the CI share is checked against `docs/ci-performance.md`.
4. **Documents in the same pull request:** the [gates 4 and 5 instructions](../z1-gates-4-5-test-instructions.md) section 5 rewritten for the part (G0 geometry table, W5, V5, M2, 5.7 measurements and stations, the probe sequence of Q3, X0 of Q4, the results form 8.2) and its sections 6.2 and 9; the [air-cut procedure](../z1-air-cut-procedure.md) first-cut paragraph; [reference-jobs.md](../reference-jobs.md) (first-cut section); [verification plan](../verification-plan.md) 4.1 and section 5 (Q7, R1); decision 0221 (a pointer to this record); `testdata/README.md` (`projects/*/` and `golden/first_cut/*` rows); the [safe-practice checklist](../z1-safe-practice-checklist.md) section 3 if the limit wording changes (Q2); the user guide's *Known limitations* (`user-docs/src/start/limitations.md`) if it names the part (today it says "one small first cut in tooling board", which stays true). OI-2152's method if Q4 is X2. OI-9902 closes. The rest of OI-2154 (travel, safe Z, firmware wording in sections 3 and 4) stays with its docs lane unless the coordinator assigns it here.

Searched for documents that mention the cone of the first cut: the gates 4 and 5 instructions (section 5.2 G0, W5, M2, 5.7 I2), the verification plan 4.1, `testdata/README.md`, `reference-jobs.md`, `sample_projects.rs` and decision records 0221, 0244, 0297 and 0653. The records stay as written (history); the others are in item 4. The user guide's *Air cut*, *Checklist* and *Checking G-code* pages and the tutorial say "the first cut" in the general sense and name no part: no change needed there. The 0.55.0 release notes describe the released cone result and stay as written.

## Alternatives considered

The options are in Q1 to Q7. Also considered and not proposed:
- **Keep the cone and fix only X0 and the criterion.** The record shows a 0.25 mm position error is 0.1 mm of diameter on this cone, and the owner asked for the steps (#826).
- **Measure the cone at the simulated radius.** It removes the bias from the unfinished tail but not the position sensitivity.
- **A separate wall-finishing operation (flat end on the faces only).** It sharpens the roots at the cost of a third operation and a tool change; F2 does this more simply if square roots are wanted.

## Consequences

On acceptance: decision 0221 gains a pointer here and its item 2 part is replaced; the verification plan and the gates 4 and 5 instructions change as in phase 2 item 4. The `first_cut` golden programs change when re-blessed (P1). The program cut in gate 5 is then a new program, so it goes through gates 1 to 4 again (air cut included) before it is cut. No requirement changes under R1. No architecture change. No schema change. No new dependency.

## Verification (of this proposal)

- **What was checked.** The figures under *What this lane measured* come from `openchuck-cli` built with `cargo build --release -p openchuck-cli` at `main` `8f61706` on Linux x86_64 (4 vCPU). Commands, per scratch project: `new`, `import`, `machine makera-z1`, `setup` with the `build_first_cut` arguments, `tool --add` (104 and 204 from `crates/openchuck-project/defaults/z1`), the committed first-cut operation files; then `regenerate --post makera-z1 --tool-numbers keep --grouping operation --acknowledge-collision-warnings` and `simulate`. The tool-length warning acknowledged there is the one the committed project also raises (a 30 mm stick-out assumed inside the 45 mm clearance radius). The scratch cone reproduces the rc3 E-10 simulation figures (0.889 mm, 0.049 mm).
- **Finer cell.** Part A at 0.05 mm (`simulate --cell 0.05`, 104 s): largest left 0.500 mm at the tail, deepest overcut 0.023 mm (0.045 mm at 0.1 mm). 0.025 mm is refused for this stock size (surface-sample limit).
- **Not verified.** No scratch program was air-cut or cut. The probe commands of Q3 and Q4 are from the firmware source and the rc3 C2P notes, unverified on a Z1. The clean lengths and wall bands are calculated from the ball radius and the fed-move limits, not measured. Times are program estimates. Nothing here says any program is safe or correct for a machine (NFR-12).
- **Records checks:** `cargo xtask records` and `cargo xtask trace` run before the push (results in the pull request).
