**Test record, Makera Z1, 0.55.0-rc3 (A-limit fix): part 2 of 3 (Part C2, A-axis alignment)**

Continues the part 1 comment above. The diagram in this part is not reproduced. One transcription note: at station 1, A180, the notes read `Zt0 = -34.768` where the PRB line shows -34.798; the value was used only to set the side-touch height and does not enter the axis result.

---

## Part C2: A-axis alignment check before gate 5 (proposed limits, to be confirmed)

Gate 5 measures the part to +/-0.1 mm, so a misaligned or loose A axis would be blamed on the software. This check measures the machine with a dial indicator on stock hardware only: no community firmware, no controller changes. It records errors; it does not compensate for them. No Makera-published procedure or tolerance for this was found for the Z1, so the limits below are proposals, not Makera specifications.

Use a straight, round test bar (ground rod or a dowel known to be round) held in the chuck exactly as the gate 5 stock will be, with the tailstock if one is used. Dial indicator on a magnetic base on the spindle nose or bed, 0.01 mm resolution or better.

| Step ID | Action | Proposed limit | Recorded outcome |
| --- | --- | --- | --- |
| C2-01 | Chuck and bar runout: indicate the bar close to the jaws, rotate A by hand or by `G1 A90` steps through 360 degrees; read total indicator reading (TIR) | 0.03 mm or less | 20mm from chuck face TIR near jaws: -0.06mm to 0.26mm 3 Jaw chuck: 23mm from chuck face: -0.1 to 0.04mm 74mm from chuck face: -0.15 to 0.08 |
| C2-02 | Same at the free end of the bar (about 80 mm from the jaws) | 0.05 mm or less; the difference from C2-01 shows bar bend or chuck tilt | 75mm from chuck face TIR far end: -0.01 to 0.35 |
| C2-03 | Axis parallel to X, vertical: jog X along the top of the bar over 80 mm with A fixed; read the indicator change | 0.03 mm or less over 80 mm | 10mm from chuck face: 0.25mm 74mm from chuck face: 0.0mm 3 jaw A360: 74mm from chuck face: 0.0 14mm from chuck face: 0.04 A90 (same setup as A180): 74mm from chuck face: 0.19 18mm from chuck face: 0.18 A180 (dial reset): 73mm from chuck face: 0.00 18mm from chuck face: 0.07 A270 (same setup as A180): 73mm from chuck face: 0.08 18mm from chuck face: 0.14 |
| C2-04 | Axis parallel to X, horizontal: same along the side of the bar (indicator on the side, A fixed) | 0.03 mm or less over 80 mm | 75mm from chuck face: 0.0mm 20mm from chuck face: -0.19mm 3 jaw 72mm from chuck face: A0: 0.05 A90: 0.20 A180: 0.17 A270: 0.02 19mm from chuck face: A270: 0.27 A180: 0.36 A90: 0.36 A0: 0.32 |
| C2-05 | Axis height: indicate the top of the bar at A0, then turn A 180 degrees and indicate the same spot; half the difference is the centre-height error against G54 Z0 | 0.02 mm or less against 48.567 (collet-face calibration, 14 Nov; 48.568 in the rc2 record) | A0: 0.0mm A180: 0.27mm Error: \_\_\_\_ |
| C2-06 | A0 position and repeat: set A0, turn to A90, A180, A270 and back to A0 with `G1` moves; indicator on the bar's side at a mark; compare the reading at A0 each time | Same reading within 0.02 mm | A0: 0.0 A90 back to A0: 0.01 A180 back to A0: 0.01 A270 back to A0: 0.01 Readings: \_\_\_\_ |
| C2-07 | A backlash: indicator on a flat clamped to the bar at a known radius; approach A+10 from A0, read; approach it from A+20 coming back, read; the difference over the radius gives the lash | Record only; if over 0.03 mm at the radius, report it before gate 5 | 23mm radius A0: 0.05 A-10 to A0: 0.06 A-20 to A0: 0.07 Lash: \_\_\_\_ |
| C2-08 | Chuck, bar and any tailstock tightened as for gate 5; repeat C2-01 once | Same as the first reading within 0.01 mm (nothing moved) | TIR: \_\_\_\_ (not measurable: the gate 5 stock is not cylindrical) |

### C2 investigation summary (20 and 21 Nov 2026)

Dial-indicator checks with a 10 mm linear rod as the reference. The gate 5 stock is a square box, so C2-08 could not be done as written. The readings are in the table above; the analysis that was in the C2-03 and C2-04 cells is moved here.

The rod sits off-centre in the chuck, so its top reads high at one angle and low at the opposite one. The average of the two is the axis. This is the basis of the dial method (four angles) and of the probe steps (C2P).

