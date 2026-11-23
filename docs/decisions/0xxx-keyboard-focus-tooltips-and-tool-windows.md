# 0xxx: Keyboard focus that the user can see: tool windows take the keyboard, Tab scrolls to the control, tooltips open on focus (UI-14, UI-10, UI-16; issues #577, #578, #580, #581; lane A11Y-FOCUS)

- **Status:** Proposed by lane A11Y-FOCUS on 23 Nov 2026. Items 1 to 7 of *Decision* follow requirement UI-14 and specification section 16 as written and are built in the lane's pull request. Items 1 and 3 also make a choice the owner is asked to confirm (questions Q1 and Q2 below; the recommendation is built). Questions Q3 to Q6 are the owner's and nothing is built for them until they are answered.
- **Date:** 23 Nov 2026.
- **Requirements:** UI-14 (every command from the menus and the keyboard, every control by keyboard, anything shown on hover also shown on keyboard focus), UI-10 (an unavailable step says why), UI-16 (the reference screens at 1366 by 768). Specification sections 5 (the primary action), 12 (supporting windows) and 16 (keyboard and accessibility).
- **Builds on:** the whole-application user interface review (lane UIR, 7 Nov 2026; report on branch `claude/lane-uir-report`, `docs/handoffs/lane-uir-report.md`, findings F1, F2, F4 to F12); [decision 0164](0164-tool-windows-inside-the-main-window.md) (tool windows inside the main window); [decision 0440](0440-inspector-label-and-entry-widths.md) (inspector widths: a cut default hint or list text is the known cost, its whole text in the tooltip); OI-536 (Esc in a tool library text field ends the edit only).
- **Open items:** adds OI-xxx1 (the owner's answers to Q1 to Q6) and OI-xxx2 (the 1366 by 768 drawing nits F12 c to e). Leaves OI-539 open (the committed keyboard-only run is not part of this lane) and OI-2110 open (screen readers, moved to 2027.1 with #579).

Numbering: provisional. The coordinator assigns the record and `OI-` numbers when the pull request is released (owner's rule of 23 Nov 2026). No project schema number: nothing stored changes. No library API changes: only `openchuck-ui` changes.

## Context

The review found that a keyboard user cannot see where focus is (F1, #577), never sees most tooltips, including why a step is unavailable (F2, #578), and lands behind a tool window that has just opened (F5, #580), with smaller problems in #581. egui 0.37 does none of these by itself: it scrolls to a widget only when asked, shows a tooltip only on hover, gives a new window no focus, and gives a disabled widget no focus at all. The lane also found that the shell's single-letter keys (N, G, F2, Del, F, V, Q to R) stopped working once Tab had reached any control, because the shell asked egui whether *any* widget had the keyboard, where its own comments say only a text field should take those keys.

## Decision

1. **A tool window that opens takes the keyboard (#580).** The export, import, New operation, tool library and machine library windows give keyboard focus to the first control of their body when they open: the import file field, the New operation name, the tool library search, the machine library's first button, the export dialog's first choice. Not the title bar's close box, and not a footer button. Built once in `floating.rs` for every tool window (`focus_starts_here` marks where the body starts). Q1 asks the owner to confirm the choice.
   - A consequence of the existing rule (specification 12.1, OI-536): where the first control is a text field, the first Esc ends that edit and a second Esc closes the window.
2. **Tab scrolls the control it reaches into view (#577).** Every scroll area in the application keeps the control that has just taken the keyboard in view, scrolling only as far as needed and without animation (`focus.rs`, `show_keeping_focus`). This covers the inspector, the export dialog with its acknowledgement at the foot, the machine and tool libraries, the dock, the project panel and the other dialogs. A test fails if a new scroll area is shown any other way.
3. **Tooltips open on focus as well as hover (#578).** Every tooltip is given through one helper (`focus.rs`, `Tip::tip` and `tip_ui`): while its control has the keyboard the tooltip is shown at once; otherwise it opens on hover as before. A test fails if a hover-only call comes back. This is what `field_row::tip` and the About window already did; they now use the helper.
   - **Unavailable steps (UI-10).** A step tab that is out of reach stays a Tab stop, draws the focus ring, and shows its reason on focus; Enter on it does nothing. Q2 asks the owner to confirm.
   - **Other disabled controls** keep egui's rule: they take no focus, so their reason is read on hover only. The primary button's reason is also in the footer status (specification 5). Q2 option B would change this.
4. **Letter keys work while a button has the keyboard.** The single-letter and function keys of specification 16 (N, G, F5, F2, Del, F, V, Q, W, E, R) are left only to a text field with the keyboard, as the code's own comments say; a button, menu or step tab that Tab has reached no longer swallows them. Space, Enter, the arrows and Esc still belong to a focused control first (they press it or move focus), and the playback keys still wait for no control to have focus.
5. **Help > Keyboard shortcuts lists every key that is built (#581 F7):** F and V, Space and the arrows for playback, Ctrl Enter, Ctrl +, Ctrl − and Ctrl 0 (egui's interface scale keys, kept within the scale range), Ctrl Shift Z, Q W E on an edge, and Tab and Shift Tab. The keys of section 16 that are not built (Ctrl H, Ctrl K, S, M, B and the numpad views) are left out.
6. **Every command is in a menu (#581 F8):** Project > Add setup (as the Setup step's "+ Add setup", with the same reason when it is unavailable), and Edit > Delete deletes a selected selection set as Del does.
7. **The New operation dialog follows the primary-action rule (#581 F10):** "Add" is filled and bottom right with Cancel to its left; Tab reaches Cancel and then Add. When it cannot add, it is an ordinary disabled button whose tooltip says why ("Choose a setup first", "Choose a strategy this setup can use").
8. **The 1366 by 768 nits (#581 F12).**
   - a. A field's sentence wrapping back under its label (Model > Units, the grip warning) was fixed on `main` before this lane by issue #635 (`field_row::reason_line` draws the sentence as one block under the field). Nothing changed here.
   - b. "Automatic (too…" and "Stock diamete…" are the cut list text and default hint that decision 0440 (accepted) accepts as its known cost, with the whole text in the tooltip; that tooltip now also opens on focus (item 3). Nothing changed here.
   - c to e. The Setup front pane's "Held" leader and "A axis" label, the 3D view's "A0" mark over the A+ label, and the project tree's last line under the legend with no scroll cue are not changed by this lane: OI-xxx2.

## Questions for the owner

Each has a recommendation. Q1 and Q2 are built as recommended (both are small to change); Q3 to Q6 wait for the answer.

**Q1 (#580). Where does keyboard focus go when a tool window opens?**

- **A (recommended, built).** The first control in the window's body. Typing goes into it if it is a text field.
- B. The window is focused but no control; the first Tab goes to its first control.
- C. The window's primary button.

**Q2 (#578). Controls that are unavailable.** egui gives a disabled control no focus, so its reason cannot be reached by Tab.

- **A (recommended, built).** An unavailable step tab stays a Tab stop, shows its reason on focus and does nothing on Enter. Other disabled buttons stay out of the Tab order; the primary button's reason is in the footer status.
- B. Every disabled button with a reason is a Tab stop (more stops, a larger change).
- C. Unavailable controls stay out of the Tab order; the footer status is the only keyboard route to a reason.

**Q3 (#581 F4). What is reading order for Tab in the main window?** Today: menus, health badge, workflow actions (right), step tabs, footer, project panel, inspector, dock, view toolbar, view. Dialogs: footer buttons right to left, then the body.

- **A (recommended).** Menus, step tabs, G-code status and health badge, workflow actions, project panel, view toolbar, view, dock, inspector, footer (primary last). Dialogs: the body, then the footer buttons left to right.
- B. Strictly by screen position, top to bottom then left to right.
- C. The main window as it is; only dialogs change (body first).

**Q4 (#581 F6). The numpad view keys.** egui 0.37 cannot tell numpad digits from top-row digits.

- **A (recommended).** Specification 16 reads "1, 3, 7, 0 (top row or numpad): front, right, top, isometric", only while no text field has the keyboard (as F and V). Alt and Ctrl digits keep their meanings.
- B. The keys are dropped from section 16; views by toolbar, View menu and view cube only.
- C. Section 16 is kept and an open item waits for egui numpad key codes.

**Q5 (#581 F9). "Simulate from"** (specification 11.5, workflow bar) is not built and has no open item.

- **A (recommended).** An open item; not built in 2026.2; specification unchanged.
- B. It is removed from the specification.

**Q6 (#581 F11). Esc in tool windows.** Today only the tool library closes on Esc.

- **A (recommended).** Esc does what each window's Close or Cancel does, with the same guard: the first Esc ends a text edit; a window with unsaved edits (a tool or machine library draft) asks its existing "Discard the changes?" question; the export dialog closes. Section 16's Esc row says so.
- B. Esc closes a window only when it has no unsaved edits; with edits it does nothing.
- C. As today (the tool library only).

## Consequences

- Keyboard users see where focus is, read every tooltip, and start in a window that has just opened.
- A tool window's first control shows the focus ring when the window opens; the tutorial's screenshots of those windows change (re-captured in the same pull request).
- egui scrolls one frame after the key (it applies a scroll target when the next frame starts), so the control is drawn in view two frames after Tab; at the application's frame rate this is not visible.
- No G-code, project file, settings file or library API changes.

## Verification

Filled in by the lane before the pull request; see below.
