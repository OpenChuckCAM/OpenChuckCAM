# 0711: Round facing's links that turn A rise above the whole setup first (OP-24, OP-14, NFR-07, NFR-12; issue #880; lane PATH-QUALITY)

- **Status:** Proposed by lane PATH-QUALITY on 28 Nov 2026. It records the owner's answer on issue #880 ("880 A": Round's links that turn A first rise above the setup's enclosing radius plus the margin, then turn A, then come down; whole-turn rings unchanged; the check's strict proof stays), relayed by the coordinator (decision 0049). Only the project owner accepts this record (decision 0049).
- **Date:** 28 Nov 2026.
- **Issue:** #880 (Round facing with the Part extent refused by Facing's rapid-clearance check). This record settles the link part. The Baluster example's remaining refusal, from Round's completion check, is issue #996.
- **Requirements:** OP-24 (Facing), OP-14 (links), NFR-07, NFR-12.
- **Amends:** nothing. Facing's rapid check (`facing/motion.rs`, `verify_rapid_clearance`) is unchanged.
- **Open items:** none added.

Numbering: decision 0711, provisional: the coordinator gives the number at the release push. No project schema number is needed.

## Context

Facing's rapid check proves a rapid that turns A clear only when it is above the whole setup's enclosing radius (the stock, the part and the held regions, each bounded by the corner of its outline across the axis) or off the stock along X. Only fixed-A rapids are measured against the stock already removed.

Round over the Part extent on box stock cuts arcs where the corners stand above the finished size. The assembler links one arc to the next at a height just above the stock the operation has already turned, so it turned A inside the setup's reach. That is probably clear in reality, but the check cannot prove it, so the operation was refused with "rapid move N has no continuous complete-tool separation from stock, the whole part and protected geometry".

On the Baluster's round bar (Round, Finished size 22, Part extent, held start), the refused rapid was the same kind: a link turning A from 745° to 755° at 13 mm from the axis, against an enclosing radius of 16.97 mm (the corner of the part's square outline).

## Decision

1. **A pass may say how far from the axis a link into it must be before it turns A** (`Pass::turn_above`, a new public field). The assembler's link rises to that height first, turns A there, and comes down to the pass. A link that does not turn A, or is already above that height, is unchanged. This holds for the link raised to the retract height when a low link is refused, too.
2. **Round facing gives every pass the whole setup's enclosing radius plus the approach distance.** That is above the height at which the rapid check proves a turning rapid clear, by the same gap as any other link above the stock. Whole-turn rings, which do not turn A between passes, and links already above it, are unchanged.
3. **The check is unchanged.** Option B of the plan, teaching the check the stock Round has already turned, was not chosen: the strict proof stays.

## Verification

- `openchuck-path`, `facing_held_limit::req_op_24_req_nfr_07_round_on_box_stock_turns_a_above_the_setup_between_its_arcs`: the issue's minimal case, Round over the Part extent on box stock X 5 to 25, ±4 by ±5, with no grip. It was seen failing first with the rapid refusal. It now generates, and every rapid that turns A is at least √(4² + 5²) + 1 mm from the axis. At least one such rapid exists.
- The existing Round tests on cylinder stock (`facing_held_limit`, the Facing suites in `openchuck-app`) pass unchanged.
- The Baluster: with this change the rapid refusal is gone, but Round's completion check (`facing/validation/legacy.rs`) then refuses it ("round has an unfinished axial/angular strip", or "raised/off-axis Round composite-target completion is not checked"). That is issue #996, under investigation, so no Baluster test is in this change.

Not verified: no air or wood cut. Generated G-code remains the user's to verify before it is run on a machine (NFR-12).