**Findings**

- **The spindle bearing runs true.** A smooth area behind the A-axis nose thread reads 0.01 mm TIR. The nose thread itself reads 0.24 mm (-0.02 to +0.22), so a chuck that screws onto it sits about 0.12 mm off-centre. That is an effect of the chuck mount, not an axis fault.
- **Rod runout (C2-01, C2-02).** 4-jaw: TIR 0.32 mm at 20 mm and 0.36 mm at 75 mm from the chuck face (limits 0.03 and 0.05). One jaw of the 4-jaw does not seat tightly when fully closed. 3-jaw: 0.14 mm at 23 mm and 0.23 mm at 74 mm. These limits cannot be met with this rod and chuck mount.
- **C2-05 (A0 against A180, 0.27 mm)** is the rod's offset in Z, not an axis-height error. Turning a rod 180 degrees cannot show the axis height; that comes from C2P (the average of the A0 and A180 tops, minus the radius).
- **C2-06 and C2-07 pass.** A0 repeats within 0.01 mm after A90, A180 and A270. A repeatability at 23 mm radius is 0.01 to 0.02 mm (readings 0.05, 0.06, 0.07), under the 0.03 mm limit. All approaches came from the same side, so this is repeatability, not a two-sided lash.
- **Vertical tilt (C2-03, 3-jaw).** Averaging the four angles removes the rod's own tilt. Near minus far is +0.040 mm over 55 mm, which is 0.0007 mm/mm (0.04 degrees), range 0.025 to 0.055 mm, about 2 times the proposed 0.03 mm over 80 mm. The rod's own tilt in the jaws is about 0.04 mm. The 4-jaw reading (0.25 mm over 64 mm) was mostly the rod cocked in the unseated jaw.
- **Horizontal tilt (C2-04).** 3-jaw: near minus far is 0.22 mm over 53 mm (A0 +0.27, A90 +0.16, A180 +0.19, A270 +0.25), which is 0.0041 mm/mm (0.24 degrees), about 11 times the limit. The rod's own tilt is about 0.06 mm. The 4-jaw run, indicated on the other side of the bar, gave 0.19 mm over 55 mm: the same size and direction once the sides are allowed for. So this is the A axis against the X travel, not the chuck.
- **Headstock adjustment tried (21 Nov).** The headstock has four mounting screws about 16 mm apart on X, so correcting the yaw needs about 0.07 mm of difference at the mounts. The adjustment could not be made by hand.
- **C2-08** could not be measured: the gate 5 stock is not cylindrical.

**What this means for gate 5** (estimates, not verified)

- The vertical tilt acts directly on the cut radius: the error is 0.0007 mm/mm times the distance from where Z0 was set. If Z0 was set near the chuck, that is about 0.04 mm of radius at the middle of the cone (0.08 mm of diameter), plus about 0.02 mm of spread across the cone.
- The horizontal yaw acts only to second order on a round section (about 0.01 mm at a 0.35 mm axis offset), so it matters much less for the cone.
- The X position where Y0 and Z0 were set is not recorded.

**Decision (21 Nov 2026):** the yaw is not corrected and gate 5 uses the 4-jaw chuck. To remove most of the effect, G54 Y and Z are set again from the axis position measured with the probe at the cone's X (C2P: two stations, averages of A0 and A180 so the rod's offset and bend cancel; the stations were 60 mm apart in the event, not the 65 mm planned).

**Proposed allowances for the gate 5 results (to be confirmed).** Keep the +/-0.1 mm criterion. Record next to the measurements these known contributions: the Z tilt left after re-setting (0.0007 mm/mm times the distance from the cone middle, about +/-0.01 mm across the cone), the Y yaw of 0.0041 mm/mm (second order), the thread eccentricity of the chuck mount (it moves the stock, not the part), and the 4-jaw jaw that does not seat. A result outside +/-0.1 mm that lies within these is recorded as explained, not as a pass. Changing the limit itself needs a decision record (decision 0221).

### C2 with the 3D probe instead of a dial indicator on the spindle

From a read of the Z1 firmware source v1.1.3 (not tried on a machine beforehand, nothing here is verified safe; if a step gives an unexpected result, stop). Probe fitted: Makera 3D Probe (the stock wired probe is for Z touches only; side touches need the Makera 3D Probe, or the 3D probe rod with a proven electrical path to the table).

**Rules for every touch:**

