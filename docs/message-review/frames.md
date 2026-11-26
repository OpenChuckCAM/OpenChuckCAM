# Message review: which frame each printed position is in

Inventory for the owner's decision of 26 Nov 2026 on issue OI-9998 ("every coordinate should be in a single reference frame", relayed by the coordinator) and the proposed [decision 0699](../decisions/0699-every-position-is-in-the-work-frame.md). It lists every place the application, the command line, the library crates and the user guide print a position along the rotary axis (or a point), and says which frame the number is in today. It was gathered by reading the code at `main` (17dcdded) and each variable's origin; it is a reading, not a test, and a row found wrong is corrected when the record is built. Lengths, distances, deltas and angles have no frame and are listed only to say so.

## The frames

| Name used here | What it is | Origin |
| --- | --- | --- |
| MODEL | The model file's own coordinates. Nothing prints these. | The file's origin |
| PLACED | The model after the Model step's placement steps, shared by every setup. | The placed model's origin |
| SETUP | The setup frame of [decision 0201](../decisions/0201-setup-offset-moves-the-part.md): X along the rotary axis, the part placed and then moved by **Offset along X**; the stock, the held regions, the tabs, the work zero and every toolpath are given in it. | The machine's rotary reference point (the chuck or headstock face, the profile's rotary reference) |
| WORK (the WCS) | What the G-code, the simulation's readout and the viewport's cursor readout use: SETUP X minus the setup's **Zero at X**; Y and Z are distances from the axis. | The work zero, the program's X0, which is the work axes' origin marker in the viewport |
| MACHINE | The machine's own coordinates: Rotary reference X plus SETUP X. Travel limits and the work offset register. | The machine's origin |

The work zero is a value of the setup (**Zero at X**, `work_offset.x`), measured from the rotary reference point, so WORK equals SETUP only when **Zero at X** is 0. The example projects and the tutorial set **Zero at X** to the part's offset (17.99 in the examples, 18 in the tutorial), so there the two differ by that much.

## Conversion points

| Where | From to |
| --- | --- |
| `openchuck-app/src/model.rs` `place` | MODEL to PLACED |
| `openchuck-types/src/setup.rs` `SetupFrame::place_part`, `PartExtent::in_setup`; `model.rs` `PlacedPart::in_setup`, `setup_part`; `scene.rs` | PLACED to SETUP (adds **Offset along X**) |
| `openchuck-types/src/kinematics.rs` `RotaryFrame::work`; `openchuck-app/src/posting.rs` | SETUP to WORK (origin moved by **Zero at X**) |
| `openchuck-post/src/convert.rs` | the SETUP tool tip to the WORK words of the program |
| `openchuck-app/src/playback.rs` (`pose.work`) | SETUP to WORK for the playback readout |
| `openchuck-ui/src/viewport_overlays.rs` `readout` | the cursor point (SETUP) minus the work-X marker to WORK |
| `openchuck-types/src/setup.rs` `along_check`; `openchuck-app/src/overview.rs` | SETUP to MACHINE for the travel messages |
| `openchuck-types/src/setup.rs` `expected_work_offset`; CLI `setup` | SETUP work zero to the MACHINE value of the register |
| `openchuck-app/src/posting.rs` (check limits) | MACHINE travel minus the zero to WORK limits for the G-code check |

## What prints which frame

| Frame | Sites | Where |
| --- | --- | --- |
| SETUP | about 66 | every position in a path warning, note or refusal (about 26); the setup, workholding and stock texts (types 5, app about 25), the command line (about 10), the Setup step's rows and the workholding panel (UI 4) |
| WORK | 5 | the viewport readout ("G54 work coordinates"), the playback line `X.. Y.. Z.. A..` (no frame word), the G-code check "X v is outside a to b", the tool-change "below the safe Z", the simulator comparison |
| MACHINE | about 11 | the travel and reference lines (CLI, machine view), the travel warnings' "machine X" half, the connection-position refusal |
| PLACED | 3 | the Position row's breakdown ("Placement X a to b + offset N mm"), the CLI `bounds` line, the inspector's note |
| NONE (a length, delta, angle) | about 35 | tab and grip lengths, "moved N mm", translate deltas, A angles, plane offsets |
| UNCLEAR | 2 | the unrolled-surface 2D point in `unroll.rs`, and the simulation's stored deviation points (taken to be SETUP) |

No message prints MODEL coordinates. G-code comments print no position (operation, tool and time only).

## openchuck-path (all SETUP except one MACHINE)

The path crate is given the part, stock, held regions and tabs in the SETUP frame (`openchuck-app/src/operation.rs` `.in_setup`), so every "X" in its messages is SETUP. The sites:

