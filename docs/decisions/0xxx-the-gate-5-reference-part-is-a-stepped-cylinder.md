# 0xxx: The gate 5 reference part is a stepped cylinder finished with a flat end, compared with the simulated part (issue #826; lane STEPPED-CYL)

- **Status:** proposed (23 Nov 2026) by lane STEPPED-CYL, phase 1 (documents only), updated the same day with **the owner's answers to Q1 to Q7** (23 Nov 2026, relayed by the coordinator, decision 0049): Q1 A, Q2 L1, Q3 M1, Q4 X2, **Q5 F2** (not the lane's recommendation: finishing with the flat end 104), Q6 P1, Q7 R1. The record is written to those answers and awaits the owner's acceptance (OI-9901). F2 raises two facts for the owner to confirm (OI-9903: which 3.175 mm flat end is fitted, and the 104 preset for finishing). Nothing is built until the record is accepted. The number is provisional: the coordinator assigns the final record and `OI-` numbers when the Merge Manager releases the pull request.
- **What it would amend (on acceptance):** [decision 0221](0221-gate-5-first-cut-project-and-limit.md) item 2 (the part: a cone becomes stepped cylinders) and the comparison behind its ±0.1 mm limit (the model becomes the simulated part); [verification plan](../verification-plan.md) section 4.1, row "First-cut part (gate 5)", and section 5, gate 5 row; the method of OI-2152 for setting X0 (Q4); the first-cut project's finishing tool chosen in OI-921 (204 becomes 104, Q5). Decision 0221's ±0.1 mm limit is **not** changed by this proposal unless the owner chooses so in Q2.
- **Date:** 23 Nov 2026.
- **Requirements:** none covers a gate 5 reference part (decision 0221 said the same of the cone), and the owner chose none (Q7, R1). The phase 2 tests trace to the IDs the cone's tests already use: GC-07, NFR-06 and NFR-08 (golden programs), GC-10 (gate 3), SIM-03 and SIM-05 (simulation), SET-09 and OP-21 (held regions and tabs), GC-12 (setup sheet). NFR-12 throughout: generated G-code is the user's to verify before it is run, and nothing here says a program is safe or verified.
- **Builds on:** [decision 0221](0221-gate-5-first-cut-project-and-limit.md) (accepted; the first-cut project, tooling board, the ±0.1 mm limit); [0297](0297-helix-strategies-turn-at-faces-across-the-axis.md) (accepted; a helix turns once at each face across the axis); [0161](0161-tools-keep-clear-of-the-gripped-length.md) and [0160](0160-held-region-is-the-stock-section.md) (held regions); [0244](0244-bounded-first-cut-linuxcnc-replay.md) (bounded gate 3 replay of the first-cut program); [0649](0649-the-z1-profile-takes-the-firmware-1-1-3-measurements.md) and [0651](0651-the-z1-rotary-reference-y-is-the-firmware-g54-y.md) (the Z1 reference setup); [0653](0653-verification-report-for-release-0-55-0.md) (the 0.55.0 verification report and the waiver of the partial rc3 first cut, OI-2151).
- **Open items:** adds OI-9901 (acceptance of this record with the owner's answers), OI-9902 (phase 2: the build) and OI-9903 (the finishing tool's identity and preset under F2). Depends on OI-2152 (the repeat first cut; Q4 refines its X0 method and leaves the decision to the owner) and OI-2154 (the gates 4 and 5 instructions are stale; phase 2 rewrites their section 5, and the rest of OI-2154 stays with its docs lane unless the owner asks this lane to take it). Neither is closed here.

Numbering: provisional ("0xxx"; OI-9901 to OI-9903 are numeric placeholders, because `cargo xtask records` refuses an `OI-` number that is not a number), on the owner's rule of 23 Nov 2026 (reference numbers are monotonic in merge order; no reserved blocks). No project schema number: the phase 2 project is written at the current schema and nothing stored changes format. No spike.

## Context

