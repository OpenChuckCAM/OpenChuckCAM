# 9901: A swarf entry or exit that would cut a face next to the face set (OP-07, OP-11, OP-12, NFR-07, NFR-12; issue #572; lane SWARF-LEADS)

- **Status:** Proposed by lane SWARF-LEADS on 23 Nov 2026. The owner chose option B on 23 Nov 2026, relayed by the coordinator (decision 0049), and it is built; the owner's acceptance of the record is not yet given. The number 9901 and OI-9901 are provisional: the coordinator assigns the real ones at release.
- **Date:** 23 Nov 2026.
- **Requirements:** OP-07 (swarf), OP-11 (entry and exit moves), OP-12 (gouge checks where defined), NFR-07 (invalid input gives a clear error), NFR-12 (the user verifies the program).
- **What it amends:** [0532](0532-swarf-keeps-off-the-faces-at-its-ends.md) OI-2081 (the entry and exit moves are checked against the face set only), and the module rule of [0026](0026-entry-moves.md) item 7 only if option D is chosen.
- **Builds on:** [0532](0532-swarf-keeps-off-the-faces-at-its-ends.md) (the faces next to the face set, `RegionConstraint::neighbours`; a pass stops short of an end face; every face next to it is measured and a cut into it is warned of), [0645](0645-a-swarf-ramp-follows-the-pass-along-the-face.md) (a swarf ramp follows the pass, so it cuts no deeper than the pass against the whole part), [0544](0544-a-lead-on-a-pass-that-turns-a-stays-in-the-plane-of-the-cut.md) (the lead on a pass that turns A), [0536](0536-message-levels-operation-reference-and-setting-links.md) (message levels and setting links).
- **Open items:** OI-2081 (closed by this record: the entries and exits are measured against the faces next to the face set, and option B built); OI-9901 (closed: the owner chose option B on 23 Nov 2026, relayed); OI-2078 (unchanged: the sampling of the faces next to the face set, here also the sampling slack of option B).

## Context

A swarf operation's input surface is its face set (`BrepSurface::with_faces`, decision 0111). The shared entry and exit check (`crates/openchuck-path/src/assemble/entry.rs`, `Entry::gouge`, and `assemble/lead.rs`, `gouge_turning` for a pass that turns A) drops the tool on that surface only. So a lead-in, lead-out, straight ramp or helix is checked against the selected faces and never against the faces next to them, which decision 0532 gave swarf for its passes.

At an end of a face that stands against a wall, swarf stops the pass short so the tool just keeps off the wall (0532 item 1). A lead-out then carries the tool forward along the pass by the lead radius, into that wall, and nothing measures it.

![Seen down the tool axis at the end of a swarf pass: the pass stops short so the tool keeps off the end wall (face 1, next to the face set); the lead-out arc then carries the tool 2.990 mm into that wall, silently on main. Item 1 measures every lead and helix position against the faces next to the face set; under option B the lead-out is refused, naming face 1.](9901-swarf-entries-and-exits-meet-the-faces-next-to-the-face-set.svg)

### Measured (public fixtures only)

`crates/openchuck-path/tests/swarf.rs`: the limit wall (`y = 8`, X 22 to 38) with a square end wall at X = 38 as face 1 and/or a floor at Z = 10 as face 2, given as the faces next to the face set, tolerance 0.01 mm, the 6 mm flat end mill and a 6 mm ball end mill, Keep off the end faces on. The cut is the deepest cut of the entry and exit moves alone into the given faces, by the independent checker over each machine move (24 by 24 samples a face, 8 points a move), on `main` `0ff4b13`:

| Faces next to the face set | Tool | Lead, radius = tool radius (default) | Lead, radius 1 mm | Ramp | Helix |
| --- | --- | --- | --- | --- | --- |
| End wall | flat | **2.990, no warning** | **1.010, no warning** | 0.010 | 0.000 |
| End wall | ball | **2.990, no warning** | **1.010, no warning** | 0.010 | 0.000 |
| Floor | flat | 0.000 | 0.000 | 0.000 | 0.000 |
| Floor | ball | 2.924 | 2.824 | 2.750 | 2.803 |

