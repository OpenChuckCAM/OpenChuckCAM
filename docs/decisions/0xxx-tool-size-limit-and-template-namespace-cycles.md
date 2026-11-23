# 0xxx: A largest tool size, template namespaces freed after a render, and checked mutation-run arguments (TL-02, TL-03, TL-04, GC-03, NFR-05, NFR-07; issues #628, #627, #794, #582; lane FUZZ-NFR07, pull request #853)

- **Status:** Accepted by the project owner on 23 Nov 2026, relayed by the coordinator (decision 0049). The owner chose option A for item 1 (a 1000 mm bound) and option A for item 2 (the namespace cleanup), and agreed item 3 (count, seed and max_seconds all checked). Written by lane FUZZ-NFR07.
- **Date:** 23 Nov 2026.
- **Requirements:** TL-02 and TL-03 (standard and custom tool shapes), TL-04 (tool fields), GC-03 (user-editable post definitions), NFR-05 (double precision, millimetres), NFR-07 (invalid input gives a clear error, never a crash), GC-07 and NFR-06 (deterministic output).
- **Open items:** raises OI-xxxx (report the namespace cycle to the template engine's maintainers, and drop the workaround of item 2 when a release frees it). No earlier open item tracks these issues.

Requirements and architecture are not edited by this record. No project, tool or machine schema changes. Library API: one addition, `openchuck_types::tool::profile::MAX_PROFILE_SIZE` (a `pub const`, minor-compatible); nothing removed or changed in signature. Generated G-code is unchanged for every valid tool and every built-in post.

## Context

Two nightly fuzz targets failed on `main`:

- **`tool_profile_toml` (issues #628 and #627, the same run 42465214501, 8 Nov 2026).** A custom profile file with one line segment to a radius of about 7.8e307 mm passed the monotone rule of [decision 0012](0012-sp-13-tool-profiles.md) item 2, because the number is finite. The fuzz target's check `height_at(end * i / 16)` then overflowed to infinity and `height_at` gave `None` (`assertion failed: h.is_some_and(f64::is_finite)`). The defect is not a non-finite value getting in. A finite size far larger than any tool is accepted, and every later product of two lengths (squares in the arc maths, sweeps, bounds) can overflow. The base64 copy of the input kept in the issue decodes to 357 bytes, not the 381 of the artifact. It is cut short: its radius is 7.8e283, which passes, so the regression file restores the missing 24 digits (radius 7.777717777777778e307), which reproduces the nightly panic at the same file and line.
- **`post_definition` (issue #794, run 43578275505, 14 Nov 2026).** Exit 77 from LeakSanitizer, not a crash. The input, a damaged copy of the LinuxCNC post, contains `{% set st.a = st %}`: a template-engine namespace that holds itself. The engine (minijinja 3.0.3; 3.1.0 has the same code) frees values by reference counting, so that cycle is never freed. A direct cycle, one through a list and one through a second namespace all leak. Macros do not leak, because the engine already breaks those. The namespace type is private to the engine, so its assignments cannot be intercepted.

Issue #582 (sev3): the STEP mutation program (`examples/step_mutations.rs`, [decision 0534](0534-nightly-step-mutation-budget-and-settings-fuzz-targets.md)) read a mistyped `max_seconds` as "no time budget". Its `count` and `seed` had the same silent default.

## Options and recommendation

**Item 1, a size larger than any tool (#628).**

- **A (recommended, built): one fixed upper bound in the profile check.** Every coordinate of a profile point or arc centre is at most `MAX_PROFILE_SIZE` = 1000 mm from the tip in radius and height, and so is an arc radius given in a profile file. Because `Tool::profile` builds every standard shape through the same check, this covers the profile file, project snapshots, the tool editor and the tool library. Before that, `Tool::profile` refuses a diameter or shank diameter above 2000 mm, and a flute or overall length above 1000 mm, naming the field the user typed. 1000 mm is far above any cutter and far below overflow: the largest product in the solvers is then about 1e6.
- **B: bound only in the profile text loader.** This leaves project files, the library and the editor open to the same value.
- **C: clamp to the limit.** This silently changes the user's tool, against NFR-07.
- **D: fix only the fuzz target's arithmetic.** The tool stays accepted.

![The (radius, height) half-plane of a tool profile. A real cutter of a few millimetres lies well inside the accepted square of 1000 mm by 1000 mm from the tip. The nightly fuzz input, one line to a radius of 7.8e307 mm, runs far outside it and is refused, naming the segment, the size and the limit.](0xxx-tool-size-limit.svg)

**Item 2, a namespace that holds itself (#794).**

- **A (recommended, built): free the namespaces after each render.** The post's template environment replaces the `namespace` function with one that calls the engine's own and records the namespace it makes. When the render ends, successfully or not, every key of every recorded namespace is set to `none` by a small generated template of `set` statements, the only way to change a namespace from outside the engine. That breaks every cycle, since namespaces are the only values a template can change. Because the cleanup must write `ns.key`, `namespace(...)` refuses a key that is not a name (ASCII letters, digits and `_`, not starting with a digit), with a template error naming the key. Such a key could only be read as `ns["a b"]`, and no built-in or example post uses one. Non-string keys were already dropped by the engine. The output text does not change. The code is in the crate's private `engine` module, so the public API does not change.
- **B: turn the fuzz target's leak check off for this target**, document the bounded leak and report it upstream. This would also hide any future leak in our own code.
- **C: report upstream only**, and leave the nightly job failing until the engine is fixed.

![Before: after `{% set st.a = st %}` the namespace's key refers back to the namespace, so it is still held when the render ends and is never freed. After: when the render ends, or fails, every key of every namespace the render made is set to none, nothing refers to the namespace any more, and it is freed.](0xxx-namespace-cycle.svg)

With A, the owner is still asked to report the cycle upstream (OI-xxxx). The lane cannot reach other repositories.

**Item 3, the mutation program's arguments (#582).** No choice needed: as the issue proposes, a `count`, `seed` or `max_seconds` that is given but is not a whole number prints which one and the usage, and exits with code 2. A missing argument keeps its default (100, 1, no budget). The nightly passes literal numbers, so its run does not change.

## Decision

The owner's choices (23 Nov 2026, relayed): item 1 option A with a limit of 1000 mm (diameters 2000 mm), item 2 option A, and item 3 as proposed.

### Upstream report (for the owner to file, OI-xxxx)

Text to paste in an issue on the minijinja repository:

> **A `namespace` that holds itself is never freed (memory leak).** minijinja 3.0.3 and 3.1.0 (`Namespace` in `src/value/namespace_object.rs` holds its values in a `Mutex<BTreeMap<Arc<str>, Value>>`; `{% set ns.attr = value %}` inserts with no cycle check). Rendering
>
> ```jinja
> {% set st = namespace(a=1) %}{% set st.a = st %}
> ```
>
> with `Environment::new().render_str(...)` leaks the namespace: it holds an `Arc` to itself, so its count never reaches zero. LeakSanitizer reports the allocation in `Namespace::set_value`. The same happens through a list (`{% set st.a = [st] %}`) and through two namespaces (`{% set p = namespace() %}{% set q = namespace(p=p) %}{% set p.q = q %}`). Closures do not leak, since the engine breaks those cycles itself. Found by fuzzing a user-editable template (cargo-fuzz, libFuzzer with leak detection, Rust nightly 1.102.0, x86_64 Linux). A fix could track namespaces as closures are tracked, and clear them when the render ends.

## Consequences

- A tool or profile with a size above the limit is refused, in the editor, a profile file, a tool file or a project, with a message naming the segment or field, the value (in exponent form when huge) and the limit. The editor's size fields show the limit as their maximum. The user guide's tool library page states the limits.
- A post template's `namespace` keys must be names. The post guide says so.
- No valid tool, profile or built-in post changes, and no G-code changes (GC-07, NFR-06).

## Verification

Lane FUZZ-NFR07, branch `claude/lane-fuzz-nfr07`, from `main` `8f61706`, 23 Nov 2026, Linux container (4 cores), Rust 1.97.0 stable for the tests and nightly 1.102.0 (2026-11-21) with `cargo-fuzz` for the fuzz targets.

**Reproduced on `main` before any change:**

- `tool_profile_toml` on the restored input (`testdata/tools/profiles/broken_too_large.toml`, without its comment lines): `panicked at fuzz_targets/tool_profile_toml.rs:21:13: assertion failed: h.is_some_and(f64::is_finite)`, the nightly failure.
- `post_definition` on `leak-0fa904286f4d0ea1976da456fec2183ff57fa19e` (the run's artifact, 4032 bytes, SHA-1 as its name, kept as `testdata/posts/fuzz_namespace_cycle.txt`): `LeakSanitizer: detected memory leaks`, allocated in minijinja's `Namespace::set_value`. Five hand-made templates: a namespace in itself, in a list in itself, two namespaces holding each other, a namespace made with another in its defaults that is then stored back, each leaked. A macro capturing a namespace stored in it, and a plain namespace, did not leak.

**After the change:** the restored #628 input runs clean (`Executed ... in 0 ms`). The #794 artifact and all six templates run with exit 0 and no leak report. A 10-minute run of each target from the nightly's seeds (plus the two new files) found nothing: `tool_profile_toml` 7,704,103 runs, `post_definition` 2,537,665 runs (`-max_total_time=600 -timeout=10`).

**Tests written from the requirement** (named by requirement ID):

| Test | Crate | Covers | Before the fix |
| --- | --- | --- | --- |
| `req_tl_03_req_nfr_07_profile_sizes_above_the_limit_are_rejected` | `openchuck-types` (`tests/tools.rs`) | item 1: the #628 radius, a point, a height and an arc centre above the limit refused naming the segment, and the limit itself valid with a finite envelope | did not compile (no limit) |
| `req_tl_02_req_nfr_07_tool_sizes_above_the_limit_are_rejected` | `openchuck-types` | item 1: diameter, flute length, overall length and shank diameter refused by field, and the largest allowed tool valid | did not compile |
| `req_tl_03_req_nfr_07_profile_corpus_reads_or_names_the_fault` (extended) | `openchuck-project` (`tests/tools.rs`) | item 1: `broken_too_large.toml` refused with "segment 1: the radius 7.777717777777778e307 mm is larger than 1000 mm" | would fail (the file was accepted) |
| `tool_edit::req_tl_04_req_nfr_07_size_fields_show_the_largest_tool_size` | `openchuck-app` | item 1: the editor's size fields carry the limit | failed (no maximum) |
| `engine::tests::req_nfr_07_a_namespace_that_holds_itself_is_freed_after_the_render` | `openchuck-post` (unit) | item 2: a value stored in a namespace cycle is freed after the render, also when the render fails after making the cycle (watched through a weak reference) | failed with the cleanup disabled |
| `engine::tests::req_nfr_07_namespaces_render_as_before` | `openchuck-post` (unit) | item 2: namespace state, keyword-like keys (`if`) and the output unchanged | passed (pins behaviour) |
| `engine::tests::req_nfr_07_a_namespace_key_that_is_not_a_name_is_refused` | `openchuck-post` (unit) | item 2: `"a b"`, `""` and `"9a"` refused, nothing leaked | failed (accepted) |
| `req_nfr_07_a_template_with_a_namespace_cycle_posts_without_a_crash` | `openchuck-post` (`tests/posts.rs`) | item 2: the #794 input loads and its post ends in a template error naming line 62, after the cycle is made | passed (the leak is not visible without the sanitiser; the unit test above watches it) |
| `req_nfr_07_a_namespace_key_that_is_not_a_name_is_a_template_error` | `openchuck-post` | item 2: the error reaches the post's caller naming `namespace` | failed (accepted) |

Item 3: `cargo check -p openchuck-import --features kernel --example step_mutations` passes. Run by hand, `-- 10 1 33OO`, `-- 10 x` and `-- ten` each print the argument at fault and the usage and exit 2. The example has no test target (it needs the kernel and the worker).

Also `profile::tests::req_tl_03_req_nfr_07_an_arc_radius_above_the_limit_is_refused` (`openchuck-types`, unit): an arc given by a radius of 1e300 mm is refused with the size and the limit, and one of exactly 1000 mm is not.

**Runs (uninstrumented, `cargo nextest run`):** `openchuck-types`, `openchuck-project`, `openchuck-post`, `openchuck-import` and `openchuck-app` together: 1267 of 1267 passed, 1 skipped, 1003 s on 4 cores. The slow tests are existing `openchuck-app` integration tests and are unchanged. Each new test takes under 0.1 s. The built-in posts' golden files and the reference jobs pass unchanged (GC-07, NFR-06). `cargo fmt --all`, `cargo clippy --workspace --all-targets` and `cargo clippy -p openchuck-import --features kernel --example step_mutations` give no warnings. `cargo xtask deps`, `licences` and `trace` (105 of 105 P1 requirements) pass. `cargo xtask records` reports only the provisional `OI-xxxx`, which the renumbering clears. `cargo semver-checks --baseline-rev origin/main` (v0.51.0) on `openchuck-types` and `openchuck-post`: no semver update required.

**Coverage (instrumented, each crate on its own tests, lower bounds):** `openchuck-types` `tool/mod.rs` 99.7 % and `tool/profile.rs` 94.6 % of lines. Every line this change adds is reached once the arc radius test above was added. The lines still missed are older defensive branches. `openchuck-post` `engine.rs` 80.5 %, `render.rs` 96.9 %, `definition.rs` 86.8 %. Every new line of the namespace wrapper and its cleanup is reached. The missed lines are the older filters' error branches (`number`, `decimals`, `sqrt` and others), which this crate's own tests do not call.

**Not verified:** the nightly workflow itself (it runs after merge); whether the owner wants another limit. The upstream report is not filed (OI-xxxx).
