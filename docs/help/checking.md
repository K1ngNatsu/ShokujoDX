---
id: checking
title: Checking a design
group: production
keywords: check, preview, stitch player, play, sew-out, simulate, stitch cursor, travel, stitch list, functions, design info, warnings, density map, layers, build up, thread look, 3d, light, light presets, tilt, tilt view, turn, magnifier, compare, before and after, show through, gaps, high thread, thread breaks, needle points, jumps, trims, product, t-shirt, mockup, repeats, continuous, approval, sew time, stitch count
---
# Checking a design

Check each design before you sew it. The program shows how the machine
sews it, and warns you about common faults.

## The stitch player

**View > Stitch Player** shows a bar under the canvas. It plays the
sew-out: the stitches show as the machine sews them.

- **Play the sew-out** and **Pause**, and the speed.
- **To the start**, **Back One Color**, **Forward One Color**, **To the
  end**.
- Drag the bar to move through the sew-out.

## The stitch cursor

**View > Stitch Cursor** marks one stitch. The stitches after it show
dimmed. Move it with the keys:

- **,** and **.**: one stitch back or forward (**Shift**: 10, **Ctrl**:
  100, **Ctrl+Shift**: 1000).
- **Page Up**, **Page Down**: the previous or the next object.
  **Ctrl**: the previous or the next color. **Shift**: the previous or the
  next machine function. **Alt**: the previous or the next segment.
- **Functions** (View > Stitch Cursor) sets the kind of function that
  Shift+Page Up and Shift+Page Down go to: any function, trims, color
  changes, stops, jumps or speed changes.
- **Center at Stitch** moves the view to the cursor.

With the cursor in an object, **Edit > Select > At Stitch Cursor**
selects it, **Edit > Split at Needle** cuts it there, and **Paste Special** can paste at the cursor.

## The stitch list

**View > Stitch List** shows the Stitch list panel: every stitch, jump,
trim, stop and color change, with its place, its length and its color.
Click a row to move the stitch cursor there. **Functions only** shows
only the jumps, trims, color changes and stops.

## Design info

The **Design Info** panel shows the numbers of the design:

- the size, the hoops it fits, the number of objects, stitches, colors,
  color changes, trims, jumps and stops;
- the travel that shows: on top of sewn stitches and on the fabric;
- the sew time at the speed of the machine;
- the fabric, the top thread and the bobbin thread, and the thread cost;
- the density (the most layers of thread);
- the colors in sew order, with the thread of each.

Under the numbers are the **Warnings**. Click a warning to select the
object. The program warns about:

- stitches outside the hoop field, or outside the safety margin;
- stitches shorter than the shortest stitch (the thread breaks);
- many trims in one object (try another start or end point);
- a density that is too high;
- thread that builds up in a place (more layers than the limit);
- lettering that is smaller or larger than the size range of its font;
- on a cap frame, a sew order that is not from the center out;
- in free-standing lace, a part that touches no other part;
- travel stitches that show: more than 2 mm on top of sewn stitches in
  one object, or a piece of 4 mm or more on the fabric;
- objects that make no stitches;
- in stitch objects (a machine file opened as stitches): stitches longer
  than the machine sews, color changes to the same thread, long jumps
  with no trim, pieces too short for the trimmer, starts and ends with
  no lock stitches, and travel of the file that shows. **Fix Stitches**
  fixes most of them.

## The density map

**View > Density Map** colors the stitches by the layers of thread: red
where objects on top of each other build up more layers than the limit
(**Options > Density limit**, 5 layers at the start). Design info lists
these places. Remove hidden parts with **Artwork > Shaping > Flatten**,
or make the density more open.

## The thread look

**View > Thread Look** shows the stitches as threads, as they look on the
fabric. Its menu has **Flat** or **3D (light and shade)**, the direction
and the height of the light, the strength of the shade, and the **Fabric
texture** (the surface of the fabric of the design in the fabric color:
the threads of its weave, the loops of a towel or the grain of leather,
in the light of the thread look).
**Default light** sets them back.

With the **Quality** Good or Best, the 3D look shows the surface of
the stitches: thread on thread is higher, each area is lighter or darker
by the direction of its stitches to the light, and high parts put
shadows on the fabric. Each needle hole is a small dent (larger on
leather). On a towel or fleece with no topping, thin stitches sink into
the pile (see [Fabrics, styles and templates](fabrics-and-styles)). What
is not thread has its own surface: appliqué fabric with its weave and a
cut edge, rhinestones with facets that take the light, the holes of
cutwork, no fabric under lace, padding under a trapunto fill, and on a
quilt the batting between the lines of stitches. Metallic thread has
small glints at a large zoom. **Lights off** shows a dark room: only
glow-in-the-dark thread shows, in its own light. The
program makes the surface in parts while you
work; a part that is not ready yet shows the faster look for a moment.
Choose the **Quality** Fast on a slow computer (see [The 3D view](3d-view)).

## Check the look

The **Check** panel of the View tab has tools to check a design before
you sew it.

- **Light**: the presets **From the top left** (the default), **Flat
  from above**, **Low from the side** (it shows each height and each
  gap) and **Two lights**, and the direction and the height of the
  light. Hold **L** and drag on the design to move the light: in the
  middle of the view it is straight above, at the edge it is low.
- **Tilt View** shows the embroidery from an angle on the canvas. Drag
  up and down to tilt it and across to turn it, drag with the right
  button to move it, and turn the wheel to zoom. **Heights** makes the
  heights larger (×1 is the real height). The light stays in its place
  when the design turns, as when you turn a hoop under a lamp. The tools
  of the canvas wait until you click **Close**. **Tilt Window** shows
  the same view in a window, so you can change the design and see it
  from an angle.
