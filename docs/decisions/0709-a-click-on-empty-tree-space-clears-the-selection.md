# 0709: A click on empty space in the project panel clears the selection (UI-06, UI-14; lane UI-FOLLOWUPS)

- **Status:** Proposed by lane UI-FOLLOWUPS on 28 Nov 2026. It is built to the project owner's answers of 27 Nov 2026, relayed by the coordinator (decision 0049): Q1 A ("Keep Esc consistent for now": only a click on empty tree space clears the selection) and Q2 A (with nothing selected, the inspector shows what it shows when a project opens). The record itself waits for the owner's acceptance (OI-2280).
- **Date:** 28 Nov 2026.
- **Requirements:** UI-06 (the project tree), UI-14 (keyboard access; unchanged here, see the question in OI-2280).
- **Amends:** [specification](../ui-design/ui-specification.md) section 6 (a new paragraph, *Clearing the selection*) and section 16 (a *Selection* line), edited in the same pull request. Step 5 of the owner check in [decision 0707](0707-the-plain-key-generates-every-stale-operation.md) says a click on empty tree space keeps the operation selected; that is no longer so, and that record's check is not repeated.
- **Open items:** adds OI-2280 (the owner's acceptance of this record, and the keyboard question).

Numbering: decision 0709 and OI-2280, the next free across `main` and the unmerged branches on 28 Nov 2026; the coordinator confirms them at release. No project schema number and no library change.

## Context

Issue #925 (the owner's request of 25 Nov 2026). Once an operation was selected, the only ways to have nothing selected were to choose another item, delete the operation or change step. Esc and a click on empty space left the selection as it was. With nothing selected, Shift G generates every stale operation ([decision 0707](0707-the-plain-key-generates-every-stale-operation.md)), and owner checks had to tell the user to click the model to deselect.

The lane asked two questions (relayed by the coordinator):
- **Q1:** what clears the selection? A: a click on empty tree space only; B: that, and Esc as the last step of its order; C: B, and a click on the view background. The owner chose A.
- **Q2:** with nothing selected, what does the inspector show? A: what it shows when a project opens; B: nothing. The owner chose A.

## Decision

1. A click on the empty space of the project panel, below its last row, clears the selection: no model, setup or operation is selected, and the inspector shows what it shows when a project opens ("Choose an item in the project panel to see it here.").
2. It does not apply to:
   - a click on a row, a group header or its "+", a row's own controls (drag handle, enable box, eye) or the scroll bar, which do what they do there;
   - picking, or a shown selection set, which keep their own ways out (Esc, another step).
3. While a context menu is open, or a text field has the keyboard, the click only closes the menu or ends the edit.
4. Esc is unchanged and does not clear the selection. There is no click on the view background.

Clearing the selection is part of the same interface step as choosing a row. If the inspector holds a typed value not yet applied, it asks first, as it does for choosing another row.

## Alternatives considered

- **Esc as well (Q1 B):** the lane's recommendation. The owner chose to keep Esc's jobs as they are for now.
- **A click on the view background (Q1 C):** rejected. Such a click is easy to make by accident while turning the view.
- **An empty inspector (Q2 B):** rejected in favour of the screen a project opens on, so no new screen is needed.

## Consequences

- `openchuck-ui`:
  - `shell/tree.rs`: the empty space below the scroll area takes a click.
  - `shell.rs`: `TreeAction::ClearSelection`.
- Specification sections 6 and 16, the user guide's [Keyboard and shortcuts](../../user-docs/src/reference/keyboard.md), and the release notes.
- The look of no screen changes, so the tutorial screenshots stay as they are.

## Owner check

On real hardware:
1. Open any project with an operation (for example a copy of `testdata/examples/baluster/baluster-project`).
2. On **Operations**, click an operation in the project panel. Then click the empty part of the panel below the last row. Expect no row tinted, and the inspector reading "Choose an item in the project panel to see it here."
3. Click the model row, then the empty space again. Expect the same.
4. Select an operation, then right-click it so its menu opens, and click the empty space. Expect the menu to close and the operation to stay selected.
5. Select an operation and press <kbd>Esc</kbd>. Expect the operation to stay selected.
6. **Pass:** each step does as written. **Fail:** say which step, and what happened.

## Verification

Linux cloud container, 4 vCPU, no GPU; branch `claude/lane-ui-followups-925-clear-selection`, from `main` `f906fa5f`.

**Tests, written first from the owner's answers, in `apps/openchuck-ui/src/shell/tree_selection_tests.rs`:**
- `req_ui_06_a_click_on_empty_tree_space_clears_the_selected_operation`: an operation, then a click on its row still selects it.
- `req_ui_06_a_click_on_empty_tree_space_clears_a_selected_model_or_setup`.
- `req_ui_06_while_a_context_menu_is_open_the_click_only_closes_it`.
- `req_ui_06_req_ui_14_while_a_text_field_has_the_keyboard_the_click_only_ends_the_edit`.
- `req_ui_06_req_ui_05_picking_keeps_its_own_way_out`.
- `req_ui_14_esc_does_not_clear_the_selection`: a guard of the unchanged Esc, so it passed before the change as well.

The first four failed against `main`. The menu, text-field and picking tests each failed with their guard taken out, then passed with it back. Each test takes under a second uninstrumented.

**Suites and checks:**
- `cargo nextest run -p openchuck-ui`: 832 tests, all passed (before the picking test was added; that test then passed with the other five).
- `cargo clippy --workspace --all-targets`, with no warnings after the last commit.
- `cargo xtask deps`, `licences`, `records`, `trace` and `changelog check-pr`, all ok.
- Local coverage of these tests: every new line runs except the closing brace for a panel the tree fills, which has no empty space.

**Not verified:** a real mouse and window manager (the tests send egui pointer events); that is the owner check above.
