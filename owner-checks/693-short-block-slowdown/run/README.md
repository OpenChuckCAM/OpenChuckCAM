# Run output for the #693 (OI-992) check

The command-line output behind the tables in [`../CHECK.md`](../CHECK.md). Release build of `main` at `6b039e5` (0.62.0), Linux x86_64, 4 threads, 24 Nov 2026.

| File | Command |
| --- | --- |
| `regen-report.txt`, `regen-slowdown.txt` | `openchuck-cli --threads 4 regenerate project-<mode> --post makera-z1 --tool-numbers keep --grouping operation --out programs/<mode> --acknowledge-collision-warnings` (they wrote the files in `../programs/`) |
| `sim-report.txt`, `sim-slowdown.txt` | `openchuck-cli --threads 4 simulate project-<mode>` |
