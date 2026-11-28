# CI-HYGIENE-B: wgpu 30.0.2, uuid 1.29.3 and clap 4.6.10, the patch updates of the Dependabot round of November 2026 (lane CI-HYGIENE)

- **Status:** Proposed by lane CI-HYGIENE on 28 Nov 2026, under [decision 0687](0687-monthly-dependabot-updates-taken-by-a-lane.md) (accepted), which sets how a lane takes Dependabot's updates. It records the verification only; no new choice is made.
- **Number:** provisional (CI-HYGIENE-B), assigned by the coordinator at release. No open item, no schema number, no spike.
- **Date:** 28 Nov 2026.
- **Lane:** CI-HYGIENE, branch `claude/lane-ci-hygiene-deps-a`, from `main` at `f906fa5`.
- **Issues:** #973 (the closed Dependabot pull requests #920 and #921).
- **Requirements:** NFR-14, NFR-10, NFR-06 and GC-07. No requirement change.

## Decision

The three patch updates go in one pull request (decision 0687, items 3 and 4):

- `wgpu` 30.0.1 to 30.0.2 (#920);
- `uuid` 1.29.0 to 1.29.3 and `clap` 4.6.7 to 4.6.10 (#921, which offered 4.6.9; 4.6.10 is the newest patch).

Cargo moved `clap_builder` to 4.6.10, `clap_derive` to 4.6.9 and `syn` 3 to 3.0.10 with them. No crate enters or leaves the tree, and every licence is unchanged (MIT OR Apache-2.0, `uuid` Apache-2.0 OR MIT). The rows in `docs/dependencies.md` are updated.

The resvg (#922) and rfd (#923) updates are 0.x minor releases and have their own pull requests (decision 0687, item 4).

## Verification

Environment: the lane's cloud container, Linux x86-64, 4 cores, Rust 1.97.0, cargo-nextest 0.9.152.

- **Lock files.** `cargo update -p wgpu -p uuid -p clap` in `/`, and `cargo update -p uuid` in `fuzz/`, after the pins were edited. The fuzz workspace holds no `wgpu` or `clap`. Its `syn` 3 stays at 3.0.7, as no pin there moves it.
- **Tests.**
  - `cargo nextest run --locked -p openchuck-project -p openchuck-cli -p xtask -p openchuck-viewport`: 583 passed in 374 s. These include the command-line end-to-end tests, which compare the committed projects' output with the golden programs byte for byte at 1, 2 and N threads (GC-07, NFR-06).
  - `cargo nextest run --locked -p openchuck-ui`: 827 passed, 1 skipped, in 564 s.
  - The viewport render tests found no adapter here, so their assertions were skipped. The UI smoke test's three legs in CI are the evidence for `wgpu`.
- **uuid.** Its serde change does not apply: the `serde` feature is off, and `openchuck-project` writes and parses IDs as hyphenated text.
- **clap.** `openchuck-cli --help` prints as before, and the CLI's parsing tests pass.
- **Checks.** `cargo clippy --workspace --all-targets` gave no warning. `cargo xtask deps`, `licences`, `records` and `trace` are ok.
- **Not verified here.** `cargo deny` and `cargo audit` (Fast checks in CI), the GPU legs on Windows and macOS (UI smoke in CI), and the fuzz targets (the fuzz workspace resolves).