- **Magnifier** shows the thread at 20 times its real size in a round
  window at the pointer.
- **Compare 3D** shows the stitches on the canvas and the 3D look in the
  split view, at the same place and zoom (**Same Place** in the split
  view).
- **Before and After** keeps the stitches as they are now. Change a
  setting, then drag the line on the canvas: the left side shows the
  stitches before, the right side the stitches now.
- **Fabric Shows Through** marks the places where the fabric shows
  through a fill or a satin, from the laid threads (their widths,
  heights and gaps). Fills that are open by design (a blend, a texture,
  motifs) are not checked. Make the density higher, or change the angle
  of the rows, until the place closes.
- **High Thread** marks the places where the thread is higher than the
  limit over the fabric (foam and padding do not count): a risk of
  thread breaks.
- **Travel That Shows** marks the travel stitches that no stitch covers:
  red where they lie on top of stitches sewn before them, blue where they
  lie on the fabric. Travel under stitches that sew later, between the
  rows of a fill and along the edge of an area does not show. Change the
  start or the end point of the object, or the sew order, so that the
  travel goes under the stitches that sew later.

Set the limits of the two checks in **Options** (**Fabric shows
through** and **Thread height limit**).

## Other views of the stitches

**View > Show** has **Jumps** (jumps and travel lines), **Needle Points**
(a dot at each needle point) and **Trims and Colors** (a symbol at each
trim and color change). **Stitch Parts** colors the stitches by the kind
of their part: runs (blue), columns (orange), rows of a fill (green) and
other stitches (purple); the underlay of a part has its color, paler.
Lines show the objects that the parts become: the line of a running
object, the two sides of a satin column, and the outline and the holes of
a fill. It shows how the program
reads the stitches of a machine file of another program, the first step
to make objects of them.

## Fix a machine file

A machine file opens as stitch objects. **Edit > Machine Functions >
Fix Stitches…** checks them and fixes the faults that it can:

- **Join color blocks of the same thread**: the color change to the same
  thread goes.
- **Remove stitches shorter than** the shortest stitch of the machine:
  the next stitch starts at the needle point before.
- **Split stitches longer than** the longest stitch of the machine into
  equal stitches.
- **Trim jumps longer than** the trim distance.
- **Add lock stitches at starts and ends** that have none.

Each fix has a check box and the number that the check found. **Apply
Fixes** makes the fixes that are on, as one undo step. The other needle points
stay where they are. Pieces too short for the trimmer and travel that
shows have no fix: Design info shows where they are. On the command
line, `emb-cli check` and `emb-cli fix` do the same for a file.

## The product

**Design > Product** shows the design at its real size on a T-shirt
(short or long sleeves), polo, sweatshirt, hoodie, cap, beanie, apron,
bag, towel or cushion, or on a picture of your own product (**My picture…**, at its real width). Set the **Color** and where
the design goes (**Put the design at**). **Show Product** shows or hides
it. **Picture on Product** saves a PNG picture of the design with the
thread look on the product, for a customer. The product does not sew.
A drawn product has light and shade, folds and the texture of its
fabric, in the light of the thread look; with the 3D look, the
embroidery on it has its heights, its sheen and its shadows in the same
light; on a cap, the picture shows the design a little
curved round the crown. **Add a design…** shows one more design (a
project or a machine file) on the product, for example a name on a
sleeve with this design at the left chest: set its place in the list
of **Other designs on the product**. It shows on the canvas and in the
picture, and it does not sew. A garment has a front and a back:
**Side** in the Product window (or **Turn Product**, Ctrl+Shift+T)
shows the other side, and the design goes to the same place on it.
Each other design is on a side. With **Both sides in the picture**,
the saved picture shows the front and the back next to each other.
The swatches under **Color** are the usual colors of blank garments.
With **Trim color**, the trim parts of the product (for example the
collar of a polo, the front panels or the brim of a cap, the cuffs of a
sweatshirt) get a second color; clear a part in **Trim parts** to give
it the product color.

For a photo of your own product, **Edit the Photo…** opens the
**Product Photo** window. **Pick Color** and a click on the product mark
the area of that color (seams and stitches in it too; **Tolerance** sets
how near a color must be); **Remove Color**, **Brush** and **Eraser**
change the mask. The marked part takes the product color with the light
and the shade of the photo, so one photo of a white product shows the
product in each color. **Mark a Place** and a click mark a usual place
for a design; give it a name and the size of its largest design. **Save
in My Products** keeps the photo with its real width, its mask and its
places; choose it again in **My Products**. **Export…** and **Import…**
move a product to another computer.

**Design > Turntable** shows a drawn product with its designs from more
sides: it turns in steps of 15°. Use the arrows or the slider, drag the
picture, or click **Play**. The program makes the 24 views in the
background after each change, so a turn is fast on a slow computer too.
The product has a soft round shape, made from its drawing; the canvas
stays flat. **Save GIF…** saves the turn as an animated picture, and
**Save Four Views…** saves the front, two quarter turns and the back next
to each other. A photo of a product cannot turn.

## Repeats

**Design > Repeats** shows the design again in **Columns** and **Rows**,
to check the join of a border that goes on (a continuous design) or of
designs that overlap.

- **Step to the right** and **Step down** set the place of the copies. A
  step down too gives a slanted row; a step to the side too gives a
  half-drop. **Side by side** sets steps the size of the design.
- **Join end to start** sets an auto start at the left and an auto end at
  the right of the design, so the needle goes from copy to copy.
- **Show Repeats** shows or hides them. The copies do not sew until you
  click **Make Repeats**: then they are objects, after the design.

## Print preview

**Design > Print Preview** shows the production worksheet and the
approval sheet. See [Files and output](files-and-output).
