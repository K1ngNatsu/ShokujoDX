---
id: objects-and-stitches
title: Objects and stitches
group: design
keywords: object, stitch, running, satin, fill, tatami, appliqué, lettering, vector, stitch object, density, spacing, underlay, light, heavy, double, pull compensation, stitch length, split, short stitches, liquid, omit last stitch, effects, accordion, hand stitch, cording, corners, mitre, auto jump, auto spacing, jagged edge, trapunto, properties, position, size
---
# Objects and stitches

A design is a list of objects. The program makes the stitches of each
object from its shape and its settings. When you change the shape or a
setting, the stitches change at once.

## Kinds of objects

- **Lines**: running stitches along a line (single, triple, or a column
  of a stitch type along the line).
- **Columns**: satin stitches across a column from one side to the other.
  Good for borders and letters up to about 10 mm wide.
- **Fills**: rows of stitches that cover an area (tatami and other fill
  types).
- **Appliqué**: a placement line, a stop, a tack-down, a stop and a cover
  satin. An appliqué object with more pieces can have up to four layers
  of fabric (Object Properties > Layers): each layer sews in turn, and
  the covers come last. See [Tutorial: an appliqué](tutorial-applique).
- **Lettering**: text in a font. Each letter has satin and fill parts.
  See [Lettering and monograms](lettering).
- **Vectors**: artwork with no stitches. **Convert** makes embroidery
  from it. See [Vector artwork and shaping](artwork).
- **Stitch objects**: stitches from a machine file or a photo stitch. The
  program sews them as they are. Reshape moves, adds and deletes single
  needle points. **Recognize Outlines** (Artwork > Convert) changes them
  into fills that you can scale.
- **Cross Stitch** and **rhinestones**: see [Specialty
  decoration](specialty).

## Object Properties

Select an object to see its settings in the **Object Properties** panel.
Select more objects of one kind to change them all. Each setting has a
word next to it that tells what the value means, for example "dense" or
"long". Put the pointer on the name of a setting to see what it does.

- **Thread**: the thread of the object.
- **Position and size**: the place, the width and the height, with
  **Keep proportions**; **Rotate**, **Mirror** and **Center in Hoop**.
- **Machine Functions**: see [Changing objects](editing).

## The settings of the stitches

- **Density** (spacing): the distance between rows or satin stitches.
  Smaller is denser. 0.4 mm is normal. Too dense makes the fabric stiff
  and can break needles.
- **Stitch length**: the length of fill and running stitches. Short
  stitches follow curves better; long stitches are faster.
- **Angle**: the direction of the rows of a fill. The **Angle** tool sets
  it with two clicks.
- **Pull compensation**: extra width, because the stitches pull the
  fabric in. Stretchy fabric needs more.
- **Split stitches over**: satin stitches longer than this get extra
  needle points, so they do not snag.
- **Short stitches at curves**: shorter stitches on the inside of tight
  curves, so the curve does not get too dense.
- **Omit last stitch**: the column ends one stitch early, on the other
  side. Use it where a column joins the next column.
- **Liquid**: the stitch angle changes evenly along the column, also
  where the stitch angle lines turn it sharply.
- **Puff (3D foam)**: see [Specialty decoration](specialty).

## Underlay

Underlay is stitches under the top stitches that hold the fabric and lift
the top stitches.

| Underlay | Use |
|---|---|
| Normal | The program selects the underlay from the size of the object. |
| Light | Less underlay, for thin or delicate fabric. |
| Heavy | More underlay, for soft, stretchy or high-pile fabric. |
| Double | Two layers that cross: two zigzags on columns, two tatami layers on fills. |
| Lace | For free-standing lace: a run along the edge two times, then a zigzag or rows across it. |
| None | No underlay. Use it for small text and thin details. |
| Custom | Underlay layers set one by one. |

The fabric of the design (Design > Fabric) changes the underlay of the
objects that have automatic underlay.

## Effects of columns

The **Effects** part of Object Properties changes how a satin column
sews:

- **Accordion spacing**: the spacing changes from start to end, for
  shading. **Custom**: set the spacing at points along the object.
- **Hand stitch**: needle points move a little at random, so the
  stitches look sewn by hand.
- **Cording**: a line, a stop to lay a cord on it, then an open zigzag
  that holds the cord down.
- **Corners**: **Mitre** (the outer edge comes to a point), **Cap** (the
  outer edge is cut off straight) or **Lap** (the column stops and a new
  one starts over the corner).
- **Auto jump**: no extra needle points in long stitches; stitches longer
  than the machine allows go as jumps.
- **Spacing measured at**: where across the column the spacing is
  measured on curves (0 at rail A, 0.5 in the middle, 1 at rail B).
- **Auto spacing**: narrow parts of the column get more open spacing,
  wide parts denser spacing.
- **Jagged edge**: the edge stitches end at random distances, for fur or
  grass.
- **Pull compensation** in percent, and **Rail B pull**: more pull for
  wider columns, and a pull of its own for side B.
- **Trapunto**: travel goes along the edge, not across the inside, so the
  inside can be padded.

## The current properties

New objects get the **current properties**: the settings of the
property bar and of the Styles panel. **Design > Make Current** takes the
settings of the selected objects; **Apply Current** gives the current
properties to the selected objects. See [Fabrics, styles and
templates](fabrics-and-styles).

## Stitch types

Each kind of object has stitch types: lines can be motif runs or
columns, columns can be zigzag, E stitch and more, fills can be contour,
spiral, motif fills and more. See [Stitch types](stitch-types).
