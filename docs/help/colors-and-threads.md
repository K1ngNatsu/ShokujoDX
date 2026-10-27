---
id: colors-and-threads
title: Colors and threads
group: production
keywords: color, colour, thread, chart, brand, madeira, isacord, sulky, colorway, background, fabric color, palette, match, change color, wheel, complementary, cycle, split thread, remove unused, import chart, gpl, csv, thread use, bobbin, cost, spool
---
# Colors and threads

## The Threads panel

**Color > Threads** shows the Threads panel.

- **Design threads** lists the threads of the design. Click the color of
  a thread to change it. Click its name to make it the thread of new
  objects.
- To give an object another thread, select the object and click the
  thread. Or right-click the object and use **Thread**. Object Properties
  has the thread too.
- **Add thread** adds a thread.
- **Remove Unused** removes the threads that no object uses.
- **Cycle Colors**: each used thread takes the color of the next one. Do
  it again to try more combinations.
- **Split Thread**: gives the selection its own thread, so its color can
  change alone.

## Thread charts

A thread chart is the list of colors of a thread brand. The program has
the charts of 75 brands and kinds of thread (Brother, Madeira, Isacord,
Sulky, Robison-Anton, Gunold, Marathon, Floriani and more).

- Select a chart in **Color > Chart**, or in the Threads panel.
- **Search** finds a thread by its name or code.
- Click a chart thread to give its color and name to the current thread.
  **Ctrl+click** adds it as a new thread.
- **Match to Chart** gives each used thread the nearest color of the
  chart.
- **Import Chart** reads a chart from a GIMP palette (`.gpl`) or a CSV
  file (`code,name,#rrggbb`). The chart stays for the next start.

Picture to design can take its colors from the chart: see [Pictures and
Picture to design](pictures).

## Colorways

A colorway is one set of colors for the design. Make more colorways to
sew the same design in other colors, for example for fabrics of other
colors.

- **New Colorway** makes a copy of the current colors, to change.
- **Current** (Color tab) or a click in the Threads panel shows a
  colorway.
- **Delete Colorway** deletes the current colorway. The arrows in the
  Threads panel change the order of the colorways.
- The fabric color is part of the colorway.

Export and Output design write the current colorway.

## The fabric color

**Color > Fabric Color** sets the **Background**: the color of the fabric
inside the hoop. The canvas, the thread look and the pictures show the
fabric in that color. **No Background** removes it. The 3D thread look
shows the weave of the fabric of the design (Design > Fabric) under the
color.

## The color wheel

Open **Color wheel** in the Threads panel. It shows the colors of the
design on a wheel. Click a dot to make it the current thread. Under the
wheel are colors that go with the current thread (the opposite color, the
colors next to it, and others), as threads of the chart. Click one to use
it for the current thread; Ctrl+click adds it as a new thread.

## How much thread a design uses

**Design Info** shows the top thread of each color and the bobbin thread
of the design. The worksheet shows them too. The numbers are an estimate
from the stitches and the thickness of the fabric of the design (Design >
Fabric). The tension, the thread and the fabric change them.

To see the cost of the thread, set the length and the price of a spool
and of a bobbin, and the currency, in **Options**. A price of 0 shows no
cost.

## Tips

- Formats such as DST have no colors. The machine shows its own colors.
  **Output Design** can write thread color files next to the machine
  file. See [Files and output](files-and-output).
- Keep the objects of one color together in the sew order, so the
  machine changes the thread less often. See [Arranging and the sew
  order](arranging).