- `G38.2` only (an alarm on a miss), always with F 20 to 60, and every distance short, 5 mm at most. The X, Y and Z words in `G38.2` are RELATIVE distances (`G38.2 Z-5 F60` means go down up to 5 mm from here, not to Z-5), and soft limits are not checked during a probe move.
- Before each touch send `M119` and check it prints `Probe: 0`. If the probe is already triggered the move aborts with an error.
- The reading appears in the console as `[PRB:x,y,z:flag]` (flag 1 = contact), in MACHINE coordinates, and only X, Y and Z can be probed.
- Back off with `G91` `G0 Z2`, then `G90` before any absolute move. Rotate A only with the probe well above the bar: `G90 G0 A90`, always in the same direction.
- Check `M220` is back at 100 before measuring. Do NOT use `M495`, `M480.x` or the other Makera probing routines for this check: they change G54.
- To clear a halt send `M999` or `$X`, then home.

**Top touch (Z):** jog the probe to 2 to 3 mm above the bar, then `M119`, `G38.2 Z-5 F60`, `G91 G0 Z1`, `G90`, `G38.2 Z-2 F20`, record the Z, `G91 G0 Z3`, `G90`. Take three slow touches and use the mean.

**Exact sequence for the 10 mm rod in the 4-jaw chuck, two stations.** About 45 to 60 minutes and about 20 touches. Every Y and Z reading is in MACHINE coordinates from the `[PRB:...]` line, so nothing depends on the G54 that the Z1 set for itself. Why it works: the rod is off-centre and slightly bent, and that offset turns with A, so it changes sign between A0 and A180. The average of the A0 and A180 readings at a station is therefore the axis itself, whatever the rod's offset or bend, and the difference between the two stations' averages, divided by the distance between them, is the axis tilt.

| Step ID | Action | Proposed limit | Recorded outcome |
| --- | --- | --- | --- |
| C2P-00 | Setup. Rod tight in the 4-jaw (tug and turn test); do not touch the chuck or the rod from here on. Fit the Makera 3D Probe and send M499. Record G54 as found (X, Y, Z) with the READ G54 block. Micrometer: rod radius R1 at station 1 and R2 at station 2. Station 1 is at least 10 mm clear of the jaws. Station 2 is a fixed distance further along X. The rod must reach at least 5 mm past station 2 | Rod does not move | G54: -118.3900, -124.3900, -39.6435, 0.0000, 0.0000. TL0 0 and M499 ref: -68.626. d1 (station 1 from the chuck face) 88 mm. R1 5.015, R2 5.005 |
| C2P-01 | SETUP block, then STATION block at station 1, A0. At the first final top touch send M114 immediately and record the WCS Z (the anchor) | The two final top touches agree within 0.02 mm | See the STATION records below |
| C2P-02 | Move to station 2; STATION block at station 2, A0 | Same | See the STATION records below |
| C2P-03 | ROTATE block to A180; STATION block at station 2, A180 | Same | See the STATION records below |
| C2P-04 | Move to station 1; STATION block at station 1, A180 | Same | See the STATION records below |
| C2P-05 | CALCULATION block | Z tilt about 0.0007 mm/mm and Y tilt about 0.0041 mm/mm (from the dial run), each within 0.001. Axis Z (spindle reference) and axis Y within the stated distance of the profile values; if either is further out, stop and report | See the CALCULATION record below |
| C2P-06 | SET G54 block: Y and Z only, at the cone's X; X unchanged | G54 X as found | Y -124.136, Z -39.710 (see the SET G54 record) |
| C2P-07 | VERIFY block at station 1 | Axis reads 0 within 0.02 in WCS Z and WCS Y (expected residuals Z -0.005, Y +0.027 at station 1) | Z -0.0004, Y +0.0213: pass (see the VERIFY calculation) |

**Probe notes (from a read of the Z1 firmware source and the Makera wiki; untried on a Z1 beforehand):**

