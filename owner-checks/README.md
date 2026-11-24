# Owner check packs (milestone 2026.2)

The owner's rule of 25 Nov 2026: every issue that needs the project owner to test by hand has clear instructions and example projects, so the check can be done efficiently. Each pack below is one `CHECK.md`, written so that it can be pasted as an issue comment, with its example project and screenshots in the same folder. Links inside the packs are relative, so they work at any commit of this branch.

This branch (`owner-checks/packs`) is **not merged**. It was cut from `main` at `6b039e5` (0.62.0), and issues link to its files at a pushed commit. It holds only public or synthetic material. No private project of the owner's is used or needed; for #801 the owner sends numbers only.

> [!WARNING]
> Nothing here shows that any program is safe to run. Generated G-code is yours to verify before it runs on a machine. Machine steps follow the [air-cut procedure](../docs/z1-air-cut-procedure.md) and the [gates 4 and 5 instructions](../docs/z1-gates-4-5-test-instructions.md).

| Issue / item | Pack | Where | Time | What it needs from the owner |
| --- | --- | --- | --- | --- |
| #632 pencil at a sharp inside corner | [632-pencil-corner/CHECK.md](632-pencil-corner/CHECK.md) | Application only | 10 min | Pass counts off and on; expected 1 pass each |
| #801 and OI-2167 rapids that touch the stock (Subby) | [801-rapid-depth/CHECK.md](801-rapid-depth/CHECK.md) | Command line, own project | 10 min | **Numbers only** from `simulate` on the owner's own Subby copy; a public control project is included |
| #693 / OI-992 short-block slowdown | [693-short-block-slowdown/CHECK.md](693-short-block-slowdown/CHECK.md) | Application, then the Z1 (air cut); cut optional | 15 min + 1 h | Simulator comparison, then attended air cut of the Report and Slow down programs; firmware and delivery method |
| OI-2152 repeat first cut (stepped cylinder) | [oi-2152-repeat-first-cut/CHECK.md](oi-2152-repeat-first-cut/CHECK.md) | Application, then the Z1 | half a day | First OI-2197 (tool 104 and its finishing preset) and the air cut of the new program, then the cut and the measurements |

**Not packed:** #577, #578 and #580 (keyboard focus, tooltips on focus, tool windows taking the keyboard; [decision 0677](../docs/decisions/0677-keyboard-focus-tooltips-and-tool-windows.md)). The owner closed them on 23 Nov 2026.

**Other issues awaiting the owner's feedback:** on 24 Nov 2026 the only open issue labelled `awaiting-fix-feedback` was #801.

## Folder layout

Each pack folder holds:

- `CHECK.md`: the pack. It covers what to check and why, the expected result, numbered steps, pass and fail, and what to report back.
- `project/` (or `project-report/` and `project-slowdown/`): the example project, ready to open at the current schema (29). How it was made is in the pack, under "How the project was made".
- `screens/`: true captures from the release build on Xvfb (Mesa lavapipe), at 1600 × 900.
- `programs/`: G-code written by the application, where the pack compares programs. SHA-256 for each is in the pack.
- `run/`: the command-line output behind the pack's figures, with a short `README.md`.

All figures were measured on `main` at `6b039e5` (0.62.0), Linux x86_64, release build with the STEP reader, 24 Nov 2026.
