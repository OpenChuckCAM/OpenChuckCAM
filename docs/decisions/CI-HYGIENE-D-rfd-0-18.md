# CI-HYGIENE-D: rfd 0.18.0 for the file and message dialogs (lane CI-HYGIENE)

- **Status:** Proposed by lane CI-HYGIENE on 28 Nov 2026, under [decision 0687](0687-monthly-dependabot-updates-taken-by-a-lane.md) (accepted). It records the verification only; no new choice is made.
- **Number:** provisional (CI-HYGIENE-D), assigned by the coordinator at release. No schema number, no spike.
- **Date:** 28 Nov 2026.
- **Lane:** CI-HYGIENE, branch `claude/lane-ci-hygiene-deps-c`, from `main` at `f906fa5`.
- **Issues:** #973 (the closed Dependabot pull request #923).
- **Requirements:** NFR-14, NFR-10 and NFR-09 (no network client). No requirement change.

## Decision

`rfd` moves from 0.17.2 to 0.18.0. It is a 0.x minor release, so it has its own pull request (decision 0687, item 4). The features stay the defaults, `xdg-portal` and `wayland`, with GTK off.

What changed upstream, and its effect here:

- **MSRV 1.92.** Under the pinned Rust 1.97.
- **A GTK4 backend.** It is behind a feature that stays off.
- **Match-all filters.** A match-all filter now matches names without a dot. No dialog here uses one: the model, machine and tool dialogs each list their extensions.
- **GTK3 message text.** It is no longer a format string. GTK3 is off.

Crates:

- `pollster` moves from 0.4.0 to 1.0.1, which was already in the tree.
- On macOS it adds `objc2-uniform-type-identifiers` 0.3.2 (Zlib OR Apache-2.0 OR MIT, all in `deny.toml`). It comes from the `objc2` project, which the tree already uses, and has no build script.
- No network client is added (NFR-09).

The register row in `docs/dependencies.md` is updated.

## Verification

Environment: the lane's cloud container, Linux x86-64, 4 cores, Rust 1.97.0, cargo-nextest 0.9.152.

- **Lock files.** `cargo update -p rfd` in `/`. The fuzz workspace does not hold `rfd`.
- **Tests.** `cargo nextest run --locked -p openchuck-ui`: 827 passed, 1 skipped, in 583 s.
- **Checks.** `cargo clippy --workspace --all-targets` gave no warning. `cargo xtask deps`, `licences`, `records` and `trace` are ok.
- **Not verified here.** The dialogs themselves. The container has no desktop or portal service, and no test opens a native dialog. The parts to check by hand on Linux (portal), Windows and macOS are:
  - open, save and import;
  - the extension filters;
  - the error and warning message boxes.

  Also not verified here: `cargo deny` and `cargo audit` (Fast checks in CI), and the macOS build (CI).
