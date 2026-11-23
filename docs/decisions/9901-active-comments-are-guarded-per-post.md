# 9901: A post lists the comments its controller acts on, and project text never starts one (GC-03, GC-04, GC-10, NFR-07, NFR-12; issue #851; lane POST-COMMENTS)

- **Status:** Proposed by lane POST-COMMENTS on 23 Nov 2026; **not yet accepted**. The owner directed on 23 Nov 2026 (relayed by the coordinator) that the words be defined in the post definition, not hard-coded (item 1), and answered the record's three questions the same day (relayed): option B, the mark `_`, and the post author's own comments stay active (item 2). The owner added a requirement: the user is warned of every text that was changed and where it came from (item 3). All three are built. **Number provisional** (9901): the coordinator assigns the real number when the pull request is released.
- **Date:** 23 Nov 2026.
- **Requirements:** GC-03 and GC-04 (post definitions and options), GC-10 (the LinuxCNC post), GC-09 (the Makera Z1 post), GC-07 and NFR-06 (deterministic output), NFR-07 (clear errors, no surprises from input), NFR-12 (the user verifies G-code).
- **Related:** [decision 0019](0019-sp-10-template-engine.md) item 4 (comment sanitising), [decision 0533](0533-safety-notice-post-option.md) (the NFR-12 notice, word for word), [decision 0656](0656-post-line-length-limit.md) (comment wrapping, issue #840), [decision 0007](0007-sp-12-z1-conventions.md) (the Z1 firmware source).
- **Open items:** OI-9901 (the owner's acceptance of the record; provisional number). The choices it held are made.

Requirements and architecture are not edited by this record. No project or machine schema change: the list is an optional post option with an empty default, like `max_line_length` (decision 0656), and the warning is an ordinary run warning with nothing saved.

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

## Decision

Items 1 and 2 were put to the owner as below; the choices are under *Owner's answers*, and item 3 is the owner's addition.

1. **Where the list lives (the owner's direction, 23 Nov 2026):** an optional post option `active_comment_prefixes`, a list of the openings of comments the controller acts on. A comment matches when its text, after its leading spaces, starts with one in any case (a superset of LinuxCNC's rule, which is case-insensitive for the interpreter's words and case-sensitive for the task's). The built-in LinuxCNC post lists the fourteen openings above; the Makera Z1 post leaves it out, as does any post whose controller has none. Entries are checked when the post is loaded: 1 to 32 printable ASCII characters without spaces or parentheses, at most 64, none starting with the guard mark of option B, and, with the notice on, none that starts a word of the notice (else a wrapped notice line would change, against decision 0533).
2. **What happens when project text would start an active comment:**
   - **A: refuse.** Posting stops with a message naming the text and what to rename (NFR-07). Simple to explain for a name that starts with `MSG,`, but a wrapped fragment depends on where the line breaks, so a name that posts on one post or line limit is refused on another, and the user has to guess what to rename. A header or footer of the post's own is not refused.
   - **B (recommended): neutralise with a mark.** The comment is written with one character, `_`, in front (`(_MSG,direct title)`), so the controller compares `_msg,...` and reads an ordinary comment. Applied to every text from the project the post passes to its template (title, revision, generator, each operation comment, each tool name), after the sanitiser, and to each line a wrapped comment starts, counting the mark in the line's width so the line still fits. The start of a wrapped comment is the template's own and is not marked, so a post author's `(MSG,...)` header is kept. Ordinary names give the same bytes as before; the text stays readable, nothing is lost, and the output is deterministic.
   - **C: break the comma.** Replace the character after the opening (`MSG,` becomes `MSG;`). Changes the user's text, and does not work for openings with no comma (`LOGCLOSE`, `PYRELOAD`, `PROBEOPEN`, `PROBECLOSE`, `RPY`), so it needs a second rule for those.
   - **D: a fixed hard-coded word list** in the sanitiser instead of the post option. Rejected by the owner's direction: a post for another controller could not say what its controller does, and the Z1 would lose names it writes correctly.

## Recommendation and owner's answers

The lane recommended item 1 as directed and option B for item 2: it removes the effect without a refusal that depends on line breaks, keeps every word of the user's text, leaves the post author's own active comments working, keeps the NFR-12 notice word for word (no LinuxCNC opening starts a word of it, and the option check keeps it so for any post), and changes no golden program.

**Owner's answers** (23 Nov 2026, relayed by the coordinator):

1. Item 2: **option B**, a one-character mark.
2. The mark: **`_`**.
3. The post author's own header or footer comment that starts with a listed opening **stays active**; only the later lines of one that is wrapped are marked.
4. **Added:** whenever text is marked, the user is **shown a warning** that says which text was changed and where it came from, so that someone who puts a command into a shared project or an imported tool library is seen. This is item 3.

## Item 3: the warning (the owner's addition)

- **What the post reports.** `PostOutput::guarded_comments` lists each mark the post wrote, as a `GuardedComment`: its `CommentSource` (`Title`, `Revision`, `Generator`, `Operation(tag)`, `Tool(tag)`, or `Post` for a comment of the post's own or of the program's own words), the opening matched as the post lists it, the text as the job gave it (before sanitising) and the character in that text where the mark went. A text the template was given and guarded is reported only when the template wrote it at the start of a comment. A wrapped line is attributed to the job text whose span holds the start of the line; a line held by none is the post's own. Each (source, place, text) is reported once, in source order, so the report is deterministic (GC-07, NFR-06).
- **What the user sees.** `openchuck-app` turns the report into one run warning per field and text, for example:
  `setup 'Setup 1': the project name "MSG,direct title" would start a comment with "MSG,", which the controller of the post 'LinuxCNC verification' acts on, so the post wrote "_" in front of it; check where this text came from`.
  The field is the project name, the setup name or the label in the file's title (the title is split by where the mark went), the operation name, `the name of tool T<n>`, the revision, the version line, or a comment of the post. The text is quoted and escaped (control characters, quotes, backslashes and non-ASCII as `\u{..}`), and cut after 48 characters with `...`, so a name can neither write to a terminal nor end the quote. It says nothing about whether the program is safe or checked (NFR-12).
- **Where it appears.** It is an ordinary warning of the run: in the Results window's Messages and the status count after generating with a post, and as a `warning:` line on the command line. It needs no dismissal and nothing is saved in the project file: no schema change. The G-code changes only by the mark.

![What the LinuxCNC post writes, before this change and with option B, and the warning the run shows. 1: a project named "MSG,direct title" gave the title line (MSG,direct title), which LinuxCNC shows as an operator message; with B it is (_MSG,direct title), an ordinary comment. 2: the title "abcdefghijklmnopqrMSG,x" at a line limit of 20 was wrapped to (abcdefghijklmnopqr) and (MSG,x); with B the second line is (_MSG,x), the mark counted in the 20. 3: a header line the post's author writes, (MSG,Check the stock is held), is kept as it is. 4: the Makera Z1 post writes (MSG,direct title) unchanged. Below, the warning for case 1, naming the project name, its text, the opening MSG, and the post.](9901-active-comments-are-guarded-per-post.svg)

No screenshot: the comment lines are only in the exported file, and the warning is an ordinary line of the Messages list and of the command line's output, whose wording the diagram shows; the Messages tab itself is unchanged.

Generated G-code is still the user's to verify before it is run (NFR-12).

## Consequences (as built)

- `openchuck-post`: `PostOptions::active_comment_prefixes` and `PostOutput::guarded_comments` (new public fields of structs built by literal, so breaking for `cargo-semver-checks`: the change needs a workspace minor bump, which the coordinator assigns at release, decision 0048); the public constant `ACTIVE_COMMENT_GUARD` and the types `CommentSource` and `GuardedComment`; the guard in `TemplatePost::post` and in the comment wrap. The built-in LinuxCNC `post.toml` lists the openings; the Z1 `post.toml` says why it lists none.
- `openchuck-app` (`posting.rs`): the warning. No change to the command line or the interface: both already show the run's warnings.
- No golden program changes: no ordinary name starts with a listed opening.
- User guide: `reference/writing-a-post.md` (the option), `reference/gcode-and-posts.md` (what the LinuxCNC post does with such names, the warning and what to do); `crates/openchuck-post/posts/README.md`; the release notes' development version; the post and application crates' changelogs.

## Verification

Lane POST-COMMENTS, branch `claude/lane-post-comments`. First push: written on `main` `286472a7`, rebased on `0ff4b13d` (#841). Second push (the owner's answers and the warning, the allowed pre-ready push): on the same base, not merged with later `main` (that is the release push). 23 Nov 2026, Linux container, Rust 1.97.0.

**Tests written from the requirement first.** Each failed before its change: the first five by not compiling against the new option, the sixth against the new report, the seventh by asserting the warning before `openchuck-app` wrote it (checked again by disabling the warning: it fails). With the match disabled in the built code, the first, second and fourth fail. The oracle is LinuxCNC's rule, written into the test from its source independently of the post definition:

| Test | Crate | Covers |
| --- | --- | --- |
| `req_gc_10_req_nfr_07_a_project_name_never_starts_an_active_comment_on_linuxcnc` | `openchuck-post` | the built-in list holds every LinuxCNC opening; issue item 1 (`(_MSG,direct title)`); every opening in upper and lower case, after spaces, as the title and the operation name; deterministic |
| `req_gc_10_req_gc_08_a_wrapped_line_never_starts_an_active_comment` | `openchuck-post` | issue item 2 (`(abcdefghijklmnopqr)` then `(_MSG,x)`); a word too long for a line that starts with an opening (its first line marked, the mark counted); every opening inside the title, setup part, operation and tool name, cut and after a space, at limits 20 to 48; every comment line fits; deterministic |
| `req_gc_09_the_z1_post_lists_no_active_comments_and_writes_names_as_they_are` | `openchuck-post` | the Z1 post lists none and writes `(MSG,direct title)` as it is |
| `req_gc_03_req_gc_04_the_posts_own_active_comments_are_kept` | `openchuck-post` | a header `(MSG,...)` is kept (the owner's answer 3); when wrapped, its first line is kept and a later line is marked |
| `req_gc_04_req_nfr_07_bad_active_comment_prefixes_are_refused` | `openchuck-post` | empty, spaced, parenthesised, mark-led and notice-word entries and a list of 65 refused at load, naming the option; a notice word is allowed with the notice off |
| `req_gc_10_req_nfr_07_each_guarded_text_is_reported_once_with_its_source` | `openchuck-post` | nothing reported for ordinary names or on the Z1; the title after spaces (place 2); at a limit of 20 the title cut in a word, an operation name and a tool name, once each, in source order; a title guarded at its start with a second opening at a wrap; a wrapped header line reported as the post's; deterministic |
| `posting::req_gc_10_req_nfr_07_a_guarded_name_is_a_warning_naming_its_source` | `openchuck-app` | a project named `MSG,direct title`: `(_MSG,direct title - Setup 1)` and the exact warning; the Z1 neither changes nor warns; with a limit of 20, warnings for the project name, the operation name and `the name of tool T1`, one each, the tool's escape character shown as `\u{1b}` and its text cut at 48 characters with `...`; no raw control character in any warning; deterministic |

The golden program tests of both posts pass unchanged. No committed post output starts a comment with a LinuxCNC opening (the one match under `testdata/`, `gcode/linuxcnc_post_subset.ngc` line 48, is a hand-written G-code reader fixture, not post output).

**Runs (uninstrumented, `cargo nextest run`):** `openchuck-post` 86 of 86, 3.1 s; `openchuck-app` and `openchuck-cli` 980 of 980, 1 skipped, 1,890 s (37 slow tests, none new; the new app test takes 0.9 s, each new post test under 0.2 s). `cargo fmt --all`, `cargo clippy --workspace --all-targets` (no warnings), `cargo xtask deps` (with `messages` and `topics`), `licences`, `records` and `trace` pass. `openchuck-ui` is unchanged (it shows the run's warnings as before) and its tests were not run; it builds under clippy. The user guide's `check.py` and link checks were not run here (no `mdbook` in this container); CI runs them.

**Coverage (instrumented, `openchuck-post`'s own tests):** `render.rs` 98.4 % of lines, `options.rs` 94.6 %. New lines not reached: the end of a comment's text that is all spaces after its last word (`wrap_words`; a sanitised comment line never ends in spaces before `)` in the built-in template). In `openchuck-app`, the new test reaches the project name, operation and tool fields and `quoted`; the setup-name, file-label, program-title, revision, version-line, post-comment and unknown-tool arms of `guarded_comment_warnings` are not reached by a test (the instrumented application suite was not run for this push; the nightly run measures it).

**Not verified:** no program was run in the LinuxCNC simulator or on the Z1 for this change; the LinuxCNC and firmware behaviour above is from their sources at the commits named. The warning was not seen in the interface by hand (it is an ordinary run warning, which the interface already lists). Generated G-code is the user's to verify before it is run (NFR-12).
