# 0710: Generating again with nothing changed reuses the posted files (GC-07, NFR-04, NFR-06; issue #698; lane SIM-UX)

- **Status:** Accepted by the project owner on 25 Nov 2026, relayed by the coordinator (decision 0049), on the lane's confirmation that nothing dated is kept with the posted files. Proposed by lane SIM-UX on 24 Nov 2026; the coordinator released the pull request on the owner's answers of 24 Nov 2026 as one that closes on its tests (no owner check).
- **Date:** 24 Nov 2026.
- **Requirements:** GC-07 (the same project, post and version give byte-identical G-code), NFR-04 (never block, use the cores well), NFR-06.
- **Amends:** nothing. It builds OI-2022, which [decision 0510](0510-interface-answers-kept-per-project-change.md) left open. Posting speed itself (OI-1993) is the lane's next pull request for issue #698.
- **Open items:** closes OI-2022; adds OI-2281 (the owner's acceptance of this record), closed by the acceptance of 25 Nov 2026.

Numbering: decision 0710 and OI-2281, assigned by the coordinator at release (drafted as "SIM-UX-C" and OI-9953). No project schema number and no library-family change.

## Context

A run with a post (the export dialog's preview and its write, the G-code panel beside playback, `openchuck-cli regenerate --post`) reuses every cached operation, then posts the whole program and checks it again. Measured on `main` `6b039e5` (release build, 4-vCPU cloud container, uninstrumented), generating again with nothing changed:

| Project | Without a post | With the Makera Z1 post |
| --- | --- | --- |
| Baluster tutorial project (29,098 moves, 0.81 MB of G-code) | 0.07 s | 0.43 to 0.44 s |
| Carving test piece (628,329 moves, 13.8 MB) | 0.9 s | 8.7 to 9.3 s |

## Decision

1. **The posted files are kept with everything posting reads.** After a run posts, the pipeline cache keeps its posted files (the G-code, the checks' reports, the notes and warnings, the posted toolpath, the times per file and the short-block settings) with a key made of:
   - every operation of the run, by the key of its cached result, which fixes its toolpath and the stock it was made on;
   - the project, which holds every name, number and setting the program writes;
   - the post as loaded, so a post folder is read again each run and an edit to its files is seen;
   - the grouping;
   - the cache's generation.
2. **They are reused only when everything is equal.** A later run whose key equals the kept one takes the kept files, unchanged, so the bytes are the ones posting again would give (GC-07). Any difference posts again and replaces the kept files. With caching off, or an operation without a key, nothing is kept or reused.
3. **One set is kept.** It is dropped when the cache is cleared, as when a strategy is registered.

## Alternatives considered

- **Key by a hash of the toolpath.** Rejected: hashing hundreds of thousands of moves costs a share of what is saved, and the operations' keys already fix the toolpath.
- **Keep the posted files of several posts or groupings.** Rejected for now: moving between two posts in the export dialog would post once per change either way, and each set costs memory (on the carving, 13.8 MB of G-code plus the posted toolpath).

## Consequences

- `openchuck-app`: `PipelineCache` keeps the last posted files (`PostedKey`), `posting::post_toolpath_with` takes a loaded post, and `PostedFiles` is `Clone`. `Application::posts_made` (hidden) counts the posts made, for tests. Not a library-family crate, so no version bump; its changelog says so.
- Memory: one set of posted files while the application runs.

## Verification

Linux cloud container, 4 vCPU, no GPU; branch `claude/lane-sim-ux-c`, rebased on `main` `361613e`.

**Test, written first from GC-07 and NFR-04 and watched failing.** With the reuse turned off it failed at "the posted files were reused" (2 posts, not 1); with it on it passes.
- `posting::req_gc_07_req_nfr_04_posting_again_with_nothing_changed_reuses_the_posted_files` (`openchuck-app` integration; 1.1 s). It checks:
  - a second run with nothing changed gives the same files, reports, notes, warnings, posted toolpath and times without posting;
  - another grouping posts again;
  - renaming an operation, a name the program writes but not part of the operation's key, posts again, and the program has the new name;
  - another post posts again;
  - a post folder whose template is edited on disk posts again, with the edit in the output;
  - the bytes reused equal a fresh application's (GC-07).
- `posting::req_gc_07_req_gc_12_a_reused_post_takes_the_new_time_for_the_sheet_and_names` (0.4 s), added on the owner's question (25 Nov 2026): do reused files carry a stale date? Nothing kept is dated. The G-code holds no clock time (GC-07). The setup sheet's generation time and a `{date}` in the file-name templates are read from the clock when the sheet and the names are made, after the run. With a fixed clock moved on two days, a run that reuses the posted files gives the same G-code bytes, a sheet with the new time, and file names with the new date. The test passes on this change unmodified; it pins the behaviour.

**Suites.** `cargo nextest run -p openchuck-app -p openchuck-ui -p openchuck-cli`: 1,750 tests, all passed. They include the golden-program, export-dialog, posting and determinism tests. Also `cargo fmt --all --check`, `cargo clippy --workspace --all-targets` with no warnings, and `cargo xtask deps`, `licences`, `records`, `trace` and `changelog check-pr`.

**Timings, release build, uninstrumented: generating again with the Makera Z1 post.**

| Project | `main` `6b039e5`, nothing changed | This branch, first post | This branch, nothing changed | Without a post |
| --- | --- | --- | --- | --- |
| Baluster tutorial project (0.81 MB) | 0.43 to 0.44 s | 0.41 s | 0.06 to 0.07 s | 0.07 s |
| Carving test piece (13.8 MB) | 8.7 to 9.3 s | 8.28 s | 0.84 to 0.87 s | 0.9 s |

With nothing changed, generating with a post now costs what generating without one costs. Posting itself (the first post) is unchanged here; that is OI-1993, the lane's next pull request for #698.

**Coverage.** `cargo llvm-cov nextest --profile coverage -p openchuck-app` on the new test and the posting gate test, then `cargo llvm-cov clean --workspace`; the uncovered new lines are listed in the pull request.

**Not verified:**
- The owner's projects (none used).
- Posting with caching turned off (`set_cache_versions(0)`). No key is made, so nothing is kept or reused. The existing tests with caching off pass, but none of them posts, so this path was checked by reading the code only.
