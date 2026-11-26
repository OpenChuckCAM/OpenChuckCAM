# 0699: Every position a message gives is in the work frame (UI-12, NFR-07, NFR-12, SET-05, GC-07; the owner's decision of 26 Nov 2026 on OI-2237; lane WORDING-DOCS)

- **Status:** Proposed by lane WORDING-DOCS on 26 Nov 2026, with the owner's five answers recorded below (26 Nov 2026, relayed by the coordinator, decision 0049): the owner has answered the questions but has not said he accepts the text of this record, so it is not accepted and is guidance, not authority (decision 0049; only the project owner accepts a record). Number assigned by the coordinator.
- **Date:** 26 Nov 2026.
- **Requirements:** UI-12 (results and messages), NFR-07 (clear messages), NFR-12 (the user verifies the G-code, and a position in a message that matches the G-code is one they can verify), SET-05 (work zero and part position), GC-07 (a run's output follows its project: the G-code must not change).
- **Amends, if accepted:** nothing yet. Option B would add a rule to [decision 0536](0536-message-levels-operation-reference-and-setting-links.md) (one form for messages) and change what [decision 0201](0201-setup-offset-moves-the-part.md) item 5 says the Setup step shows; it keeps 0201's frames.
- **Open items:** OI-2260 (the owner's acceptance of this record), OI-2261 (the build lane below). OI-2237 (raised in PQ-4, earlier called OI-9998) is the issue this answers; its question 5 is answered below, and it closes when the work is built.
- **Inventory:** [message-review/frames.md](../message-review/frames.md) lists every place a position is printed and its frame. A drawing is in [the diagram](0699-every-position-is-in-the-work-frame.svg).

## Context

The owner's decision (26 Nov 2026, relayed by the coordinator): "every coordinate should be in a single reference frame; the most logical is the shown origin point, i.e. the WCS origin; tell me if you disagree; there may be specific exceptions for certain messages." It follows OI-9998, raised when lane PATH-QUALITY, in PQ-4, found that adaptive spacing's warning would say "X 52.900", a position in the setup frame (measured from the rotary reference point), where the part's own start is at X 17.99 in that frame. If that project's **Zero at X** is 17.99, the same point is 52.900 − 17.99 = 34.910 from the work zero, which is what the playback and cursor readouts and the G-code would show. (This lane did not open that project; the figures are the PATH-QUALITY lane's and the **Zero at X** value is an assumption.)

What the code does today (the inventory has every site):

- **Two frames carry almost all positions, and they differ by Zero at X.**
  - The **setup frame** (decision 0201): X is measured from the machine's rotary reference point. The stock, the held regions, the tabs, the work zero and every toolpath are given in it, so every position in a path message, the Setup step's rows, the workholding panel, the setup sheet and most of the command line is in it (about 66 sites).
  - The **work frame** (the WCS): the setup frame moved by **Zero at X** (`RotaryFrame::work`; Y and Z are distances from the axis). The G-code, the simulation's playback line, the viewport's cursor readout ("G54 work coordinates") and the G-code checks are in it (5 sites).
  - They agree only when **Zero at X** is 0. In the example projects and the tutorial it is the part's offset (17.99 and 18), so a warning "at X 39.193" is X 21.203 in the G-code and on the readouts.
- **The shown origin marker is the work zero.** The viewport's work axes start at **Zero at X** (`scene.rs`, at setup X = `work_offset.x`), which is also the value of the Setup step's **Zero at X** field. So the WCS origin the owner points to is that marker, and it is not the setup frame's zero (the rotary reference point) unless **Zero at X** is 0.
- **The setup fields are typed in the setup frame.** **Start X** (and **X from** and **X to**), the held regions, the tabs and the grips are positions measured from the rotary reference point, and **Zero at X** itself is too. **Offset along X** is a distance, not a position; the Position row shows the part's range in the setup frame. A user who types **Zero at X** 18 and then reads "X 21.203" in a warning must subtract by hand to find the position they typed.
- **A few positions are in other frames:** the machine frame for travel limits and the register value (about 11 sites), and the placed model for the Position row's breakdown, the command line's `bounds` line and the inspector note (3 sites). Nothing prints the model file's own coordinates. About 35 sites print lengths, deltas and angles, which have no frame.
- **The wording is loose:** "work zero at model X" (setup sheet, command line) and "Model X of the work zero" mean the setup frame; "model frame" (the command line's `bounds` line) is the placed model; the viewport readout says "work coordinates" even when no setup is chosen; the playback line is the work frame with no frame word.

## Options

- **Option A: messages only.** Every message gives its positions in the work frame. The setup fields stay measured from the rotary reference point and say so.
  - A message that quotes a field the user typed (a tab at X 12.99 to 17.99) would then give different numbers from the field. That is the mismatch the owner's rule is meant to remove, so A leaves the main case open.
- **Option B: messages and the setup fields, converted at the edge.** Every position a user reads or types is in the work frame: the messages, the Results, the command line, and the Setup step's fields (**Start X**, held regions, tabs, grips, and the Position row). The project keeps storing the setup frame; a position is converted when it is printed or typed, once, in one place each way.
  - **Zero at X** and **Rotary reference X** are the only two values still given from another origin, labelled "from the rotary reference point" and "machine X", because they define the work frame and the machine's place.
  - When the user changes **Zero at X**, the stock, the held regions and the tabs stay where they are physically (decision 0201 item 1: the work zero is set against the machine, not the part), so their typed numbers change by the difference. The Setup step says so on that field.
  - The G-code is unchanged byte for byte (GC-07): nothing about generation or posting changes.
- **Option C: store the work frame.** The setup frame's origin becomes the work zero; the stock, the tabs, the held regions and the toolpaths are stored and computed in it.
  - No position needs converting for a message: every number in the code is already in the one frame.
  - Changing **Zero at X** would then move every stored position with it, or rewrite every stored position to keep the part where it is; either way it reverses decision 0201 item 2 (the zero stays put when the part moves) and needs a project schema bump with a migration of every stored position.
  - Computing in a shifted frame can change the last bit of a position, so the 3-decimal G-code of an existing project could in rare cases change a digit (GC-07), and every golden file would need checking.

## Recommendation

**Option B (the owner's answer, question 1), with a guard that makes a missed conversion fail a test.**

- The inventory's strongest finding is that two frames meet in the same sentence: the travel message gives "at X v" in the setup frame and "machine X v" in the machine frame, and a G-code check quotes the work frame for the same position. A rule stated for messages only (A) leaves the fields and the messages disagreeing, which is the case a user meets first (they type a tab and read a warning about it).
- C removes the need to convert but changes what a stored value means and reverses an accepted record; the mismatch it removes is the one B removes at the edge, without changing a stored value or the G-code.
- B's risk is a site that prints a raw setup-frame number. The guard answers it: every position goes through one formatter (a function in `openchuck-types` taking the setup frame and returning the work-frame text, with the frame word the exceptions need), and the message check in `xtask` (decision 0536's ratchet) fails on a position-looking `X {` format string that does not use it. The completeness test for exceptions is a list in the code of the exception labels.

## Proposed rule and exceptions

**Every position a message gives is in the work frame, measured from the work zero (the shown origin marker), written "X 21.203 mm" with no frame word.** A position in another frame is written with its label. The exceptions the inventory justifies:

| Exception | Label | Why |
| --- | --- | --- |
| Machine limits and travel: "the machine reaches X −207.000 to −1.000", the register value, the rotary reference | "machine X" | They are the machine's own numbers, set and checked on the machine; a user compares them with the controller. |
| **Zero at X**, and **Rotary reference X** | "from the rotary reference point" and "machine X" | They define the work frame and where it sits. |
| The model's own geometry: an import's size, the placement steps, the `bounds` line, a face's position on the model, selections and face ids | "model X" | They describe the model file and its placement, before any setup; the work frame does not exist yet (no setup chosen). |
| Lengths, distances, widths, depths, deltas ("moved 18 mm", "tab 2 mm long") and angles (A) | none | They have no frame. |
| The viewport's cursor readout with no setup chosen | "model X" (not "work coordinates") | There is no work zero until a setup is chosen. |

A message that quotes a setup field uses the field's frame, which under B is the work frame, so a typed number and the message about it agree. A travel message gives both, each labelled: "at X 21.203 mm (machine X −177.187)".

## What it would need

- **Messages only (A or the message half of B):** the position formatter and its guard in `openchuck-types` (or `openchuck-path`), the work zero as an input of `openchuck-path` (a new field on the generation context it already takes), and the app's, the command line's and the interface's own strings.
- **The setup fields (B):** `openchuck-app` and `openchuck-ui` (the setup form's conversion in and out, the Position row, the workholding panel, the setup sheet, the command line's `setup` options), the tooltips and the user guide pages and screenshots that quote positions.
- **Scope, as information and not as a reason for or against an option:** B touches about 66 print sites and the Setup step's fields, and adds the work zero to the input of `openchuck-path`; C touches every stored position.
- **Library impact:** a new input to `openchuck-path` and a new function in `openchuck-types` are API changes; adding a field to a struct that users build by literal is breaking, so the workspace minor version is bumped with a changelog entry (decision 0048; the coordinator assigns the number). The wording of messages is a changed output of `openchuck-path` (a crate changelog line each). `openchuck-gcode` and `openchuck-post` need no change: they are already in the work frame. The G-code does not change.
- **Lanes (proposed, for the coordinator):**
  - the message formatter, the guard and the path crate's messages: a path-messages lane;
  - the setup fields, the Position row, the workholding panel and the command line: the setup and interface lanes;
  - the guide and the screenshots that quote positions (the example pages' Results text and images would all read 17.99 less in the work frame): a docs lane, after the code, because the screenshots need the STEP kernel build.
- **Tests (from the requirements):** a test per site family that the message gives the work-frame number for a setup whose **Zero at X** is not 0 (so a missed conversion fails); a test that a position typed in a setup field and the message quoting it agree; a test that the formatter's labels are the exceptions' labels and no others; the `xtask` check above. A project with **Zero at X** 0 must read the same as today.

## Questions for the owner and the answers (all five answered on 26 Nov 2026, relayed)

1. **Which option: A, B or C?** **Answered, 26 Nov 2026 (relayed by the coordinator, decision 0049): B** (messages and the setup fields in work-zero X; the project file and the G-code unchanged; **Zero at X** and **Rotary reference X** are the only values still given from the other origin, labelled so; decision 0201 is kept). The record as a whole is not accepted yet.
2. When the user changes **Zero at X**, should the typed numbers of the stock, the held regions and the tabs change (the part stays where it is, as in decision 0201) or should the stock and workholding move with the zero? **Answered, 26 Nov 2026 (relayed by the coordinator, decision 0049): they stay where they are physically** (decision 0201); the typed numbers shift by the change in **Zero at X**; the **Zero at X** field's tooltip says so. Display and tooltip only, no project change.
3. The label wording of the exceptions, and whether the machine's numbers in a travel message are shown beside the work-frame position or only the work-frame position is shown. **Answered, 26 Nov 2026 (relayed by the coordinator, decision 0049):** (i) the labels are accepted as proposed: "machine X" (travel limits, the G54 register value, **Rotary reference X**), "from the rotary reference point" (**Zero at X** itself), "model X" (import, placement, face geometry, and the cursor readout when no setup is chosen), and no label for lengths, deltas and angles; (ii) a travel message shows both positions, each labelled (for example "at X -82.760 (machine X -263.160)"). The list of labels is enforced by a test, so no other label appears.
4. Whether the examples' Results text and screenshots are re-captured in one change after the code, or page by page. **Answered, 26 Nov 2026 (relayed by the coordinator, decision 0049): in one change.** All the example pages and screenshots are re-captured in one docs change after the code lanes have landed, so the guide is never half in one frame. Docs only.

5. **OI-2237: should a message that gives X name its frame ("setup X")?** **Answered, 26 Nov 2026 (relayed by the coordinator, decision 0049): no frame word on a normal position.** "Setup X" is never written. Every unlabelled position is work-zero X, and only the exceptions of question 3 carry a label.

## The build lane (proposed, for the coordinator)

The work implied by option B is a breaking library change, so it is a separate lane after this record is accepted. Proposed name: **WORK-FRAME**, in three pull requests in this order (each opened as a draft, one push after release):

1. **WF-1, the position formatter and the path crate.**
   - Scope: the work zero becomes an input of `openchuck-path` (a new field on the generation context); one position formatter in `openchuck-types` taking the setup frame and giving the work-frame text with the labels of question 3 (the label list is a constant); every position in an `openchuck-path` message goes through it (about 26 sites) and `openchuck-types`' setup and workholding messages (5 sites); the `xtask` message check fails on a position-looking `X {` format string that does not use it (decision 0536's ratchet).
   - Library impact: breaking; the workspace minor version is bumped with changelog entries for `openchuck-types` and `openchuck-path` (decision 0048; the coordinator assigns the number).
   - Exit criteria: a test per message family that its X is the work-frame number for a setup whose **Zero at X** is not 0; a test that the formatter's labels are exactly the five of question 3; the `xtask` check fails on a raw position string and passes on the formatter; a project with **Zero at X** 0 reads as today; G-code and golden files unchanged (GC-07).
2. **WF-2, the application, the setup fields, the command line and the interface.**
   - Scope: the setup fields (**Start X** and the box's **X from** and **X to**, the held regions, the tabs, the grips and the Position row) typed and shown in work-zero X, converted at the edge, with the project file unchanged; the **Zero at X** tooltip says that the stock, held regions and tabs stay where they are when it changes (question 2); the travel messages show both positions, each labelled (question 3); the setup sheet, the command line (`setup`, `--tab`, `--held-start`, the `bounds` line, the "work zero ... at model X" wording), the viewport readout (labelled "model X" with no setup chosen) and the playback line; `openchuck-app` and the application crates are outside the library family.
   - Exit criteria: a position typed in a setup field and the message quoting it agree (a test per field family); changing **Zero at X** moves no stored value and no G-code byte; the travel message shows both positions; the unit and CLI tests of the changed rows pass; the user-docs and release notes say it (the guide pages only for the changed texts; the examples wait for WF-3).
3. **WF-3, the user guide and the screenshots (docs only).** One change that re-captures all the example pages' quoted text and screenshots and the tutorial's numbers in work-zero X (question 4), after WF-1 and WF-2 have merged, on a machine with the STEP kernel build; `CAPTURED.toml` hashes true.

- **Where it sits:** it needs only this record accepted and the numbers assigned; WF-1 first (it changes the library), WF-2 after WF-1 has merged, WF-3 last. It touches `openchuck-path`, so it must not run beside a lane that is changing path messages; the coordinator orders it in the work breakdown's "Parallel work" table (a row to add when the record is accepted: not added here, because the table is the coordinator's).

## Consequences

If accepted as recommended, no stored value and no G-code changes; a warning's X reads the same as the playback readout and the G-code for the same point; a user can compare a warning, the readout and the program directly (NFR-12). A project with **Zero at X** 0 reads the same as today. Every example's quoted positions change (17.99 less), as do the tutorial's and the screenshots'.

## Verification

Docs only. The inventory was read from the code at `main` (17dcdded) with one search pass of `crates/` and `apps/` and a check of the key claims: the work zero marker's place (`scene.rs`), `RotaryFrame::work` and the G-code conversion (`convert.rs`), the playback line and cursor readout (`playback_panel.rs`, `viewport_overlays.rs`), and `SetupFrame` and decision 0201. The counts are by site, approximate. Not verified: no program was run; the 52.900 and 34.910 figures are the PATH-QUALITY lane's, not re-measured; the "last bit" risk of Option C is a reading, not a measurement.
