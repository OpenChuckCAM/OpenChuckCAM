# SIM-UX-G: G generates every stale operation, Shift G the selected one (UI-14, UI-09; lane SIM-UX)

- **Status:** Proposed by lane SIM-UX on 24 Nov 2026, built to the project owner's answers of 24 Nov 2026, relayed by the coordinator: swap Generate and Generate stale on the keys, and F5 follows G. Awaiting the owner's acceptance (decision 0049).
- **Date:** 24 Nov 2026.
- **Requirements:** UI-14 (every command from the menus and the keyboard), UI-09 (generation runs off the interface thread with progress and cancel; unchanged).
- **Amends:** [decision 0117](0117-slice-22-shell-frame.md), "Keys" (G and F5 generated the selected operation; Shift G and Shift F5 every stale operation), and the line of [specification section 16](../ui-design/ui-specification.md) that lists them, edited in the same pull request.
- **Open items:** adds OI-9951 (the owner's acceptance of this record).

Numbering: provisional. The coordinator assigns the record number and the OI number at release, in merge order. No project schema number and no library change.

## Context

The Operations step's main button, at the bottom right, is **Generate stale (N)**: it generates every stale operation. Its key was Shift G (or Shift F5), while the plain G (or F5) generated the selected operation and the ones before it, a secondary action. The owner asked on 24 Nov 2026 for the main action to have the plain key. Asked what F5 should do, he confirmed that F5 follows G.

## Decision

1. **G and F5** generate every stale operation, as **Generate stale** does.
2. **Shift G and Shift F5** generate the selected operation and the ones before it. With no operation selected they generate every stale operation, as G did before, so the key is never dead.
3. The Project menu shows the keys beside its items ("Generate selected" Shift+G, "Generate stale" G); Help > Keyboard shortcuts lists them; the row menu's "Generate" tooltip says "(Shift G)".
4. Nothing else changes: what each action generates, the refusal of a turned-off operation, cancel with Esc, and the footer buttons and their labels.

There are no saved key bindings, so nothing is migrated. A key for "Regenerate all" (issue #559) is not part of this record; the owner has it open.

## Alternatives considered

- **Keep the keys** (no change): the owner asked for the change.
- **F5 for the selected operation** (F5 left as it was): rejected; the owner chose F5 to follow G, so the plain function key runs the main action, as F5 conventionally does.

## Consequences

- `openchuck-ui`: the key constants and their handling (`shell.rs`), the shortcuts list and the menu's shortcut texts (`shell/frame.rs`), the row menu's tooltip (`shell/tree.rs`).
- Specification section 16 and the user guide's [Keyboard and shortcuts](../../user-docs/src/reference/keyboard.md); the release notes. The tutorial and the worked examples click the **Generate stale** button and name no key, so they and their screenshots are unchanged.

## Owner check

On real hardware, because key presses on a real keyboard and window manager are what this changes:
1. Copy the baluster tutorial project, `testdata/examples/baluster/baluster-project`, and open the copy (**File > Open project**).
2. On **Operations**, click **2 Finish** in the project tree, then press <kbd>Shift</kbd>+<kbd>G</kbd>. Expect 1 Rough and 2 Finish to generate (the Jobs tab: "Generated 2").
3. Change any value of 1 Rough (for example its stepover) and press <kbd>Enter</kbd>, then click once on the 3D view, so no text field has the keyboard. Both operations are now stale. Press <kbd>G</kbd>. Expect both to generate, as **Generate stale** does.
4. Repeat steps 2 and 3 with <kbd>Shift</kbd>+<kbd>F5</kbd> and <kbd>F5</kbd>.
5. Make an operation stale as in step 3. Then click the model, **baluster.stl**, in the project tree, so that no operation is selected (clicking an empty part of the tree, or Esc, keeps the operation selected), and click the 3D view. Press <kbd>Shift</kbd>+<kbd>G</kbd>. Expect every stale operation to generate.
6. Open the **Project** menu and **Help > Keyboard shortcuts**. Expect "Generate selected" with Shift+G and "Generate stale" with G.
7. **Pass:** each step does as written. **Fail:** say which step, the key, and what happened (nothing, the other action, or an error).

## Verification

Linux cloud container, 4 vCPU, no GPU; branch `claude/lane-sim-ux-g`, rebased on `main` `361613e`.

**Tests, written first from UI-14 and the owner's answers, and watched failing against `main`'s keys** (all three failed):
- `shell::frame_tests::req_ui_14_shift_g_generates_the_selected_operation_and_g_every_stale_one` (2.9 s), the old `req_ui_14_g_generates_…` reversed;
- `shell::frame_tests::req_ui_14_f5_follows_g_and_shift_g_with_nothing_selected_generates_every_stale_one` (1.8 to 3.4 s), new;
- `shell::frame_tests::req_ui_14_shift_g_on_a_turned_off_operation_is_refused_not_a_full_run` (0.6 s), renamed.

**Suites and checks:**
- `cargo nextest run -p openchuck-ui`: 755 tests, all passed.
- `cargo fmt --all --check` and `cargo clippy --workspace --all-targets`, with no warnings.
- `cargo xtask deps`, `licences`, `records`, `trace` and `changelog check-pr`, all ok.

**Not verified:** a real keyboard and window manager (the tests send egui key events); that is the owner check above.