- The Z1 has three probes on the same connector, and the firmware cannot tell which is fitted: the stock wired probe (Z touches only), the 3D probe rod (needs the work electrically connected to the table) and the Makera 3D Probe accessory (X, Y, Z on conductive or non-conductive work, factory calibrated, recalibrate after a collision). The Makera 3D Probe is fitted, so side touches are possible.
- `G38.2` through `G38.5` exist; `G38.2` alarms ("ALARM: Probe fail") on a miss, `.3` does not. `G38.6` is the tool-setter calibration: do not use it here. The X, Y and Z words are relative distances, the soft limits are not checked while probing, and `G20` is ignored. If the probe is already triggered the move aborts with "ZProbe triggered before move" and the machine halts.
- There is no automatic back-off: each probe command is a single feed. The trigger is polled at 1 kHz and the axes stop abruptly, so the reading can overshoot by about one millisecond of travel (0.001 mm at F60). Debounce, rates and repeatability are not published; read the machine's own values with `M503` and `config-get sd zprobe.debounce_ms`.
- Since firmware 1.1.3, with the probe as the active tool (T0 or T9999) a trigger during a non-probing X or Y move, or a Z move in one direction, halts with "ALARM: unexpected probe trigger". If that happens, clear it with `M999` or `$X`, then home.
- Machine position readings `[PRB:x,y,z:flag]` exclude the tool offset, so they are the spindle reference position; differences between two touches do not depend on the tool length. For a work-frame reading send `M114` right after a touch (it includes the WCS and the tool offset); `M114.2` prints the machine position; `M114.4` prints A.
- If auto-levelling compensation is active, clear it first with `M370` (this deletes the levelling grid). Makera's routines `M495`, `M493` and `M480.x` set the work offset: do not use them for this check. The tool number of the probe (0 or 9999) on the Z1 is unverified; use the Makera Controller's probe selection so its calibration runs.
- Expected accuracy 0.005 to 0.02 mm per touch (resolution 0.001 mm); the mean of three touches helps. Same-direction comparisons cancel backlash; opposite-direction comparisons carry the tip-diameter bias in the diameter but give a good centre. The probe cannot measure: a continuous runout sweep, bar straightness apart from axis tilt (the A0 and A180 average separates most of it), A backlash, or the bottom of the bar.
- Do not use the community method "G38.2 Z-150 F100" for aligning stock: on this firmware it is a 150 mm relative plunge with soft limits off.

**Commands for the C2P sequence (Makera 3D Probe; unverified on a Z1, nothing here is verified safe).** Values in angle brackets are filled in at the machine. R is the rod radius at the station in use. Zt0, Y_est, Ya, Yb, Yc and Zt come from the `[PRB:x,y,z:flag]` reply (machine coordinates). A line with G53 uses machine coordinates for that line only. G0 is a rapid: keep every move short and watch the probe against the rod and the jaws.

```
SETUP (once)
G21
G90
M220 S100                 (feed override 100 %)
M499                      (the active tool must be the probe)
M114                      (record the WCS and machine positions)
(Record G54 as found. Jog by hand: probe tip 2 to 3 mm above the rod crest at station 1, clear of the jaws, A0, within 0.5 mm of the rod centre in Y.)
```

**Record: SETUP**

```
[19:30:55] G21
[19:30:55] ok
[19:30:58] G90
[19:30:58] ok
[19:31:04] M220 S100
[19:31:04] ok
[19:31:09] M499
[19:31:09] tool:9999 ref:-77.592 cur:-68.626 offset:8.966
[19:31:09] ok
```

```
READ G54 (probe stationary, at station 1)
M114                       (record the WCS position: this is the anchor reading for CALCULATION)
$#                         (works on this Z1, confirmed on 21 Nov 2026. It lists G54 to G59 and, on the TL0 line, the tool length offset, separately)
Record: G54 as found X ____ Y ____ Z ____ and TL0 as found ____.
Do NOT send G10 L2 as a test. On 21 Nov a G10 L2 P1 Z also cleared TL0 (8.9659 to 0), which moved the WCS Z by that amount as well.
```

**Record: READ G54**

```
[19:37:28] M114
[19:37:28] ok C: X:34.3900 Y:-0.0000 Z:7.6434
[19:37:41] $#
[19:37:41] [G54:-118.3900,-124.3900,-48.6094,0.0000,0.0000]
[19:37:41] [G55:0.0000,0.0000,0.0000,0.0000,0.0000]
[19:37:41] [G56:0.0000,0.0000,0.0000,0.0000,0.0000]
[19:37:41] [G57:0.0000,0.0000,0.0000,0.0000,0.0000]
[19:37:41] [G58:0.0000,0.0000,0.0000,0.0000,0.0000]
[19:37:41] [G59:0.0000,0.0000,0.0000,0.0000,0.0000]
[19:37:41] [G59.1:0.0000,0.0000,0.0000,0.0000,0.0000]
[19:37:41] [G59.2:0.0000,0.0000,0.0000,0.0000,0.0000]
[19:37:41] [G59.3:0.0000,0.0000,0.0000,0.0000,0.0000]
[19:37:41] [G28:0.0000,0.0000,0.0000]
[19:37:41] [G30:0.0000,0.0000,0.0000]
[19:37:41] [G92:0.0000,0.0000,0.0000,0.0000,0.0000]
[19:37:41] [TL0:8.9659]
[19:37:41] [PRB:-9.8900,-12.3900,-68.6259:1]
[19:37:41] ok
[19:40:17] M114
[19:40:17] ok C: X:34.3900 Y:-0.0000 Z:7.6434
[19:40:31] G10 L2 P1 Z7.6434
[19:40:32] ok
[19:40:34] M114
[19:40:34] ok C: X:34.3900 Y:-0.0000 Z:-39.6434
[19:43:28] G10 L2 P1 Z-39.6435
[19:43:28] ok
[19:43:32] M114
[19:43:32] ok C: X:34.3900 Y:-0.0000 Z:7.6435
```

