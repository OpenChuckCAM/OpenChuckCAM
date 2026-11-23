# 9902: Swarf's reach past the end of the rulings is said after the lift, as information (OP-07, OP-12, NFR-12, UI-12; issue #624; lane SWARF-LEADS)

- **Status:** Proposed by lane SWARF-LEADS on 23 Nov 2026, with option (c) built as the recommendation; the choice is the owner's (OI-9902) and is queued by the coordinator. The number 9902 and OI-9902 are provisional: the coordinator assigns the real ones at release.
- **Date:** 23 Nov 2026.
- **Requirements:** OP-07 (swarf), OP-12 (gouge checks), NFR-12 (the user verifies the program), UI-12 (messages).
- **What it amends:** the message inventory row P75c (`docs/message-review/path.md`; decision [0536](0536-message-levels-operation-reference-and-setting-links.md) step 2 left it to this issue), and the note sentence of [0491](0491-swarf-keeps-the-tip-off-a-floor.md) item 2, which was measured before the lift.
- **Builds on:** [0491](0491-swarf-keeps-the-tip-off-a-floor.md) (the lift clear of avoid and check sets), [0532](0532-swarf-keeps-off-the-faces-at-its-ends.md) item 3 (every face next to the cut face is measured and a cut into it warned of).
- **Open items:** OI-9902 (new, provisional: the owner's choice).

## Context

Each swarf face's note (information) says how far the tool's end reaches past the end of the rulings at the tip: its corner radius plus the tip extension, a constant for the face, measured before any lift. The message review classed that sentence (P75c) as a warning, since it may cut a floor. Split out as a warning it fired with the floor in an avoid set and the ball lifted clear of it, which `swarf::req_op_02_req_op_07_req_op_12_a_floor_in_an_avoid_set_lifts_a_ball_tool_instead_of_refusing` rightly forbids (issue #624, lane MSGREVIEW). A lift can also cover only part of the face.

Since decision 0532 item 3, with the faces next to the face given (the application always gives them on a STEP part), a real cut into a floor that shares an edge with the face is measured and warned of, naming the face and the depth.

## Options put to the owner

- **(a)** A warning on the reach after the lift (the largest left at any pose). It warns on every face where a ball's end hangs over nothing (an open end), and repeats 0532's measured warning where a floor is there.
- **(b)** A warning giving the length along the face where the end still reaches past the rulings after the lift. The same costs, with a length.
- **(c) (recommended, built)** Information, said after the lift. The measured warning of 0532 item 3 stays the warning for a real cut. Cost: a floor that does not share an edge with the face (OI-2080) is not measured, so only this sentence mentions it.
- **(d)** As (c), but a warning, as (a), when the faces next to the face set were not given (library callers without them); in the application the same as (c).

## Decision (as built for option (c); pending the owner)

1. With no lift, the sentence is unchanged.
2. Where the tip is lifted at every pose by at least the reach: "The tool's end would reach {d} mm beyond the end of the rulings at the tip (its corner radius and the tip extension); it is lifted at least that far all along the face, so it ends level with them or short of them."
3. Where it is lifted less somewhere: "… ; where it is lifted least it still reaches {r} mm beyond them, so it cuts into any face that meets them there, such as a floor, unless that face is in an avoid or check set."
4. It stays a sentence of the face's information note; the inventory row P75c moves to TO-INFO.
5. No public API change (the least lift is read from the poses); no version bump on its own. NFR-12: the note does not say the floor is uncut; generated G-code remains the user's to verify.

## Verification

- `swarf::req_op_07_req_op_12_req_nfr_12_the_reach_past_the_rulings_is_said_after_the_lift` (`crates/openchuck-path/tests/swarf.rs`): a ball lifted clear of a floor in an avoid set all along gets sentence 2 and no "so it cuts into any face"; unlifted, the sentence is as before; a flat end 1 mm past the rulings over a floor rising along the pass, lifted only in part, gets sentence 3 with 1 minus the least lift; through the strategy, no warning. It fails with the change taken out (sentence 2 absent), and passes with it.
- The swarf tests of `openchuck-path` pass (53, uninstrumented).
