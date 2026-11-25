---
id: stitch-types
title: Stitch types
group: design
keywords: stitch type, outline type, fill type, column type, satin, zigzag, e stitch, blanket, raised, tatami, square, coil, contour, string, motif, program split, flexi split, user split, offset, spiral, island coil, radial, maze, florentine, guide line, stipple, cross stitch, laydown, tartan, plaid, sett, gradient, meander, circular, guided, ripple, mesh, motifs, run types, backstitch, stem stitch, sculpture, blends, borders, buttonholes, carving, stamp, lean, bands, column bands, band overlap
---
# Stitch types

The stitch type sets the pattern of the stitches of an object. Set it in
Object Properties, or for many objects at once with **Edit > Stitch Types**:

- **Outline Type**: the selected lines sew as a plain line, or as a
  column of a stitch type along the line (with a width).
- **Fill Type**: the stitch type of the selected fills.
- **Column Type**: the stitch type of the selected satin columns and of
  lines with a width.

Each change is one undo step. Put the pointer on a type in the list to
see what it does.

## Lines

The run type (property bar and Object Properties):

| Run type | What it sews |
|---|---|
| Run | Even stitches along the line. |
| Backstitch | Two half steps forward, one back, like a hand backstitch. |
| Stem stitch | A backstitch with the forward stitches a little to one side, like a hand stem stitch. |
| Sculpture run | Triple and single stitches in turn, so the line looks like a string of beads. |
| Manual | The points of the line are the needle points. |

**Triple** sews each stitch three times (a bean stitch), for a heavier
line. The outline types of a line are also:

- **Motif run**: a motif repeated along the line. Set the **Motif
  length**, **Motif height** and **Gap**.
- **Ripple**: copies of the line that get smaller round a point, or move
  along a guide line. Set the number of **Lines**, the **Size of the last
  line** and the **Point**. To add a guide, select the line and a guide
  line, then **Edit > Stitch Types > Ripple Guide**.
- A column type (see below): the line sews as a column of a set width,
  with the line as its middle.

## Columns

| Column type | What it sews |
|---|---|
| Satin | Smooth stitches from side to side. For borders and lettering. |
| Zigzag | Open satin: stitches from side to side with space between them. |
| E stitch | Blanket stitch: a line on one side, with stitches across like the teeth of a comb. |
| Raised satin | Satin over more satin layers, so the column stands up. Best up to 7 mm wide. |
| Tatami | Rows of short stitches across the column, for wide borders. |
| Square | Square teeth from side to side, an open border. |
| Coil | Loops across the column, like a spring seen from above. Avoid sharp corners. |
| Contour | Lines along the column that follow its shape. |
| String | Slanted stitches that overlap like twisted string. |
| Motif column | A motif repeated along the column, as high as the column is wide. |
| Program split | Satin rows with the needle points in a pattern that bends with the column. |

**Lean** (in Object Properties, for satin, raised satin and zigzag
columns) is the angle of the top stitches from square across the column,
from −45° to 45°. On a knit, stitches that lean a little do not sink
between the rows of the knit; a larger angle is a look. The underlay does
not lean. Near an end of the column the stitches fan from the corner, so
no needle point goes out of the column.

## Fills

| Fill type | What it sews |
|---|---|
| Tatami | Straight rows at the angle, in a pattern that does not show lines. |
| Satin (and other column types) | The column type across the shape at the angle. The shape sews in parts, so no stitch goes outside it. |
| Square | Straight rows with the needle points in line: a grid of even stitches. |
| Double square | Two layers of square stitching that cross, like a woven grid. |
| Contour | Lines that follow the shape from one end to the other. |
| Offset | Rings inside the edge, from the edge to the middle. |
| Spiral | One line that turns from the edge to the middle. |
| Island coil | Rings of loops from the edge to the middle, like hand chenille work. |
| Radial | Lines from a center point out to the edge, like the rays of a sun. |
| Maze | An open line that goes round to the middle and back out between its rows. |
| Florentine | Rows that bend like a guide line. |
| String | Straight lines from edge to edge that overlap in curves, like string art. |
| Cross stitch | Crosses on a grid, as in hand cross stitch. |
| Stipple | A meandering line that fills the shape and never crosses itself. |
| Motif fill | Rows of a motif. A gradient changes the size from row to row; the ball bends the rows. |
| Program split | Tatami rows with the needle points in a pattern that makes a picture. |
| Flexi split | A program-split pattern that bends with the sides of the shape. |
| User split | Rows split where they cross lines that you draw. |
| Laydown | A light fill under the design that holds down the pile of towels and fur. |
| Tartan | Stripes of color in two directions, as on woven tartan cloth. |
| Gradient | Rows that change from one color to the next across the shape. |
| Meander | One line along a pattern of tiles (squares, triangles or hexagons). |
| Circular | Rings round a center point, sewn as a spiral. |
| Guided | Rows that follow one or more guide lines and change from one to the next. |
| Lace mesh | The net of free-standing lace: open rows in two directions, each sewn there and back. |

