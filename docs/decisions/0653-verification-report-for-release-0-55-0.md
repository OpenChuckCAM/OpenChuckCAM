# 0653: Verification report for release 0.55.0 (rc3; gates 4 and 5 on the Makera Z1; GC-09, GC-17, OP-13, NFR-12; issue #586; lane VERIFY-REPORT)

- **Status:** Accepted by the project owner on 22 Nov 2026, relayed by the coordinator (decision 0049). Proposed by lane VERIFY-REPORT the same day. It is the release verification report of [verification plan](../verification-plan.md) section 9 (Records) and work breakdown slice 20 item 13, drafted from the 0.55.0-rc3 test record. It changes no requirement, no architecture and no program behaviour. The number is provisional: the Merge Manager assigns the final number at release.
- **Date:** 22 Nov 2026.
- **Commit covered:** `aa73540022c3994e2cf27e350cff6fb47713672b` (tag `v0.55.0-rc3`, OpenChuckCAM 0.55.0), built by release dry run [43698165113](https://github.com/OpenChuckCAM/OpenChuckCAM/actions/runs/43698165113) on 15 Nov 2026.
- **Open items:** adds OI-2151 (the gate 5 result on rc3 is partial), closed by the recorded waiver below, and OI-2152 (the repeat first cut with X0 probed). Names OI-992, OI-2144, OI-2107, OI-304, OI-23 and OI-2149 as still open, OI-471 and OI-2143 as closed by pull request #828 (not merged on 22 Nov 2026), and OI-105 as decided and closing in the REL-NOTES-SCREENS pull request. See *Open items* below.
- **Raw evidence:** [archive/0653-release-0-55-verification](../archive/0653-release-0-55-verification/README.md), the three parts of the rc3 test record (issue #586, comments 7072055765, 7072070836 and 7072079025).

Generated G-code is the user's to verify before it is run on a machine (NFR-12). This report states what was observed on one Makera Z1 with one build. It does not verify any program as safe, for that machine or any other.

## Decision

Release 0.55.0 is reported against the first-release exit criteria of verification plan section 9 as set out under *Verification*, on the evidence of the rc3 test record. The report names one commit (above), lists the waivers and deviations, and states what was not verified. The gate 5 result is partial; it is waived for 0.55.0 (*Waiver: gate 5 for 0.55.0* below, OI-2151). Several exit criteria are outside the rc3 test record and are marked as not assessed here; this record does not decide them.

## Verification

### What was tested, and where

| Item | Record |
| --- | --- |
| Build | OpenChuckCAM 0.55.0, commit `aa73540022c3994e2cf27e350cff6fb47713672b` (`main` after #815), built 2026-11-15T10:54:06Z by dry run 43698165113 (green, 1 h 44 min; the release gate and the strict user guide check passed). Windows archive installed from the zip. Unsigned archive hashes are in part 1 of the record |
| Comparison builds | rc2, commit `bc1df28b`; the A-limit merge, `90e78107` |
| Machine | Makera Z1, controller firmware 1.1.3, ESP32 1.1.3; programs sent over WiFi |
| Controller limits read | `M203.1` printed `X: 20 Y: 20 Z: 10 A: 30 B: 1.66667` (A 30 deg/s = 1800 deg/min) |
| Work offset | Set in 4-axis mode. G54 found at X -118.39, Y -124.39, Z -39.6435 on 21 Nov 2026 (C2P-00). For gate 5, G54 Y and Z were set from the probed axis at the cone's middle: Y -124.136, Z -39.710 (C2P-06) |
| Conditions, Parts B and C | No tool and no material, feed override 100 %, enclosure closed, stop button in reach, timed runs from phone video with sound |
| Dates | Part A and B: 16 Nov 2026. Part C: 20 Nov 2026 (the C-01 file is dated 2026-11-20). Part C2: 20 and 21 Nov 2026. Part D: the first cut, after C2 on 21 Nov; analysis and sign-off 22 Nov 2026. The posted record does not give the date of the cut |
| Operator | PH; signed off 22 Nov 2026 |
| Reference programs | Generated with `cargo run -p openchuck-cli --example sample_projects` from the rc3 source and posted with `openchuck-cli regenerate --post makera-z1 --tool-numbers keep --setup-sheet`; the A-16 to A-18 software runs used the rc3 Linux archive's command-line tool |

### Instruments

| Instrument | Used for | Resolution and accuracy, as recorded |
| --- | --- | --- |
| LinuxCNC simulator (sim configuration, 4-axis XYZA), in CI | Gate 3 replay of the changed programs (A-10) | Not applicable |
| `fdiff.py` (decision 0647 archive) and `zdiff.py` (decision 0649 archive) | Diff of rc3 programs against rc2 after removing F words, safe-Z words and estimates (A-04) | Exact text comparison |
| Phone video with sound | Run times, plunge to retraction (B-01 to B-04, C-02) | Seconds |
| Controller console (`M203.1`, `M114`, `M114.4`, `$#`, `[PRB:...]`) | Limits, work offset, probe positions | 0.001 mm (probe) and 0.0001 mm (offsets) as printed |
| Dial indicator on a magnetic base, with a 10 mm linear rod as reference bar | C2-01 to C2-07 | Resolution not recorded; the procedure asks for 0.01 mm or better |
| Makera 3D Probe (`G38.2`, F 20 to 60) | C2P axis position and tilt, G54 Y and Z | Reads 0.001 mm; expected 0.005 to 0.02 mm per touch (not measured) |
| Micrometer (type not recorded) | Rod radius at the two probe stations: R1 5.015, R2 5.005 mm | Not recorded |
| Digital vernier, generic | E-26 and E-27 part measurements | 0.01 mm resolution; accuracy typically two to three times that, and its flat jaws read high on a taper |

### Results by gate

**Gates 1 to 3 (software).** Every check in Part A passed.

- A-02: the pull request checks and the dry run were green on `aa73540`.
- A-03, A-04: ten reference programs were generated and compared with rc2. Each differs from rc2 only in F words, the safe-Z words (Z40.000 to Z45.000 on rapids and retracts) and the estimates; cutting coordinates and line counts are the same.
- A-05, A-18: the F words of the A-turning blocks are lower than rc2 in sphere, cone (F709.4 to F464.1), cone_held_tabs (709.0 to 464.1), groove (681.8 to 501.3) and lobed (522.4 to 459.7 and 813.2 to 456.5); X words in the first 20 lines are identical to rc2; cone_axial, cone_axial_lead, cone_indexed, drilling, face_flow and swarf differ in nothing else there.
- A-07: header and footer as rc2; no `G93` in any program.
- A-08, A-11, A-12: the Z1 profile reads A and rapid A 1800 deg/min; travel X -207 to -1, Y -160 to -1, Z -102 to -1; safe cylinder, safe Z, minimum clearance radius and recommended minimum safe Z 45; spindle 4000 to 13,000 rpm; spindle-to-axis 72.0; rotary reference Y -123.39, X -198.39; maximum tool number 1000; no unconfirmed values.
- A-09, A-13: a project holding the rc2-era Z1 copy warns that it differs from the built-in profile; nothing was changed silently. The A-09 message names axis height, travel, maximum feeds, clearance, spindle speed range, spindle-to-axis distance, recommended minimum safe Z, minimum clearance radius and which values are confirmed. A-13 (no other warning about travel or clearance) was recorded as expected; the record does not say whether A-09 and A-13 used the same project.
- A-10: LinuxCNC replay of the changed programs: every move reached, no fault.
- A-14, A-15: the sphere setup sheet shows safe Z 45 and the expected work offset; a new Z1 project with the tutorial tool warns "may be too long to clear the workholding".
- A-16: a new Facing (end) project posts with the Z1 post (84 lines, "replay matches the toolpath (tip 0.0007 mm, A 0.0000 deg)", footer `G0 Z45.000`, `M5`, `G53 G0 Z-3`, `G28`, `M2`); no home-position refusal.
- A-17: all 12 reference projects post with the Z1 post with no travel refusal. face_flow posts only with `--acknowledge-collision-warnings` (its 3 mm ball has an 8 mm flute; stock stands up to 12.017 mm above the tip near X 78.2, A 195.3) and warns of an unreached 187.5 mm² on the Floor; cone_held_tabs and first_cut give the non-blocking held-region warning.

**Gate 4 (air cut on the Z1, no tool, no material).**

Carried over from rc2 (11 to 14 Nov 2026, firmware 1.1.3), on the A-04 diff: the SP-12 conventions (A direction, `G92.4` unwind, offsets), positions, collisions, end states and A direction of all ten reference programs; cone_axial, cone_indexed, cone_axial_lead, drilling and swarf timing (ratios 0.97 to 1.10); the tool change `M5`, `T<n> M6` (OI-41, closed 14 Nov); A-only moves with F above 1200 not refused or clamped (OI-267, closed 14 Nov); the block rate, 7 ms on 1.1.3 on split files (decision 0648).

Re-run on rc3, setup-sheet estimate against measured time, plunge to retraction:

| Step | Program | Estimate | Measured | Against estimate | Against rc2 measured |
| --- | --- | --- | --- | --- | --- |
| B-01 | sphere | 2:13 (133 s) | 2:07 (127 s) | -4.5 % | 127 s, 0 % |
| B-02 | groove | 3:08 (188 s) | 3:05 (185 s) | -1.6 % | 185 s, 0 % |
| B-03 | cone_held_tabs | 4:32 (272 s) | 4:26 (266 s) | -2.2 % | 269 s, -1.1 % |
| B-04 | cone_axial (control) | 7:39 (459 s) | 7:47 (467 s) | +1.7 % | 434 s, +7.6 % |
| C-02 | first_cut, op 1 | 55:52 (3352 s) | 58:23 (3503 s) | +4.5 % | Never run on rc2 |
| C-02 | first_cut, op 2 | 12:00 (720 s) | 11:39 (699 s) | -2.9 % | Never run on rc2 |

The percentages are calculated from the recorded times. All are within the 10 % band. B-04's estimate moved from 7:01 on rc2 to 7:39 on rc3 (A-06 attributes it to the higher safe Z; not checked further), and its +7.6 % against rc2 was accepted without investigation on 20 Nov 2026 ([issue #586 comment 7050899047](https://github.com/OpenChuckCAM/OpenChuckCAM/issues/586#issuecomment-7050899047)). B-05 (optional) was not run.

- B-06: during the sphere helical pass the controller reported a lower feed than the written F, as on rc2; on pure XYZ moves (cone_axial) the reported feed matches. Recorded as controller behaviour.
- B-07: no stutter, pause or alarm in any run.
- B-08: the safe-Z position at Z45 was recorded as "as expected"; the tool fitted and the machine Z read are not recorded.
- C-01: the first-cut program's tool-change lines are `M5`, `T1 M6` (lines 8 and 9), `M5`, `T2 M6` (11846, 11847) and a final `M5` (15963).
- C-02, C-03: motion as simulated, A did not turn at the unwind, no alarm or refused line; the far end of the part, the tab and tail rings moved smoothly at 100 % (the OI-993 motion was not seen).
- C-04 to C-07 (OI-993): the baluster Rough in Rings program posted with `combine_rotary_runs` on and off gives the same coordinates, the "on" file one block per pure-A run. Air cuts, not timed: on, smooth rotation; off, "significant jerkiness on strings of pure A rotation commands". On is the better setting for the machine; it is the built-in posts' setting. OI-993 closed on 20 Nov 2026 (#819).
- G93 on firmware 1.1.3: ignored, as on 1.1.2 (OP-13; [status report of 20 Nov 2026](https://github.com/OpenChuckCAM/OpenChuckCAM/issues/586#issuecomment-7049066645)).

**A-axis alignment (Part C2, before gate 5): partial.** Proposed limits, not Makera specifications.

| Check | Limit (proposed) | Result |
| --- | --- | --- |
| C2-01, C2-02 rod runout near and far | 0.03 and 0.05 mm TIR | 4-jaw 0.32 and 0.36 mm (one jaw does not seat when fully closed); 3-jaw 0.14 and 0.23 mm. Not met. The spindle bearing reads 0.01 mm TIR; the A-axis nose thread reads 0.24 mm, so a chuck screwed onto it sits about 0.12 mm off-centre |
| C2-03 vertical tilt against X | 0.03 mm over 80 mm (0.00038 mm/mm) | 0.0007 mm/mm by dial (four-angle average), 0.0004 mm/mm by probe. About 1 to 2 times the limit |
| C2-04 horizontal tilt against X | Same | 0.0041 mm/mm by dial, 0.0022 mm/mm by probe. About 6 to 11 times the limit. Not corrected: the headstock could not be adjusted by hand |
| C2-05 axis height by A0 and A180 | 0.02 mm | Not measurable this way: 0.27 mm is the rod's offset. Axis height taken from C2P instead |
| C2-06 A0 repeat | 0.02 mm | Within 0.01 mm after A90, A180 and A270 |
| C2-07 A repeatability at 23 mm radius | Report if over 0.03 mm | 0.01 to 0.02 mm (0.05, 0.06, 0.07), all approaches from one side: repeatability, not two-sided lash |
| C2-08 runout repeat on the gate 5 stock | 0.01 mm | Not measurable: the stock is square |
| C2P-07 G54 from the probed axis, checked at station 1 | Z -0.020 to +0.010, Y +0.012 to +0.042 | Z -0.0004, Y +0.0213: within the limits, and within 0.006 mm of the first measurement at that station |

Side effects recorded: `G10 L2` with a Z word cleared the tool length offset (TL0 8.9659 to 0) and made the probe the reference tool, and a later offset step reset G54 Y, which was set again.

**Gate 5 (first cut, tooling board 35 x 35 x 100 mm, 4-jaw chuck, tailstock): partial.** The cut ran with no unexpected event; no defect was logged. E-07 to E-17, E-19, E-20, E-22 to E-25 and E-29 were recorded as completed or as expected; E-21 as not applicable. D-02: the controller's feed during the helical finishing pass agreed with the written F. D-03: cutting time as for the air cut. D-05: no chatter or burn noted.

E-26, cone diameters against the model (cone 20 mm at X 73 to 10 mm at X 98); limit ±0.1 mm (decision 0221):

| Station X (mm) | Model (mm) | Measured (mm) | Difference (mm) |
| --- | --- | --- | --- |
| 75 | 19.2 | 19.34 | +0.14 |
| 80 | 17.2 | 17.62 | +0.42 |
| 85 | 15.2 | 15.56 | +0.36 |
| 90 | 13.2 | 13.69 | +0.49 |
| 95 | 11.2 | 11.54 | +0.34 |

None of the five is within ±0.1 mm. E-27: tab diameter 9.8 mm against 10; the part's length, the tab positions and the tailstock piece are not recorded. The measured slope is -0.391 mm/mm against the model's -0.400; residuals about the best-fit line are about ±0.12 mm.

### Deviations from simulation

The simulation (E-09, E-10; 0.1 mm cell) gave a largest overcut of 0.049 mm, a largest material left of 0.889 mm at the unfinished tail, and 93.3 % of samples within ±0.1 mm (6.7 % left not classified). Against it, the cut part (E-28: "matches"; the analysis in part 3 of the record):

- **Position along X.** The shoulder where the tab steps up to the cone measured 74.43 mm from the stock start against 73.0 in the model: the cone sits about 1.4 mm further toward the tailstock than programmed. The tab starts at 70.2 mm, consistent with the same shift. Positions were taken by visual alignment, about ±0.2 mm. The cause is attributed to the X0 setting; how X0 was set is not recorded, so the cause is a hypothesis.
- **Stock and tail.** The stock is 99.5 mm long against the project's 100. With the shift, the stock ends at about program X 98.1, so no 2 mm tailstock piece remains and about 2.2 mm of the cone's small end is unfinished, where the simulation also shows material left.
- **Diameter, with the shift allowed for.** Against the model moved 1.2 mm along X, the differences are about -0.34 (X 75, near the shoulder), -0.06, -0.12, 0.00 and -0.14 mm; the widest diameter measured 19.7 mm against 20.0 and the tab 9.8 against 10. That is a radial undersize of about 0.05 to 0.15 mm, borderline against ±0.1 mm. The simulated radius at the five stations was not recorded, so the measured part was not compared with the simulated part station by station.
- Nothing was cut outside the simulated region: no fed move below X 70.1 or above X 95.9 (E-10), and no marks on the gripped length or gouges (E-25).

All figures in this subsection are calculated from the recorded readings; they are estimates and are not verified.

### Deviations from the procedure

- **The program cut is not the program air-cut.** Safe Z was changed to 35 mm for gate 5 (E-05, checked against the chuck height on the machine), so E-17's condition (the file air-cut at C11) does not hold for the program that was cut. The cut program was not air-cut.
- **Roughing tool.** A "12 mm variant" was fitted for roughing instead of tool 104 (E-06); which tool is not recorded.
- **Override.** 100 % at the start (E-18 asks for 10 %).
- **X0.** Not set or recorded as the procedure asks (E-14 refers to C2, which set Y and Z only).
- **C2 limits.** C2-01 to C2-04 were not met and the machine was not corrected before gate 5; their effect was allowed for by setting G54 Y and Z at the cone's X.
- **Earlier waivers still in force:** the original K4 block-rate waiver retained on 25 Oct 2026 and the reference-program waivers recorded in [archive/0244-z1-gates-4-5](../archive/0244-z1-gates-4-5/z1-gates-4-5-results-and-review-record.md#10-results). The rc3 gate 5 result is waived for 0.55.0 (below).

### Against the first-release exit criteria (verification plan section 9)

| Criterion | State on this record |
| --- | --- |
| Every P1 requirement has passing evidence or a recorded waiver | Not assessed here. A-02: the pull request checks and the dry run were green on `aa73540`. The traceability report of that commit is not quoted in the test record |
| No open defect of severity 1 or 2 | The rc3 test record logs no defect. The issue tracker was not reviewed for this report (release checklist, *Known issues*) |
| Release gate passes | Passed in dry run 43698165113 at `aa73540`. The real release run has not happened (OI-304, OI-2107) |
| Gates 1 to 5 passed for the reference job set on the Z1, results stored, G93 recorded | Gates 1 to 3: passed (Part A). Gate 4: passed for the ten reference programs (rc2 runs carried over on the A-04 diff, with rc3 spot checks) and for the first-cut program (C-02). Gate 5: partial, waived for 0.55.0 (below, OI-2151). G93: recorded, ignored on 1.1.3. Results stored in [archive/0653-release-0-55-verification](../archive/0653-release-0-55-verification/README.md) |
| Fuzzing ran for a set period with no unresolved crash | Not assessed here |
| Licence and advisory reports clean or decided; each spike has a written result | Not assessed here. Pull request #827 (decision 0652, heapless advisories) is open on 22 Nov 2026 |
| The G-code warning is present and checked (NFR-12) | Not separately checked in the rc3 record; A-07 found header and footer as rc2 |
| User documentation current; release checklist documentation items ticked | The dry run's strict user guide check passed at `aa73540`. The items of the application release checklist (`docs/releasing.md`) are not ticked in this record; the Z1 air cut item is met (gate 4 above) |
| Release notes drafted by a lane and reviewed before publication | Not assessed here |
| A verification report is issued and the plan reviewed | This record is the draft report. Findings for the plan review are listed below |

### Waiver: gate 5 for 0.55.0

Decided on 22 Nov 2026, relayed by the coordinator (decision 0049).

- **What is waived:** the first-release exit criterion "G-code gates 1 to 5 passed" (verification plan section 9), for gate 5 only and for release 0.55.0 only. Gate 5 on rc3 is partial: E-26 is not met as written (0 of 5 cone diameters within ±0.1 mm of the model, decision 0221), and the program cut is not the program that was air-cut (safe Z 35 mm, another roughing tool, 100 % override).
- **Why:** the cut ran with no unexpected event and matched the simulation; nothing was cut outside the simulated region. The diameters are inconclusive rather than a fault found in the program: the cone sits about 1.4 mm along X from the X0 setting, a vernier on a cone cannot resolve ±0.1 mm, and with the shift allowed for the radial undersize is about 0.05 to 0.15 mm, borderline against the limit.
- **Release notes:** 0.55.0 states in its limitations that gate 5 is partial (REL-NOTES lane).
- **Follow-ups, milestone 2026.2:** the stepped-cylinder test part (issue #826), and a repeat first cut with X0 set by probing the stock end and the part measured in the machine, using the program that was air-cut (OI-2152).
- **What the waiver is not:** it is not a claim that the generated G-code is verified or safe for any machine. Generated G-code is the user's to verify before it is run (NFR-12).

### Findings for the plan review

- The cone is a weak gate 5 test part: 1 mm of position error along X reads as 0.4 mm of diameter, and a vernier on a taper cannot resolve ±0.1 mm. Proposed in the record, not decided: compare the measured radius with the simulated radius at the same X within ±0.1 mm, and report simulation against model separately; a change to the limit or the comparison needs a record amending decision 0221. A stepped-cylinder reference part is filed as issue #826 (milestone 2026.2).
- X0 is not set by a written step: the record proposes probing the stock's end face, putting the measured stock length in the project and recording the grip depth.
- In-machine measurement with the probe (top touches at A0 and A180 per station) is proposed in place of removing the part.
- The C2 limits are proposals with no Makera source; C2-05 and C2-08 cannot be done as written on a rod and a square stock.

### What was not verified

- That any program is safe to run, on this Z1 or another machine (NFR-12).
- The program actually cut at gate 5 (safe Z 35 mm): not air-cut and not compared with the air-cut program.
- E-27 in full (length, tab positions, tailstock piece), the simulated radius at the E-26 stations, the roughing tool fitted, how X0 was set.
- Gate 5 to ±0.1 mm: not met as written; inconclusive with the X shift allowed for; waived for 0.55.0 (OI-2151), repeat cut OI-2152.
- B-05 (cone or lobed timing); the timing of the OI-993 on and off runs; the B-08 safe-Z check with a stated tool.
- The A-axis two-sided backlash (C2-07 approached from one side only); the axis tilt is measured but not corrected.
- The rotary reference Y: rc3 carries -123.39; the firmware's G54 Y read -124.39 on 21 Nov 2026 (the 14 Nov figure was a typo). Pull request #823 changes the built-in value; it is not merged on 22 Nov 2026, and its effect on rc3 output was not assessed here.
- The whole-file block-rate program (OI-2144 (a), deferred) and blocks under 5 ms (OI-2144 (b)).
- Selective short-block slowdown in representative cutting (OI-992).
- Anything on macOS or Linux on the machine side: the Z1 runs used the Windows build.
- The exit criteria marked *Not assessed here* above.

This report was drafted by lane VERIFY-REPORT in a Linux cloud container on 22 Nov 2026, branch `claude/lane-verify-report` from `main` at `37211de9`, from the three test-record comments on issue #586 as returned by the GitHub API, the status report of 20 Nov 2026 (comment 7049066645) and its update (comment 7050899047). No measurement was repeated. `cargo xtask records`: see the pull request.

### Open items

- **OI-2151 (new, closed):** the gate 5 result on rc3 is partial; waived for 0.55.0 on 22 Nov 2026 (relayed by the coordinator, decision 0049).
- **OI-2152 (new):** the repeat first cut, in 2026.2, with X0 set by probing the stock end, the part measured in the machine and the program that was air-cut.
- **OI-105:** rotary positioning precision. Decided on 22 Nov 2026 (relayed by the coordinator, decision 0049): the precision is stated in the Z1 page of the user guide and in the 0.55.0 release notes only, as one measured result on one machine at 23 mm radius (C2-06 and C2-07 above), next to Makera's 0.1 degree vendor figure; it is not stated on the setup sheet or in the machine profile. OI-105 closes in the REL-NOTES-SCREENS pull request, not here.
- **OI-992:** selective short-block slowdown on the Z1 in representative cutting; open.
- **OI-2144:** (a) the whole-file refusal, deferred on 14 Nov 2026; (b) blocks under 5 ms, open.
- **OI-2107, OI-304:** what only the first real release run can prove; open.
- **OI-23:** legal review of the CLA and the OpenCASCADE notice wording; open.
- **OI-2149:** the example projects' Z1 copies lack the minimum clearance radius; open.
- **OI-471, OI-2143:** closed by pull request #828 on the rc3 record; not merged on 22 Nov 2026.
- Closed before this report: OI-993 (20 Nov 2026), OI-41 and OI-267 (14 Nov 2026).
