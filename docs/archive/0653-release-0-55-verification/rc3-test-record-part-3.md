**Test record, Makera Z1, 0.55.0-rc3 (A-limit fix): part 3 of 3 (Part D, gate 5 first cut; stop rules and results)**

Continues the part 2 comment above. The three diagrams and two simulation screenshots in this part are not reproduced; their captions are kept in the text.

---

## Part D: gate 5, first cut (only after Parts A to C pass)

Use the gate 5 steps E-01 to E-30 of the rc2 test record unchanged: tooling board 35 x 35 x 100, tool 104 as T1 and tool 204 as T2, limit +/-0.1 mm. This is the first run with a tool and material, so the checklist for first cuts applies: soft material held firmly and square, eye and ear protection, feed override low at first, the operator at the machine for the whole cut.

What rc3 changes for gate 5: the written feed on helical passes is now the feed the machine can reach, so the controller should show the same feed as the program. Record it.

| Step ID | Tester action | Expected outcome | Recorded outcome |
| --- | --- | --- | --- |
| E-01 | Confirm gate 4 for this program | Passed | Passed, Machine calibrated |
| E-02 | Go through the first-cut safety items | All true: the air cut for this program passed and its sheet is stored; the material is soft and low-cost (tooling board), held firmly and square, and cannot slip or loosen as A turns; the stock is no larger than the project's; the tool is the project's tool, fitted with the correct stick-out and tightened, and the tool number in the program is the tool in the spindle; spindle speed, plunge and cutting feeds are as expected and within the machine's limits (1200 mm/min X and Y, 600 Z, 1800 deg/min A); override starts low | Completed |
| E-03 | Tool change with a **real tool** (OI-41): run `M5`, `T<n> M6` attended with a real tool and watch its length measurement | The tool change runs and the length is measured. The air cut ran `T1 M6` with no tool, which shows only that the line is accepted. Record what was observed | As expected |
| E-04 | First-cut limit | ±0.1 mm on every measurement in E-26 and E-27 (decision 0221) |  |
| E-05 | First-cut project: generated from the checkout, geometry as above | The project passes gates 1 to 3; the cone's direction is confirmed (record when) | Deviation: Safe Z adjusted to 35mm to support tool. Safe Z checked against chuck height on machine. |
| E-06 | Tools: show yourself the setup sheet's Tools and Operations tables | Roughing uses **tool 104** (Z1 O-flute 3.175x25, single flute, "Plastic": 10000 rpm, 0.1 mm per tooth, plunge 300, stepover 2.2225, stepdown 1.0); finishing uses **tool 204** (Z1 ball 3.175x22, two flutes, "Plastic": 12000 rpm, 0.0208 mm per tooth, plunge 200). The program numbers them T1 (104) and T2 (204): fit 104 as T1 and 204 as T2. Record that the finishing tool is two-flute | Tool changed to 12mm variant for roughing. |
| E-07 | Setup checks on the setup sheet and in the app's Workholding and tabs section | Held region at the start 68 mm, box section, and the chuck grips 10 mm or less of it (measure the jaw depth); **no grip warning** under Grip length; the 10 mm tab at X 68 to 73; "Held at the end" reads 2 mm, X 98 to 100, diameter 10; no error in setup checks (write down every warning; "safe Z cannot be checked against the Z travel" means the axis height is unconfirmed: back to P8); the part offset reads "part moved 73 mm" | As expected |
| E-08 | Material, feeds and stick-out | The setup's material is **Plastic** and "Speed from" reads "tool data: Plastic"; the program's F words are within the profile's limits and the plunge F matches the preset; each tool sticks out at least as far as the deepest cut plus a margin and its flute length covers the depth from the stock surface to the part (the box's corners are 24.7 mm from the axis); tool length is measured by the Z1 itself (no `G43`): follow the machine's procedure | As expected |
| E-09 | Simulate (`Simulate`, or `openchuck-cli simulate <project>`), default 0.1 mm cell or finer. Write down the largest material left and where (the probe and "Go to largest left" show it) | It completes with no out-of-date banner; **no overcut** beyond the tolerance and no rapid move that removes material | As expected |
| E-10 | In the simulation: check the held regions and tabs, play the program through, keep a screenshot of the coloured part, the histogram and the worst values | Held regions and tabs stay whole in the simulated stock (the 2 mm tailstock piece is not cut through); **no fed move below X 70.1 or above X 95.9** | As expected (two screenshots, not reproduced here: largest overcut 0.049 mm; largest material left 0.889 mm; within ±0.100 mm 93.3 %, 6.7 % left not classified) |
| E-11 | Measure the stock (35 x 35 x 100 mm) | It matches the project's stock, never larger | As expected |
| E-12 | Load the stock with the gripped length from E-07 and bring up the tailstock; tighten both. Turn A by jog a full turn at low feed. **Jog the tool, in the air, to X 95.9 at the radius of the cone's tail end and up to safe Z** | The stock does not move when turned or pulled by hand; A turns true by eye and nothing touches; **the tailstock body clears the tool and holder at both positions** | Completed |
| E-13 | Fit tool 104 as T1 | The tool number in the controller is the program's `T1`; stick-out as E-08 | Completed |
| E-14 | Set the work zero with **Makera's own routine**: Y and Z on the A axis, X0, A0 | G54 Y0 Z0 on the axis centreline, X0 where the setup sheet's work offset line says, A0 as the routine sets it. Record the face X0 was taken from, how A0 was set and G54 X, Y, Z in machine coordinates; compare the G53 Z of G54 with the profile's axis height | Set as in section C2 |
| E-15 | Jog to the program's first `G0` position in G54 (X, Y0, safe Z, A0) at low feed | The tool is above the stock at the X the program starts at and clears the jaws and the tailstock body | As expected |
| E-16 | Protection | Eye and ear protection on; loose clothing, hair and jewellery secured; dust extraction or a mask | Completed |
| E-17 | Reread the program one more time | It is the file that was air-cut at C11. If the tool, speeds or stock changed, read it from the start again and air-cut it again | Completed |
| E-18 | Set feed override to 10 %; hand on feed hold and the emergency stop | Done. Record the override used | 100% |
| E-19 | Start. Watch the tool change and the spindle start | `M5`, `T1 M6`, `S10000 M3` as the program says | As expected |
| E-20 | Watch the first plunge | The tool comes down at the plunge feed to the first cut, no faster | As expected |
| E-21 | Raise the override slowly, only while the cut sounds and looks steady | Steady cut at each step; record the override steps and what it sounded like | N/A |
| E-22 | At the program's tool change to T2 (finishing): change to tool 204 by hand as the controller asks, with the spindle stopped | Tool 204 fitted with the correct stick-out and tightened; its length measured; `S12000 M3`; the finishing pass starts as the simulation shows | As expected |
| E-23 | Stop at any unexpected sound, vibration, smell, smoke or motion | None. For tooling board, watch for tearing, the stock moving, or the tailstock piece breaking | None |
| E-24 | At the end, wait until everything has stopped before opening the enclosure | Spindle stopped, everything stationary | As expected |
| E-25 | Look at the part before it is removed. Photograph it from four sides (A 0, 90, 180, 270) | Both tabs whole, no marks on the gripped length, no gouges | As expected |
| E-26 | Measure the diameters at 5 or more stations along the 25 mm, away from the tabs (table below) | Each within **±0.1 mm** of the model at that X | See the table below |
| E-27 | Measure the part's length and the tabs' positions and diameters | 25 mm; tab 10 mm diameter at X 68 to 73; tailstock piece 10 mm at X 98 to 100; each within ±0.1 mm | tab: 9.8 |
| E-28 | Compare with the simulation (E-09, E-10) | Where the simulation showed material left the part has it too, about as much; where it showed none, there is none. A difference larger than 0.1 mm is a finding | matches |
| E-29 | Surface | Note anything that looks wrong: steps, chatter, a seam where A wraps | None |
| E-30 | Decide the result | **Pass** when E-01 to E-28 passed, the cut ran with no unexpected event, and E-26 to E-28 are within ±0.1 mm; otherwise **fail**: a machine-side cause (stock slipped, wrong offset) is recorded and the cut repeated; a cause in the program is a defect (section F) | **Pass / fail:** see the results table (Partial) |