Some fill types have more settings in Object Properties: the **Pattern**
and **Pattern size** of the split types, the **Center** of radial and
circular fills, the **Tiles** of a meander, **3D warp (ball)** of a motif
fill, and **Carving stamps** (shapes pressed into a tatami: the rows get
needle points along their lines).

## Fills with lines: guide lines and split lines

- **Guide Line** (Florentine): select a fill and a line, then **Edit >
  Stitch Types > Guide Line**. The rows bend like the line. The line is
  used up.
- **Guide Lines** (Guided): select a fill and one or more lines, then
  **Guide Lines**. The rows follow the lines and change from one to the
  next.
- **Split Lines** (User split): select a fill and one or more lines, then
  **Split Lines**. The rows are split where they cross the lines, so the
  lines show in the fill.

Object Properties shows the number of guide or split lines, with a
button to remove them.

## Tartan

1. Select a fill, then **Edit > Stitch Types > Tartan**.
2. Type the **Sett**: a color code and a thread count for each stripe,
   for example `K4 R24 K24 Y4` (K black, R red, Y yellow). **Setts…** has
   some setts to start from. The window lists all the color codes.
3. **Mirrored**: the stripes go out to the last stripe and back, as in
   most tartans.
4. **Width of one count** sets the size of the stripes.
5. Click **Make tartan**.

The program makes one fill for each color, in one group. A tartan has a
trim between the stripes of one color, so no thread goes over the other
colors.

## Gradient

1. Select a fill, then **Edit > Stitch Types > Gradient**.
2. Add the colors, each with a thread and a place from 0 % (one side of
   the shape) to 100 % (the other side).
3. Click **Make gradient**.

The fill angle sets the direction of the rows. Each row has one color, so
the density stays the same. A gradient of one color alone fades to the
fabric.

## Bands

1. Select a satin column, then **Edit > Stitch Types > Bands**.
2. Choose a thread for each band, from the first side of the column.
   **Add band** adds a band (at most six).
3. **Overlap** sets how far each band goes into the band next to it. A
   small overlap keeps the fabric from showing between the bands.
4. Click **Make bands**.

The program makes one satin column for each band, in one group, with the
rails of the selected column. The bands have equal widths. The first band
sews the underlay of the whole column, and pull compensation goes only
out at the two sides of the column. The lean of a band changes the lean
of all the bands of the column. **Remove bands** makes the column one
thread again.

## Motifs

**Edit > Stitch Types > Motifs** opens the motif library:

- **Use for the selection**: lines become motif runs, satin columns motif
  columns, fills motif fills.
- **Create from selection**: the stitches of the selected objects become
  a motif. The left and right middle of their box are its reference
  points.
- **Rename** and **Delete** change your own motifs. Designs keep their
  copy of a motif.

## Families

A family makes an item from some numbers: the family **Waves** makes a
motif from the number of the waves and their height. Most of the motifs,
the program-split patterns, the carving stamps and the clipart items of
the program come from families, and you can make an item of your own.

- **Families…** opens the Families window. The button is in the Motifs
  window, next to the lists of the patterns and of the stamps in Object
  Properties, and in the Clipart panel.
- Choose the kind and the **Family**. **Built-in items** are the items of
  the family that are in the lists of the program. Change the numbers: the
  picture shows the item.
- **Use for the selection** uses a motif or a pattern for the selected
  objects. **Add to the selected fills** puts a stamp in the middle of
  each selected fill. **Insert** puts a clipart item at the center of the
  view.
- **Keep in the library** adds a motif to your motifs, or a clipart item
  to your clipart, with the **Name** that you type.
- The **Description** is the name of the family and its numbers, for
  example `Waves: 2, 0.3`. The same description gives the same item.

A decorative border is a family too: in **Digitize > Borders**, **Shape**
gives a border another shape, and **Size of the stitches** makes its
stitches smaller or larger.

## Blends

**Digitize > Blends** blends two or more colors in one fill or satin
column: layers whose density changes from one side to the other. Set the
**Second color**, then click **Make a color blend**. Add more layers, and
set the density of each layer from start to end.

## Borders

**Digitize > Borders** adds a decorative border (satin, runs, blanket,
rope or motifs) round the selection, or round the whole design if
nothing is selected. Set the **Space round the objects**, then click
**Add the border**.

## Buttonholes

**Digitize > Buttonholes** makes buttonholes and eyelets in seven shapes
and in standard sizes: one, or a string along the selected line. See
[Buttonholes and eyelets](buttonholes).
