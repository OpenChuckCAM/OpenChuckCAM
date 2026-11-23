# CHANGELOG-FLOW-A: A changelog since the last release, checked on each pull request, published with each nightly build and reset by each release (NFR-19, NFR-12, INT-07; lane CHANGELOG-FLOW)

- **Status:** Proposed by lane CHANGELOG-FLOW on 23 Nov 2026. The coordinator approved the plan and answered its five questions the same day (below). The project owner has not accepted the record, and the rule text it adds to `AGENTS.md` and `CLAUDE.md` is quoted in the pull request for the owner's review (OI-9961).
- **Date:** 23 Nov 2026.
- **Requirements:** NFR-19 (the user documentation, with the development version and a version for each release), NFR-12 (generated G-code is the user's to verify; nothing here may suggest otherwise), INT-07 (breaking library changes recorded in a changelog).
- **Builds on:** [decision 0023](0023-nightly-builds.md) (nightly builds), [decision 0091](0091-release-workflow-and-gate.md) (the release workflow and gate), [decision 0400](0400-user-documentation-site.md) section R (the release checks of the user guide), [decision 0052](0052-ci-speed-up.md) (the draft gate: Fast checks run on a draft), [decision 0055](0055-push-discipline-and-draft-pull-requests.md) (one push; drafts), [decision 0516](0516-ci-architecture.md) and [docs/ci-performance.md](../ci-performance.md) (the CI budget), and the owner's rules that release notes carry no decision, open item or requirement numbers and that a lane drafts the release notes (`docs/releasing.md`, 1 Nov 2026).
- **Open items:** adds OI-9961 (the owner's acceptance of this record and its rule text). Issue #833 (the generated release page text) stays a separate follow-up (the coordinator's answer 3).

Numbering: provisional. The coordinator assigns the record number and the OI number at release, above `main`'s highest at that time, in merge order. No project schema number and no library API change.

## Context

The owner asked on 23 Nov 2026: "Can we ensure that all changes are contributing to a changelog that is used as part of the nightly publishing flow so that users know what has changed since the last official release?"

On `main` at `445286b`:

- `.github/workflows/nightly-build.yml` wrote the notes of the rolling `nightly` pre-release inline: the commit, the date, "not a release", the G-code notice and the archive contents. It gave no list of changes.
- `user-docs/src/release-notes.md` has a *Development version* section. The guide's *latest* version, built from the `nightly` tag by `docs-site.yml`, shows it, but the pre-release page that users download from did not.
- Lanes added a line there, and to each crate `CHANGELOG.md`, only because their briefs told them to. Nothing checked it.
- `docs/releasing.md` had the release-notes lane draft the new version's section from the crate changelogs, merged pull requests and closed issues. The release check (`check.py --release`) only checked that a `## <version>` heading exists. Nothing moved or emptied *Development version* at a release, so after 0.55.0 it would have kept growing across releases.
- The crate changelogs are written for developers: lane names, decision numbers, a version section on most pull requests. They cannot be shown to users as they are.

## Decision

<img src="CHANGELOG-FLOW-A-changelog-flow.svg" alt="Flow: a pull request adds a bullet under Development version, or says No user-visible change with a reason; the Fast checks step checks it. The nightly pre-release notes and the latest user guide are made from the section. At a release, start-release moves the bullets into a draft section for the version, the owner edits it, and the release workflow refuses a release while the draft or any bullet remains; the Development version then starts again, empty.">

1. **The changelog is the *Development version* section of `user-docs/src/release-notes.md`.** It holds one top-level bullet for each change users could notice since the last release, in plain words, with no decision, open item or requirement numbers. Issue numbers are allowed because they link to the tracker. The section's introduction and its closing G-code reminder are not entries. Nothing in it may suggest that generated G-code is checked or safe (NFR-12; `check.py` already fails such wording in the guide's source).
2. **One tool, `user-docs/tools/changelog.py`** (Python standard library, like the other guide tools; tests in `user-docs/tools/tests/test_changelog.py`, run by the guide's CI action). It has four commands:
   - `nightly-notes` writes the pre-release notes from the checked-out commit (item 3).
   - `check-pr` is the pull request check (item 4).
   - `start-release <version>` and `check-release <version>` are the release step (item 5).
3. **The nightly pre-release notes say what has changed.** `nightly-build.yml` runs `changelog.py nightly-notes` on the built commit in place of its inline text. The notes are, in order:
   - the build line, "This is not a release", and the program's own G-code notice, read from `notice.rs` as before (NFR-12);
   - *What has changed*: the *Development version* bullets, introduced as the changes since the last release with its version (the highest `## X.Y.Z` heading), and a link to the latest guide's release notes. The guide's relative links are rewritten to `https://openchuck.cam/latest/...`. An empty section gives "Nothing user-visible has changed since the last release, X.Y.Z." Before any release the notes say there has been none;
   - a link to each published library crate's `CHANGELOG.md` at the built commit, for developers. Their entries are not copied, because their wording is internal;
   - the archive contents, as before.

   The notes are at most 60,000 characters (GitHub allows 125,000 in a release body). Past that, whole bullets are left out and one line says how many more are in the guide's release notes. The same text goes to the run summary from the `changes` job, so a manual run on another branch, which publishes nothing, still shows it.
4. **The pull request check.** It is a step of the existing Fast checks job, `Changelog since the last release`, on `pull_request` events only, drafts included (Fast checks run on a draft, decision 0052). It is checked in that job's "every check that was due ran and passed" step. It does not run on a push to `main`, a merge queue run or a scheduled run, which have no pull request description.
   - **Triggered by** a changed file under `apps/*/src/` or `apps/*/assets/`, `crates/*/src/` (not `openchuck-testkit`), `crates/openchuck-post/posts/`, `testdata/examples/` (the examples download) or `user-docs/src/`. Two exceptions: `release-notes.md` itself, and the screenshot records (`*.toml`), which the checks read and users never see.
   - **Not triggered by** anything else: `docs/`, the registers, `AGENTS.md`, `README.md`, workflows, `xtask/`, tests, benches, crate examples, other `testdata/`, a crate's `CHANGELOG.md` or `Cargo.toml` alone, and the guide's tools and theme.
   - **Passes when** the pull request adds or changes a bullet under *Development version*, compared with the merge base. Removing a bullet does not count. When a library-family crate (`openchuck-types`, `-path`, `-post`, `-gcode`, and the built-in posts for `-post`) changes, that crate's `CHANGELOG.md` must also change (INT-07).
   - **Escape hatch (the coordinator's answer 1):** a line of its own in the pull request description, `No user-visible change: <reason>`, with a reason that is not empty. A bullet marker, bold or any case is accepted. It covers both requirements.
   - **Always:** a new or changed bullet that carries a decision, open item or requirement number fails, whether or not the escape hatch is present. A bullet the pull request did not touch is not its problem.
   - **The failure message** names the files that triggered the check, what is missing, how to fix each part, and the escape hatch.
   - **When the description is read.** It comes from the event, so the check reads it when the pull request is opened, on each push, and when it is marked ready for review. The Merge Manager marking a pull request ready is the point at which a corrected description is read again, with no push (the coordinator's addition c). The `edited` event is not added, because it would run the whole workflow again for each edit of the description.
   - The diff is taken from the merge base of the pull request's base commit and its head, so changes that reached `main` after the branch was made are not counted.
5. **The release resets the changelog.**
   - The release-notes pull request starts with `changelog.py start-release <version>`. It moves the bullets into a new `## <version>` section above the previous release, under *What has changed*, ending with the same G-code reminder and starting with a draft comment that says what else to write. *Development version* keeps only its introduction and reminder.
   - `start-release` refuses a version that is not newer than the last release, and a page without *Development version*.
   - The release workflow's guide check (`check.py --release`, through `changelog.py check-release`) now also refuses a release whose section still has the draft comment, or whose *Development version* still has bullets. The existing check that the version has a section is unchanged and keeps passing.
   - A dry run before the release-notes pull request merges reports these failures, as it already reports the missing section.
   - Nothing here tags, publishes or touches crates.io.
6. **Documents.**
   - `AGENTS.md` gets the rule, and `CLAUDE.md` gets the step before a pull request and the local command.
   - `docs/releasing.md` gets the flow and the stronger release checklist item.
   - The guide's *Install and first run* says where to find what changed, and the release notes' introduction says what the section means.
   - On the coordinator's answer 5, the stale "no stable release yet" sentences in `install.md` and `README.md`, "Signing is not part of the first release" and "before the first release" in `limitations.md` now read as true after the 0.55.0 alpha.
   - This pull request adds its own *Development version* bullet.

### The coordinator's answers (23 Nov 2026)

1. The escape hatch is the description line `No user-visible change: <reason>`, with a reason that is not empty.
2. The check runs on drafts too, in Fast checks.
3. #833 stays a separate follow-up. `changelog.py` is written so a later `release-page` command can use the same parser for the release page's "Before you use it" text.
4. No manual nightly-build run (runner minutes). The local script run, the unit tests and a dry run of the notes against the real release notes and an empty section are enough.
5. Fix the stale "no stable release yet" wording in this pull request.

The coordinator's additions:

- (a) the failure message names the escape hatch;
- (b) a table of example diffs shows that docs-only and changelog-only pull requests are not caught;
- (c) mark-ready is the re-read point, and it is documented;
- (d) `start-release` is tested on a fixture;
- (e) the in-flight lane pull requests are checked (Verification).

## Alternatives considered

- **Copy the crate changelogs' *Unreleased* entries into the nightly notes.** Not chosen. They carry lane names and decision numbers, which the owner's rule keeps out of user text, and lanes write version sections, not *Unreleased*. The notes link them instead.
- **A file per change (fragments, as towncrier does), merged at release.** No merge conflicts between lanes. Not chosen: the registers already merge as unions (decision 0130), one Markdown section is what the guide shows, and fragments would need a build step before the guide.
- **Generate the changelog from merged pull request titles.** Not chosen. The titles are internal (lane names, decision numbers), and it would need the GitHub API at build time.
- **An xtask (Rust) check.** Not chosen. The parser is needed in the nightly publish job, which has no Rust toolchain, and the guide's tools are already Python with tests. One parser serves the check, the notes and the release.
- **A commit trailer as the escape hatch.** Not chosen (answer 1). It is harder to see in review and cannot be corrected without a push.
- **Add the `edited` event to re-run the check on a description edit.** Not chosen. It runs the whole CI workflow again. Mark-ready already reads the description.
- **A separate CI job.** Not chosen. A step in Fast checks costs about a second and no job slot (decision 0516 budget).

## Consequences

- Every pull request that changes what users see now needs a *Development version* bullet or a reason. Lanes in flight that touch product code without one fail Fast checks until they add one or the description line (Verification lists the seven in flight).
- The nightly pre-release page shows the changes since the last release. The guide's *latest* release notes show the same section.
- The next application release's notes pull request must run `start-release` (or move the bullets by hand) and finish the section, or the release is refused. This is intended.
- Library releases (`lib-v` tags) are unchanged. Their `CHANGELOG.md` entries are now checked on each pull request that changes a family crate.
- Changed files: `user-docs/tools/changelog.py` (new), `user-docs/tools/check.py`, `user-docs/tools/tests/test_changelog.py` and `fixtures/` (new), `user-docs/tools/tests/test_check.py`, `.github/workflows/ci.yml` (one step), `.github/workflows/nightly-build.yml`, `AGENTS.md`, `CLAUDE.md`, `README.md`, `docs/releasing.md`, `user-docs/src/release-notes.md`, `user-docs/src/start/install.md`, `user-docs/src/start/limitations.md`, the decision index and `docs/open-items.md`.

### CI performance (docs/ci-performance.md checklist)

- **Runner minutes and critical path.**
  - Pull request: one step in Fast checks, which already runs on every pull request. The script runs in about 0.05 s locally (Python start-up included). It adds no job and no minutes to speak of, and nothing to the critical path, because Fast checks finish long before the build.
  - Nightly build: one checkout (about 5 s) and the script in the `changes` job when a build is due, and the script in place of the inline text in `publish`. Under 0.2 runner-minutes a night.
- **New heavy job:** none. The step is checked in Fast checks' own "every check that was due ran and passed" step, with due = `pull_request` event, so a skipped step on a pull request fails the job.
- **Called workflows:** none changed (`build-archives.yml` is untouched).
- **`needs` on a matrix:** unchanged.
- **`tools/ci/main-run-mode.test.sh`:** passes. The light, full, pull request and scheduled cases are unchanged, because the step's condition is the event and the run-mode logic is not touched.
- **Permissions:** unchanged (Fast checks `contents: read`; the nightly `changes` job `actions: read`, `contents: read`).
- **Windows first, no check removed:** unchanged.
- **Run once:** the draft pull request's Fast checks run the step on this pull request itself. No manual nightly-build run (answer 4). The notes were generated locally on the real tree (Verification).

## Verification

Environment: the lane's cloud container, Python 3.13, at `main` `445286b` plus this change.

- **Tests first.** `user-docs/tools/tests/test_changelog.py` was written before `changelog.py`. With the module absent, the run failed (import error). After the module was written, two tests failed: a regex in the bound test, and the `start-release` fixture, which did not exist yet. Both were fixed. The fixture's expected output (`fixtures/release-notes-after-0.60.0.md`) was written by hand from the rule in item 5, not copied from the program's output.
  - `test_check.py` gained `test_changelog_flow_release_needs_an_empty_development_version_in_the_source`, which failed before `check.source_release_problems` existed.
- **Result:** `python3 -m unittest discover -s user-docs/tools/tests`: 110 tests, OK. Of these, 36 are in `test_changelog.py`, among them `test_changelog_flow_the_table_of_example_diffs`, with 27 example diffs.
- **The table of example diffs** (the coordinator's addition b). These are not triggered:
  - `docs/decisions/...` with `docs/open-items.md`; `AGENTS.md`, `CLAUDE.md`, `README.md`; `docs/releasing.md` with `.github/workflows/ci.yml`;
  - a crate `CHANGELOG.md` alone, or with `Cargo.toml` and `Cargo.lock`;
  - crate tests, benches and examples; the test kit; `xtask`; `testdata/golden`;
  - the release notes alone; the screenshot records; the guide's tools and theme.

  These fail without an entry or the escape hatch:
  - application source and assets, and an application crate;
  - a library-family crate (and passes only with both its entry and its `CHANGELOG.md`);
  - the built-in posts (same as a library crate);
  - the example projects; a guide page; a new screenshot.
- **Nightly notes on the real tree.** `python3 user-docs/tools/changelog.py nightly-notes` on this branch lists the two *Development version* bullets under "Changes since the last release, 0.55.0". The keyboard bullet's link became `https://openchuck.cam/latest/reference/keyboard.html`.
- **An empty section.** On a copy of the tree after `start-release 0.60.0`, the notes said "Nothing user-visible has changed since the last release, 0.60.0." and `check-release 0.60.0` failed on the draft comment, as intended. `check-release 0.55.0` on the real page fails because *Development version* has bullets, which is what a re-run of that release would now report.
- **Draft PR (the coordinator's addition e).** These in-flight lane pull requests were checked with `check-pr` against their own base and description, from their heads on 23 Nov 2026:

  | Pull request | Lane | Result today |
  | --- | --- | --- |
  | #845 | STEPPED-CYL | passes (adds a bullet; no triggering file) |
  | #853 | FUZZ-NFR07 | passes (a bullet; the `openchuck-types` and `openchuck-post` changelogs) |
  | #854 | FACEFLOW | passes (three bullets; the `openchuck-path` changelog) |
  | #856 | PENCIL-REST phase 1 | passes (documents only) |
  | #857 | POST-COMMENTS | passes (a bullet; the `openchuck-post` changelog) |
  | #865 | DOCS-SPOT-1 | passes (only screenshot records under `user-docs/src/`). It failed before the `*.toml` exception, and that failure led to the exception. |
  | #866 | SWARF-LEADS | passes (a bullet; the `openchuck-path` changelog) |

- **Timing:** `check-pr` on #854's diff took 0.055 s and `nightly-notes` 0.046 s, wall clock.
- **Not verified:**
  - the workflow steps on GitHub, which the draft pull request's Fast checks run for `check-pr`. The nightly step runs only at the next nightly build after merge, because a manual run is not wanted (answer 4);
  - how GitHub renders `<kbd>` in a release body (it renders on github.com, but was not seen on this page);
  - the release check on a real release.
- **Open items:** OI-9961.
