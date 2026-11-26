# Ramp entry across strategies: investigation notes (issues #786, #421)

- **Commit:** `main` d1674b7c5ba33e15227dff542814eeff1722a23e (OpenChuckCAM 0.63.0), clean tree.
- **Binaries:** `openchuck-cli` release built at d1674b7 (23:17Z, copied to `cli` here so a concurrent rebuild could not change it mid-run); `openchuck-step-worker` built release from the same tree (`--features kernel`, OCCT already in `target/OCCT`).
- **Method:** throwaway copies of the public example projects (`testdata/examples/baluster`, `blisk-start`, `crankshaft-simultaneous`) with operation and selection-set TOML files written by `op.sh` / `sel.sh`; each operation run alone by `each2.sh` (optionally after a Plunge roughing "Base" op so the finishing op sees carried stock), with `openchuck-cli regenerate` (refusals, notes) or `openchuck-cli simulate` (overcut against the part, cell 0.1 mm; "rc=1" = overcut beyond one cell). Raw logs: `*.each`, G-code in `out/`. Nothing in the repository was edited; `git status` clean.
- Tools: baluster/crank T104 3.175 flat, T204 3.175 ball; blisk T1 3 mm flat, T2 3 mm ball. Defaults unless stated (Ramp angle 3 deg, Allow single-point plunge off, Steeper ramp near axis off, max 15 deg, approach 1 mm, tolerance 0.01).
- Blisk face ids: 19 = blade wall, 20 = hub floor (the user guide's numbering; the notes confirm "face 19", "face 20").

## 1. Code map: who uses the shared entry

Every strategy goes through `strategy::generate_with` -> `assemble_limited` (`crates/openchuck-path/src/strategy.rs:587`) -> `Entry::enter` (`assemble/entry.rs:167`). No strategy builds its own entry; face flow's `assemble` (`face_flow.rs:3119`) only orders its passes. Per-strategy hooks:

- `Strategy::ramps_round_the_part` (`strategy.rs:338`, default true). Only swarf overrides it to false (`swarf.rs:2888`, decision 0645).
- `Strategy::limits_descent_feed` (0548): roughing (`roughing.rs:240`), Facing Flat/Round (`facing.rs:68`).
- Edge following refuses every style but Plunge (`edge_following.rs:830`): "edge following cuts below the surface, where a lead, ramp or helix entry cannot be checked against it; use a plunge entry".
- Radial drilling's schema offers only Plunge (`drill.rs:241-250`): `"ramp" is not one of plunge.`
- Facing End: passes start in air beside the face (`facing/end.rs:272`), so Ramp finds no stock over the first point and plunges through air (`entry.rs` `follow`, `here <= goal`). Facing also validates the path (`facing/validation.rs:316-366`).
- App: tool-cannot-plunge check on `straight_plunges` for Ramp/Lead (`crates/openchuck-app/src/operation.rs:1137-1151`).

### What Ramp does (entry.rs `ramp` :379, `ring_plan` :755), by the shape of the pass start (`assemble/pass_shape.rs`)

| Pass starts as | Ramp | Proof / check | 
| --- | --- | --- |
| Line | back and forth over the pass to its first rapid (`follow` :486, `path_ramp::back_and_forth`); a line shorter than the tool radius is followed on past it | floors = the pass's own heights (`emit` :631, dominance, 0543 s3). No drop on the part. |
| Partial ring | back and forth, A reversing | same (dominated) |
| Full ring (round_the_part) | round the ring before its first point (`ring_ramp::plan`) | floors = the ring's own heights (dominated) |
| Spiral (round_the_part) | round the axis at the start X (`ring_ramp::plan_spiral`), floors from `touch_height` (:1138), a drop of the grown tool on `input.surface` every `angle_step(head radius)` | **not dominated**; only a sampled drop on `input.surface` |
| Ring/spiral, tip on axis | descends while A turns (0543 point 2) | plunge, listed in `straight_plunges` |
| Ring/spiral, swarf | back and forth over the pass (0645) | dominated |
| Single point | refused unless Allow single-point plunge (D7) | plunge |
| Stretch < 10 x tolerance (0.1 mm) | refused, "Set Entry to Plunge or Helix", **no option** | - |
| >10 turns | refused unless Steeper ramp near axis (and then up to the maximum angle) | - |

Lead and Helix: `gouge` (:1183) drop on `input.surface`, plus the faces next to the face set for swarf only (`Entry::new` :122, 0682).

### What `input.surface` is (app `operation.rs:743-751`)

- Mesh part: the whole mesh (a triangle selection limits the region, not the surface), plus held regions/tabs (`CompositeSurface`).
- STEP part with a face selection: **only the selected faces** (`BrepSurface::with_faces`), except face flow, which gets the whole part. Plus held regions/tabs.
- So on STEP, a spiral ramp, a lead and a helix of helical, axial, indexed, simultaneous, pencil (no faces), rest (no faces allowed) limited to faces are checked against the face set only (the #572 family). The dominated ramps need no part check.

### Stock the ramp starts above (D5)

Roughing carries stock per layer; every other strategy uses the stock at the operation's start (OI-2111). After an earlier op the app gives what it left (`operation.rs:940-952`).

## 2. Strategy table

| Strategy | Entry options | What Ramp does today | Checked against | Code |
| --- | --- | --- | --- | --- |
| Rotary roughing, continuous around (spiral) | P/L/R/H + single-point, steeper, face nubs | spiral ramp round the axis per layer; partial rings back and forth; end rings descend on axis | spiral: sampled drop on input.surface (whole part for roughing, no faces); others: pass floors | shared, entry.rs :379/:755 |
| Roughing, Rings | same | ring ramp / back and forth (partial rings dominate on baluster) | floors | shared |
| Roughing, along (lines) | same | back and forth along the line; end rings round the axis | floors / spiral drop | shared |
| Roughing, indexed | same | back and forth at fixed A | floors | shared |
| Helical finishing spiral / rings | P/L/R/H + single-point, steeper | spiral ramp / ring ramp; end-face ring on axis | spiral: drop on input.surface (face set only on STEP with faces) | shared |
| Axial finishing | same | back and forth along X; its end rings round the axis | floors / spiral drop | shared |
| Indexed finishing | same | back and forth at fixed A | floors | shared |
| Simultaneous spiral / axial | same | spiral/ring ramps, back and forth on partial rings | spiral: drop on input.surface (face set only on STEP) | shared |
| Rest machining spiral / rings | same (no face sets) | as helical | whole part | shared |
| Pencil | same | creases that turn A and drift X are "spirals": ramp round the part | whole part, sampled | shared |
| Face flow | same | passes that turn A are "spirals": ramp round the part, through other features | whole part (app exception), sampled | shared; face_flow keeps `ramps_round_the_part` = true |
| Swarf | P/L/R/H | back and forth along the face (0645) | floors (ramp); lead/helix: face set + neighbours (0682) | shared, swarf.rs:2888 |
| Edge (and curve) following | Plunge only (others refused) | - | - | edge_following.rs:830 |
| Facing Flat / Round | P/L/R/H | Flat: back and forth; a single-point pass refuses. Round: rings | floors | shared |
| Facing End | P/L/R/H | plunges through air (passes start beside the face) | facing validation | facing/end.rs:272 |
| Radial drilling | Plunge only | - | - | drill.rs:241 |

## 3. Results on public models

### Baluster (STL, held 8 mm at the start, stock 24 mm)

Defaults (Ramp only, nothing else on), op alone:

| Op | Result |
| --- | --- |
| Roughing (usual = around), along not tested at defaults | refused: "pass 3 is a single point, so there is no path for the ramp to follow. Turn on Allow single-point plunge, or set Entry to Plunge or Helix." |
| Roughing Rings | refused: pass 49 single point |
| Roughing indexed | refused: pass 299 single point |
| Helical spiral / rings, Axial, Rest spiral / rings, Sim axial | refused, 10-turn bound near the axis (2.0 mm 16.4 turns; 2.935 mm 10.4; 3.019 mm 10.0; 1.431 mm 30.2) |
| Indexed finishing | refused: pass 216 single point |
| Simultaneous spiral, Pencil, Facing End, Facing Round | generated |
| Facing Flat (size 10) | refused: "pass 5 is a single point" |

After a Plunge roughing op (realistic), defaults: helical spiral (pass 80), rings (164), axial (514), rest (207), rest rings (4) all refused at "1.000 mm from the rotary axis ... 10.2 turns"; sim axial pass 32 1.431 mm 12.3 turns; indexed pass 216 single point. **Every finishing strategy but the simultaneous spiral and pencil refuses Ramp at its defaults on the baluster.**

With Allow single-point plunge + Steeper ramp near axis, after roughing, simulated: helical spiral/rings, axial, indexed, rest rings, sim spiral, pencil, roughing along: all generate, overcut <= 0.050 mm (one cell; Base alone 0.0396). Ramp-entered ops add many moves (axial 392k moves; indexed 1.77M).

- **Steepen gives up early (defect):** Sim axial after roughing, both options on: "pass 32 turns round the part with its tip 1.431 mm from the rotary axis, so at 4.09 degrees a ramp round it would take 10.0 turns, more than the 10 allowed. Raise Ramp angle, or turn on Steeper ramp near the axis, or set Entry to Plunge or Helix." The angle is far below the 15 deg maximum and the message asks to turn on a setting that is on. `Entry::steepen` (entry.rs ~1016) scales tan by turns/10 x (1+1e-6) four times; `ring_ramp::plan` adds a whole turn (`delta += whole`) where the pass rises, so turns do not fall as 1/tan and the loop stops short.
- Roughing around (spiral) with single-point on: refused pass 120, 0.746 mm, 19.4 turns; Plunge to remove face nubs does not help (the part fills the core there); Steeper on: generates (5.81 deg). Roughing Lead: refused (lead-out cuts 0.039 mm into the finished surface, pass 107). Roughing Helix: refused (runs over the held region at X 26.516). Only Plunge works at defaults on roughing.

### Blisk (STEP, faces 19 wall, 20 floor; no roughing first, full 42 mm oak stock)

| Op | Result |
| --- | --- |
| Swarf wall, Ramp | generated; back and forth along the face, 3 A reversals; simulate same overcut as Plunge (0.0558 both) |
| Swarf Lead / Helix | refused, would cut 11.046 / 10.578 mm into the finished surface (the face itself) |
| Face flow floor, Ramp | generated, **simulate fails: 0.379 mm overcut at (77.782, 5.109, 19.335), 851 samples > 0.1 mm** (Plunge: 0.0455, passes). G-code: each pass entered by a ramp round the part at its start X, up to 4.4 turns (A 1584.6 down to the pass) at F3-F7, over the blades, at X 60.223 machine (0.23 mm past the blades' end face) |
| Pencil (whole part), Ramp | generated, **simulate fails: 0.542 mm overcut at (77.990, 19.925, 1.000), 5254 samples** (Plunge: 0.0483). G-code: ramp round the part at fixed X (A 461.8 -> -143), 1.7 turns over the blades |
| Face flow wall, Ramp | generated (not simulated) |
| Simultaneous wall (face 19), Ramp defaults | generated (23 passes back and forth, 4482 A reversals; the rest spiral ramps); **simulate fails: 0.6843 mm at (71.149, -17.781, 9.151)**, 10 places 0.31-0.68 (Plunge: 0.0485) |
| Simultaneous floor (face 20), Ramp defaults | generated; **simulate fails: 0.4915 mm at (76.284, 4.279, 19.534)**, 10 places 0.25-0.49 (Plunge: 0.0460) |
| Helical floor, Ramp (+ single point, steeper) | refused: "pass 93 starts with a stretch of 0.096 mm, shorter than the 0.100 mm (ten times the tolerance) a ramp needs to go back and forth over. Set Entry to Plunge or Helix." |
| Axial floor, Ramp | refused: stretch 0.064 mm (pass 5) |
| Indexed floor, Ramp | refused: stretch 0.077 mm (pass 72) |
| Helical floor Lead / Helix | refused, 4.397 / 4.969 mm into the finished surface |
| Roughing whole, Ramp defaults | refused: pass 1 single point |
| Roughing whole, around / rings / along / indexed with single-point + steeper | all generate (rings: 1769 passes back and forth, 20795 A reversals) |
| Rest / Edge with a face set | refused by design (whole part only) |
| Drilling, Ramp | "\"ramp\" is not one of plunge." |

### Crankshaft (STEP, held 6 mm, stock 38 mm, no faces)

| Op | Result |
| --- | --- |
| Roughing around, Ramp, defaults / +single point | refused: pass 520, 4.000 mm, 12.1 turns |
| + steeper | refused: pass 628, 1.000 mm, "even at the Maximum ramp angle of 15.00 degrees ... 11.3 turns" |
| Roughing along, single point + steeper | generates; simulate passes (0.0427) |
| Simultaneous spiral, Ramp defaults | refused: pass 3, 4.621 mm, 12.4 turns |
| Simultaneous spiral, + single point + steeper | generates (32 passes steepened to 6.53 deg), **simulate fails: 0.4234 mm at (32.990, 7.123, 7.660)** (Plunge: 0.0441) |
| Helical spiral, + single point + steeper | generates (828 passes steepened to 7.91 deg; 852k moves vs 62k), **simulate fails: 0.2606 mm at (55.168, 4.072, 3.702), 10 places 0.16-0.26** (Plunge: 0.0455) |

## 4. Diagnosis

1. **Spiral ramps gouge (sev2 candidate).** The only non-dominated ramp, D2 (a), turns round the axis at the start X with floors from a drop sampled at the A step of the tip's own radius (`angle_step(head radius)`), and follows steep floors (D4 answer A). Where the part round the axis at that X is not a surface of revolution (blades, crank webs and pins), the straight moves between samples cut it: 0.26-0.68 mm on two public STEP parts and four strategies (face flow, pencil, helical, simultaneous; simultaneous at its defaults on the blisk face sets). Swarf avoided it only through 0645's opt-out; face flow and pencil, whose passes turn A without cutting all round, did not get it. On a STEP face set (helical, simultaneous) the drop is on the face set only.
2. **"Ramp round the part" is used by strategies whose passes do not cut all round** (face flow, pencil, simultaneous on a face set): 4.4 turns over the blades at F3 to enter a floor pass.
3. **Needless refusals:** (a) single points refuse by default in roughing, indexed finishing and Facing Flat, while Lead plunges them with a note (0544 L4) - the neighbour styles disagree; (b) 10-turn bound refuses almost every finishing op on the baluster at defaults (end rings 1 mm from the axis) - Steeper ramp near the axis is off by default; (c) stretches under 0.1 mm refuse with no option (Allow single-point plunge does not cover them) - axial, indexed, helical on the blisk floor; (d) steepen stops short (above).
4. **Inconsistent between neighbours:** same op on STL vs STEP-with-faces checks spiral ramps/leads/helices against different surfaces; Lead and Helix refused on most of these cases, so Plunge is often the only working style.

## 5. #421 verdict

- OI-1270 retired and OI-1271 closed in `docs/open-items.md` (Closed table) by decision 0543 D10 / D7.
- Ramp on STEP parts generates: blisk roughing (around, rings, along, indexed), swarf, face flow, pencil, simultaneous; crankshaft roughing along (simulated, overcut 0.0427 <= cell) and helical/simultaneous (generate).
- Allow single-point plunge: off -> "Ramp entry (operation 'R-whole-ramp'): pass 1 is a single point, so there is no path for the ramp to follow. Turn on Allow single-point plunge, or set Entry to Plunge or Helix."; on -> "10 passes, from pass 1 to pass 1721 ... are single points with no path for a ramp to follow, so they are entered by a straight plunge, as Allow single-point plunge asks." (blisk STEP); same on baluster (268 passes, indexed roughing).
- Nothing of #421's scope is left; it can be closed with this evidence. The gouges and refusals found belong to #786 (they are not STEP-specific: pencil on the blisk has no face set, the crank has none).

## 6. Candidate rule sets

Common to all: keep 0543's dominated ramps (line, ring, partial ring, swarf) as they are; they simulated clean everywhere (swarf identical to Plunge).

**A. Follow the pass, never round the part (S-M).** Every ramp is a pose of its own pass raised along the tool axis: retire the spiral ramp (D2 (a)) for every strategy, i.e. spirals go back and forth over their own first turn up to the first rapid (0645 generalised, `ramps_round_the_part` removed or defaulting to false; rings keep the ring ramp, which is dominated). No part check needed for any ramp; tabs and held regions are already covered by dominance plus `check_clear`. Also: a stretch under 10 x tolerance is treated as a single point (Allow single-point plunge applies) instead of refused; fix `steepen`. Risk: low for safety (all ramps dominated); a spiral pass whose first turn is short at the axis still hits the 10-turn/leg bounds; A reversals on spirals (as swarf already). Golden impact: every saved operation whose pass starts as a spiral (helical spiral, roughing around, simultaneous spiral, rest spiral, pencil/face flow creases) changes; ring and line goldens unchanged. Public API: removes or changes the default of `Strategy::ramps_round_the_part` (a breaking change if removed: minor bump, 0048); no new types.

**B. Keep round-the-part only where the pass cuts all round, and prove it on the whole part (M).** `ramps_round_the_part` default false; true only for roughing, helical, rest and simultaneous when not limited to faces. The spiral ramp's floor is checked by the shared `Entry::gouge_moves` drop on the **whole part** (pass the model surface, not the face set, to `Entry`; the larger #572 fix), sampled at the A step of the largest radius the tool reaches at that X (not the tip's), and the moves between samples are checked (or sampled at half the chord); a hit refuses by name instead of following steep floors. Same whole-part check for Lead and Helix (closes #572 for every strategy). Defaults: Allow single-point plunge on for roughing and Facing (matches Lead's plunge); Steeper ramp near axis on by default for new operations (0545 D3 pattern). Risk: medium (a sampled check is still a sampled check; more refusals where the floor is not round). Golden: face flow, pencil, face-set finishing spirals change; plain finishing/roughing spirals change only where refused before or sampled differently. API: `StrategyInput` or `generate_in_region` gains a whole-part surface (or `RegionConstraint` carries it): additive or breaking depending on form; one trait default change.

**C. Per-strategy entry policy (L).** Each strategy declares which entry styles and ramp shapes it allows (an enum: FollowPass, RoundTheAxis, PlungeOnly), shown in the form (styles not allowed greyed with the reason, as drilling and edge following already do ad hoc), plus B's whole-part check for every non-dominated move, plus a per-operation "ramp fallback" (refuse / plunge / helix) for passes the ramp cannot enter (single points, short stretches, >10 turns) with a note naming them. Risk: medium-high (UI, schema, guide, many messages). Golden: as B plus form/schema; may need a project schema number if the fallback is stored. API: new public enum and trait method (additive), `ramps_round_the_part` deprecated.

Recommendation for the record: A now (it removes every gouge found, is small, and matches the owner's principle in 0543/0645 "a ramp follows the toolpath"), with the refusal fixes (stretch < 0.1 mm as a single point; `steepen`), and the defaults question (single-point plunge, steeper near axis) put to the owner as its own item; B's whole-part check for Lead/Helix as a follow-up (#572 family).
