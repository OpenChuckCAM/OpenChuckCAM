# 0673: Check the bottom of the Z travel for each tool (SET-01, OP-14, GC-08, NFR-12; issue #692, OI-1781; lane Z1-SETUP)

- **Status:** proposed by lane Z1-SETUP on 22 Nov 2026. The owner accepted the recommended set in principle on 22 Nov 2026, relayed by the coordinator, with one change: the tool's stick-out belongs in the tool library; until a tool has one, the check uses the assumption (flute length + 5 mm) and the warning and the guide say so (D1 below). Awaiting the owner's acceptance of this text (OI-2186).
- **Date:** 22 Nov 2026.
- **Requirements:** SET-01 (the machine's travel), OP-14 (links at the clearance), GC-08 (axis limit violations found before export), NFR-07, NFR-12. No requirement text changes.
- **What it amends:** nothing in `requirements.md` or `architecture.md`. It builds the case [decision 0495](0495-minimum-clearance-and-clearance-from-stock.md) item 6c left unbuilt, beside the top-of-travel check of [decision 0318](0318-z1-spindle-height-warning-switched-on.md) and 0495 item 6b, with the Z1's measured values of [decision 0649](0649-the-z1-profile-takes-the-firmware-1-1-3-measurements.md).
- **Open items:** OI-1781 (closed when this is built); OI-2186 (this record's choices).

## Context

On a machine that measures each tool's length (`automatic_tool_length`), the spindle rises or falls with the tool so that every tool's tip reaches the programmed Z. Decision 0318 checks the top of the Z travel: with the spindle-to-axis distance *D* (from the spindle at the top of the Z travel down to the rotary axis) and a tool that sticks out *S*, its tip can rise at most *D − S* above the axis.

The bottom is the same geometry the other way. With the Z travel span *T* (`travel.max.z − travel.min.z`), the spindle at the bottom of the travel is *D − T* above the axis, so a tool's tip can go down to *D − T − S* and no lower. A short tool (small *S*) cutting deep near the axis may need the spindle below the bottom of the travel. The program check (`posting.rs`) checks Z against the travel converted with the axis height, which holds for the reference tool only; a tool shorter than the reference tool goes lower than that check assumes.

On the built-in Z1, *D* = 72.0 and *T* = 101.0 (Z −102 to −1), so *D − T* = −29.0: the spindle at the bottom is 29 mm below the axis, and any tool can take its tip at least 29 mm below the axis. A 4-axis toolpath's tip stays at or above the axis almost everywhere, so the check is not expected to warn on the Z1 with a normal setup; it matters for a machine with a short Z travel or a high axis, and for a profile edited by the user.

## Choices for the owner

**D1. The stick-out** (the owner's change). The tool library holds no stick-out: `Tool` (`openchuck-types`) has the shape, diameter, flute length, overall length, shank diameter, flute count, material and centre cutting, and nothing about the holder or how far the tool is set in the collet; the overall length gives no stick-out without the depth in the collet. Decision 0318's top-of-travel check and decision 0495 item 6b assume flute length + 5 mm (`STICK_OUT_BEYOND_FLUTES`). So:

- **Now (this pull request):** the check uses the same assumption and its warning states it: "assuming it sticks out 5 mm beyond its flutes (*S* mm in all; the tool library holds no stick-out)". The guide's machine profile reference says the same. The words do not tell the user to set a stick-out in the tool library, since there is no field to set yet.
- **Proposed, a second pull request after the 0.55.0 crates publish (OI-1782, which already tracks a stick-out per tool):** an optional `stick_out` on `Tool`, in tool library files and the project's tool snapshots, written only when set. Its cost: a new public field on `openchuck_types::tool::Tool`, a struct that is not `non_exhaustive`, so a **breaking change** for `cargo-semver-checks` and a workspace minor bump (the 0.56.0 bump, shared if another change takes it, decision 0048); the next free **project schema** number (29 at the time of writing, claimed in the work breakdown first; OI-2187's proposed field would then take the one after), with an additive migration that changes only the number and refuses an older file already holding the field, as schema 8 did for `centre_cutting`; no machine profile schema; one row in the tool form (**Stick-out**, blank allowed, with a tooltip: "How far the tool sticks out of the collet, tip to collet face. Blank uses the flute length + 5 mm"), the tool library window and the command line's `tool` options; the top-of-travel check (0318), the workholding case (0495 item 6b) and this check take it when set and keep the stated assumption when not; with the field, the warning ends "set the tool's stick-out in the tool library for a better check", as the owner asked; the guide's tool reference and the machine profile reference follow.

**D2. Which moves count.**

- **A (recommended): every move of the operation's toolpath**, rapids and links included: the spindle must reach every point.
- **B: cutting moves only.** Links are at the clearance and are not low; there is no practical difference, but B leaves out plunges and lead-ins below the cut.

**D3. How the warning is classed** (decision 0538).

- **A (recommended): a warning, like the top of the travel.** A travel limit is not a possible collision: the controller's soft limit stops the machine (decision 0538). Shown with the operation's warnings (Results tab, Messages, the export checks and the command line); nothing to acknowledge.
- **B: refuse export.** Not recommended: the coordinator's brief and decision 0495 item 2 keep every check advisory.

**D4. The words** (recommended; the owner may change them):

> tool *n* may need the spindle below the bottom of the Z travel: its tip goes down to *z* mm from the axis, and assuming it sticks out 5 mm beyond its flutes (*S* mm in all; the tool library holds no stick-out), the spindle at the bottom of the travel takes its tip only down to *D − T − S* mm. Use a longer tool or a longer stick-out, or keep the cut further from the axis, and check the Z travel on your machine before running.

The lead-in names the topic and the operation in the one form of decision 0536 ("Tool length (operation '*name*'): …"). Machines that do not measure tool lengths, or give no spindle-to-axis distance, get no guess, as for the top (OI-1782).

## Proposed decision

D1 (the stated assumption now, the field proposed), D2 A, D3 A and D4 as written, with D1's words. A pure function in `openchuck-app` beside `spindle_height` (`spindle_depth`, say) takes the profile, the tool and the operation's lowest tip, worked out from the toolpath as the tip's height above the rotary axis along the tool axis (the dot product of the tip and the tool axis in the model frame, since the tool axis crosses the rotary axis at right angles on a 4-axis machine). The pipeline adds the warning to the operation's group warnings after the toolpath is made, so it shows wherever an operation's warnings show. To three places, as the G-code: a tip within 0.0005 mm of the limit passes. No G-code changes (GC-07).

## Consequences (if accepted as proposed)

- None of the samples, examples or reference jobs warns (the Z1's tip limit is at least 29 mm below the axis); a test asserts it for the reference jobs, and the worked cases are tested on a profile with a short Z travel.
- The user guide's machine profile reference says that the spindle-to-axis distance and the Z travel are used for both ends of the travel (no decision or `OI-` numbers in the guide).
- Generated G-code is still the user's to verify before it runs (NFR-12). No warning means only that this check found nothing; it is not a claim that the program stays in the travel.

## Verification

To be written when the choices are answered and the check is built.
