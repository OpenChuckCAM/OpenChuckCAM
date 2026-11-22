# 0672: Warn when the stock starts behind the Z1's headstock face, and say what the travel numbers mean (SET-05, SET-06, SET-09, UI-07, NFR-12; issues #812 and #820; lane Z1-SETUP)

- **Status:** proposed by lane Z1-SETUP on 22 Nov 2026. The owner accepted the first proposal's recommended set in principle on 22 Nov 2026 (every machine, a 0.0005 mm threshold on the stock's start, a plain warning, the #820 travel message to gain the model X and a fix), relayed by the coordinator, with two required changes: (1) short wording that says what to do first, with explanation where the user meets it (tooltips, and a user guide section with a diagram and a worked Z1 example); (2) an answer to whether model X 0 can be assumed to be the headstock on every machine. This revision answers both. Not pushed until the owner agrees the text (OI-2185).
- **Date:** 22 Nov 2026.
- **Requirements:** SET-05 (the part's position along the rotary axis), SET-06 (the setup checks), SET-09 (the held region), UI-07 (the inspector's explanations), NFR-07, NFR-12. No requirement text changes.
- **What it amends:** nothing in `requirements.md` or `architecture.md`. It builds on [decision 0011](0011-slice-3-machine-stock-setup.md) items 1, 2 and 4 (the frames and the rotary reference point), [decision 0201](0201-setup-offset-moves-the-part.md) (the part offset moves only the part), [decision 0649](0649-the-z1-profile-takes-the-firmware-1-1-3-measurements.md) item 5 and [decision 0651](0651-the-z1-rotary-reference-y-is-the-firmware-g54-y.md) (the Z1's reference point is the headstock face), and uses the warning classes of [decision 0538](0538-a-possible-collision-is-a-warning-to-acknowledge.md) and the message form of [decision 0536](0536-message-levels-operation-reference-and-setting-links.md).
- **Open items:** OI-2185 (the owner's agreement to this text); OI-2187 (a profile field naming what the reference point is: proposed, not added).

## Context

On a fresh tutorial project the Cylinder stock fills in a start of X −1.000 and the setup checks say nothing (issue #812), although on the Makera Z1 that is 1 mm inside the headstock. When the stock starts far enough behind to leave the travel, the user gets "the start of the stock is at machine X −263.160, outside the travel −207.000 to −1.000" (issue #820): a machine coordinate the user never typed, and no way to fix it. The owner's note on #812: the chuck's length varies with the jaws and how far they grip, so the user must measure it, not rely on the tutorial's 18 mm.

## Is model X 0 always the headstock? (the owner's question)

**What X 0 is, by construction.** Every position along the rotary axis in a setup is measured from the machine profile's **rotary reference point**, `rotary_origin` (decision 0011 items 1 and 4): the stock's **Start X**, the held regions and tabs, the work zero (**Zero at X**), and the part after its **Offset along X** (decision 0201). The machine X of a setup position *x* is `rotary_origin.x + x` (`RotaryFrame::on_axis`; `expected_work_offset` for the work zero; the travel checks of `check_setup`; the program check in `posting.rs`). So setup X 0 is the rotary reference point on every machine, by construction. The program's own X0 is the work zero, which is X 0 only when **Zero at X** is 0.

**What the reference point is on the machine.** That is the profile author's choice, and the program does not know it. Decision 0011 item 4 defines `rotary_origin` as "a point on the rotary axis such as the chuck face centre". On the built-in Makera Z1 it is the centre of the headstock face next to the chuck (decision 0649 item 5), because the controller works from there and the chuck's face moves with the jaws. The only record of what the point is lies in the free-text provenance `source` of `rotary_origin`; there is no field for it. The library's doc comment and the inspector's **Position** tooltip still say "the chuck face" (`MachineProfile::rotary_origin`; `POSITION_EXPLAINED` in `crates/openchuck-app/src/setup_form.rs`), which has been wrong for the Z1 since 0649.

**So the assumption is not safe on other machines.** A warning that model X 0 is the headstock face is right on the built-in Z1 and gives a **wrong warning** elsewhere:

- a profile whose reference point is the **chuck face** expects the gripped stock to start behind X 0, inside the jaws: every correct setup on it would warn;
- a profile whose reference point is in front of the headstock (an older copy of the Z1 is one: its reference X −180.4 is 17.99 mm in front of the face, decision 0649) would warn about stock that is clear;
- a profile whose reference point is behind the headstock face would miss stock inside the headstock.

None of these can be told apart without a field that says what the reference point is.

## Choices for the owner

**R. Which machines get the warning.**

- **R-A (recommended): copies of the built-in Makera Z1 whose Rotary reference X is the built-in's, −198.39** (the headstock face). They are known as the old-copy checks of decisions 0647 and 0651 know a Z1 copy, by the copy's library `id` (or, for a copy stored without one, its name, Makera Z1), and by the reference X, so an old copy with −180.4 does not warn. The warning names the headstock face. No other machine gets it, so it is never wrong; the tooltips and the guide section below explain X for every machine. No schema change. A user's own copy of the Z1, saved under a new `id`, does not get it either; the guide says so.
- **R-B: R-A now, and later an optional machine profile field naming what the reference point is**, for example `rotary_reference = "headstock_face" | "chuck_face" | "other"`, so any machine can get the right warning: with `headstock_face` the warning below; with `chuck_face` a warning only when the stock starts behind X 0 by more than its held region at the start (the grip); with `other` or none, no warning. **Not added here** (the owner asked for a proposal only). Its cost: a format change, machine profile schema 9 and project schema 29 (the machine copy is a project file; claimed in the work breakdown first), an additive migration that changes only the number, one row in the machine form and the machine library, and the value in the built-in Z1. Tracked as OI-2187.
- **R-C: every machine, worded "behind the rotary reference point" without naming the headstock** (the first proposal). Gives the wrong warnings listed above. Not recommended.

**T. The threshold and the class** (accepted in principle, unchanged): the stock's start, or with no stock the part's start, more than 0.0005 mm behind X 0; a plain warning under the setup's **Part position**, nothing to acknowledge (decision 0538).

**W. The words** (short, what to do first; the tutorial's fresh stock). With R-A:

> Stock (setup 'Setup 1'): the stock starts at X −1.000, 1.000 mm inside the headstock (X 0 is the headstock face on the Makera Z1). Measure from the headstock face to where your chuck grips, with your jaws fitted, and set the stock's Start X to at least that; move the part (Offset along X) and the work zero (Zero at X) with it.

With no stock: "the part starts at X …", and "set the part's Offset along X so it starts at least that far along, and move the work zero (Zero at X) with it". With R-C the first sentence would read "… 1.000 mm behind the machine's rotary reference point (X 0)", naming no headstock.

The first proposal said "move the part along X (Offset along X and Zero at X), or shorten the stock". That was wrong: **Offset along X moves only the part** (decision 0201), so it does not take the stock out of the headstock, and shortening the stock does not either. The fix is the stock's **Start X**.

**M. The travel message along the rotary axis** (#820). Today: "the start of the stock is at machine X −263.160, outside the travel −207.000 to −1.000". Proposed, worked out in the application from the profile's Rotary reference X, so `openchuck-types` and its public API do not change:

> Travel (setup 'Setup 1'): the start of the stock, at X −64.770, is 56.160 mm past the machine's X travel: it would be at machine X −263.160, and the machine reaches −207.000 to −1.000. Move the stock towards +X (Start X), with the part (Offset along X) and the work zero (Zero at X), or shorten it. X is measured from the machine's rotary reference point, at machine X −198.390.

The same for the end of the stock ("towards −X"), the part (Offset along X only) and the work zero (Zero at X only). The first proposal's version needed a new field in `SetupWarningKind::OutsideTravel`, a breaking change; this one does not.

**E. Explanation where the user meets it** (the owner's change 1).

- **Tooltips.** **Offset along X**, **Zero at X**, the stock's **Start X**, the read-only **Position** row and the machine form's **Rotary reference X** each say, in one or two sentences, that X is measured from the machine's rotary reference point (the headstock face on the Makera Z1), and what the value moves and does not move: Offset along X moves only the part; Start X moves the stock; Zero at X moves only the program's X0. The Position tooltip's "(the chuck face)" goes.
- **The user guide.** One section, "Where things sit along X", in the concept page *Stock and the held region* (`concepts/stock-and-held-region.md`), which already explains the stock and the grip; the tutorial's step 4 and the machine profile reference (**Rotary reference X, Rotary reference Y**) link to it rather than repeat it. The 0.55.0 release notes describe that release and are not changed. It has an inline SVG in the guide's style (`currentColor`, with `<title>` and `<desc>`, as in *How 4-axis rotary machining works*) showing, left to right: the headstock and its face, the chuck and jaws, the rotary reference point (X 0, at machine X = Rotary reference X), the stock's Start X, the held region, the part (placement plus Offset along X), the work zero (Zero at X, the program's X0) and the machine's X travel. A worked Z1 example in numbers, the #820 case: Start X −64.770 is machine X −198.390 + (−64.770) = −263.160, outside −207.000 to −1.000; the stock must start at X −8.610 or more to be inside the travel, and at the measured chuck length or more to be clear of the chuck. No decision or `OI-` numbers in the guide.

## Proposed decision

R-A, T, W, M and E as written. In code: `SetupWarningKind::StartsBehindReference { what, x }` (a new variant of a `non_exhaustive` enum, not a breaking change) with the words of W, raised by the application for a qualifying Z1 copy beside `check_setup`'s own warnings, wherever those are shown (setup checks, Messages, the export checks, the command line); M as the application's text for the along-axis travel warning; the tooltips and the guide section of E. R-B is recorded as OI-2187. No G-code changes (GC-07).

## Consequences (if accepted as proposed)

- A new Z1 project warns until its stock is placed past the chuck. The tutorial sets the part offset and work zero first (decision 0650) and then the stock, so its screens should not change. The samples and examples start their stock at X 5.99 or later (checked 22 Nov 2026: the lowest is `cone_held_tabs`), so none warns; a test asserts it.
- Machines other than a current copy of the built-in Z1 get no new warning (R-A); the guide says so.
- The doc comment of `MachineProfile::rotary_origin` says "such as the centre of the headstock face next to the chuck (the Makera Z1's)" in place of "the chuck face" (documentation only).
- Generated G-code is still the user's to verify before it runs (NFR-12). The warning is not a claim that a stock in front of the face clears the chuck.

## Verification

To be written when the text is agreed and the change is built.
