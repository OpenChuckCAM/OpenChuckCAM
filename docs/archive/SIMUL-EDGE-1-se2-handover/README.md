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
| `lobed` | `testdata/projects/lobed` (copy the whole folder) | already in the project |
| `crank_stl` | `cases/crank_stl` | `testdata/stl/crankshaft.stl` as `1e3aad73c0dd8ea5cbc114b6b2ae1cf6.stl` |
| `blisk_flat`, `blisk_ball` | `cases/blisk_flat`, `cases/blisk_ball` | `testdata/stl/blisk.stl` as `9fd8d40508a0e423dd17ef53b9f3b74f.stl` |
| `step_flat`, `step_ball` | `cases/step_flat`, `cases/step_ball` | `testdata/step/valid/blisk.step` as `blisk.step` |

```bash
cargo build --release -p openchuck-cli          # also builds openchuck-step-worker
W=$(mktemp -d); A=docs/archive/SIMUL-EDGE-1-se2-handover
mkdir -p $W/bins $W/runs
cp target/release/openchuck-cli target/release/openchuck-step-worker $W/bins/   # the STEP cases need the worker beside the CLI
cp -r testdata/projects/lobed $A/cases/* $W/runs/
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
