# PROJUX-B-4: One file for a bug report, written on the user's computer from the Help menu or the command line (NFR-20, NFR-09, NFR-16, NFR-17, NFR-18, NFR-07; issue #754; lane PROJECT-UX-B)

- **Status:** Proposed by lane PROJECT-UX-B on 28 Nov 2026. It is built as the project owner answered on 25 Nov 2026, relayed by the coordinator (decision 0049):
  - Q4a A: a new P2 requirement, NFR-20. The owner agreed its text and its verification row as written here, relayed on 25 Nov 2026.
  - Q4b A: what goes in, and the form, as item 2.
  - The bug-report form gets a note, with the agreed wording.

  The owner has not yet accepted the record itself (OI-9945).
- **Date:** 28 Nov 2026.
- **Requirements:** NFR-20 (new, P2, added by this record), NFR-09 (offline, nothing sent), NFR-16 (build information), NFR-17 (the log), NFR-18 (the action list), NFR-07 (a clear refusal).
- **Amends:** `docs/requirements.md` gains NFR-20 and `docs/verification-plan.md` its row, both in the wording the owner agreed. `.github/ISSUE_TEMPLATE/bug_report.yml` gains the note, with the agreed wording and the placeholder "0.xx.0" until the coordinator gives the version at release.
- **Open items:** adds OI-9945 (the owner's acceptance) and OI-9946 (the owner's check on Windows). Provisional numbers; the coordinator assigns the record number and the OI numbers at release, in merge order.

Numbering: provisional. NFR-20 was the next free requirement number on `main` and on every branch on 24 Nov 2026 (the coordinator confirmed). No project schema number. No library-family change, so no version bump (decision 0048). No new dependency: the zip is written by this code (no compression), and nothing uses a network client (`cargo xtask deps`).

## Context

A bug report needs the diagnostic log (NFR-17), the action list in it (NFR-18), the build information (NFR-16) and often the project. Today the user finds these by hand: About has a button for the log folder and a Copy for the build. Issue #754 asks for one command that gathers them, shows what is included, and sends nothing.

## Decision

