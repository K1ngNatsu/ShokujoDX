---
id: tutorial-sew-well
title: "Tutorial: make a design sew well"
group: tutorials
path: drawing
time: 20
level: practice
before: tutorial-logo-by-hand
keywords: tutorial, sew well, quality, fabric, preset, knit, t-shirt, density, spacing, underlay, pull compensation, pull, push, puckers, gaps, density map, warnings, design info, check
---
# Tutorial: make a design sew well

In this tutorial you prepare the leaf logo for a T-shirt. The stitches
are the same as in the logo tutorial, but the settings are not good yet:
the fill is too dense. You learn the settings that make a design sew
well on its fabric: the fabric preset, the density, the underlay and the
pull compensation.

## Open the start design

Click [Open the start design](sample:sew-well). The status line shows a
warning. Click the warning: **Design Info** shows it. The rows of the
leaf are 0.20 mm apart. Such a fill is stiff, and the needle can break.

## Tell the program the fabric

1. Click **Design > Fabric and Styles > Fabric**.
2. In the **Fabric** list, select **T-shirt knit**. The window shows
   what the fabric changes, and the backing to use.
3. Keep **Change the satin and fill objects of the design** on, and click
   **Apply**.

![The Fabric window with the T-shirt knit](sew-well-fabric)

A knit stretches, so the program makes the rows a little more open, sets
a heavy underlay and adds pull compensation. The rows of the leaf are
still too near (0.22 mm), and now the status line shows two warnings:
the density, and short stitches that the dense rows make.

## Set the density

1. Click **Digitize > Select > Select**, then click the leaf (not the
   vein).
2. In Object Properties, set **Density** to 0.4. This is the distance
   between the rows in mm: a larger value makes fewer stitches.
3. The two warnings go away.

Usual values: 0.4 mm for fills, 0.35 to 0.45 mm for satins. A thin
fabric or a thick thread needs more open rows.

## Set the pull compensation

The stitches pull the fabric in, so a satin column sews narrower than
its shape. **Pull compensation** makes it wider by this much.

1. Click the stem.
2. In Object Properties, set **Pull compensation** to 0.3.
3. Push **Esc**.

A knit needs more (0.3 to 0.4 mm) than a woven fabric (0.2 mm). Look at
**Underlay** in Object Properties too: the underlay sews under the top
stitches and holds the fabric. **Heavy** is good for a knit; **Light**
is good for small text.

## Look at the density map

1. Click **View > Show > Density Map**. Colors show how many layers of
   thread are at each place: blue is one layer, red is too many.
2. The places where the vein and the stem go over the leaf have more
   layers. That is normal for a small part.
3. Click **View > Show > Density Map** again to turn it off.

![The density map of the leaf](sew-well-density)

## Check it

- The status line shows no warning.
- **Design Info** shows the trims, the density and the warnings of the
  design.
- Sew a test on the same fabric, with the same backing, before you sew
  the real garment.

## If it does not look right

- **Waves round the design.** Use a firmer backing, and hoop tighter. A
  larger density value helps too. See [Questions and
  problems](troubleshooting).
- **Gaps between a satin and a fill.** Add pull compensation, or let the
  two objects overlap by 1 mm.
- **The fabric shows through a fill.** The rows are too far apart: set
  a smaller density value, or a heavier underlay.

## Next

- [Tutorial: columns that turn](tutorial-columns)
- [Fabrics, styles and templates](fabrics-and-styles)
- [Checking a design](checking)
