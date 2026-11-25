# 8. Generate the toolpaths

1. Press **Generate stale (2)** at the bottom right. "Stale" means the operation has no up-to-date [toolpath](../glossary.md#toolpath).
2. Wait until the status bar reads **Ready** again. **1 Rough** and **2 Finish** show a green dot in the project tree: they are up to date.

The status bar shows the progress. On this part it takes a few minutes on a computer with no graphics card, and less with one.

<figure>
  <img src="images/13-generated.png" width="1920" height="1080"
       alt="The toolpath of the Rough operation drawn over the part in the front view, with the Finish path hidden by its eye icon.">
  <figcaption><b>Figure 1.</b> The roughing toolpath in orange in the front view, with the held region hatched on the left. The Finish path is hidden.</figcaption>
</figure>

The orange lines are the roughing toolpath in the front view, with the held region hatched on the left. The eye icon beside an operation shows or hides its path in the view; hiding a path changes only the view, never what is machined.

The legend at the top left of the view names the kinds of move by colour: **Cut**, **Link**, **Rapid**, and **Entry and exit** with its three kinds, **Plunge**, **Lead-in and ramp** and **Lead-out**. Each has a tick box: untick one to hide those moves, for example the rapids while you look at the lead-ins. The same choices are in the view's **Show** menu (<kbd>V</kbd>) under **Toolpath moves**, and in **View > Toolpath moves**. On the Simulate step the legend of moves is at the top left of the view while **Toolpaths** is ticked in **Show**. While a kind is hidden, a label beside **Show** says which, such as "Hidden: Rapid". To show every kind again, choose **Show all moves** in **Show** or in **View > Toolpath moves**. The choice is yours and is kept on this computer for every project. It changes only what the view draws: the G-code, the simulation and the checks are the same, and in playback the tool still moves along the hidden moves.

Use the view buttons (**Front**, **Top**, **Iso**, **Fit**) and the mouse to look around.

The Messages list under the view holds two warnings: the tool-length warning from [step 7](07-operations.md) and the setup's grip warning from [step 5](05-hold.md), so the project chip still reads **Needs attention**. Click a message to see what it is about. The **Results** tab also has a note on **2 Finish**: no part of the tool comes within 0.500 mm of the held region, so the surface right beside the grip is not cut. That is what this tutorial means to do: the grip leaves the plain end uncut.

Press **Continue to simulate**.
