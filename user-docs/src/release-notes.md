# Release notes

What changed in each release of OpenChuckCAM, what it cannot do yet, and which Makera Z1 machine values were confirmed for it. Each release adds a section headed with its version number, newest first, before it is released. The release workflow refuses to publish a release whose section is missing.

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

### Changes since the recent development builds

If you used a development build, these change what the program does or what it writes. Operations saved with an earlier build open in this one, but some give a different program, as listed here. Generate them again and check the new program before you run it.

Ramp entry
: A ramp now follows the pass's own path down to its first cut point: back and forth over the start of a line, round a ring before its first point, or round the part at the start of a spiral (a swarf pass is the exception, below). The program of every operation with **Entry** set to `ramp` changes. Ramps beside a held region, on a part that rises along its length and on passes that turn A now generate where they were refused before.

: A swarf pass, which turns A along a blade, is not ramped round the part like a spiral; its ramp goes back and forth along the blade, following the pass. A swarf operation with **Entry** set to `ramp` can give a different program.

: **Back-and-forth ramp** no longer has an effect. It is shown only on an operation saved with it on.

: Each leg of a back-and-forth ramp is now at least as long as the tool radius wherever the start of the pass is that long. A ramp that needs little descent is then shallower than the ramp angle, never steeper. Ramps whose legs were shorter than the tool radius are now longer.

