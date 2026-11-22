# Evidence for decision record 0653

Raw evidence cited by [decision record 0653](../../decisions/0653-verification-report-for-release-0-55-0.md) (lane VERIFY-REPORT): the verification report for release 0.55.0, built from the 0.55.0-rc3 test record on the Makera Z1.

| File | What it is |
| --- | --- |
| `rc3-test-record-part-1.md` | Part 1 of 3 of the rc3 test record: common conditions, the rc3 build, what carries over from rc2, Part A (before the machine, A-01 to A-18), Part B (Z1 spot checks, B-01 to B-08) and Part C (first-cut air cut and OI-993, C-01 to C-07). Verbatim copy of [issue #586 comment 7072055765](https://github.com/OpenChuckCAM/OpenChuckCAM/issues/586#issuecomment-7072055765), posted 22 Nov 2026. |
| `rc3-test-record-part-2.md` | Part 2 of 3: Part C2, the A-axis alignment check (dial indicator C2-01 to C2-08, the probe sequence C2P-00 to C2P-07 with the controller console logs, the calculation and the G54 verification). Verbatim copy of [comment 7072070836](https://github.com/OpenChuckCAM/OpenChuckCAM/issues/586#issuecomment-7072070836). |
| `rc3-test-record-part-3.md` | Part 3 of 3: Part D, gate 5 first cut (E-01 to E-30, the E-26 measurements, the analysis and X0 finding, D-01 to D-05), the stop rules, the defect table and the results table with the sign-off. Verbatim copy of [comment 7072079025](https://github.com/OpenChuckCAM/OpenChuckCAM/issues/586#issuecomment-7072079025). |

The files were copied from the comment bodies as returned by the GitHub API on 22 Nov 2026, with no edits. The diagrams and simulation screenshots of the working record were not posted to the issue and are not here. The record covers build 0.55.0-rc3, commit `aa73540022c3994e2cf27e350cff6fb47713672b`; this folder was added on `claude/lane-verify-report` from `main` at `37211de9`.

Generated G-code is the user's to verify before it is run on a machine (NFR-12). These records state what was observed on one machine and do not verify any program as safe.
