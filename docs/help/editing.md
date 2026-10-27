---
id: editing
title: Changing objects
group: design
keywords: edit, copy, cut, paste, paste special, paste after, duplicate, delete, clipart, select, by color, by type, split, join, break apart, knife, split at needle, hole, remove holes, fill holes, corner radius, hole from outline, frame, simplify, nodes, continue line, redwork, branching, close shape, reverse, swap sides, machine functions, stop, trim, tie-in, tie-off, lock stitch
---
# Changing objects

The **Edit** tab changes the objects that you have. Most commands work on
the selection; a command that cannot run now is dimmed.

## Clipboard

- **Copy** (Ctrl+C), **Cut** (Ctrl+X) and **Paste** (Ctrl+V). Options >
  Editing sets where Paste puts the copies.
- **Duplicate** (Ctrl+D) makes a copy a little to the side. Options sets
  the distance.
- **Paste After Selected** puts the copies just after the selected
  objects in the sew order, not at the end.
- **Paste Special** asks where the copies go: **At the same place**,
  **Moved by a distance**, **Centered on the hoop**, **Centered on the
  stitch cursor**, or **First stitch at the stitch cursor**.
- **Delete** (Del) deletes the selection.

## Clipart

Clipart items are pieces of designs that you use again, for example a
frame or a logo.

- **Clipart** shows the Clipart tab. Double-click an item, or drag it
  onto the design.
- **Save as Clipart** keeps the selected objects as an item, with a name
  and keywords for the search.

See [The design library and clipart](library).

## Select

- **Select All** (Ctrl+A), **Next** (Tab) and **Previous** (Shift+Tab).
- **At Stitch Cursor** selects the object that the stitch cursor is in.
- **By Color** selects all objects of one thread. **By Type** selects all
  running lines, all satin columns or all fills.

## Split and join

- **Knife**: cut the selected objects along a line. Click two points.
- **Break Apart**: one object for each separate part of an object.
- **Join**: one object from the selected objects of one kind and color:
  lines and columns whose ends touch, fills that touch. It puts back
  together what Break apart or the knife separated.
- **Split at Needle**: cut the object under the stitch cursor in two,
  where the cursor is.

## Shape

- **Angle** and **Direction Line** set the direction of the stitches. See
  [Drawing objects](digitizing).
- **Add Hole** draws a hole in the selected fill. **Remove Holes** fills
  the holes of the fill (the holes sew over). **Fill Holes** makes a new
  fill object in each hole.
- **Corner Radius**: rounds the corners of a fill or an appliqué with
  straight sides. Set one radius for **All corners**, or a radius for
  each corner (numbered from the first point of the outline).
- **Hole from Outline**: adds a hole with the shape of the outline, at a
  distance inside it. Use it for a frame or a border.
- **Quick Simplify**: fewer nodes, with the standard settings. The shape
  stays within 0.2 mm.
- **Simplify…**: fewer nodes with your settings: the **Tolerance** (how
  far the new outline can be from the old one), **Corners** (a turn
  sharper than this stays a corner), and **Curves** or **Straight lines**
  between the corners. The window shows the number of nodes before and
  after. Simplify never adds nodes.

## Lines and columns

- **Continue Line**: add points to the end of the selected line, or to
  its start (click near the start first).
- **Redwork**: join the lines into one object. Each line sews two times,
  and the object ends where it starts. Good for outlines with no trims.
- **Branching**: join the lines into one object, sewn with the fewest
  lines sewn two times.
- **Close Shape** closes open lines, vectors and stone runs with a
  straight line. **Close with Curve** closes them with a curve that
  continues the ends.
- **Reverse Direction**: the lines, columns, vectors and stone runs go the
  other way. The start and end change places.
- **Swap Sides**: the two sides of the selected satin columns change
  places, with the settings of each side.

## Stitch types

The **Stitch Types** panel sets the **Outline Type** of lines, the **Fill Type** of fills and the **Column Type** of columns, and has the special
fills: motifs, guide lines, split lines, tartans, gradients and ripples.
See [Stitch types](stitch-types).

## Machine functions

Machine functions tell the machine what to do after an object. Select
the objects, then click:

- **Stop After**: the machine stops after the selected objects, for
  example to put on appliqué fabric.
- **Always Trim**: always trim after the selected objects.
- **Never Trim**: do not trim after the selected objects. A long gap
  becomes a jump.
- **Tie-In**: lock stitches where the selected objects start after a trim.
- **Tie-Off**: lock stitches where the selected objects end before a trim.

Object Properties > **Machine Functions** shows the same settings for one
object, with **Trim after: Automatic** (trim when the next object is far,
else a travel stitch) and **Sew slowly** (only U01 files keep it). The
stitch list shows each function; see [Checking a design](checking).