The rc3 first cut (issue #586, [rc3 test record part 3](../archive/0653-release-0-55-verification/rc3-test-record-part-3.md), owner's measurements of 22 Nov 2026) cut the cone of decision 0221: 20 mm diameter at X 73, falling to 10 mm at X 98, between the 10 mm tab (X 68 to 73) and the 2 mm tailstock piece (X 98 to 100), on 35 x 35 x 100 mm tooling board. The cut ran with no unexpected event and matched the simulation, but the diameters could not test the ±0.1 mm limit:

- A cone loses 0.4 mm of diameter per mm of X. The shoulder was measured at X 74.43 against 73.0: the part sat about 1.4 mm toward the tailstock, which reads as about 0.56 mm of diameter. The five E-26 differences were +0.14 to +0.49 mm.
- The residuals about the best-fit line were about ±0.12 mm, more than the limit under test. A generic digital vernier (0.01 mm) with flat jaws also reads high on a taper.
- With the shift allowed for, the radii were under by about 0.05 to 0.15 mm (tab 9.8 mm against 10). This could not be separated from the position error.
- E-26 compared with the model, but the simulation expects material left: up to 0.889 mm at the unfinished tail (fed moves stop at X 95.9 beside the held end, decision 0161).
- Removing the cone from the stock to measure it removed part of the tab.

The record's own proposals: compare the measured radius with the simulated radius at the same X and report simulation against model separately; set X0 by probing the stock's end face; measure in the machine with the probe (top touches at A0 and A180); use a stepped-cylinder part. Issue #826 asks for that part.

**What a stepped cylinder changes.** On a cylinder the diameter does not depend on where along X the instrument sits, so a micrometer or vernier reads size alone. The faces between steps (square to the axis) give X positions directly. A work-zero error then shows as a position error, separate from a size error. Steps of several diameters also separate a radial error that is the same at every radius (tool length measurement, axis height) from one that grows with radius (a Z scale error).

## Decision (the owner's answers, 23 Nov 2026, relayed)

1. **The part (Q1, A).** Three steps between the tab and the tailstock piece: Ø24 mm at program X 73 to 80, Ø17 mm at X 80 to 88, Ø10 mm at X 88 to 98. Decision 0221's stock (35 x 35 x 100 mm tooling board), 68 mm held region at the start, 10 mm tab at X 68 to 73 and 2 mm Ø10 tailstock piece (held end) at X 98 to 100 are kept.
2. **The criterion (Q2, L1).** Each measurement is compared with **the simulated part at the same X**, and the simulation's deviation from the model is reported separately. Limit: ±0.1 mm on a diameter (micrometer), ±0.05 mm on a radius (probe), ±0.1 mm on wall positions and step lengths. Decision 0221's number is unchanged; what it is compared with changes.
3. **Measuring (Q3, M1).** Probe first, in the machine. Then a 0–25 mm micrometer across each step at A0 and A90, with the stock out of the chuck and the part still on it. The probe commands are unverified on a Z1 (see Q3).
4. **X0 (Q4, X2).** Probe the stock's tail end face; X0 = that machine X − the probe's tip radius − the **project's** stock length (100 mm). The program that was air-cut is cut unchanged. The stock must be no longer than 100 mm and at most about 1 mm short. If the probe will not fit beside the tailstock centre, touch the face before the tailstock comes up.
5. **Finishing (Q5, F2).** Helical finishing with the **flat end 104** (Z1 O-flute 3.175x25, single flute), the roughing tool: **one tool**, no tool change, square roots at each wall. Tool 204 (ball) leaves the project. This changes the owner's tool choice of OI-921 (finish with 204); see *What F2 changes*.
6. **The project (Q6, P1).** The cone is replaced in place: `testdata/projects/first_cut/` and `testdata/golden/first_cut/` keep their names.
7. **No new requirement (Q7, R1).** Verification plan 4.1 and the section 5 gate 5 row carry the part and the criterion (wording in Q7).

## What F2 changes

- **Tools.** The project holds one tool, 104, numbered T1. The program has one `T1 M6` (at the start) and restarts the spindle (`S10000 M3`) for the finishing operation; there is no mid-program tool change. The rc3 steps for the second tool (E-06's T2, E-22) go. In the scratch single-file program: `M5`, `T1 M6`, `S10000 M3` at lines 8 to 10, `S10000 M3` again at the finishing operation, and `M5` at the end.
- **Speeds and feeds (OI-9903).** Both operations take 104's "Plastic" preset from the Z1 tool library: 10000 rpm, 0.1 mm per tooth (1000 mm/min), plunge 300 mm/min, stepover 2.2225 mm (the finishing helix's pitch), stepdown 1.0 mm. The owner chose this preset for **roughing** (OI-921). For **finishing** it is the library's value, not yet confirmed by the owner: it is marked unconfirmed in the instructions until the owner confirms it or gives another.
- **Which tool is fitted (OI-9903).** The rc3 record says the roughing tool was "changed to 12mm variant" and does not say which. The deepest cut, from the box's corners (24.7 mm from the axis) to the tab and the Ø10 step (5 mm), is 19.7 mm. The 104 has 25 mm of flute and reaches it; the Z1 library's 3.175 mm flat end with 12 mm of flute (101, "O-flute metal 3.175x12") does not. Under F2 the flat end also cuts the finished faces, so the tool fitted must be the project's tool. The owner is asked to confirm that a 104 (3.175 mm, 25 mm flute) is to hand. If a different tool will be fitted, the project is changed to it before gates 1 to 4.
- **Measuring is easier.** No root radius: each wall is flat over its whole height. Each tailstock-facing wall gives a 3.5 mm band for a probe touch (centre at radius about 10.25 at X 80, 6.75 at X 88), with about ±1.7 mm of margin. The flat bottom reaches 1.59 mm beyond the fed-move limit, so the Ø10 step is finished to about X 97.5. Clean lengths for a micrometer: Ø24 7.0 mm, Ø17 8.0 mm, Ø10 about 9.5 mm.
- **The simulated part is the model.** A flat end square to a cylinder leaves no scallop; at a pitch of 2.22 mm (less than the 3.175 mm diameter) the flat bottom covers every X. So inside the clean lengths the simulated radius is the model's within the toolpath tolerance, and the comparison of Q2 is close to a comparison with the model.
- **What the gate no longer tests.** A ball finishing drop on a curved surface (the cone's case) is not cut. Surface finish from a single-flute O-flute at 1000 mm/min on tooling board is unknown until the cut; record it (rc3 E-29).
- **Time.** Finishing about 3:47 against 11:56 with the ball on the cone; the whole program about 1.5 % shorter than the cone's (table below).

## What this lane measured (scratch, not committed)

To ground the options, the lane built `openchuck-cli` in release mode at `main` `8f61706` and made scratch projects with the CLI. Each was made as `build_first_cut` in `apps/openchuck-cli/tests/e2e.rs` makes the first-cut project: the same stock, part offset, work X, held regions, tab, material and tools, and the committed project's two operations (rotary roughing with tool 104; helical finishing, 5 degree steps). Only the part (and, for F2, the finishing tool) changed: an exactly revolved STL written by a small script, with walls square to the axis. Programs come from `regenerate --post makera-z1`; material comes from `simulate` at the default 0.1 mm cell. None of this is committed; phase 2 builds the part with tests.

The scratch cone reproduces the committed project: its operation estimates (55:53 roughing, about 3353 s, as in the golden program and the rc3 air-cut sheet's 55:52) and its simulation (largest material left 0.889 mm at the tail, deepest overcut 0.049 mm; rc3 E-10). The times below are each operation's "about N s" estimate, the figure the setup sheet shows and the air cut measured against. They are estimates, not machine times. An earlier draft of this record quoted cut time only, which leaves out rapids and links.

| Part (program X 73 to 98) | Roughing | Finishing, helical, ball 204 | Total | Against the cone | Face turns (decision 0297) |
| --- | --- | --- | --- | --- | --- |
| Cone 20 to 10 mm (today's part) | 55:53 | 11:56 | 67:49 | — | 1 (the face beside the tab) |
| **A. Ø24 x 7, Ø17 x 8, Ø10 x 10** | 62:59 | 12:47 | 75:46 | +11.7 % | 3, at X 71.41, 81.59, 89.59 |
| B. Ø24 x 7, Ø18 x 8, Ø12 x 10 | 61:31 | 12:46 | 74:17 | +9.5 % | 3 |
| C. Ø24 x 12, Ø12 x 13 | 55:54 | 12:23 | 68:17 | +0.7 % | 2 |
| D. Ø24 x 6, Ø20 x 6, Ø15 x 6, Ø10 x 7 | 66:21 | 13:05 | 79:26 | +17.1 % | 4 |

Finishing of part A (Q5):

| Finishing of part A | Time | Whole program | What it leaves |
| --- | --- | --- | --- |
| **Helical, flat end 104 (F2, chosen)** | 3:47 | 66:46 (−1.5 % against the cone) | Square roots; the cylinders the model's within tolerance; one tool |
| Helical, ball 204 (F1) | 12:47 | 75:46 | Scallop about 0.016 mm on the cylinders (calculated, r − √(r² − (s/2)²), r 1.5875, s 0.4536); the ball's 1.59 mm radius at each wall's root |
| Axial, ball 204 (F3, default angle step) | 58:50 | 121:49 | Walls cut on every pass; 20,337 moves |

Simulation of part A at the 0.1 mm cell:

- **Finished with the flat end (F2):** largest material left 0.502 mm, at the very end of the Ø10 step (program X 98, against the held end). Deepest overcut 0.046 mm, on the Ø24 step, within one cell.
- **Finished with the ball (F1):** largest material left 0.502 mm, deepest overcut 0.045 mm. At a 0.05 mm cell this falls to 0.023 mm, so it follows the cell size: a resolution effect of the simulation, not a cut into the part.
- **Finer cells:** a 0.025 mm cell is refused on this stock (more than the 64,000,000 surface samples supported). Phase 2 reads the simulated radius at the stations at the finest cell the stock allows, and states it.

## The questions and the owner's answers

Each question keeps the options it was asked with. The owner's answer is marked.

### Q1. The part: number, diameters and lengths of the steps — **answer: A**

All options keep decision 0221's layout and the #586 choices. Steps get smaller toward the tailstock, as the cone did, so the tailstock body sits beside the smallest diameter. Fed moves stop at X 95.9 beside the held end (decision 0161), as on the cone. Clean lengths below are for the ball finish (F1), as asked; under F2 they are longer (*What F2 changes*).

| Option | Steps (program X) | Clean length per step (F1) | Wall heights (radial) | Notes |
| --- | --- | --- | --- | --- |
| **A (recommended; chosen)** | Ø24 X 73–80; Ø17 X 80–88; Ø10 X 88–98 | 7.0, 6.4, 6.3 mm | 7 (tab to Ø24, facing the chuck), 3.5, 3.5 mm | Radii 12, 8.5 and 5 mm, a 2.4 : 1 spread, to tell a constant radial error (tool length, axis height) from one that grows with radius (Z scale). The last step meets the tailstock piece flush, as the cone's small end did. All three fit a 0–25 mm micrometer. +11.7 % with the ball finish; −1.5 % with F2 |
| B | Ø24 X 73–80; Ø18 X 80–88; Ø12 X 88–98 | 7.0, 6.4, 6.3 mm | 7, 3, 3 mm | Radii 12, 9 and 6; about 1.4 mm of flat wall above a ball's root radius. +9.5 % |
| C | Ø24 X 73–85; Ø12 X 85–98 | 12.0, 9.3 mm | 7, 6 mm | Only two radii. +0.7 % |
| D | Ø24, Ø20, Ø15, Ø10 at X 73–79–85–91–98 | 6.0, 4.4, 4.4, 3.3 mm | 7, 2, 2.5, 2.5 mm | Flats shorter than a 6.35 mm micrometer anvil. +17.1 % |

### Q2. The criterion and the limit (decision 0221) — **answer: L1**

What is compared: the measured radius against the simulated radius at the same X (as the rc3 record proposed), with the simulation against the model reported separately. Positions: each wall's X against the simulated wall; step lengths wall to wall.

| Option | On a diameter (micrometer) | On a radius (probe) | Positions and lengths | Change to decision 0221 |
| --- | --- | --- | --- | --- |
| **L1 (recommended; chosen)** | ±0.1 mm | ±0.05 mm | ±0.1 mm | None to the number. The reference becomes the simulated part |
| L2 | ±0.2 mm | ±0.1 mm | ±0.1 mm | Looser: the rc3 record's wording "radius within ±0.1 mm" taken literally |
| L3 | Pass ±0.1 mm; 0.1 to 0.2 mm "partial, finding recorded" | Pass ±0.05; partial to ±0.1 | ±0.1 mm | Adds a partial band |

The rc3 radial undersize (0.05 to 0.15 mm, not separable there) may fail L1. If it does, the three radii say whether it is constant or grows with radius. Listed beside the results, as the rc3 record proposed: Z tilt after re-setting G54 (about 0.004 mm over this length), the tool length measurement (about ±0.02 mm), yaw (second order on a round section).

### Q3. Measuring in the machine with the probe — **answer: M1**

| Option | What | Notes |
| --- | --- | --- |
| **M1 (recommended; chosen): probe first, then micrometer** | Before the part leaves the machine: top touches with the Makera 3D Probe at A0 and A180, three stations per step. The radius is the mean of the two tops in G54 Z, with G54 Z0 on the axis as set by the probe (C2P). Then same-direction X touches (probe moving −X) on the two tailstock-facing walls and the stock's tail end face. Then, with the stock out of the chuck and the part still on it, a 0–25 mm micrometer across each step at A0 and A90 | The probe reads radius about the machine's own axis. The A0 and A180 mean cancels an offset of the part. Same-direction X touches cancel the tip's diameter: each wall's distance from the tail end face is a difference of two touches. The micrometer reads diameter with no axis at all and cross-checks the probe. The part is not cut off the stock, so the tab is not cut through |
| M2: probe only | As M1 without the micrometer | No independent check of the axis Z |
| M3: micrometer and vernier only | Diameters by micrometer, step lengths by vernier depth rod | Positions to about ±0.05 mm, from the end face |

Stations (program X, inside the clean lengths under F2): Ø24 at 74.5, 76.5, 78.5; Ø17 at 82, 84, 86; Ø10 at 90, 93, 96. Wall touches: the wall at X 80 at radius about 10.25, the wall at X 88 at about 6.75, the tail end face (program X 100) at a radius clear of the tailstock centre. The wall at X 73 faces the chuck. It sits in a groove about 4.5 mm wide (the held region's uncut face is at about X 68.5) and is not touched by default.

**The probe commands are unverified on a Z1.** They would follow the rc3 C2P conventions (`G38.2` only, F 20 to 60, relative distances of at most 5 mm, `M119` before each touch) and come from the firmware source, as the C2P notes say. Nothing here says they are safe; the owner runs them attended and stops at anything unexpected.

### Q4. Setting X0 for the repeat first cut (OI-2152) — **answer: X2**

| Option | Method | Notes |
| --- | --- | --- |
| X1: OI-2152 as written | Probe the tail end face; X0 = machine X − tip radius − **measured** stock length; the measured length into the project | Each blank changes the program, so it is regenerated and air-cut again |
| **X2 (recommended; chosen)** | The same touch; X0 = machine X − tip radius − the **project's** stock length (100 mm) | The part keeps its place against the tailstock end. A short stock (rc3: 99.5 mm) loses length only from the uncut free stock at the chuck end. The air-cut program is cut unchanged. The stock is no longer than 100 mm and at most about 1 mm short; the real grip depth is measured and recorded (W1) |
| X3: as rc3 | Makera's routine from the headstock face plus an offset | The method whose result was 1.4 mm off (cause not recorded) |

The tip radius is the probe's own calibration value. If the probe will not fit between the end face and the tailstock centre with the tailstock up, touch the face before bringing the tailstock up. The wall touches after the cut then show any shift the tailstock caused, as a position error separate from size. OI-2152's method text is updated to X2 in the phase 2 pull request; OI-2152 itself stays open until the repeat cut is recorded.

### Q5. The finishing strategy at the shoulders — **answer: F2** (the lane recommended F1)

| Option | Finishing time on part A | Notes |
| --- | --- | --- |
| F1 (the lane's recommendation): helical with ball 204, as today | 12:47 | Kept the owner's tool choice (OI-921) and the tool change. Decision 0297 turns once at each face, so the walls are cut by the side of the tool. The ball's 1.59 mm radius stays at each wall's root |
| **F2 (chosen): helical with flat end 104** | 3:47 | Square roots and full-height flat walls; one tool and no tool change. See *What F2 changes* |
| F3: axial with ball 204 | 58:50 | Much longer; no measurement advantage |

### Q6. The project folder, golden programs and gate 1 to 3 coverage — **answer: P1**

| Option | What | Notes |
| --- | --- | --- |
| **P1 (recommended; chosen): replace the cone in place** | `testdata/projects/first_cut/` and `testdata/golden/first_cut/` keep their names; the part becomes part A, finished per F2; the golden programs are re-blessed | Every document, test and the gate 3 chunking (decision 0244, which splits whatever program the file holds) already name `first_cut`. CI cost stays about the same or falls: the first-cut golden tests are already among the longest (decision 0516, OI-2036), and the F2 program has 12,133 moves against the cone's 15,928 (single-file programs). The cone stays in history: v0.55.0, the rc3 record and decision 0653 name its commit |
| P2: a new project beside the cone | `first_cut_steps` added; the cone kept | Doubles the first-cut tests and the gate 3 replay |
| P3: a new project; the cone removed | As P1 under a new name | Every reference to `first_cut` changes |

### Q7. Requirement wording — **answer: R1**

| Option | Wording |
| --- | --- |
| **R1 (recommended; chosen): no new requirement; the verification plan carries it** | Verification plan 4.1, row "First-cut part (gate 5)": "±0.1 mm on a diameter and on lengths and positions, ±0.05 mm on a radius (decision 0221; decision 0xxx), each measured against the simulated part at the same X, with the simulation against the model reported separately. The part is a stepped cylinder whose diameters are measured on cylinders and whose faces square to the axis give positions along X. Software tolerance cannot beat what the machine holds." Section 5, gate 5 row, pass criterion: "Part matches the simulated part within the limit in 4.1; the simulation's deviation from the model and the findings recorded"; its "Check" column reads "(wax, foam or tooling board)" |
| R2: a new requirement | A P2 NFR for a first-cut reference project (wording in the earlier draft of this record) |

Under R1 `docs/requirements.md` is not edited. The phase 2 tests trace to existing IDs.

## Phase 2: what is built after acceptance (OI-9902)

One code and test pull request, as a draft, from `main`:

1. **The part.** A stepped-cylinder test part in `openchuck-testkit` with an exact radius function and its own mesh, with two rings at each wall at the same X so the faces are square to the axis. The shared `radial_mesh` spaces rings evenly and would slope the walls. `first_cut_part` in `apps/openchuck-cli/tests/common/sample_projects.rs` returns it, and `sample_projects` writes `models/part.stl` for the owner to open and cut from stock. Tests first: the radius at each step, a closed mesh with no inside-out pieces, and walls exactly square to the axis.
2. **The project and golden programs** (P1, F2). `build_first_cut` adds only tool 104 and makes both operations with it (finishing: helical, 5 degree steps), numbered T1. `testdata/projects/first_cut/` is rebuilt with `OPENCHUCK_BLESS_PROJECTS=1` and `testdata/golden/first_cut/` re-blessed (both posts). The existing `first_cut` tests (gates 1 and 2 at 1, 2 and N threads; simulation without overcut; the setup sheet) keep their names. The setup-sheet test changes to one tool. Gate 3 re-chunks the LinuxCNC program (decision 0244), and the chunk count is recorded.
3. **New tests**, named by requirement ID. The simulated radius at each station of Q3 is the model's within the tolerance (SIM-03). The face turns fall at each wall (X 71.41, 81.59, 89.59, decision 0297). No fed move below X 70.1 or above X 95.9 (SET-09). The program has one tool and one `M6` (GC-09). The setup sheet states the part and the one tool (GC-12). Each test is timed uninstrumented; any over about a minute is listed in `.config/nextest.toml`, and the CI share is checked against `docs/ci-performance.md`.
4. **Documents in the same pull request:**
   - The [gates 4 and 5 instructions](../z1-gates-4-5-test-instructions.md): section 5 rewritten for the part. That covers G0 geometry, W3 and W5, the 5.3 tools (one tool, 104, its preset for finishing marked unconfirmed until OI-9903 closes; T0, T1, T5 stick-out against the 19.7 mm depth), V3 and V5, M2 (the tailstock check at the Ø10 end), M4 (X0 by X2), 5.6 (no tool change), 5.7 (the stations and the probe and micrometer measurements of Q3, against the simulated part), 5.8 (L1), the form 8.2, the checklist 6.2 and section 9.
   - The [air-cut procedure](../z1-air-cut-procedure.md): its first-cut paragraph and the tool change line.
   - [reference-jobs.md](../reference-jobs.md): the first-cut section.
   - The [verification plan](../verification-plan.md): 4.1 and section 5 (R1).
   - [Decision 0221](0221-gate-5-first-cut-project-and-limit.md): a pointer to this record.
   - `testdata/README.md`: the `projects/*/` and `golden/first_cut/*` rows.
   - The [safe-practice checklist](../z1-safe-practice-checklist.md) section 3: the limit line names the simulated part.
   - OI-2152's method text (X2). OI-9902 and, if answered, OI-9903 close.

   The rest of OI-2154 (travel, safe Z, firmware wording in sections 3 and 4) stays with its docs lane unless the coordinator assigns it here.

**Documents that mention the cone of the first cut.** Searched: the gates 4 and 5 instructions (5.2 G0, W5, M2, 5.7 I2), the verification plan 4.1, `testdata/README.md`, `reference-jobs.md`, `sample_projects.rs` and decision records 0221, 0244, 0297 and 0653. The records stay as written (history); the others are in item 4.

**No change needed:**
- The user guide's *Air cut*, *Checklist* and *Checking G-code* pages and the tutorial say "the first cut" in the general sense and name no part.
- *Known limitations* says "one small first cut in tooling board", which stays true.
- The 0.55.0 release notes describe the released cone result.

## Alternatives considered

The options are in Q1 to Q7. Also considered and not proposed:
- **Keep the cone and fix only X0 and the criterion.** The record shows a 0.25 mm position error is 0.1 mm of diameter on this cone, and the owner asked for the steps (#826).
- **Measure the cone at the simulated radius.** It removes the bias from the unfinished tail but not the position sensitivity.
- **A separate wall-finishing operation after a ball finish.** Superseded by the owner's F2.

## Consequences

On acceptance:
- Decision 0221 gains a pointer here. Its item 2 part is replaced, and its tool line ("only the Z1 library's 3.175 mm single-flute or ball-nose tools") is still met by 104.
- OI-921's finishing choice (204) is replaced by 104 for the first-cut project.
- The verification plan and the gates 4 and 5 instructions change as in phase 2 item 4.
- The `first_cut` golden programs change when re-blessed (P1). The program cut in gate 5 is then a new program, so it goes through gates 1 to 4 again (air cut included) before it is cut, with the tool that will be fitted (OI-9903).
- No requirement change (R1), no architecture change, no schema change, no new dependency.

## Verification (of this proposal)

- **What was checked.** `openchuck-cli` was built with `cargo build --release -p openchuck-cli` at `main` `8f61706` on Linux x86_64 (4 vCPU).
  - Each scratch project was made with `new`, `import`, `machine makera-z1`, `setup` with the `build_first_cut` arguments, and `tool --add` (104, and 204 where used, from `crates/openchuck-project/defaults/z1`), plus the committed first-cut operation files. For F2 the finishing operation's tool was set to 104 and 204 was removed.
  - Then `regenerate --post makera-z1 --tool-numbers keep --acknowledge-collision-warnings`, with `--grouping operation` for the per-operation times, and `simulate`.
  - The tool-length warning acknowledged there is the one the committed project also raises: a 30 mm stick-out assumed inside the 45 mm clearance radius.
  - The scratch cone reproduces the committed program's roughing estimate (about 3353 s) and the rc3 E-10 simulation figures (0.889 mm, 0.049 mm).
- **Finer cell.** Part A with the ball finish at 0.05 mm (`simulate --cell 0.05`, 104 s): largest left 0.500 mm at the tail, deepest overcut 0.023 mm (0.045 mm at 0.1 mm). 0.025 mm is refused for this stock size (surface-sample limit).
- **Not verified.**
  - No scratch program was air-cut or cut.
  - The probe commands of Q3 and Q4 come from the firmware source and the rc3 C2P notes, unverified on a Z1.
  - The clean lengths and wall bands are calculated from the tool radius and the fed-move limits, not measured.
  - The surface finish of a single-flute flat end on tooling board is not known.
  - Times are program estimates.
  - Nothing here says any program is safe or correct for a machine (NFR-12).
- **Records checks:** `cargo xtask records` and `cargo xtask trace` were run before each push.
