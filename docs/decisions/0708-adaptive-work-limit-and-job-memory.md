# 0708: Adaptive spacing's limits are set per operation by an Adaptive work limit, and a toolpath job stops with an error before it passes the memory it may use (OP-04, OP-12, NFR-04, NFR-06, NFR-07, NFR-12, GC-07, UI-15; issue #847; lane PATH-QUALITY)

- **Status:** Proposed by lane PATH-QUALITY on 28 Nov 2026. It records the owner's answers on issue #847 and the texts the owner approved on 27 and 28 Nov 2026, relayed by the coordinator (decision 0049). The last four, approved as written on 28 Nov 2026, were the form note (item 7), the Settings line above 75 %, the line where the computer's memory cannot be read, and the size line (item 5). Only the project owner accepts this record (decision 0049).
- **Date:** 28 Nov 2026.
- **Issue:** #847 (adaptive spacing stops with a partial path at its limit of 1,000,000 geometry queries, with no way to let it go on). Also carries #993 (a face without a box of its own, in the measure of decision 0701).
- **Requirements:** OP-04 (finishing), OP-12 (gouge checks), NFR-04 (performance), NFR-06 and GC-07 (determinism), NFR-07 (no crash; a clear error), NFR-12 (the user verifies the program), UI-15 (settings).
- **Amends:** accepted [decision 0239](0239-adaptive-pass-spacing.md) items 8 to 10 (the limits are fixed; they bound work, not memory), and proposed [decision 0701](0701-entries-beside-a-selection.md) item 1 (the 65 by 65 grid for a face without a box).
- **Builds on:** [0239](0239-adaptive-pass-spacing.md) (adaptive spacing, its limits and the Partial path warning), [0322](0322-simulate-playback-frame-cost.md) and [0333](0333-playback-information-with-generation.md) (a memory setting that is the user's and this computer's, not the project's), [0048](0048-pre-1-0-api-changes.md) (the library's version).
- **Open items:** none added.

Numbering: decision 0708, provisional: the coordinator gives the number at the release push (0705 to 0707 were taken on 28 Nov 2026). No project schema number is needed: the new parameter is an ordinary operation parameter, and an operation that does not set it keeps its program.

## Context

Adaptive spacing (decision 0239) bounds each operation's work by fixed limits: 1,000,000 geometry queries, 1,000,000 points, 10,000 added passes. Where a limit is met it keeps the path made so far as a partial path. On large or finely toleranced parts the limit is met and nothing the user can set lets it go on (#847). Raising the limits raises the memory a job holds: measured on the public Baluster example, about 1.7 kB for each point of the passes once its G-code is written. A job that runs the computer out of memory is ended by the system, which NFR-07 does not allow.

## Decision