- `words.rs` `x_span` and its use in the unreached-area list (`simultaneous.rs`); the off-axis section zones, the drive between X a and b, the shank breaks "X x near A a", the tilt "(at X x)" and the refused move "cannot keep the move to X, A" (`simultaneous.rs`).
- Face flow's unreached list (`face_flow.rs`); the facing messages ("(X a to b)", "held start (X lo to hi)", "left unfaced", `facing/held.rs`, `facing/validation.rs`, `facing/end.rs`, `facing/end/routing.rs`).
- The finishing "end face at X" family (`finishing.rs`), shoulders (`faces.rs`), pencil, flutes, rest ("between X first and last"), the reach over a move (`strategy.rs`), entry (`assemble/entry.rs`), the round-the-axis list (`assemble.rs`), the edge-following and swarf points "at (x, y, z)" (`edge_following.rs`, `swarf.rs`), and the stock box (`stock.rs`).
- MACHINE: `program.rs` "connection position (x, y, z) exceeds the machine's linear travel".

## openchuck-types and openchuck-app (SETUP except where marked)

- Setup checks: a tab not at the head or tail; overlapping tabs; "starts at X x, n mm inside the headstock (X 0 is the headstock face on the Makera Z1)"; "grip ... covers ... of the part (X a to b)"; the travel fallback "at machine X v" (MACHINE).
- Workholding: tab and held-region texts ("tab n (X a to b mm)", "Held span X a to b", "Head zone X a to b mm", "Kept clear to X", "part comes to a point at X", "Not inside the stock (X a to b mm)", "cylinder or frustum X lo to hi").
- The setup sheet and setup form: stock "from X", box "X a to b", "work zero at model X", held and tab columns, the Position row and its "Placement X a to b + offset N mm" breakdown (PLACED), and the tooltips of **Start X**, **Offset along X**, **Position** and **Zero at X** (all describing SETUP).
- Facing display ("Target X", "X a to b").
- The overview's travel text: "at X value minus reference" is SETUP, and "would be at machine X v ... the machine reaches min to max" is MACHINE, in one sentence.
- Machine view and editor: "Travel X a to b", "Axis height (machine Z)", "G54 Z in machine coordinates (G53)", "Machine X of the rotary reference point" (MACHINE).

## openchuck-gcode, openchuck-post, openchuck-sim, the command line and the interface

- G-code checks and replay: "X v is outside a to b" and the tool-change Z are WORK (the check is given the machine travel minus the zero); the comparison's "needs a move to tip (x, y, z)" is the SETUP tip; no simulation message prints a position.
- Command line: the added default tab and the tab and stock lines are SETUP; "work zero: G54 on the rotary axis at model X" is SETUP though it says "model"; "bounds ... (model frame)" is PLACED; the reference, travel and "set G54 to machine X Y Z" lines are MACHINE; the "most material left" and "deepest overcut ... at (x, y, z)" and the rapid-move end are SETUP; the G-code check lines are WORK.
- Interface: the viewport readout and the playback line are WORK; the Position row, the workholding panel rows and the run panel's "Most material left ... at X Y Z" and "Overcut ... at X Y Z" are SETUP; the viewport's axes and labels carry no numbers.

## Wording that does not match the frame

- "work zero at model X" (setup sheet and CLI), "Model X of the work zero" (CLI help and the types doc) mean SETUP X.
- "model frame" in the CLI `bounds` line and in the `SetupReport.part` doc: the first is PLACED, the second is actually SETUP.
- The viewport readout says "work coordinates" even when no setup is chosen, when the point is PLACED and the zero is the origin.
- The playback line has no frame word but is WORK.
- The setup warnings quote SETUP or MACHINE numbers while the G-code check quotes WORK numbers for the same position.
- `setup.rs` hard-codes "headstock face on the Makera Z1" for X 0.

## User guide pages that print a position

| Page | What it prints | Frame |
| --- | --- | --- |
| `tutorial/04-stock.md`, `05-hold.md` | **Position** "X 18.000 to 79.000 mm", the stock "starting at X 17", the held region "X = 17 to X = 25" | SETUP (with **Zero at X** 18, WORK would read 0 to 61) |
| `concepts/stock-and-held-region.md` | the Where things sit along X section and worked example: the SETUP and MACHINE frames, with the travel message quoted | SETUP and MACHINE, named |
| `reference/command-line.md` | the `setup` options, "machine X" | SETUP and MACHINE |
| `reference/machine-profiles.md` | Rotary reference X, "model X 0 is the headstock face" | MACHINE and SETUP |
| `examples/*.md` (flute length, simultaneous finishing, swarf, tabs and held regions, create a post and others) | warnings and Results quoted with SETUP X ("at X 39.193, A 342.3", "end face at X 78.990", "X 22.078 to 27.950", "Most material left: ... at X 57.99") and screenshots of the same rows | SETUP (WORK would be 17.99 less in these projects) |
| `safety/air-cut.md`, `safety/checklist.md` | "X0 at the part's X origin" | WORK |

Posts: the built-in posts print program words only (WORK), and their comments carry no position.
