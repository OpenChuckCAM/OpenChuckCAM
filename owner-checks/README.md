# Owner check packs (milestone 2026.2)

The owner's rule of 25 Nov 2026: every issue that needs the project owner to test by hand has clear instructions and example projects, so the check can be done efficiently. Each pack below is one `CHECK.md`, written so that it can be pasted as an issue comment, with its example project and screenshots in the same folder. Links inside the packs are relative, so they work at any commit of this branch.

This branch (`owner-checks/packs`) is **not merged**. It was cut from `main` at `6b039e5` (0.62.0), and `main` at `361613e` (0.63.0) was merged into it for the #791 pack. Issues link to its files at a pushed commit. It holds only public or synthetic material. No private project of the owner's is used or needed; for #801 the owner sends numbers only.

> [!WARNING]
> Nothing here shows that any program is safe to run. Generated G-code is yours to verify before it runs on a machine. Machine steps follow the [air-cut procedure](../docs/z1-air-cut-procedure.md) and the [gates 4 and 5 instructions](../docs/z1-gates-4-5-test-instructions.md).

| Issue / item | Pack | Where | Time | What it needs from the owner |
| --- | --- | --- | --- | --- |
| #632 pencil at a sharp inside corner | [632-pencil-corner/CHECK.md](632-pencil-corner/CHECK.md) | Application only | 10 min | Pass counts off and on; expected 1 pass each |
| #801 and OI-2167 rapids that touch the stock (Subby) | [801-rapid-depth/CHECK.md](801-rapid-depth/CHECK.md) | Command line, own project | 10 min | **Numbers only** from `simulate` on the owner's own Subby copy; a public control project is included |
| #693 / OI-992 short-block slowdown | [693-short-block-slowdown/CHECK.md](693-short-block-slowdown/CHECK.md) | Application, then the Z1 (air cut); cut optional | 15 min + 1 h | Simulator comparison, then attended air cut of the Report and Slow down programs; firmware and delivery method |
| #870 refused setup value no longer locks the setup (PR #891, merged) | [870-refused-setup-value/CHECK.md](870-refused-setup-value/CHECK.md) | Application only, Windows (nightly with `c8400189`) | 5 min | Start X refused with the tab message, Workholding not locked, Enter applies after the fix, one Undo |
| #871 a dialog's own buttons never ask about an unconfirmed field (PR #891, merged) | [871-dialog-buttons/CHECK.md](871-dialog-buttons/CHECK.md) | Application only, Windows (nightly with `c8400189`) | 10 min | No question from Import model, New operation, Rename, Post and export, Open project; the inspector still asks |
| #705 undo history list (PR #904) | [705-undo-history/CHECK.md](705-undo-history/CHECK.md) | Application only, Windows (build of PR #904 until it merges) | 10 min | Title bar, Ctrl H list, jump to a step, saved mark, keyboard; Keep as one row after #891 |
| #789 show or hide each kind of toolpath move (PR #898) | [789-path-move-kinds/CHECK.md](789-path-move-kinds/CHECK.md) | Application only (build of PR #898 until it merges) | 20 min | Legend tick boxes, chip, Show and View menus, Simulate legend, restart, smoothness on a 92,000-move path |
| #791 face flow on a face that closes round (cylinder, blisk outside face) | [791-face-flow-closed-face/CHECK.md](791-face-flow-closed-face/CHECK.md) | Application only | 15 min | No refusal; 52 rings on the cylinder; then the owner's own #791 project, with numbers to report |
| PR #910: G and F5 generate every stale operation; Shift G and Shift F5 the selected one (no GitHub issue) | [pr-910-generate-keys/CHECK.md](pr-910-generate-keys/CHECK.md) | Application only, Windows (build of PR #910 until it merges) | 10 min | Each key's result in the Jobs tab; the Project menu and Help > Keyboard shortcuts |
| PR #911 (#559): Project > Regenerate all, Ctrl Shift G | [pr-911-regenerate-all/CHECK.md](pr-911-regenerate-all/CHECK.md) | Application only, Windows (build of PR #911 until it merges) | 10 min | The menu item, its key, both operations made again, the same program; the missed-change warning is covered by a test |
| Window opening on Windows (OI-9932, PR #906; no GitHub issue) | [window-opening/CHECK.md](window-opening/CHECK.md) | Application only, Windows (build of PR #906 until it merges) | 10 min | Five starts: fresh, un-maximised, maximised, small in a corner, second monitor; the two lines of settings.toml |
| OI-2152 repeat first cut (stepped cylinder) | [oi-2152-repeat-first-cut/CHECK.md](oi-2152-repeat-first-cut/CHECK.md) | Application, then the Z1 | half a day | First OI-2197 (tool 104 and its finishing preset) and the air cut of the new program, then the cut and the measurements |

**Not packed:** #577, #578 and #580 (keyboard focus, tooltips on focus, tool windows taking the keyboard; [decision 0677](../docs/decisions/0677-keyboard-focus-tooltips-and-tool-windows.md)). The owner closed them on 23 Nov 2026.

**Issues labelled `awaiting-fix-feedback`** (25 Nov 2026): #801, #791, #632, #870 and #871. #705 and #789 move to that label when their pull requests (#904, #898) merge, as will #706 when its pull request does. #693 (OI-992) and OI-2152 are open items for the owner's machine checks, not fix feedback.

## Folder layout

Each pack folder holds:

- `CHECK.md`: the pack. It covers what to check and why, the expected result, numbered steps, pass and fail, and what to report back.
- `project/` (or `project-report/` and `project-slowdown/`): the example project, ready to open at the current schema (29). How it was made is in the pack, under "How the project was made".
- `screens/`: true captures from the release build on Xvfb (Mesa lavapipe), at 1600 × 900.
- `programs/`: G-code written by the application, where the pack compares programs. SHA-256 for each is in the pack.
- `run/`: the command-line output behind the pack's figures, with a short `README.md`.

Figures were measured on `main` at `6b039e5` (0.62.0), 24 Nov 2026, except #791's (`main` at `361613e`, 0.63.0, against 0.62.0 as the before) #789's (PR #898 at `4664c82`) and #705's (PR #904 at `5d377f54`), each against `main` at `361613e` as the before, 25 Nov 2026. All runs used Linux x86_64 and a release build with the STEP reader.
