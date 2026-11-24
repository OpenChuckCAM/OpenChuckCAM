# Run output for the #632 check

The command-line output behind the "Expected result, as measured here" table in [`../CHECK.md`](../CHECK.md). Release build of `main` at `6b039e5` (0.62.0), Linux x86_64, with the STEP reader (`openchuck-step-worker`), 4 threads, 24 Nov 2026. The project folder is shown as `project`. "on" is a copy of the project with `exact_contact = true` on the pencil operation.

| File | Command |
| --- | --- |
| `off.txt`, `on.txt` | `openchuck-cli --threads 4 regenerate project --post makera-z1 --tool-numbers keep --grouping operation --out OUT` |
| `sim-off.txt`, `sim-on.txt` | `openchuck-cli --threads 4 simulate project` |
