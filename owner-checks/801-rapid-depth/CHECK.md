# Owner check for #801 and OI-2167: rapids that touch the stock (Subby: Periscope and Propeller)

**For:** the project owner, on your own computer, with your own Subby project. **Time:** about 10 minutes. **Machine:** none.

> [!IMPORTANT]
> **Numbers only.** Run the command on your own copy of the Subby project and send back only the numbers asked for below. Do not attach, commit or share the project. Nobody on the project side opens the project attached to #801.

> [!WARNING]
> A rapid listed as "within the simulation's cell" is a statement about what the simulation can resolve. It does not mean the move is safe. Generated G-code is yours to verify before it runs on a machine.

## What changed, and what to check

#801 (nightly 0.44.0) failed `openchuck-cli simulate` on seven rapid moves:

- **Periscope** (axial finishing, tapered ball r0.5, 5°): move 1, 0.016 mm³, and move 289, 0.001 mm³.
- **Propeller** (simultaneous finishing, 3 mm ball): moves 1045, 1600, 6011, 13423 and 14133, each **0.000 mm³**.

The rule is now [decision 0661](../../docs/decisions/0661-a-rapid-that-removes-less-than-the-simulation-resolves.md), option C, built in 0.57.0. Every rapid that touches the stock is listed **with its depth**, the deepest it went into any one column of the simulated stock.

- **No deeper than one cell** (0.1 mm at the default cell): it is information. It does not fail `simulate` and asks for no acknowledgement.
- **Deeper than one cell:** it fails `simulate`, as before, and is a warning to acknowledge before export.
- A volume under 0.0005 mm³ prints as "less than 0.001 mm³" in the application. The command line prints four decimals.

**OI-2167 is a separate question.** Does the Periscope's first approach still come down into stock that roughing left on the periscope? At 0.016 mm³ it was about 1.6 mm deep in one column, so if it is still there it stays a failure. That would be a toolpath problem, apart from the reporting change.

| Check | Expected result |
| --- | --- |
| Propeller's five rapids | Either gone, or listed "within the 0.100 mm cell", and **not counted** as problems |
| Periscope move 289 (0.001 mm³) | Probably within the cell: listed, not counted |
| Periscope move 1 (OI-2167) | **Unknown.** Gone, within the cell, or still deeper than the cell. The answer decides whether OI-2167 needs a toolpath fix. |
| `simulate`'s last line | Fails only if a rapid is deeper than the cell, or the stock is overcut |

## The example project (public companion)

[`project/`](project/) is the public baluster (`testdata/examples/baluster/`) with the same kinds of operations as Subby's Periscope and Propeller:

- **Rough**: rotary roughing, flat end 104, 0.3 mm left;
- **Axial finish**: axial finishing, ball 204 (3.175 mm), defaults;
- **Simultaneous finish**: simultaneous finishing, ball 204, defaults.

It is the run [decision 0661](../../docs/decisions/0661-a-rapid-that-removes-less-than-the-simulation-resolves.md) used. It has **no rapid that touches the stock**, so it is a control. Run it first to see that your build simulates this sequence cleanly, and how the output reads. It does **not** reproduce #801: only your Subby project does. Its output from here is in [`run/simulate.txt`](run/simulate.txt).

<details><summary>How the project was made (to regenerate it)</summary>

From a checkout of `main` at `6b039e5` or later, with `openchuck-cli` on the path:

```sh
P=owner-checks/801-rapid-depth/project
cp -r testdata/examples/baluster/baluster-project $P
rm $P/operations/34e3aa5e-36cc-4614-be6d-ec39cf22f5f8.toml      # the example's helical Finish
# add the two operation files as committed here:
#   operations/0c0c0801-0000-4000-8000-000000000302.toml  (axial_finishing, tool c0de0001-...-000000000204, order W)
#   operations/0c0c0801-0000-4000-8000-000000000303.toml  (simultaneous_finishing, the same tool, order X)
# and set the project's name to "Owner check 801: rapid depth (public baluster)"
openchuck-cli resave $P
```

The command line has no command that adds an operation, so the two operation files are committed as written (each is ten lines).

</details>

## Steps

1. Install a nightly of **24 Nov 2026 or later** (0.57.0 or later has the change; the figures here are from 0.62.0, `main` at `6b039e5`). Note its version: `openchuck-cli --version`.
2. **Control:** run the companion project:
   ```sh
   openchuck-cli simulate owner-checks/801-rapid-depth/project
   ```
   It takes about 40 s. Expect `simulated in …`, no `rapid move` lines, and exit status 0. The three operation lines should read about:
   ```text
       operation 'Rough': 9946 moves, 102664 poses, removed 12099.6 mm3
       operation 'Axial finish': 58089 moves, 644504 poses, removed 2175.7 mm3
       operation 'Simultaneous finish': 18484 moves, 112409 poses, removed 10.2 mm3
   ```
3. **Your project:** make a copy of your Subby project folder. The program may ask to save it at the current schema, so work on the copy. Run:
   ```sh
   openchuck-cli simulate <copy of your Subby project>
   ```
   Do not pass `--cell`: it saves the cell into the project. The default cell is 0.1 mm.
4. In the output, find the lines under `operation 'Periscope'` and `operation 'Propeller'`. A rapid that touches the stock now reads like this (numbers made up):
   ```text
       operation 'Periscope': N moves, N poses, removed V mm3
         rapid move 1 goes 1.234 mm into the stock and removes 0.0160 mm3, ending at (68.192, 0.000, 17.464)
         rapid move 289 goes 0.045 mm into the stock, within the 0.100 mm cell and removes 0.0010 mm3, ending at (...)
   ```
   - With no `, within the … cell` after the depth, the rapid is a problem.
   - With it, the rapid is listed only.
5. Read the last line: `simulated in …` (pass), or `error: N problem(s) found in the simulation above`.
6. Optional, in the application: open the copy, run **Simulate**, and look at **Messages**. A deeper rapid reads "Rapid cut (operation 'Periscope'): move 1 is a rapid that goes D mm into the stock, deeper than the simulation's cell of 0.100 mm, and removes V mm³ of material", and is a warning to acknowledge. A shallower one reads "… touches the stock D mm deep, within the simulation's cell of 0.100 mm …" and is information.

## Pass and fail

- **#801, the reporting part, passes** when the Propeller's rapids (and any other rapid that removes less than the cell resolves) are either gone or listed `within the 0.100 mm cell` and are not counted in `problem(s)`.
- **OI-2167 (Periscope move 1):**
  - gone or within the cell: OI-2167 can be closed;
  - still deeper than the cell: OI-2167 stays open as a toolpath item. The approach should stop above the stock roughing left. Your numbers then say how deep and where, with no project needed.
- **Fail (reporting):** a rapid listed `within the … cell` still counted as a problem, or a rapid printed with no depth.

## What to report back

Paste this into #801, filled in. Numbers only: no project, no files.

```text
#801 / OI-2167 rapid check
Build: openchuck-cli --version =
Control (public baluster): exit status = ___ ; rapid move lines = ___ (expected none)

Subby (my copy, default cell 0.1 mm):
Last line: simulated / ___ problem(s)
Periscope rapids:  move | depth mm | within cell? | volume mm3 | end (x, y, z)
                   ...
Propeller rapids:  move | depth mm | within cell? | volume mm3 | end (x, y, z)
                   ...
Any other operation's rapid lines:
Overcut lines (if any):
```

If the Periscope's move 1 still fails, please add the Periscope operation's strategy settings that differ from the defaults (stepover, stock to leave, entry and links), as numbers only. That is enough to try to rebuild the case on a public part.