1. **Adaptive work limit (the owner's answers A and D).** Every strategy with adaptive spacing has a setting "Adaptive work limit" in the Advanced section, shown only with adaptive spacing on: the most geometry queries adaptive spacing may make for the operation. It is an integer from 1,000,000 (the default, decision 0239 item 8's limit, so an operation saved before it keeps its program) to 1,000,000,000. A value out of the range is refused by name.
2. **The other limits rise in step.** The points and added passes an operation may make rise by the same factor as the setting over its default, up to 100,000,000 points and 1,000,000 passes. The levels and pending intervals are unchanged. The limits are charged in decision 0239's one fixed order, so the same setting always gives the same program at any thread count.
3. **The Partial path warning says what to do.** It says what a geometry query is ("one drop of the tool onto the part"), which limit was met and, at the ceiling, that it is the most Adaptive work limit allows. Where raising the setting would help, it adds: "To finish it, raise Adaptive work limit (Advanced), which takes longer, or need fewer queries: a larger Allowed excess or Smallest spacing, or a smaller Stepover".
4. **A memory limit, kept by the program itself.** The host gives each job's context the most memory it may account for (`JobContext::memory`). Adaptive spacing charges 1.7 kB for each point of the operation's passes (a measured constant, `BYTES_PER_POINT`) at the start of refining and before each pass it adds. A job that would pass the limit stops with an error, `PathError::OutOfMemory`; nothing is made for that operation, and the project is unchanged. The charge comes in the same fixed order as the limits, so the stop is the same at any thread count; a job that finishes makes the same program whatever the limit (GC-07). It is a limit the program keeps itself: it does not stop the system ending the program when the computer runs short for other reasons.
5. **Memory a toolpath job may use (the owner's answer on Q6).** A personal setting in the app's Settings, a share of this computer's memory from 10 to 90 % in steps of 5, 50 % by default, shown with its size; above 75 % a line warns that little may be left for anything else. Where the computer's memory cannot be read there is no limit, and Settings says so. The command line takes `--job-memory PERCENT`, as it takes `--threads`, since it does not read the app's settings.
6. **The error says what happened and what to change**, in the owner's approved words, naming where the limit is set (Settings or `--job-memory`), and whether a raised Adaptive work limit is the likely cause or the part needs more passes than fit.
7. **A note in the form.** Where the most the operation could need at its Adaptive work limit (the points ceiling there × 1.7 kB) is more than a job may use, a line under the field says so: "At this limit the operation could need up to 47 GB; toolpath jobs may use 32 GB on this computer (Settings). If it needs more, it is stopped with an error." It is a note, not a refusal: most parts never come near the most.
8. **A face without a box of its own (issue #993, amends 0701 item 1).** In the measure of entries beside a selection, a face whose `ParametricFace::bounds` is `None` is measured against every entry, instead of being bounded from a 65 by 65 grid over its untrimmed surface, which could miss part of it. The reach is taken from the faces that have boxes.

## Alternatives and risks

- **A time limit** was not chosen: it would make the program depend on the machine and its load (GC-07).
- **Reading the free memory** was not chosen: it changes from moment to moment, so the same project could stop on one run and not the next. A share of the installed memory is the same each run.
- **Risk:** the charge per point is measured on one public example; other parts may hold more or less per point. The simulation's own memory is not charged here (playback has its own limit, decision 0322). Where the charge is low, the system may still end a job on a computer short of memory; the setting's tooltip says so.

## Verification

Tests were written from the owner's answers first, each seen failing before the code:

- `openchuck-path`, `tests/adaptive_work_limit.rs`: every adaptive strategy has the setting under its own name, hidden with adaptive spacing off; a value out of range is refused by name, and the default equals leaving it unset; a job over its memory limit is stopped and makes nothing, with the same `reached` at 1, 2 and 8 threads; raising the limit confirms more places (a 30 mm cylinder at an excess and spacing of 0.0001 mm, about 27 s in release); every adaptive finish carries the memory limit into its work.
- `openchuck-path` unit tests: the limits rise with the setting to their ceilings; the Partial path warning says what a query is and how to finish; a job that would pass its memory limit is stopped; a face without a box is near every entry (#993).
- `openchuck-app`: an operation over the memory a job may use is stopped and named (`tests/operation.rs`); the form says when a raised limit could need more memory than allowed; the setting is personal and bounded (`tests/settings.rs`); the error texts (`error.rs` unit test).
- `openchuck-cli`: `--job-memory` is taken and nonsense refused (`tests/cli.rs`).
- `openchuck-ui`: Settings sets the memory a toolpath job may use, keeps it, gives it to the application, and says when the memory cannot be read (`shell::frame_tests`).

Found while testing: the pipeline's sub-contexts were made with `JobContext::new` and dropped the memory limit; they now use `JobContext::with_progress`, and a guard test checks every adaptive finish.

Not verified: no air or wood cut; the 1.7 kB charge on parts other than the Baluster. Generated G-code remains the user's to verify before it is run on a machine (NFR-12).
