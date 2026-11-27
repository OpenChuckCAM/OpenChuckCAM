# PROJUX-B-2: Lengths and feeds are shown and typed in inches as a user setting, a unit can be typed with a value, and the setup sheet can be written in inches (UI-18, UI-26, NFR-05, NFR-07, GC-07, GC-12, NFR-12; issue #704; lane PROJECT-UX-B)

- **Status:** Proposed by lane PROJECT-UX-B on 27 Nov 2026. It is built as the project owner answered on 25 Nov 2026, relayed by the coordinator (decision 0049): Q2a C (inches everywhere in the interface and on the setup sheet; messages and results text stay in millimetres), Q2b A (the units that can be typed), and Q2c A, A, A (the sheet's unit is chosen in the export dialog, defaulting to the display unit, with `--sheet-units` on the command line defaulting to mm; millimetres in brackets on the values entered at the machine, with a banner; no units comment in the G-code). The owner has not yet accepted the record itself (OI-9943).
- **Date:** 27 Nov 2026.
- **Requirements:** UI-18 (inch display and entry as a user setting; a unit typed with a value; values stored in millimetres, P2), UI-26 (number boxes read a sum), NFR-05 (millimetres internally), NFR-07 (a clear refusal), GC-07 and NFR-06 (deterministic output), GC-12 (the setup sheet), NFR-12 (the user verifies the program).
- **Builds on:** [decision 0206](0206-math-in-number-boxes.md) (sums in number boxes), [decision 0233](0233-unconfirmed-edits-before-leaving-a-form.md) (the guard on unconfirmed fields), [decision 0267](0267-output-file-names-sheet-hashes-and-staged-writes.md) (per-user choices that are inputs of the written files), [decision 0677](0677-keyboard-focus-tooltips-and-tool-windows.md) (keyboard).
- **Amends:** the specification's section 8 (*Sums in number boxes*: a unit at the end is read; *Typed units*; a new *Shown in inches* bullet), section 10 (the footer's switch) and the D4 row. No requirement, architecture or project-format change.
- **Open items:** OI-800 is narrowed: units are now read, while percent and functions stay open. Adds OI-9943 (the owner's acceptance) and OI-9944 (the owner's check on a native build). Provisional numbers; the coordinator assigns the record number and the OI numbers at release, in merge order.

Numbering: provisional. No project schema number: the choice is a per-user setting (`UserSettings::units`, which existed and was always "mm"), and the project stores millimetres as before. No library-family change, so no version bump (decision 0048).

## Context

Every length and feed is shown and typed in millimetres. The footer read "mm · deg" and could not be changed. A number box read a sum but refused a unit ("0.005 in" could not be read, OI-800). UI-18 asks for inch display and entry as a user setting, and for a unit typed with a value to be accepted, with values stored in millimetres (NFR-05).

## Decision

This lane's second pull request (B2) builds items 1 to 6. A third (B2b) builds item 7, the setup sheet.

1. **A per-user choice, "mm | in", in the footer (section 10).** It is remembered in the user's settings (`units = "mm"` or `"in"`). Any other text reads as millimetres. The project, the libraries and the G-code are unchanged by it.
2. **Number boxes show and read the chosen unit.** A box whose stored unit is millimetres shows inches to four places, and a feed box shows inches per minute to one place. Degrees, rpm and plain numbers are unchanged. The box hands its caller the entry in millimetres, as it always did, so every form works unchanged. The conversion is exact (25.4 mm to the inch) and kept to a billionth, so 0.1 in is stored as 2.54 mm.
3. **A value confirmed unchanged is never stored again.** Enter on the shown "0.0315 in" gives back the stored 0.8 mm exactly, not 0.8001 mm. Showing a value rounded in inches can never change it.
4. **A unit can be typed with a value or a sum (Q2b A).** A length box reads `mm`, `cm`, `in` and `"`, a feed box `mm/min` and `in/min`, whichever unit is shown. There is one unit, at the end: "0.005 in", "1/8 in", "1/8"", "3 + 0.5 mm". The box then says what it stored: "Stored as 0.127 mm", or "Stored as 0.2 mm (0.0079 in)" when the shown unit is another. Any other word is refused like an unreadable entry, with a line naming the units the box reads, and the last value stays (NFR-07). A plain number in millimetres is still stored as typed (decision 0206).
5. **What else follows the switch:**
   - the footer's tolerance;
   - the tool library's cutting data: the table, its tooltip and the entry boxes, whose draft is read in the unit it was opened in;
   - the model's Size, its placement steps and their drag boxes;
   - the tool profile's point boxes and the diameter on its preview;
   - the playback readout: X, Y, Z and the feed.
6. **What stays in millimetres (Q2a: messages and results text):**
   - text the application writes: messages, notes, the held span and "Kept clear", strategy notes;
   - the Results tab, including its deviation range boxes;
   - the setup sheet's own sentences in millimetres, until item 7;
   - the project files and the G-code (always `G21`);
   - the command line.

   This text carries its unit ("mm") everywhere, so a number is never read in the wrong unit.
