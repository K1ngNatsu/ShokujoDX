---
id: checking
title: Checking a design
group: production
keywords: check, preview, stitch player, play, sew-out, simulate, stitch cursor, travel, stitch list, functions, design info, warnings, density map, layers, build up, thread look, 3d, light, needle points, jumps, trims, product, t-shirt, mockup, repeats, continuous, approval, sew time, stitch count
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
- objects that make no stitches.

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
texture** (the weave of the fabric of the design under the fabric color).
**Default light** sets them back.

## Other views of the stitches

**View > Show** has **Jumps** (jumps and travel lines), **Needle Points**
(a dot at each needle point) and **Trims and Colors** (a symbol at each
trim and color change).

## The product

**Design > Product** shows the design at its real size on a T-shirt,
polo, sweatshirt, cap, bag, towel or cushion, or on a picture of your own
product (**My picture…**, at its real width). Set the **Color** and where
the design goes (**Put the design at**). **Show Product** shows or hides
it. **Picture on Product** saves a PNG picture of the design with the
thread look on the product, for a customer. The product does not sew.

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
