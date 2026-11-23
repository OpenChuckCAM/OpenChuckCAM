# PENCIL-REST-A: Pencil clean-up judges material left from the tool, rests clear of a wall it touches, and does not cut stretches shorter than a set length (issue #632; pull request #856; lane PENCIL-REST)

- **Status:** accepted by the project owner on 23 Nov 2026, relayed by the coordinator (decision 0049). Proposed the same day by lane PENCIL-REST, phase 1. The owner's answers, all of 23 Nov 2026 (relayed): **Q1 M1**; **Q2 P1 only**, rejecting P2 after the lane measured it (Q2 below); **Q3 D2**, plus an option for the shortest stretch to cut; **Q4 E1**, with the setting moved to *Advanced* for every strategy that has it and its tooltip explaining the trade-off; **Q5 K2**, the 0.55.0 known-issue line amended in this pull request. Figures of causes A and B, of the shortest cut and of P2 are included under the owner's standing rule that records carry diagrams (relayed). Phase 2, the build, is in the same pull request (OI-9942).
- **Number:** provisional. Under the owner's rule of 23 Nov 2026, provisional numbers are unique across lanes, and real numbers follow `main` in merge order. The record is `PENCIL-REST-A`, carrying the lane's name. The open items are OI-9941 and OI-9942, from the numbers the coordinator set aside for this lane (OI-9941 to OI-9943), because `cargo xtask records` accepts only a number after `OI-`. The coordinator assigns the record and `OI-` numbers when the Merge Manager releases the pull request. No project schema number: nothing stored changes format (item 3's value is an ordinary strategy parameter). No spike.
- **Date:** 23 Nov 2026.
- **Lane:** PENCIL-REST, branch `claude/lane-pencil-rest`, from `main` at `286472a` (library family 0.56.0). Issue #632 (milestone 2026.2), which refers to #351 and #257.
- **Requirements:** OP-09 (rest machining and pencil clean-up), OP-27 (exact surface contact for STEP parts), OP-23, UI-07 and UI-19 (the inspector's fields and its Advanced section), NFR-06, GC-07, NFR-07, NFR-12. No requirement change. `docs/requirements.md` and `docs/architecture.md` are not edited.
- **Amends (on acceptance):** [decision 0526](0526-stepall-close-out-and-exact-contact-setting.md) item 2 (where *Exact STEP surface contact* sits: *Advanced*, not *Pattern and tolerance*) and item 3 (its words). It also amends the UI specification, section 11.4, where it names that row's place.
- **Builds on:** [0044](0044-slice-15-rest-and-pencil.md) items 1 and 5 (the remaining-material map, the pencil rest), [0162](0162-simulation-results-histogram-and-probe.md) item 6 (each pencil move checked against drops along it), [0226](0226-pencil-on-the-inside-corners-of-a-step-part.md) (pencil on a STEP part's inside corners), [0209](0209-step-geometry-for-every-strategy.md), [0211](0211-step-accuracy-measured.md), [0256](0256-no-fixed-toolpath-mesh-qualifies.md), [0277](0277-exact-ball-drops.md), [0297](0297-helix-strategies-turn-at-faces-across-the-axis.md) (a helix turns once at each face the tool's side touches), [0338](0338-exact-outline-drops-not-adopted.md) (the exact outline drop for a flat end), [0526](0526-stepall-close-out-and-exact-contact-setting.md) (Exact STEP surface contact, off by default for pencil; OI-1351).
- **Open items:** adds OI-9941 (the owner's answers and acceptance; closed 23 Nov 2026) and OI-9942 (phase 2, the build). No open item is kept for P2: the owner rejected it. Relates to OI-1351 (pencil's exact contact is over the 2-times limit) without closing it, and to OI-192 (pencil makes one pass per crease) without changing it.

## Context

Issue #632 (lane OWNERCHECKS, 8 Nov 2026) uses the public `testdata/step/valid/shouldered_shaft.step`, turned along X in 26 mm stock from X −10 with 6 mm held. An axial finish with a 3.175 mm tapered ball (tip radius 0.5 mm, 5 degrees) is followed by pencil clean-up with a 2 mm flat end at its defaults (smallest material to clean 0.15 mm, crease angle 30 degrees, largest step 0.5 mm). With **Exact STEP surface contact** off (pencil's default, decision 0526), the operation makes 6 passes and 40 moves at the one sharp inside corner (X 12). With it on, it makes 31 passes and 390 moves. The project is lane OWNERCHECKS' check for #351 and #257: `owner-checks/351-257-exact-step-contact/` on branch `owner-checks/2026.1-alpha` (synthetic, public model only). The branch named in the lane brief, `owner-checks/351-257-exact-step-contact`, is no longer on the remote.

The lane brief relays lane FACEFLOW's reproduction on current `main`. Off, pencil cuts 7 degrees of the 360 degree corner, and 3 of its 6 passes are single points (a plunge to the corner and straight out). On, it cuts 316 degrees in 31 stretches, none a single point. Lane FACEFLOW put both down to the remaining-material test sitting at its 0.15 mm threshold for the thin ridge a tapered ball leaves in a sharp corner, with a 0.0025 mm shift of the contact (tessellation against exact face) flipping it. This lane found no pull request by lane FACEFLOW for #632, open or closed (searched 23 Nov 2026), so that analysis is taken as relayed.

Records read: 0044, 0162, 0209, 0210, 0211, 0226, 0256, 0277, 0297, 0338, 0526. Open items read: OI-81, OI-189, OI-192, OI-835, OI-1350, OI-1351.

### How pencil decides today (code on `main`, `crates/openchuck-path/src/pencil.rs`)

1. **The rest.** At each sample along the crease, the tool is dropped on lines across it and rests where its tip is lowest. It is then moved towards the crease while it stays as low (within `REST_TOLERANCE`, 1e-5 mm), and the last such place is found by 40 bisections. Against a wall rising from a floor, that place is where the tool's side **just touches the wall**, to about 1e-12 mm.
2. **Moves between rests.** Each move is checked against drops along it (decision 0162 item 6). Where the tool would be lower than a drop allows, a point dropped half way is put in. Where eight halvings do not clear it, the pass ends there.
3. **Material left.** A placed point is cut only when `RemainingMap::near(p.contact)` holds: a sample of the part with more than `min_remaining` of stock above it lies within `reach` of the drop's **contact point**. `reach` is the tool's radius plus the sampling spacing (1 + 0.5 = 1.5 mm for a 2 mm flat end).

### What this lane measured

The experiment (scratch, not committed) used `openchuck_testkit::brep::shouldered_shaft`, the closed form of the same part (decision 0226), with `BrepSurface` exact drops off and on. The axial tapered-ball finish of the check project was simulated on a 0.1 mm `DexelStock`, then pencil ran with the 2 mm flat end. Release build on `main` `286472a`, Linux x86_64, 4 vCPU. The OpenCASCADE kernel was not built (8 to 71 minutes), so the STEP file itself was not read. The closed form's tessellation (96 facets round, polygon trims) differs from the import's, so its counts differ from the STEP project's. It shows the mechanism:

| Case (closed form) | Off | On |
| --- | --- | --- |
| Pencil at `min_remaining` 0.10, 0.15 and 0.20 | Refuses at each threshold: "found material left but the tool cannot rest in any corner that holds it" | At each threshold: 31 passes, 621 points, 251 degrees of the corner, none single-point; 3.7 to 4.1 s |
| Where the drop puts the flat end's contact (X) | X 10.000: the far edge of the flat, 2 mm from the corner | Anywhere from X 9.99 to 11.63, varying with A |
| Nearest material sample to the contact | 1.556 to 1.779 mm, all beyond the 1.5 mm reach | 0.07 to 1.93 mm; 546 of 1025 points within reach |
| Nearest material sample to the tool's axis (tip to 10 mm up) | 0.556 to 0.780 mm, every point within reach | Every resting point within reach |
| Tool drops at X 11.0000 exactly (flat's side flush with the shoulder at X 12) | Rests on top of the shoulder (tip at radius 12) at most A, on the bar (radius 8) at some | The same knife edge, at different A |
| Tool drops at X 10.999 | On the bar at every A | On the bar at every A |
| Points of the passes lifted onto the shoulder's top (radius 12) | — | 262 of 621 |

So on the closed form neither setting is decided by the 0.15 mm threshold: 0.10, 0.15 and 0.20 give the same output. Two things decide it:

- **Cause A: material is judged from the contact point, and a flat end's contact is arbitrary.** A flat end on the bar touches it along a line across its whole flat bottom (the bar is straight along X under it). Which point of that line the drop reports depends on the surface: the tessellation reports the far edge (X 10), the exact faces anywhere between. The corner's material is 1 mm from the tool's axis but 1.5 to 2 mm from the far edge, right at the 1.5 mm reach. A ball end's contact in a corner is unique, which is why decision 0044's test (a ball, a 105 degree corner) never showed this.
- **Cause B: the rest is placed flush against the wall, on a knife edge.** The bisection of the rest stops where the flat's side touches the shoulder face. There, a drop of the same tool can read the graze either way: as clear, with the tool on the bar, or as resting on the shoulder's top, 4 mm higher. That holds for the rest itself and for a bridge point dropped half way at a slightly different A. The bridge check then finds the move "would cut the part", and the pass rides up onto the shoulder or ends. That splits the corner into stretches and lifts points 4 mm. In the STEP project's 390 moves, this is the likely source of the moves out and back. In the closed form, cause A alone decides the off case, because the far-edge contact is out of reach at every point.

![Cause A, a section through the rotary axis at the sharp corner: a 2 mm flat end rests on the bar (radius 8) with its side against the shoulder wall at X 12. The tapered ball left a thin wedge of material on the wall. The tessellated drop reports the contact at X 10, the flat's far edge; a 1.5 mm reach from there does not reach the wedge (nearest 1.56 to 1.78 mm), so nothing is cut. The exact drop reports the contact anywhere from X 9.99 to 11.63, in and out of reach, so the corner splits into 31 passes. Measured from the tool's axis at X 11 (option M1), the wedge is 0.56 to 0.78 mm away at every point.](PENCIL-REST-A-pencil-cause-a-contact-point.svg)

![Cause B, the same section: the rest is placed at X 11 exactly, where the flat's side lies in the wall's plane. A drop of the tool there reads as resting on the bar at some A and on the shoulder's top (radius 12, 4 mm higher) at others, on the tessellation and the exact faces alike. The move check of decision 0162 then rides the pass up 4 mm or ends it: with exact contact on, 262 of 621 points are lifted and the corner is cut in 31 passes. Option P1 rests the tool a tenth of the tolerance clear (X 10.999), where every A rests on the bar.](PENCIL-REST-A-pencil-cause-b-flush-rest.svg)

The figures are drawn to scale (1 mm to 40 px) except the material wedge, which is drawn wider so it can be seen.

Both causes were confirmed by changing the scratch build, then reverted. A cleared rest (backed off the wall by 0.001 mm) and the material test from the tool's tip and axis, together, give one closed pass around the whole corner (360 degrees, 193 points, no point lifted) with exact contact **off and on alike**, at each threshold. Off took 0.11 s; on took 1.9 s (about 16 times, still over OI-1351's 2-times limit). Either change alone is not enough. With only the cleared rest, "on" still gives 31 passes and "off" still refuses. With only the tool-based test, both give one pass, but "on" lifts 448 points onto the shoulder.

The STEP project differs in counts (6 passes off, not a refusal) because its tessellation is the importer's (0.01 mm deflection, exact circular trims), so some contacts land within reach. Phase 2 confirms the STEP project with the kernel (Verification).

## Decision (the owner's answers of 23 Nov 2026, relayed)

1. **Material is judged from the tool as placed, not from its contact point (Q1, M1; the owner's answer).** A placed point is cut when a sample with material left lies within reach of the **tool's axis over its cutting length**: the distance from the sample to the axis segment is at most the tool's cutting radius plus the sampling spacing. This is the same `reach`, measured from the axis: the material the tool's own body covers, widened by the sampling spacing. For a flat end it no longer depends on which point of a line contact the drop reports.

   It is not a superset of today's test. A ball's contact can lie up to its radius from the axis, so a sample just beyond the contact's far side counts today and not under M1. In a corner the material lies between the axis and the corner, so little change is expected for a ball; phase 2 measures it.

   `RemainingMap` gains a query by segment (`near_segment`, or the like). `near` by point stays for rest machining, whose contact is unique. This applies on mesh and STEP parts alike.
2. **The rest stays clear of a wall it touches (Q2, P1; the owner's answer).** After the bisection, the rest is backed away from the crease by a tenth of the operation's tolerance (0.001 mm at the default 0.01 mm). The material left on the wall is within the tolerance the user set. Bridge points are dropped from the same cleared line. The shared drop is not changed: Q2 below shows that changing it (P2) changes other strategies' output.
3. **Short stretches are not cut, and the notes say where (Q3, D2 with the owner's option).**
   - Pencil gains a setting, *Shortest cut* (key `min_length`, the key and convention of edge following's *Shortest chain*; millimetres; *Advanced* section, where `min_length` already sits).
   - A stretch of the corner is cut only when its length along the crease, from its first wanted point to its last, is at least *Shortest cut*.
   - A stretch of one point (zero length) is never cut, whatever the setting.
   - **Default: empty, meaning the tool's cutting diameter.** The field's hint shows the value and its source ("2.000 mm, the tool's diameter"), as the inspector already does for values taken from the tool (UI specification section 8, *Hint and source*). The owner's purpose is fewer pecks. A stretch shorter than the tool's diameter moves the tool less than its own width along the corner: it is a plunge, a dab and a retract that cleans no more than one footprint. One diameter is the shortest stretch that is a cut along the corner rather than a peck, and it scales with the tool, where a fixed millimetre figure would be too long for a 0.5 mm tool and too short for a 6 mm one. A user who wants every stretch cut enters 0. Single points are still left uncut, with the note.
   - **The note** (information level, NFR-12): "Pencil clean-up left N stretches uncut, shorter than the shortest cut (L mm): at X x, A a; …", listing up to five places and then "and M more", followed by "Lower Shortest cut to cut them."
   - **Tooltip:** "Stretches of an inside corner shorter than this are not cut, so the tool does not plunge in for a short dab. Empty: the tool's diameter. A single point is never cut. The notes say where each stretch left uncut is."
   - A saved pencil operation has no value, so it takes the default. Its output changes anyway under item 1, so no migration is proposed and no schema number is needed.

   ![Shortest cut: one inside corner unrolled, A 0 to 360 degrees. Where material is left, a long stretch (A 20 to 205) is cut; a stretch of about 2 degrees and a single point are shorter than the shortest cut (by default the tool's diameter, 2 mm, about 14 degrees on a corner of radius 8) and are not cut. A note names both places and says to lower Shortest cut to cut them.](PENCIL-REST-A-pencil-shortest-cut.svg)
4. **Exact STEP surface contact stays off by default for pencil, moves to *Advanced*, and its tooltip explains the trade-off (Q4, E1; the owner's answer).** After items 1 and 2, on and off agree on where the corner is cut, as the experiment shows. The setting then changes only how exactly the tool sits on the faces (decision 0338: 0.0066 mm deepest on the blisk against 0.0342 mm), not which stretches are cut. It is still about 16 times slower on the closed form and 31 times on the blisk (OI-1351), over the 2-times limit (decision 0526 item 8 is unchanged).
   - **Where:** the row moves from *Pattern and tolerance* to *Advanced* (`FormSection::of("exact_contact")`). The setting is one key in one place, so it moves for every strategy that has it: helical, indexed, rest and simultaneous finishing (on by default) as well as axial finishing and pencil clean-up (off). This amends decision 0526 item 2 and the UI specification, section 11.4. The owner confirmed this reading on 23 Nov 2026 (relayed): every strategy that has the setting.
   - **Tooltip** (the setting's description; it amends decision 0526 item 3 and the wording of issue #634, at the owner's request to state the cost). The numbers are the measured examples of decisions 0277, 0338 and this record, stated as examples, not promises:
     - Outline kind (axial finishing, pencil clean-up): "On a STEP part, drop the tool (a ball, tapered ball or flat end) on the part's exact surfaces instead of the triangles made from them. The tool then sits closer to the true surface: on a test part, the deepest cut into it fell from about 0.03 to under 0.01 mm. Generation is much slower: on the parts measured, about 7 times for axial finishing and 16 to 31 times for pencil clean-up. It does not change which corners or areas are cut. No effect on a mesh part."
     - Ball kind (helical, indexed, rest and simultaneous finishing): "On a STEP part, drop a ball-end tool on the part's exact surfaces instead of the triangles made from them. The tool then sits closer to the true surface: on a test part, the deepest cut into it fell from about 0.04 to under 0.005 mm. Generation can be slower: up to about 1.8 times on the parts measured. No effect on a mesh part."
   - The user guide says the same without figures (the owner's rule: no numbers in the release notes or the guide).
5. **The 0.55.0 known-issue line is amended now (Q5, K2; the owner's answer).** `user-docs/src/release-notes.md`, 0.55.0 *Known issues*, now reads: "pencil clean-up with a flat end can leave most of a sharp inside corner uncut, or, with **Exact STEP surface contact** on, cut it in many short passes; check the corner in the simulation (#632)". Phase 2 adds the development-version line for the fix.
6. **No change to the threshold or the tessellation (M3 not chosen).** On the closed form, `min_remaining` from 0.10 to 0.20 changes nothing, and a finer tessellation would not move a flat end's contact off the far edge.

### What changes in output

- **Golden files and reference jobs:** none from items 1 to 4. No golden program, reference job (`testdata/golden/reference`, `testdata/projects`) or example project (`testdata/examples`) has a pencil operation (searched 23 Nov 2026), and item 4 changes where a row is shown, not a value.
- **Pencil operations:** every pencil operation can change, on mesh and STEP parts. Item 1 can make points wanted, or not, where they were not, or were. Item 2 rests the tool 0.001 mm further from a wall. Item 3 drops stretches shorter than a tool diameter. With a ball end in a V corner (decision 0044's test) the rest touches no wall and the contact is unique, so the change should be nil or small. Phase 2 measures it.
- **The OWNERCHECKS project:** expected one pass around the corner at X 11 (less the 0.001 mm clearance), off and on alike. Today it gives 6 passes and 40 moves off, and 31 passes and 390 moves on. Phase 2 measures it on the STEP file.
- **Cost:** in moves, one closed pass of about 2πr / 0.5 mm points (about 100 to 200 cutting moves at radius 8) with one entry and exit, against 40 moves (off, most of the corner missed) and 390 (on). In time: 0.11 s off and 1.9 s on, on the closed form, against about 0.1 s and 4 s today. The extra material test per point is a few distance tests against the map's nearby cells.
- **Library:** expected not breaking. The changes are private to `pencil.rs`, plus an added method on `RemainingMap`, a parameter added to pencil's schema, and a section change in `openchuck-app`'s form. The coordinator is told before any version change.

### Risk

- **Item 1 can change where a ball cuts.** It can stop cutting where material lay just beyond a ball's contact but outside the tool's body widened by the spacing. It can cut a little more where material lies beside the tool but not under its contact, for example at the ends of a blisk's crease, where a blade's material is within reach of the axis. Every point is still a dropped rest checked by decision 0162 item 6, so it cannot gouge; at worst it adds air moves. Phase 2 measures the blisk pencil (decision 0211's `step_accuracy` example) before and after.
- **Item 2 leaves material on the wall.** It leaves up to a tenth of the tolerance on a wall that pencil touches: within the tolerance, and far under the 0.15 mm threshold.
- **Item 3 leaves short stretches uncut** where material was found, and says where. That is the owner's choice (D2), and a user can lower the setting to 0.
- **The closed form is not the STEP import.** The mechanism is general (any flat end on a surface straight under it, any tool flush with a wall), but the STEP counts are confirmed only in phase 2.

## The owner's questions

All five are answered (marked). Q2 keeps the analysis the owner asked for before answering it.

### Q1. How pencil decides "material left" across a crease: **answer M1**

| Option | What | Output change | Cost | Risk |
| --- | --- | --- | --- | --- |
| **M1 (recommended; chosen): from the tool** | Material within reach of the tool's axis over its cutting length (item 1) | Flat ends cut the whole corner where material is; ball ends little changed (measured in phase 2) | A few more distance tests per point | Can add short stretches beside material that is not under the contact (no gouge) |
| M2: smooth along the crease | Keep the contact test; close gaps shorter than a tool diameter and extend each stretch by a radius (hysteresis) | Fills the gaps between stretches it already finds, but not the 353 of 360 degrees the off case misses | Small | Cuts where nothing was found; does not fix a contact that is out of reach everywhere (the closed form's off case) |
| M3: change the threshold or the tessellation handling | Lower `min_remaining`, or a finer drop surface | None on the closed form (0.10 to 0.20 identical) | Finer tessellation: decision 0256's cost | Does not address the cause |
| M1 + M2 | Both | As M1, with any remaining gap closed | As M1 | As M1 and M2 |

### Q2. Where the rest sits against a wall: **answer P1 only; P2 rejected by the owner**

| Option | What | Output change | Risk |
| --- | --- | --- | --- |
| **P1 (recommended; chosen): back off by a tenth of the tolerance** | Item 2: 0.001 mm at the default | Pencil only: passes stay on the bar; no point lifted onto the shoulder | Leaves at most 0.001 mm on the wall |
| P2 (rejected by the owner): make the shared drop treat a flush side as no touch | Change the drop used by every strategy (tessellation and exact) so a part that only touches the tool's rim, never coming inside it, does not hold the tool up | Pencil fixed as by P1 (measured); **16 tests in other strategies fail and two sets of golden programs change** (measured, below) | Breaks face detection and facing that rely on the flush contact (decision 0297); needs its own redesign |
| P1 now, P2 as a separate decision later (not chosen) | P1 in phase 2; P2 investigated later with its own record | As P1 now | As P1 now |
| P3: leave it | — | 31 passes, 4 mm lifts on the closed form with exact contact | The defect stays |

**The owner's question: would P2 adversely affect any other strategy? Yes.** The lane built P2 in the scratch build and ran the whole workspace's tests with it, then reverted it.

![Option P2, looking down the tool axis. Left: a wall flush with the tool's rim, touching it only at one point on the rim. Today it holds the tool up at the wall's top; under P2 it would not, and the tool slides past it. Helical finishing finds the faces it turns at by this case. Middle: a wall that comes inside the rim holds the tool up today and under P2 alike. Right: a flat end on a slope touches at its rim, but the slope comes inside the rim, so P2 asks only whether a part comes inside.](PENCIL-REST-A-pencil-p2-rim-rule.svg)

**What P2 is in code.** Every strategy drops the tool through `SurfaceSource::drop_tool`.
- On a mesh part, or a STEP part's tessellation, that is `cutter.rs`: `vertex`, `facet` and `edge` against the tool's `RevolvedProfile`. A vertex counts when it lies within the cutting radius, rim included (`height_at` accepts up to the radius plus 1e-9 mm). An edge counts where it comes within the radius (`roots(r_max)` with `disc >= 0`, which takes an edge just touching the rim circle).
- On exact faces it is `brep/drop.rs` (a ball, decision 0277) and `brep/outline.rs` (a tapered ball or flat end, decision 0338). `Outline::lift` takes points at `rho <= R`, and the search may already skip what reaches the tool only within `RIM_SLACK` (0.0001 mm) of its rim, as a search shortcut rather than a rule.
- Callers: rotary roughing (`roughing.rs`, its indexed form in `adaptive.rs`), helical (`helical.rs`, its face turns in `faces.rs`), axial, indexed (through `wrap_fit.rs`), simultaneous (`simultaneous.rs`, `simultaneous/hidden.rs`), rest machining (`rest.rs`, `rest/walls.rs`), pencil, face flow (`face_flow.rs`), extension past the stock (`extend.rs`), the region constraint (`region.rs`), leads and entries (`assemble/lead.rs`, `assemble/entry.rs`), and `solid.rs` and `composite.rs`, which pass drops to their parts.

P2 was built in its narrowest form: a vertex lying within 1e-6 mm of the rim, and an edge whose nearest approach to the axis is within 1e-6 mm of the rim, no longer hold the tool on a mesh. On exact faces, a point within 1e-6 mm of the rim no longer holds the tool in the outline drop. A part that comes inside the rim still holds it as today, so a slope under a flat end is unchanged.

**Measured** (`OC_P2=1 cargo test --release --workspace --no-fail-fast`, the same release build with P2 switched by an environment variable read at run time; `main` `286472a`; 28.6 minutes, 4 vCPU). 3439 tests passed and 17 failed. Each failure was run again on the same binaries with P2 off, and all passed. One, a UI zoom test, also passed with P2 on when run alone (contention, not P2). That leaves **16 failures caused by P2**:

| Where | Tests failing with P2 | What changed |
| --- | --- | --- |
| Golden programs (`openchuck-cli` `tests/e2e.rs`) | `first_cut` at 1, 2 and N threads; `blisk_helical_2` and `blisk_helical_5` end to end | The first-cut golden program differs at line 20, `G0 X71.411 A315.000` against `G0 X71.412`: the helix's face turn at X 71.41 (decision 0297) moves. The blisk helical programs change too |
| Helical finishing's faces (`openchuck-path` `tests/helical_faces.rs`) | 4 tests: a free end faced in layers down to the axis; a free end flush with the flutes on round stock is faced; a free end on the setup's own stock is faced and said; a face the held region keeps the tool from is named | A free end got **0 face layers against 6**. The face turns and free-end facing find a face where the tool's side touches it: exactly the flush contact P2 removes |
| Walls (`tests/walls.rs`) | `helical_finishing_meets_walls_in_whole_moves` | 8 wall moves against 3 expected |
| Roughing (`tests/roughing_region.rs`, `spiral_rings.rs`, `ramp_follows.rs`, `adaptive.rs`) | 4 tests | Roughing on a selected region no longer reduced the indexed cutting; rings, a free end's nub plunge, and indexed lines at their own A change |
| Leads (`tests/lead_follows.rs`) | Helical rings on the blisk: each end's lead | Leads change with the passes |
| Exact outline drop (`tests/brep_outline_drop.rs`) | Drops on a tilted plane match the closed form | The scratch build applied P2 to every point near the rim in the exact outline drop, so a flat end on a slope, which touches at its rim, dropped up to 1e-6 mm lower. A built P2 would apply the rule per face, not per point; that is extra design work |

**Why.** Some strategies use the flush contact on purpose. Decision 0297's helix turns once at each face across the axis where the tool's side touches it, and finds that face with the drop. Free-end facing and the wall moves of helical finishing rely on the same contact. P2 is mathematically the stricter drop: a part that only touches the rim does not stop a tool moving along its axis. But those strategies would need another way to find a face the side touches before P2 could be adopted.

**Other risks of P2** (from the code, not measured):
- **Gaps:** a tool placed exactly flush with a wall by construction (a grid position at the wall's X less the radius, as in the first-cut project) would no longer stop at the wall's top.
- **Gouges:** at most 1e-6 mm into a part that only touches the rim, across the axis. This is far under every tolerance, and the simulation's 0.1 mm cell cannot see it.
- **Under-cut:** none expected beyond the facing lost above.
- **Determinism:** unchanged in kind. P2 is a fixed rule, so the same input gives the same output at any thread count, and the existing 1, 2 and N thread tests passed with P2 except where the output itself changed.
- **P2 alone does not remove pencil's own knife edge in general.** The rest's bisection would find the new boundary (1e-6 mm inside the wall) instead. On this rotationally symmetric part that boundary depends on X alone, so P2 alone fixed the closed form: one pass at 360 degrees, off and on, nothing lifted. On a part whose wall is not square to X (a blisk blade), the bridge points between two rests at that boundary can still fall either side of it. P1's clearance of a tenth of the tolerance keeps them on one side.

**Cost of P2.** Two golden sets re-blessed (`first_cut` on both posts, so the gate 3 replay and the gate 5 programs change; the blisk helical end-to-end programs). A redesign of how decision 0297's face turns, free-end facing and helical wall moves find a face the tool's side touches. Per-face rules in the exact outline and ball drops. Accuracy re-measured where decisions 0211, 0277 and 0338 measured it. P1 costs one line in `pencil.rs` and changes pencil only.

**The lane recommended P1, and the owner chose P1 only (23 Nov 2026, relayed).** P2 is rejected, not deferred: the shared drop keeps counting a part that touches the tool's rim, as the strategies above rely on. The measurements stay here as the reason.

### Q3. Single-point passes: **answer D2, with a shortest-cut option**

| Option | What | Output change |
| --- | --- | --- |
| D1 (the lane's first recommendation): extend to a tool diameter | Extend a short stretch along the crease | A pass always cuts along the corner; at most a tool diameter more feed per stretch |
| **D2 (chosen), with the owner's option** | A one-point stretch is never cut; a stretch shorter than *Shortest cut* (default the tool's diameter) is not cut; the note says how many and where (item 3) | Material found at those places stays; the note lets the user check it (NFR-12); fewer pecks |
| D3: leave them | — | A plunge and straight out, which cleans only the tool's footprint |

### Q4. What Exact STEP surface contact means for pencil: **answer E1, the row in *Advanced*, the tooltip stating the cost**

| Option | What | Notes |
| --- | --- | --- |
| **E1 (recommended; chosen): stays off by default; the words say it changes how closely the tool sits, not which stretches are cut** | Item 4; the row moves to *Advanced* for every strategy that has it, with the tooltips of item 4 | About 16 times slower on the closed form after the fix, 31 on the blisk (OI-1351): over the 2-times limit, so decision 0526 item 8 still applies |
| E2: on by default for pencil | Needs the owner to waive OI-1351's limit for pencil | A saved project keeps what it holds (schema 27); a new pencil operation on a STEP part would take seconds more |

### Q5. The 0.55.0 known-issue line about #632: **answer K2, amended in this pull request**

| Option | What |
| --- | --- |
| K1 (the lane's recommendation): amend the line in phase 2's pull request | One pull request changes the code and the words |
| **K2 (chosen): amend the 0.55.0 line now** | Done in this pull request (item 5), with no figures |

## Phase 2: what is built after acceptance (OI-9942)

One code and test pull request, as a draft, from `main`. Tests are written from OP-09 and this record first, and watched failing. The repeatable experiment becomes the first test.

1. **Tests first** (`openchuck-sim` `tests/rest.rs`, which already runs pencil against simulated stock with no kernel; the closed-form `shouldered_shaft`, an axial tapered-ball finish, then pencil with a 2 mm flat end):
   - `req_op_09_req_op_27_a_flat_end_cleans_the_whole_sharp_corner_with_exact_contact_off_and_on`: one closed pass around the corner with each setting, the same stretches. It fails today: off refuses, on gives 31 passes.
   - `req_op_09_req_nfr_12_pencil_rests_clear_of_a_wall_and_never_lifts_onto_it`: every cutting tip at the bar's radius and within the tolerance short of the wall, none on the shoulder. It fails today with exact contact on (262 lifted points).
   - `req_op_09_material_left_does_not_depend_on_where_a_flat_end_touches`: the same wanted stretches whichever point of a line contact the drop reports.
   - `req_op_09_req_nfr_12_stretches_shorter_than_the_shortest_cut_are_left_and_said`: a single point and a stretch shorter than *Shortest cut* are not cut; the note gives their count and places. At 0, every stretch longer than a point is cut. Empty means the tool's diameter.
   - The simulation after pencil: no overcut, and the worst material left in the corner lowered.
   - `req_nfr_06_...`: identical at 1, 2 and N threads (the existing pencil determinism test extended).
   - In `openchuck-app`: the *Exact STEP surface contact* row is in *Advanced* (the existing `req_op_27_req_ui_07_...` test changed to say so), *Shortest cut*'s hint names the tool's diameter, and the tooltips read as item 4.
   - The existing pencil tests (`openchuck-path` `tests/rest.rs` and `tests/brep_pencil.rs`; `openchuck-sim` `tests/rest.rs`, including issue 217's tooth test) are kept. Any that changes is explained.
2. **The code:** `pencil.rs` (the cleared rest, the tool-based test, the shortest cut and its note, the new parameter) and `rest.rs` (`RemainingMap`'s query by segment). In `openchuck-path` `contact.rs`, the two descriptions. In `openchuck-app` `form.rs`, `exact_contact` in *Advanced*, and *Shortest cut*'s default from the tool.
3. **The STEP project as a regression case.** With the kernel built (`--features kernel-tests`, `openchuck-app` `tests/step_strategies.rs`), pencil on `shouldered_shaft.step` after the axial finish: off and on give the same passes, one around the corner. The OWNERCHECKS project's figures are re-run with the command-line tool and recorded in this record's Verification section. The owner's check for #632 is the same project, off and on (lane OWNERCHECKS re-issues it).
4. **Time:** the blisk pencil of decision 0211's `step_accuracy` example, off and on, before and after. OI-1351's figure for pencil is updated if it moves; the 2-times limit is not changed.
5. **Documents in the same pull request:**
   - The user guide (`user-docs/src/reference/operations.md`): the Pencil clean-up section (what it cuts, *Shortest cut*'s row, and that the exact setting does not change which stretches are cut), and the *Exact STEP surface contact* entry, now under *Advanced* and without figures.
   - The UI specification, section 11.4: the row's place.
   - The development-version release-notes line.
   - The `openchuck-path` and `openchuck-app` CHANGELOG entries.
   - The verification plan's OP-09 row, if its method changes (it should not).
   - The decision index, and `docs/open-items.md` (OI-9942 closed).
   - Screenshots: any user guide figure that shows the *Pattern and tolerance* section of a STEP operation is re-captured, with its `CAPTURED.toml` hash naming the branch commit. None shows a pencil path today.

## Alternatives considered

- **Judge material from the crease point** (the B-rep edge point the rest came from) in place of the tool's axis: simple, but the crease is not where a tool resting beside a wall cuts. In a V-corner the axis and the crease are close, but a flat end cuts a band its diameter wide, which the axis segment describes.
- **A larger reach** (for example two radii): hides cause A on this part, but cuts beside material that is not under the tool and still depends on the contact's arbitrary place.
- **Exact contact on by default** to make both settings agree: they do not agree today, and the setting is not the cause.
- **A fixed default for *Shortest cut*** (for example 1 mm): too long for a small tool and too short for a large one; the tool's diameter scales with it (item 3).

## Consequences

On acceptance:
- Phase 2 (OI-9942) builds items 1 to 4 with the tests above.
- Pencil output changes for every pencil operation (no golden, reference job or example project holds one).
- Decision 0526 items 2 and 3 are amended (the row's place and its words), and the UI specification's section 11.4 with them.
- No requirement, architecture, schema or dependency change. Library: expected not breaking (to be confirmed with the coordinator before the build).
- Documents: as phase 2 item 5; the 0.55.0 known-issue line is already amended here.

## Verification (of this proposal)

- **What was checked.** A scratch test in `openchuck-sim` (not committed), `cargo test --release -p openchuck-sim`, at `main` `286472a`, Linux x86_64, 4 vCPU:
  - The closed-form `shouldered_shaft(4)` in 26 mm stock from X −10, at a 0.1 mm dexel cell.
  - Axial finishing (stepover 0.5) with the check project's tapered ball on the tessellation, simulated.
  - Pencil (`PencilFinishing::on_brep`) with its 2 mm flat end, `min_remaining` 0.10, 0.15 and 0.20, exact drops off and on.
  - The nearest material sample to each placed point's contact and to its tool axis, from `RemainingMap::detect` with pencil's own spacing and reach.
  - The flat end dropped at X 10.99 to 11.0001 and A 0 to 5 degrees.
  - The changes of items 1 and 2, put temporarily into `pencil.rs` behind environment switches.
  - P2 put temporarily into `cutter.rs` (`vertex`, `edge`) and `brep/outline.rs` (`Outline::lift`) behind an environment switch. The workspace's tests were run with it on (`OC_P2=1 cargo test --release --workspace --no-fail-fast -- --test-threads 4`), and each failure was run again with it off on the same binaries.
  - Every change was reverted afterwards (`git status` shows only documents).
  - Results as in *What this lane measured* and Q2.
- **Figures.** The four SVG figures were drawn for this record and checked by rendering them in a browser.
- **Not verified.**
  - The STEP file itself (the OpenCASCADE kernel was not built).
  - The OWNERCHECKS project's 6 / 40 and 31 / 390 counts (relayed from lane FACEFLOW and the OWNERCHECKS check).
  - The blisk.
  - Item 3 on a part with short stretches (none arose on the closed form once items 1 and 2 were in).
  - The simulation of the changed pencil path (overcut and material left).
  - The STEP kernel tests with P2 (not built).

  These are phase 2's tests, except P2, which the owner rejected.
- **Records checks:** `cargo xtask records` and `cargo xtask trace` were run before the push.
- Generated G-code is the user's to verify before it is run on a machine (NFR-12); nothing here says a pencil path is safe.
