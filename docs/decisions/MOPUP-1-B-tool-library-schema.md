# MOPUP-1-B: Tool-library files have their own schema number, and a file a build cannot read says it needs a newer version (TL-01, PRJ-04, NFR-07; issue #864; lane MOPUP-1)

- **Status:** Proposed by lane MOPUP-1 on 26 Nov 2026; not yet accepted (OI-9921). The project owner chose option A on issue #864 on 26 Nov 2026, relayed by the coordinator (decision 0049): library files get their own schema number, raised when a field is added; builds from then on refuse a newer library file cleanly and name a file that fails only on an unknown field as written by a newer version; nothing is ever dropped. The owner confirmed A again after the corrected facts below (26 Nov 2026, relayed). The owner has not read this record's text.
- **Date:** 26 Nov 2026.
- **Requirements:** TL-01 (the tool library), PRJ-04 (files written by another version), NFR-07 (a clear error, never a crash).
- **Builds on:** [decision 0014](0014-slice-4-tool-library.md) (the library folder, one file per tool), [decision 0033](0033-slice-14-rotary-roughing.md) (`centre_cutting` added without a schema step), [decision 0673](0673-check-the-bottom-of-the-z-travel-for-each-tool.md) (`stick_out`, also without one), [decision 0008](0008-slice-1-project-model.md) item 12 (the project schema's rule).
- **Open items:** adds OI-9921 (the owner's acceptance of this record).

Numbering: provisional; the coordinator assigns the record and OI numbers at release. Tool-library schema 2 is claimed in the work breakdown's schema table, confirmed by the coordinator on 26 Nov 2026; the project schema is unchanged (29). `openchuck-project` changes; no library-family crate changes, so there is no library version bump.

## Context

Issue #864, from CodeRabbit's note on #852. The facts, checked in the code at the release tags `app-v0.55.0` and `app-v0.55.1` (the only releases; "0.58" in the issue is a nightly build):

- The library is one file per tool. `ToolLibrary::open` reads each on its own: a file that fails is listed under the library's problems and the other tools open. That is so in every version, and it is kept.
- The file reader checks a file's `schema` number before it reads the fields. A released 0.55 build therefore refuses a tool file that declares a schema above 1 with "uses project schema 2, but this version reads up to 1; open it with a newer OpenChuckCAM". The wording says "project", which is wrong for a tool file; a released build cannot be changed.
- `stick_out` was added to `Tool` by #852 (merged 23 Nov 2026, 0.59.0), but `LIBRARY_SCHEMA` stayed 1. So the nightly builds since then write `stick_out` into tool files that declare schema 1. A 0.55 build sees schema 1, reads the fields, and fails on the unknown field: the user sees "unknown field `stick_out`" for each such tool, not "needs a newer version".
- `centre_cutting` (decision 0033) is read by every release, so it does not need a step.
- No other field of a tool file, a tool's profile or its cutting data is newer than 0.55 (`git diff app-v0.55.0` over the tool types and the library).

## Decision

1. **The tool library has its own schema number** (the owner's answer A). `tools::LIBRARY_SCHEMA` is the newest this build reads, now 2:
   - 1: slice 4, with `centre_cutting`.
   - 2: a tool's `stick_out`.

   A later field that an older build cannot read raises it again, with a row in the work breakdown's schema table, as for the project schema.
2. **A file declares the lowest schema that holds it.** A tool file declares 2 only when its tool has a `stick_out`, and 1 otherwise; a tool set file declares 1. A released build keeps reading every tool it can, and refuses only those it cannot, by their number, with its newer-version message. This is the project schema's rule: an older file is refused only once it holds the newer field.
3. **A newer schema reads as a tool library file.** This build refuses a tool or tool set file of a schema above 2 with "'…' uses tool library schema 3, but this version reads up to 2; open the tool library with a newer OpenChuckCAM" (`ProjectError::LibrarySchemaTooNew`).
4. **An unknown field reads as written by a newer version.** A file of a schema this build reads that holds a field it does not know is refused with "'…' holds `field`, which this version does not read: the tool library file was written by a newer version of OpenChuckCAM, or the field is misspelt; open the tool library with a newer OpenChuckCAM" (`ProjectError::WrittenByNewer`). The field is named, so a typo can still be found.
5. **Nothing is dropped and nothing is rewritten on reading.** Schema 1 files read as before, with a `stick_out` kept if a nightly build wrote one. A file is written only when it is saved, and then it declares the schema of item 2.
6. **Released builds.** A 0.55 build refuses a tool saved by this version with a stick-out cleanly ("open it with a newer OpenChuckCAM"), and opens the other tools. A tool saved by a nightly build since 23 Nov 2026 (schema 1 with a stick-out) still shows 0.55 the "unknown field `stick_out`" error until a build with this change saves it again. The user guide says so.

## Consequences

- `openchuck-project` gains two error variants (`ProjectError` is `non_exhaustive`) and writes tool files with schema 1 or 2. The application, the interface and the command line show the messages through the library's problems as before.
- The user guide's tool library page says how a library shared with an older version behaves. The release notes say the same.
- The next field added to tool files raises the library schema to 3 under item 2. TOOL-LOAD's claimed TL-1b (issue #844), which adds `largest_cut_section` to cutting data in tool library files, is the first expected: a tool file declares 3 only when it holds that field.
- The project file's tool snapshots are unchanged; they follow the project schema (29 for `stick_out`).
- A rollback after a save (CodeRabbit on #964): once a build with this change saves a tool with a stick-out, it declares schema 2. A nightly build from 23 to 27 Nov 2026 (0.59 to 0.73) reads `stick_out` but accepts only library schema 1, so it skips that tool with its newer-version message; the other tools open, and the file is not changed. This affects nightly builds only, loses no data, and is inherent in the owner's option A (the same trade as a raised project schema).

## Verification

Environment: Linux container, Rust stable, branch `claude/lane-mopup-1-toollib`, built on `main` at `b2f1550` and rebased on `10a9e6e` (after #947) before the push.

Tests were written from the issue and the owner's answer before the code, and failed on `main`. Times are uninstrumented, from `cargo nextest`; each is under 0.1 s.

- `openchuck-project`, `tests/tools.rs`:
  - `req_tl_01_req_prj_04_a_tool_file_declares_the_lowest_library_schema_that_holds_it`: a tool without a stick-out, and with `centre_cutting`, declares 1; with a stick-out, 2; a tool set file, 1; each reads back unchanged. On `main` the stick-out tool declared 1.
  - `req_tl_01_req_nfr_07_a_library_file_of_a_newer_schema_says_it_needs_a_newer_version`: a tool and a tool set file of schema 3 name "tool library schema 3", "reads up to 2" and "newer OpenChuckCAM", never "project"; in a library folder the other 8 tools open and the problem is listed. On `main` it read "uses project schema 3, but this version reads up to 1".
  - `req_tl_01_req_nfr_07_a_tool_file_with_an_unknown_field_was_written_by_a_newer_version`: a field this build does not know is named, as written by a newer version. On `main`: "unknown field `holder_gauge`, expected one of …".
  - `req_tl_01_a_schema_1_tool_file_with_a_stick_out_reads_and_is_not_rewritten_on_open`: a nightly-written file reads with its stick-out, opening the library leaves its bytes unchanged, and saving it again declares 2. It passed on `main` too: it guards item 5.
- `tools::tests::req_nfr_07_bad_tool_files_give_clear_errors` takes schema 3, not 2, as its too-new case.

Results before the push:
- `cargo nextest run -p openchuck-project`: 212 passed.
- `cargo nextest run -p openchuck-app -p openchuck-ui -p openchuck-cli -E 'test(/tool/) | test(/library/) | test(/schema/)'`: 215 passed, and 224 after the rebase.
- `cargo fmt --all --check` and `cargo clippy --workspace --all-targets`: clean.
- `cargo xtask deps`, `licences`, `records` and `trace`: ok.
- Coverage, checked locally on `openchuck-project`'s own tests (`cargo llvm-cov nextest --profile coverage -p openchuck-project`): every new line is covered except the pass-through of other read errors (for example a merge conflict in a library file) in `read_library_file`, which keeps the error the file reader gives.

Not verified here:
- A released 0.55 build was not run on a schema-2 file; its behaviour is read from its code at the tag (the schema check comes before the fields are read).