```
STATION (a = the A angle; R = the rod radius at this station; the probe starts 2 to 3 mm above the crest, within 0.5 mm of the centre in Y)
A. Rough top touch
M119                      (must print Probe: 0)
G38.2 Z-5 F60             (record machine Y as Y_est and machine Z as Zt0)
G91
G0 Z3
G90
B. Side touches (machine coordinates)
G53 G0 Z-19.618
G53 G0 Y-133.49
G53 G0 Z-39.633        (tip bottom at about the rod's axis height; watch the jaws and the rod)
M119
G38.2 Y8 F30              (record machine Y: Ya)
G91
G0 Y-2
G90
G53 G0 Z-19.618
G53 G0 Y-115.49
G53 G0 Z-39.633
M119
G38.2 Y-8 F30            (record machine Y: Yb)
G91
G0 Y2
G90
G53 G0 Z-19.618
C. Final top touches at the centre
(Yc = (Ya + Yb)/2 for this station and angle)
G53 G0 Y<Yc>
G91
G0 Z-12
G90
M119
G38.2 Z-5 F60
G91
G0 Z1
G90
G38.2 Z-2 F20             (record machine Z: first final touch. At station 1, A0 send M114 now: that is the anchor)
G91
G0 Z3
G90
G38.2 Z-4 F20             (record machine Z: second final touch; the two agree within 0.02 mm; Zt = their mean)
G91
G0 Z3
G90
```

**Record: STATION.** Station 1 is at machine X -84 (far, 88 mm from the chuck face); station 2 is at machine X -144 (near, 28 mm).

*Station 1, A0, part A (rough top touch):*

```
[19:50:41] M114
[19:50:41] ok C: X:34.3900 Y:-0.0000 Z:7.6435
[19:51:35] G38.2 Z-5 F60
[19:51:39] [PRB:-84.000,-124.390,-34.617:1]
[19:51:39] ok
[19:51:58] M114
[19:51:58] ok C: X:34.3900 Y:0.0000 Z:5.0266

[19:52:40] G38.2 Z-5 F60
[19:52:43] [PRB:-84.000,-124.390,-34.618:1]

[19:53:06] G38.2 Z-5 F60
[19:53:09] [PRB:-84.000,-124.390,-34.618:1]
[19:53:09] ok
```

Zt0 = -34.618

*Station 1, A0, part B (side touches):*

```
[20:03:58] G38.2 Y8 F30
[20:04:07] [PRB:-84.000,-130.041,-39.633:1]

[20:32:01] G38.2 Y8 F30
[20:32:09] [PRB:-84.000,-130.038,-39.633:1]

[20:39:52] G38.2 Y-8 F30
[20:39:58] [PRB:-84.000,-118.238,-39.633:1]

[20:41:04] G38.2 Y-8 F30
[20:41:08] [PRB:-84.000,-118.238,-39.633:1]

[20:41:47] G38.2 Y-8 F30
[20:41:51] [PRB:-84.000,-118.238,-39.633:1]
```

Ya = -130.040, Yb = -118.238, Yc = -124.139

*Station 1, A0, part C (final top touches):*

```
[20:48:54] G38.2 Z-5 F60
[20:48:57] [PRB:-84.000,-124.139,-34.611:1]

[20:49:39] G38.2 Z-2 F20
[20:49:42] [PRB:-84.000,-124.139,-34.604:1]
[20:49:42] ok
[20:50:03] M114
[20:50:03] ok C: X:34.3900 Y:0.2513 Z:5.0391

[20:50:50] G38.2 Z-4 F20
[20:51:00] [PRB:-84.000,-124.139,-34.608:1]
[20:51:00] ok
[20:51:05] M114
[20:51:05] ok C: X:34.3900 Y:0.2513 Z:5.0357
```

Zt = -34.606 at A0

*Station 1, A180:*

```
[20:57:27] G38.2 Z-5 F60
[20:57:30] [PRB:-84.000,-124.139,-34.798:1]
```

Zt0 = -34.768 (as noted; the PRB line shows -34.798). Rod axis height used for the side touches: -39.783.

```
[21:01:54] G38.2 Y8 F30
[21:02:02] [PRB:-84.000,-129.974,-39.783:1]

[21:03:56] G38.2 Y-8 F30
[21:04:02] [PRB:-84.000,-118.184,-39.783:1]
```

Ya = -129.974, Yb = -118.184, Yc = -124.079

