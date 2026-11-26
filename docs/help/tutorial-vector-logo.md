---
id: tutorial-vector-logo
title: "Tutorial: a vector logo"
group: tutorials
path: artwork
time: 20
level: practice
before: tutorial-patch
keywords: tutorial, vector, vectors, logo, svg, pdf, eps, ai, emf, wmf, import, artwork, import artwork, import as vectors, weld, shaping, cut, intersect, convert, to fill, to satin, to outline, border, add border, trace, trace picture, auto-trace
---
# Tutorial: a vector logo

In this tutorial you make embroidery from a vector logo: you weld its
shapes into one shape, convert the shapes to fills, and add a border.
Then you trace a picture into vector shapes.

## Vector artwork

A vector file has shapes, not pixels: SVG, PDF, AI, EPS, EMF or WMF.
There are three ways to bring one in:

- **File > Open** opens an SVG file as a new design.
- **Artwork > Import > Import Artwork** makes objects with stitches at
  once: fills, satins and outlines.
- **Artwork > Import > Import as Vectors** adds the shapes as vector
  artwork with no stitches. Clean up the shapes first, then convert
  them. This tutorial does this.

## Open the start design

Click [Open the start design](sample:vector-logo). It has a logo as
vector artwork, as after **Import as Vectors**: a cloud made of four
shapes that overlap (three circles and a base), and a sun. Vector
artwork shows in its colors, with no stitches.

## Weld the cloud

Shapes that overlap sew two times where they overlap. Weld them into one
shape.

1. Drag a box round the cloud. The four shapes of the cloud are
   selected, not the sun.
2. Click **Artwork > Shaping > Weld**. The cloud is one shape.

![The cloud welded into one shape](vector-welded)

The other commands of **Artwork > Shaping** cut shapes: **Front Minus
Back** cuts the shape in front out of the shape behind it, and
**Intersect** keeps where the shapes overlap.

## Convert to stitches

1. Click **Edit > Select > Select All**.
2. Click **Artwork > Convert > To Fill**. The cloud and the sun are fills,
   each in the color of its shape. (**To Satin** makes satin columns of
   narrow shapes, and **To Outline** makes running stitches round the
   shapes.)
3. Drag a box round the cloud to select it.
4. Click **Artwork > Borders and Offsets > Add Border**, then **Add
   outlines**. The cloud has a satin border.
5. Click **View > Show > Thread Look**, and select **Thread Look** in its
   menu.

![The logo in stitches](vector-stitches)

## Trace a picture

A logo that is a picture (PNG, JPEG) has no shapes. **Trace Picture**
makes a vector shape of each color area of the picture.

1. Click [Open the leaf logo](sample:logo-backdrop). The picture of a leaf
   is the backdrop picture.
2. Click **Artwork > Trace > Trace Picture**. Each color area of the
   picture is a vector shape in its color: three shapes in three
   colors.

![The leaf traced into vector shapes](vector-traced)

3. Convert the shapes as above. Before you convert, look at small
   shapes: delete the shapes that you do not want to sew.

For a picture with many colors, **Artwork > Import > Picture to Design**
does all the steps (see [Tutorial: a patch from a
picture](tutorial-patch)).

## If it does not look right

- **The sun is selected too.** The box selects only the shapes that are
  all in it. Draw the box round the cloud only.
- **A fill sews two times where shapes overlap.** Weld or cut the shapes
  before you convert them.
- **Trace Picture makes many small shapes.** The picture has noise or
  soft edges. Use a clean picture, or delete the small shapes.

## Next

- [Tutorial: a file from a customer](tutorial-customer-file)
- [Vector artwork and shaping](artwork): all the import, shaping and
  convert commands
