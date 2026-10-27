---
id: artwork
title: Vector artwork and shaping
group: design
keywords: border position, inside, outside, centered, vector, artwork, svg, pdf, ai, emf, wmf, import, draw artwork, convert, to fill, to satin, to outline, to vector, recognize outlines, shaping, weld, intersect, exclude, combine, front minus back, back minus front, flatten, divide, remove overlaps, unlink, linked, keep originals, overlap, border trace, offsets, outlines, add border, export outlines, cutter
---
# Vector artwork and shaping

Vector artwork is shapes with no stitches. It shows on the canvas, but it
sews nothing, and machine files leave it out. You use it as a guide, or
convert it to embroidery.

## Import artwork

The **Artwork > Import** panel reads SVG, PDF, AI, EMF and WMF files:

- **Import as Vectors**: the paths become vector artwork. Convert them
  later.
- **Import Artwork**: the paths become embroidery objects at once (fills
  and lines).
- **Import as Lines**: the outline of each shape and each stroke becomes
  a running-stitch line.
- **Insert Stitches** adds the stitches of a machine file to the design,
  as stitch objects.

You can also drag an SVG or PDF file onto the window.

## Draw artwork

Turn on **Artwork > Draw Artwork**. The draw tools (Shape, Line,
Freehand, Rectangle, Ellipse, Star) then make vector artwork in place of
embroidery objects. Turn it off to draw embroidery again.

**Vectors** shows or hides the artwork. **Dim Vectors** shows it pale, so
the stitches stand out.

## Convert

Select vectors, then:

- **To Fill**: each closed shape becomes a fill, with its holes.
- **To Satin**: each closed shape becomes a satin. A long shape gets a
  turning column along its middle, the ring of an O gets a ring column,
  and other shapes get a fill with the satin stitch type. No stitch goes
  outside the shape or over another part of it. A line becomes a satin
  column along it.
- **To Outline**: all paths become one running-stitch line.

Other commands of the panel:

- **To Vector**: the selected embroidery objects become vector artwork
  (their outlines, no stitches).
- **Recognize Outlines**: stitch objects (from a machine file or a photo
  stitch) become fills of the area that their stitches cover, so you can
  scale them and they keep their density.

## Shaping

Shaping makes new shapes from two or more selected shapes (vectors,
fills, appliqué and other objects with an area). The front shape is the
one that sews last.

| Command | Result |
|---|---|
| Weld | One shape of all the selected shapes. |
| Intersect | Only the area that all selected shapes cover. |
| Exclude | The areas that the shapes do not share. |
| Combine | One object of the selected vectors, or of the selected appliqué objects. |
| Front minus back | The front shape, with the others cut out of it. |
| Back minus front | The back shape, with the others cut out of it. |
| Flatten | Each shape without the parts that the shapes in front of it hide, so nothing sews twice. |
| Divide | The shapes cut into pieces where they overlap; each overlap goes to the front shape. |
| Remove overlaps | The selected shapes cut out of the objects under them. |

The options of the panel:

- **Keep originals**: the selected objects stay; the result comes after
  them.
- **Keep linked**: Flatten, Divide and Remove overlaps cut again when
  their objects move. **Unlink** ends the links of the selection; the
  objects stay as they are.
- **Overlap**: how far a cut piece goes under the shape that cut it, so
  no fabric shows between the pieces. Remove overlaps and the knife use
  it too.

## Border trace

**Artwork > Border Trace** adds a border round the selected objects, a
gap round them, or both, cut out of the objects under them. In the gap,
the fabric shows.

- **Border width**, the **Stitch** (a column type, a run or a triple run)
  and the **Thread**.
- **Border position**: where the border sits on the edge of the objects.
  **Outside the line**: the border goes round the objects and covers only
  a little of their edge (a column goes 0.4 mm over it, so no fabric
  shows). **Centered on the line**: the middle of the border is on the
  edge. **Inside the line**: the border covers the edge of the objects,
  and the design does not get larger.
- **Sew Order**: **Objects, then border** or **Border, then objects**.
- **Gap width**: the space round the border (or round the objects, with
  no border) where the fabric shows.

The trace stays linked: it is cut again when its objects move. Change it
in the window while its objects are selected. **Remove border trace**
brings back the cut objects.

The objects and their border are one group, so a click selects all of
them. To change the stitches of one of them (for example the fill type),
double-click it, or click its button under **Objects of the group** in
the Properties panel. Break Apart and Join do not work on linked objects:
use **Artwork > Unlink** first.

## Offsets

**Artwork > Offsets** adds outlines at a distance round the selected
objects (**Outside**) or in them (**Inside**):

- **Distance** of the first outline, the number of **Outlines** and the
  **Spacing** between them.
- **Corners**: **Round**, **Sharp** or **Cut**.
- **Make**: a **Line**, a **Vector** or a satin **Border** of a width.
- **Border position** (for a border): **Outside the line** (away from the
  middle of the objects, so a border at distance 0 does not cover them),
  **Centered on the line**, or **Inside the line** (toward the middle of
  the objects). Outside and inside stay the same for outlines inside the
  objects.

**Add Border** opens the same window, set to make one satin border, at
the border position that you used last.

## Export outlines

**Artwork > Export Outlines** writes the shapes of the objects as SVG or
EMF paths, for vector programs and cutters. For appliqué pieces, use
**Digitize > Cutter File**.
