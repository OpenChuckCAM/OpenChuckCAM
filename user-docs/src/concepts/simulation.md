# Simulation and deviation colours

The simulation cuts a model of your stock with every operation of the setup, in order, and then compares what is left with the part. It shows what the program will remove before anything is cut, and where it leaves too much material or cuts too deep. It is a check on the toolpaths, not a test of the program on your machine: generated G-code must still be verified by you before it is run on a machine.

## How the stock is modelled

The stock is modelled as a dense grid of thin columns, like a bed of nails, each column cut short where the tool passes. The width of a column is the **Cell size**, a setting of the project, 0.1 mm unless you change it. It can be from 0.025 mm to 1 mm.

The cell size sets how fine the simulation is. The deviation it reports is accurate to about one cell. A smaller cell takes much longer and uses much more memory: half the cell takes about eight times the time and four times the memory. Change it on the Simulate step, under **RESOLUTION**; the line under it says how much time and memory the new value costs.

Later operations in a setup are generated against the stock the earlier ones left: rest machining finds its material this way, and the flute-length check measures it. So the cell size also changes the toolpaths of those operations, and changing it makes the simulation and those operations out of date.

**Skip stock simulation**, in the Playback tab, makes only the toolpaths, which is quicker. There is then no cut part, no deviation and no check of the stock before posting, and the export asks you to confirm that you know.

## Deviation colours

On the Simulate step, tick **Deviation colours** in the Inspector. The cut part is then coloured by how far each point is from the part's surface:

| Colour | Means |
| --- | --- |
| Red-orange | Overcut: cut deeper than the part's surface, by more than the simulation's own error |
| Grey | On the surface, within the tolerance shown under the scale |
| Ice blue | Up to 0.25 mm left |
| Sky blue | 0.25 to 1 mm left |
| Strong blue | More than 1 mm left |
| Violet, cross-hatched | Unreachable (enclosed): material no tool on this machine can reach |
| Dark slate, hatched | Not measured: nothing was sampled there, or it is a closed gap too narrow to read (see below) |

The scale on the view shows the bands from **Overcut** through **On** to **Left > 1 mm**. **Colour by** chooses **Deviation** (both), **Remaining** (only material left) or **Overcut** (only overcut). Hold the mouse still over the part to read the value at that point.

The colours are drawn on the cut part from the nearest point the comparison measured. Tick **Part** in the view's **Show** to see them on the part's own model, which is finer.

## The results

Under **SIMULATION RESULTS**, for each setup:

Largest overcut
: The deepest cut into the part. "within ±0.1 mm" means it is inside the tolerance and is not counted as an overcut.

Overcut samples
: How many measured points are overcut, and in how many places.

Largest material left
: The most material standing above the part's surface.

Within ±... mm
: The share of the surface within the tolerance.

Unreachable (enclosed)
: The share of the surface no tool can reach, because the part closes it off across the rotary axis, such as a bore along the axis. Remove it another way, for example by drilling.

Left, not classified
: Material left that is not shown to be unreachable.

A hairline slit or fold the model closes (a gap under about two simulation cells thick, 0.2 mm at the default cell) is not enclosed material: the simulation cannot say how much is left in it. It is left out of all the numbers above and the distribution, drawn as **Not measured**, and a note in **Messages** (topic **Mesh check**) says how many places, how much surface and where, as X from the work zero and a share of the part's length. The note is a warning when they are one percent or more of the surface. Fix it in the model: a real cavity two cells or more across still counts as enclosed.

**DEVIATION DISTRIBUTION** splits the surface into bands, with how many points fall in each; untick a band to hide it on the part, or type a range in **Show only from** and **to**. **BY OPERATION** lists each operation's moves and the volume it removed, and says **Clean**, or how many rapid moves cut deeper than one cell into the stock: such a rapid move is a fault in the toolpath.

## Tool load and removal rate

As it cuts the stock, the simulation measures how much material each move of the tool removes, so you can see how hard the tool works before you run the program:

Cut section
: The material removed per millimetre the tool travels, in mm², over a stretch of one tool diameter of travel: about the depth of the cut times its width. This is what loads the tool.

Engaged length
: How much of the cutting edge is in the material, measured along the tool. A long engaged length on a thin sliver is a light cut; the cut section tells them apart.

Removal rate
: The material removed per minute, in mm³/min. **Average** is over the cutting time, the time of the moves that remove material; rapids and moves in air are left out. **Highest** is the highest over any stretch of one tool diameter.

Each operation is measured against its **Largest cut section**. Unless you set one on the operation, it is twice a full-width slot at the stepdown in the tool's cutting data for the stock material (for a 3.175 mm tool with a 1 mm stepdown, 6.35 mm²); with no cutting data, the tool's diameter squared. Where stretches of the path remove more, the operation gets a **Tool load** warning with the heaviest cut, where it is and how much of the flute was in the material. Rough the setup first, use **Finishing layers** where the operation has them, take a smaller stepdown or stepover, or, if you judge the load acceptable for your tool and machine, set a larger **Largest cut section** on the operation. Radial drilling is measured but not warned of: a drill cuts on its end by design.

Under **BY OPERATION**, **REMOVAL RATE, MM³/MIN** lists each operation's **Average** and **Highest** removal rate and its heaviest cut (**Load**), and the setup's own figures in **This setup**. Rest the pointer on a figure for what it means and the move it is at. The **Results** tab gives the same for each operation as information, and one line for the whole job.

The limit is a nominal value for a warning, not a measured limit for your tool and machine. A warning says the cut is heavy; no warning does not say the cut is acceptable. With **Skip stock simulation** on, nothing is measured, and the **Results** tab says so.

## Reading the result

- Grey where the part is finished, and blue where a later operation will cut, is what you want to see.
- Red is an overcut: find it, and change the operation, the tool or the tolerance. The simulation measures to about one cell, so a value around the cell size is at the limit of what it can tell.
- Blue that no later operation removes is material left on the finished part. Inside corners sharper than the tool's radius always keep some.
- A rapid cut is a rapid move that goes deeper into the stock than one cell. It is a fault: do not post the program until it is gone. It is a possible collision, so the export also asks you to acknowledge it before it writes anything.
- A rapid move that touches the stock no deeper than one cell is within what the simulation can tell apart: it is listed in the **Results** tab as information, with how deep it went and the volume, and is not counted as a rapid cut. A finer cell shows smaller contacts. Look at it in the playback all the same: the simulation does not say that the move is safe.

The simulation does not model the chuck, a tailstock, the tool holder or the machine, and it does not check them against the part. See [Known limitations](../start/limitations.md).

## Playing the cut

The **Playback** tab plays the program: step through it move by move or operation by operation, play it at up to 10 000 times its speed, or set it to play the whole program in 10 s or 60 s. **Go to move** jumps to a move. **Turn part with A** turns the part, as the machine does, instead of the tool. Drag under the timeline to set a window of moves to play and show.

The time shown is the programmed time, the moves at their programmed feeds; it is not an estimate of how long the machine takes.
