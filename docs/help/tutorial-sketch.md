---
id: tutorial-sketch
title: "Tutorial: a badge in sketch mode"
group: tutorials
path: more
time: 30
level: advanced
before: tutorial-logo-by-hand
keywords: tutorial, sketch, parametric, cad, dimension, diameter, exact size, circle, polygon, hexagon, regions, badge, constraint, change the size, linked
---
# Tutorial: a badge in sketch mode

In this tutorial you draw a badge with exact sizes in sketch mode: two
circles and a hexagon. You make embroidery from the regions of the
sketch, then change a size: the stitches follow.

A sketch is a drawing with dimensions and constraints, as in a CAD
program. It sews nothing itself. See [Sketches](sketch).

## Open the start design

Click [Open the start design](sample:sketch-start). It is empty and has
three threads: navy, white and gold.

## Start a sketch

Click **Digitize > Sketch > Parametric Sketch**. The **Sketch** tab opens.
The origin of the sketch is a ring with a cross at the middle of the
hoop: points snap to it.

## Draw the circles

1. Click **Sketch > Create > Circle**.
2. Click the origin (the center), then a point 30 mm to the right of it.
3. Click the origin again, then a point 25 mm to the right of it. The two
   circles have the same center: the origin.

## Draw a hexagon

1. Click **Sketch > Create > Polygon** (click the arrow next to
   **Rectangle**, then **Polygon**). A new polygon has six sides.
2. Click the origin, then a point 12 mm to the right of it.

## Give the circles their sizes

1. Click **Sketch > Constrain > Dimension**.
2. Click the outer circle, then a place for the dimension, at the top
   right. A box shows the diameter of now.
3. Type `60` and push Enter. The circle is 60 mm across.
4. Click the inner circle, then a place at the bottom right. Type `52` and
   push Enter.
5. Push Esc to go back to **Select**.

![The sketch with its two dimensions](sketch-dimensions)

You can also type other units (`2 in`) or a calculation (`60 - 8`).

## Make embroidery from the regions

The curves make three closed regions: the ring between the circles, the
disc between the inner circle and the hexagon, and the hexagon.

1. Click **Sketch > Sketch > Regions**.
2. Click the color **Navy** in the Threads strip. Click inside the disc
   (between the hexagon and the inner circle), then **Fill** under **Make
   Embroidery** in the Properties panel.
3. Click **White**. Click inside the hexagon, then **Fill**.
4. Click **Gold**. Click inside the ring, then **Satin**.
5. Click **Sketch > Exit > Finish Sketch**.

The badge now has a navy fill, a white fill and a gold satin ring. They
sew in the order that you made them.

## Change a size

The objects stay linked to the sketch.

1. Click the gold ring. In the Properties panel, click **Edit Sketch**.
2. Right-click the dimension of the outer circle (60), and click **Change
   Value**.
3. Type `66` and push Enter. The ring is wider, and its stitches follow.
4. Click **Sketch > Exit > Finish Sketch**.

![The badge after the change, with the 3D thread look](sketch-stitches)

Undo takes back the change of the sketch and of its objects in one step.

## If it does not look right

- **The circles do not have the same center.** Click exactly on the
  origin: the pointer shows a mark when it snaps.
- **A region does not shade.** The region is not closed: the curves must
  meet or cross.
- **The new dimension is in parentheses.** The sketch sets that size
  already: the dimension is driven and changes nothing.
- **The badge does not change.** Change the sketch (Edit Sketch), not the
  objects: a made object does not scale alone.

## Next

- [Tutorial: a script that does the work](tutorial-script)
- [Sketches](sketch): constraints, parameters and more tools
