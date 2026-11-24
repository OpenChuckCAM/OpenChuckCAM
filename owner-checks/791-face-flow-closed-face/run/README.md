# Run output for the #791 check

The command-line output behind the "As measured here" table in [`../CHECK.md`](../CHECK.md), 25 Nov 2026, Linux x86_64, 4 threads, release builds with the STEP reader: "before" is `main` at `6b039e5` (0.62.0), "new" is `main` at `361613e` (0.63.0). Project folders are shown by name. The `-new-defaults` runs use a copy of the project whose operation also has the settings a new face-flow operation starts with (`min_length = "tool_diameter"`, `pattern = "automatic"`, `ring_step = "blended"`, `stay_down = true`; stepover 1 mm).

| File | Command |
| --- | --- |
| `regenerate-<project>-before.txt`, `regenerate-<project>-new.txt`, `regenerate-<project>-new-defaults.txt` | `openchuck-cli --threads 4 regenerate <project> --post makera-z1 --tool-numbers keep --grouping operation --out OUT` |
| `simulate-cylinder-new.txt`, `simulate-cylinder-new-defaults.txt` | `openchuck-cli --threads 4 simulate <project>` |
