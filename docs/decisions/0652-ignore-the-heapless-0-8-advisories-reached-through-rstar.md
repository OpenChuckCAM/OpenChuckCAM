# 0652: Ignore the heapless 0.8.0 advisories reached through rstar (NFR-14; issue #825; lane AUDIT-HEAPLESS)

- **Status:** Accepted by the project owner on 22 Nov 2026, relayed by the coordinator (decision 0049). Proposed by lane AUDIT-HEAPLESS the same day; the owner had first accepted option (c), the ignore below, as a risk acceptance and as the freeze exception for this sev1 CI fix, also relayed. The number is provisional: the Merge Manager assigns the final number at release.
- **Date:** 22 Nov 2026.
- **Context:** NFR-14 (dependency health), `docs/dependencies.md` (the advisory review rule and the `parry3d-f64` row), [decision 0052](0052-ci-speed-up.md) and [decision 0516](0516-ci-architecture.md) (`cargo audit --deny warnings` in the Fast checks job, required on every pull request). It changes no requirement, no architecture, no dependency, no `Cargo.lock` and no program behaviour.
- **Open items:** adds OI-2150 (remove the ignore when `parry3d-f64` moves to rstar 0.14 or later).

## Problem

On 18 Nov 2026 the RustSec database added nine advisories against heapless before 0.9.4, RUSTSEC-2026-0574 to RUSTSEC-2026-0582. All nine are classed `informational = "unsound"`, not as vulnerabilities. The Fast checks job runs `cargo audit --deny warnings`, which turns them into a failure, so every pull request has failed since then, even though `main`'s `Cargo.lock` is unchanged since the last green push run (issue #825). `cargo deny check` still passes.

heapless 0.8.0 reaches the workspace by one path only: `parry3d-f64` 0.31.1 → `rstar` 0.13.0 → `heapless` 0.8.0. `parry3d-f64` is used by `openchuck-geom` (picking) and `openchuck-path` (the surface queries).

## Options

1. **Update the lockfile only (`cargo update -p heapless`).** Not possible. Every advisory is patched only in heapless 0.9.4 and later. There is no 0.8.x fix, and rstar 0.13.0 and 0.13.1 both require `heapless ^0.8`.
2. **Bump rstar or `parry3d-f64`, or drop the path.** Not possible today. rstar 0.14.0 requires `heapless ^0.9`, but `parry3d-f64` 0.31.1, the latest release, requires `rstar ^0.13.0`, and parry's `master` branch (workspace version 0.31.1) still pins `rstar = "0.13.0"`. rstar is a mandatory parry dependency, not behind a feature. That leaves two routes: a `[patch]` fork of parry or rstar, or replacing parry with our own bounding volume hierarchy (the fallback in [decision 0015](0015-sp-4-ray-cast-sweep.md)). Both are far beyond a CI fix, and both touch geometry code.
3. **Ignore the nine advisories, with this reach analysis (chosen).**

## Decision

1. **`.cargo/audit.toml` ignores RUSTSEC-2026-0574 to RUSTSEC-2026-0582.** Each ID has a comment naming the affected API and why it is not reached. The block comment gives the removal condition and points here. `cargo audit` reads the file from the repository root, which is where CI runs `cargo audit --deny warnings`, so the workflow does not change. `deny.toml` needs no ignore, because `cargo deny check` passes. This is the repository's first audit exception, and future ones go in the same file, each with a record.
2. **Why nothing we build reaches the affected code:**
   - rstar 0.13.0 uses exactly one heapless type, `heapless::binary_heap::BinaryHeap`: a 32-element stack heap in its nearest-neighbour iterator (`src/algorithm/nearest_neighbor.rs`, lines 15 and 161).
   - None of the nine advisories names `BinaryHeap`. The affected APIs are:

     | Advisory | Affected API |
     | --- | --- |
     | RUSTSEC-2026-0574 | `sorted_linked_list::FindMutView::pop` |
     | RUSTSEC-2026-0575 | `spsc::Spsc::peek` |
     | RUSTSEC-2026-0576 | `IndexMap::truncate` |
     | RUSTSEC-2026-0577 | panicking `Drop` in `Deque::clear`, `HistoryBuf::clear` and `write`, `IndexMap::clear`, `retain` and `truncate`, and `vec::IntoIter::drop` |
     | RUSTSEC-2026-0578 | `SortedLinkedList::zeroize` |
     | RUSTSEC-2026-0579 | `Deque::try_from` (0.9.2 to 0.9.3 only) |
     | RUSTSEC-2026-0580 | the `arc_pool`, `box_pool` and `object_pool` treiber stack |
     | RUSTSEC-2026-0581 | `Deque::make_contiguous` |
     | RUSTSEC-2026-0582 | `IndexMap::insert` at `u16::MAX` entries |

   - In `parry3d-f64` 0.31.1, rstar is used only in `transformation::mesh_intersection`, which this workspace never calls. Our code uses `shape::TriMesh`, its BVH (`partitioning`), `query::{PointQuery, Ray}` and `math::Vector` (`openchuck-geom/src/pick.rs`, `openchuck-path/src/surface.rs`).
   - So the ignore cannot change a geometry result, generated G-code or a golden file. The build is the same as before: no dependency, version or lockfile changes.