7. **The setup sheet in inches (Q2a C and Q2c A, A, A; pull request B2b).** The export dialog has "Setup sheet units: mm | in", defaulting to the display unit. `openchuck-cli` has `--sheet-units mm|in`, defaulting to mm. The sheet's unit is an input of the export (as the file name templates are, decision 0267): the same project, post, version and sheet unit give the same bytes (GC-07). The existing millimetre goldens are unchanged, and inch goldens are added. An inch sheet opens with "Lengths on this sheet are in inches. The program is in millimetres (G21)." Every value carries its unit, and the values entered at the machine also carry their millimetres in brackets: "0.7500 in (19.050 mm)" for the work zero, part offset, safe Z, clearance radius, tool lengths and stock size. The G-code gets no units comment.
8. **Switching units with text typed in a box** resolves that text first, as leaving a form does (decision 0233): Keep stores it in the unit it was typed in, and then the switch goes ahead. A number typed in millimetres is never read in inches.

Not chosen:
- Messages and results text in inches (Q2a B): they carry ready-made "mm" text from the application and the CAM libraries.
- A unit after each number of a sum (Q2b B).
- A per-project unit: UI-18 asks for a user setting, and a project would need a schema number.

## Verification

Tests written first from UI-18's verification row ("Inch display round-trips to millimetres; a value typed with a unit converts exactly; an unknown unit is refused with a message"), named by requirement. They use generated projects only. Times are uninstrumented (`cargo nextest run`).

- `openchuck-ui`, `src/units.rs` unit tests:
  - `req_ui_18_req_nfr_05_inch_display_round_trips_to_millimetres`;
  - `req_ui_18_a_value_typed_with_a_unit_converts_exactly`;
  - `req_ui_18_req_nfr_07_an_unknown_unit_is_refused_with_a_message`;
  - `req_ui_18_the_note_says_what_was_stored`;
  - `req_ui_18_hints_show_their_number_in_the_users_unit`;
  - `req_ui_18_readouts_and_read_only_lengths_follow_the_unit`;
  - `req_ui_18_the_setting_reads_mm_or_in_and_anything_else_as_mm`.
- `openchuck-ui`, `shell/units_tests.rs`, through real frames on the operation inspector:
  - `req_ui_18_req_nfr_05_a_length_box_shows_and_reads_inches_and_stores_millimetres`: the saved project holds 2.54 mm;
  - `req_ui_18_a_shown_value_confirmed_unchanged_stores_nothing_new`: no new revision;
  - `req_ui_18_a_unit_typed_with_a_value_converts_and_says_what_was_stored`;
  - `req_ui_18_req_nfr_07_an_unknown_unit_is_refused_and_the_last_value_kept`;
  - `req_ui_18_the_footer_switch_changes_the_units_and_remembers_them`;
  - `req_ui_18_req_ui_07_switching_units_resolves_a_typed_value_first`.
- `openchuck-ui`, `model_panel` tests: a placement step reads in inches.
- Changed: `shell::numbox_tests::req_ui_26_the_last_good_value_is_kept_while_an_entry_is_unreadable` typed "5 mm" as an unreadable entry (decision 0206: no units read). This record makes "5 mm" readable, so the test now types "5 ft". It still checks that an unreadable entry keeps the last good value.

Results, commands, environment and coverage are added when the checks have run before the push.

Not verified here: native drawing and keys, which are the owner's check (OI-9944).

## Owner check

What to check, and the result expected:
1. The footer's **mm | in** switch changes every number box of an operation, the tool library's cutting data and the playback readout to inches, and back.
2. Typing `1/8 in` in a length box in millimetres stores 3.175 mm and says so.
3. Typing `0.2 mm` while inches are shown stores 0.2 mm and says "Stored as 0.2 mm (0.0079 in)".
4. Pressing Enter on a value shown in inches without changing it leaves the project unchanged: the title bar does not say "Edited, not saved".
5. Typing `2 ft` is refused with a line naming the units, and the last value stays.

Steps, on a nightly build of the pull request (Windows):
1. **File > Open project…** on a copy of `testdata/examples/baluster/baluster-project`.
2. Select the first operation. Click **in** at the bottom right (check 1). Open **Tools > Tool library…** and look at the cutting data, then **Simulate** and play a few moves.
3. Click **mm**. In **Stepover** type `1/8 in` and press Enter (check 2).
4. Click **in**. In **Stepover** type `0.2 mm` and press Enter (check 3).
5. Save (Ctrl S). Click in **Stepover** and press Enter (check 4).
6. Type `2 ft` and press Enter (check 5).

A pass is all five as expected. A fail is anything else: report the step, what was shown (a screenshot helps) and the version from Help > About.
