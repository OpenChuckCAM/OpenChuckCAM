# 0662: The lead-in move that meets the surface is timed for its own travel (OP-13, OP-11, NFR-12; issue #685; lane SIM-FINISH)

- **Status:** proposed (22 Nov 2026) by lane SIM-FINISH, built in the lane's pull request. It awaits the owner's acceptance (decision 0049).
- **Date:** 22 Nov 2026.
- **Builds on:** [0544](0544-a-lead-on-a-pass-that-turns-a-stays-in-the-plane-of-the-cut.md) item 3 (L3: the arc of a lead on a pass that turns A carries no contact point above the surface), [0034](0034-slice-12-baseline-finishing.md) (contact feed and timing).
- **Context:** OP-13 (feed and timing of simultaneous moves), OP-11 (leads). It changes no requirement, no architecture and no API.
- **Open items:** none new.

## Problem

On a pass that turns A, the lead-in's arc moves above the surface carry no contact point, so the move builder measures their length on the tool tip and leaves its contact reference at the tip. The last lead-in move, which meets the surface, carries the first cut point's contact point, so its length was measured from the tip before it to that contact point: it counted the jump from the tip to the contact point as well as the move. Where the surface slopes under a ball, the contact point is off the tip and the move was timed for the extra length. The issue measured about 0.8 mm extra (0.09 s) on one move of an owner check; it is never faster than the move needs, so no limit was exceeded.

## Decision

Before the move that meets the surface, the contact reference is set to where the first cut point's contact would be with the tool at the arc's last point: the arc's last tip plus the first cut point's offset from its tip to its contact (`crates/openchuck-path/src/assemble/lead.rs`, `turning_lead_in_moves`). The move's counted length is then the tip's own travel, and the contact reference after it is the first cut point's contact, so the cut after it is timed as before. A lead on a pass that holds A already starts its contact reference this way (`assemble/entry.rs`, the straight lead's `tip + offset`), so the two leads now agree.

The issue offered this and the other choice, timing that one move on the tip as the arc above it is; both give the same length (the tip's travel), so there is no choice for the owner to make in the numbers. This form keeps the contact reference continuous into the cut.

## Consequences

- **Changed output:** on a pass that turns A with Entry Lead, the F word of the lead-in's last move can rise where the contact point is off the tip (a sloping surface under a ball or bull-nose tool, or any tool where the contact is off its axis). Nothing else changes: positions, the arc, every other move's timing. Roughing, helical finishing, rest machining and the rings with a turning lead are the strategies that make such leads.
- No API change; no project schema.
- Generated G-code stays the user's to verify before it runs on a machine (NFR-12).

## Verification

### Tests

- `lead_follows::req_op_13_req_op_11_the_lead_in_move_that_meets_a_sloping_surface_is_timed_for_its_own_travel` (`crates/openchuck-path/tests/lead_follows.rs`): helical finishing in rings with Entry Lead on a cone 8 to 12 mm radius over 30 mm with a 6 mm ball; for every lead-in, the last move's counted length equals the tip's travel from the arc's last point. Written first: it failed on `main` (0.484 mm counted for 0.277 mm of travel) and passes with the change, 0.85 s uninstrumented.

### Runs

- The whole `openchuck-path` suite with this change and decision 0663's (before decision 0660's note change): 932 tests passed, 1 skipped, 1,166 s (`cargo nextest run -p openchuck-path`, debug build, uninstrumented, 4 cores, Linux), at the lane's working tree on `main` 57b8ac1b. The lead tests of `lead_follows.rs` (decision 0544) are among them; no golden program changed (the only reference job with a lead, `cone_axial_lead`, is axial finishing, whose passes hold A).
- Not verified: a program from an owner check (the issue's measurement was on the owner's check project, which this lane did not use); the change is shown on the synthetic cone above.
