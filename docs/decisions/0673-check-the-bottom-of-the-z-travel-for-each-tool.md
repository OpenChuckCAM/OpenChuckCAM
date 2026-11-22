# 0673: Check the bottom of the Z travel for each tool (SET-01, OP-14, GC-08, NFR-12; issue #692, OI-1781; lane Z1-SETUP)

- **Status:** proposed by lane Z1-SETUP on 22 Nov 2026, for the owner's choices D1 to D4 (asked through the coordinator). Not built until they are answered.
- **Date:** 22 Nov 2026.
- **Requirements:** SET-01 (the machine's travel), OP-14 (links at the clearance), GC-08 (axis limit violations found before export), NFR-07, NFR-12. No requirement text changes.
- **What it amends:** nothing in `requirements.md` or `architecture.md`. It builds the case [decision 0495](0495-minimum-clearance-and-clearance-from-stock.md) item 6c left unbuilt, beside the top-of-travel check of [decision 0318](0318-z1-spindle-height-warning-switched-on.md) and 0495 item 6b, with the Z1's measured values of [decision 0649](0649-the-z1-profile-takes-the-firmware-1-1-3-measurements.md).
- **Open items:** OI-1781 (closed when this is built); OI-2186 (this record's choices).

## Context

On a machine that measures each tool's length (`automatic_tool_length`), the spindle rises or falls with the tool so that every tool's tip reaches the programmed Z. Decision 0318 checks the top of the Z travel: with the spindle-to-axis distance *D* (from the spindle at the top of the Z travel down to the rotary axis) and a tool that sticks out *S*, its tip can rise at most *D − S* above the axis.

The bottom is the same geometry the other way. With the Z travel span *T* (`travel.max.z − travel.min.z`), the spindle at the bottom of the travel is *D − T* above the axis, so a tool's tip can go down to *D − T − S* and no lower. A short tool (small *S*) cutting deep near the axis may need the spindle below the bottom of the travel. The program check (`posting.rs`) checks Z against the travel converted with the axis height, which holds for the reference tool only; a tool shorter than the reference tool goes lower than that check assumes.

On the built-in Z1, *D* = 72.0 and *T* = 101.0 (Z −102 to −1), so *D − T* = −29.0: the spindle at the bottom is 29 mm below the axis, and any tool can take its tip at least 29 mm below the axis. A 4-axis toolpath's tip stays at or above the axis almost everywhere, so the check is not expected to warn on the Z1 with a normal setup; it matters for a machine with a short Z travel or a high axis, and for a profile edited by the user.

## Choices for the owner

**D1. The stick-out the check assumes** (the tool library holds no stick-out, OI-1782).

- **A (recommended): flute length + 5 mm, as decision 0318.** One assumption for both ends of the travel, in the same words; OI-1782's stick-out per tool would replace both.
- **B: flute length only.** The least a tool can stick out and still cut with its whole flute: the worst case for the bottom, so it warns earlier. The two checks would then assume different stick-outs for the same tool.

**D2. Which moves count.**

- **A (recommended): every move of the operation's toolpath**, rapids and links included: the spindle must reach every point.
- **B: cutting moves only.** Links are at the clearance and are not low; there is no practical difference, but B leaves out plunges and lead-ins below the cut.

**D3. How the warning is classed** (decision 0538).

- **A (recommended): a warning, like the top of the travel.** A travel limit is not a possible collision: the controller's soft limit stops the machine (decision 0538). Shown with the operation's warnings (Results tab, Messages, the export checks and the command line); nothing to acknowledge.
- **B: refuse export.** Not recommended: the coordinator's brief and decision 0495 item 2 keep every check advisory.

**D4. The words** (recommended; the owner may change them):

> tool *n* may need the spindle below the bottom of the Z travel: its tip goes down to *z* mm from the axis, and assuming it sticks out *S* mm (flute length + 5 mm), the spindle at the bottom of the travel takes its tip only down to *D − T − S* mm. Use a longer tool or a longer stick-out, or keep the cut further from the axis, and check the Z travel on your machine before running.

The lead-in names the topic and the operation in the one form of decision 0536 ("Tool length (operation '*name*'): …"). Machines that do not measure tool lengths, or give no spindle-to-axis distance, get no guess, as for the top (OI-1782).

## Proposed decision

D1 A, D2 A, D3 A and D4 as written. A pure function in `openchuck-app` beside `spindle_height` (`spindle_depth`, say) takes the profile, the tool and the operation's lowest tip, worked out from the toolpath as the tip's height above the rotary axis along the tool axis (the dot product of the tip and the tool axis in the model frame, since the tool axis crosses the rotary axis at right angles on a 4-axis machine). The pipeline adds the warning to the operation's group warnings after the toolpath is made, so it shows wherever an operation's warnings show. To three places, as the G-code: a tip within 0.0005 mm of the limit passes. No G-code changes (GC-07).

## Consequences (if accepted as proposed)

- None of the samples, examples or reference jobs warns (the Z1's tip limit is at least 29 mm below the axis); a test asserts it for the reference jobs, and the worked cases are tested on a profile with a short Z travel.
- The user guide's machine profile reference says that the spindle-to-axis distance and the Z travel are used for both ends of the travel (no decision or `OI-` numbers in the guide).
- Generated G-code is still the user's to verify before it runs (NFR-12). No warning means only that this check found nothing; it is not a claim that the program stays in the travel.

## Verification

To be written when the choices are answered and the check is built.
