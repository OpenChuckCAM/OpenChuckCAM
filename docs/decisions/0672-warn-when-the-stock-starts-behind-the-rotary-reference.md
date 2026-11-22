# 0672: Warn when the stock starts behind the rotary reference point (SET-05, SET-06, SET-09, NFR-12; issue #812; lane Z1-SETUP)

- **Status:** proposed by lane Z1-SETUP on 22 Nov 2026, for the owner's choices D1 to D4 (asked through the coordinator). Not built until they are answered.
- **Date:** 22 Nov 2026.
- **Requirements:** SET-05 (the part's position along the rotary axis), SET-06 (the setup checks), SET-09 (the held region), NFR-07, NFR-12. No requirement text changes.
- **What it amends:** nothing in `requirements.md` or `architecture.md`. It builds on [decision 0649](0649-the-z1-profile-takes-the-firmware-1-1-3-measurements.md) item 5 (the Z1's rotary reference point is the headstock face, so model X 0 is that face) and [decision 0651](0651-the-z1-rotary-reference-y-is-the-firmware-g54-y.md); it uses the warning classes of [decision 0538](0538-a-possible-collision-is-a-warning-to-acknowledge.md) and the message form of [decision 0536](0536-message-levels-operation-reference-and-setting-links.md).
- **Open items:** OI-2185 (this record's choices).

## Context

A setup's part offset and work zero are measured along the rotary axis from the machine's rotary reference point (`MachineProfile::rotary_origin`, SET-05). On the built-in Makera Z1 that point is the centre of the headstock face next to the chuck (decision 0649 item 5), so model X 0 is the headstock face and anything at a model X below 0 would be inside the headstock.

The setup checks (`openchuck_types::setup::check_setup`) check the stock's ends only against the machine's X travel. On the Z1 the travel runs 8.61 mm behind the face (machine X −207 against the face at −198.39), so a stock that starts at X −1 passes: a fresh tutorial project whose Cylinder stock fills in a start of X −1.000 shows "The stock, the clearance and the travel raise no warnings" (issue #812). The toolpath check catches moves outside the travel, but not a stock inside the headstock. When the stock starts far enough behind to leave the travel, the user gets "the start of the stock is at machine X −263.160, outside the travel −207.000 to −1.000" (issue #820), which gives machine coordinates the user did not type and no way to fix it.

The owner's note on #812 (15 Nov 2026): the chuck's length in front of the headstock is not a constant (it varies with the jaws and how far they grip), and the Makera controller lets the user offset X0, so the warning must tell the user to **measure** the distance from the headstock face to where the chuck grips the stock, not rely on an assumed value such as the tutorial's 18 mm.

## Choices for the owner

**D1. Which machines get the check.**

- **A (recommended): every machine.** Model X 0 is the rotary reference point on every machine (SET-05). The warning says where that point is on the Z1 and that a stock cannot start behind it. No schema change. A user-made profile whose reference point is the chuck's face (stock gripped behind it) would warn wrongly; the user guide's definition already describes the reference point as the headstock face on the Z1, "where the controller works from".
- **B: only machines that say their reference point is the headstock face.** A new optional machine profile field (for example `reference_is_headstock_face`, or the headstock face's model X), set on the built-in Z1. Precise, but a format change: machine profile schema 9 and project schema 29 (claimed first), a migration, a machine form field and a provenance entry.
- **C: only copies of the built-in Makera Z1** (known by its library `id`, as the feed word of decision 0076 is). No schema change, but a rule tied to one machine.

**D2. What is checked, and the threshold.**

- **A (recommended):** the start of the stock (the held region at the start lies inside the stock, so it is covered); with no stock, the start of the part. It warns when that start is behind model X 0 by more than 0.0005 mm (the G-code's three places, as the clearance checks).
- **B:** as A, and also a warning when the stock starts within an assumed chuck length (for example 18 mm) of the face. Not recommended: the owner's note says the length varies and must be measured.

**D3. How the warning is classed** (decision 0538).

- **A (recommended): a warning, like the travel checks.** Shown in the setup checks and Messages under the setup's Part position section (where the fix is made); generation and export run; nothing to acknowledge.
- **B: a possible collision**, which must be acknowledged before export, like a safe cylinder inside the stock. Every new project starts with a stock at X −1 (its part at X 0), so every new project would need the acknowledgement until the part is moved.

**D4. The words** (recommended; the owner may change them). For the stock:

> the start of the stock is at X −1.000, 1.000 mm behind the rotary reference point (X 0; on the Makera Z1 the headstock face), so part of it would be inside the headstock or the chuck. Measure the distance from the headstock face to where your chuck grips the stock, with your jaws fitted, and move the part along X by at least that (Offset along X and Zero at X), or shorten the stock.

With no stock, "the start of the part" in place of "the start of the stock", and "move the part along X by at least that (Offset along X and Zero at X)" alone. The machine values it uses (the rotary reference point) are named when they are not confirmed, as every setup check does.

**D5 (recommended, no choice needed unless the owner disagrees): the travel warning along the rotary axis says how to fix it** (#820). "the start of the stock is at machine X −263.160, outside the travel −207.000 to −1.000" becomes:

> the start of the stock (X −64.770) is at machine X −263.160, outside the travel −207.000 to −1.000. Move the part and the stock along X (Offset along X and Zero at X), or shorten the stock.

The model X comes first because it is the number the user typed; the machine X stays for checking against the controller. The same for the end of the stock, the part and the work zero ("Move the work zero (Zero at X)" for the work zero). It needs the model X in `SetupWarningKind::OutsideTravel`, a new field: a breaking change of `openchuck-types` and so a workspace minor bump (decision 0048), which waits until the coordinator says the crates.io publish of 0.55.0 is done.

## Proposed decision

D1 A, D2 A, D3 A, D4 and D5 as written: one new `SetupWarningKind`, `StartsBehindReference { what, x }`, in `openchuck-types` (the enum is `non_exhaustive`, so adding a variant is not a breaking change), raised by `check_setup` after the clearance checks and before the travel checks, and shown in the app's setup checks, Messages and export checks and the command line's `setup` report as the other setup warnings are. No G-code changes (GC-07): it is a check only.

## Consequences (if accepted as proposed)

- Every new project warns until its part is moved past the chuck: the user guide's tutorial already sets the part offset and work zero before the stock (decision 0650), so its screens should not change; `user-docs` pages that describe the setup checks get a sentence. No decision or `OI-` numbers in the guide.
- The example and sample projects start their stock at X 5.99 or later (checked 22 Nov 2026: the lowest is `cone_held_tabs`, 5.990); a test asserts that none warns.
- Generated G-code is still the user's to verify before it runs (NFR-12). The warning is not a claim that a stock in front of the face clears the chuck.

## Verification

To be written when the choices are answered and the check is built.
