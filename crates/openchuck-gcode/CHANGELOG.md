# Changelog

## 0.80.0

- **Family release (0.80.0; PATH-QUALITY; issue #880):** `openchuck-path` adds `Pass::turn_above` (breaking) and raises Round facing's links that turn A above the setup; no change to this crate.

## 0.75.0

- **Family release (0.75.0; PATH-QUALITY; issue #957):** `openchuck-path` measures a lead or helix entry of an operation limited to a selection against the part's faces outside the selection that it comes near, and refuses one that would cut them; no change to this crate.

## 0.74.0

- **Family release (0.74.0; PATH-QUALITY; issue #834):** `openchuck-path` faces the ends that do not reach round the rotary axis in arcs, with a new Cutting direction setting, and warns where a tab misses an end; no change to this crate.

## 0.73.0

- **Family release (0.73.0; PATH-QUALITY; issues #951 and #959):** `openchuck-path` refines how ramps are measured and fitted, measures a plunge's depth from where the stock begins, and refuses a stretch past a point that would make too many points; no change to this crate.

## 0.72.0

- **Family release (0.72.0; MOPUP-1, PR M4; issue #896):** `openchuck-path` spaces face flow's Follow the outline with a scallop height for the face's largest principal curvature (changed output on faces that curve most between their parameter directions). No change to this crate.

## 0.71.0

- **Family release (0.71.0; PATH-QUALITY, PQ-3; issue #779; decision 0695, Q2, accepted):** `openchuck-path` adds the parameter Finish to a point (helical, axial and indexed finishing; off for saved operations) and changes output only where it is on; no change to this crate.

## 0.70.0

- **Family release (0.70.0; PATH-QUALITY, PQ-2b; issue #786; decision 0695, Q9 part 3, accepted):** `openchuck-path` changes ramp output (a whole ring whose one turn would descend more than the largest depth per traverse is ramped back and forth over itself). No change in this crate.

## 0.69.0

- **Family release (0.69.0; PATH-QUALITY, PQ-4; issue #826, OI-2198; decision 0695, Q6, accepted):** `openchuck-path` changes output only for helical finishing as a spiral with adaptive spacing on (it measures the helix only outside the held regions' bands and the face turns); no API change; no change to this crate.

## 0.68.0

- **Family release (0.68.0; PATH-QUALITY, PQ-2; issue #786; decision 0695, accepted):** `openchuck-path` breaks its API (`Strategy::ramps_round_the_part` is removed, and `params::EntryParameters` gains the public field `ramp_depth_per_traverse`) and changes ramp output; no change to this crate.

## 0.67.0

- **Family release (0.67.0; SEV1-929, #948 follow-up; decision 0692 amendment 2, accepted):** `openchuck-path` changes output (on a controller that counts A in its F word, a fed turn of A with the tool tip fixed is timed for A's counted travel where an end's A is not on three decimals). No change in this crate.

## 0.66.0

- **Family release (0.66.0; SEV1-929 follow-up; issue #943; decision 0692 amendment 1, accepted):** `openchuck-path` changes output (a fed turn of A with the tool tip fixed is timed for the line the post may write for it, and every fed move whose change of X and Y is below the written band, none included, is timed for the lower of the linear and Z limits). No change in this crate.

## 0.65.0

- **Family release (0.65.0; MOPUP-1, PR M3; issues #887 and #895):** `openchuck-path` breaks its API (`swarf::Limits` gains the public field `least_lift_along_axis`) and changes the swarf face note's reach past the rulings for a tapered tool; restricted Facing refuses a host keep-out with a span the grip does not give. No change in this crate.

## 0.64.0

- **Family release (0.64.0; SEV1-929; issue #929; decision 0692, accepted):** `openchuck-path` changes output (a fed move whose small X or Y change can be lost when the post writes A to three decimals is timed for the lower of the linear and Z limits, and the line's written allowance grows by the shift writing A gives its end points, so the F word stays within the limit gate 1 applies). No change in this crate.

## 0.63.0

- **Family release (0.63.0; lane FACEFLOW; issues #791 and #854; decisions 0675 and 0686, accepted):** `openchuck-path`'s `UnreachedReason` gains the variant `TooShort`, a breaking change; no change to this crate.

## 0.62.0

- **Family release (0.62.0; lane FACING-GRIP; issue #500; decision 0685, accepted):** `openchuck-path`'s `FaceDescription` gains the public field `unfaced` and `FacingGeometry::Round` the field `unfaced`, a breaking change; no change to this crate.

## 0.61.0

- **Family release (0.61.0; SWARF-LEADS; issues #572 and #624; decisions 0682 and 0683, accepted):** `openchuck-path` changes output (a swarf lead or helix that would cut a face next to the face set deeper than the pass is refused; swarf's note gives the reach past the rulings after the lift); no public API change and no change to this crate.

## 0.60.0

- **Family release (0.60.0; POST-COMMENTS; issue #851; decision 0681, accepted):** `openchuck-post`'s `PostOptions` gains the public field `active_comment_prefixes` and `PostOutput` the public field `guarded_comments`, a breaking change; no change to this crate.

## 0.59.0

- **Family release (0.59.0; lane Z1-SETUP, PR 2; issue #692; decision 0673):** `openchuck-types`'s `Tool` gains the public field `stick_out`, a breaking change; no change to this crate.

## 0.56.0

- **Breaking (0.56.0, POST-SAFETY; issue #703; OI-933; decision 0656, accepted):** `line_length_check(source, max_chars)` returns a `FindingKind::LineTooLong` finding for each line longer than `max_chars` characters (the line end not counted), naming the line and both lengths (GC-08). A `match` on `FindingKind` adds the variant.

## 0.54.0

- **Family release (0.54.0, or the next free version, as the Merge Manager sequences it; RAMPENTRY; issues #639, #642; decision 0543, accepted):** `openchuck-path`'s `EntryParameters` gains `allow_single_point_plunge` and loses `back_and_forth`, and `StrategyInput::entry_safety` and the `entry_safety` module are removed (decision 0543 D10), a breaking change; no change to this crate.

## 0.52.0

- **Family release (0.52.0, or the next free version, as the Merge Manager sequences it; PATHFEED; issue #641; decision 0547, accepted):** `openchuck-path`'s `Pass` gains the public field `plunge_fed` and `facing::end::EndPassPlan` the public field `passes_above_area`, a breaking change; no change to this crate.

## 0.51.0

- **Family release (0.51.0, or the next free version, as the Merge Manager sequences it; MSGREVIEW step 3; issue #567; decision 0536, accepted):** `openchuck-path` removes `flutes::WARNING_LEAD` and adds the `topic` module, a breaking change; no change to this crate.

## 0.50.0

- **Family release (0.50.0, or the next free version, as the Merge Manager sequences it; MSGREVIEW step 1; issue #567; decision 0536, accepted):** `openchuck-path` replaces `PathNote` and `PathWarning` with one `Notice` type, a breaking change; no change to this crate.

## 0.49.0

- **Family release (0.49.0, SAFETYNOTICE; OI-2000; decision 0533, accepted):** `openchuck-post`'s `PostOptions` gains the public field `safety_notice`, a breaking change; no change to this crate.

## 0.47.0

- **Family release (0.47.0, TABCUT; issue #555; decision 0527, accepted):** `openchuck-path`'s `RegionConstraint` gains the public field `outside`, a breaking change; no change to this crate.

## 0.46.0

- **Family release (0.46.0, STEPALL-CLOSE; issue #351; decision 0526, accepted):** `openchuck-path`'s `Strategy::exact_contact` returns the kind of Exact STEP surface contact a strategy offers, a breaking change; no change to this crate.

## 0.39.0

- **Family release (0.39.0, lane A-MOVES, issue #503; decision 0506, accepted):** gate 2 (`compare`, `compare_with_moves`) accepts IR poses part way along a fed pure-A block at exactly the same written X, Y and Z, in order; each is compared and timed as the part of the block that reaches it, and observers see that part. No API change.

## 0.34.0

- **Breaking family release (0.34.0, lane CLEARANCE, issue #336; decision 0495, accepted):** no change in this crate; all family crates advance together under 0048.

## 0.33.0

- **Family release (0.33.0, SWARF-FLOOR; issue #435; decision 0491, accepted, relayed by the coordinator):** on swarf, Cut short lifts the tool's tip off an avoid or check set before it cuts the pass short, and Tip extension takes a negative value; `openchuck-path`'s `swarf::Pose` and `swarf::Limits` gain fields (an API break) and saved swarf operations with such a set can be made differently. No change in this crate; all family crates advance together under 0048.

## 0.32.0

- **Family release (0.32.0, CRANKEXCESS; issue #439; decision 0410, accepted):** simultaneous finishing's turn at a face beside a section off the rotary axis goes round that section's own centre, which changes `openchuck-path`'s output for such parts, so the family moves together. No change in this crate.

## 0.31.0

- **Family release (0.31.0, ENDFACE stage 3 second pull request, OI-1453; decision 0239, accepted):** adaptive spacing for rotary roughing's two axial forms and flat Facing, on for a new operation, changes their output (decision 0239 item 13, which extends decision 0048 beyond API breaks). No change in this crate; all family crates advance together under 0048.

## 0.30.0

- **Breaking family release (0.30.0, OP20, decision 0368):** `openchuck_path` gains `StrategyInput::shank`, `StrategyOutput::unreached` and `shank_breaks`, and `GeneratedPath::unreached` and `shank_breaks`. No change in this crate; all family crates advance together under 0048.

## 0.29.0

- **Breaking family release (0.29.0, ENDFACE stage 3, decision 0239, accepted):** `openchuck_path::PathWarning` gains `requires_acknowledgement` and `Strategy::passes_in_zones` returns `StrategyOutput`, and adaptive spacing on for a new operation changes its output (decision 0239 item 13). No change in this crate; all family crates advance together under 0048.

## 0.28.0

- **Breaking family release (0.28.0, Z1AXIS, decision 0317):** `openchuck_types::machine::MachineProfile` gains the spindle-height fields and `ProfileField` three variants. No change in this crate; all family crates advance together under 0048. Project schema 22 and machine profile schema 7 in `openchuck-project`.

## 0.27.0

- **Breaking family release (0.27.0, BUG-SWARF4, decision 0272):** `openchuck_path::swarf::FacePath::overcut_warning` takes a checked `Clearing`. No change in this crate; all family crates advance together under 0048. No project-schema migration.

## 0.26.0

- Independently validate declared pass-boundary resets, including rounded-away retracts, while rejecting undeclared resets and subsequent whole-turn errors (accepted 0247, #337). Library family advances together to 0.26.0 under decision 0048.

## 0.23.0

- STEPALL-M release integration: optional host stage timing (types/path) and corrected XYZ rounding at the written rotary angle (post/replay, issue #324); retain main’s GC-17 block-time and rotary-boundary APIs. Library family advances together after 0.22.0.

## 0.22.0

- GC-10: `compare_simulator_range` checks bounded LinuxCNC fixtures against the original unsplit source and independently replayed prefix state. `SimReport` gains default-empty `boundaries` and the public `SimBoundary` type; struct constructors must supply the new field. Ordinary whole-program JSON remains compatible.

- Lockstep library-family version for GC-17’s machine-profile and post API additions (accepted decision 0222, lane GATES). Existing whole-program replay behavior is unchanged.

All notable changes to `openchuck-gcode` are recorded here.

## Unreleased

- GC-10: `compare_simulator_range` accepts a boundary feed or speed that matches the original's single-precision rounding to 1e-6, as LinuxCNC holds them (F2755.6 reads back as 2755.60009765625; lane HELICALFACES). No API change.

- `compare_with_moves` exposes each actual replayed move with its source group/segment, controller counter shift and programmed fed duration. Observations are provisional until the complete comparison succeeds. Existing `compare` delegates with a no-op observer and preserves its result/structural checks; no motion is silently accepted because a callback ran.

- **Breaking family release (0.25.0, FACING, decision 0252):** `StrategyInput::facing` adds the optional whole-setup context. Host struct literals supply the new field; use `None` for other strategies and legacy whole-stock Facing. All family crates advance together under 0048. No project-schema migration.

- Version 0.24.0 with the library family (GC-17, decisions 0222/0245): configurable short-block reporting and geometry-preserving slowdown; profile/post/report APIs extended. Legacy project copies retain Off.

- Use the written rotary angle when identifying rounded-away moves; keep tip comparison against the original IR and unchanged tolerances (issue #324).
- Version 0.21.0 with the library family (decision 0241): `PostJob` and `ConvertOptions` require `a_range` and work-frame `safe_z`; `Step::Unwind` is replaced by the non-moving `RotaryBoundary`. `Regeneration` keeps `canonical_toolpath`, and `Application::planned_files` takes the regeneration. Hosts connect canonical groups through `openchuck_path::program::connect_group` in file order before posting.

- Replay `G92.4 A0 S0` with the Z1 firmware's strict `|A| > 360` condition (GC-09, decision 0241, #292). Exactly +360 and -360 remain unchanged, so subsequent absolute A motion is checked against the actual controller convention. XYZ and spindle state are unchanged by this command.
- Version 0.20.0 with the library family (`MachineProfile` in `openchuck-types` gains `min_spindle_rpm` and `max_spindle_rpm`, and `ProfileField` gains `SpindleRange`, breaking changes; decision 0231).
- Version 0.19.0 with the library family (`BrepEdge` in `openchuck-types` gains `curve`, a breaking change; decision 0227); no change to this crate's items.
- Version 0.18.0 with the library family (`PencilFinishing` in `openchuck-path` becomes a struct with a private field, no longer `Copy` or `Eq`, a breaking change; decision 0226); no change to this crate's items.
- Version 0.17.0 with the library family (`Tab` in `openchuck-types` gains `auto`, and `RotaryFrame::machine` is removed, breaking changes; decisions 0201 and 0202).
- Version 0.16.0 with the library family (decision 0204); no change to this crate's items.
- Version 0.15.0 with the library family (`PostJob` and `ConvertOptions` in `openchuck-post` gain `operation_spindle`, a breaking change; decision 0196).
- Version 0.14.0 with the library family (`RegionConstraint` in `openchuck-path` gains `boundary_limits` and `watch`, a breaking change; decision 0176).
- Version 0.13.0 with the library family (`StrategyOutput` and `GeneratedPath` in `openchuck-path` gain `warnings`, a breaking change; decision 0175).
- Version 0.12.0 with the library family (`StrategyInput` in `openchuck-path` gains `keep_out`, a breaking change; decision 0161).
- Ready to publish to crates.io (decision 0138): an `include` list (the library sources and the files they read; the tests stay out of the package), keywords and categories, and a README that links the repository. No API change.
- Version 0.11.0 with the library family (`PostJob` in `openchuck-post` gains `generator`, a breaking change; decision 0151).
- Version 0.10.0 with the library family (`assemble::smoothed_contacts` in `openchuck-path` takes the rotary mapping, a breaking change; decision 0115).
- Version 0.9.0 with the library family (`MaxFeeds` in `openchuck-types` gains `rotary_in_feed`, a breaking change; decision 0076).
- Version 0.8.0 with the library family (`Pass` in `openchuck-path` gains `rapids`, a breaking change; decision 0051).
- `compare` counts no A travel for the program's first move (from the controller's A to the first group's start), as the IR does not, so a toolpath that starts away from A 0 passes gate 2; moves between groups still count on both sides (decision 0036).
- Version 0.7.0 with the library family (`Tool` gains `centre_cutting` and `Pass` gains `stock`, breaking changes; decision 0033).
- Version 0.6.0 with the library family (`PostJob` and `ConvertOptions` in `openchuck-post` gain `operation_names`, a breaking change; decision 0028).
- Version 0.5.0 with the library family (the B-rep face record in `openchuck-types` is a breaking change, OI-123).

- Crate created as a shell (work breakdown slice 0).
