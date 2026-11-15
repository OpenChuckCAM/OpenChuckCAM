# Evidence for decision record 0649

Raw evidence cited by [decision record 0649](../../decisions/0649-the-z1-profile-takes-the-firmware-1-1-3-measurements.md) (lane PROFILE-1.1.3, issues #586 and #793): the built-in Makera Z1 takes the owner's firmware 1.1.3 measurements. The old programs are those of `main` at `90e78107`; the new ones are the golden files of the record's pull request.

| File | What it is |
| --- | --- |
| `zdiff.py` | The golden diff check: for each changed golden file, compares the old and new text line by line and allows only a Z word of 40.000 that becomes 45.000 (the default safe Z and safe cylinder), an estimate in an operation's comment, a setup-sheet line about the clearance, work offset or time, a rounding tie of the layout shift (one X word 0.001 mm apart, or written or left out as modal, and the F word that follows), and in the first-cut programs a rapid link height between roughing layers, printed for review. Exits non-zero on any other change. Run from the repository root: `python3 docs/archive/0649-z1-firmware-1-1-3/zdiff.py 90e78107`. |
| `zdiff-output.txt` | Its output for this change: no other change in any file. |
| `tie-lines.txt` | The line numbers of the rounding ties in the Z1 programs, for the owner's spot check. |

The first-cut link heights (`G0 Z21.334` to `Z21.599` and `Z23.355` to `Z23.356`) were traced to the 45 mm safe cylinder: the same project with the layout shift and the clearance set back to 40 writes the old values, and without the shift but with 45 the new ones (the lane's run of `openchuck-cli regenerate`, 14 Nov 2026).

Generated G-code is the user's to verify before it is run on a machine (NFR-12); these checks compare programs with programs and do not verify them.