**E-26 measurements** (model diameters are for a cone 20 mm at X 73 falling to 10 mm at X 98)

| Station X (mm) | Model diameter (mm) | Measured (mm) | Difference (mm) |
| --- | --- | --- | --- |
| 75 | 19.2 | 19.34 | +0.14 |
| 80 | 17.2 | 17.62 | +0.42 |
| 85 | 15.2 | 15.56 | +0.36 |
| 90 | 13.2 | 13.69 | +0.49 |
| 95 | 11.2 | 11.54 | +0.34 |

**Instrument used and its resolution:** digital vernier (generic), 0.01 mm. The differences are calculated from the recorded values; none of the five is within 0.1 mm.

### Gate 5 first cut: analysis and X0 finding (22 Nov 2026)

**E-26 as measured.** Diameters against the model at X 75, 80, 85, 90, 95: +0.14, +0.42, +0.36, +0.49, +0.34 mm. As written (within 0.1 mm of the model at every station) none of the five passes. The slope of the measured diameters is -0.391 mm/mm against the model's -0.400, so the shape is right. The residuals about the best-fit line are about +/-0.12 mm: a cone loses 0.4 mm of diameter per mm of X, so 0.25 mm of error in where the jaws sit gives 0.1 mm on diameter, and measuring a cone with jaws cannot test +/-0.1 mm. (Diagram: why a position error looks like a diameter error on a cone: a 1.4 mm position error reads as 0.56 mm of diameter at every station.)

