# decision 0698, phase 1: engagement on the public crankshaft, measured in the simulation

Raw evidence for [decision 0698](../../decisions/0698-managing-tool-load.md) (its *Verification (phase 1)* section). Produced by lane TOOL-LOAD on 25 Nov 2026 at `main` `aadf3cc` (workspace 0.63.0), in a Linux cloud container with 4 cores, with nothing else running during the timed runs. Not a source of truth: where these notes and the record differ, the record holds. Generated G-code remains the user's to verify before running it on a machine.

| File | What it is |
| --- | --- |
| `README.md` | This file: how the runs were made, and their results. |
| `prototype-engagement-tally.patch` | The throwaway change to `openchuck-sim` used to measure engagement (applied to a working copy, built, run, and reverted; never committed to a branch). With `PROTO_ENG=<prefix>` set, `openchuck-cli simulate` writes one CSV per operation: for each move that removed material, the volume removed, the highest and lowest removed point along the tool axis above the tip, and the tip's travel in the part frame. |

## The projects

Built with the release command line from the repository's public STL crankshaft (`testdata/stl/crankshaft.stl`), with the same setup as the guide's example `testdata/examples/crankshaft-simultaneous` (its STEP model needs the STEP worker, which this container did not have built):

```text
openchuck-cli new P --name crank
openchuck-cli import P testdata/stl/crankshaft.stl
openchuck-cli transform P models/<file> centre
openchuck-cli machine P makera-z1
openchuck-cli setup P --new "Setup 1" --part models/<file> --stock cylinder=38,46,-4 --held-start 6 --material Hardwood
openchuck-cli tool P "Z1 O-flute 3.175x25" --add --library crates/openchuck-project/defaults/z1 --number 104
openchuck-cli tool P "Z1 ball 3.175x22" --add --library crates/openchuck-project/defaults/z1 --number 204
```

The command line cannot add an operation (OI-310), so each operation was written as a file in `operations/` (schema 27):

- `crank_fin`: one operation, *Finish*, strategy `simultaneous_finishing`, tool 204 (Z1 ball 3.175x22), no parameters (the defaults).
- `crank_rf`: *Rough*, strategy `rotary_roughing`, tool 104 (Z1 O-flute 3.175x25), `stepdown = 1.0`, then the same *Finish*.

Then `openchuck-cli regenerate P --post makera-z1 --tool-numbers keep --out P/out --acknowledge-partial-path --acknowledge-collision-warnings` and `openchuck-cli simulate P` (0.1 mm cells, the default).

## Times (wall clock, release build, one run each)

| Project | regenerate | simulate (regenerates, then simulates) | simulate with the prototype tally |
| --- | --- | --- | --- |
| `crank_fin` | 8.7 s | 20.8 s | 21.4 s |
| `crank_rf` | 41.8 s | 48.8 s | 50.7 s |

The finish alone has 19,506 moves and 258,370 simulation poses; simulating it takes about 12 s of the 20.8 s. The tally adds about 0.6 s there (about 5 % of the simulation) and about 2 s to the two operations (about 4 % of the whole run). The prototype allocates per row and per batch and was not tuned.

## What the simulation says today

`crank_fin`: material 52,171.5 mm³ to 17,510.9 mm³; the finish removes 34,660.6 mm³. `crank_rf`: the rough removes 35,274.9 mm³, the finish after it 476.0 mm³. Neither run gives a warning about the load on the tool; the warnings are the material left, the inside corners, the holder not checked on hidden-surface passes and the held region. There is no flute-length warning: the deepest stock over the ball is under its 22 mm of flute.

## The measure

For each simulation pose, the prototype adds up, over the dexel columns it cuts, the length of material removed (times the cell area: the volume) and the highest and lowest point of the removed material along the tool axis, measured from the tip. These are summed (volume, travel) or taken as the largest and smallest (heights) over the poses of each move. From them:

- **engaged length**: the highest removed point less the lowest, along the tool axis, per move;
- **cut section**: volume removed divided by the tip's travel in the part frame, taken over a sliding stretch of at least one tool diameter (3.175 mm) of travel through the moves that cut, because most of the finish's moves are shorter than 1 mm (only 208 of about 17,000 are 1 mm or longer) and a short move's volume includes the tool's own footprint where it starts.

## Results

Moves that removed material, and their engaged length (mm):

| Operation | Moves cutting | 1 mm or more | one diameter (3.175 mm) or more | 16 mm or more | Largest |
| --- | --- | --- | --- | --- | --- |
| Finish alone | 18,581 | 16,275 | 10,699 | 3,276 | 21.50 |
| Rough | 22,336 | 1,077 | 24 | 0 | 14.68 |
| Finish after Rough | 15,464 | 330 | 22 | 0 | 10.13 |

The largest engaged lengths of the rough and of the finish after it are slivers: the four longest remove 0.0 to 0.53 mm³ in their move.

Cut section over one tool diameter of travel (mm²); the tool's own "slot at its stepdown" is stepdown × diameter = 1.0 × 3.175 = 3.175 mm² for both tools in Hardwood:

| Operation | Stretches | 1 or more | 3.175 or more (1 ×) | 6.35 or more (2 ×) | 10 or more | Largest |
| --- | --- | --- | --- | --- | --- | --- |
| Finish alone | 19,183 | 14,464 | 7,944 | 4,757 | 3,263 | 185.74 |
| Rough | 23,120 | 17,287 | 1,388 | 0 | 0 | 5.32 |
| Finish after Rough | 19,171 | 34 | 0 | 0 | 0 | 1.47 |

Lane PATH-QUALITY's earlier estimate on the STEP example ([decision 0695](../../decisions/0695-finishing-edges-points-and-ramps.md), *#844*), made outside the program on 0.3 mm voxels, found 8,643 moves of the finish alone at one diameter or more and 2,771 at 16 mm or more, largest 21.5 mm, and 163 moves of the finish after roughing at 1 mm or more, largest 11.3 mm. The figures here are on the STL and on 0.1 mm cells, so they differ in the counts; the largest engaged length is the same.

## Not done

Nothing was cut or air-cut. The width of cut was not measured separately (the record's measure gives it as the section divided by the engaged length). No owner project was used. The per-move CSV files (about 5 MB) were not kept; the tables above are their summary.
