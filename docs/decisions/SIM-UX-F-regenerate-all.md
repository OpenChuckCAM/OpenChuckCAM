# SIM-UX-F: Regenerate all makes every operation again and says when a kept toolpath was out of date (UI-09, UI-11, UI-14, GC-07, NFR-06, NFR-12; issue #559; lane SIM-UX)

- **Status:** Proposed by lane SIM-UX on 24 Nov 2026, built to the project owner's answers relayed by the coordinator:
  - Q7 C (24 Nov 2026): only "Regenerate all", in the menu that holds the generate commands (the Project menu), with no per-operation action;
  - Q8 B (24 Nov 2026): a warning when a forced run makes a toolpath that differs from the kept one;
  - the key Ctrl Shift G (26 Nov 2026).
  Awaiting the owner's acceptance (decision 0049).
- **Date:** 24 Nov 2026.
- **Requirements:**
  - UI-09: generation off the interface thread, with progress and cancel, unchanged.
  - UI-11: the stale and "Simulation out of date" rules keep their meaning.
  - UI-14: every command in a menu and on the keyboard.
  - GC-07 and NFR-06: the same project, post and version give the same bytes.
  - NFR-12: a toolpath that was out of date is said, not hidden.
- **Amends:** [specification section 16](../ui-design/ui-specification.md) gains the key, edited in the same pull request. Nothing else is amended.
- **Open items:** adds OI-9954 (the owner's acceptance of this record).

Numbering: provisional. The coordinator assigns the record number and the OI number at release, in merge order. No project schema number and no library-family change.

## Context

Generation reuses an operation's kept result when its cache key, a hash of everything the operation is made from (architecture section 6), is unchanged. If an edit changes something the key does not hold, the operation is not marked stale, and its old toolpath stays on screen and can be exported. Issue #558 was such a case: a tab's length on an unheld end. It was fixed on 8 Nov 2026.

Before this change, nothing in the program made an up-to-date operation again, except closing and reopening the project (the cache lives in memory, PRJ-03). The owner asked for a workaround (#559). Asked what it should cover, the owner chose a single "Regenerate all" (Q7 C) that also reports a missed change (Q8 B).

## Decision

1. **Project > Regenerate all** (Ctrl Shift G), after "Generate and simulate", makes every enabled operation again in machining order, whatever the cache holds. It simulates when the project does, as Generate does. Its progress, its cancel (Esc) and its refusals are those of any generation. The tooltip says what it is for.
2. **The new results replace the kept ones**, so the next Generate reuses them.
3. **A toolpath that comes out differently is a warning.** For each operation whose new toolpath (moves, notes, warnings, tool, part bounds, unreached areas, facing data) differs from the one kept under the same key, the run has one warning, in the form of decision 0536: "Regenerate all (operation '<name>'): its toolpath came out differently from the one kept, although nothing it is made from had changed, so a change was not noticed. The new toolpath is the one used now. Please report it, with this message: build <version>, key <first 12 characters>, <n> moves kept, <m> moves now, first difference at move <k>."
   - The warning names the operation and nothing else from the project.
   - The key is a hash and does not reveal the project.
   - The warning shows in Messages and Results, and as `warning:` on the command line.
4. **Nothing changes when nothing was missed.** An operation whose toolpath is the same gives no warning, and its G-code is byte-identical (GC-07).

## Alternatives considered

- **Regenerate the selected operation and the ones after it** (Q7 B), or only the selected one (Q7 A): not chosen; the owner chose C.
- **Say nothing when a toolpath differs** (Q8 A): not chosen; a missed change is a fault the owner wants to hear about.
- **Ctrl G:** rejected, because the export dialog's G-code preview uses Ctrl G for "go to line".

## Consequences

- `openchuck-app`: `Application::start_regenerate_all`; the pipeline takes a `force` flag that skips cache hits and compares with the kept result.
- `openchuck-ui`: the menu item, the key and its handling, the shortcuts list, and `RunPanel::start_regenerate_all`.
- Specification section 16, the user guide's Keyboard and shortcuts, and the release notes.

## Owner check

1. Open the baluster tutorial project, `testdata/examples/baluster/baluster-project` (copy it first), and press **Generate stale (2)**.
2. Open the **Project** menu. Expect **Regenerate all** with "Ctrl+Shift+G", after "Generate and simulate"; its tooltip says what it is for.
3. Press <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>G</kbd>. Expect both operations to generate again (Jobs: "Generated 2"), with no warning, and the program unchanged. **Post and export** shows the same line counts and times.
4. **Pass:** each step does as written. **Fail:** say which step and what you saw.

## Verification

Linux cloud container, 4 vCPU, no GPU; branch `claude/lane-sim-ux-f`, rebased on `main` `361613e`.

**Tests, written first from the owner's answers (Q7 C, Q8 B) and UI-14, and watched failing.**
- `operation::req_ui_09_req_gc_07_regenerate_all_makes_every_operation_again_and_says_when_one_differs` (`openchuck-app` integration; 2.1 to 3.4 s). It uses a test strategy whose path depends on a value outside its cache key, to stand in for a missed change. It checks that:
  - with nothing missed, Regenerate all makes both operations again, reuses none, gives the same toolpath and gives no warning;
  - after the hidden change, Generate reuses the old path (the bug being worked round), while Regenerate all makes the new one and warns once, for that operation only, naming it, with the build and the move counts;
  - the next Generate reuses the new path.
  With the forced run turned off, it failed at the first check.
- `shell::frame_tests::req_ui_14_req_ui_09_ctrl_shift_g_makes_every_operation_again` (`openchuck-ui`; 3.5 to 4.8 s): Ctrl Shift G makes every operation again although none is stale.

**Suites and checks:**
- `cargo nextest run -p openchuck-app -p openchuck-ui -p openchuck-cli`: 1,751 tests, all passed.
- `cargo fmt --all --check` and `cargo clippy --workspace --all-targets`, with no warnings.
- `cargo xtask deps`, `licences`, `records`, `trace` and `changelog check-pr`, all ok.

**Not verified:** a real keyboard and window manager (the owner check above), and the owner's projects (none used).
