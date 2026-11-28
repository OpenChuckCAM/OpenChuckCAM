# CI-HYGIENE-A: the Microsoft apt source is dropped before `apt-get update`, Gate 3's replay step has its own limit, test waits in `openchuck-ui` scale with coverage, and `openchuck-buildinfo` is gated (lane CI-HYGIENE)

- **Status:** Proposed by lane CI-HYGIENE on 28 Nov 2026, built to the plan the coordinator accepted on 28 Nov 2026. No owner decision was needed: no check leaves a run, no test is moved off a profile, and no new job is added.
- **Number:** provisional (CI-HYGIENE-A); the coordinator assigns the decision number at release. No open item, no project schema number, no spike.
- **Date:** 28 Nov 2026.
- **Lane:** CI-HYGIENE, branch `claude/lane-ci-hygiene`, from `main` at `f906fa5`.
- **Issues:** #983, #974, #982, and the nightly-coverage issue #170.
- **Requirements:** none changed. The changes keep CI's existing checks working (verification plan section 8; decisions 0102 and 0516). `docs/requirements.md` and `docs/architecture.md` are not edited.
- **Amends:** `docs/ci-performance.md`. It adds a line to *Keep each test short* (the test waits) and one to the workflow checklist (the apt source). Edited in this pull request.

## Context

- **#983.** The UI smoke test's Ubuntu leg failed twice in `sudo apt-get update` (jobs 140386776501 and 140727529302).
  - The runner image lists `https://packages.microsoft.com/ubuntu/24.04/prod`. That source answered `403 Forbidden`, and apt then reported "no longer signed" and exited with 100.
  - The job installs nothing from it: Mesa, Xvfb and the X11 libraries come from Ubuntu's archive.
- **#974.** Gate 3 share 5 (job 139937012458, #952's run) was cancelled 55 minutes after it started, against the job's `timeout-minutes: 45`.
  - The Actions API still shows the replay step as `in_progress` and the upload and post steps as `pending`, and the job's log returns 404. So the runner stopped reporting, and GitHub reaped the job after the job limit and its grace period.
  - The code was not the cause. `cargo xtask replay` already stops a program after 900 s (`wait_with_timeout`) and removes its container.
  - Its sibling shares took 13 to 20 minutes.
- **#982.** About 80 waits in `openchuck-ui`'s tests had fixed wall-clock limits (10 to 600 s). Instrumentation for coverage slows the hot loops 6 to 11 times, so two playback tests crossed their 120 s limit in a coverage run with no fault in the code.
- **#170.** The nightly Coverage run failed on 26 and 27 Nov 2026 (runs 45826893444 and 46004791142) on a configuration problem: `openchuck-buildinfo` had coverage data but was not listed in `xtask/coverage.toml`.

## Decision

