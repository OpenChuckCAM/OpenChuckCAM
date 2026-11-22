# Stock and the held region

## Stock

The [stock](../glossary.md#stock) is the material you start with: on a rotary machine, usually a round bar or a square block. The program cuts the part out of the stock, so it needs to know the stock's shape, size and position along the axis. With that it knows where material is, where the cutter can move fast through air, and how much each pass removes.

In the program's Setup step you choose the stock's shape:

| Shape | Use it for |
| --- | --- |
| Cylinder | A round bar. You give its diameter and length. |
| Box | A square or rectangular block. |
| Bounding box | A box that just surrounds the part, with an allowance you set. |

The program can fill in a size that surrounds the part with a small allowance. Measure your real stock and enter its true size: if the real stock is larger than the project's, the first passes cut more than the program planned.

You also choose the stock's [material](../glossary.md#material). The tool library holds cutting data for each material, which the operations use for their speeds and feeds.

## Where things sit along X

Every position along the rotary axis in a setup is measured from one point, the machine's **rotary reference point**: X 0. The [machine profile](../reference/machine-profiles.md#rotary-axis) gives its place on the machine as **Rotary reference X**, a machine coordinate. On the Makera Z1 it is the centre of the headstock face, the face the chuck is mounted on. On a machine of your own it is wherever its profile says; the program does not know what is there.

Four values of the setup are measured from X 0, and each moves only one thing:

| Value | Where | What it moves |
| --- | --- | --- |
| **Start X** | Stock | The stock, with its held regions and tabs. |
| **Offset along X** | Part position | The part only, within the stock. **Position** shows where the part is then. |
| **Zero at X** | Work offset and clearance | The work zero: the program's X0, which you set on the machine as the work offset (G54). |
| **Rotary reference X** | Machine profile | Where X 0 is on the machine, in machine X. It is the machine's, not the setup's. |

The machine X of a setup position is **Rotary reference X** plus that position. The program uses it to check the stock, the part, the work zero and every move against the machine's X travel.

<figure>
<svg viewBox="0 0 720 300" width="720" height="300" role="img" aria-labelledby="along-t along-d" style="max-width:100%;height:auto">
<title id="along-t">Where things sit along X</title>
<desc id="along-d">A side view along the rotary axis. At the left is the headstock; its face, where the chuck is mounted, is X 0, the rotary reference point. The chuck and its jaws stand in front of the face. The stock starts at Start X, past the chuck, and the jaws grip its first length, the held region. The part sits inside the stock, moved along by Offset along X. The work zero, the program's X0, is at Zero at X. Below, a bar shows the machine's X travel, which reaches a little behind the headstock face.</desc>
<g fill="none" stroke="currentColor" stroke-width="1.5">
<rect x="20" y="60" width="70" height="140"/>
<line x1="90" y1="40" x2="90" y2="225" stroke-dasharray="4 3"/>
<rect x="90" y="85" width="60" height="90"/>
<line x1="150" y1="100" x2="190" y2="100"/><line x1="150" y1="160" x2="190" y2="160"/>
<rect x="160" y="102" width="420" height="56"/>
<path d="M160 102 l30 56 M175 102 l15 28 M160 130 l15 28" stroke-width="1"/>
<rect x="215" y="110" width="300" height="40" rx="12"/>
<line x1="10" y1="130" x2="700" y2="130" stroke-dasharray="10 4 2 4"/>
<line x1="230" y1="60" x2="230" y2="185" stroke-dasharray="2 3"/>
<line x1="90" y1="250" x2="160" y2="250"/><path d="M160 250 l-7 -4 v8 z"/>
<line x1="90" y1="270" x2="215" y2="270"/><path d="M215 270 l-7 -4 v8 z"/>
<line x1="90" y1="290" x2="230" y2="290"/><path d="M230 290 l-7 -4 v8 z"/>
<line x1="40" y1="232" x2="690" y2="232" stroke-width="4"/>
</g>
<g fill="currentColor" font-size="13" font-family="inherit">
<text x="22" y="54">Headstock</text>
<text x="96" y="34">X 0: rotary reference point</text>
<text x="96" y="80">Chuck</text>
<text x="163" y="182">Held</text>
<text x="330" y="96">Part</text>
<text x="530" y="96">Stock</text>
<text x="236" y="72">Zero at X (program X0)</text>
<text x="560" y="224">Machine X travel</text>
<text x="168" y="254">Start X</text>
<text x="223" y="274">Offset along X</text>
<text x="238" y="294">Zero at X</text>
</g>
</svg>
<figcaption><b>Figure 1.</b> Along the rotary axis. X 0 is the rotary reference point, on the Makera Z1 the headstock face. The stock starts at Start X, past the chuck; the part sits in it at Offset along X; the program's X0 is at Zero at X. The machine's X travel is a range of machine X.</figcaption>
</figure>

### A worked example on the Makera Z1

The Z1's **Rotary reference X** is −198.39 and its X travel is −207.000 to −1.000.

- A stock with **Start X** −64.770 starts at machine X −198.390 + (−64.770) = −263.160. That is 56.160 mm past the travel, and the setup checks say so: "the start of the stock, at X −64.770, is 56.160 mm past the machine's X travel …".
- Any **Start X** of −8.610 or more is inside the travel, but anything below 0 is inside the headstock. On a copy of the built-in Makera Z1 the setup checks warn about that too: "the stock starts at X −1.000, 1.000 mm inside the headstock …".
- To be clear of the chuck, the stock must start at least the chuck's length from the face. That length depends on your chuck and on how the jaws are fitted and grip, so measure it, on your machine, from the headstock face to where the jaws grip the stock. Set **Start X** to at least that, and move the part (**Offset along X**) and the work zero (**Zero at X**) with it.

The warning for a stock behind X 0 is given only on a copy of the built-in Makera Z1, whose X 0 is known to be the headstock face. On another machine's profile, X 0 may be the chuck face, where a gripped stock rightly starts behind it, so no such warning is given there: check it yourself. No check covers the chuck itself.

## The held region

The chuck grips a length at one end of the stock. That length must never be cut, and no toolpath may go there. In OpenChuckCAM it is the [held region](../glossary.md#held-region): you tick **Held at the start** and set the **Grip length**, the length the jaws cover.

If you also support the far end, for example with a [tailstock](../glossary.md#tailstock), tick **Held at the end** in the same way.

> [!WARNING]
> A held region is a length along the axis, not a solid body. OpenChuckCAM does not model the chuck, its jaws or a tailstock, and does not check them against the tool or the holder. Check these clearances yourself, on the machine, before you run a program.

## When the part comes free

When the last material joining the part to the held stock is cut, the part comes free and can fall or be caught by the cutter. If your job cuts the part free, leave [tabs](tabs.md): small bridges of material, cut through by hand afterwards. Many turned parts never come free, because one end stays joined to the held bar; the program warns you when it cannot tell.
