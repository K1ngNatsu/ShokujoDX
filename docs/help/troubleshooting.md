---
id: troubleshooting
title: Questions and problems
group: start
keywords: problem, question, faq, pucker, gap, trim, jump, break, stiff, small text, machine does not read, file, slow, crash, missing, wrong color, too many stitches, overlap, build up, svg, density, density map, layers, picture, photo, picture to design, objects, thread breaks, birdnest, loops, outline does not line up, registration, shift, pile, towel, sinks, off center, hoop, too large, does not fit, letters fill in, button dimmed, cannot click, lost panel
---
# Questions and problems

## The fabric puckers

The stitches pull the fabric together. Use a stronger backing (a
cut-away for knits), and hoop the fabric tight. Set the correct fabric
(**Design > Fabric**). Make the fill less dense, or use a lighter
underlay.

## There are gaps between a fill and its border

The stitches pull the fill in. Make the **pull compensation** larger, or
let the border go a little over the fill. **Artwork > Border Trace** adds
a border that goes under the fill edge; set its **Border position** to
**Centered on the line** or **Inside the line** to cover more of the
edge. Picture to design has an
**Overlap** setting for the same reason.

## The outlines do not line up with the fills

The fabric moved while the machine sewed. Use a stronger backing and hoop
tight. Sew each outline just after the fill under it, not at the end of
the design. More underlay helps the fill keep its shape.

## The design has too many trims

Each trim takes time. Keep the objects of one color together in the sew
order (**Arrange > Fewer Color Changes**). **Arrange > Shortest Order**
puts the objects of each color nearest first. **Closest Join** lets short
gaps get travel stitches, not trims. Design info warns about objects
with many trims: try another start or end point.

## Picture to design makes too many objects

A photo has many small areas of each color, and each area that is apart
from the others needs a trim. In the wizard:

- **Background:** use **Cut out subject**, so only the subject sews.
  **Remove the edge color** works well only on a plain background.
- **Simplify:** make the specks larger (**Remove specks under**), use
  less detail (a larger **Detail**), or use more **Even out**.
- **Finish:** the checks show the trims and the very small objects, and a
  button changes the settings for you.

A logo or a drawing with flat colors gives the best result. For a photo
of a person, **Photo stitch** is often better.

## Satin from an SVG piles up on the machine

Convert the vectors again with **To Satin**, or set the stitch type of the
fill to **Satin**: each shape then sews in parts with no stitch outside
it. To control the direction of the stitches, use **Edit > Direction Line** on the object.

## Small text does not sew well

Embroidery fonts have a size range; the font list shows it. Below about
5 mm, letters lose their shape. Use a font that is made for small text
(**Fonts**, then **For small text**: these fonts go down to 4 mm), or make
the text larger. Set the underlay of small text to **None** or **Light**.

## The letters fill in or look heavy

The satin is too dense for the size, or the thread is too thick. Make the
density of the lettering more open (Object Properties > Satin >
Density), use a lighter underlay, or make the text larger.

## The stitches sink into a towel or fleece

The pile comes up through the stitches. Add **Specialty > Laydown** under
the design, use a water-soluble topping, and set the fabric **Towel
(terry)** or **Fleece and sweatshirt**. See [Tutorial: a monogram on a
towel](tutorial-towel).

## The design is stiff and the needle breaks

The stitches are too dense, or many layers are on top of each other. To
find the places, turn on **View > Density Map**: it colors the design by
the layers of thread, and places over the limit are red. Design info
lists each place; click one to go to it. The limit is in **Options**
(5 layers at the start).

Make the density larger (more open). Remove the hidden parts of the lower
objects with **Artwork > Shaping > Flatten**.

## The thread breaks or makes loops under the fabric

- Very short stitches break the thread: Design info warns about stitches
  shorter than the shortest stitch.
- Too many layers in one place (see the density map).
- On the machine: check the threading, the tension, the needle (a new
  embroidery needle of the correct size) and the bobbin.

## The machine does not read the file

- Check the format of your machine (the **Machine** list on the Design tab). Most home
  machines read only their own format.
- Check the folder on the USB stick. **Send to USB** uses the folder of
  your machine.
- Check that the design fits the hoop of the machine: Design info tells
  if stitches are outside the hoop field.

## The design is not at the center on the machine

The machine puts the design where its file says, from the hoop center.
**Arrange > Center in Hoop** moves the selection to the hoop center. Use
**Design > Hoop Template** to mark the place on the fabric. **Hoop Options > Auto start and end** sets where the needle goes first.

## The design is larger than my hoop

**Auto Hoop** selects a larger hoop if the machine has one. **Design > Rotate Design** turns the design 90°, and **Turn hoop** turns the hoop.
Or sew the design in parts: see [Designs larger than the hoop](hooping).

## The colors on the machine are wrong

Formats such as DST have no colors. The machine shows its own colors.
**Output Design** can write thread color files next to the machine file.
Also check which colorway is current (Color tab): Export writes the
current colorway.

## A machine file gets too dense when I scale it

A machine file has needle points, not shapes. Use **Artwork > Convert > Recognize Outlines** first: the stitches become fills, which keep their
density when you scale them.

## A button is dimmed

The command needs something first, for example a selection, a backdrop
picture, or two objects. Put the pointer on the button: the tooltip tells
what it does, and the help (F1) tells what it needs.

## I cannot find a command

Push **Ctrl+K** and type a part of its name. The help topic of each
ribbon tab lists all its commands. In simple mode, some commands are only
on the full ribbon: turn simple mode off (View > Simple Mode).

## A panel is gone

The × of a panel closes it. **View > Panels** opens it again.

## Why does the program start with a page?

That is the start page. Clear **Show at start** on it if you want the
empty design at the start.

## The program stops or is slow

The program saves backups and offers them at the next start. A crash
report is in the `crashes` folder of the settings (**Help > About** shows
where). Send it with your feedback. A design with very many stitches or a
large backdrop picture is slower; hide the thread look while you work.
