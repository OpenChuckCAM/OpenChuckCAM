# 0xxx: Keyboard focus that the user can see: tool windows take the keyboard, Tab scrolls to the control, tooltips open on focus (UI-14, UI-10, UI-16; issues #577, #578, #580, #581; lane A11Y-FOCUS)

- **Status:** Proposed by lane A11Y-FOCUS on 23 Nov 2026. The owner answered questions Q1 to Q6 on 23 Nov 2026, relayed by the coordinator: Q1 option B (not the recommendation), Q2 to Q6 option A. Everything in *Decision* is built in the lane's pull request as answered. The owner is to be asked to accept the record.
- **Date:** 23 Nov 2026.
- **Requirements:** UI-14 (every command from the menus and the keyboard, every control by keyboard, anything shown on hover also shown on keyboard focus), UI-10 (an unavailable step says why), UI-16 (the reference screens at 1366 by 768). Specification sections 5 (the primary action), 12 (supporting windows) and 16 (keyboard and accessibility).
- **Builds on:** the whole-application user interface review (lane UIR, 7 Nov 2026; report on branch `claude/lane-uir-report`, `docs/handoffs/lane-uir-report.md`, findings F1, F2, F4 to F12); [decision 0164](0164-tool-windows-inside-the-main-window.md) (tool windows inside the main window); [decision 0440](0440-inspector-label-and-entry-widths.md) (inspector widths: a cut default hint or list text is the known cost, its whole text in the tooltip); OI-536 (Esc in a tool library text field ends the edit only).
- **Open items:** adds OI-xxx1 ("Simulate from" not built, the owner's answer Q5 A) and OI-xxx2 (the 1366 by 768 drawing nits F12 c to e). Leaves OI-539 open (the committed keyboard-only run is not part of this lane) and OI-2110 open (screen readers, moved to 2027.1 with #579).

Numbering: provisional. The coordinator assigns the record and `OI-` numbers when the pull request is released (owner's rule of 23 Nov 2026). No project schema number: nothing stored changes. No library API changes: only `openchuck-ui` changes.

## Context

The review found that a keyboard user cannot see where focus is (F1, #577), never sees most tooltips, including why a step is unavailable (F2, #578), and lands behind a tool window that has just opened (F5, #580), with smaller problems in #581. egui 0.37 does none of these by itself: it scrolls to a widget only when asked, shows a tooltip only on hover, gives a new window no focus, and gives a disabled widget no focus at all. The lane also found that the shell's single-letter keys (N, G, F2, Del, F, V, Q to R) stopped working once Tab had reached any control, because the shell asked egui whether *any* widget had the keyboard, where its own comments say only a text field should take those keys.

## Decision

1. **A tool window that opens takes the keyboard (#580; the owner's answer Q1 B).** When the export, import, New operation, tool library or machine library window opens, nothing behind it keeps the keyboard (a field that had it applies its value, as on any focus loss) and no control in the window is focused; the first Tab (or Shift Tab) goes to the first control of its body (the import file field, the New operation name, the tool library search, the machine library's first button, the export dialog's first choice), not to the File menu behind it. A click elsewhere first ends that. Built once in `floating.rs` (`focus_starts_here` marks where the body starts).
   - With no field editing when the window opens, the first Esc closes it (item 9). Typing before the first Tab goes to the main window's keys behind the window, as it did before this lane.
2. **Tab scrolls the control it reaches into view (#577).** Every scroll area in the application keeps the control that has just taken the keyboard in view, scrolling only as far as needed and without animation (`focus.rs`, `show_keeping_focus`). This covers the inspector, the export dialog with its acknowledgement at the foot, the machine and tool libraries, the dock, the project panel and the other dialogs. A test fails if a new scroll area is shown any other way.
3. **Tooltips open on focus as well as hover (#578).** Every tooltip is given through one helper (`focus.rs`, `Tip::tip` and `tip_ui`): while its control has the keyboard the tooltip is shown at once; otherwise it opens on hover as before. A test fails if a hover-only call comes back. This is what `field_row::tip` and the About window already did; they now use the helper.
   - **Unavailable steps (UI-10).** A step tab that is out of reach stays a Tab stop, draws the focus ring, and shows its reason on focus; Enter on it does nothing (the owner's answer Q2 A).
   - **Other disabled controls** keep egui's rule: they take no focus, so their reason is read on hover only. The primary button's reason is also in the footer status (specification 5). Q2 option B, not chosen, would have changed this.
4. **Letter keys work while a button has the keyboard.** The single-letter and function keys of specification 16 (N, G, F5, F2, Del, F, V, Q, W, E, R) are left only to a text field with the keyboard, as the code's own comments say; a button, menu or step tab that Tab has reached no longer swallows them. Space, Enter, the arrows and Esc still belong to a focused control first (they press it or move focus), and the playback keys still wait for no control to have focus.
5. **Help > Keyboard shortcuts lists every key that is built (#581 F7):** F and V, Space and the arrows for playback, Ctrl Enter, Ctrl +, Ctrl − and Ctrl 0 (egui's interface scale keys, kept within the scale range), Ctrl Shift Z, Q W E on an edge, and Tab and Shift Tab. With item 10, 1, 3, 7 and 0 too, and the Esc row says what Esc does in a tool window (item 9). The keys of section 16 that are not built (Ctrl H, Ctrl K, S, M and B) are left out.
6. **Every command is in a menu (#581 F8):** Project > Add setup (as the Setup step's "+ Add setup", with the same reason when it is unavailable), and Edit > Delete deletes a selected selection set as Del does.
7. **The New operation dialog follows the primary-action rule (#581 F10):** "Add" is filled and bottom right with Cancel to its left; Tab reaches Cancel and then Add. When it cannot add, it is an ordinary disabled button whose tooltip says why ("Choose a setup first", "Choose a strategy this setup can use").
8. **Tab follows reading order (#581 F4; the owner's answer Q3 A).** In the main window: menus, step tabs with the G-code status, the health badge, the workflow actions, the project panel, the view's toolbar and the view, the dock, the inspector, the footer (status first, the primary button last); then round to the menus. Shift Tab goes the other way; with nothing focused it starts at the primary button. In a tool window: its body's parts in reading order (the import window's file header, then its body; the tool library's list, form and data; the machine library's list and form), then its footer's buttons left to right, then round inside the window. egui moves focus in drawing order, and panels are drawn before the middle of a window, so `focus.rs` routes Tab between regions: each region is drawn through `focus::region`, and when Tab leaves a region a stand-in with no place on screen holds the keyboard for one frame and the next region's first (or last) control takes it; a region with no control, or not drawn, is passed. Inside a region the drawing order stands, and rows that were drawn right to left (the main footer's right-hand items, the Operations step's actions, the export, import and tool library footers) are drawn left to right at the right end (`focus::row_at_right`), so they look the same and read left to right.
9. **Esc in tool windows (#581 F11; the owner's answer Q6 A).** Esc does what the window's Close or Cancel does, through the same guard: in a text field it ends the edit only, and a second Esc closes; a tool or machine library draft with changes asks its "Discard the changes?" question first; the export dialog closes. It applies to the window on top only, not while a pop-up is open (that Esc closes the pop-up) or a question is open in front of the window. Built once in `floating.rs` for every tool window; the tool library's own Esc now only cancels a stage of its unused-tools cleanup.
10. **The digit view keys (#581 F6; the owner's answer Q4 A).** 1, 3, 7 and 0, on the top row or the number pad (egui 0.37 reads both as the same key), turn the 3D view to the front, right, top and isometric views while no text field has the keyboard, as F and V do, and only where the toolbar's view buttons work (a model in the project, not Setup's two panes). With Alt or Ctrl the digits keep their meanings (the steps; collapsing panels). Specification section 16 is edited to say so.
11. **"Simulate from" (#581 F9; the owner's answer Q5 A)** is not built in 2026.2; OI-xxx1 records it, and the specification is unchanged.
12. **The 1366 by 768 nits (#581 F12).**
   - a. A field's sentence wrapping back under its label (Model > Units, the grip warning) was fixed on `main` before this lane by issue #635 (`field_row::reason_line` draws the sentence as one block under the field). Nothing changed here.
   - b. "Automatic (too…" and "Stock diamete…" are the cut list text and default hint that decision 0440 (accepted) accepts as its known cost, with the whole text in the tooltip; that tooltip now also opens on focus (item 3). Nothing changed here.
   - c to e. The Setup front pane's "Held" leader and "A axis" label, the 3D view's "A0" mark over the A+ label, and the project tree's last line under the legend with no scroll cue are not changed by this lane: OI-xxx2.

## Questions for the owner, and the answers

Each had a recommendation. The owner answered on 23 Nov 2026 (relayed by the coordinator): **Q1 B, Q2 A, Q3 A, Q4 A, Q5 A, Q6 A**. Q1 B was not the recommendation; it is built as answered.

**Q1 (#580). Where does keyboard focus go when a tool window opens?**

- A (recommended). The first control in the window's body. Typing goes into it if it is a text field.
- **B (the owner's answer, built).** The window is focused but no control; the first Tab goes to its first control.
- C. The window's primary button.

**Q2 (#578). Controls that are unavailable.** egui gives a disabled control no focus, so its reason cannot be reached by Tab.

- **A (recommended; the owner's answer, built).** An unavailable step tab stays a Tab stop, shows its reason on focus and does nothing on Enter. Other disabled buttons stay out of the Tab order; the primary button's reason is in the footer status.
- B. Every disabled button with a reason is a Tab stop (more stops, a larger change).
- C. Unavailable controls stay out of the Tab order; the footer status is the only keyboard route to a reason.

**Q3 (#581 F4). What is reading order for Tab in the main window?** Today: menus, health badge, workflow actions (right), step tabs, footer, project panel, inspector, dock, view toolbar, view. Dialogs: footer buttons right to left, then the body.

- **A (recommended; the owner's answer, built).** Menus, step tabs, G-code status and health badge, workflow actions, project panel, view toolbar, view, dock, inspector, footer (primary last). Dialogs: the body, then the footer buttons left to right.
- B. Strictly by screen position, top to bottom then left to right.
- C. The main window as it is; only dialogs change (body first).

**Q4 (#581 F6). The numpad view keys.** egui 0.37 cannot tell numpad digits from top-row digits.

- **A (recommended; the owner's answer, built).** Specification 16 reads "1, 3, 7, 0 (top row or numpad): front, right, top, isometric", only while no text field has the keyboard (as F and V). Alt and Ctrl digits keep their meanings.
- B. The keys are dropped from section 16; views by toolbar, View menu and view cube only.
- C. Section 16 is kept and an open item waits for egui numpad key codes.

**Q5 (#581 F9). "Simulate from"** (specification 11.5, workflow bar) is not built and has no open item.

- **A (recommended; the owner's answer, built).** An open item; not built in 2026.2; specification unchanged.
- B. It is removed from the specification.

**Q6 (#581 F11). Esc in tool windows.** Today only the tool library closes on Esc.

- **A (recommended; the owner's answer, built).** Esc does what each window's Close or Cancel does, with the same guard: the first Esc ends a text edit; a window with unsaved edits (a tool or machine library draft) asks its existing "Discard the changes?" question; the export dialog closes. Section 16's Esc row says so.
- B. Esc closes a window only when it has no unsaved edits; with edits it does nothing.
- C. As today (the tool library only).

## Consequences

- Keyboard users see where focus is, read every tooltip, and start in a window that has just opened.
- A tool window opens with no control focused; the New operation dialog's buttons moved, so the tutorial's screenshots were re-captured in the same pull request.
- Region routing holds the keyboard on an invisible stand-in for one frame when Tab crosses a region; a new panel or dialog part must be drawn through `focus::region` to keep its place in the order (the reading-order tests fail otherwise).
- egui scrolls one frame after the key (it applies a scroll target when the next frame starts), so the control is drawn in view two frames after Tab; at the application's frame rate this is not visible.
- No G-code, project file, settings file or library API changes.

## Verification

Lane A11Y-FOCUS, branch `claude/lane-a11y-focus`, from `main` `8f61706`, 23 Nov 2026, Linux container, Rust 1.97.0, egui 0.37.0.

**Tests written from the requirement first** (`openchuck-ui`; each watched failing on the unchanged code unless said otherwise):

| Test | Covers | Before the change |
| --- | --- | --- |
| `shell::focus_tests::req_ui_14_a_new_window_takes_the_keyboard` | item 1, #580 (Q1 B): when each of the five tool windows opens nothing has focus; the first Tab goes into its layer and the next stays there | failed on the first version (the first Tab went to the File menu); rewritten for answer B together with the change |
| `shell::focus_tests::req_ui_14_tab_scrolls_the_focused_control_into_view` | item 2, #577: Tab through the main window (an operation in the inspector), the export dialog and the machine library at 1366 by 768; no stop is cut by its clip | failed (inspector fields below y 718 stayed out of view); checked again with the fix switched off |
| `focus::tests::req_ui_14_tab_scrolls_the_focused_control_into_view` | item 2 in a bare scroll area | failed with the fix switched off |
| `focus::tests::req_ui_14_every_scroll_area_keeps_the_focused_control_in_view` | item 2: no scroll area in the application is shown without it (source scan) | written with the change; it caught a dock scroll area the first pass missed |
| `focus::tests::req_ui_14_a_tip_opens_on_keyboard_focus` | item 3: a tooltip opens when Tab reaches its control, with no pointer | written with the helper |
| `focus::tests::req_ui_14_every_tooltip_also_opens_on_focus` | item 3, #578: no hover-only tooltip call is left (source scan) | written with the change; the unchanged sources had 142 such calls |
| `shell::focus_tests::req_ui_10_req_ui_14_an_unavailable_steps_reason_opens_on_focus` | item 3, UI-10: in the empty state Tab reaches Setup and shows why; Enter does not open it | not run on the unchanged code: written with the change; there the tab took no focus, so Tab could not reach it |
| `shell::focus_tests::req_ui_14_letter_keys_work_while_a_button_has_the_keyboard` | item 4: with a button focused by Tab, N opens New operation | failed |
| `shell::focus_tests::req_ui_14_the_shortcut_list_names_every_key` | item 5, #581 F7 | failed (Ctrl Shift Z and the others missing) |
| `shell::focus_tests::req_ui_14_every_command_is_in_a_menu` | item 6, #581 F8: Project > Add setup adds the setup; Edit > Delete deletes a selected set | failed ("Add setup" not in the menu) |
| `shell::focus_tests::req_ui_10_the_new_operation_dialog_has_its_primary_action_bottom_right` | item 7, #581 F10: Add right of Cancel, at the foot, right-most, filled; Tab reaches Cancel then Add | failed (Add left of Cancel) |
| `shell::focus_tests::req_ui_14_tab_follows_reading_order` | item 8, #581 F4 (Q3 A): Tab visits the main window's nine regions in order and wraps; Shift Tab the other way, starting at the footer | written with the change; the unchanged order is the review's K4 (menus, badge, actions, steps, footer, …); its first run caught the badge counted with the actions |
| `shell::focus_tests::req_ui_14_a_dialog_reads_its_body_then_its_footer` | item 8: in each of the five tool windows, the parts in order, the footer last, then round inside the window | written with the change (the review's K4: the footer came first) |
| `shell::focus_tests::req_ui_14_esc_closes_every_tool_window_as_its_close_does` | item 9, #581 F11 (Q6 A): Esc closes each of the five; in the import file field the first Esc ends the edit; a machine library draft asks "Discard the changes to this machine?" | written with the change |
| `viewport::tests::req_ui_14_the_digit_keys_turn_to_the_standard_views` | item 10, #581 F6 (Q4 A): 1, 3, 7, 0 give front, right, top, isometric; with Ctrl nothing | written with the change; its first run caught a missing condition in the test (no model) |

Changed existing tests: `shell::keys_tests::req_ui_14_the_keys_are_left_to_any_dialog_or_window_in_front_of_the_view` asserted that Esc does *not* close the export and New operation windows; with Q6 A it now asserts that it closes them (and still not the error dialog, which is not a tool window), and that the keys behind them stay untouched. `shell::four_step_tests::req_ui_14_tab_reaches_the_simulate_actions_in_reading_order_and_enter_opens_the_dialog` is unchanged and passes; it found that the router first needed a key press per region crossing and lost its place on an overlay over the view, both fixed (a crossing redraws the frame at once with `request_discard`; a control outside every region belongs to the region focus was in). `req_tl_01_esc_closes_the_library_window_unless_a_field_has_the_keyboard` is as on `main` again.

**Runs (uninstrumented, `cargo nextest run -p openchuck-ui`):** 738 of 738 passed, 1 skipped, 407 s, before the F7, F8 and F10 tests were added; the four slow tests are the existing playback tests over 60 s, untouched. The new tests each take under 3 s alone; the shell scroll walk (about 600 Tab presses) takes about 12 s with other tests running beside it. No new long-running test. `cargo fmt --all`, `cargo clippy --workspace --all-targets` (no warnings), `cargo xtask deps`, `licences` and `trace` (105 of 105 P1 requirements) pass; `cargo xtask records` passes except for the two provisional `OI-xxx` numbers, which it cannot read until they are assigned.

**The real application:** the 18 tutorial screens were re-captured from the release build on Xvfb with Mesa lavapipe (`user-docs/tools/capture.py`, 1920 by 1080; the script clicks "Add" at its new place), last at branch commit `298d24f` after the owner's answers were built. The capture ran the whole tutorial through to written G-code; screen 09 shows the New operation dialog with Add filled at the bottom right, and no screen shows a stray tooltip. The user guide builds and `user-docs/tools/check.py --screens strict` passes on the branch. The tutorial's `CAPTURED.toml` names the commit the screens were captured at, `298d24f` on this branch; a squash merge drops it, so the screens are marked reviewed against `main` after the merge by the docs lane (DOCS-SPOT). The home page's two hero screens were not re-captured (no window or focus in them).

**Coverage (instrumented, `openchuck-ui`'s own tests, a lower bound), on the final code:** 741 of 742 passed (1 961 s, `--no-fail-fast`); `area_view_tests::req_ui_11_another_setup_of_the_same_run_draws_its_own_areas` hit its 120 s settle deadline under instrumentation, as in the first run, and passes uninstrumented. `openchuck-ui` 90.4 % of lines on its own tests (below the 95 % target before this lane as well). `focus.rs` 90.8 %: the lines not reached are the two `const fn` region constructors (evaluated at compile time), the clean-up of an empty region with no region left to try (defensive), and the router's fallback for Tab from a control outside every region with nothing drawn after it (the Simulate walk reaches the regions through the region-end path instead). `floating.rs` 98.7 %: one defensive early return. The new menu, key list, footer and New operation lines of `shell/frame.rs` and `run_panel.rs` are reached by their tests.

**Not verified:** a real keyboard on Windows and macOS, and contrast of the focus ring on a real screen (OI-493, OI-623); the keyboard-only run through every step as a committed test (OI-539); the example figures of the user guide were not re-captured (their windows were opened by the mouse; the only change they could show is a focus outline on a window's first control); F12 c to e (OI-xxx2).