3. **The owner's condition: the ignore holds only while the reach analysis holds.** Re-check it at once, and remove the ignore or bring it back to the owner, if `openchuck-geom`, `openchuck-path` or any other crate starts to use parry's `mesh_intersection`, another parry API that reaches rstar, rstar directly, or heapless directly. A new advisory against heapless or rstar is not covered by this ignore: it fails `cargo audit` and needs its own review.
4. **Removal (OI-2150).** Remove all nine ignores in the pull request that moves `parry3d-f64` to a release on rstar 0.14 or later (heapless 0.9.4 or later). Until then, the monthly pin review (`docs/dependencies.md`) checks for such a release. The `parry3d-f64` row of `docs/dependencies.md` notes the ignore.

## Verification

Run by lane AUDIT-HEAPLESS on 22 Nov 2026 in a Linux cloud container (4 cores), branch `claude/audit-heapless-825` from `main` at `37211de`.

- **Reproduced on `main` `37211de`:** `cargo audit` (cargo-audit 0.23.0, 1,555 advisories loaded) lists the nine advisories against heapless 0.8.0 and exits 0, and `cargo audit --deny warnings` (the CI command) exits 1. `cargo tree -i heapless -e normal` shows the single path above.
- **Versions checked** in the crates.io index on 22 Nov 2026 and on parry's `master` branch: heapless 0.9.4 is the latest; rstar 0.13.x requires `heapless ^0.8` and 0.14.0 requires `^0.9`; `parry3d-f64` 0.31.0 and 0.31.1 require `rstar ^0.13.0`, as does parry `master`.
- **Reach:** the heapless and parry sources read as described in item 2 (rstar 0.13.0's only heapless import, and parry's only rstar use). A search of `crates/` and `apps/` finds no use of `mesh_intersection`, `rstar` or `heapless`.
- **After the change:** `cargo audit --deny warnings` from the repository root exits 0, and exits 1 again with `.cargo/audit.toml` moved away. `cargo deny check` (advisories, bans, licences and sources ok), `cargo xtask deps`, `licences`, `records` and `trace` (105 of 105 P1 requirements have tests), `cargo fmt --all --check` and `cargo clippy --workspace --all-targets` (no warnings) pass. The CI `changes` filter for `rust` includes `.cargo/**`, so the pull request runs the audit step.
- **No output change:** the change touches only `.cargo/audit.toml` and documents. `Cargo.lock`, every `Cargo.toml` and all source are unchanged (`git diff --stat` against `main`), so the compiled program, its G-code and the golden files are the same. As a baseline, `cargo nextest run -p openchuck-geom -p openchuck-path --locked` (uninstrumented, the default profile) passed: 980 tests run, 980 passed, 1 skipped, in 1,346.5 s; the 19 slow tests are existing ones, the longest about 3 minutes, none over 10 minutes. No test was added. The `openchuck-app`, `openchuck-import`, `openchuck-ui` and `openchuck-cli` suites and the gate 2 golden tests were not re-run, because no input to them changed.
- **Not verified:** the advisories' fixes themselves (heapless 0.9.4 is not built here). Nothing ran on a machine. Generated G-code is the user's to verify before running it (NFR-12).
