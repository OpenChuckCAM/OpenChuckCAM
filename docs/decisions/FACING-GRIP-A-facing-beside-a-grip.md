# FACING-GRIP-A: Facing beside a held region faces what the tool reaches and says what it leaves (OP-24, SET-09, NFR-07, NFR-12; issue #500; lane FACING-GRIP)

- **Status:** Proposed by lane FACING-GRIP on 23 Nov 2026, from the project owner's decision on issue #500 the same day ("Facing should cut up to the held start with a stated limit instead of refusing"). The owner answered the lane's four questions the same day, relayed by the coordinator: Q1, edit OP-24 in `docs/requirements.md` with the wording below (an owner-agreed edit; no other requirement or architecture edit); Q2, always, with no setting; Q3, no simulation hatch; Q4, include restricted legacy Flat and Round in this change. The owner also directed that the stated limit is information, not a warning. The owner has not yet accepted the record itself (OI-9995).
- **Date:** 23 Nov 2026.
- **Requirements:** OP-24 (facing), SET-09 (held regions), NFR-07 (a clear message, naming what to change), NFR-12 (generated G-code is the user's to verify), NFR-05, GC-07, NFR-06.
- **Amends:** [decision 0252](0252-facing-ends-sides-and-extents.md) (accepted), for held regions only: section 8 ("Any forbidden removal or unfinished required region beyond tolerance refuses and publishes no usable partial operation"), section 11 (the OP-24 text "refuse without a usable partial path") and the limit in section 13 ("whole-stock Box coverage intersecting held geometry refuses (never cropped silently)"). Tabs, protected geometry and every other unfinished region are unchanged: they still refuse. It supersedes the held-region wording of [decision 0667](0667-facing-completion-at-slab-edges-and-beside-a-grip.md) section 2, which reworded that refusal.
- **Open items:** adds OI-9995 (the owner's acceptance of this record).

Numbering: provisional. The coordinator assigns the record number and the OI number at release, above `main`'s highest at that time, in merge order. No project schema number: nothing stored changes. The library API change takes the next workspace minor version in merge order (0.60.0 at the time of writing; decision 0048).

## Context

On the owner's Baluster set-up (box stock 72.4 x 35 x 35 mm, model scaled 1.15, the start held for 8 mm from X 16.990 to 24.990, a 3.175 mm flat tool), Box facing of +Z was refused for both the Whole stock and the Part extents. The part starts at X 17.990, inside the grip. How the refusal came about, on `main` `445286b`:

1. The application widens each held interval by the tool's largest radius and `HELD_CLEARANCE` (0.5 mm) into `StrategyInput::keep_out`: the tool's centre stays out of X 14.815 to 27.078 (decision 0161).
2. `plan_box_sides` laid its rows across the whole requested extent, the grip included.
3. The shared `keep_clear` dropped every row whose centre was inside `keep_out`. The first row it kept was the next one on the stepover's grid, at about X 27.96.
4. The Box completion check (`flat_raster_gap`) found the strip from the extent's start to X 26.37 unproved and refused. Decision 0667 only reworded that refusal.

Restricted legacy Flat and Round (a Part or Part plus tabs extent, or a margin) were refused the same way by their completion checks. Legacy Flat and Round over the whole stock with no margin already did what the owner asks: `keep_clear` cut them short, warned, and nothing checked completion. End facing is not affected: it faces the end away from the grip.

## Decision

Beside a held region, Facing faces what the tool reaches with its clearance from the grip, leaves the rest of the requested coverage unfaced, and says where along X it stops, why and how much is left, in an information note. Always: there is no setting.

![Box facing on +Z beside the Baluster's held start](FACING-GRIP-A-facing-beside-a-grip.svg)

1. **The limit is the host's.** The tool's centre keeps out of each held interval widened by the tool's largest radius (its cutting profile or its shank) and `HELD_CLEARANCE`, exactly as the host's `keep_out` (`openchuck-path` `facing::held::keep_out`, the same measure as the application's `tool_reach`). Generation refuses if the host's `keep_out` is not that span, because Facing could not then state where it stops.
2. **What each mode faces.** The requested coverage along X is cut down at each held end to where the mode's completion check proves the face finished.
   - **Box:** the rows' centres run from the limit, so the first row sits on it and nothing is lost to the stepover's rounding. The face is finished to the tool's edge: the limit less the tool's radius, 0.5 mm from the grip when the shank is no wider than the cut.
   - **Round:** its rings already start on a whole ring at the limit (decision 0550). Its check counts the tool's radius less its 0.001 mm interpolation reserve, so the face is finished from the limit less that.
   - **Legacy Flat:** its passes run along X from the limit. Their round ends are not counted by its check. Its passes are also dropped onto the composite target, the part and the held region with the stock to leave round them, which can lift their ends off the face beside the grip. So where Flat is finished is read from the actual cut: the furthest from the grip at which every line along X is at the face. The note and the overlay (`describe_path`) and the completion check read the same cut. A cut that stops more than the tool's diameter plus the stock to leave beyond the limit is not explained by the grip; it is left to the check, which refuses it.
3. **The completion proof covers the faced region.** Each check (`flat_raster_gap` for Box and Flat, the Round slab check) runs unchanged on the faced region instead of the requested one. Box's removal-domain check uses the faced rectangle too, which is stricter. A gap anywhere in the faced region still refuses. The Box refusal no longer names the grip, since a strip beside the grip is no longer a gap.
4. **What is left is stated, as information.** For each held end, one note (topic Held region) names the faces it applies to, the held interval, the 0.5 mm the whole tool keeps from it, where the face is finished, and the unfaced X range and its length, with what to change. On the Baluster, Box Whole stock: "facing at A 0.000° stops short of the held start (X 16.990 to 24.990): no part of the tool comes within 0.500 mm of it, so the face is finished from X 25.490, and X 16.990 to 25.490 (8.500 mm along X) of the requested extent is left unfaced. To face it, shorten the held length or face it in a setup that holds the other end." The completion note says the faces were covered up to the held region and points at that note. The Facing overlay draws where each face stops and marks it with one short label per held end, "Not faced: held start" (or "Not faced: held far end"): the owner's direction of 23 Nov 2026 (relayed) is that the view's labels stay short and the numbers go in the notes. There is no simulation hatch (the owner's answer Q3): the unfaced stock lies mostly in the grip, not on the part's surface, and the simulation's remaining-stock values still show it.
5. **Nothing left to face refuses.** When the limit leaves no positive span, Facing refuses, naming the grip and the clearance, before anything is generated.
6. **Unchanged:** tabs (still protected and refused, never cropped); End facing; every clearance, protection, shank, stepdown, engagement and footprint check; Flat and Round over the whole stock with no margin; any Facing whose coverage stays clear of the grip by the tool's reach and clearance (its plan is the same, so its G-code is byte-identical).

## OP-24, the owner's agreed edit

`docs/requirements.md` OP-24, in the sentence decision 0252 section 11 added: "Requested faces that cannot be reached completely with supported tool geometry, clearance and machine limits refuse without a usable partial path, except beside a held region: there Facing faces what the tool reaches with its clearance from the grip, leaves the rest of the requested coverage unfaced, and its notes say where along X it stops, why and how much is left; where nothing is left to face, it refuses." The requirement's closing reference names this record. No other requirement and no architecture text changes.

## Alternatives considered

- **Keep refusing** (decision 0667): the owner decided against it on #500.
- **A setting "At a held region: stop short and say so / refuse"** (the lane's Q2 alternative): not chosen. Every other strategy, and whole-stock legacy Facing, already stop short beside a grip (decision 0536 step 2), and a setting would add a parameter and its persistence for no case the owner named.
- **A warning:** the lane first proposed a warning, as `keep_clear`'s note for other strategies is. The owner chose information: the limit is the program doing what was asked within the grip's rule, not a problem with the path.
- **An unreached-area hatch in the simulation** (`Unreached` with `NearHeldRegion`): not chosen (Q3). It marks part-surface points, and most of the unfaced stock is in the grip.
- **Legacy Flat at the keep-out limit:** stating Flat's limit as the keep-out alone would claim a face its check cannot prove where the composite target lifts the pass ends. Reading the cut keeps the statement exact.

## Safety

The change only ever reduces what is cut. Three independent layers keep the tool off the grip. Facing's own limit (item 1). The shared `keep_clear` and `check_clear` with the host's `keep_out`, which still refuse any fed move into it. Facing's protected-clearance check against the held solids. No new motion is added. An extent that would need the tool inside the held region is never cut. Generated G-code remains the user's to verify before it is run on a machine (NFR-12).

## Consequences

- `openchuck-path`: new `facing/held.rs` (`Unfaced`, the crop, the host check, the notes, the Flat measurement); `facing/description.rs` (`FaceDescription::unfaced` and `faced()`, `FacingGeometry::Round::unfaced`, `FacingGeometry::unfaced()`, `describe_path`); `facing/plan.rs` (Box rows from the limit); `facing/validation.rs` and `validation/legacy.rs` (the checks on the faced region; `ROUND_RESERVE`); `facing.rs` (the host check, the notes, the Extent tooltip).
- **Breaking library change:** `FaceDescription` and `FacingGeometry::Round` gain public fields. Workspace minor bump with changelog entries (decision 0048).
- `openchuck-app`: the completion note and the Facing overlay.
- **Documents:** OP-24 (above); the verification plan's OP-24 row; the user guide's Facing reference, known limitations and release notes; the crate changelogs; OI-9995.
- **Determinism (GC-07, NFR-06):** the limit is arithmetic on the setup and, for Flat, on the generated path, so it is the same at any thread count. Tested at 1 and 4 threads on the Baluster.
- **Found, not fixed here:** restricted Round facing (the Part extent) on the Baluster is refused, with or without the grip and on both its box and round-bar stock, by Facing's rapid-clearance check ("rapid move ... has no continuous complete-tool separation"). It predates this change and is ticketed separately (#880). Round's limit is therefore tested on a block in `openchuck-path`, not on the Baluster.

## Verification

See the pull request's Verification section, copied here when the lane's checks complete.
