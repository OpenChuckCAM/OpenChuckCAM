# Known limitations

OpenChuckCAM is in early development. Read this page before you rely on it for a real job. It describes the development version; each release's copy of this guide describes that release, and its [release notes](../release-notes.md) list what changed.

## You check every program

The program does not know your machine, your fixture or your tool. Before you run a program:

- check the origin, the height of the rotary axis, the tool lengths, the direction of A and the unwinding at the end of the program on your controller;
- run an [air cut](../glossary.md#air-cut) first, with no tool and no material.

[Checking G-code before you run it](../safety/checking-gcode.md) says how. The Makera Z1 [air cut](../safety/air-cut.md) and [safe-practice checklist](../safety/checklist.md) are the project's own starting points.

## Very little has run on a real machine

The runs the project has recorded are attended air cuts of the project's ten reference programs and of one first-cut program on one Makera Z1, and one small first cut in tooling board, by the project owner. The maintainer has run other jobs on the same Makera Z1, but their details are not recorded here. Nothing has been run on any other machine.

The values in the built-in Makera Z1 profile were confirmed on one machine, the project owner's, with the supplied chuck. They can differ on yours, so measure them on your own machine. A value you change in a profile of your own is marked unconfirmed, and the status bar says so, for example "Clearance unconfirmed". See [Confirm machine profile values](../how-to/confirm-machine-values.md).

How precisely A turns was measured once, on that machine: see [Rotary positioning on the Makera Z1](../reference/machine-profiles.md#rotary-positioning-on-the-makera-z1).

## What the program covers

Machines
: 4-axis rotary work only: the work is held in a chuck and turned by one [rotary axis](../glossary.md#rotary-axis), called A, which turns about X. There is no 3-axis flat-bed milling, no 5-axis work and no second rotary axis.

Controllers
: Two [posts](../glossary.md#post-processor): the Makera Z1, and LinuxCNC (the one the project uses to check its output). Other controllers need a post you write and check yourself. The output is in millimetres only, and A is continuous: it does not wrap at 0 to 360 degrees.

Collisions
: The program does not model the chuck, a tailstock, the spindle, the tool holder or the machine body, and does not check them against the part. The chuck is represented only by a [held region](../glossary.md#held-region), a length at the end of the stock that is never cut; no part of the tool comes within 0.5 mm of it, so Facing leaves the stock beside it unfaced and its notes say how much. The tool's flute length is checked against the stock it cuts: a roughing layer deeper than the flutes is refused, and stock deeper than the flutes elsewhere gives a warning. That warning, and the other checks that may find a collision (such as a tab shorter than the tool is wide), must be acknowledged before the program is written; they cover the part, the stock, the tabs and the held regions, not the holder or the machine. Check clearances yourself.

Models
: STL (ASCII and binary) and STEP. An STL file has no exact surface, so accuracy is measured against its triangles. A STEP file keeps its exact faces and edges for selection, drilling and swarf, but most toolpaths on a STEP part are still computed from a mesh made from it. On a STEP part, Facing reads the part's bounds from its exact faces; on a free-form (B-spline) face it can stop a little short and leave slightly more stock, and the operation's notes say how much. STEP files from CAD programs other than OpenCASCADE have had little testing. A changed model cannot be re-imported into a project, and drawings (DXF or SVG curves) cannot be imported. The import dialog can copy the model into the project folder; a project that only links to a file breaks if the file moves.

Strategies
: Rotary roughing; facing; helical, axial, indexed and simultaneous 4-axis finishing; rest machining; pencil clean-up; edge and curve following; swarf; radial drilling. Swarf and radial drilling need a STEP model. Reaching tight inside corners is limited: a round-ended tool cannot cut a corner sharper than its own radius, so check the corners in the simulation with **Deviation colours**. Past the part's end, finishing runs on at the height of its last point, so a core of stock stays past a domed end, or past a pointed end with **Finish to a point** off, unless **Face free ends** or Facing takes it down (see [Extend past the stock](../reference/operations.md#settings-some-strategies-share)). With **Finish to a point** on, helical, axial and indexed finishing cut a pointed end down to the rotary axis. Simultaneous 4-axis finishing can refuse a part it cannot follow; it says so instead of cutting it badly. Pencil clean-up with a flat end mill in a sharp inside corner beside a wall can find the material left in the corner at only a few places and cut a small part of the corner, sometimes as a plunge and lift at one place, for example after a tapered ball has finished the part; **Exact STEP surface contact** changes how much it cuts. Check such corners in the simulation. There are no canned drilling cycles in the output.

Partial paths
: New finishing operations have [adaptive spacing](../glossary.md#adaptive-spacing) on. Where it cannot confirm the material left between passes before it reaches its limits, it keeps the path made so far, warns you, and asks you to accept the partial path before it writes any G-code. A partial path is not complete. On the tutorial's part with the default settings this happens, so the tutorial turns adaptive spacing off.

Project files
: The project format may still change in a later release.

## Time and size

Generating toolpaths and simulating are not instant. On the tutorial's small part, generating the two operations and preparing the simulation each took a few minutes on a computer with no graphics card. Fine [stepovers](../glossary.md#stepover) on large parts take much longer, and the simulation's cell size trades accuracy against memory and time. The programmed time the program shows is the sum of the moves at their programmed feeds, not an estimate of the machine's time.

## Projects and older versions

A project saved by this version can hold settings an older version does not know, such as **Steps between rings** on face flow finishing or **Tool diameter** for **Shortest cut** or **Shortest chain**. An older version still opens the project, but refuses to generate an operation that uses one of them, naming the setting. Open the project with this version or a newer one.

## Systems

Development builds exist for Linux, Windows and macOS (Apple silicon). Most hands-on checking has been done on Linux. The Windows build has had less, and the macOS build has not been tested by hand: it is built and its automatic tests run, but nobody has worked through the program on a Mac.

The programs are not signed, so Windows and macOS warn or refuse the first time you start one. [Install and first run](install.md#download-a-development-build) gives the steps for each system.

## Accessibility

There is no screen reader support: the program does not expose its controls to a screen reader. Most of the program can be used from the keyboard ([Keyboard and shortcuts](../reference/keyboard.md)), but a run through every step with the keyboard alone has not been tested. A disabled button cannot get keyboard focus, so its reason shows only when you hold the mouse over it; the main button's reason is also in the bottom bar.