: A pass whose start is shorter than the tool radius is no longer refused: the ramp goes back and forth over it, down to a length of ten times the [tolerance](glossary.md#tolerance) (0.1 mm at the default). Over such a short length the tool's centre may go down into uncut stock, so a tool that cannot cut at its centre is refused there.

New entry settings
: Under **Entry**, used only with ramp entry: **Allow single-point plunge** (off by default) plunges a pass that is a single point instead of refusing the operation. **Steeper ramp near the axis** (off by default) uses a steeper angle, up to **Maximum ramp angle** (15° by default), where a ramp round a small ring would need more than 10 turns.

: **Plunge to remove face nubs** (off by default), with **Face nub diameter**: in roughing, where the part comes to a point near the rotary axis with stock beyond it, the tool plunges at the axis while the part turns to remove the narrow nub of stock left there, instead of ramping round ever smaller rings. The tool must be able to plunge (centre cutting), and it cuts with its full width.

Lead entry
: On rings and spirals the lead arcs now come down onto the cut and rise from it while the part turns, at the ring's position along the part. They no longer run along the part, so operations that were refused next to a held region or on a part that rises along its length now generate. Lead entry on other passes is unchanged.

: A lead whose descent goes down through stock, rather than through air, is now refused for a tool that cannot cut at its centre, as a plunge entry already was. A pass with no cut to lead into (a single point, or a ring on the rotary axis) is entered by a straight plunge, and a note names it.

Plunges into stock
: An operation with ramp or lead entry is refused, naming the tool and the first pass, when the tool cannot cut at its centre and a pass is entered by a straight plunge into the stock.

Steps between rings
: A new setting, **Steps between rings**, beside **Passes** for helical finishing, rest machining, continuous rotary roughing, round Facing and the spiral pattern of simultaneous finishing. **Blended**, the default for new operations, moves to the next ring while A turns on by **Blend over** (45° by default), instead of stepping along the part with A held. Operations saved before keep **Sharp** steps and their program. Blended steps make a program longer: on the baluster example in Rings, about 18 % for roughing and 10 % for finishing, as programmed time.

Rings beside a held region
: Helical finishing and rest machining in **Rings** beside a held region now start on a whole ring at the edge of the held region, as roughing does, instead of with a short cut along the part with A held. Ring operations with a held region at an end of the part change, saved ones too.

Extend past the stock
: Helical finishing with **Extend past the stock** now carries its spiral or rings on past the part in their own motion, so the stock past the part's end is cut round the part, down to the height of the pass's last point. It is not cut down to the rotary axis: past a pointed or domed end a core of stock stays, which **Face free ends** or a Facing operation takes down (see [Extend past the stock](reference/operations.md#settings-some-strategies-share)). Saved operations with it on change, and the run-out is longer; the operation's note gives the turns and the length added.

Roughing descents
: Rotary roughing and Facing (flat and round) now go down into the stock inside a pass no faster than the plunge feed.

End facing
: Facing with the **Unheld end** shape: **Stepover (slice thickness along X)** now sets the slice along X and **Stepdown (band along the tool)** the band along the tool, as a tool's cutting data defines them. Saved operations are converted when the project is opened, so they cut as before.

: Where there is room beside the stock, End facing now enters each band from the air instead of moving the tool sideways into the end face; where there is not, it enters at the plunge feed. It checks how deep each band cuts along the tool against the stepdown, and adds passes from the top of the stock where the stock rises above the area being faced.

Swarf
: A new swarf setting, **Keep off the end faces** (on by default), stops a pass short of a face that meets an end of the faces it cuts, and the note says how much is left uncut. Saved swarf operations may give a different program.

Messages
: Refusals and other messages now take one form: what was checked, then what it was checked on, then what was found, for example "Stepover (operation 'Rough'): must be above 0 mm.". **Show setting** in Messages opens the setting a message names. Messages no longer quote requirement or decision numbers, and a count of one reads in the singular ("1 layer").

: A possible collision, such as a tab shorter than the tool is wide, a rapid move that the simulation found removing material, or a safe cylinder that does not enclose the stock, is now a warning that must be acknowledged before the files are written.

: The overall status reads "Nothing to report" when there is nothing to report. "Saved", "Opened" and "Added" are entries in the Jobs tab, not messages.

Workholding
: **Apply** in the workholding section now stores the workholding when an automatic tab has no room or overlaps a tab placed by hand, and keeps that finding as a warning on the tab's row. Only a tab outside the end zones is still refused.

Makera Z1 A limit
: The built-in Makera Z1 profile now limits A to 1800 degrees per minute (30 degrees per second), not 3600. The Z1's controller holds A to that rate: on firmware 1.1.3, `M203.1` in the console prints `A: 30`. Programs that turn A while cutting take longer and their F words change, and the time estimates on the setup sheet and in the simulation use the controller's limit; positions and rapids are unchanged. A project made with an earlier build keeps its own copy of the profile and warns that it has the old A limit: choose the Makera Z1 again in the setup's machine section to update it, then generate again. If `M203.1` on your Z1 prints `A: 60`, its limit is 3600 and you can set that in your own copy of the profile.

Makera Z1 measured values
: The built-in Makera Z1 profile takes the project owner's measurements on firmware 1.1.3. The travel is the controller's soft limits: X -207 to -1, Y -160 to -1 and Z -102 to -1 (it was -200 to 0, -200 to 0 and -100 to 0), so a program that needs Y below -160 is now flagged. The rotary reference point is the centre of the headstock face next to the chuck, at machine X -198.39 and Y -123.39, not -180.4 and -108.8, so model X 0 is now that face: in a new project, start the stock about the chuck's length along X, and for the same work zero the expected G54 X shown is 17.99 mm lower. The sample and example projects have been moved along X to keep their place on the machine; their programs are the same apart from the safe Z. The spindle to axis distance is 72.0 mm, measured from the collet face, not 73.1. A minimum clearance radius of 45 mm clears the supplied chuck with its jaws reversed and fully open; the default safe cylinder radius and safe Z are 45 mm, not 40, and the recommended minimum safe Z is 45 mm, not 30. The spindle speed range is 4000 to 13000 rpm, so an operation below 4000 rpm cannot be posted for the Z1. A tool with flutes longer than 22 mm now warns that it may be too long to clear the workholding within the Z travel (the program assumes a tool sticks out 5 mm more than its flutes). A project made with an earlier build keeps its own copy of the profile and its old values: choose the Makera Z1 again in the setup's machine section to take the new ones, then generate again. For another chuck or jaw setup, set the setup's own minimum clearance.

Icon
: OpenChuckCAM has its own icon in the window and the taskbar, and on the Windows program file.

### Bug fixes

- The default tabs and **Add default tabs** in the workholding section now follow a change to the stock straight away.
- A workholding **Apply** is stored when its check finishes, even if you have moved on to another item first.
- A ramp is no longer refused as a cut into the finished surface on a pass whose tool edge touches a face of the part, such as a roughing ring beside an end face.
- Helical finishing and rest machining in **Rings**, with no held region over an end, no longer leave out a ring past the part's end.
- A finishing operation no longer warns of a step across the rotary axis near the chuck when that step is only the end of the held bar. The paths and the G-code are unchanged. The warning that nothing within one tool radius of the held region is cut still appears.
- An operation limited to selected faces no longer runs past them at a convex edge.
- End facing on stock that is exactly a whole number of slices longer than needed now writes its G-code.
- The refusal to write files with possible collisions not yet acknowledged no longer shows a doubled full stop.
- When a recovered project's checkpoints cannot be written, Messages always shows the error "Recovery checkpoint: …" with **Save**, instead of sometimes a warning.
- Zooming over a shown selection set on a large model no longer redraws the whole set on each frame.
- A swarf ramp on a part with blades no longer sweeps the tool through the neighbouring blades' tips (found on a blisk).
- A program file that contains an End facing operation, grouped by setup, by project or by tool, is no longer refused when posting with "rotary counter reset is not at a reached declared boundary or does not match the controller" (Makera Z1) or "the G-code ends before the IR does" (LinuxCNC). The program is unchanged.

### Known limitations

[Known limitations](start/limitations.md) lists what the program cannot do yet. The most important for this release:

- **Very little has run on a real machine.** The recorded runs are attended air cuts of the project's ten reference programs and of one first-cut program, and one small first cut in tooling board, on one Makera Z1. Nothing has been run on any other machine.
- **The programs are not signed.** Builds for this release are not signed or notarised. macOS refuses to open the program the first time and Windows SmartScreen may warn. [Install and first run](start/install.md) and the README in each download give the steps.
- **The macOS build has not been tested by hand.** It is built and its automatic tests run, including starting the program, but nobody has worked through it on a Mac: not the first-run dialogs of a downloaded copy, opening it from Finder or the Dock, the clipboard, Command-key shortcuts, the menu bar, full screen or the file dialogs. It is for Apple silicon only. The Windows build has had less hands-on testing than the Linux build.
- **Collisions with the machine are not checked.** The program does not model the chuck, the tailstock, the spindle, the tool holder or the machine body.
- **The end of a pointed part.** Past the part's end, finishing runs on at the height of its last point, so a core of stock stays past a pointed or domed end unless **Face free ends** or Facing takes it down, and a small blunt nub can stay at the point even then.
- **Partial paths.** With [adaptive spacing](glossary.md#adaptive-spacing) on, a finishing operation can stop short and ask you to accept a partial path. A partial path is not complete.
- **No screen reader support.**
- **The project format may change** in a later release.

### Known issues

These are the open bugs known at this release, grouped by what they affect, with the most serious first in each group. The number in brackets is the issue on the [issue tracker](https://github.com/OpenChuckCAM/OpenChuckCAM/issues?q=is%3Aissue+is%3Aopen+label%3Abug), where you can follow it or add what you find.

These two problems are planned to be fixed in the next release, 2026.2.

- Facing does not yet work on STEP parts: it refuses with "no established projection bound". Workaround: import the part as STL for the Facing operation, or face the stock to length before it goes in the chuck and leave Facing out (#800).
- Face flow cannot be applied to a closed shape whose edges do not join, for example a cylinder (#791). Workaround: choose faces that are not closed shapes, or split the face along its seam in your CAD program.

**Programs, and settings you could rely on wrongly.** These can change a program, or let you believe it does something it does not.

- In a swarf operation, the lead, ramp or helix at the start and end of a pass is checked only against the faces the operation cuts, not the walls beside them. A lead-out at the end of a pass can run into a neighbouring wall with no warning, and a ball-end tool's lead on a wall that stands on a floor can cut into the floor. Check the start and end of every swarf pass in the simulation (#572).
- In simultaneous finishing, **Largest lean on hidden surfaces** has no effect: set to 0° it gives the same program as the default 90°, and no warning lists surfaces left uncut. Do not rely on it; read the lean the operation's note reports (#619).
- Copying a machine profile keeps the "Confirmed on the machine" marks of the values you do not edit, so a copy made for another machine shows values as confirmed that nobody has checked on it. Treat every value in a copied profile as unconfirmed and measure it on your machine. In the same window, after you correct a refused value, its red error stays at the bottom until you save (#603).
- A tool profile file edited by hand accepts a radius far beyond any real tool (a number hundreds of digits long) instead of refusing it, and the tool's size then cannot be worked out. Give real sizes in a tool profile (#628).
- In a post definition you write yourself, coolant set to flood or mist writes `M8` or `M7` at each spindle start but never `M9`, so coolant stays on until the end of the program, and then only if your controller turns it off there. The built-in posts have coolant off. If you turn coolant on, add `M9` to your post where coolant must stop (#598).
- In a post definition, `unwind = "rotate"` writes no move: A is not turned back by whole turns between operations, and its value keeps growing through the program. The built-in LinuxCNC post uses this setting. The built-in Makera Z1 post resets the A count instead and is not affected (#599).
- In a post definition you write yourself, the check that the program carries the notice that it must be verified accepts the notice text anywhere in the program, not only as comment lines at its start. Write the notice as whole comment lines at the start, as the built-in posts do (#574).
- Rest machining and helical finishing can give many warnings that "the contact feed is 0.0 mm/min" on passes at the free end of the part, where A turns at almost no radius. It is not yet known whether only the message is wrong. Check the feeds near the tip of the part in the program before you run it (#510).
- On axial and simultaneous finishing, the simulation can report rapid moves that remove material, some of them shown as 0.000 mm³, and the command-line simulation then fails. This was found on an earlier development build and has not been checked again on this release. Look at each reported rapid in the simulation before you acknowledge it (#801).
- End facing on a round bar can be refused when posting with "the actual end sweeps leave an unfinished required region in an axial layer", with larger slices. Workaround: a smaller **Stepdown (band along the tool)**, about 1 to 1.5 mm (#650).
- Facing with the **Box** shape refuses on the baluster example with "an unfinished strip remains inside the requested extent" (#500).
- A helical finish with a selection set in **Avoid** set to **Cut short** gives about four times the moves of the same finish without it, and takes about twice as long to generate (#608).
- Pencil clean-up with a flat-end tool on a STEP part makes many more passes at a corner with **Exact STEP surface contact** on than with it off (31 against 6 in one case). It is not yet known whether this is intended (#632).
- The last move of a lead-in, where the tool meets the surface on a pass that turns A, is given a slower feed than its length needs, by a fraction of a second. It is never faster than it should be (#685).

**Keyboard use and accessibility.**

- A window that opens over the main window (export, import, new operation, tool library, machine library) does not take keyboard focus: the first <kbd>Tab</kbd> goes to the **File** menu behind it. Click in the window first (#580).
- <kbd>Tab</kbd> can move to a field that is scrolled out of view, in the Inspector, on the export window's acknowledgement tick box and in the machine library, and nothing scrolls it into view. Scroll with the mouse, or make the window larger (#577).
- Most tooltips open only when the mouse is over a control, so you do not see them from the keyboard. This includes the reason a step tab gives for being unavailable (#578).
- There is no screen reader support, and many controls you can reach with <kbd>Tab</kbd> have no name (#579).
- Smaller keyboard and layout problems: the <kbd>Tab</kbd> order does not always follow the reading order; **Help > Keyboard shortcuts** leaves out some keys (<kbd>F</kbd>, <kbd>V</kbd>, the playback keys, <kbd>Ctrl</kbd>+<kbd>Enter</kbd>, <kbd>Ctrl</kbd>+<kbd>+</kbd> and <kbd>Ctrl</kbd>+<kbd>-</kbd>, <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Z</kbd>); adding a setup and deleting a selection set are in no menu; <kbd>Esc</kbd> closes only the tool library window; the **Add** button of the new operation window is not highlighted; and at 1366 x 768 some labels are cut short or overlap (#581).
- With a selection set in **Avoid**, the **Cut short**, **Warn only** and **Ignore** choices wrap onto a second line that starts left of the other settings. Every choice still works (#609).

**Tools, machines, workholding and selection.**

- A setup can be refused with a travel message such as "the start of the stock is at machine X -263.160, outside the travel -207.000 to -1.000" while the stock looks well inside the machine in the view. The message does not say why. On the Makera Z1, model X 0 is the face of the headstock, so stock that starts at a negative X, for example round a model imported centred on X 0, lies behind that face. Move the part and the work zero along X with **Offset along X** and **Zero at X** so the stock starts past your chuck (see [Set up the stock](tutorial/04-stock.md)). If the message stays, add your project to the issue (#820).
- In the machine section, a project whose copy of the Makera Z1 differs from the built-in profile only in the rotary reference point says it differs in "axis height". The difference is in the reference point. **Update the copy from the library** takes the built-in values (#824).
- The held-region warning says nothing within one tool radius of the held region is cut. The zone left uncut is wider, the tool radius plus 0.5 mm, and corners next to it can keep up to about 0.9 mm more material (#774).
- In the workholding section, the head zone line is out of date until you press **Apply**: it can show a 5 mm zone when the grip already reaches the part. Press **Apply** to bring it up to date (#767).
- `openchuck-cli setup` with `--default-tabs` is refused when only one end of the part can have a default tab, and writes nothing. Add the tabs in the workholding section of the program, which adds the one there is room for, or add that end's tab by hand (#676).
- An operation's **Geometry** section is crowded and hard to read when it has several selection sets (#759).
- After a project that had recovery data is deleted, Messages keeps warning that the project is missing, and the warning cannot be dismissed (#681).
- The default tools file starts with a comment that carries an internal reference number. It does not change the tools (#775).

**Generating and the Results tab.**

- After **Generate** is refused, Results shows the new error but still lists the previous run's moves and warnings under it. They belong to the earlier run (#766).
- Hiding a path with its eye icon makes Results say "The project has changed since this run. Generate again." Showing or hiding paths changes nothing, and you do not need to generate again (#607).
- The pass order message can give a time such as "about -11.4 s". A minus sign means the moves between passes are that much shorter: it is a saving (#612).
- In rest machining, an area that cannot be reached can be listed with no length ("X 10.000 to 10.000"), and the **Pass order** choices show `by_region` and `after_main` instead of words (#613).
- For face flow finishing and swarf, the **Geometry** section says "With nothing in Cut on, the operation cuts the whole part", but both refuse without a set in **Cut on**. Save the faces as a selection set and put it in **Cut on** (#614).
- Face flow finishing can give two pass counts that seem to disagree, such as "cuts 12 passes on 1 face" and "in 17 passes" (#631).
- The tree and the selection set's settings say "1 faces" and "1 edges", and the message shown when a STEP file opens does not name face flow finishing among the operations that use its exact faces (#615).

**Other.**

- When `openchuck-cli` refuses to write files because a possible collision or a partial path has not been acknowledged, it does not print the reminder that generated G-code must be verified. Nothing is written; the reminder is printed on the run that writes the files (#645).
- On macOS, the OpenCASCADE notice says you may replace `openchuck-step-worker`, but not that replacing it inside `OpenChuckCAM.app` breaks the app's signature. Sign it again with `codesign --force --deep --sign - OpenChuckCAM.app` (#589).

Other open issues concern the project's own tests, build and documentation and do not affect programs. They are listed on the [issue tracker](https://github.com/OpenChuckCAM/OpenChuckCAM/issues?q=is%3Aissue+is%3Aopen+label%3Abug) with the label bug.

### Makera Z1 machine values

The built-in Makera Z1 profile was measured on the project owner's Z1, on firmware 1.1.2 and again on 1.1.3, with the supplied chuck. Every value in it is marked confirmed. These are the values confirmed on that machine:

- the rotary axis along X, the direction of A+ and the continuous A range;
- the axis height, with the probe as the reference tool;
- the rotary reference point, the centre of the headstock face next to the chuck (on firmware 1.1.3; its Y is the G54 Y that the firmware sets in its 4-axis setup);
- the travel, which is the controller's soft limits on firmware 1.1.3 with the rotary module fitted;
- the maximum feeds and the rapid rates, including the A limit of 1800 degrees per minute on firmware 1.1.3;
- the lowest spindle speed, 4000 rpm;
- the spindle to axis distance;
- the tool length measurement on the tool setter;
- the minimum block time of 7 ms;
- the minimum clearance radius, the default safe cylinder and safe Z, and the recommended minimum safe Z, all 45 mm, for the supplied chuck with its jaws reversed and fully open.

Not confirmed, or confirmed only in part:

- **The highest spindle speed**, 13000 rpm, is the owner's statement that it has been run; no run was recorded.
- **The block window** of 32 blocks comes from the firmware source and the owner's statement; it was not measured on the machine.
- **The clearance values** hold only for the supplied chuck with its jaws reversed and fully open. For any other chuck or jaws, measure and set the setup's own minimum clearance.
- **The highest tool number**, 1000, is the project's choice, not a limit of the controller.
- **How straight the A axis runs along X** is not part of the profile: the program assumes the axis is parallel to the X travel. On the owner's Z1 it was not quite parallel (about 0.1 to 0.2 mm sideways and under 0.04 mm up or down over 50 mm), and the chuck's screw mount held the stock about 0.1 mm off centre. For the first cut the owner set G54 Y and Z from the axis measured with the probe at the part, not at the headstock.
- **The size of a finished part.** The one first cut, in tooling board, ran as the simulation showed with nothing cut that should not have been, but its measured diameters do not show whether the part is within 0.1 mm: the work zero along X was about 1.4 mm out, which on a cone reads as a diameter error.
- **Other firmware, other Z1 machines and other chucks** have not been checked. Every value can differ on your machine, confirmed or not, and a value you change in your own copy of the profile is marked unconfirmed until you confirm it.

While setting the work offsets for that first cut, setting G54 with `G10 L2` and a Z word cleared the tool length offset and made the probe the reference tool, and a later offset step reset G54 Y. Check the offsets in the console (`$#`) after you set them, before you run a program.

Measure the values on your own machine before you rely on them: see [Confirm machine profile values](how-to/confirm-machine-values.md).

Generated G-code must be verified by you before it is run on a machine. OpenChuckCAM does not guarantee that a program is safe or correct for any machine, tool or setup.

## Development version

The *latest* version of this guide describes the nightly build, the [development build](start/install.md#download-a-development-build) made each night from the main branch. Its limits are on [Known limitations](start/limitations.md).

Generated G-code must be verified by you before it is run on a machine. OpenChuckCAM does not guarantee that a program is safe or correct for any machine, tool or setup.