**X0 finding.** Positions were measured from the stock start (by visual alignment, so about +/-0.2 mm). The shoulder where the tab steps up to the cone is at 74.43 mm against the model's 73.0, so the cone sits about **1.4 mm further toward the tailstock** than programmed. The measured widest diameter of the cone is 19.7 mm (model 20.0). The tab starts at 70.2, which is consistent with the same shift: the fed moves never go below X 70.1 and the ball's edge reaches about X 68.5 at the stock surface, which with a 1.4 mm shift is about 69.9. The stock is 99.5 mm long (99.55 and 99.52 are one measurement, with some contact error), 0.5 mm short of the project's 100. With the shift, the stock ends at program X of about 98.1: this part has no 2 mm tailstock piece, and the cone's small end is at the end of the stock. The fed moves stop at program X 95.9 (about 97.3 on the part), so about 2.2 mm of the cone's small end is unfinished, as the simulation showed. The cause is the X0 setting, about 1.4 mm toward the tailstock from the stock start. If X0 was set from the chuck face assuming 10 mm gripped, the real grip would be about 11.4 mm; how X0 was set is not recorded, so this is a hypothesis. (Diagram: programmed and measured features along X, to scale; programmed shoulder at X 73.0, measured at 74.43, +1.43 mm.)

**E-26 with the shift allowed for.** Against the model moved 1.2 mm along X, the cone's diameters read about -0.34 (X 75, near the shoulder), -0.06, -0.12, 0.00 and -0.14 mm. The widest end (-0.3 mm) and the tab (9.8 mm against 10, -0.2 mm) are also under. That is a consistent radial undersize of about 0.05 to 0.15 mm. The simulation (E-10) gave 93.3% of samples within +/-0.1 mm, a largest overcut of 0.049 mm and a largest material left of 0.889 mm at the unfinished tail.

**Conclusion.** The program's geometry is right and the first cut ran as the simulation showed. The diameters are not conclusive as a pass or a fail: the position error is a work-zero setting error, not a program or machine accuracy fault, and the remaining radial undersize of about 0.05 to 0.15 mm is borderline against +/-0.1 mm. The cone is a poor test part (position error of 1 mm is 0.4 mm of diameter). Nothing was cut that should not have been.