```
[21:08:10] G38.2 Z-5 F60
[21:08:14] [PRB:-84.000,-124.079,-34.798:1]

[21:08:49] G38.2 Z-2 F20
[21:08:53] [PRB:-84.000,-124.079,-34.793:1]
[21:08:53] ok
[21:08:54] M114
[21:08:55] ok C: X:34.3900 Y:0.3112 Z:4.8504

[21:09:12] G38.2 Z-4 F20
[21:09:21] [PRB:-84.000,-124.079,-34.796:1]
```

Zt = -34.7945

*Station 2 (rod diameter 10.01, R = 5.005), A0:*

```
[21:13:32] M114
[21:13:32] ok C: X:-25.6100 Y:0.3112 Z:9.8473

[21:14:34] G38.2 Z-5 F60
[21:14:39] [PRB:-144.000,-124.079,-34.681:1]
```

Zt0 = -34.681

```
[21:20:04] G38.2 Y8 F30
[21:20:14] [PRB:-144.000,-130.212,-39.686:1]

[21:21:45] G38.2 Y-8 F30
[21:21:51] [PRB:-144.000,-118.413,-39.686:1]
```

Ya = -130.212, Yb = -118.413, Yc = -124.3125

```
[21:25:59] G38.2 Z-5 F60
[21:26:02] [PRB:-144.000,-124.312,-34.679:1]

[21:26:57] G38.2 Z-2 F20
[21:27:01] [PRB:-144.000,-124.312,-34.673:1]
[21:27:01] ok
[21:27:02] M114
[21:27:02] ok C: X:-25.6100 Y:0.0775 Z:4.9710

[21:27:28] G38.2 Z-4 F20
[21:27:37] [PRB:-144.000,-124.312,-34.676:1]
[21:27:37] ok
```

Zt = -34.6745

*Station 2, A180:*

```
[21:30:32] G38.2 Z-5 F60
[21:30:35] [PRB:-144.000,-124.312,-34.699:1]
```

Zt0 = -34.699; axis target = -39.704

```
[21:33:49] [PRB:-144.000,-130.066,-39.704:1]
[21:35:46] [PRB:-144.000,-118.267,-39.704:1]
```

Ya = -130.066, Yb = -118.267, Yc = -124.1665

```
[21:38:37] [PRB:-144.000,-124.166,-34.694:1]
[21:38:37] ok C: X:-25.6100 Y:0.2237 Z:4.9494
[21:38:47] [PRB:-144.000,-124.166,-34.697:1]
[21:38:47] ok
```

Zt = -34.6955

```
ROTATE (after the A0 stations, probe well above the rod)
G53 G0 Z<Zt + 15>
G90
G0 A180                   (always turn the same way)
(Move to within 0.5 mm of the rod centre in Y again, probe 2 to 3 mm above the crest, before the STATION block.)
```

```
CALCULATION (machine coordinates; s = station 1 or 2, a = A0 or A180)
Yc[s,a] = (Ya + Yb)/2              Zt[s,a] = mean of the two final top touches
Zaxis[s] = (Zt[s,A0] + Zt[s,A180])/2 - R[s]     (spindle-reference Z of the axis; the rod's offset and bend cancel)
Yaxis[s] = (Yc[s,A0] + Yc[s,A180])/2
Z tilt = (Zaxis[2] - Zaxis[1]) / spacing      Y tilt = (Yaxis[2] - Yaxis[1]) / spacing     (mm per mm)
Rod offset at station s = ((Yc[s,A0] - Yc[s,A180])/2 , (Zt[s,A0] - Zt[s,A180])/2)
Rod bend between the stations = the difference of the two rod offsets (it does not change the axis result)
At the cone's middle: dc = its distance from the chuck face (75.5 mm if W1 says 10 mm gripped); d1 = station 1 from the chuck face
Y0 = Yaxis[1] + Y tilt * (dc - d1)                (machine Y of the axis at the cone's middle)
Zc = Zaxis[1] + Z tilt * (dc - d1)                (spindle-reference Z of the axis there)
ZtAnchor = machine Z of the anchor touch;  ZwAnchor = the WCS Z that M114 gave at that touch
dZ = ZwAnchor + (Zc - ZtAnchor)                   (how far the axis sits above the current WCS Z0)
```

**Record: CALCULATION** (the values, not commands)