1. **The Microsoft apt source is removed before `apt-get update`.** This applies to every step that runs apt on a hosted Ubuntu runner:
   - `ci.yml` (UI smoke test);
   - `ui-smoke.yml` (launch the archive);
   - `nightly.yml` (the sanitizer's symbolizer).

   Each step deletes the files in `/etc/apt/sources.list.d` that name `packages.microsoft.com` and prints what it removed. The floor container's apt calls are unchanged; the container has no such source and the step finds nothing.
2. **The Gate 3 replay step has `timeout-minutes: 30`.** A stuck step fails at 30 minutes instead of running to the job's 45, and the reports are still uploaded (`if: always()`).
   - This cannot help a runner that stops reporting, which was the case in #974. Only GitHub can end that job, at the job's limit.
   - A program that hits xtask's 900 s limit and is run once more can reach the step limit; it was failing either way.
3. **Test waits use `crate::testing::wait_limit(seconds)`.**
   - The helper returns the limit as written, and ten times it when the build has `cfg(coverage)`, which cargo-llvm-cov sets (its `--no-cfg-coverage` turns it off; neither CI nor xtask uses that flag).
   - Every `Instant::now() + Duration::from_secs(n)` and `elapsed() < Duration::from_secs(n)` in `openchuck-ui`'s tests now calls it: 84 waits in 19 files.
   - Durations that are not wait limits are unchanged: product limits such as the graphics stall limit and the bench scene limit, and the simulated `tick` durations of the checkpoint tests.
   - `cfg(coverage)` is declared in the workspace's `unexpected_cfgs` check-cfg list. No test changes what it checks, and none moves between nextest profiles.
4. **`openchuck-buildinfo` is gated.** It is listed in `gated` in `xtask/coverage.toml`, since its own tests alone reach 97.1% of lines, above the 95% target.

## Alternatives considered

- **Tolerating the failure (`apt-get update || true`).** Rejected: it would also hide a real failure of Ubuntu's own archive, and the install would then fail later with a less clear message.
- **A lower job limit for Gate 3 instead of a step limit.** Rejected: the job limit is what reaps a dead runner, and lowering it saves nothing in that case. The step limit catches a live hang.
- **Taking the two playback tests out of the coverage profile.** Rejected: they are short uninstrumented, and AGENTS.md keeps the long list for tests over about a minute uninstrumented.
- **A limit counted in work done instead of seconds.** Not taken: a hang does no work, so it would need a wall clock anyway.
- **`openchuck-buildinfo` as report-only.** Not taken: it reaches the target.

## Consequences

- No new job and no change to the critical path. The apt step costs under a second.
- A step-level hang in Gate 3 now costs at most 30 runner minutes instead of 45.
- A new test wait in `openchuck-ui` calls `wait_limit` (ci-performance.md).

## Verification

Environment: the lane's cloud container, Linux x86-64, 4 cores, Rust 1.97.0, cargo-nextest 0.9.152 and cargo-llvm-cov 0.9.2, branch `claude/lane-ci-hygiene` from `main` at `f906fa5`.

- **The apt step.**
  - Run in a shell with `bash -e` against a copy of a sources folder holding `microsoft-prod.list` and `ubuntu.sources`: the Microsoft file was removed and named, and the other was kept.
  - Run against a folder with no match (the floor container's case): nothing was removed and the step went on.
  - The hosted-runner check is the manual `ci.yml` run on the branch (pull request).
- **The step limit.** A YAML key on the replay step only; the replay itself is unchanged. It is exercised by the same manual run.
- **Manual `ci.yml` run on the branch** (run 46135228816, `workflow_dispatch` at `930319a`, which runs the full layout, macOS test shares included): all 40 jobs passed, and `CI required checks` passed.
  - **UI smoke.** Ubuntu 2.0 min, Windows 3.0 min, macOS 3.2 min. The Ubuntu log shows `Removing apt sources: /etc/apt/sources.list.d/microsoft-prod.list`, followed by an `apt-get update` with no error (#983).
  - **Gate 3.** The five shares took 17.0 to 19.2 min, and their replay steps 16.5 to 18.5 min, under the 30-minute step limit (#974).
  - **Test shares.** Ubuntu 9.0 to 20.4 min, Windows 9.2 to 14.6 min, macOS 34.2 and 42.6 min.
  - **Whole run.** The wall time was 122.7 min, mostly queueing for the project's shared runners while other runs were going. The last job was macOS Tests share 2 of 2, which a pull request run does not have.
  - **Runner minutes.** 529.8, the sum of the job durations.
  - **Not measured.** Peak jobs at once and queue time.
- **`wait_limit`, uninstrumented.**
  - `cargo nextest run -p openchuck-ui --locked`: 828 tests passed, 1 skipped, in 565.7 s.
  - The longest test was `shell::simulate_tests::req_ui_16_req_sim_04_the_playback_dock_lays_out_whole_in_every_state` at 70.7 s. It is already listed in both profiles of `.config/nextest.toml`.
  - `testing::wait_limit_is_longer_only_in_a_coverage_build` passed, with the limit as written.
- **`wait_limit`, instrumented.**
  - `cargo llvm-cov nextest --profile coverage -p openchuck-ui --locked --no-report -E 'test(/^playback_panel::tests::/) | test(=testing::wait_limit_is_longer_only_in_a_coverage_build)'`: 38 tests passed, in 1042.8 s.
  - The helper's test passed with the tenfold limit, which shows that cargo-llvm-cov sets `cfg(coverage)`.
  - `playback_panel::tests::req_ui_11_a_stale_toolpath_keeps_the_part_colours` took 182.8 s, and `req_ui_11_a_part_file_edited_after_the_colours_is_drawn_plain_once_the_scene_is_read_again` took 135.1 s. Both are past the old 120 s limit; these are the two tests of #982.
  - The three `final_cut_part` tests took 452 to 467 s, under their tenfold limit.
- **`openchuck-buildinfo`.**
  - `cargo llvm-cov -p openchuck-buildinfo --locked --summary-only --ignore-filename-regex '/(tests|examples|benches)/'`: 9 tests passed, 97.10% of lines (310 lines, 9 missed).
  - `cargo test -p xtask coverage`: 10 tests passed.
- **Checks.**
  - `cargo fmt --all` and `cargo clippy -p openchuck-ui --all-targets`: no warning.
  - `cargo xtask deps`, `licences`, `records` and `trace`: all ok.
  - `bash tools/ci/main-run-mode.test.sh`: ok. The job layout is unchanged.
- **Not verified here.**
  - The nightly Coverage run (the first night after merge shows whether the issue closes).
  - The `wait_limit` waits in the other `openchuck-ui` modules under instrumentation (the nightly run covers them).
  - Line coverage: the change is in test code apart from the helper, which its own test runs.
