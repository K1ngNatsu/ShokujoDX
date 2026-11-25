---
id: tutorial-customer-file
title: "Tutorial: a file from a customer"
group: tutorials
path: artwork
time: 20
level: practice
before: tutorial-vector-logo
keywords: tutorial, customer, machine file, stitch file, dst, pes, exp, jef, vp3, open, recognize, recognize objects, stitches, objects, size, resize, scale, larger, smaller, thread colors, outlines, write, export
---
# Tutorial: a file from a customer

In this tutorial you open a machine file from a customer, make objects
from its stitches with **Recognize Objects**, change its size, and write
it again.

A machine file (DST, PES, EXP, JEF, VP3 and others) has only stitches:
no shapes and no settings. If you make it larger as stitches, the
stitches get longer and the density gets lower. Objects have their
shapes and settings, so the program makes new stitches at the new size.

## Open the file

Click [Open the start design](sample:customer-dst). It is the file
**LEAF.DST**, as **File > Open** opens it: one stitch object for each
color. A DST file has no thread colors: the program gives the colors
of its standard list. Set the thread colors that the customer wants
(the Threads strip). The start design shows the stitches on a white
shirt (**Color > Fabric Color**), as the first color of the file is
black.

![The stitches of the file](customer-file)

## Recognize Objects

1. Click **Edit > Select > Select All**.
2. Click **Artwork > Convert > Recognize Objects**. The window shows the
   file and the objects that the program found in its stitches: the
   fill of the leaf (with its hole), the satin of the stem, the running
   stitch of the vein, and the parts of the underlay of the fill.

![The Recognize Objects window](customer-recognize)

3. Look at the list. A part that the program does not recognize stays
   stitches. Clear a part to keep its stitches as they are.
4. Click **Accept**. The objects replace the stitch objects, in one undo
   step. The status line says what the program made: 6 objects, 5 with a
   stitch type and 1 that stays stitches.

If the customer also sent the artwork (SVG, PDF, AI, EPS, EMF or WMF),
click **Outlines…** before **Accept**: the objects get the shapes of the
artwork, which are more exact than the shapes from the stitches.

## Change the size

1. Click **Edit > Select > Select All**.
2. In Object Properties, under **Position and size**, keep **Keep
   proportions**, and set **W** (the width) to 80 mm. The program makes
   the stitches again at the new size, with the same density.

![The leaf, larger](customer-larger)

## Write the file again

Click **Design > Output > Export**, then **Export…**, and select the
format of the machine of the customer. Also save the project (**Save**):
it keeps the objects, so you can change the design again later.

## If it does not look right

- **Recognize Objects is gray.** Select the stitch objects first.
- **A part stays stitches.** The program did not find a shape in it (for
  example a fill with a pattern). Keep it as stitches, and do not make
  the design much larger or smaller.
- **The colors are wrong.** A DST file has no colors. Click a thread in
  the Threads strip, and set its color.

## Next

- [Tutorial: an appliqué](tutorial-applique)
- [Files and output](files-and-output): the file formats and Recognize
  Objects