- **The end wall** is the bug: every lead-out runs into it by about its radius, silently. The pass stops with the tool touching the wall (by the tolerance), so any lead-out that goes forward cuts it, whatever its radius.
- **The floor under a ball** is cut by the entries no deeper than by the pass itself: the ball's straight side starts a ball radius above its tip, so the pass cuts the floor about 3 mm, and 0532 item 3 already warns of that ("Swarf on face 0 cuts … into face 2, which meets it along the ends of its rulings …"). The entries add area to that cut, not depth.
- **The blisk** (`testdata/step/valid/blisk.step`, placed on the axis as the kernel tests place it; blades 7, 9, 13 and 19; the 3 mm flat end mill; tolerance 0.01 mm; the faces next to each blade given, as the application gives them; eight variants each: Lead at radius 1.5 (the default), 0.5 and 1 mm, Ramp at 3 and 30 degrees, Helix at 3, 30 and 89 degrees; 32 runs; release build). The cut is by the test kit's bounded checker (decision 0255) against every face of the blisk.
  - On `main` `0ff4b13`: 26 runs are refused because the entry would cut the blade itself (5.4 to 11.0 mm); the 6 made (Ramp on every blade, Helix at 89 degrees on blades 7 and 19) cut the blisk no deeper than their pass: 0.0268 mm against the pass's 0.0265 mm (the blade's twist) on blades 7 and 19, 0.299 mm against 0.299 mm on blades 9 and 13 (Ramp follows the pass, decision 0645).
  - With item 1 and option B: the same 26 refused and the same 6 made, with the same cuts. Only the reason given for two changes: the default Lead on blades 9 and 13 is now refused at its lead-in, which would cut face 8 (0.310 mm) or face 12 (0.329 mm) deeper than the pass, where on `main` its lead-out was refused on the blade (5.4 and 10.9 mm). So the blind spot is not reached on the blisk, as issue #572 found.

## Decision

