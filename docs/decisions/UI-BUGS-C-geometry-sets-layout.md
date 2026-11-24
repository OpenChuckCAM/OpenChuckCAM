# UI-BUGS-C: The operation's Geometry section shows each selection set once, on its own line, with its choice under it, and a set on Ignore greyed and struck through (UI-07, UI-16, OP-02, OP-21; issue #759; lane UI-BUGS)

- **Status:** Approved by the project owner on 24 Nov 2026 with the Ignore change, relayed by the coordinator (decision 0049): D1 A, D2 A, and a set on Ignore is drawn greyed and struck through instead of warned about (design item 4). Proposed by lane UI-BUGS on 24 Nov 2026, design first as the owner asked (Q6 option A). The build follows as the lane's third pull request, after lane FACEFLOW's pull request #854, which changes the same section, has merged.
- **Date:** 24 Nov 2026.
- **Requirements:** UI-07 (the inspector's fields and choices), UI-16 (the reference screens: nothing cut off or overlapping at the inspector's widths), OP-02 (selection sets in an operation's roles), OP-21 (Avoid and Check, and what a set limits).
- **Amends:** the layout of the Geometry section that [decision 0215](0215-selection-sets-in-the-inspector-and-direct-set-editing.md) and the specification's section 11.4 describe; the roles, the choices and every message stay as they are. [Decision 0440](0440-inspector-label-and-entry-widths.md) (the label column) is followed.
- **Open items:** adds OI-9983 (the owner's approval of this design), closed by the approval; adds OI-9985 (D3: what becomes of the operation's own warning for a set on Ignore), which the build waits on.

Numbering: provisional. The coordinator assigns the record number and the OI number at release, in merge order. No project schema number and no library change: only how the section is drawn changes.

## Context

Issue #759 (user report, Windows, 0.54.0): with several selection sets in an operation's roles, the Geometry section is "very messy". On `main` (`aa99eab`), the section draws, for each role (Cut on, Contain within, Avoid, Check):

1. one wrapped row: the role's name in the label column, then a small button per set ("name ×") and an "add…" list. When the buttons do not fit, they wrap to the next line at the pane's left edge, under the label;
2. for each set in Avoid or Check, a second row: the set's name again, muted, then Cut short, Warn only and Ignore. This row wrapped left of the label column too (#609, fixed in the lane's second pull request);
3. under each set, its own warnings: "The set 'X' limits nothing." for Ignore, and the long note on Warn only's sampling, repeated for every set on Warn only;
4. the role's refusals and each set's reach notes, with their links.

So each set in Avoid or Check is named twice, the buttons wrap under the label, and a note shared by every set on Warn only is repeated. The left half of the diagram shows it with a synthetic example at the inspector's 340 px.

![The Geometry section today and as proposed](UI-BUGS-C-geometry-section.svg)

## Proposed design (for the owner's approval)

