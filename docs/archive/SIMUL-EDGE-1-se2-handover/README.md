# SIMUL-EDGE SE-2: hand-over note and timing cases

The work-in-progress state of SE-2 (the proven move check for simultaneous finishing, OP-12) on branch `claude/lane-simul-edge-se2-proven-check`, as of 27 Nov 2026. It is cited by decision record SIMUL-EDGE-1 (`docs/decisions/SIMUL-EDGE-1-continuous-seed-and-whole-tool-clearance.md`, on branch `docs/decision-SIMUL-EDGE-1-continuous-seed-and-clearance`; the record has no number yet). This folder is raw evidence and a working note, not a source of truth and not a verification of generated G-code (NFR-12).

Files:

- `README.md`: this note.
- `timed.sh`: the timing script.
- `cases/`: the project files of five of the six timing cases, without their models (see *How to run the six cases*).

## Where the work is

- **Code:** this branch. It sits on SE-1 (`250b6f05`, the content of PR #933), with the SE-2 commits after it. The temporary debug code (the `GSTAT` and `MDBG` counters, the `NOCHECK` switch) has been removed. The counters are no longer available, and `timed.sh` does not use them.
- **Record:** branch `docs/decision-SIMUL-EDGE-1-continuous-seed-and-clearance`, commits `eb8370b4`, `267dc823`, `180eb926`, `330bf17f`, `83359fff` and `2b99f409` on top of #926's head. They are for #926's next allowed push.
- **Owner's answers in force** (relayed, 27 Nov 2026; written into the record by `83359fff` and `2b99f409`):
  - **Q1 A:** the first sample spacing grows the tool by up to half the tolerance. A failed sample is rechecked finer, a quarter of the growth each time, three times (`CHECK_GROWTH` = 0.5, `CHECK_LEVELS` = 4 in `simultaneous.rs`).
  - **Q2 A:** the move check proves against the operation's tolerance. There is no separate check tolerance.
  - **Q3:** the clear-pose work (clearance jumps, the whole-stretch bound, reuse of triangles and clips along a move) goes to a new lane. It is not part of SE-2.

## What each piece does

### `crates/openchuck-path/src/gap.rs` (the tool-gap query)

- **Measure C:** the depth below each face, along its own normal, with the layer. A face counts tool points only down to `limit` below it.
- **Convex-neighbour rule** (the owner's option A). Below a face, a point counts only where it is also at least the acceptance below the planes of the faces that meet it at a convex edge or corner (`clips`). Above a face, clearance is counted without the clips. See `convex_neighbours`.
- **Moving queries** use the Lagrangian dual over the prism (`measure`, `multipliers`, `dual_over`), as in SE-1. The move check no longer uses them.
- **Single-pose queries** (`measure_single`):
  - Bound: planes touching the tool from below (`cheap_planes`), bounded at the piece's corners. Where that is short of the acceptance, the bound is taken over the polytope of the region and the plane (`polytope_bound`, `polytope_lowest`), which can show the region empty.
  - Real values: columns at the centre, at the corners and at the polytope's lowest corner, and the free lowest point.
- **Separation tests:**
  - `beside`: the sides, every corner neighbour's plane, and towards the tool's nearest point;
  - `below_layer`: leaning planes towards the layer's floor;
  - a side margin of 1e-7 mm (`SIDE_MARGIN`).
- **Search:** first in, first out among tied pieces. A capped search with nothing found answers "unproven" (a lower bound, with the upper bound infinite), never "no surface".
- **Wall filter** (`skip_walls_thinner_than`), with `MeshSurface::walls` and `wall_at`, used by the thin-wall classification.

### `crates/openchuck-path/src/refine.rs`

- Refines a STEP tessellation at load, by longest-edge bisection with the new points on the exact faces. An edge midpoint shared by two faces is projected onto both faces in turn.
- Refines until every triangle's error is at most a quarter of the query limit, with a cap of 4 million triangles.
- `BrepSurface::tool_gap` uses it when the largest measured error is over a quarter of the limit. The result is cached per target.

### `crates/openchuck-path/src/simultaneous.rs` (the move check)

- **Layer:** the layer is 1.25 × the tolerance (`LAYER`).
- **`check_motion`:** proves a move by single-pose checks of the tool grown by `CHECK_GROWTH` × tolerance (half, by Q1 A).
  - The samples are spaced so that no tool point moves more than twice the growth between them.
  - A failed sample is rechecked finer, with a quarter of the growth, for `CHECK_LEVELS` = 4 levels (the first check plus three rechecks).
- **Travel:** counts only the tool points within the part's reach of the rotary axis.
- **`thin_wall`:** classifies a failed move as a thin wall. It casts a ray through the wall, then makes a second query that leaves out the wall's triangles, so a real gouge still wins. Its areas get `UnreachedReason::ThinWall` and their own warning (topic "Thin walls").

### `crates/openchuck-testkit/src/contact.rs` (the independent checker)

- `Geometry::face_gap(tool, tip, axis, layer, inside, settings)`: the same measure, written separately.
- It finds convex neighbours pair by pair (`convex_planes`).
- It bounds pieces by tangent and leaning planes, and over the polytope's corners (`corners_least`) where there are clips.

## Tests to keep passing

`cargo-nextest` is not installed in this container, so these use `cargo test`:

```bash
cargo test -p openchuck-path --lib simultaneous::tests::req_op_12      # the 9 move-check unit tests
cargo test -p openchuck-path --test integration tool_gap::             # the 21 tool-gap tests
cargo test -p openchuck-path --test integration simultaneous_walls::   # the thin and thick tubes
cargo test -p openchuck-testkit --test contact                         # the 22 checker tests
```

- **The 9 move-check unit tests** in `simultaneous.rs` (`simultaneous::tests::`):
  - `req_op_12_the_move_check_finds_a_cut_deeper_than_its_layer` (the plunge);
  - `req_op_12_a_moving_tool_grazing_a_face_is_proven_clear_at_once`;
  - `req_op_12_the_crankshaft_pin_root_move_is_proven_clear`;
  - `req_op_12_a_grown_tool_grazing_the_crankshaft_web_is_settled_at_once`;
  - `req_op_12_a_flat_end_along_a_blisk_blade_is_settled_at_once`;
  - `req_op_12_a_flat_end_overhanging_the_blisk_rim_is_settled_at_once`;
  - `req_op_12_a_sample_the_coarse_growth_fails_is_proven_finer`;
  - `req_op_12_a_move_that_dips_between_its_ends_is_found`;
  - `req_op_12_a_move_the_check_proves_cuts_no_deeper_at_any_pose` (a property test against the checker along moves).
- **The 21 tests in `tests/tool_gap.rs`.**
- **Both property tests in `tests/tool_gap.rs`**, against the independent checker:
  - `tool_gap::req_op_12_tool_gap_agrees_with_the_independent_checker`;
  - `tool_gap::req_op_12_tool_gap_grazing_a_face_agrees_with_the_independent_checker`.
- **`tests/simultaneous_walls.rs`:**
  - the thin tube (0.004 mm wall, 96 facets): 98.2 s, listed as long in `.config/nextest.toml`;
  - the thick tube: 0.4 s.
- **`openchuck-testkit/tests/contact.rs`:** 22 tests, including the square-edge test.

## How to run the six cases

The models are the public test models, copied into each case's `models/` folder under the name its `project.toml` gives (the MD5 sums were checked against `testdata/`):

| Case | Project | Model to copy into `models/` |
| --- | --- | --- |
| `lobed` | `testdata/projects/lobed` (copy the whole folder) | not stored: generated by the tests (see the command below) |
| `crank_stl` | `cases/crank_stl` | `testdata/stl/crankshaft.stl` as `1e3aad73c0dd8ea5cbc114b6b2ae1cf6.stl` |
| `blisk_flat`, `blisk_ball` | `cases/blisk_flat`, `cases/blisk_ball` | `testdata/stl/blisk.stl` as `9fd8d40508a0e423dd17ef53b9f3b74f.stl` |
| `step_flat`, `step_ball` | `cases/step_flat`, `cases/step_ball` | `testdata/step/valid/blisk.step` as `blisk.step` |

```bash
cargo build --release -p openchuck-cli
# The STEP worker is built only with the kernel feature (the first build of
# the OpenCASCADE kernel takes 8 to 71 minutes):
cargo build --release -p openchuck-import --features kernel --bin openchuck-step-worker
W=$(mktemp -d); A=docs/archive/SIMUL-EDGE-1-se2-handover
mkdir -p $W/bins $W/runs
cp target/release/openchuck-cli target/release/openchuck-step-worker $W/bins/   # the STEP cases need the worker beside the CLI
cp -r testdata/projects/lobed $A/cases/* $W/runs/
# The lobed model is not stored: the tests generate it (sample_projects::write_stl
# of sample_projects::lobed() at 201 by 720, apps/openchuck-cli/tests/common/).
# Write it to $W/runs/lobed/models/part.stl, for example from a scratch test
# that calls those two functions.
for c in crank_stl; do mkdir -p $W/runs/$c/models; cp testdata/stl/crankshaft.stl $W/runs/$c/models/1e3aad73c0dd8ea5cbc114b6b2ae1cf6.stl; done
for c in blisk_flat blisk_ball; do mkdir -p $W/runs/$c/models; cp testdata/stl/blisk.stl $W/runs/$c/models/9fd8d40508a0e423dd17ef53b9f3b74f.stl; done
for c in step_flat step_ball; do mkdir -p $W/runs/$c/models; cp testdata/step/valid/blisk.step $W/runs/$c/models/blisk.step; done
RUNS=$W/runs $A/timed.sh $W/bins/openchuck-cli lobed crank_stl blisk_flat blisk_ball step_flat step_ball
```

- `timed.sh` runs `regenerate` with 4 threads and the LinuxCNC post. For each case it prints:
  - the wall time;
  - the program's peak RSS;
  - the STEP worker's peak RSS;
  - the number of G-code lines.
- Each case is limited to 1 h.
- **Expected line counts at growth half:**
  - `lobed`: 1,818 lines;
  - `crank_stl`: 20,506 lines.
  - Neither reports a gouge or a thin wall.

## Measured (release build, 4 threads, this container)

| Case | Old clamped check, 25 Nov (proved little) | Move check off | Growth a tenth | Growth half (Q1 A, the default now) |
| --- | --- | --- | --- | --- |
| `lobed` | 1.7 s | 2.0 s | 46.2 s | 25.1 s and 25.2 s (1,818 lines, 328 MB) |
| `crank_stl` | 1.7 s | 2.0 s | 678.6 s | 302.2 s and 309.2 s (20,506 lines, 47 MB) |
| `blisk_flat` | 7.8 s | not run | over 1 h; about 3 to 4 h from its rate | not run |
| `blisk_ball`, `step_flat`, `step_ball` | 40.7 s, 140 s, 168 s | not run | not run (held by the coordinator) | not run |

- **The numbers to beat** (growth half): `lobed` 25.1 s and `crank_stl` 302.2 s.
- Not measured under this method:
  - the blisk cases;
  - the STEP load step;
  - the refined mesh's memory.

## Cost model

- Samples per move = tool travel / (2 × growth).
  - Travel is 0.2 to 2.5 mm, mostly from turning about the rotary axis.
  - At a growth of half of 0.01 mm that is about 90 samples per move; at a tenth, about 430.
- Each sample is a single-pose query of about 0.3 ms, with about 3 pieces.
- So the cost is about proportional to travel / (growth × tolerance). No sound way is known to need fewer samples at a given growth. That is the clear-pose work of the new lane (Q3).

## Ideas for the new lane (Q3, not tried here)

1. **A cheap exact test for poses well clear of the surface,** skipping samples there.
   - The Euclidean distance from tool to mesh is 1-Lipschitz in tool displacement, unlike C.
   - So a sample whose tool is a distance c clear of every face proves every pose within a displacement of c − (layer + growth) of it, and the samples can jump that far.
   - Poses in contact still need full sampling.
2. **Bounding a whole stretch of a move before sampling.** Either of:
   - one grown-tool query with the growth equal to the stretch's whole travel, where the clearance allows it;
   - the exact motion bound (the dual over a moving query) with a small piece budget first, falling back to samples only where it is undecided.
3. **Reuse along a move:** neighbouring samples see the same triangles, so the candidate triangle set and the per-triangle clips could be reused.

## Not done before a pull request

- **Clippy fails** (`cargo clippy -p openchuck-path -p openchuck-testkit --all-targets`, 27 Nov 2026):
  - 62 "indexing may panic" errors, mostly in `refine.rs`, with some in `gap.rs`, `brep.rs` and `openchuck-testkit/src/contact.rs`;
  - warnings:
    - 11 uses of the disallowed `std::collections::HashMap` (the project uses `BTreeMap` for determinism, GC-07);
    - the unread field `Refined::faces`;
    - 3 complex types;
    - 2 collapsible `if`s;
    - one strict float comparison.
- **Not yet done:**
  - coverage;
  - `cargo xtask` checks;
  - the changelog;
  - the record's Verification section;
  - the version bump (provisional 0.74.0).

## Lane SE-PERF (27 and 28 Nov 2026): the move check made faster

Branch `claude/lane-se-perf`, from `906f49c` above. A performance lane only: the measure, the tolerance, the layer (1.25 × the tolerance), the convex-neighbour rule, the growths (`CHECK_GROWTH`, `CHECK_LEVELS`) and the sample positions are unchanged. This section is a working note, as the rest of this file is. Its evidence is in `se-perf/` beside this file. It is not a verification of generated G-code (NFR-12).

### Measured (release build, 4 threads, this container, each case run alone)

| Case | `906f49c` here | Now | G-code lines | Result |
| --- | --- | --- | --- | --- |
| `lobed` | 27.5 s | 3.2 s | 1,818 | byte-identical to `906f49c`'s output |
| `crank_stl` | 342.3 s | 11.9 s | 20,506 | byte-identical to `906f49c`'s output |
| `blisk_ball` | not run (hours) | 135.2 s | 129,975 | no gouge or thin wall reported |
| `blisk_flat` | not run (hours) | 3,579 s | 110,916 | 4 gouge areas reported; completes, just inside the 1 h limit of `timed.sh` |
| `step_ball` | not run | 522.9 s | 95,222 | 163 gouge areas reported (see *The STEP gouge areas*) |
| `step_flat` | not run | did not finish within the 1 h limit | | |

- With the move check off, `lobed` and `crank_stl` took 2.0 s.
- `lobed` and `crank_stl` are byte-identical to `906f49c`'s output at 1, 2 and 8 threads (GC-07). `blisk_ball` gives the same program at 1, 4 and 8 threads.
- `blisk_ball`'s program was the same at every build of this lane that completed it. There is no `906f49c` output of a blisk case to compare with: they did not finish there.
- Not installed here: `cargo-nextest` (times are `cargo test`'s) and `cargo-llvm-cov` (no coverage figure).

### What was built

- **A stretch is bounded before it is sampled** (the owner's method 2). `check_motion` is given the move's motion. A stretch of samples is asked as one query: the tool grown as its samples' is, over the stretch's own motion (the shift and A's turn), answered from each face's own bounds alone (`GapQuery::within_pieces(0)`).
  - Proven, the stretch needs no sample: the grown tool is no deeper than the tolerance at every pose of it, which is more than its samples show.
  - Undecided, it is asked once more with the finest level's growth, which proves the tool itself at every pose as that level's samples would. Still undecided, it is halved, down to one sample. What is left is sampled exactly as before.
- **The bounds hold over a part of a move** (`gap.rs`, `Search::bound`, `Body::cheap_planes`). Each plane is fixed to the surface, with the least the tool reaches along it at any pose of the part (`Body::dual_over`). What the planes bound at one pose they bound over the part.
- **The bound over a turning part is tighter** (`Body::dual_over`). The turn and the shift are bounded as one motion, from a point of the tool on its axis (a ball end's centre), not as two worst cases. Before this a stretch proved about 4 samples; after it, a whole move on the lobed part.
- **A query with an acceptance is answered from the bounds first** (`Search::prove`): no piece is split and no real value looked for. The target is the accuracy above the acceptance, and the bounds are the search's own first bounds, so what it proves the search proves too.
- **A sample with a finer recheck splits at most 4,096 pieces** (`SAMPLE_PIECES`; accepted by the coordinator as inside the owner's rules, 27 Nov 2026).
  - *Why:* on the crankshaft, nine samples (the ball over the edge of the end face at X 0) ran to the search's cap of 200,000 pieces, 16 to 18 s each, about 147 s of the 342 s job on one thread. Each answered "unproven" and was then proven at the next finer level.
  - *The number:* no sample that was proven needed more than 128 pieces on the crankshaft or the lobed part, nor more than 4,096 on the blisks (the largest seen was between 2,049 and 4,096, twice, on the flat-end blisk; a sample cut short there is rechecked finer).
  - *What it cannot change:* a sample cut short at the cap is unproven, never a pass, and is checked again finer. The last level (the finest growth) keeps the search's full cap, so what is reported as a gouge or a thin wall is decided as before.
- **Separating planes the single-pose bound lacked** (`Body::parted`; accepted by the coordinator, 27 Nov 2026).
  - *What was wrong:* where a tool passes a small piece of a face a few micrometres clear (a ball under it below the layer; a flat end's corner at the layer's floor, also inside the thin region a nearly level neighbour leaves a face to count), nothing counts, but only a plane leaning as the tool's own surface does parts them. The bound had none, so the piece was halved until the pieces ran out: unproven, 17 to 30 s a sample at the finest growth. On the blisks that was thousands of samples (the "hours").
  - *What is added:* the direction from points of the piece's column to the tool's nearest point, that direction laid square to the face, and the direction at the column's own nearest point to the tool, each asked of the face's prism above the layer's floor and of the clipped region.
  - *What it can change:* at the last level such a sample used to end as a reported gouge. It is now proven. So on a part with such places it removes gouge reports that came from the search running out of pieces, not from the measure. Each case has a test against the independent checker (`contact.rs`), seen failing first.
- **A failed move ends at its first failure** (`MoveCheck::span`): a failed sample is checked finer before the next is taken. The failure returned is the same one.
- **The test that the tool lies beside a piece** is made once, when the piece is measured, and a real tool point over the face answers it at once (`Body::over`). The answers do not change.
- **The counting surface of adaptive spacing passes the tool gap query on** (`adaptive.rs`, commit `4c4d499`). It did not, so every query took the trait's default (every triangle searched, their neighbours found afresh). `adaptive::req_op_05_req_op_04_simultaneous_axial_passes_are_added_until_the_allowed_excess_is_met` did not finish in an hour on `906f49c`; it takes 31 s.

### The gouge areas the blisks report (replayed through the independent checker)

- **`blisk_flat` (4 areas).** Of 10,532 failed samples logged before the retries, 28 were replayed (`se-perf/stl-flat-gouge-replay.txt`). The 10 deepest are real overcuts: the checker reads them at or beyond the layer. The other 18 are undecided by both: the product's bound is 0.0101 to 0.0125 mm and the checker runs out of 200,000 cells.
- **`step_ball` (163 areas).** Of 791 failed samples, none has a real tool point found deeper than 0.0099 mm. 20 were replayed (`se-perf/step-ball-gouge-replay.txt`): on the mesh alone, with no triangle error, the product proves all 20, and the checker reads all 20 within the tolerance. The reports come from the measure on an exact part, not from this lane's changes:
  - the bound of a face beside an almost level convex neighbour is the tool's depth below the face's own plane (0.0082 to 0.0087 mm in four traced samples, where the checker reads 0 to 0.0073 mm), and the triangle's error (OI-9974) is then taken off it;
  - three remedies were measured with switches, none built in (`se-perf/step-ball-remedies.txt`, `se-perf/step-ball-refinement-targets.txt`): refining to an eighth and a sixteenth of the layer (164 and 165 areas, 543 and 688 s, 280 and 481 MB); the fitter keeping a move only when its error is at most the tolerance less the stated accuracy (163 areas, 391 s); the bound not taking its shortcut in that band (166 areas);
  - at a sixteenth, two real overcuts (0.0124 and 0.0125 mm, the checker agreeing) are found that a quarter does not show.
  - The owner decided on 27 Nov 2026 that the measure on exact parts is redesigned in SE-2 (lane SIMUL-EDGE). The refinement target stays a quarter of the layer.
- The warning prints the tolerance as the depth when no deeper point was found ("cutting about 0.010 mm into it"). A draft wording that says so is with the coordinator.

### Found and not this lane's to fix (all on `906f49c`)

- `simultaneous::req_op_12_every_tool_profile_finishes_the_lobed_part_without_gouging` fails: the move check's grown tools cannot be built for a concave custom profile (`grow_profile`, `simultaneous.rs` line 857 at `906f49c`).
- `simultaneous::req_op_05_exact_faces_drive_the_path_with_their_own_normals` fails: on the exact cylinder of radius 12 the tool gap reads a 6 mm ball resting on the surface as cutting at the layer (lower −0.012614, upper −0.012386), so every move is left out. `906f49c` gives the same answer to the last digit for the same pose; the test did not finish there in 90 minutes.
- `cargo xtask` (the `messages` check): two "`{} areas`" strings in `simultaneous.rs` (25 and 26 Nov).
- `cargo clippy -p openchuck-path --all-targets`: 53 errors and 18 warnings, none on a line this lane changed.
- Refining a STEP tessellation runs on one thread (78 s at a quarter on the blisk, 373 s at a sixteenth): issue #990.

### Checks run on the final code (28 Nov 2026)

- `cargo fmt --all -- --check`: clean. `cargo xtask deps`, `licences`, `records`, `trace`: pass (105 of 105 P1 requirements have tests).
- `cargo test -p openchuck-path`: the library's 152 tests pass. Of the 896 integration tests, every module passes except `simultaneous`, which has the two inherited failures above; seven long tests of that module were still to be run one at a time when this was written (see the coordinator's log for their result).
- New tests: 6 move-check unit tests and 2 tool-gap tests (one a property test against the independent checker), each written from the case it covers and seen failing first, except the first two stretch tests, which were confirmed to fail with the stretch pass disabled after they were written.
- The thin tube of `simultaneous_walls` takes 2 to 3 s (was 98.2 s) and is off the long list in `.config/nextest.toml`. Several tests of `simultaneous_hidden`, `simultaneous_off_axis` and `simultaneous_rings` take over a minute under `cargo test`; they were not timed alone here and the long list was not otherwise changed.

### Not done

- Method 1 (clearance jumps) was not built, by the coordinator's decision: at every pose of a finishing move the tool is in contact, so its clearance is about zero. The stretch bound is the directional form of the same idea.
- The flat-end blisks are still at or over an hour. No search runs to its cap there any more; what is left is the number of queries (about 720,000 stretch queries in 5 minutes on `blisk_flat`), much of it moves that fail and are split and retried by the refinement, which this lane did not change.
- Coverage, the changelog and the record's Verification section.

### The measurement switches

`se-perf/*.diff` are throwaway patches against the code head `5f66a7d`, not applied in the tree: the dump of every failed final sample and of the refined mesh (`dump-tracing.diff`), the refinement target (`refinement-target-tracing.diff`), the two remedies (`option-b-tracing.diff`, `shortcut-band-tracing.diff`), the timing trace (`timing-tracing.diff`) and the replay through `contact.rs` on the triangles round each pose (`replay-test.diff`, `replay-test-flat.diff`).
