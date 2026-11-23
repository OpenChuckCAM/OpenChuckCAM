# 0xxx: Pencil clean-up rests clear of a wall it touches and judges material left from the tool, not its contact point (issue #632; lane PENCIL-REST)

- **Status:** proposed (23 Nov 2026) by lane PENCIL-REST, phase 1 (documents only). Nothing is built until the owner accepts the record (OI-9911). The owner's questions Q1 to Q5 below, each with options and a recommendation, were sent to the coordinator to put to the owner (decision 0049). The number is provisional: the coordinator assigns the final record and `OI-` numbers when the Merge Manager releases the pull request.
- **Date:** 23 Nov 2026.
- **Lane:** PENCIL-REST, branch `claude/lane-pencil-rest`, from `main` at `286472a` (library family 0.56.0). Issue #632 (milestone 2026.2), which refers to #351 and #257.
- **Requirements:** OP-09 (rest machining and pencil clean-up), OP-27 (exact surface contact for STEP parts), OP-23, NFR-06, GC-07, NFR-07, NFR-12. No requirement changes are proposed. `docs/requirements.md` and `docs/architecture.md` are not edited.
- **Builds on:** [0044](0044-slice-15-rest-and-pencil.md) items 1 and 5 (the remaining-material map, the pencil rest), [0162](0162-simulation-results-histogram-and-probe.md) item 6 (each pencil move checked against drops along it), [0226](0226-pencil-on-the-inside-corners-of-a-step-part.md) (pencil on a STEP part's inside corners), [0209](0209-step-geometry-for-every-strategy.md), [0211](0211-step-accuracy-measured.md), [0256](0256-no-fixed-toolpath-mesh-qualifies.md), [0338](0338-exact-outline-drops-not-adopted.md) (the exact outline drop for a flat end), [0526](0526-stepall-close-out-and-exact-contact-setting.md) (Exact STEP surface contact, off by default for pencil; OI-1351).
- **Open items:** adds OI-9911 (acceptance), OI-9912 (phase 2, the build) and OI-9913 (a flat end flush with a wall is a knife edge for every drop, in strategies other than pencil). Relates to OI-1351 (pencil's exact contact is over the 2-times limit) without closing it, and to OI-192 (pencil makes one pass per crease) without changing it.

Numbering: provisional ("0xxx"; OI-9911 to OI-9913 are numeric placeholders, because `cargo xtask records` refuses an `OI-` number that is not a number, chosen apart from lane STEPPED-CYL's OI-9901 to OI-9903), on the owner's rule of 23 Nov 2026 (reference numbers are monotonic in merge order; no reserved blocks). No project schema number: nothing stored changes. No spike.

## Context

Issue #632 (lane OWNERCHECKS, 8 Nov 2026): on the public `testdata/step/valid/shouldered_shaft.step`, turned along X in 26 mm stock from X −10 with 6 mm held, an axial finish with a 3.175 mm tapered ball (tip radius 0.5 mm, 5 degrees) and then pencil clean-up with a 2 mm flat end at its defaults (smallest material to clean 0.15 mm, crease angle 30 degrees, largest step 0.5 mm). With **Exact STEP surface contact** off (pencil's default, decision 0526) the operation makes 6 passes and 40 moves at the one sharp inside corner (X 12); on, 31 passes and 390 moves. The project is lane OWNERCHECKS' check for #351 and #257, `owner-checks/351-257-exact-step-contact/` on branch `owner-checks/2026.1-alpha` (synthetic, public model only). The branch named in the lane brief, `owner-checks/351-257-exact-step-contact`, is no longer on the remote.

The lane brief relays lane FACEFLOW's reproduction on current `main`. Off, pencil cuts 7 degrees of the 360 degree corner, and 3 of its 6 passes are single points (a plunge to the corner and straight out). On, it cuts 316 degrees in 31 stretches, none a single point. Lane FACEFLOW put both down to the remaining-material test sitting at its 0.15 mm threshold for the thin ridge a tapered ball leaves in a sharp corner, with a 0.0025 mm shift of the contact (tessellation against exact face) flipping it. This lane found no open or closed pull request by lane FACEFLOW for #632 (searched 23 Nov 2026), so that analysis is taken as relayed.

Records read: 0044, 0162, 0209, 0210, 0211, 0226, 0256, 0277, 0338, 0526. Open items read: OI-81, OI-189, OI-192, OI-835, OI-1350, OI-1351.

### How pencil decides today (code on `main`, `crates/openchuck-path/src/pencil.rs`)

1. **The rest.** At each sample along the crease the tool is dropped on lines across it. It rests where its tip is lowest. It is then moved towards the crease while it stays as low (within `REST_TOLERANCE`, 1e-5 mm), and the last such place is found by 40 bisections. Against a wall rising from a floor, that place is where the tool's side **just touches the wall**, to about 1e-12 mm.
2. **Moves between rests.** Each move is checked against drops along it (decision 0162 item 6). Where the tool would be lower than a drop allows, a point dropped half way is put in. Where eight halvings do not clear it, the pass ends there.
3. **Material left.** A placed point is cut only when `RemainingMap::near(p.contact)` holds: a sample of the part with more than `min_remaining` of stock above it lies within `reach` of the drop's **contact point**. `reach` is the tool's radius plus the sampling spacing (1 + 0.5 = 1.5 mm for a 2 mm flat end).

### What this lane measured

The experiment (scratch, not committed) used `openchuck_testkit::brep::shouldered_shaft` (the closed form of the same part, decision 0226), `BrepSurface` with exact drops off and on, the axial tapered-ball finish of the check project simulated on a 0.1 mm `DexelStock`, then pencil with the 2 mm flat end. Release build on `main` `286472a`, Linux x86_64, 4 vCPU. The OpenCASCADE kernel was not built (8 to 71 minutes), so the STEP file itself was not read: the closed form's tessellation (96 facets round, polygon trims) differs from the import's, and its counts differ from the STEP project's. It shows the mechanism:

| Case (closed form) | Off | On |
| --- | --- | --- |
| Pencil at `min_remaining` 0.10, 0.15 and 0.20 | Refuses: "found material left but the tool cannot rest in any corner that holds it", at each threshold | 31 passes, 621 points, 251 degrees of the corner, none single-point, at each threshold; 3.7 to 4.1 s |
| Where the drop puts the flat end's contact (X) | X 10.000: the far edge of the flat, 2 mm from the corner | Anywhere from X 9.99 to 11.63, varying with A |
| Nearest material sample to the contact | 1.556 to 1.779 mm, all beyond the 1.5 mm reach | 0.07 to 1.93 mm; 546 of 1025 points within reach |
| Nearest material sample to the tool's axis (tip to 10 mm up) | 0.556 to 0.780 mm, every point within reach | Every resting point within reach |
| Tool drops at X 11.0000 exactly (flat's side flush with the shoulder at X 12) | Rests on top of the shoulder (tip at radius 12) at most A, on the bar (radius 8) at some | The same knife edge, at different A |
| Tool drops at X 10.999 | On the bar at every A | On the bar at every A |
| Points of the passes lifted onto the shoulder's top (radius 12) | — | 262 of 621 |

So on the closed form neither setting is decided by the 0.15 mm threshold: 0.10, 0.15 and 0.20 give the same output. Two things decide it:

- **Cause A: material is judged from the contact point, and a flat end's contact is arbitrary.** A flat end on the bar touches it along a line across its whole flat bottom (the bar is straight along X under it). Which point of that line the drop reports depends on the surface: the tessellation reports the far edge (X 10), the exact faces anywhere between. The corner's material is 1 mm from the tool's axis but 1.5 to 2 mm from the far edge, right at the 1.5 mm reach. A ball end's contact in a corner is unique, which is why decision 0044's test (a ball, a 105 degree corner) never showed this.
- **Cause B: the rest is placed flush against the wall, on a knife edge.** The bisection of the rest stops where the flat's side touches the shoulder face. There, a drop of the same tool (the rest itself, or a bridge point dropped half way at a slightly different A) reads the graze either as clear (the tool on the bar) or as resting on the shoulder's top, 4 mm higher. The bridge check then finds the move "would cut the part", rides up onto the shoulder or ends the pass. That splits the corner into stretches and lifts points 4 mm. In the STEP project's 390 moves this is the likely source of the moves out and back. In the closed form, Cause A alone decides the off case, because the far-edge contact is out of reach at every point.

Both were confirmed by changing the scratch build, then reverted. A cleared rest (backed off the wall by 0.001 mm) and the material test from the tool's tip and axis, together, give one closed pass around the whole corner (360 degrees, 193 points, no point lifted) with exact contact **off and on alike**, at each threshold. Off took 0.11 s; on took 1.9 s (about 16 times, still over OI-1351's 2-times limit). Either change alone is not enough: with only the cleared rest, "on" still gives 31 passes and "off" still refuses. With only the tool-based test, both give one pass, but "on" lifts 448 points onto the shoulder.

Why the STEP project differs in counts (6 passes off, not a refusal): its tessellation is the importer's (0.01 mm deflection, exact circular trims), so some contacts land within reach. Phase 2 confirms on the STEP project with the kernel (Verification).

## Proposed decision (the lane's recommendations; the owner decides Q1 to Q5)

1. **Material is judged from the tool as placed, not from its contact point (Q1, option M1).** A placed point is cut when a sample with material left lies within reach of the **tool's axis over its cutting length**: the distance from the sample to the axis segment is at most the tool's cutting radius plus the sampling spacing (the same `reach`, measured from the axis). That is the material the tool's own body covers, widened by the sampling spacing. For a flat end it no longer depends on which point of a line contact the drop reports. It is not a superset of today's test: a ball's contact can lie up to its radius from the axis, so a sample just beyond the contact's far side counts today and not under M1. In a corner the material lies between the axis and the corner, so little change is expected for a ball; phase 2 measures it. `RemainingMap` gains a query by segment (`near_segment`, or the like); `near` by point stays for rest machining, whose contact is unique. This applies on mesh and STEP parts alike.
2. **The rest stays clear of a wall it touches (Q2, option P1).** After the bisection, the rest is backed away from the crease by a tenth of the operation's tolerance (0.001 mm at the default 0.01 mm): the material left on the wall is within the tolerance the user set. Bridge points are dropped from the same cleared line. The shared drop is not changed (that would change every strategy; see OI-9913).
3. **No single-point pass (Q3, option D1).** A stretch of wanted points shorter than the tool's diameter is extended along the crease, both ways equally where the crease allows, to the tool's diameter, so a pass always cuts along the corner. It is still checked against drops (item 2 and decision 0162 item 6). With items 1 and 2 the closed form has none, so this is a guard.
4. **Exact STEP surface contact stays off by default for pencil (Q4, option E1).** After items 1 and 2, on and off agree on where the corner is cut, as the experiment shows. The setting then changes only how exactly the tool sits on the faces (decision 0338: 0.0066 against 0.0342 mm deepest on the blisk), not which stretches are cut. It is still about 16 times slower on the closed form (31 on the blisk, OI-1351), over the 2-times limit. Its description and the user guide say that on and off cut the same stretches.
5. **No change to the threshold or the tessellation (option M3 not chosen).** On the closed form, `min_remaining` from 0.10 to 0.20 changes nothing, and a finer tessellation would not move a flat end's contact off the far edge.

### What changes in output

- **Golden files and reference jobs:** none. No golden program, reference job (`testdata/golden/reference`, `testdata/projects`) or example project (`testdata/examples`) has a pencil operation (searched 23 Nov 2026).
- **Pencil operations:** every pencil operation can change, on mesh and STEP parts, because of item 1 (points can be wanted or not where they were not or were) and item 2 (rests 0.001 mm further from a wall). With a ball end in a V corner (decision 0044's test), the rest touches no wall and the contact is unique, so the change should be nil or small. Phase 2 measures it.
- **The OWNERCHECKS project:** expected one pass around the corner at X 11 (less the 0.001 mm clearance), off and on alike, in place of 6 passes / 40 moves off and 31 passes / 390 moves on. Phase 2 measures it on the STEP file.
- **Cost:** in moves, one closed pass of about 2πr / 0.5 mm points (about 100 to 200 cutting moves at radius 8) with one entry and exit, against 40 moves (off, most of the corner missed) and 390 (on). In time: off 0.11 s, on 1.9 s on the closed form, against about 0.1 s and 4 s today. The extra material test per point is a few distance tests against the map's nearby cells.
- **Library:** expected not breaking. The changes are private to `pencil.rs`, plus an added method on `RemainingMap`. The coordinator is told before any version change.

### Risk

- Item 1 can stop cutting where material lay just beyond a ball's contact but outside the tool's body widened by the spacing, and can cut a little more where material lies beside the tool but not under its contact: for example at the ends of a blisk's crease, where a blade's material is within reach of the axis. Every point is still a dropped rest checked by decision 0162 item 6, so it cannot gouge; it adds air moves at worst. Phase 2 measures the blisk pencil (decision 0211's `step_accuracy` example) before and after.
- Item 2 leaves up to a tenth of the tolerance on a wall that pencil touches. This is within the tolerance and far under the 0.15 mm threshold.
- Item 3 cuts up to a tool diameter of corner where only one point found material: a few millimetres of feed at worst.
- The closed form is not the STEP import. The mechanism is general (any flat end on a surface straight under it, any tool flush with a wall), but the STEP counts are confirmed only in phase 2.

## The owner's questions (most important first)

### Q1. How pencil decides "material left" across a crease

| Option | What | Output change | Cost | Risk |
| --- | --- | --- | --- | --- |
| **M1 (recommended): from the tool** | Material within reach of the tool's axis over its cutting length (item 1) | Flat ends cut the whole corner where material is; ball ends little changed (measured in phase 2) | A few more distance tests per point | Can add short stretches beside material that is not under the contact (no gouge) |
| M2: smooth along the crease | Keep the contact test; close gaps shorter than a tool diameter and extend each stretch by a radius (hysteresis) | Fills the gaps between stretches it already finds, but not the 353 of 360 degrees the off case misses | Small | Cuts where nothing was found; does not fix a contact that is out of reach everywhere (the closed form's off case) |
| M3: change the threshold or the tessellation handling | Lower `min_remaining`, or a finer drop surface | None on the closed form (0.10 to 0.20 identical) | Finer tessellation: decision 0256's cost | Does not address the cause |
| M1 + M2 | Both | As M1, with any remaining gap closed | As M1 | As M1 and M2 |

### Q2. Where the rest sits against a wall

| Option | What | Output change | Risk |
| --- | --- | --- | --- |
| **P1 (recommended): back off by a tenth of the tolerance** | Item 2: 0.001 mm at the default | Passes stay on the bar; no point lifted onto the shoulder | Leaves at most 0.001 mm on the wall |
| P2: treat a graze as no touch in the drop itself | Change the shared drop (tessellation and exact) so a side exactly flush with a face does not count | Same for pencil; also changes every strategy's drops | Wide change to a shared, accuracy-measured function (decisions 0211, 0277, 0338) |
| P3: leave it | — | 31 passes, 4 mm lifts on the closed form with exact contact | The defect stays |

### Q3. Single-point passes

| Option | What | Output change |
| --- | --- | --- |
| **D1 (recommended): extend to a tool diameter** | Item 3 | A pass always cuts along the corner; at most a tool diameter more feed per stretch |
| D2: drop them, with a note | A one-point stretch is not cut; the note says how many were left and where | Material found at those points stays; the note lets the user check it (NFR-12) |
| D3: leave them | — | A plunge and straight out, which cleans only the tool's footprint |

### Q4. What Exact STEP surface contact means for pencil

| Option | What | Notes |
| --- | --- | --- |
| **E1 (recommended): stays off by default; the words say it changes how closely the tool sits, not which stretches are cut** | Item 4 | About 16 times slower on the closed form after the fix, 31 on the blisk (OI-1351): over the 2-times limit, so decision 0526 item 8 still applies |
| E2: on by default for pencil | Needs the owner to waive OI-1351's limit for pencil | A saved project keeps what it holds (schema 27); a new pencil operation on a STEP part would take seconds more |

### Q5. The 0.55.0 known-issue line about #632

The 0.55.0 release notes (`user-docs/src/release-notes.md`, Known issues) say "pencil clean-up with **Exact STEP surface contact** on can make many more passes at a corner (#632)". The finding is wider: with it **off**, the default, a flat end can leave most of a corner uncut.

| Option | What |
| --- | --- |
| **K1 (recommended): amend the line in phase 2's pull request, and add the development-version line for the fix** | One pull request changes the code and the words |
| K2: amend the 0.55.0 line now, in this documents pull request | Users of 0.55.0 learn sooner; the line would read: "pencil clean-up with a flat end can leave most of a sharp inside corner uncut, or, with **Exact STEP surface contact** on, cut it in many short passes; check the corner in the simulation (#632)" |

## Phase 2: what is built after acceptance (OI-9912)

One code and test pull request, as a draft, from `main`. Tests are written from OP-09 and this record first and watched failing. The repeatable experiment becomes the first test.

1. **Tests first** (`openchuck-sim` `tests/rest.rs`, which already runs pencil against simulated stock, no kernel; the closed-form `shouldered_shaft`, an axial tapered-ball finish, then pencil with a 2 mm flat end):
   - `req_op_09_req_op_27_a_flat_end_cleans_the_whole_sharp_corner_with_exact_contact_off_and_on`: one closed pass around the corner with each setting, the same stretches. It fails today: off refuses, on gives 31 passes.
   - `req_op_09_req_nfr_12_pencil_rests_clear_of_a_wall_and_never_lifts_onto_it`: every cutting tip at the bar's radius and within the tolerance short of the wall, none on the shoulder. It fails today with exact contact on (262 lifted points).
   - `req_op_09_material_left_does_not_depend_on_where_a_flat_end_touches`: the same wanted stretches whichever point of a line contact the drop reports.
   - `req_op_09_a_pencil_pass_is_never_a_single_point` (D1; or the note, under D2).
   - The simulation after pencil: no overcut, and the worst material left in the corner lowered.
   - `req_nfr_06_...`: identical at 1, 2 and N threads (the existing pencil determinism test extended).
   - The existing pencil tests (`openchuck-path` `tests/rest.rs`, `tests/brep_pencil.rs`; `openchuck-sim` `tests/rest.rs`, including issue 217's tooth test) are kept. Any that changes is explained.
2. **The code:** `pencil.rs` (the cleared rest, the tool-based test, the stretch rule) and `rest.rs` (`RemainingMap`'s query by segment). The application note and the setting's description change (E1).
3. **The STEP project as a regression case.** With the kernel built (`--features kernel-tests`, `openchuck-app` `tests/step_strategies.rs`), pencil on `shouldered_shaft.step` after the axial finish: off and on give the same passes, one around the corner. The OWNERCHECKS project's figures are re-run with the command-line tool and recorded in this record's Verification section. The owner's check for #632 is the same project, off and on (lane OWNERCHECKS re-issues it).
4. **Time:** the blisk pencil of decision 0211's `step_accuracy` example, off and on, before and after. OI-1351's figure for pencil is updated if it moves; the 2-times limit is not changed.
5. **Documents in the same pull request:** the user guide's Pencil clean-up section (`user-docs/src/reference/operations.md`: what it cuts and that the setting does not change which stretches are cut) and the Exact STEP surface contact row there; the setting's description (tooltip) if E1's wording changes it; the development-version release-notes line, and the 0.55.0 known-issue line (Q5); the `openchuck-path` and `openchuck-app` CHANGELOG entries; the verification plan's OP-09 row if its method changes (it should not); the decision index; `docs/open-items.md` (OI-9912 closed; OI-9913 updated). Screenshots: none show a pencil path today; if one is re-captured, its `CAPTURED.toml` hash names the branch commit.

## Alternatives considered

- **Judge material from the crease point** (the B-rep edge point the rest came from) in place of the tool's axis: simple, but the crease is not where a tool resting beside a wall cuts. In a V-corner the axis and the crease are close, but a flat end cuts a band its diameter wide, which the axis segment describes. Not recommended.
- **A larger reach** (for example two radii): hides Cause A on this part, but cuts beside material that is not under the tool and still depends on the contact's arbitrary place.
- **Exact contact on by default** to make both settings agree: they do not agree today, and the setting is not the cause (E2 is offered only after the fix).

## Consequences

On acceptance:
- Phase 2 (OI-9912) builds items 1 to 4 as chosen, with the tests above.
- Pencil output changes for every pencil operation (no golden, reference job or example project holds one).
- No requirement, architecture, schema or dependency change. Library: expected not breaking (to be confirmed with the coordinator before the build).
- Documents: as phase 2 item 5.

## Verification (of this proposal)

- **What was checked.** A scratch test in `openchuck-sim` (not committed), `cargo test --release -p openchuck-sim`, at `main` `286472a`, Linux x86_64, 4 vCPU:
  - The closed-form `shouldered_shaft(4)` in 26 mm stock from X −10 at a 0.1 mm dexel cell.
  - Axial finishing (stepover 0.5) with the check project's tapered ball on the tessellation, simulated.
  - Pencil (`PencilFinishing::on_brep`) with its 2 mm flat end, `min_remaining` 0.10, 0.15 and 0.20, exact drops off and on.
  - The nearest material sample to each placed point's contact and to its tool axis, from `RemainingMap::detect` with pencil's own spacing and reach.
  - The flat end dropped at X 10.99 to 11.0001 and A 0 to 5 degrees.
  - The two changes of items 1 and 2, put temporarily into `pencil.rs` behind environment switches, then reverted (`git status` clean).
  - Results as in *What this lane measured*.
- **Not verified.** The STEP file itself (the OpenCASCADE kernel was not built); the OWNERCHECKS project's 6 / 40 and 31 / 390 counts (relayed from lane FACEFLOW and the OWNERCHECKS check); the blisk; item 3 (no single-point pass arose once items 1 and 2 were in); the simulation of the changed pencil path (overcut and material left). These are phase 2's tests.
- **Records checks:** `cargo xtask records` and `cargo xtask trace` were run before the push.
- Generated G-code is the user's to verify before it is run on a machine (NFR-12); nothing here says a pencil path is safe.