**Criteria.** E-26 compares with the model, but the simulation expects up to 0.889 mm of material left at the unfinished tail. Proposed, to be confirmed (decision 0221): compare the measured radius with the simulated radius at the same X, within +/-0.1 mm, and report simulation against model separately. Known contributions to list beside the results: the Z tilt left after re-setting G54 (about 0.004 mm over this cone), the tool length measurement (about +/-0.02 mm), and the yaw (second order on a round section). (Diagram: model, simulated part and measured part; the simulation against the model is the program's known deviation, the measured part against the simulation is what gate 5 should test.)

**Next time.** Set X0 by touching the end face of the stock with the probe (off the tailstock centre) and take X0 as that machine X minus the measured stock length; put the measured stock length in the project; measure and record the real grip depth (W1). Measure the part in the machine with the probe (top touches at A0 and A180 at each station, mean of the two minus the axis Z at that X; about +/-0.02 mm) so the part need not be removed. Use a stepped-cylinder reference part, which a micrometer measures independently of X position (an enhancement is filed as issue #826, milestone 2026.2).

**Gaps in the record:** the other E-27 values (length, tab positions, tailstock piece), the simulated radius at the five stations, which "12 mm variant" roughing tool was fitted (E-06), the override used at the start (E-18 asks for 10 %, the record says 100 %), and the program that was cut (safe Z was changed to 35 mm, E-05, so it is not the program that was air-cut). The instrument's accuracy is typically two to three times its resolution, and its flat jaws read high on a taper. All figures above are estimates calculated from the recorded readings and are not verified. Generated programs and these measurements remain the user's to verify (NFR-12).

### Part D summary table

| Step ID | Action | Expected outcome | Recorded outcome |
| --- | --- | --- | --- |
| D-01 | Gate 5 steps E-01 to E-30 as in the rc2 record: link the record used |  | See above |
| D-02 | Controller feed during the helical finishing pass against the written F word | Agree (rc2: 30 to 45 % lower than written) | As expected |
| D-03 | Cutting time against the header estimate | Within 10 % | As per air cut |
| D-04 | Cone diameter at X 75 / 80 / 85 / 90 / 95 mm | 19.2 / 17.2 / 15.2 / 13.2 / 11.2 mm, each within +/-0.1 mm | See above |
| D-05 | Finish and tool condition | Recorded; notes on any chatter or burn | None |

## Stop rules, defect report and results

**Stop at the first of:** unexpected motion, an alarm, a refused or rejected line, a measured time outside the band with a stutter or pause, a result that disagrees with the diff check (Part A), or anything unexpected. Testing stops there and an earlier pass is not reused. The event is logged below with the program and the rc3 commit attached, and reported.

| Defect | Step ID | Program and commit | What happened | Severity |
| --- | --- | --- | --- | --- |
| None logged |  |  |  |  |

| Part | Result (pass / partial / fail) | Notes |
| --- | --- | --- |
| A: before the machine | Pass |  |
| B: Z1 spot checks | Pass |  |
| C: first-cut air cut and OI-993 | Pass |  |
| C2 - A-Axis alignment | Partial | Dial indicator and probe measurements on a 10 mm rod, 20 and 21 Nov. The spindle bearing runs true (0.01 mm TIR) but the chuck mount does not: the nose thread reads 0.24 mm TIR, and rod TIR was 0.32 and 0.36 mm (4-jaw, one jaw not seating fully) and 0.14 and 0.23 mm (3-jaw) against proposed limits of 0.03 and 0.05 mm. A0 repeated within 0.01 mm and A repeatability was 0.02 mm at 23 mm radius (within limit). Axis tilt against X, averaged over four angles: vertical 0.0007 mm/mm by dial and 0.0004 by probe; horizontal 0.0041 mm/mm by dial and 0.0022 by probe; proposed limit 0.00038 mm/mm (0.03 mm over 80 mm), so the vertical tilt is about 1 to 2 times the limit and the horizontal about 6 to 11 times. The headstock adjustment could not be made by hand, so the tilt is uncorrected. C2-05 cannot give axis height by A0 and A180 on a rod, and C2-08 could not be measured on the square stock. Allowed for by setting G54 Y and Z from the probe-measured axis at the cone (Y -124.136, Z -39.710); a check at the far station read within 0.006 mm of the prediction. Side effects: G10 L2 with a Z word cleared the tool length offset and made the probe the reference tool, and a later offset step reset G54 Y, which was set again. The limits are proposals, not Makera specifications. |
| D: gate 5 first cut | Partial | Cut completed with no unexpected event; part matched the simulation. E-26 not met as written (0 of 5 within ±0.1 mm, +0.14 to +0.49 mm). Cone position error of about 1.4 mm from the X0 setting, and jaw measurement on a cone, make the diameters inconclusive. After allowing for the shift, radial undersize about 0.05 to 0.15 mm. Deviations: safe Z 35 mm, roughing tool variant, 100% override, program differed from the air-cut. Follow-up, future testing approach: probe measurement in the machine, stepped-cylinder reference part. |
| Carried over from rc2 as listed | Pass | Confirmed by Part A diff check: yes |

Signed off (name, date): Paul Hampson / 22 Nov 2026

---
_Generated by [Claude Code](https://claude.ai/code)_