1. **Help > Save files for a bug report…** opens a dialog:
   - A line: "One .zip file on this computer. Nothing is sent anywhere; attach it to your report yourself. Issues on GitHub are public."
   - Two ticks that cannot be cleared: Build information, and Log and action history.
   - Three the user chooses: the project's own files (ticked by default), Model files, and Program files last written (both not ticked).
   - The list of exactly what the file will hold, with sizes, which updates as ticks change.
   - **Save…** (the platform's save dialog, "openchuckcam-bug-report.zip") and **Close**. The result is said under the list: "Saved ~/…/openchuckcam-bug-report.zip", or why it was not.
2. **What goes in (Q4b A):**
   - always `build-information.txt` (the About window's Copy text) and the log files (`logs/openchuckcam.log` and the rotated ones);
   - when ticked, `project/…`: the project folder's own files (`project.toml`, setups, tools, operations, selection sets), without `models/` and without the recovery data;
   - when ticked, `models/…`: the project's model files;
   - when ticked, `programs/…`: the files the last export wrote;
   - always `contents.txt` last. It lists every entry with its size, says what is not included, says what could not be read, and says that the home folder reads "~".
3. **The home folder reads "~"** in every entry name and in every text entry (the logs, the build information, the project's files), in either kind of separator. A file that cannot be read is listed in `contents.txt` as "could not be read" and left out; the file is still written (NFR-07).
4. **One zip, written by the program's own code**: entries stored without compression, UTF-8 names, CRC-32, and a fixed entry date so the same inputs give the same bytes. No compression library is needed (Q4b A). It is written to a temporary file beside the chosen name, then renamed, so a failed write never leaves half a file under that name. A file that would not fit a plain zip (over 4 GiB or 65,535 entries) is refused with a sentence.
5. **The command line writes the same file:** `openchuck-cli bug-report --out FILE [--log-folder FOLDER] [--project PROJECT [--models]] [--program FILE]...`. It prints the same list, and says nothing was sent.
6. **The bug-report form** gets the agreed note before "Project, model or other files", and the "Log file" box starts "If you attached the bug-report file, you do not need to paste the log." The form keeps "0.xx.0" until the coordinator gives the version at release. This is a `.github/ISSUE_TEMPLATE` change: no workflow changes, so the checklist of `docs/ci-performance.md` for `.github/workflows/` does not apply.
7. **The guide:** a section "The bug-report file" in `reporting.md` (the anchor the form links to), and the command in the command-line reference.

Not chosen (the owner's Q4b):
- A compressed zip through a new dependency.
- A folder instead of a file: a folder cannot be attached to an issue.

## Verification

Tests written first from NFR-20's verification row, named by requirement. They use generated folders only. Times are uninstrumented.

- `openchuck-app`, `tests/bug_report.rs` (in `tests/integration.rs`), 0.006 to 0.14 s each:
  - `bug_report::req_nfr_20_by_default_the_file_holds_the_build_and_the_logs_and_no_project`;
  - `bug_report::req_nfr_20_the_project_models_and_programs_are_in_only_when_chosen`: no model files and no recovery data in the project's part;
  - `bug_report::req_nfr_20_req_nfr_09_the_home_folder_reads_as_a_tilde_in_names_and_text`, in either separator;
  - `bug_report::req_nfr_20_the_list_shown_before_saving_names_exactly_what_is_written`, names and sizes;
  - `bug_report::req_nfr_20_the_zip_reads_back_and_is_the_same_bytes_for_the_same_inputs`: the test's own reader checks every CRC and size, `unzip -t` passes where installed, and two writes give the same bytes;
  - `bug_report::req_nfr_20_req_nfr_07_a_missing_log_is_said_not_a_crash`: a missing log and project, and a folder that cannot be written;
  - `bug_report::req_nfr_20_no_log_found_is_said_in_the_file`;
  - `bug_report::req_nfr_20_crc32_matches_the_standard_check_value`.
- `openchuck-ui`, `shell/bug_report_tests.rs`, through real frames: `req_nfr_20_help_lists_what_the_file_holds_and_saves_it_with_the_defaults`, `req_nfr_20_ticking_model_files_adds_them_to_the_list_and_the_file`, `req_nfr_20_req_nfr_07_a_file_that_cannot_be_written_says_why`. The shell's dialog checks (`req_nfr_18_every_dialog_has_a_name`, the dialog walk) cover the new dialog.
- `openchuck-cli`, `tests/cli.rs`: `req_nfr_20_cli_writes_the_bug_report_file_with_what_was_asked`.
- `cargo xtask deps`: no network crate.

Results of the full suites, coverage and environment are added before the pull request opens.

Not verified here: the native save dialog, Explorer opening the zip, and attaching it to an issue, which are the owner's check (OI-9946).

## Owner check

What to check, and the result expected:
1. **Help > Save files for a bug report…** lists the build information, the logs, the project's files and `contents.txt`, and no model file, before anything is saved.
2. Ticking **Model files** adds the model to the list.
3. **Save…** writes the zip where chosen, and the dialog says "Saved ~\…".
4. Windows Explorer opens the zip. `contents.txt` lists the same entries, and no text in it shows your user folder: it reads `~`.
5. On GitHub, **New issue > Bug report** shows the note, which links to the guide's section. The zip can be attached to a test issue; delete that issue after.

Steps, on a nightly build of the pull request (Windows):
1. Open a copy of `testdata/examples/baluster/baluster-project`.
2. **Help > Save files for a bug report…** (check 1). Tick **Model files** (check 2).
3. **Save…**, choose the Desktop (check 3), then open the file in Explorer and open `contents.txt` and `logs\openchuckcam.log` (check 4).
4. The form check (check 5) is made on the repository once the pull request has merged, because GitHub shows the form from the default branch.

A pass is all five as expected. A fail is anything else: report the step, what was shown (a screenshot helps) and the version from Help > About.