| Quantity | Value |
| --- | --- |
| Zaxis[1], Zaxis[2] | Station 1 (X -84, far, 88 mm from the chuck face): -39.7153. Station 2 (X -144, near, 28 mm): -39.6900 (R1 5.015, R2 5.005; with R2 5.02 it is -39.7050). The stations are 60.000 mm apart, not 65. |
| Yaxis[1], Yaxis[2] | Station 1: -124.1090. Station 2: -124.2395. |
| Z tilt, Y tilt (mm per mm) | Per mm away from the chuck, over 60 mm. Z tilt -0.00042 (far end lower by 0.025 mm; -0.00017 with R2 5.02). Y tilt +0.00218 (far end 0.131 mm toward +Y). |
| Rod offset at station 1, at station 2 | Station 1: Y -0.030, Z +0.094 mm. Station 2: Y -0.073, Z +0.011 mm. |
| Rod bend between the stations | 0.094 mm over 60 mm (Y +0.043, Z +0.084). It does not change the axis result. |
| dc, d1 (mm from the chuck face) | dc 75.5 (if W1 says 10 mm gripped). d1 88; station 2 is at 28. |
| Y0 | -124.136 (-124.2395 + 0.002175 x 47.5). The current G54 Y is -124.390, so the axis is 0.254 mm toward +Y of it. |
| Zc | -39.710 (-39.6900 - 0.000421 x 47.5) |
| ZtAnchor, ZwAnchor | ZtAnchor -34.604; ZwAnchor +5.0391 (station 1, A0, first final touch) |
| dZ | -0.067 (5.0391 + (-39.710 + 34.604)): the axis is 0.067 mm below the current WCS Z0. New G54 Z = -39.6435 + 0 + (-0.067) = -39.710. New G54 Y = -124.136. |

```
SET G54 (Y and Z only. The Z1's own 4-axis setup has set G54 itself: X from the headstock face plus the 83.050 mm X0 offset, Y from the machine configuration, Z from Makera's probing routine. After this, do not use the Makera Controller's work-origin or 4th-axis set-up buttons: they set G54 again.)
Use the stored G54 from the READ G54 block, then:
G10 L2 P1 Y<Y0>                      (Y0 from the CALCULATION block, machine coordinates)
G10 L2 P1 Z<G54 Z as found + TL0 as found + dZ>     (dZ from the CALCULATION block; the stored G54 Z from the READ G54 block)
Read G54 again: X must be as found, Y = Y0, Z = as found + dZ. If X changed, restore it with G10 L2 P1 X<as found>.
M114
G10 L2 with a Z word also clears the tool length offset (seen on 21 Nov: TL0 went from 8.9659 to 0), which is why TL0 is added in the Z line. Check $# afterwards: G54 Z as set and TL0 0.0000. Do not send M500. Read G54 again at the start of the gate 5 session and send the two G10 lines again if it has gone back.
```

**Record: SET G54**

```
[21:46:08] G10 L2 P1 Y-124.136
G10 L2 P1 Z-39.710
[21:46:09] ok
```

```
VERIFY (station 1, machine X -84.000; A180 first, then A0. The expected values are for the G54 set from the calculation: Y -124.136, Z -39.710. R1 = 5.015)
$#                              (G54 must read Y -124.1360 and Z -39.7100, TL0 0.0000)
M499                            (probe active, offset 0.000)
G53 G0 Z-19.618                 (15 mm above the rod, probe clear)
G53 G0 X-84.000
G53 G0 Y-124.136
M114.4                          (A must read 180; if not, G0 A180)
A180, top touches:
G91
G0 Z-12
G90
M119                            (must print Probe: 0)
G38.2 Z-5 F60
G91
G0 Z1
G90
G38.2 Z-2 F20                  (record PRB Z)
M114                            (record the WCS Z)
G91
G0 Z3
G90
G38.2 Z-4 F20                  (record PRB Z)
M114                            (record the WCS Z)
G91
G0 Z3
G90
A180, side touches:
G53 G0 Z-19.618
G53 G0 Y-133.2
G53 G0 Z-39.79                  (tip bottom at about the rod's axis height)
M119
G38.2 Y8 F30                    (record PRB Y: Ya)
G91
G0 Y-2
G90
G53 G0 Z-19.618
G53 G0 Y-115.0
G53 G0 Z-39.79
M119
G38.2 Y-8 F30                   (record PRB Y: Yb)
G91
G0 Y2
G90
G53 G0 Z-19.618
A0:
G0 A0
G53 G0 Y-124.136
(Repeat the A180 top touches and side touches above at A0, with the side-touch height G53 G0 Z-39.62.)
Results:
Axis Z in WCS = (mean WCS Z at A0 + mean WCS Z at A180)/2 - 5.015      expected -0.005, accept -0.020 to +0.010
Axis Y in WCS = (Yc at A0 + Yc at A180)/2 + 124.136, Yc = (Ya + Yb)/2   expected +0.027, accept +0.012 to +0.042
(Not 0: G54 is set for the cone's middle, 75.5 mm from the chuck face. Station 1 is at 88 mm, so the tilt leaves Z -0.005 and Y +0.027 there. At station 2 (X -144, 28 mm) the values would be Z +0.020 and Y -0.104.)
```

