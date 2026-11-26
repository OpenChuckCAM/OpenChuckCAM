# 0695: Simultaneous finishing over sharp edges, a true point at a finial, one rule for ramp entries, and ramps on STEP parts (OP-05, OP-06, OP-11, OP-12, OP-20, OP-21, NFR-07, NFR-12, GC-07; issues #710, #779, #786, #421, #844, #826; lane PATH-QUALITY)

- **Status:** Accepted by the project owner on 25 Nov 2026, relayed by the coordinator (decision 0049), at head `23d3117` of pull request #913, with the answers below (OI-2230, closed). Proposed by lane PATH-QUALITY on 25 Nov 2026 (phase 1, planning only; no code is built until the owner answers). Only the project owner accepts this record (decision 0049). The questions are Q1 to Q7 below, each with options, a recommendation and what follows "if you answer as recommended". **Q1 answered (25 Nov 2026, the owner's answer relayed by the coordinator): option C**, the general continuous seed and a clearance test of the whole tool, taken into its own lane, SIMUL-EDGE; #710 leaves this lane's scope. **Q3 answered (25 Nov 2026, relayed): option A**, with the owner's design requirement that a ramp is there to manage the tool's load (*Q3: the owner's answer and the tool-load requirement* below). **Q4 answered (25 Nov 2026, relayed): option A**, with the tool-load point applied to its defaults. **Q2 answered (25 Nov 2026, relayed): (a) A, (b) A, (c) B**: the adaptive stepover over the tip is built now, with Finish to a point (not this lane's recommendation, A). **Q6 answered (25 Nov 2026, relayed): option A.** **Q5 answered (25 Nov 2026, relayed): #844 taken into its own lane, TOOL-LOAD**, in this milestone (neither A nor B as written). **Q7 answered (25 Nov 2026, relayed): option A**; the coordinator closed #421 with a comment naming decision 0543 D7 and D10 and PQ-2. **All seven questions are answered, and the record is accepted (25 Nov 2026, relayed; OI-2230 closed).** **Addendum (25 Nov 2026):** two later questions, Q8 (the tool-radius bound on every ramp: the owner answered A, and after seeing the measured impact said to build it, 25 Nov 2026, relayed; OI-2238) and Q9 (the settings that configure a ramp: **the owner answered B, with the default at the tool radius and A's regrouping, and the whole-ring part measured first**, 25 Nov 2026, relayed; OI-2239).
- **Date:** 25 Nov 2026.
- **Issues:** #710 (simultaneous finishing on the blisk: sharp edges and walls facing along the axis; OI-318, OI-337), #779 (a true point at a finial; an adaptive stepover for such curves), #786 (ramping across every strategy), #421 (ramps on STEP parts and a plunge into single-point passes; OI-1270, OI-1271), and the fold-in checks of #844 (a tool engaged along its whole length) and #826's OI-2198 (adaptive spacing on the stepped cylinder with a flat end).
- **Requirements:** OP-05, OP-06, OP-20 (simultaneous finishing, tool axis, hidden surfaces), OP-11 (ramping), OP-12 (gouge protection), OP-14 (linking), OP-21 (tabs), SET-09 (held regions), NFR-07, NFR-12, GC-07 and NFR-06 (determinism).
- **Would amend, if answered as recommended:** accepted [decision 0543](0543-a-ramp-entry-follows-the-toolpath.md) D2 (a spiral is ramped round the axis at its start X) and accepted [decision 0645](0645-a-swarf-ramp-follows-the-pass-along-the-face.md) item 1 (`Strategy::ramps_round_the_part`), by Q3; accepted [decision 0212](0212-roughing-direction-and-extend-past-the-stock.md) part B item 3 and accepted [decision 0546](0546-run-outs-follow-ring-and-spiral-passes.md) item 3 (a run-out keeps the height of the pass's last point), by Q2. It builds on [decision 0363](0363-simultaneous-finishing-off-axis-sections-walls-and-held-ends.md) item 2 (the wall move and the narrow seed fix), [0113](0113-toolpath-choices-rounding-cannot-flip.md) and [0133](0133-simultaneous-fitter-tests-the-quarter-points.md) (the move test), [0344](0344-finishing-stepdown-and-free-ends.md) (free-end turns), [0545](0545-blended-steps-between-rings.md) D3 (defaults that keep saved programs) and [0680](0680-the-gate-5-reference-part-is-a-stepped-cylinder.md) (where OI-2198 was found).
- **Open items:** adds OI-2230 to OI-2236 (provisional numbers; listed under *Open items* below). Adds this lane's findings to OI-318, OI-337 and OI-2198 in place. Depends on no open item for Q1, Q3, Q4 and Q6; Q2 depends on 0344's free-end turn and its cut-free warning (OI-1393 is related).

Numbering: proposed as PATH-QUALITY-1 with OI-9990 to OI-9999 (provisional); renumbered at release on 26 Nov 2026 by the coordinator to decision 0695 and OI-2230 to OI-2239 (OI-9991 to OI-9997 became OI-2230 to OI-2236, PQ-4's OI-9998 became OI-2237, OI-9990 became OI-2238 and OI-9999 became OI-2239). No project schema number is needed: new operation settings are stored in the operation's free parameter map, as decisions 0390, 0543 and 0545 did.

## For the owner

**Your answers (25 Nov 2026, relayed by the coordinator):**

| Question | Answer | What it means |
| --- | --- | --- |
| Q1 (#710) | C | the general continuous seed and a whole-tool clearance test, in its own lane, SIMUL-EDGE (OI-2235) |
| Q2 (#779) | (a) A, (b) A, (c) B | Finish to a point (new operations), to the axis with the "Part cut free" warning, and the adaptive stepover over the tip now (PQ-3, after PQ-4, within a time budget) |
| Q3 (#786) | A, with a tool-load requirement | every ramp follows its own pass; the ramp is designed to manage the tool's load (PQ-2) |
| Q4 (#786) | A | a plunge only where no ramp fits, each said with its depth; new operations get Allow single-point plunge and Steeper ramp near the axis on (PQ-2) |
| Q5 (#844) | its own lane, TOOL-LOAD | neither A nor B as written; PQ-2's load figures are shared with it |
| Q6 (OI-2198) | A | fix the three causes (PQ-4) |
| Q7 (#421) | A | #421 closed by the coordinator |

Accepted on 25 Nov 2026 (relayed; OI-2230 closed). The build is PQ-2 (independent), PQ-4, then PQ-3.

What this record asked of you, in the order the coordinator put it:

1. **Q1 (#710), answered C and moved to lane SIMUL-EDGE: the blisk's simultaneous finish cuts into its blade tips today.** It is no longer refused; it generates, and the simulation finds up to 0.94 mm cut into the blade tips with the 3 mm ball on the STEP blisk and 1.7 to 2.5 mm with the 3 mm flat end. This is a gap in gouge protection (OP-12), so it is put first. Recommended: **A**, tighten the move check so these moves are caught, break the pass there with a warning, and document the blisk's limit and its workaround.
2. **Q3 (#786), answered A with a tool-load requirement: a ramp that goes round the part cuts into it.** Face flow, pencil, simultaneous finishing on a face set and, with the steeper ramp on, helical and simultaneous on the crankshaft cut 0.26 to 0.68 mm into the part when Entry is Ramp (the same operations with Plunge do not). Recommended: **A**, every ramp follows its own pass, back and forth, as swarf's already does.
3. **Q4 (#786), answered A: needless ramp refusals.** Recommended: **A**, a stretch shorter than the shortest ramp is entered as a single point is, the steeper-ramp search is fixed, and new operations get Allow single-point plunge and Steeper ramp near the axis on (saved operations keep theirs).
4. **Q2 (#779), answered (a) A, (b) A, (c) B: the finial's tip.** Recommended: **A**, a new setting **Finish to a point** (on for new helical, axial and indexed finishing operations; saved operations keep their programs) that carries the finish past a pointed end down to the rotary axis, with the existing "part cut free" warning.
5. **Q6 (OI-2198), answered A: adaptive spacing on the stepped cylinder.** Recommended: **A**, fix the three causes found.
6. **Q5 (#844), answered: taken into its own lane, TOOL-LOAD.** Recommended: **A**, keep #844 separate in 2027.1.
7. **Q7 (#421), answered A: close it.** Recommended: **A**, close #421 on the evidence below: both its items were settled by decision 0543 and ramps generate on STEP parts.

And what to look at: the two figures below, and the evidence in [docs/archive/0695-phase-1](../archive/0695-phase-1/README.md).

## Context

All of this was reproduced on `main` at `d1674b7` (version 0.63.0) with release builds of `openchuck-cli` and `openchuck-step-worker`, on the repository's public models only: the blisk (`testdata/step/valid/blisk.step` through `testdata/examples/blisk-start`, and `testdata/stl/blisk.stl` with OI-318's setup), the baluster (`testdata/examples/baluster`), the crankshaft (`testdata/examples/crankshaft-simultaneous`) and the stepped first-cut part (`testdata/projects/first_cut`, rebuilt with the command line as `build_first_cut` builds it). The command line cannot add an operation (OI-310), so each run copied a project, wrote an operation file and ran `openchuck-cli regenerate` and then `openchuck-cli simulate` (0.1 mm cells; an overcut deeper than one cell counts as a cut into the part). Exact settings, messages and times are in the archive.

### #710: simultaneous finishing on the blisk

OI-318 recorded that simultaneous finishing refused the blisk at every setting. That is no longer so: since decision 0363 item 2 (the wall move, and the seed moving with the lean on a flat bottom), it generates at every setting tried. But the simulation shows it cutting into the part:

| Run | Generates | Simulation |
| --- | --- | --- |
| STL blisk, 3 mm ball, defaults (5 degree step, 10 degree tilt); also tilt 0, and tilt 0 with a 2 degree step | yes, 83 to 118 s | within one cell (0.058 mm at a blade tip); 2.59 mm left at the root of the shoulder |
| STL blisk, 3 mm flat end, defaults; also no hidden lean | yes, 9 to 18 s | **cuts 1.7 to 2.0 mm** into blade tips |
| STEP blisk, 3 mm ball, defaults; also exact contact off, and tilt 0 | yes, 188 to 369 s | **cuts 0.91 mm** into blade tips; 2.56 mm left at the root |
| STEP blisk, 3 mm flat end, defaults | yes, 197 s | **cuts 2.55 mm** into blade tips |

One example from the STEP ball run: one block turns A by 86 degrees and takes the ball from the top of a blade tip to its side, cutting 0.94 mm into the tip on the way.

What causes it, in `crates/openchuck-path/src/simultaneous.rs`:

1. **A move over an edge is tested only at its middle once it has been halved as far as it can be.** The quarter points and the long-move poses (decision 0133) are tested only above the deepest halving; at it, only the middle is tested, and a middle the drop finds nothing under is accepted without a test (around lines 2438 to 2541).
2. **On hidden-surface passes a flat end's cut is not seen.** The check measures the gap from the tool's profile at the point the tool rests on; a flat end rests on its corner, which reads about zero at any depth (around 2253; `simultaneous/hidden.rs` around 526).
3. **The seed follows the surface normal, which jumps at a sharp edge** (OI-337 item 1; only its narrow form was built). Smoothing falls back to the facet's normal across an edge, so nothing softens the jump.
4. **The corner where the shoulder (facing along X), the hub and the blade roots meet is out of reach** of a 3 mm tool whose axis must lie across the rotary axis: about 2.5 mm stays there, reported as unreached hidden areas. That is a limit of 4-axis access with this tool, not a defect; it is the "recorded limit" #710's *Done when* allows.
5. **The largest-tilt note reads 83 to 86 degrees at the edges**, because it compares against the next face's normal; it is the edge's jump, not the lean used.

![What the move check tests when a move crosses a blade tip, today and under option A](0695-blade-tip-check.svg)

Found on the way, not part of #710: with the axial pattern the Makera Z1 post refuses to write the STL blisk's program ("replay does not match the toolpath: … rotary counter reset is not at a reached declared boundary"), while the LinuxCNC post writes it and its replay matches (OI-2233); and the STEP blisk shows off-axis zones in four strips 0.05 mm wide (X 41.2, 43.8, 49.5 and 66.0) that look spurious (OI-2234).

### #786: ramp entries across the strategies

No strategy builds its own entry: every one goes through the shared assembly (`strategy.rs`, `assemble_limited`) to `Entry::enter` (`assemble/entry.rs`). What a ramp does depends only on how the pass starts:

| The pass starts as | The ramp | Checked against |
| --- | --- | --- |
| a line, or a partial ring | back and forth over the pass's own path | the pass's own heights: every ramp position is a pose of the pass raised along the tool axis, so it cannot cut deeper than the pass |
| a whole ring | round the ring itself | the same |
| a spiral (any pass that turns A while moving along X), when the strategy's `ramps_round_the_part` is true (every strategy but swarf) | round the rotary axis at the start X (0543 D2), its floor from a drop sampled at the A step of the tip's radius (`touch_height`) | the operation's surface: on a mesh the whole part with its held regions and tabs; on a STEP part with a face set only the faces of the set (face flow gets the whole part) |
| a single point | refused unless Allow single-point plunge is on (0543 D7) | — |

Edge following and drilling offer Plunge only; facing ramps over its own passes.

What was found:

- **Cuts into the part, all where a ramp goes round the part** (Ramp, then Plunge, on the same operation): blisk face flow on face 20, 0.379 mm against 0.046; blisk pencil, 0.542 against 0.048; blisk simultaneous on face 19, 0.684 against 0.049, and on face 20, 0.492 against 0.046; crankshaft simultaneous spiral and helical with Allow single-point plunge and Steeper ramp near the axis on, 0.423 and 0.261 against 0.044 and 0.046. On face flow and pencil the program shows passes entered by up to 4.4 turns round the part, over the blades. The spiral ramp's floor is sampled, so it cuts between samples where the part has features, and on a face set it does not see the rest of the part. Swarf, which already ramps over its own path (0645), simulates the same with Ramp as with Plunge, and every line and ring ramp was clean.
- **Needless refusals:** a single point refuses Ramp at the defaults in roughing, indexed finishing and Facing, while Lead plunges the same passes with a note; after a plunge roughing on the baluster, helical, axial, rest and simultaneous axial finishing all refuse a ramp of more than 10 turns near the axis at their defaults; a stretch shorter than the shortest ramp (ten path tolerances, 0.1 mm at the default tolerance; 0543 amendment 4) refuses even with Allow single-point plunge on (axial, indexed and helical on the blisk floor).
- **A defect in Steeper ramp near the axis:** on the baluster's simultaneous axial finish it stops at 4.09 degrees, far below the 15 degree maximum, and asks the user to turn on the setting that is already on. `Entry::steepen` assumes the turns fall in proportion to the angle, but the ring ramp adds a whole turn where the pass rises, so its four tries fall short.

### #421: ramps on STEP parts

Both items were settled by accepted decision 0543 (8 Nov 2026): D10 removed the back-and-forth corridor and its certificate, so there is no STEP refusal left (OI-1270 retired), and D7 built **Allow single-point plunge** (OI-1271 closed). Checked on `main`: Ramp entry generates on the STEP blisk for roughing in all four forms, swarf, face flow, pencil and simultaneous finishing; the STEP crankshaft's roughing along the part simulates within one cell (0.043 mm); and Allow single-point plunge acts on STEP parts (off: "pass 1 is a single point …"; on: "10 passes, from pass 1 to pass 1721 … entered by a straight plunge, as Allow single-point plunge asks"). The cuts found above are not particular to STEP (pencil on the blisk and the crankshaft use no face set), so they belong to #786.

### #779: the finial's tip

On the public baluster with the 3.175 mm ball, what the finish leaves near the finial's point at X 61 (the part is a ball of radius 5 centred at X 56):

| Finish settings | X 60 | X 60.5 | X 60.75 | X 61 | X 61.25 | X 61.75 | X 62 | X 62.5 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| The part | 3.00 | 2.18 | 1.56 | 0 | — | — | — | — |
| As saved (adaptive spacing off, Face free ends on); adaptive spacing on gives the same | 3.015 | 2.741 | 2.680 | 2.660 | 0.733 | 0.239 | 0.113 | 0.002 |
| Face free ends off | 3.015 | 2.741 | 2.680 | 2.660 | 2.680 | 2.848 | 3.015 | 3.728 |
| Extend far end, Face free ends off | 3.015 | 2.741 | 2.680 | 2.660 | 2.660 | 2.660 | 2.660 | 2.660 |

![What the finish leaves at the finial's point today, and option A](0695-finial-tip.svg)

Why: the helix spans the part's bounds (`helical.rs` around 371), so its last point is at X 61 with the ball's side still on the sphere and its tip 2.66 mm from the axis; a run-out can only rise where the drop finds nothing (`extend.rs` around 382 and 466, by 0212 part B item 3 and 0546 item 3); and decision 0344's free-end turn, at tip X 61 + R, removes the tab but its round side leaves the cusp between X 61 and 62.6. A true point needs the ball's centre to reach the axis at X 61 + R, which cuts the end free.

Adaptive spacing already follows the curve: it adds passes near the tip (2.6 times the moves between X 58 and 61; 13 s becomes 57 s), but it works only inside the helix's span, so it cannot add any past X 61. This is the owner's idea 2 on #779: the adaptive stepover exists; what is missing is a path past the end for it to work on.

### OI-2198: adaptive spacing on the stepped cylinder

Reproduced exactly (23 places, the first near X 52.900 A 5653.3, 999,940 queries, 44 s; with adaptive spacing off, 21 s and no warning; in rings, no warning). The X is not in a wrong frame: it is setup X, from the helix's start at X 17.99. Two causes:

1. **The helix and its spacing measure span the held regions.** The spiral takes its span from the surface's bounds, which include the held-region and tab solids (`composite.rs` around 43; `helical.rs` around 371); the passes there are removed only afterwards (`keep_clear`, `strategy.rs` around 579). So adaptive spacing spends its queries inside the start's held region (10 of the 23 places, X 52.8 to 54.6) and reports the first place there. Rings (`ring_span`) and simultaneous finishing (`outside_held`) do not have this problem.
2. **At each step wall a flat end's height jumps between wraps**, at X = wall + R: one wrap rides the upper step, the next the lower. Halving the spacing never removes a jump (12 places, at the walls at X 99.6 and 107.6). Decision 0297's shoulder turn cuts that material, but it is added after the helix and the measure does not count it.

The flat end's scallop measure itself is right: on the cylinders nothing is added.

### #844: the tool engaged along its whole length

On the crankshaft example, the guide's Rough then Finish: 163 of 16,751 finishing moves engage 1 mm or more of the flute, at most 11 mm on slivers at the shoulders. Finish alone, with no roughing first: 8,643 moves engage at least one tool diameter and 2,771 engage 16 mm or more, up to 21.5 mm of the 22 mm flute, with no warning (the flute check of decisions 0509 and 0522 warns only past the flute length). Measured on 0.3 mm voxels of the program, outside the product. There is no engagement measure in the product (OI-1373 records the general limit).

## Questions for the owner

*The owner's standing rule (25 Nov 2026, relayed): build time and effort are not a reason for or against an option.* The day counts and "risk" notes below are information on relative complexity only. Each recommendation rests on quality, correctness, safety, usability, and the performance of the output and of the program. Where an earlier text read as if effort decided it, that reading is withdrawn. The answers already given stand.

### Q1 (#710): what the simultaneous finish does over sharp edges

**The owner's answer (25 Nov 2026, relayed by the coordinator): C**, built in its own lane, SIMUL-EDGE, not in this one. That lane owns #710, OI-2231 and OI-2235, and decides its own design and record; the options below are kept as they were put.

- **A. Catch the move and break the pass (recommended).** Test the quarter points and long-move poses at every halving; count a middle with nothing under it as a failure; measure a flat end's cut on hidden-surface passes from the whole tool, not the point it rests on; correct the largest-tilt note. Where a move still cuts after the last halving, the pass is broken there, the tool lifts round the edge as linking does (OP-14), and a warning says where material is left, as an unreached area marked on the part. The user guide's simultaneous page states the blisk's limit and the workaround: a region on the blades and hub, the shoulder and the root corners by another strategy or a smaller tool, rest machining after. About 3 to 4 days; low risk to other parts (the lobed and cone goldens have no sharp edges; the golden suite confirms). No public API change.
- **B. A, and roll round sharp convex edges.** Find an edge where the drive normals jump, and add poses that pivot the tool about it (contact held on the edge while A turns), so the tip and rim are finished instead of left. About 6 to 9 days; medium risk (a mesh's facet boundaries may read as edges); the STEP crankshaft's and blisk's programs change. No public API change.
- **C. The general continuous seed of OI-337 item 1, and a clearance test of the whole tool at every test point.** About 2 to 3 weeks; every simultaneous program changes (the lobed golden, the end-to-end goldens, the guide's figures); a clearance query added to the public `SurfaceSource` trait is a breaking change (decision 0048: a workspace minor bump and changelog).

*If you answer as recommended:* the blisk's simultaneous finish no longer cuts into its blade tips; it leaves the tips and the root corners uncut, says so in a warning with where, and the guide says how to finish them. #710 closes on its *Done when*: the limit is recorded with a clear message and a workaround. B stays open as OI-2235 for a later milestone. OI-318 and OI-337 close (OI-337's item 1 general form moves to OI-2235).

### Q2 (#779): a true point at a pointed end

Sub-question (a), the pattern:

- **A. A setting "Finish to a point" (recommended).** On helical, axial and indexed finishing: where the part ends in a point within the stock, the finish carries on past the end with the tool following the drop down until its tip reaches the rotary axis (ball centre on the axis at the point's X + R). Stored in the operation's parameter map. In 0545 D3's pattern: an operation saved without it keeps its program (off); a new operation gets it on. It still rises over tabs (OP-21) and keeps out of held regions (SET-09). About 4 to 6 days; medium risk (the parted end, tabs). Adds a public parameter name to `openchuck-path` (additive, no minor bump).
- **B. Only when Extend past the stock is on.** The run-out follows the drop down instead of holding its height (amending 0212 part B item 3 and 0546 item 3). About 3 days; but Extend is off by default, so the default baluster keeps its nub.
- **C. A note only.** A generation note says what is left past a pointed end ("about 2.7 mm stays at the point at X 61; finish to a point is not done"). About 1 day; nothing cut changes.

**The owner's answer to (a) (25 Nov 2026, relayed by the coordinator): A.** The setting **Finish to a point**, on for new helical, axial and indexed finishing operations and off for saved ones, so saved projects keep their programs; it still rises over tabs (OP-21) and keeps out of held regions (SET-09).

Sub-question (b), beyond the tool's reach, or where the point is not on the axis:

- **A. Go to the axis with the existing "Part cut free" warning (recommended)** of decision 0344, as Face free ends already does at a free end.
- **B. Hold the reached height** and say what is left in a note.

Sub-question (c), the adaptive stepover over the tip (your idea 2):

- **A. Not now (recommended):** adaptive spacing already follows the curve up to the point; extending it over the new path past the point is recorded as OI-2236. Its query budget is already met on the first-cut part (Q6), so it needs Q6 first.
- **B. Now, with Q2 (a):** about 4 more days; medium risk to generation time. **The owner's answer.**

*If you answer as recommended:* a new finishing operation on the baluster cuts the finial to a point and warns that the end is cut free; saved projects and every golden keep their programs; the guide's Extend and Face free ends lines are corrected. Accepted decisions 0212 part B item 3 and 0546 item 3 are amended for this setting only.

**The owner's answers (25 Nov 2026, relayed by the coordinator): (a) A, (b) A, (c) B.** PQ-3 builds Finish to a point, goes to the axis past the tool's reach with the existing "Part cut free" warning, and extends adaptive spacing over the new path past the point. As the record's requirement for PQ-3:

- **Adaptive spacing over the tip.** Where Finish to a point carries the finish past a pointed end, adaptive spacing measures the material left between passes there as it does inside the part's span, and adds passes where the curve needs them, with the same allowed excess and smallest spacing. It works on the span that PQ-4 sets (held regions left out), so PQ-3 is built on PQ-4.
- **The time budget, and how it is measured.** On the public baluster's Finish with adaptive spacing on (57 s on `main` `d1674b7` in this lane's container, against 13 s with it off), adding Finish to a point with adaptive spacing over the tip may add at most **25 %** to the operation's generation time, and the geometry queries spent past the part's end at most **10 %** of the operation's query limit (today 1,000,000). Measured as the median of five runs of a release build at all threads, before and after, on the same machine and in the same session, with `openchuck-cli regenerate` timed per operation; the query count read from the generation's own count (the figure adaptive spacing already reports). The same pair is measured on the stepped first-cut part with adaptive spacing on, after PQ-4. The pull request records the figures in this record's Verification; a result over the budget stops for the owner's decision rather than being merged.
- **When the budget is met during generation.** As today: the path is kept as far as it is confirmed, and the partial-path warning names the place and the limit (decision 0536's form), so nothing is cut that was not measured.

### Q3 (#786): what a ramp follows

- **A. Every ramp follows its own pass (recommended).** A pass that starts as a spiral is ramped back and forth over its own path, as swarf's already is (0645 item 2) and as lines and partial rings are (0543 D3): A reverses during the entry only, and every ramp pose is a pose of the pass raised along the tool axis, so it cuts no deeper than the pass, against the whole part whatever the face set. No ramp goes round the part. `Strategy::ramps_round_the_part` is removed (a breaking change of `openchuck-path`'s public trait: workspace minor bump and changelog, decision 0048). About 3 days; low risk. No golden changes (no reference project uses Ramp entry); saved operations with Ramp entry whose passes start as spirals get a different entry.
- **B. Round the part only where the pass cuts all round, checked against the whole part.** `ramps_round_the_part` true only for roughing, helical, rest and simultaneous without a face set; the spiral ramp, Lead and Helix checked against the whole part (the larger #572 fix) at the tool's largest radius, and refused on a hit. About 6 to 8 days; medium risk (still a sampled check; more refusals). Adds a whole-part surface to `StrategyInput` (breaking, minor bump).
- **C. Each strategy declares its ramp shapes, with a fallback setting.** The form greys out the shapes a strategy does not allow with the reason; B's whole-part check; a per-operation fallback (refuse, plunge or helix) for single points, short stretches and passes over 10 turns. About 2 to 3 weeks; medium to high risk (form, schema, guide and many messages).

*If you answer as recommended:* none of the cuts found above remains; every strategy's ramp behaves as its neighbours'; the guide's Entry text says a ramp goes back and forth over the start of the pass. Accepted decisions 0543 D2 and 0645 item 1 are amended. Lead and Helix stay checked as today; their whole-part check stays in the #572 family (OI-2232 notes it).

#### Q3: the owner's answer and the tool-load requirement

**The owner's answer (25 Nov 2026, relayed by the coordinator): A**, every ramp follows its own pass and `Strategy::ramps_round_the_part` is removed, **with a design requirement: the purpose of a ramp is to manage the tool's load, so PQ-2 is designed for that, not only for gouge safety.** As the record's requirement for PQ-2:

1. **What sets the load on a ramp.** A ramp is fed at the plunge feed, whatever its shape (`assemble/entry.rs`; decision 0548 caps a steep descent inside a pass at the same feed), so the feed does not change between shapes. Three things set how hard the tool works:
   - **the angle**, measured along the tip's real path over the part (decision 0543 D4: at the tip's radius where A turns, and along X where the pass moves along X). It sets the depth taken per millimetre of travel, `tan θ`;
   - **the total length**, `h / tan θ` for a descent `h`, the same for every shape at the same angle;
   - **the depth any point sees in one traverse.** On a ring or spiral round the axis each turn cuts `2πr · tan θ` below the turn before, the same everywhere round. On a back-and-forth ramp over a stretch of length `S`, the depth between one leg and the next is zero at the turn-round and `2S · tan θ` at the far end, so the largest step is `2S · tan θ`, and the load is not even along the leg.
2. **What changes for a pass that starts as a spiral, and how it is bounded.** The angle and the total length stay the same as today's spiral ramp at the same settings, so the depth per millimetre does not get steeper and the ramp does not get shorter. The new risk is the third point: on a stretch longer than half the circumference (`S > πr`), a back-and-forth leg takes a deeper step at its far end than one turn of today's spiral would. PQ-2 bounds it: **the leg is shortened so that `2S · tan θ` is no more than one turn of the spiral ramp at the pass's start radius (`2πr · tan θ`), and never more than the tool's radius.** The extra legs cost time, not load. On a short stretch, with legs shorter than the tool's diameter, the tool's centre goes down into its own groove, nearly a plunge. Decision 0543 already lists such a pass in `straight_plunges` and refuses it for a tool that cannot plunge; that stays.
3. **The two exceptions stay as accepted, and are said.** Where the pass itself drops more steeply than the angle just before its first point, the ramp follows that drop (0543 D4). **Steeper ramp near the axis** raises the angle only up to its maximum (0543 point 3). Both are counted in path notes today; PQ-2 keeps them and adds no other way to steepen.
4. **What the user sees.** The ramp's path note gives, per operation, the angle used, the number of legs, and the deepest step per traverse, so the load can be read before the program is run (NFR-12: a note, not a statement that the load is safe for the tool).

**Verification for PQ-2** (tests named by requirement, OP-11 and OP-12, on synthetic parts in `openchuck-path` and on the public blisk and crankshaft in `openchuck-app`):

- the ramp's slope along the tip's real path never exceeds the Ramp angle, except where one of the two exceptions of point 3 applies, and then exactly as far as they allow and with their notes;
- the ramp's programmed length is at least `h / tan θ` (it is never shortened to fit), and every ramp pose is a pose of the pass raised along the tool axis, within the pass's own stretch up to its first rapid;
- the deepest step per traverse is at most one turn of the spiral ramp at the same radius and at most the tool radius;
- for the same pass and settings, against today's spiral ramp: the same angle, the same or a longer length, and no deeper step per traverse;
- the simulation of the blisk's face flow, pencil and simultaneous on a face set, and of the crankshaft's helical and simultaneous with the steeper ramp, finds no overcut deeper than one cell with Ramp (the cases of OI-2232), and the material removed per millimetre of ramp is no more than with today's spiral at the same settings.

**How this ties to Q5 (#844).** Both are about the tool's load: a ramp manages it while the tool enters, #844 while it cuts the pass. The owner's point makes the two share their measures. PQ-2 defines the ramp's load figures (angle, depth per traverse) so that #844's engagement measure can use the same terms. It does not change the recommendation to keep #844 separate (Q5 A): the measure #844 needs is the engaged length of flute and the radial width against the stock as it is at each move, which comes from the simulation's stock, not from the path's own geometry as a ramp's does. Its warnings and limits reach every strategy and need requirement wording first. If the owner wants the two together, Q5 B (a warning on a first finishing cut deeper than one tool diameter) is the part that could ride with PQ-2 at about 1 to 2 days.

### Q4 (#786): the ramp's needless refusals

- **A. Fix and default (recommended).** (i) A stretch shorter than the shortest ramp is treated as a single point, so Allow single-point plunge covers it; (ii) the steeper-ramp search tries angles up to the maximum by the turns it actually plans, not a proportion (a defect, fixed under any answer); (iii) new operations get Allow single-point plunge and Steeper ramp near the axis on, in 0545 D3's pattern (saved operations keep their settings and programs). The single-point plunge still needs a centre-cutting tool, and is refused naming the tool otherwise (0543 D7). About 2 days; low risk.
- **B. (i) and (ii) only**, defaults unchanged. About 1.5 days.
- **C. (ii) only.**

*If you answer as recommended:* at the defaults, the finishing operations after a roughing on the baluster ramp instead of refusing; roughing and Facing plunge single points with a note, as Lead already does.

**The owner's answer (25 Nov 2026, relayed by the coordinator): A**, with the tool-load requirement of Q3 applied to it. As the record's requirement for PQ-2:

- **A plunge only where there is no room to ramp.** The new defaults trade a refusal for a plunge, never a ramp for a plunge: a pass is plunged only when it is a single point, or a stretch shorter than the shortest ramp (ten path tolerances, never above the tool radius; 0543 amendment 4), or a ring or spiral ramp that would still need more than 10 turns at the Maximum ramp angle. Wherever a ramp fits within the bounds of Q3 (the angle, the length, the step per traverse), the pass is ramped.
- **Every plunge fallback is said.** A path note (information, decision 0536) names the passes plunged and, for each kind, why (a single point, a stretch shorter than the shortest ramp, too many turns near the axis) and its depth: a plunge takes the whole descent in one move, so the note gives that depth, the "depth per traverse" of a plunge, beside the deepest step per traverse of the ramped passes (Q3 point 4).
- **Unchanged:** the plunge needs a centre-cutting tool and is refused naming the tool otherwise (0543 D7); an operation saved without the settings keeps them off and keeps its program; only a new operation gets Allow single-point plunge and Steeper ramp near the axis on (0545 D3's pattern).
- **Verification** (added to PQ-2's): a pass with room to ramp is never plunged at the new defaults; each plunged pass appears in the note with its reason and depth; the steeper-ramp search reaches the Maximum ramp angle before it refuses, and its refusal no longer names a setting that is already on.

### Q5 (#844): fold it into this lane?

- **A. Keep #844 separate, in 2027.1 (recommended).** It is a different mechanism (the stock's state and the order of operations, not the edge geometry), it touches different code (simulation, the flute check and notices across all strategies), and it needs its own requirement wording, which the issue asks the planning lane to propose. About 2 to 3 weeks when it is done.
- **B. Fold in a warning only:** when a finishing operation is the first to cut in a setup and its deepest cut exceeds one tool diameter, warn with the figure. About 1 to 2 days; a notice only, no program changes. Needs requirement wording agreed first.
- **C. Fold in the whole of #844.**

*If you answer as recommended:* #844 stays in 2027.1; this record's measurement on the crankshaft is noted on the issue. The owner's tool-load requirement on Q3 ties the two (see *Q3: the owner's answer and the tool-load requirement*): PQ-2's load figures use terms #844 can reuse; it does not change this recommendation.

**The owner's answer (25 Nov 2026, relayed by the coordinator): #844 is taken into its own lane, TOOL-LOAD**, planned first, in this milestone rather than 2027.1. It is neither A nor B as written: not part of this lane, and not deferred. This lane's measurement on the crankshaft (*Context*, #844, and the archive) is kept for TOOL-LOAD, and PQ-2 defines its ramp load figures (the angle along the tip's real path, the depth per traverse, a plunge's depth) so that TOOL-LOAD can reuse them; the two lanes agree the terms before PQ-2's notes are written.

### Q6 (OI-2198): adaptive spacing on the stepped cylinder

- **A. Fix the three causes (recommended).** Leave out the spacing measure's stations inside held regions on the spiral, keeping the helix's phase, so programs with adaptive spacing off are unchanged; count the shoulder turns of decision 0297 as passes of the measure (or skip stations within the tool's radius of a turn); name the frame in the message ("setup X") and report no place inside a held region. About 3 to 4 days; low to medium risk. No golden changes (every golden has adaptive spacing off). The first-cut project can then take today's defaults.
- **B. Only the held regions and the message.** About 1 day; the step walls still exhaust the budget.
- **C. Record it as intended** and correct only the message.

*If you answer as recommended:* OI-2198 closes; a new helical finish on a stepped part with a flat end finishes without a partial-path warning.

**The owner's answer (25 Nov 2026, relayed by the coordinator): A.** PQ-4 fixes the three causes: the held regions are left out of the spiral's spacing measure (the helix's phase kept, so programs with adaptive spacing off are unchanged), decision 0297's shoulder turns are counted by the measure, and the message names setup X and reports no place inside a held region. PQ-4 comes before PQ-3 (Q2 (c) builds on it).

### Q7 (#421): close it?

- **A. Close #421 (recommended)** on the evidence above: both its items were settled by decision 0543, and Ramp entry and Allow single-point plunge work on STEP parts. What it found that remains is in #786 (Q3, Q4).
- **B. Keep it open** until Q3 and Q4 are built.

**The owner's answer (25 Nov 2026, relayed by the coordinator): A.** The coordinator closed #421 with a comment naming decision 0543 D7 and D10 and PQ-2.

## Addendum (25 Nov 2026): the tool-radius bound on every ramp, and the ramp settings

Two questions came to the owner after PQ-2 opened (#930). They are relayed by the coordinator, and no code changes before the owner answers.

### Q8: the depth per traverse on lines and partial rings

PQ-2 bounds the depth per traverse only on passes that start as a spiral, as Q3 point 2 says. On a line or a partial ring the legs are as long as the path allows. On the public crankshaft's helical finish with the steeper ramp, one line pass therefore steps 3.068 mm per traverse, against a tool radius of 1.59 mm.

**The owner's answer (25 Nov 2026, relayed): A.** The tool-radius bound applies to every back-and-forth ramp: a leg is at most `R / (2 · tan θ)`, so the depth per traverse is never more than the tool radius. **Measure the impact first.** The numbers are below. **After seeing them, the owner said to build it (25 Nov 2026, relayed)**: it is built in PQ-2 (#930), test first (a 50 mm line ramp with a 1 mm tool radius stepped 5.000 mm per traverse in two legs; it now takes ten, at most 1.000 mm).

**Measured (25 Nov 2026).** The change was made in a throwaway worktree of PQ-2's head `8db9f4cd`, on a Linux container with release builds: on a line or partial ring the longest leg becomes `0.5 · R / tan θ` in place of no bound. Nothing was committed.

- **Tests and goldens:** all tests pass, and no golden or stored program changes.
  - `openchuck-path`: 135 unit, 878 integration and 1 doc test.
  - `openchuck-app`: 293 unit and 618 integration tests, including the golden audit, the reference jobs and the ramp and lead examples, with the two STEP ramp tests run with the kernel.
  - None of the 49 goldens or the reference projects uses Entry Ramp.
- **Shipped examples:** none uses Entry Ramp, so no example's program changes.

  For a sense of scale, the public baluster was also run with Ramp forced on, as the `ramp_example` test sets it up. Only Rough changes:
  - **Rings at 3°:** 7 of its 49 line and arc ramps change, with 18 more legs. The ramp grows from 3698.7 to 3819.5 mm, which adds 35.0 s, and the program grows from 4448.0 to 4483.1 s (+0.8%). The deepest depth per traverse on these ramps falls from 4.127 to 1.587 mm.
  - **Rings at 6°:** the program grows by 16.7 s (+0.4%).
  - **Helix:** unchanged.
  - **Overcut:** unchanged.
- **The OI-2232 cases.** Four are unchanged: blisk face flow, blisk pencil, blisk simultaneous on face 20 and crankshaft simultaneous. Their ramps already keep within the radius. Two change:

  | Case | Ramps changed | Deepest depth per traverse | Program time | Deepest overcut |
  | --- | --- | --- | --- | --- |
  | Blisk simultaneous, face 19 (ball R 1.5) | 3 of 24 arcs | 2.617 → 1.500 mm (pass 130) | 16239.2 → 16289.0 s (+0.3%) | 0.0485 mm, the same |
  | Crankshaft helical, steeper ramp (ball R 1.5875) | 4 of 12 arcs | **3.068 → 1.587 mm** (pass 1302) | 192617.8 → 192577.9 s (−0.02%) | 0.0455 mm, the same |

  - **The crankshaft's pass 1302** is a partial ring at a shoulder. It goes from 12 legs of 29.27 mm to 20 legs of 15.15 mm, and its ramp shortens from 351.2 to 302.9 mm of tip travel, because the old legs ran the whole pass and the count was rounded up.
  - **Cut section** at the deepest leg end. This is an estimate, from the ball's engaged cross-section, because the simulation reports removal per operation, not per move:
    - crankshaft pass 1302: about 8.7 → 4.0 mm²;
    - blisk face 19 pass 130: about 6.9 → 3.5 mm²;
    - the baluster's forced rough (flat end, depth × diameter): about 13.1 → 5.0 mm².
  - **Material removed:** the same per operation in every case.
- **Refusals and plunges:** no operation now refuses (too many legs or moves), and none newly falls back to a plunge.
- **Raw output:** the logs and helper scripts were kept only in the session's scratch space. They were noise for the archive and were not committed.

**One thing Q8 does not cover.** A whole ring still ramps round itself (Q3), and each turn takes `2πr · tan θ`, which grows with the radius. On the baluster's forced rough at 6°, a ring ramp reaches a depth per traverse of 4.700 mm against the 1.5875 mm radius, and Q8's bound does not touch it. Bounding it the same way would mean ramping a large ring back and forth, or limiting the angle by the radius. That is left to Q9's answer (OI-2239) rather than built silently. *(Answered: Q9 part 3, A (V1), below; built in PQ-2b.)*

### Q9: the settings that configure a ramp

The owner asks whether the settings that configure ramping should change, now that a ramp is there to manage the tool's load (Q3).

#### Today's settings

These are the settings in the Entry, links and retract section that act on an entry. They are all stored in the operation's parameter map (decisions 0390, 0543 and 0545).

| Setting | Default | What it controls | Its effect on the tool's load |
| --- | --- | --- | --- |
| **Entry** | Plunge | How the tool enters a pass: Plunge, Lead, Ramp or Helix | Plunge takes the whole descent in one move on the tool's end, the heaviest load. Ramp and Helix spread it over a length. Lead enters from the side at the pass's depth. |
| **Ramp angle** | 3° | The slope of a ramp or helix along the tip's real path | Sets the depth taken per mm of travel (`tan θ`), and so the cut section at the start of each leg. It is the same value for Ramp and Helix. |
| **Plunge feed** | from the tool | The feed of every plunge, ramp and helix | A steeper ramp at the same feed loads the tool more. There is no separate ramp feed. |
| **Allow single-point plunge** | off (on for new operations after PQ-2) | Plunges a pass that has no room for a ramp: a single point, a stretch shorter than the shortest ramp, or a ring or spiral needing more than 10 turns even at the Maximum ramp angle | Turns a refusal into the heaviest entry, a straight plunge of the whole descent. PQ-2's note names each plunge with its depth. |
| **Steeper ramp near the axis** | off (on for new operations after PQ-2) | Raises the angle of a ring or spiral ramp that would need more than 10 turns, just enough to fit in 10 | Raises the depth per mm near the axis, where the tool is close to the centre and the path is short. |
| **Maximum ramp angle** | 15° | The steepest angle the steeper ramp may use | The upper limit of that exception. |
| **Plunge to remove face nubs** and **Face nub diameter** | off, and the tool's diameter | Roughing in rings only: plunges at the axis to remove a nub instead of ramping round ever smaller rings | Another plunge, under its own setting. |
| **Helix diameter** | 0.75 of the tool | The circle of a helix entry | The smaller the circle, the closer the helix comes to a plunge. |
| **Lead radius** | the tool's radius | The arcs of a lead entry | Does not change the depth. |
| **Back-and-forth ramp** | off | Nothing since decision 0543 | Nothing. It is shown only where a saved operation has it on, marked inactive. |

The program also holds fixed bounds that no setting shows:
- at most 10 turns, or 10 turns' length, for a ring or spiral ramp;
- a shortest ramp of ten path tolerances, never more than the tool radius;
- legs at least the tool radius where the path allows it;
- after PQ-2, a depth per traverse of at most the tool radius (and, with Q8 A, on every ramp).

#### Where they confuse or conflict

1. **The angle is the only load setting, and it is indirect.** The same 3° gives a different cut section with a different tool or a different leg length. The depth per traverse, the figure the owner named in Q3, has no setting: the program fixes it at the tool radius.
2. **Three settings make a plunge, under three names.** Entry Plunge, Allow single-point plunge and Plunge to remove face nubs each plunge in different places. "Single-point" no longer describes what the setting covers, since PQ-2 added a short stretch and a ring with too many turns.
3. **Two settings for one exception that goes against the load goal.** Steeper ramp near the axis and Maximum ramp angle make the ramp steeper, which is more load per mm, so as to avoid either a plunge or a refusal. That trade only makes sense in comparison with a plunge, but the settings do not say so.
4. **One angle for Ramp and Helix.** A helix's depth per turn is `π · d_helix · tan θ`, so the same angle loads the tool differently on a helix than on a back-and-forth ramp.
5. **The overlap with lane TOOL-LOAD.** TOOL-LOAD-1 (#927) proposes **Limit tool load**: a cut-section limit for the cutting moves, measured in the simulation. Its Q6 A measures entry moves like any other and names the Ramp angle setting when the heaviest stretch is in an entry. A second load setting on the ramp would give the user two limits for one tool.

#### Every downside of each option

Under the owner's standing rule, the effort figures in this list are information only, not reasons.

*Asked by the owner (25 Nov 2026, relayed): "is the only disadvantage the extra time to build?"* No. Each downside below is marked **[saved programs]** where it could change or risk a program saved before the change, and **[surface]** where it only adds to the form, the guide or the tests.

**A. Keep the angle, and regroup, relabel and explain.**
- **The whole ring stays unbounded.** A ring ramp takes `2πr · tan θ` per turn, which can exceed the tool radius on a large ring (4.700 mm on the baluster's forced rough at 6°). Under A this is reported in the note and warned by TOOL-LOAD's TL-1, but it is not held. **[surface]** No program changes, but the load stays where it is.
- **The load is still set indirectly.** The user sets an angle, not a load, and must read the note to see the depth per traverse. **[surface]**
- **Users must find a moved setting.** The relabel ("Plunge where no ramp fits") and the move to Advanced change where users find a setting they know, and the guide's screenshots and text need redoing. **[surface]**
- **No risk to saved programs.** The parameter names do not change.

**B. A, and a "Largest depth per traverse" setting.**
- **It overlaps with TOOL-LOAD's limit.** Two settings would bound related things: this one, in mm, on the ramp, and TOOL-LOAD's Limit tool load, in mm², on everything. Users have to understand which one applies. The function shared with TOOL-LOAD takes the smaller, but the form shows both. **[surface]**
- **The default changes saved programs.** It must have a default. The tool radius is what Q8 already builds, so a saved operation changes no more than Q8 already changes it, and no golden or example changes, as measured. A different default (for example, a fraction of the radius) would change saved programs with Entry Ramp. **[saved programs, only if the default differs from the radius]**
- **Whole rings, if B holds them.** A ring ramp deeper than the setting would have to be ramped back and forth (more legs, a longer ramp) or have its angle lowered (a longer ramp). That changes the programs and times of saved operations whose rings are large and steep. On the measured baluster case, the rough's ramps would lengthen and the time would grow. Not measured: if the owner leans to B, it is measured first, as Q8 was. **[saved programs, where a saved ring ramp is deeper than the setting]**
- **It interacts with the steeper ramp.** Steeper ramp near the axis and Maximum ramp angle raise the angle, and so the depth per turn. Under B the steeper search must also keep within the setting. That makes refusals and plunges near the axis more likely, and so more plunges with Allow single-point plunge on. Users would then see three settings meet in one place. **[saved programs, near the axis only, where the steeper ramp is on; surface]**
- **It adds tests and guide pages:**
  - a parameter;
  - its range and its relation to the radius;
  - tests for each strategy's ramp shape and the steeper search;
  - a guide paragraph;
  - UI specification text.

  About 2 days more than A. **[surface]**
- **One more number** for every operation with Entry Ramp. Its good value is almost always the default. **[surface]**

**C. A load target, with the angle following.**
- **It cannot be built yet.** It needs the stock's depth before the pass (TOOL-LOAD's TL-2 `StockModel` query) and waits for lanes SIMUL-EDGE and TOOL-LOAD.
- **It duplicates TOOL-LOAD's limit.** It is either a second copy of Limit tool load or a dependency on it. **[surface]**
- **The angle would no longer be what the user set.** It would vary from pass to pass with the stock. That is harder to predict and to check before running (NFR-12).
- **Every saved Entry Ramp program would change**, unless the setting is off for saved operations. **[saved programs, unless off for saved operations]**
- **Highest risk:** the ramp's result depends on the simulated stock being right. Weeks of work.

**D. A preset (Gentle, Normal or Steep).**
- **It hides the figures** the owner wants visible: the preset sets the angle and the maximum without saying what load follows. **[surface]**
- **A third way to set the same thing.** The preset sits alongside the Advanced settings, so users have to know which one wins. **[surface]**
- **A saved operation needs a rule.** It would have to read as "Custom", so its program stays as it is. **[saved programs, only if the rule is wrong]**
- **More text to keep in step.** The preset labels, the tooltips and the guide all need keeping consistent. **[surface]**

**In short:** A and D only add form and guide surface. B risks saved programs only if its default differs from the radius, or if it is made to hold whole rings, and then the change should be measured first. C risks every saved Entry Ramp program unless it is off for them, and it cannot be built before TL-2.

#### Options

- **A. Keep the angle-based settings, and make them clearer (recommended).**
  - Group the settings: a basic group (Entry, Ramp angle, and "Plunge where no ramp fits", the new label of Allow single-point plunge), and an Advanced group (Steeper ramp near the axis, Maximum ramp angle, Plunge to remove face nubs, Face nub diameter, Helix diameter, Lead radius).
  - Each tooltip says what the setting does to the load in TOOL-LOAD's terms (Ramp angle, Depth per traverse, Cut section).
  - The fixed bounds are stated in the guide, and PQ-2's note shows the figures for each operation.
  - The labels change but the parameter names do not, so saved projects open unchanged and keep their programs. There is no schema change and no change to a golden.
  - About 1 day, in PQ-2 (#930) or straight after it. Low risk.
- **B. A, and a Depth per traverse setting.** "Largest depth per traverse", in mm, with a default of the tool radius (0 means the tool radius):
  - the ramp shortens its legs to keep within it;
  - the steeper ramp may raise the angle only while it keeps within it;
  - a plunge is named with its depth against it.

  The setting goes in the parameter map, so there is no schema number. A saved operation reads the default, so its program changes only where Q8 A already changes it. About 2 days more, with tests. Low to medium risk. It gives the owner's Q3 figure a setting of its own. Its good value will almost always be the default, though, and it is one more number for the user.
- **C. Configure by a load target, and let the angle follow.** The ramp takes its angle from a cut-section limit, TOOL-LOAD's Limit tool load, and the stock it enters. That needs the depth of the stock over the tool before the pass, which is TOOL-LOAD's TL-2 `StockModel` query. It would duplicate or depend on TL-2, which follows lane SIMUL-EDGE. Several weeks of work, medium to high risk, and not buildable before TL-2.
- **D. A simple preset, with the settings behind it.** Entry "Ramp: Gentle, Normal or Steep" sets the angle and the maximum. The settings stay in Advanced. It could be added to A or B later. On its own it hides the figures the owner wants to see.

*If you answer as recommended (A):* PQ-2 relabels and regroups the entry settings and rewrites their tooltips in TOOL-LOAD's terms. The guide's Entry page lists the fixed bounds. Nothing is stored differently. The load limit stays a single setting, TOOL-LOAD's Limit tool load. When TL-2 is built, it can decide whether Limit tool load also caps the ramp's cut section in deep stock. Option B stays open if the owner wants the depth per traverse as a setting of its own. The large-ring depth per traverse (above) is part of this question: under A it is reported in the note and measured by TOOL-LOAD's TL-1; under B it would be held to the setting by ramping such a ring back and forth.

**The owner's answer (25 Nov 2026, relayed): B, with the default at the tool radius, and A's regrouping included.**
1. A's regrouping:
   - basic and Advanced groups;
   - "Allow single-point plunge" relabelled "Plunge where no ramp fits" (the stored name is unchanged);
   - tooltips in TOOL-LOAD's terms;
   - the fixed bounds stated in the guide.
2. A new setting, **Largest depth per traverse** (mm), with 0 meaning the tool radius. At its default nothing changes beyond Q8, so saved operations keep their programs.
3. **Whole rings are measured before anything is built for them.** The measure covers back and forth over the ring (V1) and a shallower angle round it (V2), and their effect on saved ramp programs, the steeper ramp near the axis, refusals and plunges, length and time. The numbers go to the owner, and the ring-holding behaviour is built only once the owner confirms them (OI-2239 stays open for that part).

#### Whole rings: the measured impact (Q9, part 3)

**Measured on 26 Nov 2026** with a throwaway prototype on PQ-2's Q8 head `ec7d7400`, in a Linux container, using release builds. Nothing was committed. A ring counts as over the bound when its depth per traverse, `min(2πr · tan θ, rise)`, exceeds the bound (the tool radius by default). Two ways to hold such a ring were measured:

- **V1, back and forth over the ring.** The ring is first planned as today, including any steepening. If it is over the bound, it is ramped back and forth over its own points at the same angle, with legs of at most `R / (2 tan θ)`, as a partial ring is.
- **V2, a shallower angle round the ring.** The angle is capped at `R / (2πr)`, so each turn takes at most the bound. Steeper ramp near the axis may only steepen up to that cap.

**Results:**
- **No golden, stored program or saved operation changes.** None of the 17 operation files in the repository's examples and test data uses Entry Ramp, and the app suite (goldens, posting, `ramp_example`) passes under both variants.
- **Two path tests change under both variants:**
  - the descent on the axis (the axis case below; the prototype capped it, which the build does not);
  - the steeper-ramp test's ring of radius 0.75 mm with a 0.5 mm tool radius. V1 takes 66 legs there; V2 refuses it at 32.8 turns.

| Case | Before | V1 | V2 |
| --- | --- | --- | --- |
| Baluster, Ramp forced on, 6° (flat end, R 1.5875) | deepest 4.700 mm; program 3332.2 s | 1.587 mm; 12 of 16 rings changed, 4 to 8 legs each, 46 more A reversals; **+0.78%** | 1.588 mm; angles down to 1.31°; ring ramps 644 → 1505 mm; **+5.80%** |
| Baluster, 3° | 2.987 mm; 3837.0 s | 1.587 mm; 9 rings; **+0.92%** | 1.588 mm; **+2.48%** |
| Crankshaft example, rotary roughing in rings, 3° (2 of 450 passes start as whole rings) | 2.202 mm; 30961.8 s | 1.587 mm; **+0.02%** | 1.165 to 1.587 mm; **+0.17%** |
| The six blisk and crankshaft cases of OI-2232 | no whole-ring starts | unchanged | unchanged |
| Stress case: the baluster's finish on a 44 mm stock, 3°, Steeper ramp near the axis on | 5.598 mm; plunges pass 23; 4608.3 s | 1.5875 mm; plunges 23 and 24 (24 is the axis case, capped in the prototype); **−2.6%** (it keeps the steepened angle on its legs) | 1.5875 mm; **plunges 18 to 24, up to about 21 mm deep**; **+4.4%** |

- **Overcut:** the deepest overcut is the same in every case and variant (0.0427 to 0.0485 mm, within one 0.1 mm cell).
- **Cut section** at the deepest step, estimated from the engaged tool's section:
  - flat end: 14.9 → 5.0 mm² (baluster 6°) and 9.5 → 5.0 mm² (3°);
  - ball, stress case: 16.7 → 4.0 mm².

**What the numbers show:**
- V1 holds every ring to the bound with no new refusal or plunge away from the axis, keeps the Ramp angle, and puts rings under the same rule as partial rings and spirals. Its cost in the output is that A reverses during the entry on rings that went one way round.
- V2 ignores the Ramp angle on large rings (the angle comes from the radius alone).
- V2 can make a ring need `rise / R` turns, so any ring entry deeper than 10 tool radii is refused, or plunged. That is a heavier load than the ramp it replaces. In the stress case, five rings that ramp today became plunges of up to about 21 mm.
- V2 sometimes cuts shallower than needed, because whole turns are rounded up.

**Recommendation: V1.** It meets the bound everywhere with the least change to the output: the same angle, no new plunges, and the shortest programs. V2's plunges defeat the purpose of the bound.

**The axis, a further question for the owner, still open.** A descent on the rotary axis (decision 0543 point 2) cannot go back and forth. Holding it to the bound would mean capping its angle at `tan θ = 1 / (2π)`, about 9.04°, so a deep descent at the end face needs more turns. Past 10 turns it is plunged: in the stress case, a 22.8 mm descent that today fits in 10 turns at 12.87° became a plunge in the prototype. The two choices:
- **Leave descents on the axis outside the bound.** The note gives their depth per turn, up to 2.28 mm in the stress case.
- **Bound them too**, and accept the plunges.

Recommended: **leave the axis outside the bound**, because the plunge it would force is the heavier load. Until the owner answers, descents on the axis are left as they are (OI-2239).

**The owner's answer (26 Nov 2026, relayed by the coordinator): A (V1).** A whole ring whose depth per traverse would exceed the bound is ramped back and forth over its own ring at the same Ramp angle, with the same rule and the same bound function as lines and spirals; A reverses during the entry. It is built in PQ-2b (see *Verification (PQ-2b)*).

**The owner's answer on the axis (26 Nov 2026, relayed by the coordinator): A.** Descents on the rotary axis stay outside the depth limit, as built. The ramp note gives their depth per turn, and the user guide says that descents on the axis are outside the limit. No code changes. OI-2239 closes.

**Agreed with lane TOOL-LOAD (25 Nov 2026, in writing, at the owner's request; recorded in TOOL-LOAD-1 Q6 too).**
1. **One function.** A single crate-private function in `assemble/entry.rs`, owned by PQ-2, gives a ramp's largest depth per traverse: the minimum of the tool radius, one turn of a spiral ramp at its start radius (spirals only), the Largest depth per traverse setting, and an optional cap in mm. PQ-2 passes no cap.
   - TL-2 edits only the call site, to supply `L ÷ D` when Limit tool load is on.
   - PQ-2 owns `path_ramp::back_and_forth`, the follow and ring-ramp code and the ramp note, including its wording "the tool-load limit", which TL-2 only selects.
2. **Merge order.** PQ-2 (#930) merges first, and TL-2 rebases on it. TL-1 is unaffected.
3. **Terms.** Both lanes use TOOL-LOAD-1's glossary: Ramp angle, Depth per traverse and Cut section.
4. **Tests.** PQ-2 tests the ramp's geometry. TL-2 adds one test: with Limit tool load on and `L ÷ D` below the radius, the deepest ramp step is at most `L ÷ D` and the note names the tool-load limit. Neither lane re-tests the other's figure; TL-1 measures ramp moves like any other fed move.
5. **Settings.** Limit tool load adds no ramp setting, under any answer.

**What belongs where.** These belong in PQ-2:
- the ramp's geometry and its figures, known before cutting: Ramp angle, legs, Depth per traverse;
- the bound of Q8;
- the labels, the grouping and the note.

These belong in TOOL-LOAD:
- the cut section, measured in the simulation on the real stock, ramps included (its Q6 A);
- the limit and the warning;
- Limit tool load, and any rule that ties the ramp's angle to the stock (option C).

Neither lane adds a second load limit.

**Saved projects, under every option.** A saved operation keeps its settings. The labels and grouping (A) change no program. Q8 A changes the program of a saved operation with Entry Ramp on a line or partial ring whose legs are longer than `R / (2 · tan θ)`; the measurement above lists those that change. B adds no change beyond Q8. C would change programs only through TL-2's own rule for saved operations (TOOL-LOAD-1 Q7).

## Effort, risk and how the work would be split

| Pull request | Issue, answer | Effort | Risk | Golden changes | `openchuck-path` API | Can go alone |
| --- | --- | --- | --- | --- | --- | --- |
| (PQ-1) | #710, Q1 answered C | moved to lane SIMUL-EDGE | — | every simultaneous program (that lane's record) | breaking if a clearance query is added to `SurfaceSource`: minor bump | that lane |
| PQ-2 | #786 Q3 A (answered, with the tool-load requirement) and Q4 A | 6 to 7 days | low | none (no reference project uses Ramp) | breaking (`ramps_round_the_part` removed): minor bump | yes |
| PQ-3 | #779 Q2 (a) A, (b) A, (c) B (answered): Finish to a point and adaptive spacing over the tip | 8 to 10 days | medium (generation time: the budget above) | none (saved operations keep theirs) | additive (a parameter name) | after PQ-4: built on it, merged after it |
| PQ-4 | OI-2198 Q6 A | 3 to 4 days | low to medium | none | none | yes; before PQ-3 |
| — | #421 Q7 A | close with this record | — | — | — | — |

Lane SIMUL-EDGE works in `simultaneous.rs` and `simultaneous/hidden.rs`, and in `surface.rs` with the `SurfaceSource` implementations if it adds a clearance query. This lane's PQ-2 is in `assemble/entry.rs`, `strategy.rs` and `swarf.rs`, PQ-3 in `extend.rs` and `helical.rs` (and the finishing schemas), PQ-4 in `helical.rs` (the adaptive measure). PQ-3 and PQ-4 share `helical.rs`, so they are merged one after the other. In this lane only PQ-2 needs the workspace minor bump for an API change; PQ-2b (Q9 part 3) changes output only and takes its own minor bump under decision 0048 (0.70.0, in merge order). Lane SIMUL-EDGE's clearance query, if added to the public `SurfaceSource` trait, is a second breaking change; the two do not collide in what they change (PQ-2 removes `Strategy::ramps_round_the_part` and the spiral ramp's drop on the surface; SIMUL-EDGE adds to `SurfaceSource`), and they meet only in test files that implement `SurfaceSource` (for example `crates/openchuck-path/tests/swarf.rs`), a textual merge. Each takes its own minor bump in merge order (decision 0048), as the coordinator set. Each changes the user guide's text for what it changes, and each adds a *Development version* line to the release notes and an entry to `crates/openchuck-path/CHANGELOG.md`. Total for this lane, without #710: about 17 to 21 days (PQ-2 6 to 7 with the tool-load bound, notes and tests; PQ-4 3 to 4; PQ-3 8 to 10 with the adaptive stepover over the tip), so about three and a half to four weeks with the answers so far. Order: PQ-2 is independent; PQ-4 comes before PQ-3, because adaptive spacing over the tip needs PQ-4's span and query fixes, and both change `helical.rs`: PQ-3 is branched from PQ-4 (or rebased on it once it is merged) and merged after it, so they never conflict.

Nothing here needs the owner's hand test before it is built. After PQ-3 (and lane SIMUL-EDGE's build), an optional air cut or a cut in wood of the baluster's tip and a blisk region would be routed through lane OWNERCHECKS-PACKS with public example projects.

## Alternatives considered

The options not recommended under each question above are the alternatives, with their reasons. Two more were set aside:

- **Treat the blisk's tip gouge as a simulation artefact.** Not taken: the overcuts are 9 to 50 cells deep, at the blade tips where the moves cross the edge, and the same tips are within one cell with the STL and a ball; the G-code line that crosses the edge was found.
- **Raise the 10-turn limit instead of defaulting Steeper ramp near the axis on.** Not taken: the 10-turn bound of 0543 D4 keeps a ring ramp near the axis from running on for many turns, and the steeper ramp (0543 point 3) already exists for this case.

## Consequences

- **Code** (after the answers): `assemble/entry.rs`, `strategy.rs`, `swarf.rs` (Q3, Q4); `extend.rs`, `helical.rs`, `axial.rs`, `indexed.rs`, `finishing.rs` and the finishing schemas (Q2); `helical.rs` (Q6); `crates/openchuck-app` where new operations get their defaults (Q2, Q4) and in the form (Q2's setting). Tests named by requirement (OP-12, OP-11, OP-20, OP-21) on public or synthetic parts.
- **Documents:** the user guide's simultaneous finishing, Entry, Extend and Face free ends text; `docs/ui-design/ui-specification.md` for the new setting (Q2); the verification plan's OP-11 and OP-12 rows if a new test is named there. `docs/requirements.md` and `docs/architecture.md` do not change.
- **Accepted records amended** (if answered as recommended): 0543 D2, 0645 item 1, 0212 part B item 3, 0546 item 3.
- **NFR-12.** Nothing here says a program is safe. The warnings say what is left or cut free and to check it in the simulation; generated G-code remains the user's to verify before running it on a machine.

## Open items

Added (provisional numbers):

- **OI-2230:** the owner's answers to Q1 to Q7 and acceptance of this record.
- **OI-2231:** simultaneous finishing cuts into sharp convex edges (the blisk's blade tips, up to 2.55 mm with a flat end): the move check and the flat end's hidden-surface check (Q1). Settled by lane SIMUL-EDGE's build of Q1 C (OI-2235).
- **OI-2232:** a ramp that goes round the part cuts into it (face flow, pencil, simultaneous on a face set, crankshaft helical and simultaneous), and the steeper-ramp search stops short (Q3, Q4). Built by PQ-2. Lead and Helix against the whole part stay in the #572 family.
- **OI-2233:** the Makera Z1 post refuses the STL blisk's simultaneous finish in the axial pattern (the replay's rotary counter reset); LinuxCNC writes it. Not in this lane's scope; for the coordinator to route.
- **OI-2234:** spurious off-axis zones in four 0.05 mm strips on the STEP blisk (decision 0363 item 3). Not in this lane's scope.
- **OI-2235:** the build of Q1 C, the owner's answer: the general continuous seed (OI-337 item 1) and a clearance test of the whole tool at every test point, in lane SIMUL-EDGE.
- **OI-2236:** adaptive spacing over the path past a pointed end (#779's idea 2). The owner chose to build it now (Q2 (c) B, 25 Nov 2026, relayed): in PQ-3, after PQ-4, within the time budget of Q2.

Added by the addendum (provisional numbers):

- **OI-2238:** build Q8 A in PQ-2 (#930): the tool-radius bound on the depth per traverse of every back-and-forth ramp, once the owner has seen the measured impact.
- **OI-2239:** the owner's answer to Q9, the settings that configure a ramp (A, B, C or D), including whether a whole ring's depth per traverse is bounded too. Q9 answered B (built in #930); its whole-ring part answered A (V1, 26 Nov 2026, relayed) and built in PQ-2b; the descent on the axis answered A (26 Nov 2026, relayed): left outside the limit, as built. Closed.

Updated in place: OI-318 and OI-337 (the blisk now generates and cuts its blade tips; this record), OI-2198 (the causes found).

## Verification (phase 1)

- **What was checked, and how:** each issue reproduced on `main` `d1674b7c5ba33e15227dff542814eeff1722a23e` (0.63.0), in a Linux cloud container (4 cores, shared by three investigations, so times are rough), with release builds of `openchuck-cli` and `openchuck-step-worker`; operations written into copies of the public example projects; `openchuck-cli regenerate` (Makera Z1 and LinuxCNC posts) and `openchuck-cli simulate` at 0.1 mm cells. The finial radii were read from the ball's moves in the program; the engagement of #844 from 0.3 mm voxels of the program; OI-2198's places from a throwaway instrumented copy outside the repository, since deleted. The code causes were read from the source at that commit; the cause of the crankshaft helical and simultaneous ramp cuts is inferred from the code, not traced in the program.
- **Results:** the tables above. Raw notes with every command, setting and message: [docs/archive/0695-phase-1](../archive/0695-phase-1/README.md).
- **Not verified:** the STEP blisk's simultaneous finish at a 2 degree step (the run hit the container's time limit); the crankshaft's roughing in rings with Ramp; nothing was cut or air-cut; no owner project was used.
- **The figures** are drawn from the measured numbers (the finial) and as a schematic of the measured case (the blade tip).

## Verification (PQ-2, pull request #930)

- **What was built:**
  - Q3 A, with the tool-load requirement: every ramp follows its own pass, back and forth, and `Strategy::ramps_round_the_part` is removed.
  - Q4 A: a plunge only where no ramp fits, named in a note with its reason and depth. The steeper search reaches the maximum. New operations get Plunge where no ramp fits and Steeper ramp near the axis on.
  - Q8 A: the tool-radius bound on every back-and-forth ramp.
  - Q9 B, at its default: Largest depth per traverse, and one bound function as agreed with lane TOOL-LOAD.
  - A's regrouping: "Plunge where no ramp fits" and the Advanced group.
  - The whole-ring part of Q9 is not built; it waits for the owner (OI-2239).
- **How:**
  - Tests were written first, named by requirement:
    - `tests/ramp_load.rs`, 15 tests. The first seven were run on `main` `aadf3cc8`, and six failed.
    - `path_ramp` and `assemble::entry` unit tests, among them the bound function's.
    - The `ramp_follows.rs` extensions.
    - The `openchuck-app` tests for the new-operation defaults, the form groups, and a saved operation keeping its program.
  - At the default, the ramp moves were byte-identical to the Q8 commit in 60 synthetic ramp cases.
- **Results, at the release head (main `12cde6e` merged; Linux container, release build):**
  - `cargo test --release -p openchuck-path`: 144 unit, 890 integration and 1 doc test pass.
  - `-p openchuck-app`: 298 unit (1 ignored) and 625 integration tests pass. No golden changed.
  - The `openchuck-ui` form tests pass.
  - fmt, clippy, and `cargo xtask records`, `trace`, `deps`, `licences` and `changelog check-pr` all pass.
- **Simulation of the six cases of OI-2232** (public blisk and crankshaft, release CLI, 0.1 mm cells): the deepest overcut with Ramp went from 0.261 to 0.684 mm before to 0.0441 to 0.0485 mm after, the same as each operation entered with Plunge. Q8 leaves these figures unchanged.
- **Measured impact of Q8:** no golden or shipped example changes, and no new refusal or plunge. Two of the six cases change: crankshaft helical pass 1302 from 3.068 to 1.587 mm, and blisk face 19 pass 130 from 2.617 to 1.500 mm. Program times change by under 1%.
- **Not verified:**
  - No air or wood cut.
  - The STEP-kernel tests were run for the ramp cases only.
  - The cut-section figures are estimates from the tool's engaged section, because the simulation reports removal per operation only.
- **CodeRabbit's review of #930 (two Major findings, both real, fixed in its one fix push, tests first):**
  - **The depth per traverse is measured on the ramp as cut.** Where a pass rises faster than the Ramp angle, its own floor could hold an earlier visit high, so the drop between visits exceeded the bound while the note gave the bound. On the reviewer's case, 0.900 mm against 0.2 mm. The legs are now shortened until the measured drop fits.
  - **Where no leg length can soften the drop, the ramp follows it** (decision 0543 D4), and the note says so: "more than the tool radius (…), as it follows its pass's own steep drop". The drop is the pass's own step, such as pass 1851 of the blisk's roughing in rings, an 8.017 mm step. That pass is not refused, because the step is the part's. **The owner confirmed this on 26 Nov 2026 (answer A, relayed by the coordinator): such passes are followed with the note and never refused; decision 0543 D4 is unchanged (OI-2240, closed).**
  - **The stock under a ramp is probed only as far as its legs reach.** Stock beyond the legs no longer raises the ramp's start. The legs never go past that reach, so the probe still covers every leg (OP-12).
  - **Tests:**
    - `assemble::path_ramp::req_op_11_a_pass_rising_from_its_first_point_keeps_the_depth_per_traverse`;
    - `assemble::path_ramp::req_op_11_a_rise_no_leg_can_bound_follows_the_pass_s_drop`;
    - `entry_link_stock::req_op_11_req_op_12_a_ramp_probes_the_stock_only_as_far_as_its_legs_reach`.
  - **Not fixed, and recorded in one sev3 issue:**
    - a fallback plunge's depth when the rapid stop is clamped to the clearance;
    - the refusal's wording when the Ramp angle equals the maximum with Steeper ramp near the axis off;
    - a whole ring's depth per traverse is reported as the mean per turn;
    - two test precisions.
- **Open items:** OI-2232 and OI-2238 close with this pull request. OI-2239 (whole rings and the axis) stays open for the owner.

## Verification (PQ-4, pull request #928)

- **What was built (Q6 A):** helical finishing as a spiral, with adaptive spacing on, now measures the gaps between its wraps only where the whole gap is outside the held regions' keep-out bands and more than the tool's reach from each face turn (decision 0297). It no longer spends its query budget inside the held region or across a step wall, where a flat end's height jumps from one wrap to the next and no spacing removes it. Rings, and every operation with adaptive spacing off, are unchanged. No API change.
- **Built differently from the record:** Q6 A's third item, naming the frame ("setup X") in the message, was not built. No other message, the Results or the guide names a frame for X, and the places that misled are no longer reported. It is raised as OI-2237.
- **How:** tests first.
  - `adaptive_held_steps::req_op_04_req_set_09_adaptive_spacing_beside_a_held_step_confirms_a_flat_end` and `adaptive_held_steps::req_op_04_adaptive_spacing_at_step_walls_confirms_a_flat_end` use a synthetic stepped cylinder of r 10/8/6 with a 3.175 mm flat end. Both failed on `main` `aadf3cc8` with "at 36 places, the first near X 21.585", and both pass. Together they take 0.26 s.
  - The unit test `helical::tests::req_op_04_req_set_09_the_helix_is_measured_only_outside_bands_and_face_turns`.
- **Results (OI-2198's case, the stepped first-cut part with the flat end, `openchuck-cli regenerate` and `simulate`):**
  - With adaptive spacing on: no partial path, and no passes added, in 8 s, against 44 s and 23 unconfirmed places before.
  - With adaptive spacing off: the program is byte-identical.
  - The simulations of the two programs are the same: Finish removes 363.4 mm³, the most material left is 0.5018 mm at X 115.990 beside the held region, and the deepest overcut is 0.0457 mm.
  - No golden changes.
- **Coverage:** 51 of the 52 changed lines in `helical.rs` are run. The one not run is the branch for a helix with nothing left to measure.
- **Not verified:** nothing was cut or air-cut.
- **Open items:** OI-2198 closes; OI-2237 is open.

## Verification (PQ-2b, Q9 part 3)

- **What was built** (the owner's answer A (V1) to Q9 part 3, 26 Nov 2026, relayed by the coordinator; issue #786):
  - `Entry::ring_plan` plans a whole ring's ramp round itself as before, the steeper search near the axis included. Where the depth per traverse that ramp would cut, `min(2πr · tan θ, rise)` (`RingRamp::depth_per_traverse`, the figure the note already gave), is above the bound from the one function `largest_depth_per_traverse` (the tool radius, the Largest depth per traverse setting and, later, TOOL-LOAD's cap), the ring is ramped back and forth over its own points by `follow` and `path_ramp::back_and_forth`, at the angle the ring was planned at, under the same bound, the same measured drop and the same D4 rule as lines and spirals.
  - The note names the bound that held it, and the note on reversals lists the pass. A ring planned steeper near the axis keeps that angle on its legs and is named in the steeper-ramp note.
  - Unchanged: a ring within the bound (byte-identical moves and notes to `f7d88837` on three synthetic rings, compared by a throwaway dump test that was not kept); the descent on the rotary axis, not held to the bound (the owner's answer A of 26 Nov 2026: outside the limit); refusals and plunges of rings past 10 turns even at the Maximum ramp angle, decided before the ring is held. No public API change; the workspace takes its own minor bump for the output change (decision 0048), 0.70.0, in merge order.
- **Tests, written first and named by requirement** (`crates/openchuck-path/tests/ramp_load.rs`):
  - `req_op_11_a_whole_ring_deeper_per_turn_than_the_bound_is_ramped_back_and_forth_over_itself` (a ring 12 mm from the axis, a 2 mm flat end, 3°: one turn 3.95 mm against 1 mm; within the angle, depth per traverse at most 1 mm, length at least `5 / tan 3°`, every pose over the ring raised along the tool axis, A reversing in the entry only);
  - `req_op_11_a_whole_ring_within_the_bound_still_ramps_round_itself`;
  - `req_op_11_the_largest_depth_per_traverse_setting_holds_a_whole_ring`, which replaces `req_op_11_a_ramp_round_a_whole_ring_ignores_the_largest_depth_per_traverse`: the owner's answer reverses that contract;
  - `req_op_11_the_note_names_the_bound_that_holds_a_whole_ring_ramped_back_and_forth`;
  - `req_gc_07_whole_rings_held_back_and_forth_are_the_same_at_one_two_and_many_threads`;
  - `req_op_11_req_nfr_07_the_steeper_ramp_reaches_the_maximum_before_it_refuses`, re-derived: the steeper search still settles at 21.14°, the shallowest angle that fits, short of the 45° maximum, and the ring (0.6 to 1.5 mm from the axis, a 1 mm ball) is then held back and forth, in 66 legs. Its own 0.9 mm rise in its first 10 degrees is a step no leg can soften, so there the ramp follows the pass's drop (0.829 mm) and the note says "more than the tool radius (0.500 mm), as it follows its pass's own steep drop" (decision 0543 D4, OI-2240).
  - The descent on the axis is covered by `ramp_follows::req_op_11_req_op_15_a_ring_on_the_axis_descends_there_while_a_turns`, unchanged and passing (at 10° its 3.32 mm per turn is above the ball's 3 mm radius and is not held).
  - On `f7d88837` the first, third and fourth failed; the second and fifth passed, as they describe behaviour that does not change. Each new test takes under 0.2 s uninstrumented (release).
  - `openchuck-app` `operation::req_op_11_req_ui_07_the_basic_entry_settings_stay_in_entry_and_the_rest_are_in_advanced` checks the new description of Largest depth per traverse.
- **Results** (Linux container, release builds, worktree of `f7d88837`):
  - `cargo test --release -p openchuck-path`: 146 unit, 895 integration and 1 doc test pass. `-p openchuck-app`: 298 unit (1 ignored) and 625 integration tests pass. No golden changed (none uses Entry Ramp).
  - `cargo fmt --all`, `cargo clippy --workspace --all-targets` (clean), and `cargo xtask deps`, `licences`, `records` and `trace` pass.
- **Measured on the cases of the prototype** (release `openchuck-cli` of `f7d88837` and of this change, `regenerate --post makera-z1` and `simulate` at 0.1 mm cells; the crankshaft with the STEP worker):

  | Case | Deepest depth per traverse | Program time | Deepest overcut | Refusals and plunges |
  | --- | --- | --- | --- | --- |
  | Baluster, Ramp forced on, 6° | 4.700 → 1.587 mm (Rough); Finish 1.625 → 1.587 mm | 3332.2 → 3358.2 s (+0.78%) | 0.0481 mm, the same | the same 26 single-point plunges; none new |
  | Baluster, 3° | 2.987 → 1.587 mm; Finish 1.625 → 1.587 mm | 3836.9 → 3872.2 s (+0.92%) | 0.0481 mm, the same | none new |
  | Crankshaft, rotary roughing in rings, 3° | 2.202 → 1.587 mm | 30225.2 → 30230.3 s (+0.02%) | 0.0427 mm, the same | none new |
  | Stress case (baluster finish, 44 mm stock, 3°, Steeper ramp near the axis on) | rings 5.598 → 1.587 mm; the deepest is now the descent on the axis, pass 24, 2.279 mm, not held | 4608.3 → 4711.8 s (+2.25%) | 0.0481 mm, the same | pass 23 plunged, as before; none new |

  - The prototype measured the stress case at −2.6%, because it also capped the axis and so plunged pass 24; the build leaves the axis alone, so pass 24 still descends on the axis and the time rises instead.
  - The "before" figures differ slightly from the prototype's on the crankshaft (30961.8 s there), because the prototype's base was PQ-2's earlier head `ec7d7400`.
  - Raw output was kept only in the session's scratch space and is not archived.
- **Not verified:** no air or wood cut; the six blisk and crankshaft cases of OI-2232 were not re-run (the prototype found no whole-ring start in them); coverage was not run.
- **Open items:** OI-2239 closes. Its ring part is settled here, and its axis part by the owner's answer A of 26 Nov 2026: descents on the axis stay outside the limit, as built.

## Verification (PQ-3)

- **What was built (Q2 (a) A, (b) A, (c) B):**
  - **The setting.** A new setting, **Finish to a point** (`finishing::parameter::FINISH_TO_A_POINT`), on helical, axial and indexed finishing.
    - It is off in the schema, so a saved operation keeps its program.
    - `openchuck-app` writes it on for a new operation, in decision 0545 D3's pattern.
  - **Finding the point.** A point is a free end of decision 0344 that the stock carries on past, where the tool at its last drop touches the part nearer the rotary axis than its own cutting radius at every angle.
  - **The path past the point.**
    - The finish carries on past the end, following the part down to the end's last drop: the helix (or the rings) at its pitch, or each pass along X.
    - Whole turns then take the tool down by the Stepdown to the axis. The last turn puts the tool's end at its cutting radius on the axis: a ball's centre on the axis at the point's X + R.
    - The stretch is fitted on its own, so the passes over the part are unchanged.
    - Where Face free ends also acts, its turns are cut first.
  - **Warnings.** Decision 0344's two Part cut free warnings are given on the new setting, unless Face free ends already faced that end down to the axis.
  - **Adaptive spacing** measures past the point and adds passes there.
- **Built differently from the record's wording:**
  - **The run-out.** The run-out of Extend past the stock is unchanged. The record said decisions 0212 part B item 3 and 0546 item 3 are amended for this setting. As built, the setting adds its own stretch past the point, which follows the drop, and it does not turn the run-out into a descent. A run-out that followed the drop to the axis would cut through all the stock past the point at the axis. That would be a full-width slot, not a finish, and it would be cut free anyway.
  - **Query accounting.** The queries spent past the end are read as the difference in the figure adaptive spacing already reports, as the record says. No separate counter was added.
- **How:** tests first, in `crates/openchuck-path/tests/finish_to_a_point.rs` (new, 21 tests):
  - Written before the code and seen failing first:
    - the schema test;
    - the reach-the-axis tests for the three strategies (for example, helical's nearest centre past the point stood at 1.587 mm from the axis);
    - the adaptive test, which failed for axial finishing until its measure covered the point.
  - The "off is as before" test was written first as a guard, and passed before and after.
  - The gouge, warning, held-region, off-axis-end, GC-07, point-at-the-start and rings tests were written with the code or after it, and were not seen failing. The last two were added after the coverage check found the rings branch and the start end not reached.
  - In `openchuck-app`, these were also written after the code:
    - `operation::req_op_04_finish_to_a_point_is_on_for_a_new_operation_and_off_for_a_saved_one`;
    - `adaptive_example::req_op_04_req_nfr_12_the_tutorial_piece_is_finished_to_a_point`, which was seen failing with the setting off (0.734, 0.432, 0.240, 0.113 and 0.037 mm left at 0.25 to 1.25 mm past the point, matching the record's table);
    - `playback::req_ui_06_...`, updated for the new default.
- **Results, the time budget** (median of five runs each, release CLI at all threads, `openchuck-cli regenerate`, on the PQ-3 head in this lane's container). Finish time is the total less a run with Rough only.

  | Case | Adaptive spacing on | With Finish to a point | Change | Queries |
  | --- | --- | --- | --- | --- |
  | Public baluster, Finish | 17.86 s | 18.50 s | +3.6 % (budget 25 %) | 461,871 → 491,159: +29,288, 2.9 % of the 1,000,000 limit (budget 10 %) |
  | Stepped first-cut part, Finish (no point) | 3.74 s | 3.76 s | +0.4 % | no passes added in either case; the LinuxCNC program is byte-identical |

- **Results, the cut:**
  - **The baluster tip.** With adaptive spacing and Finish to a point on, the path is complete, and 80 passes are added against 73.
  - **What the finish leaves.** The radius the finish's swept ball leaves on the axis side past the finial's point is 0 at every sample from 0.25 to 1.25 mm past it.
  - **Simulation.** `simulate` at 0.1 mm gives the same most-material-left (0.9239 mm) and deepest overcut (0.0502 mm) with and without the setting, and Finish removes 2168.3 against 2153.9 mm³.
  - **Synthetic part.** On the synthetic pointed part, the generation time with adaptive spacing on grows by 3 to 4 times. The tip is a large share of that short part.
- **Indexed finishing with 4 positions.** On the synthetic part, with adaptive spacing on, indexed finishing at 4 positions cannot confirm the material just past the point: another pass would need to be closer than the Smallest spacing. The partial-path warning names the places, as anywhere else. At 8 and 16 positions it is confirmed. Helical and axial finishing confirm it. This is noted, not changed.
- **Checks:**
  - `cargo test --release -p openchuck-path` and `-p openchuck-app` pass.
  - fmt, clippy, and `cargo xtask records`, `trace`, `deps`, `licences` and `changelog check-pr` pass.
  - No golden or shipped example changes, because their operations were saved without the setting.
- **Coverage** (`openchuck-path`, the 12 tests run instrumented, before the last two were added): every added line in `faces.rs` is run, 175 of 177 in `finishing.rs`, 79 of 83 in `axial.rs`, 32 of 33 in `indexed.rs` and 104 of 142 in `helical.rs`. The helical lines not run were the rings branch and its adaptive spacing; the axial ones were a point at the start of the part; the Stepdown loop in `finishing.rs` was not run either. The two tests added then, and the Stepdown check added to the off-axis test, reach them. Left: the branch for a stock of no known length, as decision 0344's free ends have it.
- **CodeRabbit on #956** (review of `7f9459a5`): four Major findings were confirmed and fixed in the release push, each with a test written first and seen failing.
  - **Stock-to-leave zones.** A point under a part of the surface with its own stock to leave is not finished to, so its allowance is kept: `req_op_04_a_point_under_a_zone_with_more_stock_to_leave_is_not_finished_to`.
  - **Two cut rings.** With a point at each end, a held region beyond the other ring no longer suppresses the part's own warning: `req_nfr_12_a_held_region_past_another_cut_does_not_hold_the_part`.
  - **Finishing layers.** With Finishing layers on, helical finishing's run past the point is cut in the same layers: `req_op_04_the_run_past_a_point_is_cut_in_finishing_layers`. Axial and indexed finishing were already layered.
  - **Rings with a point at the start.** Rings past a point at the start are measured round their whole turn; the rings no longer take the helix's shifted start angle: `req_op_04_rings_past_a_point_at_the_start_are_measured`.
  - **A Minor, also fixed:** a Stepdown too small to resolve no longer loops without end; the turns down to the axis are at most 1000 (`req_nfr_07_a_tiny_stepdown_does_not_hang_the_turns_to_the_axis`; not run before the fix, as it would hang).
  - **CodeRabbit's second review** (of the release head `2abbf77e`) raised two more Majors. Both were confirmed, and the coordinator allowed one fix push for them; each test failed first:
    - **The selected region.** A point end is kept only where the selected region allows the tool all round at the end's last drop, so no turn goes down to the axis at an unselected point: `req_op_02_a_point_outside_the_selected_region_is_not_finished_to`.
    - **Points per turn.** Each turn down to the axis has at most 3600 points, a tenth of a degree apart, and the turns check for cancellation. At a 0.01 degree step the final turn had 36,001 points before the fix: `req_nfr_07_a_tiny_angle_step_does_not_flood_the_turns_to_the_axis`.
    - **Not changed:** its follow-up on the Stepdown cap. The cap widens the spacing only where more than 1000 turns would be needed, that is, below about 0.002 mm over a descent of at most the tool radius. That is answered on its thread.
  - **A Minor not fixed here**, for the lane's ticket: an app test of a saved helical, axial or indexed operation without the setting. The schema default (off) is tested in `openchuck-path`.
- **Not verified:** nothing was cut or air-cut.
- **Open items:** OI-2236 closes.
