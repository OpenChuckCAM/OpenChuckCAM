# 9901: A post lists the comments its controller acts on, and project text never starts one (GC-03, GC-04, GC-10, NFR-07, NFR-12; issue #851; lane POST-COMMENTS)

- **Status:** Proposed by lane POST-COMMENTS on 23 Nov 2026. The owner directed on 23 Nov 2026 (relayed by the coordinator) that the words be defined in the post definition, not hard-coded (item 1). Item 2 (what happens to such text) is the owner's choice; the lane built its recommendation, option B, so the pull request can show it, and changes it if the owner chooses otherwise. **Number provisional** (9901): the coordinator assigns the real number when the pull request is released.
- **Date:** 23 Nov 2026.
- **Requirements:** GC-03 and GC-04 (post definitions and options), GC-10 (the LinuxCNC post), GC-09 (the Makera Z1 post), GC-07 and NFR-06 (deterministic output), NFR-07 (clear errors, no surprises from input), NFR-12 (the user verifies G-code).
- **Related:** [decision 0019](0019-sp-10-template-engine.md) item 4 (comment sanitising), [decision 0533](0533-safety-notice-post-option.md) (the NFR-12 notice, word for word), [decision 0656](0656-post-line-length-limit.md) (comment wrapping, issue #840), [decision 0007](0007-sp-12-z1-conventions.md) (the Z1 firmware source).
- **Open items:** OI-9901 (the owner's choice of item 2 and acceptance; provisional number).

Requirements and architecture are not edited by this record. No project or machine schema change: the list is an optional post option with an empty default, like `max_line_length` (decision 0656).

## Context

Issue #851 (sev2, found by lane POST-SAFETY from a CodeRabbit note on #840). Some controllers act on a comment instead of skipping it. `sanitise_comment` (decision 0019 item 4) replaces parentheses, control characters and non-ASCII, but keeps letters and commas, so a name from the project can start such a comment:

1. **On `main` before #840:** the program title is `<project> - <setup>`. A project named `MSG,direct title` makes the LinuxCNC post's first line `(MSG,direct title)`, which LinuxCNC shows as an operator message.
2. **With #840's wrapping:** a wrapped comment's later lines start fresh, so a word in the middle of a title, an operation name or a tool name can start a line. With a limit of 20, the title `abcdefghijklmnopqrMSG,x` is written as `(abcdefghijklmnopqr)` then `(MSG,x)`. The built-in LinuxCNC post wraps past 255 characters, the Z1 post past 63.

### What each controller does, and how it is known

**LinuxCNC: verified from its source** (`LinuxCNC/linuxcnc`, `master` at `49d14cb6`, 23 Nov 2026; not run in the simulator for this record). `Interp::convert_comment` (`src/emc/rs274ngc/interp_convert.cc`) skips the comment's leading white space (`isspace`), lowers its case and compares the start with:

| Opening | What LinuxCNC does |
| --- | --- |
| `msg,` | shows the rest as an operator message |
| `debug,` | shows it, with parameters expanded, when parameter 5599 is set |
| `print,` | prints it to standard output |
| `log,` | writes it to the open log file |
| `logopen,`, `logappend,` | opens a log file of that name on the controller (`logopen` replaces it) |
| `logclose` (the whole comment) | closes the log file |
| `py,`, `pyrun,` | runs the rest as Python, where embedded Python is set up |
| `pyreload` | reloads the Python plugin |
| `abort,` | stops the program with the rest as its error |

Any other comment goes to the task's `COMMENT` (`src/emc/task/emccanon.cc`), which compares the text, case-sensitively, with `PROBEOPEN` (opens, and so replaces, a probe results file named by the rest), `PROBECLOSE` and `RPY` (sets an orientation). A line starting `;py,` also runs Python, but no comment the post writes starts with `;`. The comparison applies to a comment anywhere on a line, after an `N` word or after a command. Because leading white space is skipped, **a leading space does not defeat it** (the question issue #851 asked to check first). The AXIS display also reads `(AXIS,...)` comments, for its preview only; they change nothing on the machine and are not listed.

**Makera Z1 firmware: verified from its source only, not on the machine.** `MakeraInc/MakeraZ1Firmware` at `a9918d95` (18 Nov 2026), a Smoothieware fork: `GcodeDispatch::on_console_line_received` (`src/modules/communication/GcodeDispatch.cpp`) answers `ok` to a line whose first character after spaces is `(` or `;` and does nothing else with it, and cuts a command line at its first `(` or `;`; `SimpleShell::on_console_line_received` takes only lines that start with a lower-case letter or `$`. No other code reads a comment's text. So the Z1 has no active comments.

**Stock Smoothieware: verified from its source only.** `Smoothieware/Smoothieware`, branch `edge` at `91fe4286`: the same comment handling in `GcodeDispatch.cpp`. No active comments. No other controller was checked.

### Every place project text reaches a comment

| Text | Where it is written | Can it start a comment line? |
| --- | --- | --- |
| Project name (then ` - ` and the setup name, and the part label of a split program) | the title line `(<title>)` | yes, directly (item 1 of the issue); the setup name and label only by wrapping |
| Operation name, tool name | `(operation <name>, tool T<n> <tool>, about <s> s)` | only by wrapping |
| Tool name | `Step::ToolChange` `name`, which the built-in template does not write; a post's own template may | yes, in a template that writes it first |
| Revision | `(revision <id>)` | only by wrapping; the host passes none today |
| Generator | `(OpenChuckCAM <version>)` | no: fixed text from the host |
| NFR-12 notice | fixed text, two lines | only by wrapping, and no word of it starts with a LinuxCNC opening |
| Header, footer, unwind command | the post author's own lines | not project text: the author's to write, including an intended `(MSG,...)` |

## Decision to take

1. **Where the list lives (the owner's direction, 23 Nov 2026):** an optional post option `active_comment_prefixes`, a list of the openings of comments the controller acts on. A comment matches when its text, after its leading spaces, starts with one in any case (a superset of LinuxCNC's rule, which is case-insensitive for the interpreter's words and case-sensitive for the task's). The built-in LinuxCNC post lists the fourteen openings above; the Makera Z1 post leaves it out, as does any post whose controller has none. Entries are checked when the post is loaded: 1 to 32 printable ASCII characters without spaces or parentheses, at most 64, none starting with the guard mark of option B, and, with the notice on, none that starts a word of the notice (else a wrapped notice line would change, against decision 0533).
2. **What happens when project text would start an active comment:**
   - **A: refuse.** Posting stops with a message naming the text and what to rename (NFR-07). Simple to explain for a name that starts with `MSG,`, but a wrapped fragment depends on where the line breaks, so a name that posts on one post or line limit is refused on another, and the user has to guess what to rename. A header or footer of the post's own is not refused.
   - **B (recommended): neutralise with a mark.** The comment is written with one character, `_`, in front (`(_MSG,direct title)`), so the controller compares `_msg,...` and reads an ordinary comment. Applied to every text from the project the post passes to its template (title, revision, generator, each operation comment, each tool name), after the sanitiser, and to each line a wrapped comment starts, counting the mark in the line's width so the line still fits. The start of a wrapped comment is the template's own and is not marked, so a post author's `(MSG,...)` header is kept. Ordinary names give the same bytes as before; the text stays readable, nothing is lost, and the output is deterministic.
   - **C: break the comma.** Replace the character after the opening (`MSG,` becomes `MSG;`). Changes the user's text, and does not work for openings with no comma (`LOGCLOSE`, `PYRELOAD`, `PROBEOPEN`, `PROBECLOSE`, `RPY`), so it needs a second rule for those.
   - **D: a fixed hard-coded word list** in the sanitiser instead of the post option. Rejected by the owner's direction: a post for another controller could not say what its controller does, and the Z1 would lose names it writes correctly.

## Recommendation

Item 1 as directed, and option B for item 2. It removes the effect without a refusal that depends on line breaks, keeps every word of the user's text, leaves the post author's own active comments working, keeps the NFR-12 notice word for word (no LinuxCNC opening starts a word of it, and the option check keeps it so for any post), and changes no golden program. Generated G-code is still the user's to verify before it is run (NFR-12).

**Owner questions** (most important first):

1. Item 2: B (mark, recommended), A (refuse) or C (break the comma)?
2. The mark: `_`, or another single printable character? It must not be white space, which LinuxCNC skips.
3. Should a post author's own header or footer comment that starts with a listed opening stay active (recommended: yes, it is the author's text), or be marked too?

## Consequences (option B, as built)

- `openchuck-post`: `PostOptions::active_comment_prefixes` (a new public field of a struct built by literal, so breaking for `cargo-semver-checks`: the change needs a workspace minor bump, which the coordinator assigns at release, decision 0048); the public constant `ACTIVE_COMMENT_GUARD`; the guard in `TemplatePost::post` and in the comment wrap. The built-in LinuxCNC `post.toml` lists the openings; the Z1 `post.toml` says why it lists none.
- No golden program changes: no ordinary name starts with a listed opening.
- No change to `openchuck-gcode`, `openchuck-app` or the interface; the title is still built in `openchuck-app` (`posting.rs`) and guarded in the post.
- User guide: `reference/writing-a-post.md` (the option), `reference/gcode-and-posts.md` (what the LinuxCNC post does with such names); `crates/openchuck-post/posts/README.md`; release notes; the post crate's changelog.

## Verification

Lane POST-COMMENTS, branch `claude/lane-post-comments`, written on `main` `286472a7` and rebased on `0ff4b13d` (#841) before the push, where the post tests, clippy and `records` were run again; 23 Nov 2026, Linux container, Rust 1.97.0.

**Tests written from the requirement first** (`crates/openchuck-post/tests/posts.rs`; each failed before the change, by not compiling against the new option; with the match disabled in the built code, the first three fail). The oracle is LinuxCNC's rule, written into the test from its source independently of the post definition:

| Test | Covers |
| --- | --- |
| `req_gc_10_req_nfr_07_a_project_name_never_starts_an_active_comment_on_linuxcnc` | the built-in list holds every LinuxCNC opening; issue item 1 (`(_MSG,direct title)`); every opening in upper and lower case, after spaces, as the title and the operation name; deterministic |
| `req_gc_10_req_gc_08_a_wrapped_line_never_starts_an_active_comment` | issue item 2 (`(abcdefghijklmnopqr)` then `(_MSG,x)`); a word too long for a line that starts with an opening (its first line marked, the mark counted); every opening inside the title, setup part, operation and tool name, cut and after a space, at limits 20 to 48; every comment line fits; deterministic |
| `req_gc_09_the_z1_post_lists_no_active_comments_and_writes_names_as_they_are` | the Z1 post lists none and writes `(MSG,direct title)` as it is |
| `req_gc_03_req_gc_04_the_posts_own_active_comments_are_kept` | a header `(MSG,...)` is kept; when wrapped, its first line is kept and a later line is marked |
| `req_gc_04_req_nfr_07_bad_active_comment_prefixes_are_refused` | empty, spaced, parenthesised, mark-led and notice-word entries and a list of 65 refused at load, naming the option; a notice word is allowed with the notice off |

The golden program tests of both posts pass unchanged. No committed post output starts a comment with a LinuxCNC opening (the one match under `testdata/`, `gcode/linuxcnc_post_subset.ngc` line 48, is a hand-written G-code reader fixture, not post output).

**Runs (uninstrumented, `cargo nextest run`):** `openchuck-post` 85 of 85, 2.5 s; each new test under 0.2 s. `cargo fmt --all`, `cargo clippy --workspace --all-targets` (no warnings), `cargo xtask deps`, `licences`, `records` and `trace` pass. `openchuck-app`, `openchuck-gcode` and the interface are unchanged and were not rerun; they build under clippy. The user guide's `check.py` and link checks were not run here (no `mdbook` in this container); CI runs them.

**Coverage (instrumented, own tests):** `render.rs` 98.2 % of lines, `options.rs` 94.6 %; every new line is reached. The lines not reached in those files were not reached before this change.

**Not verified:** no program was run in the LinuxCNC simulator or on the Z1 for this change; the LinuxCNC and firmware behaviour above is from their sources at the commits named. Generated G-code is the user's to verify before it is run (NFR-12).
