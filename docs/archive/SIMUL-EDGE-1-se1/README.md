# SIMUL-EDGE-1, pull request SE-1: raw evidence

Cited by [decision SIMUL-EDGE-1](../../decisions/SIMUL-EDGE-1-continuous-seed-and-whole-tool-clearance.md), *Verification (SE-1)*. Produced on 25 Nov 2026 in a Linux cloud container (4 cores, Intel Xeon @ 2.30 GHz), one run at a time.

| File | What it is |
| --- | --- |
| `baseline.txt` | Generation times before any SE change: `openchuck-cli` and `openchuck-step-worker` built in release from `main` `aadf3cc8` (0.63.0), `--threads 4`, `regenerate --post linuxcnc`, uninstrumented. One simultaneous finishing operation at the strategy's defaults per project. The first two lines are failed starts (the lobed model not yet written; the crankshaft's tools have no oak cutting data), fixed and rerun below them. |
| `time_runs.sh.txt` | The script that timed each project (wall clock, moves counted in the program). |
| `lobedgen.rs.txt` | The throwaway program that wrote the lobed reference job's model as `crates/openchuck-app/tests/reference_jobs.rs` does; the project then validated against its recorded hash. |
| `gapbench.txt` | The cost of one tool gap query (this pull request's code) on the public blisk mesh, 3 mm ball and flat end, 10,000 poses resting on the part (dropped radially), one pose and swept 0.5 mm along X, with and without `accept_above(-0.006)`. |
| `gapbench.rs.txt` | The throwaway program behind `gapbench.txt`, run from a scratch Cargo project outside the repository. |

The projects timed, all public:

- **lobed**: `testdata/projects/lobed` (the lobed reference job, 3 mm ball), its model written by `lobedgen`.
- **crank_stl**: `testdata/stl/crankshaft.stl` with `openchuck-cli new`, `import`, `machine makera-z1`, `setup --new "Setup 1" --stock cylinder=38,46,-4`, material Hardwood, the two tools of `testdata/examples/crankshaft-simultaneous`, and a simultaneous operation with `Z1 ball 3.175x22`.
- **blisk_ball, blisk_flat**: `testdata/stl/blisk.stl` set up as OI-318 and PATH-QUALITY-1 did (`--stock cylinder=42,52,-1`, oak), the two tools of `testdata/examples/blisk-start`.
- **step_ball, step_flat**: a copy of `testdata/examples/blisk-start/blisk-project` (the STEP blisk) with a simultaneous operation for each tool.

Each operation was written as a file in the project's `operations/` folder (the command line cannot add one, OI-310).