1. **Each role on one line:** its name in the label column and an "Add a set… ▾" list in the value column, as the list is today.
2. **One line per set, under its role, in the value column:** the set's colour, its name, and × at the right end to take it out of the role (the button's tooltip as today: "Take it out of this role"). A set that is missing or has elements not found keeps its red border and reason, on its line.
3. **A set's choice under its name:** for Avoid and Check, Cut short, Warn only and Ignore, segmented, just under the set's name, in the value column. For Contain within on swarf, Limits the pass and Limits the tip stay on the role's own line, as they concern the role.
4. **A set on Ignore is greyed and struck through, with no warning line (the owner's change).** Its colour dot is grey and its name struck through and muted, as a disabled operation's name is in the project panel (the specification's D5, decision 0029 item 10). The line's tooltip, on hover and on keyboard focus (decision 0677 item 3), says "Ignored: this set limits nothing". The warning "The set 'X' limits nothing." is no longer drawn. A set's other warnings, a problem and its reach notes, stay once under it with their links.
5. **A note shared by the role once per role:** Warn only's sampling note is shown once under the role when any of its sets is on Warn only.
6. **Nothing wraps left of the value column**, at any inspector width (the lane's second pull request already makes wrapped rows stay in the value column, #609).
7. **Not by colour alone.** An ignored set is told apart by the strike-through and the muted name as well as the grey dot, and the Ignore choice under it shows as selected. The line's name is a Tab stop with its tooltip, as a geometry role's label is (decision 0677 item 3), and it gives its accessible name as "Bead, ignored: this set limits nothing", as the operation eye gives its state with its name (`WidgetInfo::selected`). Screen readers themselves stay with OI-2110 (2027.1); this record only makes sure the line carries its state in words.

The roles, the choices, what each does, the other messages' words and the keyboard order (role, its list, then each set's line from left to right) do not change; only where they are drawn does.

## Questions for the owner, and the answers

Asked on 24 Nov 2026 and answered on 24 Nov 2026, relayed by the coordinator.

- **D1.** The layout above (A, recommended), or keep today's layout and only stop the buttons wrapping under the label (B)? **Answer: A.**
- **D2.** Each set's line shows its colour dot (A, recommended: the colour the view draws the set in, decision 0216), or its name only (B)? **Answer: A,** with the change in design item 4: no "Limits nothing." warning; an ignored set's line greyed and struck through.
- **D3 (OI-9985, open).** The owner's change removes the Geometry section's line. The operation also has its own warning for a set on Ignore, made when it is generated (decision 0176's wording, `operation.rs` `limits_nothing`): "The avoid set 'Bead' limits nothing, because it is set to Ignore: the tool is not kept clear of it. Set it to Cut short or Warn only beside the set in the operation inspector's Geometry section, or take the set out of the operation." It is a warning, so it marks the operation amber, counts in Messages, Results, the project badge, the checks before writing and the command line's `warning:` lines. Should it **(A, recommended)** become information: said once in Results and on the command line as a `note:`, no amber mark and no count, so a forgotten Ignore can still be found after the run; **(B)** go entirely; or **(C)** stay a warning? A boundary set of swarf with both Limits the pass and Limits the tip off ("The boundary set 'Fence' limits nothing, because both … are off") is not a choice of Ignore and is not changed by this record unless the owner says so.

## Consequences

What depends on the removed line, and what the build changes with it:

- **The line's message**, "The set '{set}' limits nothing." (`apps/openchuck-ui/src/shell/inspector.rs`), goes. The message review's rows V151 (`docs/message-review/ui-other.md`, KEEP-WARNING) and U62 (`docs/message-review/ui.md`, TO-INFO?) are marked as settled by this record.
- **Ignore's tooltip** (row V135) says "The set limits nothing: the tool is not kept clear of it. The operation warns that it limits nothing."; its second sentence follows D3 (for A: "Results says so in a note.").
- **Tests that read the line:** `apps/openchuck-ui/src/shell/frame_tests.rs` (the rib and fence cases, which look for "The set 'Rib' limits nothing." and "The set 'Fence' limits nothing.") are rewritten first, to look for the struck-through name, its tooltip and its accessible name, and to find no warning line.
- **Counts and the badge:** the removed line was drawn in the inspector only; it never counted in Messages, Results or the project badge, so they do not change from it. They change only through D3 (the operation's own warning).
- **The user guide:** the operations reference says, for Ignore, "The set limits nothing, and the operation warns that it limits nothing." (`user-docs/src/reference/operations.md`); it is reworded with the build, after D3. No guide page quotes the removed line itself.
- **The specification** (section 11.4's Geometry bullet, `docs/ui-design/ui-specification.md`) is updated with the build to say how a set on Ignore is drawn.
- **A set on Ignore that limits nothing needs no other message beyond D3:** it is the user's own choice, shown on its line, and the path is made as if the set were not there; the set is still checked as any set is (its model, version and contents, decision 0229 item 1), and those problems keep their own messages.

- About seven pages of the user guide show this section (the operations reference, the selection-sets, swarf, face flow and limit-to-a-region examples and how-to); their screenshots are captured again in the build, and their text is checked.
- The inspector's layout tests gain a case with several sets per role at 340 px and at the default width.

## Verification

None yet: the design is approved, and the build's tests are written first from this record in the lane's third pull request, after #854 and the owner's answer to D3 (OI-9985).