1. **The entry and exit moves are measured against the faces next to the face set (built; not a choice).** For a strategy that is given them (swarf, `RegionConstraint::neighbours`), every lead (straight, and on a pass that turns A) and helix is measured against those faces as well as checked against the face set, before anything is written. The faces are sampled as swarf samples them for its passes (0532 item 1: at most an eighth of the tool radius apart, at least 0.05 mm, and the trimmed edge), and the cut at a position is the deepest of those samples inside the tool's cutting profile. (Dropping the tool along its axis, as the face-set check does, reads a wall the pass touches within the tolerance as the wall's whole height, so it cannot measure the pass's own cut.) A ramp that follows the pass (0645) needs no new check: every position is a pose of the pass raised along the tool axis. Other strategies are not given neighbours and are unchanged.
2. **What happens when one would cut such a face: the owner chose option B on 23 Nov 2026 (relayed; OI-9901). The options put to the owner:**
   - **Option A: refuse, as for the face set.** The operation is refused with the existing message, naming the move and the face ("pass 3: the lead-out needs an arc of radius 3.000 mm, and the tool would cut 2.990 mm into face 1, next to the face set, …; try another entry style …"), pointing at Entry.
     - Cost: a ball on a floor refuses every lead, ramp and helix (2.75 to 2.92 mm on the fixture), although the pass cuts the floor as deep and is only warned of (0532 item 3). A lead on a face that ends at a wall is refused with every radius.
   - **Option B (recommended; chosen and built): refuse, but allow the depth the pass itself cuts there.** As A, except that each face may be cut as deep as the pass cuts it at the pose the entry joins (its first pose for an entry, its last for an exit), plus the tolerance; that cut is already warned of by 0532 item 3.
     - Fixture: the lead-out at the end wall is refused (2.990 mm, against 0.010 mm allowed); a ball on a floor keeps its leads, ramps and helices as today; the flat end mill's ramp and helix at the end wall are unchanged.
     - Cost: Entry Lead cannot be used on a face that ends at a wall; the message says to use another entry style (Plunge, Ramp or Helix all fit there). The two cuts are compared on samples, and a cut between samples can be up to the spacing over the square root of two deeper than at them (the tool's clearance changes no faster than the distance), so the depth allowed is the pass's own cut, the tolerance and that slack: 0.27 mm with a 6 mm tool, 0.13 mm with a 3 mm one. A lead may therefore cut a face next to the face set up to that much deeper than the pass before it is refused; on the end-wall fixture the lead-out's 2.990 mm is refused, and on the blisk nothing made changes. A bounded check (OI-2078 for the pass) would close the gap. About 60 lines in the entry check.
   - **Option C: keep the move and warn.** The program is made with the cut; a warning names the pass, the move, the face and the depth and points at Entry, as 0532 item 2 does with Keep off the end faces off.
     - Cost: the 3 mm cut into the wall stays in the program; the user must read the warning. It undoes, for the lead-out, what Keep off the end faces is for.
   - **Option D: leave out the lead that does not fit, on that pass only.** A lead-in or lead-out that would cut a face next to the face set is replaced by a straight move along the tool axis (a plunge in, a retract out) on that pass, and an information note lists the passes and the face. Ramps and helices are refused as in B.
     - Cost: the entry changes style on its own for some passes, which decision 0026 item 7 rules out ("It does not change style on its own"); a pass of one cut point is already entered by a plunge (issue 199). About 80 lines.
   - **Option E: shorten the lead.** The largest radius up to the one set that keeps the face clear, found by bisection, with a note; where none does, B.
     - Cost: on a face that ends at a wall nothing clears (the tool already touches the wall at the stop), so it falls back to B on every such pass; it helps only where a wall stands further off than the stop. About 60 lines.
3. **Unchanged:** the pass itself (0532), the swarf ramp that follows the pass (0645), every other strategy and every entry on them, and Plunge on swarf.
4. **NFR-12.** Generated G-code remains the user's to verify before running it on a machine. Only the faces that share an edge with the face set are checked, against the tool's cutting part (0532 item 6); nothing says the entry is safe beyond that.
5. **Library (decision 0048).** Item 1 changes no public type: the faces reach the entry check through `generate_in_region`'s existing `RegionConstraint::neighbours`. Options A, B and E change only output and messages; C and D add a warning or note. None needs a minor version bump by itself; the output change is in the changelog.

## Consequences

- **Code:** `crates/openchuck-path/src/strategy.rs` (the neighbours passed to assembly), `assemble.rs` (`assemble_limited`), `assemble/entry.rs` and `assemble/lead.rs` (the check, and the chosen response).
- **Output:** swarf with Entry Lead, Ramp or Helix where a face next to the face set is cut by an entry or exit. No golden program uses swarf with these entries.
- **Documents:** the user guide's swarf and entry pages, the `openchuck-path` changelog, the release notes, OI-2081 and OI-9901.

## Verification

On branch `claude/lane-swarf-leads`, rebased on `main` `5e52b0b`. Linux, 4 cores, debug build, uninstrumented (`cargo nextest run`).

- `swarf::req_op_11_req_op_12_req_nfr_07_a_swarf_lead_out_never_runs_silently_into_an_end_wall` (`crates/openchuck-path/tests/swarf.rs`): on the limit wall with the end wall as the face next to it, flat and ball, Lead at the default radius and at 1 mm is refused naming the lead-out and face 1; without the faces next to it the lead is made as before; Ramp, Helix and Plunge are made and the independent checker finds the end wall cut by no more than the tolerance. It failed on `main` (the lead-out was made, 2.990 mm into the wall); 1.8 s.
- `swarf::req_op_11_req_op_12_a_ball_on_a_floor_keeps_entries_that_cut_it_no_deeper_than_the_pass`: a ball on a wall standing on a floor keeps its Lead, Ramp and Helix (they cut the floor 2.75 to 2.96 mm, no deeper than the pass, whose cut is warned of by 0532 item 3). It guards against option A's refusal; 1.1 s.
- The blisk (Context; `docs/archive/9901-swarf-entries-and-exits/`): the same 26 refused and 6 made as on `main`, with the same cuts by the bounded checker.
- The `openchuck-path` suite: 938 tests, all pass (the 18 that failed in the first run were the test binary rebuilt under the runner, and pass when run again), before the rebase; run again on the pushed head (see the pull request).

**The bounded check (the owner's note with the answer: "use the bounded check where you can").** The library's measurement uses face samples and the sampling slack (item 2, option B). The test kit's bounded checker (decision 0255) is a test dependency of `openchuck-path`, not a library one, so the refusal itself does not use it; every test above and the blisk measurement check the result with the bounded or independent checker. Moving a bounded check into the library is OI-2078, unchanged.

Not verified: no machine or air cut; no owner project (the owner's projects are private).

## Open items

- **OI-2081** (issue #572): closed by this record once item 2 is built.
- **OI-9901** (new, provisional number): the owner's choice in item 2.
