# Changelog

## 0.71.0

- **Family release (0.71.0; PATH-QUALITY, PQ-2b; issue #786; decision 0695, Q9 part 3, accepted):** `openchuck-path` changes ramp output (a whole ring whose one turn would descend more than the largest depth per traverse is ramped back and forth over itself). No change in this crate.

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

## Unreleased (DEPS-UPDATES)

- **Changed (template engine):** post templates render with `minijinja` 3.1.0 (was 3.0.3). No public API change. Built-in output does not change. A template a user wrote may see the engine's changes: rendering uses slightly more fuel (filters and operators now count; the budget is unchanged and the built-in posts use about a tenth of it), an error while writing `{{ ... }}` points at the whole expression, and several filters behave differently for unusual input (`max` and `groupby` keep the first of equal items; `none` and undefined values in `first`, `last`, `length` and `join`; `2 ** -1` is 0.5; `format` prints floats as Python's `str()`; `sort` by several attributes ignores case). A template that uses `max` or `groupby` on items that compare equal (including equal ignoring letter case) can render a different item than before, so a post a user wrote should have its output checked again after this update.

## 0.63.0

- **Family release (0.63.0; lane FACEFLOW; issues #791 and #854; decisions 0675 and 0686, accepted):** `openchuck-path`'s `UnreachedReason` gains the variant `TooShort`, a breaking change; no change to this crate.

## 0.62.0

- **Family release (0.62.0; lane FACING-GRIP; issue #500; decision 0685, accepted):** `openchuck-path`'s `FaceDescription` gains the public field `unfaced` and `FacingGeometry::Round` the field `unfaced`, a breaking change; no change to this crate.

## 0.61.0

- **Family release (0.61.0; SWARF-LEADS; issues #572 and #624; decisions 0682 and 0683, accepted):** `openchuck-path` changes output (a swarf lead or helix that would cut a face next to the face set deeper than the pass is refused; swarf's note gives the reach past the rulings after the lift); no public API change and no change to this crate.

## 0.60.0 (POST-COMMENTS; issue #851; decision 0681, accepted)

- **Breaking (0.60.0, POST-COMMENTS; decision 0048):** `PostOptions::active_comment_prefixes` (post key `active_comment_prefixes`, a list, empty when a post leaves it out): the openings of comments the controller acts on, such as LinuxCNC's `MSG,`. Text from the project that would start such a comment (the title, revision, generator, operation comments and tool names, after the sanitiser) and each line a wrapped comment starts are written with the new constant `ACTIVE_COMMENT_GUARD` (`_`) in front, counted in the line limit; the post's own header and footer are kept. Bad entries are refused at load (`options.active_comment_prefixes`). The LinuxCNC post lists LinuxCNC's fourteen openings; the Makera Z1 post lists none. Struct literals of `PostOptions` add the field. `PostOutput::guarded_comments` (new public field) lists each guarded text once, as a `GuardedComment` (new): its `CommentSource` (title, revision, generator, an operation or tool by tag, or the post's own comment), the opening matched, the text as given and where the mark went. No golden program changes; a LinuxCNC program for a project whose name starts with one of the openings gains the mark.

## Unreleased (FUZZ-NFR07; issue #794; decision 0679, accepted)

- **Fixed:** a template `namespace` that holds itself (directly, in a list or through another namespace) is freed after the render instead of leaking; every namespace a render makes is emptied when it ends. A `namespace` key that is not a name is now a template error. No output changes. No public API changes.

## 0.59.0

- **Family release (0.59.0; lane Z1-SETUP, PR 2; issue #692; decision 0673):** `openchuck-types`'s `Tool` gains the public field `stick_out`, a breaking change; no change to this crate.

## 0.56.0

- **Breaking (0.56.0, POST-SAFETY; issue #703; OI-933; decision 0656, accepted):** `PostOptions::max_line_length` (post key `max_line_length`, 20 to 4096 characters, read as unset when a post leaves it out): a comment line longer than it is wrapped at spaces into comment lines that fit (a word longer than a line is cut; an `N` word stays on the first line), after the NFR-12 notice check, so the notice is kept word for word. A post with a value outside 20 to 4096 is refused at load (`options.max_line_length`). The Makera Z1 post sets 63 (the firmware source's 64-byte line, OI-933), the LinuxCNC post 255. Struct literals of `PostOptions` add the field. Every Makera Z1 program writes the notice on four comment lines instead of two, and a title or operation comment longer than 63 characters on more than one line; no other byte changes.
- **Fixed (issue #839 item 3):** the NFR-12 notice check's opening comment block ends at a line that is not one comment and nothing else, such as `(title) G90`, so a notice written after it is refused. Built-in output does not change.

## Unreleased (POST-SAFETY; issues #574, #598, #599; decisions 0654 and 0657)

- **Fixed (output of a post with coolant on):** with `coolant = "flood"` or `"mist"`, the built-in template now writes `M9` before the next tool change and before the footer once coolant is on (decision 0654, option A). Built-in output does not change: both built-in posts have coolant off.
- **Fixed:** with `safety_notice` on, the notice must be written as whole comment lines, in order, in the comment block at the top; text inside a longer line or after the program has started is refused (decision 0657 item 1, OI-2083).
- **Docs:** `UnwindMode::Rotate` says what it does: nothing is written at a boundary and the A words carry the whole turns (decision 0241 item 6). No public API changes.

## 0.54.0

- **Family release (0.54.0, or the next free version, as the Merge Manager sequences it; RAMPENTRY; issues #639, #642; decision 0543, accepted):** `openchuck-path`'s `EntryParameters` gains `allow_single_point_plunge` and loses `back_and_forth`, and `StrategyInput::entry_safety` and the `entry_safety` module are removed (decision 0543 D10), a breaking change; no change to this crate.

## 0.52.0

- **Family release (0.52.0, or the next free version, as the Merge Manager sequences it; PATHFEED; issue #641; decision 0547, accepted):** `openchuck-path`'s `Pass` gains the public field `plunge_fed` and `facing::end::EndPassPlan` the public field `passes_above_area`, a breaking change; no change to this crate.

## 0.51.0

- **Family release (0.51.0, or the next free version, as the Merge Manager sequences it; MSGREVIEW step 3; issue #567; decision 0536, accepted):** `openchuck-path` removes `flutes::WARNING_LEAD` and adds the `topic` module, a breaking change; no change to this crate.

## 0.50.0

- **Family release (0.50.0, or the next free version, as the Merge Manager sequences it; MSGREVIEW step 1; issue #567; decision 0536, accepted):** `openchuck-path` replaces `PathNote` and `PathWarning` with one `Notice` type, a breaking change; no change to this crate.

## 0.49.0

- **Breaking (0.49.0, SAFETYNOTICE; OI-2000; decision 0533, accepted):** `PostOptions::safety_notice` (post key `safety_notice`, read as `false` when a post leaves it out; `true` in both built-in posts) and the constant `SAFETY_NOTICE`: on, the template writes the NFR-12 notice as two fixed comment lines after the generator line. A post with it on and `comments = false` is refused at load (`options.safety_notice`), and a template that does not write it while it is on is refused at posting. Struct literals of `PostOptions` add the field. Every program from a built-in post gains the two lines; no other byte changes.

## 0.47.0

- **Family release (0.47.0, TABCUT; issue #555; decision 0527, accepted):** `openchuck-path`'s `RegionConstraint` gains the public field `outside`, a breaking change; no change to this crate.

## 0.46.0

- **Family release (0.46.0, STEPALL-CLOSE; issue #351; decision 0526, accepted):** `openchuck-path`'s `Strategy::exact_contact` returns the kind of Exact STEP surface contact a strategy offers, a breaking change; no change to this crate.

## 0.39.0

- **Breaking family release (0.39.0, lane A-MOVES, issue #503; decision 0506, accepted):** `PostOptions::combine_rotary_runs` (post key `combine_rotary_runs`, read as `false` when a post leaves it out; `true` in both built-in posts) and `ConvertOptions::combine_rotary_runs`: a run of consecutive fed pure-A moves with the same written X, Y, Z, direction, kind and per-minute F is written as one block. Struct literals of `ConvertOptions` and `PostOptions` add the field. GC-17 reports count the blocks as written.

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

- Accept explicit clear pass-boundary metadata for nonmoving continuous-axis counter resets. Z1 rebases subsequent absolute words with existing signed/exact-360 semantics; rotate-only posts omit bookkeeping and add no rewind (accepted 0247, #337). Library family advances together to 0.26.0 under decision 0048.

## 0.23.0

- STEPALL-M release integration: optional host stage timing (types/path) and corrected XYZ rounding at the written rotary angle (post/replay, issue #324); retain main’s GC-17 block-time and rotary-boundary APIs. Library family advances together after 0.22.0.

## 0.22.0

- GC-17 (accepted decision 0222): minimum block-time profile/post fields and deterministic Report mode. The built-in Z1 uses the gate-4 measured 7 ms with window 32. Existing project copies remain Off, and Report leaves G-code unchanged. No Slow down or Merge.


All notable changes to `openchuck-post` are recorded here.

## Unreleased

- **Breaking family release (0.25.0, FACING, decision 0252):** `StrategyInput::facing` adds the optional whole-setup context. Host struct literals supply the new field; use `None` for other strategies and legacy whole-stock Facing. All family crates advance together under 0048. No project-schema migration.


- Breaking (0.24.0): `ConvertOptions` adds `short_blocks` and `feed_decimals`. Hosts use disabled defaults for legacy behaviour. `PostOutput`/`MachineProgram` return demand reports and an adjusted verification path. `PostOptions` accepts optional per-field overrides.

- Version 0.24.0 with the library family (GC-17, decisions 0222/0245): configurable short-block reporting and geometry-preserving slowdown; profile/post/report APIs extended. Legacy project copies retain Off.

- Correct XYZ at the written rotary angle before linear rounding, retaining the existing precision and replay error budget (issue #324).
- Version 0.21.0 with the library family (decision 0241): `PostJob` and `ConvertOptions` require `a_range` and work-frame `safe_z`; `Step::Unwind` is replaced by the non-moving `RotaryBoundary`. `Regeneration` keeps `canonical_toolpath`, and `Application::planned_files` takes the regeneration. Hosts connect canonical groups through `openchuck_path::program::connect_group` in file order before posting.

- Fix the Makera Z1's A-coordinate adjustment after `G92.4 A0 S0` at exactly +360 or -360 degrees (GC-04/09, decision 0241, #292). Firmware retains these values; following absolute A words now retain that offset too, avoiding an unintended full turn. The command remains a non-moving whole-turn counter reset, not a change to the saved work offset.
- Version 0.20.0 with the library family (`MachineProfile` in `openchuck-types` gains `min_spindle_rpm` and `max_spindle_rpm`, and `ProfileField` gains `SpindleRange`, breaking changes; decision 0231).
- Version 0.19.0 with the library family (`BrepEdge` in `openchuck-types` gains `curve`, a breaking change; decision 0227); no change to this crate's items.
- Version 0.18.0 with the library family (`PencilFinishing` in `openchuck-path` becomes a struct with a private field, no longer `Copy` or `Eq`, a breaking change; decision 0226); no change to this crate's items.
- Version 0.17.0 with the library family (`Tab` in `openchuck-types` gains `auto`, and `RotaryFrame::machine` is removed, breaking changes; decisions 0201 and 0202).
- Version 0.16.0 with the library family (decision 0204); no change to this crate's items.
- **Breaking (0.15.0):** `PostJob` and `ConvertOptions` have a new public field, `operation_spindle` (spindle speeds in rpm by the IR's operation tag; decision 0196, issue #249; TL-05, GC-14). An operation with an entry runs at it; any other at its tool's `ToolSetting::spindle_rpm`, as before. A bad entry is a `PostError::Setting` naming the operation. Code that builds a `PostJob` or `ConvertOptions` adds `operation_spindle: BTreeMap::new()`.
- Version 0.14.0 with the library family (`RegionConstraint` in `openchuck-path` gains `boundary_limits` and `watch`, a breaking change; decision 0176).
- Version 0.13.0 with the library family (`StrategyOutput` and `GeneratedPath` in `openchuck-path` gain `warnings`, a breaking change; decision 0175).
- Version 0.12.0 with the library family (`StrategyInput` in `openchuck-path` gains `keep_out`, a breaking change; decision 0161).
- Ready to publish to crates.io (decision 0138): an `include` list (the library sources and the files they read; the tests stay out of the package), keywords and categories, and a README that links the repository. No API change. The built-in posts are now in this crate's `posts/` folder (decision 0103), so the published crate carries them; their content and the posts' output are unchanged.
- Version 0.11.0 with the library family (`PostJob` in `openchuck-post` gains `generator`, a breaking change; decision 0151).
- **Breaking (0.11.0):** `PostJob` gains `generator`, the tool and its version (such as `OpenChuckCAM 0.11.0`) that both shipped posts write as a comment after the title, inside the `comments` option (NFR-16). A host passes the software version only, never a commit or a date, so one version always gives the same program (GC-07); a host that does not want the line passes `None`. Golden files hold `(OpenChuckCAM <version>)`, and `openchuck_testkit::golden::check` masks the version before it compares.
- Version 0.10.0 with the library family (`assemble::smoothed_contacts` in `openchuck-path` takes the rotary mapping, a breaking change; decision 0115).
- Version 0.9.0 with the library family (decision 0076). New: the template filter `feed` and `feed_str`, which write a feed with at least `decimals.f` places and at least `FEED_SIGNIFICANT_FIGURES` (three) significant figures, so a small F such as `4.237` is `F4.24`, not `F4.2`, and rounding is at most 0.5 % of a move's time. Both shipped posts write F with it; feeds of 100 and more are written as before.
- Version 0.8.0 with the library family (`Pass` in `openchuck-path` gains `rapids`, a breaking change; decision 0051).
- Version 0.7.0 with the library family (`Tool` gains `centre_cutting` and `Pass` gains `stock`, breaking changes; decision 0033).
- Version 0.6.0 with the library family (`PostJob` and `ConvertOptions` in `openchuck-post` gain `operation_names`, a breaking change; decision 0028).
- **Breaking (0.6.0):** `PostJob` and `ConvertOptions` gain `operation_names`, the display names the comments that name each operation use (GC-06, decision 0028). The comment now reads `operation <name>, tool T<n> <tool name>, about <s> s`.

- Version 0.5.0 with the library family (the B-rep face record in `openchuck-types` is a breaking change, OI-123).

- Crate created as a shell (work breakdown slice 0).
- Slice 6: neutral machine program (`to_machine_program`) and the named feed rules `linuxcnc`, `makera`, `inverse_time` and `contact`.
