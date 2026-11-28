# CI-HYGIENE-C: resvg 0.50.0 for the application icon's renderer, output byte-identical (lane CI-HYGIENE)

- **Status:** Proposed by lane CI-HYGIENE on 28 Nov 2026, under [decision 0687](0687-monthly-dependabot-updates-taken-by-a-lane.md) (accepted). It records the verification only; no new choice is made.
- **Number:** provisional (CI-HYGIENE-C), assigned by the coordinator at release. No open item, no schema number, no spike.
- **Date:** 28 Nov 2026.
- **Lane:** CI-HYGIENE, branch `claude/lane-ci-hygiene-deps-b`, from `main` at `f906fa5`.
- **Issues:** #973 (the closed Dependabot pull request #922).
- **Requirements:** NFR-14 and NFR-10. No requirement change.

## Decision

`resvg` moves from 0.49.0 to 0.50.0, and `usvg` moves with it. It is a 0.x minor release, so it has its own pull request (decision 0687, item 4).

- **Where it is used.** Only `xtask` uses it: `cargo xtask icon` renders the application icon ([decision 0542](0542-application-icon.md)). Issue #973 said the user guide's figures; that was a slip, as the guide's tools do not use it.
- **What changed upstream:**
  - bounding-box methods now return `Option`;
  - `currentColor` and pattern drift are fixed;
  - recursive references are refused;
  - the MSRV rises to 1.85, under the pinned Rust 1.97.
- **Effect here.** `xtask` uses none of the changed calls.
- **Dependencies.** No crate enters or leaves the tree, and the licence is unchanged (Apache-2.0 OR MIT). The register row in `docs/dependencies.md` is updated.

## Verification

Environment: the lane's cloud container, Linux x86-64, Rust 1.97.0.

- **Lock files.** `cargo update -p resvg` in `/`. The fuzz workspace does not hold `resvg`.
- **Icon.**
  - `cargo xtask icon --check` is ok: the committed PNG sizes match the SVG pixel for pixel.
  - `cargo xtask icon` wrote every icon file and the favicon again, and `git status` showed no change. The PNG, `.ico` and `.res` files are byte-identical to the committed ones.
- **Tests.** `cargo nextest run --locked -p xtask`: 240 passed, including the icon tests (deterministic render, size, invalid SVG an error, the `.ico` directory).
- **Checks.** `cargo clippy -p xtask --all-targets` gave no warning. `cargo xtask deps`, `licences`, `records` and `trace` are ok.
- **Not verified here.** `cargo deny` and `cargo audit` (Fast checks in CI).
