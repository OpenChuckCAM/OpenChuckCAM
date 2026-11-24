# Release notes

What changed in each release of OpenChuckCAM, what it cannot do yet, and which Makera Z1 machine values were confirmed for it. The *Development version* section at the end lists what has changed since the last release. Each release adds a section headed with its version number, newest first, before it is released. The release workflow refuses to publish a release whose section is missing.

Every section ends with the same reminder: generated G-code must be verified by you before it is run on a machine.

## 0.55.0

The first release of OpenChuckCAM, an alpha (2026.1-alpha). It is for trying the program out and for reporting what you find. Expect rough edges, and expect the project file format and the generated programs to change in later releases.

### What is in this release

- **Models.** Import STL (ASCII and binary) and STEP files. A STEP part keeps its exact faces and edges for selection, drilling and swarf.
- **Setups.** Stock, [held regions](glossary.md#held-region) for the chuck and the tailstock, [tabs](glossary.md#tab), and a [machine profile](glossary.md#machine-profile) for each setup.
- **Strategies.** Rotary roughing; Facing (flat, round, the unheld end and box sides); helical, axial, indexed and simultaneous 4-axis finishing; rest machining; pencil clean-up; face flow finishing; edge and curve following; swarf; radial drilling. See [Operations and strategies](reference/operations.md).
- **Simulation** of the stock as it is cut, with deviation colours and a probe.
- **Posts** for the Makera Z1 and for LinuxCNC, a setup sheet with each program, and a [command line](reference/command-line.md), `openchuck-cli`, that regenerates, simulates and posts a project without a window.
- **Messages.** Errors, warnings and notes about each operation, setup and export, in one place. Possible collisions and partial paths must be acknowledged before any G-code is written.
- **Example projects** in a separate download, used by the tutorial and the worked examples.

### Known limitations

[Known limitations](start/limitations.md) lists what the program cannot do yet. The most important for this release:

- **Very little has run on a real machine.** The recorded runs are attended air cuts of the project's ten reference programs and of one first-cut program, and one small first cut in tooling board, all on one Makera Z1. Nothing has been run on any other machine.
- **The accuracy of a cut part is not yet shown.** The first cut ran with no unexpected event and matched the simulation, but the measured cone diameters were 0.14 to 0.49 mm larger than the model, and the part sat about 1.4 mm toward the tailstock because of how the part's zero along X was set. Further cut testing is planned.
- **Rotary precision** was measured once, on one Z1: A returned within 0.01 mm and repeated within 0.01 to 0.02 mm at a 23 mm radius, approached from one side. See [the built-in Makera Z1 profile](reference/machine-profiles.md#rotary-positioning-on-the-makera-z1).
- **Collisions with the machine are not checked.** The program does not model the chuck, the tailstock, the spindle, the tool holder or the machine body.
- **The programs are not signed.** macOS refuses to open the program the first time and Windows SmartScreen may warn. [Install and first run](start/install.md) and the README in each download give the steps.
- **The macOS build has not been tried by hand**, and it is for Apple silicon only. The Windows build has had less hands-on use than the Linux build.
- **The end of a pointed part.** Past the part's end, finishing runs on at the height of its last point, so a core of stock stays past a pointed or domed end unless **Face free ends** or Facing takes it down.
- **Partial paths.** With [adaptive spacing](glossary.md#adaptive-spacing) on, a finishing operation can stop short and ask you to accept a partial path. A partial path is not complete.
- **No screen reader support**, and the project format may change in a later release.

**Makera Z1 values.** The built-in Makera Z1 profile was measured on the project owner's Z1 (firmware 1.1.2 and 1.1.3, the supplied chuck), and every value in it is marked confirmed: the rotary axis, A direction and A range, the axis height, the rotary reference point, the travel, the feeds and rapids (A at most 1800 degrees per minute), the lowest spindle speed, the spindle to axis distance, the tool length measurement, the minimum block time and the 45 mm clearance values. Not confirmed, or only in part:

- the highest spindle speed, 13000 rpm (stated, but no run was recorded);
- the block window of 32 blocks (from the firmware, not measured);
- the clearance values, which hold only for the supplied chuck with its jaws reversed and fully open;
- the highest tool number, 1000, which is the program's choice, not the controller's;
- how straight the A axis runs along X, which the profile does not hold: on the owner's Z1 it was slightly off;
- other firmware versions, other Z1 machines and other chucks.

A value can differ on your machine, confirmed or not: see [Confirm machine profile values](how-to/confirm-machine-values.md). On the owner's Z1, setting G54 with `G10 L2` and a Z word cleared the tool length offset; check the offsets with `$#` after you set them.

### Known issues

The open bugs at this release, most serious first in each group. The number is the issue on the [issue tracker](https://github.com/OpenChuckCAM/OpenChuckCAM/issues?q=is%3Aissue+is%3Aopen+label%3Abug).

**Programs, and settings you could rely on wrongly**

- Swarf: the lead, ramp or helix at the start and end of a pass is checked only against the cut faces, so it can run into a neighbouring wall or floor with no warning. Check the ends of every swarf pass in the simulation (#572).
- Simultaneous finishing: **Largest lean on hidden surfaces** has no effect. Read the lean in the operation's note instead (#619).
- A copied machine profile keeps the "Confirmed" marks of the values you did not edit. Treat every value in a copy as unconfirmed (#603).
- Your own post definitions: coolant on never writes `M9` (#598); `unwind = "rotate"` writes no move, which affects the built-in LinuxCNC post but not the Makera Z1 post (#599); the safety-notice check accepts the notice anywhere in the program, so write it as comment lines at the start (#574).
- A hand-edited tool profile accepts an impossibly large radius (#628).
- Rest machining and helical finishing can warn "the contact feed is 0.0 mm/min" at the free end of the part; check the feeds near the tip (#510).
- Axial and simultaneous finishing: the simulation can report rapids that remove material, some as 0.000 mm³. Look at each one before you acknowledge it (#801).

**Operations that refuse or are slow**

- Facing refuses every STEP part ("no established projection bound"). Import the part as STL for Facing, or face the stock to length before it goes in the chuck (#800).
- Face flow refuses a closed shape whose edges do not join, such as a cylinder. Split the face along its seam in your CAD program (#791).
- End facing on a round bar can be refused at posting with larger slices; use a **Stepdown (band along the tool)** of about 1 to 1.5 mm (#650). Facing with the **Box** shape refuses on the baluster example (#500).
- A helical finish with a set in **Avoid** set to **Cut short** gives about four times the moves (#608); pencil clean-up with a flat end can leave most of a sharp inside corner uncut, or, with **Exact STEP surface contact** on, cut it in many short passes; check the corner in the simulation (#632); the last move of a lead-in is fed slightly slower than it needs (#685).

**Setups, machines and workholding**

- A setup can be refused with "the start of the stock is at machine X ..., outside the travel" while the stock looks inside the machine. On the Makera Z1, model X 0 is the headstock face, so stock starting at a negative X lies behind it: set **Offset along X** and **Zero at X** so the stock starts past your chuck (#820).
- The machine section says a copy differs in "axis height" when it differs in the rotary reference point (#824).
- The held-region warning understates the uncut zone by 0.5 mm (#774); the head zone line is out of date until you press **Apply** (#767); `openchuck-cli setup --default-tabs` refuses when only one end can have a tab, so add tabs in the window (#676).

**Keyboard and accessibility**

- A window that opens does not take keyboard focus: click in it first (#580). <kbd>Tab</kbd> can move to a field out of view, and nothing scrolls it in (#577). Most tooltips open only on mouse hover (#578). There is no screen reader support (#579).
- Smaller keyboard and layout problems: tab order, missing shortcuts in **Help > Keyboard shortcuts**, actions in no menu, <kbd>Esc</kbd>, and labels cut short at 1366 x 768 (#581); a wrapped choice row in **Avoid** (#609).

**Wording and display**

- After a refused **Generate**, Results still shows the previous run under the new error (#766). Hiding a path asks you to generate again, which you need not do (#607).
- Rest machining: a saving shown as a negative time (#612), zero-length areas and raw words in **Pass order** (#613). Face flow and swarf: the hint says an empty **Cut on** cuts the whole part, but both refuse (#614); face flow shows two pass counts that seem to disagree (#631). "1 faces" and similar wording (#615).
- The **Geometry** section is crowded with several selection sets (#759). A deleted project's recovery warning cannot be dismissed (#681). The default tools file starts with an internal reference number (#775).
- `openchuck-cli` does not print the reminder to verify the G-code when it refuses for a missing acknowledgement; nothing is written (#645). On macOS, the OpenCASCADE notice does not say that replacing the STEP reader inside the app needs `codesign --force --deep --sign - OpenChuckCAM.app` (#589).

Other open bugs concern only the project's own tests, build and documentation; they are on the [issue tracker](https://github.com/OpenChuckCAM/OpenChuckCAM/issues?q=is%3Aissue+is%3Aopen+label%3Abug).

Generated G-code must be verified by you before it is run on a machine. OpenChuckCAM does not guarantee that a program is safe or correct for any machine, tool or setup.

## Development version

The *latest* version of this guide describes the nightly build, the [development build](start/install.md#download-a-development-build) made each night from the main branch. Its limits are on [Known limitations](start/limitations.md). The changes below are those since the last release; the page of the [`nightly` pre-release](https://github.com/OpenChuckCAM/OpenChuckCAM/releases/tag/nightly) lists the same, and when a release is made they move into its own section.

- **Undo history.** **Edit > Undo history** (<kbd>Ctrl</kbd>+<kbd>H</kbd>, or the ☰ button at the right of the title bar) lists every change since the project was opened, newest first, each with what it changed and how long ago, marks the changes you undid and where you last saved, and takes the project back or forward to the change you choose (#705). The title bar shows the project's name, whether it is saved, and the Undo and Redo buttons again: they had been pushed off the right of the window. See [Install and first run](start/install.md#the-first-time-it-opens).
- **Keyboard.** A window that opens takes the keyboard and the first <kbd>Tab</kbd> goes into it, <kbd>Tab</kbd> follows reading order and scrolls the control it reaches into view, <kbd>Esc</kbd> closes a tool window, <kbd>1</kbd>, <kbd>3</kbd>, <kbd>7</kbd> and <kbd>0</kbd> turn the view, tooltips open on keyboard focus (including why a step is not available yet), letter keys work while a button has focus, **Help > Keyboard shortcuts** lists every key, **Project > Add setup** and **Edit > Delete** for a selection set are in the menus, and **Add** in **New operation** is the filled button at the bottom right (#577, #578, #580, #581). See [Keyboard and shortcuts](reference/keyboard.md).
- **What has changed.** The page of the `nightly` pre-release now lists what has changed since the last release, as this section does. See [Install and first run](start/install.md#download-a-development-build).
- **Tool sizes.** A tool larger than any real cutter (a diameter above 2000 mm, a length above 1000 mm, or a custom profile point more than 1000 mm from the tip) is refused with a message naming the size, instead of being accepted.
- **Post templates.** A `namespace` key in a post template must be a name, and a template that stores a namespace inside itself no longer leaks memory.
- **Names in LinuxCNC comments.** A project, setup, operation or tool name that would start a comment LinuxCNC acts on, such as `MSG,` or `LOGOPEN,`, is written with `_` in front, so it stays an ordinary comment, and generating warns you which name was changed and where it came from. A post of your own can list its controller's words with the new option `active_comment_prefixes` ([Writing a post-processor](reference/writing-a-post.md)).
- **Swarf entries.** A lead or helix entry that would cut a face next to the selected faces deeper than the pass itself does, such as a lead-out running on into the wall at the end of a blade, is now refused, naming the face; the face's note gives the tool's reach past the end of the rulings after any lift.
- **Pencil clean-up.** With a flat end it now cleans a sharp inside corner of a STEP part in one pass, with **Exact STEP surface contact** off or on: it judges the material left from the tool as it rests, and rests just clear of the wall beside it. A new setting, **Shortest cut** (by default the tool's diameter), leaves stretches shorter than it, and single points, uncut, and the notes say where (#632).
- **Exact STEP surface contact** has moved to the **Advanced** section of every operation that has it, and its tooltip now says how much closer to the surface and how much slower it was on test parts.
- **Facing beside a grip.** Box facing, and Flat or Round facing with the Part or Part plus tabs extent or a margin, no longer refuse when the requested extent reaches the held region. They face what the tool can reach, the whole tool 0.5 mm clear of the grip, and the operation's notes say where along X the face stops, why, and how much of the extent is left unfaced (#500). See [Facing](reference/operations.md#facing).
- **Face flow on round faces.** Face flow finishing cuts a face that closes round on itself, such as the whole side of a cylinder, in rings from one end to the other, instead of refusing it (#791).
- **Face flow with fewer lifts.** Face flow finishing has four new settings (#854). **Stay down between passes** goes from one pass to the next along the face instead of lifting, where the way is clear. **Pattern** can follow a face's own outline inwards, for faces with fingers, notches or holes such as the outside of a blisk between its blades; **Automatic** chooses. Rings round a face that closes on itself never reverse and are joined by **Steps between rings** (Sharp or Blended), as on the other ring strategies. **Shortest cut** leaves out cuts shorter than it and marks them on the part as not cut; it can be a length or the tool's diameter, as can edge following's **Shortest chain**. A new operation starts with them on (Shortest cut at the tool's diameter); an operation saved before keeps its program.
- **Face flow notes.** The two pass counts in the Results now agree, the face's line saying how many of its passes cut (#631). The Geometry line for face flow and swarf says they are not generated with no face set in **Cut on** (#614). A set of one face or edge reads "1 face" or "1 edge", and the STEP import note names face flow finishing among the strategies that use the exact faces (#615).

Generated G-code must be verified by you before it is run on a machine. OpenChuckCAM does not guarantee that a program is safe or correct for any machine, tool or setup.