**Record: VERIFY**

```
[21:55:44] G38.2 Z-5 F60
[21:55:48] [PRB:-84.000,-124.136,-34.798:1]
[21:55:48] ok

[21:55:56] G38.2 Z-2 F20
[21:55:59] [PRB:-84.000,-124.136,-34.793:1]
[21:55:59] ok

[21:56:03] M114
[21:56:03] ok C: X:34.3900 Y:-0.0003 Z:4.9172

[21:56:37] G38.2 Z-4 F20 M114 G91 G0 Z3 G90
[21:56:46] [PRB:-84.000,-124.136,-34.795:1]
[21:56:46] ok C: X:34.3900 Y:-0.0003 Z:4.9150
[21:56:46] ok

Side touches:

[21:58:55] G38.2 Y8 F30
G91
G0 Y-2
G90
[21:59:02] [PRB:-84.000,-129.967,-39.790:1]

[21:59:55] G38.2 Y-8 F30
G91
G0 Y2
G90
[22:00:02] [PRB:-84.000,-118.173,-39.790:1]
```

*A0:*

```
[22:01:46] G38.2 Z-5 F60
G91
G0 Z1
G90
[22:01:49] [PRB:-84.000,-124.136,-34.600:1]
[22:01:49] ok
[22:02:26] G38.2 Z-2 F20
M114
G91
G0 Z3
G90
G38.2 Z-4 F20
M114
G91
G0 Z3
G90
[22:02:29] [PRB:-84.000,-124.136,-34.595:1]
[22:02:29] ok C: X:34.3900 Y:-0.0003 Z:5.1147
[22:02:38] [PRB:-84.000,-124.136,-34.598:1]
[22:02:38] ok C: X:34.3900 Y:-0.0003 Z:5.1116
[22:02:38] ok

Side touches:
[22:03:35] G38.2 Y8 F30
[22:03:41] [PRB:-84.000,-130.085,-39.790:1]

[22:04:25] G38.2 Y-8 F30
[22:04:32] [PRB:-84.000,-118.234,-39.790:1]
```

**VERIFY calculation (21 Nov 2026, from the readings above; station 1, R1 = 5.015)**

- **A180:** WCS Z at the two final touches 4.9172 and 4.9150, mean 4.9161. Side touches Ya -129.967, Yb -118.173, so Yc = -124.0700.
- **A0:** WCS Z at the two final touches 5.1147 and 5.1116, mean 5.1132. Side touches Ya -130.085, Yb -118.234, so Yc = -124.1595.
- **Axis Z in WCS** = (5.1132 + 4.9161)/2 - 5.015 = **-0.0004** (expected -0.005, accept -0.020 to +0.010): pass.
- **Axis Y in WCS** = (-124.1595 - 124.0700)/2 + 124.136 = **+0.0213** (expected +0.027, accept +0.012 to +0.042): pass.
- **G54 took effect:** the WCS Z at contact equals the PRB Z + 39.710 (for example -34.793 gives 4.917) and M114 shows WCS Y -0.0003 at Y -124.136.
- **Repeat against the first measurement at station 1:** axis Z (spindle reference) -39.7103 now against -39.7153, axis Y -124.1148 against -124.1090, both within 0.006 mm. Rod offset (Y, Z) -0.045, +0.099 against -0.030, +0.094: the rod has not moved.
- **Result: pass.** G54 Y -124.136 and Z -39.710 reproduce the axis at station 1 within 0.005 mm in Z and 0.006 mm in Y of what the calculation predicted. Across the cone the Z residual is at most about 0.01 mm and the Y residual about 0.03 mm (second order on a round section). Still to do before gate 5: the B-08 safe-Z spot check with the real tool, since the reference tool is now the probe, and re-read G54 and M499 at the start of the gate 5 session.

If a probe line halts, send `M999` (or `$X`) and home before continuing. Stop at the first surprise.

The probe reads 0.001 mm; the expected accuracy is about 0.005 to 0.02 mm per touch, comparable to a 0.01 mm dial indicator, and the mean of three touches helps. It cannot measure A backlash (use the magnetic-base dial indicator on the bed for C2-07) or a continuous runout sweep.

If C2-01 to C2-06 are within the limits, gate 5 can proceed and the errors above are known to be machine, not software. If not, correct the machine (re-seat or re-tighten the chuck or the module base, clean the mating faces of chips and burrs, set the live center), repeat the failing step, and record what was changed. Do not change firmware settings.

A note on the +/-0.1 mm limit: a 0.03 mm bar runout shows up as about 0.06 mm of diameter error in the cone measurements, so these are kept low before cutting.

---
_Generated by [Claude Code](https://claude.ai/code)_