---
id: sketch
title: Sketches
group: design
keywords: sketch, parametric, constraint, dimension, cad, exact size, line, arc, circle, rectangle, polygon, tangent, parallel, perpendicular, horizontal, vertical, equal, symmetric, fix, construction, fully constrained, free motions, conflict, trim, extend, split, fillet, chamfer, offset, mirror, pattern, relax, snap
---
# Sketches

A sketch is a drawing with exact sizes, as in CAD programs. You draw
lines, arcs and circles, then give them constraints (for example
horizontal, tangent or equal) and dimensions (lengths, angles and sizes).
When you change a dimension, the sketch changes and the constraints stay
true. A sketch sews nothing itself: you make embroidery from its closed
regions and its curves (see Make embroidery).

## Start and finish

- **Digitize > Start Sketch** makes a new sketch, or opens the selected
  sketch. A double-click on a sketch also opens it.
- The **Sketch** tab shows while the sketch is open. The other objects are
  dim and take no clicks.
- **Finish Sketch** closes the sketch. A sketch with nothing in it goes
  away.

Each change is one undo step; a drag is one step.

## Keys and the mouse

- **Esc** ends the command and goes back to **Select**. In Select, Esc
  clears the selection.
- **Delete** or **Backspace** deletes the selected parts with their
  constraints. A deleted point takes its curves with it.
- **Enter** or a right click ends a chain of lines.
- **Ctrl+A** selects all. **F8** shows the marks of the constraints and
  **F9** hides them (also **Show constraints** in the Properties panel).
- With Select, a drag of a part of the selection moves all of it. Ctrl+click or Shift+click adds a part to the
  selection; a drag on an empty place selects the parts in a box.
- The part under the pointer lights up before you click, with its
  constraints (also when the marks are off). A constraint under the
  pointer lights up its parts.
- A **right click** on a part selects it and shows the commands for it:
  change the value of a dimension, add a dimension or a constraint that
  fits, trim, split or extend at the click, fillet or chamfer a corner,
  construction, offset, mirror, patterns, delete its constraints, or
  delete it.
- **Undo** (Ctrl+Z) and **Redo** work as in the rest of the program.

## Draw

- **Line**: click the start, then each next point. Click the first point
  to close the shape; Enter, Esc or a right click ends the chain. Drag
  from the end of a line to draw an arc tangent to it; the chain then goes
  on from the end of the arc.
- **Rectangle**: two corners. The sides are horizontal and vertical.
- **Center Rectangle**: the center, then a corner.
- **Three-Point Rectangle**: two corners of a side (at any angle), then
  a point for the height.
- **Three-Point Center Rectangle**: the center, the middle of a side,
  then a corner.
- **Slot**: the kind is in the Properties panel. Center to center: the
  two centers, then a point for the width. Overall: the two ends, then
  the width. Center point: the middle, the center of one end, then the
  width. Three-point arc: the two centers, a point on the arc, then the
  width. Center point arc: the center of the arc, the two centers, then
  the width. The ends are half circles, tangent to the sides, with the
  same radius.
- **Tangent Circle**: click three lines; the circle touches all three
  (on the side of your clicks).
- **Circle**: the center, then a point on the circle.
- **Polygon**: the center, then a corner. Set the number of sides in the
  Properties panel. The corners are on a construction circle and the sides
  are equal. With **Polygon round its circle**, the sides touch the circle
  and the click is the middle of a side.
- **Three-Point Arc**: the start, the end, then a point on the arc.
- **Center Arc**: the center, the start, then the end (counterclockwise).
- **Tangent Arc**: click the end of a line, then the end of the arc. The
  arc goes on from the line, tangent to it.
- **Point**: a point, for example to put a constraint on.
- **Spline**: click the points of a smooth curve. Enter, a right click or
  a click on the last point ends it; a click on the first point closes it.
  The spline has a piece between each two points, with two handles (thin
  lines from the points). Drag the points and the handles to shape it; a
  tangent at each joint keeps it smooth. A spline can be tangent to a line
  or an arc at a shared end, and a point can be on it. Dimensions go on its
  points, not on the spline.

A click near a point of the sketch uses that point, so lines that meet
share their corner. **Construction** makes the selected curves
construction curves (dashed orange): they help to draw and do not become
embroidery. With nothing selected, it is on or off for new curves.

## Drawing help

While you draw, the program adds constraints. A mark next to the pointer
shows them before you click, and the points of your clicks show at once:

- A click near a point uses that point (coincident).
- A click near the middle of a line puts the point there (**M**, a
  diamond).
- A click near the place where two curves cross puts the point on both
  (**X**).
- A click near a curve puts the new point on the curve (**O**).
- A dashed line shows when the pointer is in line with a point (across
  or up and down): the click takes its x or its y.
- A line that is almost horizontal or vertical becomes so (**H**, **V**).
- A line that starts or ends at the end of an arc, in the direction of
  the arc, or that touches a circle in its direction, becomes tangent
  (**T**).
- A line that is almost at 90° to the line before it becomes
  perpendicular (**⊥**). Move the pointer over another line first, and a
  new line almost parallel or at 90° to it becomes so (**//**, **⊥**).

The tools show the shape that the next click makes.

Hold **Alt** while you click to add none of them (a click on a point
still uses the point). The check box **Add constraints while drawing** in
the Properties panel turns them off.

## Constraints

The constraint commands are the icons of the Constrain panel (the
tooltip gives the name). Select the parts first (click; Ctrl+click for
more), then click a constraint. Or click the constraint first, then the parts. The status line
says what to click.

| Constraint | Parts |
| --- | --- |
| Coincident | two points |
| Horizontal, Vertical | a line, or two points |
| Parallel, Perpendicular, Collinear | two lines |
| Tangent | a line and a circle or an arc, or two circles or arcs |
| Equal | two lines, or two circles or arcs |
| Concentric | two circles or arcs |
| Midpoint | a point and a line |
| On Curve | a point and a line, a circle or an arc |
| Symmetric | two points and the line between them |
| Fix | a point |

A mark next to the part shows each constraint (H, V, //, ⊥, T, =, ◎, M,
O, S, F). Select a mark and push Delete to remove the constraint.

## Dimensions

Click **Dimension**. The dimension that your next click makes shows as a
ghost at the pointer, so you see what you dimension. Click:

- a circle: its diameter; an arc: its radius;
- two points: the distance between them;
- a line, then an empty place: its length;
- a line, then a second line: the angle between them;
- a point and a line: the distance from the point to the line.

When the parts are picked, the ghost follows the pointer: click where the
dimension goes. For a slanted line or two points, the place chooses the
kind: above or below them a horizontal distance, at the left or the right
a vertical distance, else the distance along them. Right-click while the
dimension waits to choose the kind yourself: Aligned, Horizontal or
Vertical; for an arc Radius, Diameter or Arc Length.

A **Centerline** (Draw panel; select lines first) is a construction line
with long and short dashes. A dimension from a point to a centerline is
a diameter (⌀), as for a part that turns round the line. With Select, drag a dimension to move it; Reset Label
Place (right click) puts it back.

If the sketch sets the size already, the new dimension is **driven**: it
shows the size in parentheses and does not change the sketch. Right-click
a dimension for **Make Driven** or **Make Driving**.

Type the value and push Enter (Esc cancels; a click somewhere else keeps
the value). You can type a value in other units than those of the
program: `2 in`, `1 1/2"`, `3/4 in`, `50 mm`, `3 cm`. You can also
calculate: `2 in + 3 mm`, `120/4`, `(10 + 5) * 2`. A number with no unit
is in the units of the program, which you set in Options or in the
Properties panel of the sketch (mm or in). A double-click on a dimension
changes it again. The Properties panel lists all constraints and dimensions, with
**Change** for a value and ✕ to remove.

## Parameters

A parameter is a named value, for example `width = 80 mm`, or an
expression of other parameters, for example `border = width / 20`.

1. Click **Parameters** (Sketch tab, Dimension panel).
2. Type a name and a value or an expression, then click **Add**. Change an
   expression or a name in the list; ✕ removes a parameter that no value
   uses.
3. In the value box of a dimension, type the name of a parameter or an
   expression, for example `width`, `width - 2 * border` or
   `max(border, 3 mm)`. The label shows `width = 80.00`, or `ƒx` for
   another expression.

When you change a parameter, the dimensions that use it change, the
sketch solves, and the objects made from it follow. Expressions can use
units, `+ - * / ^`, parentheses, `pi` and the functions `sqrt`, `abs`,
`min`, `max`, `sin`, `cos` and `tan` (degrees). A number with no unit in a
sum with a length is in the units that the window shows (mm or in). A
parameter that uses itself, directly or through other parameters, is a
loop: the program refuses it.

**Templates**: save a design with a sketch that has parameters as a
template (**Save as Template**, with its objects). A new design from
that template asks for the values of the parameters, then makes the
sketch and its objects at that size.

## Sketches from artwork

- **Vector to Sketch** (Artwork tab, Convert; or right-click > Convert): the selected vector
  objects become one sketch. Straight parts become lines, round parts
  arcs and circles, other curves splines. The program adds the
  constraints that the drawing shows: horizontal and vertical lines,
  tangents where curves meet smoothly, and equal lengths and radii. Then
  you can add dimensions and change the shape.
- To use an SVG, PDF or EMF file, import it with **Import as Vectors**,
  then use Vector to Sketch.
- A **DXF** drawing (Import Artwork, or drop the file on the window) opens
  as a new sketch in the middle of the hoop, in the units of the drawing.

## Colors and free motions

- **Blue**: the part can still move.
- **Green**: the part is fixed: nothing can move it.
- **Red**: a constraint conflicts with the others. Remove it, or change a
  value.
- **Gray** marks: constraints that repeat others.

The status line gives the number of free motions. A sketch is fully
constrained when it has 0: it then needs a fixed point (Fix) or
dimensions from one, else it can still move as a whole.

## Modify

The values of these tools (the radius, the distances) are in the
Properties panel while the tool is on. Each edit is one undo step; an
edit that cannot be done changes nothing and gives a message.

- **Trim**: click the piece of a curve to remove. The piece goes from one
  curve that cuts it to the next. A curve that nothing cuts goes away; a
  circle with two cuts becomes an arc. The new ends stay on the curves
  that cut them.
- **Extend**: click near the end of a line or an arc. The end goes to the
  next curve that the extension meets, and stays on it.
- **Split**: click a line or an arc to cut it in two there. The two
  pieces stay in line (or on one circle).
- **Fillet**: click the corner of two lines. An arc of the radius rounds
  the corner, tangent to both lines; type the radius in the box. The
  corner stays as a point on both lines, so a dimension to the corner
  stays. A length of a line becomes the distance to the corner.
- **Chamfer**: click the corner of two lines. A line cuts the corner:
  the same distance on each line, two distances (the first on the older
  line), or a distance and an angle (set the kind in the Properties
  panel).
- **Stretch**: drag a box round the points to move, then click a base
  point and where it goes. The curves that end at the points stretch.
- **Offset**: select curves and click Offset, or click Offset and then a
  curve (it and the curves joined to it). The copy is at the distance:
  positive outside of a closed shape (else on the left), negative on the
  other side. An offset line stays parallel at its distance; an offset
  arc keeps its center.

## Move, copy, rotate and scale

Select the parts first.

- **Move**: click a base point, then where it goes. Or type X and Y in
  the Properties panel and click **Move by the Values**.
- **Copy**: as Move, with a copy; the parts stay.
- **Rotate**: set the angle in the Properties panel (positive is
  counterclockwise), then click the center.
- **Scale**: set the factor, then click the base point. The dimensions
  between the parts scale too.

A dashed preview shows where the parts go. Rotate and Scale can copy
(**Copy** in the Properties panel). The parts move as near as their
other constraints let them.

## Copy

Select the curves first.

- **Mirror**: click Mirror, then the line to mirror about. Each point of
  the copy is symmetric to its point (a point on the line is shared), so
  the copy follows when you change the curves.
- **Rectangular Pattern**: set the columns, the rows and the spacing in
  the Properties panel, then click **Make Pattern**.
- **Circular Pattern**: set the copies and the angle (360° all round),
  then click the center.

The copies of a pattern have the constraints of the curves between them,
but their places are free.

## Relax

When constraints conflict (red), **Relax** removes them, dimensions
first, until no conflict is left. The message and the Properties panel
list what it removed. Undo brings them back.

## Automatic dimensions

**Automatic Dimensions** (Dimension panel) adds what the selected curves
(or all curves) need: horizontal and vertical for lines that are almost
so, then lengths, diameters, radii and angles, with the sizes of now. It
adds nothing that repeats or conflicts. The place of the sketch stays
free: add a **Fix** to a point to place it.

## Center points

Select points and click **Center Point** (Draw panel) to show them as a
cross, for example the place of a hole.

## Relax mode

**Relax Mode** (Dimension panel) is on or off. While it is on, a drag
changes the dimensions that hold the parts: they take the new sizes. The
other constraints (horizontal, tangent and so on) stay.

## Make embroidery

The curves (not the construction curves) make closed regions where they
cross: a circle in a circle makes a ring and a disc; a shape in a region
is an island, and the region gets a hole there.

1. Click **Regions** (Sketch tab), then click inside the regions that you
   want (they are shaded; click again to unpick). With no region picked,
   the commands take all regions.
2. In **Make Embroidery** (Properties panel), or with a right click,
   click **Fill**, **Satin**, **Appliqué** or **Vector**.

For curves (also construction curves): select them, then **Run**, **Satin
Line** (the width is next to it) or, for two curves, **Satin Between** (a
satin column with the two curves as its sides).

The new objects take the current properties and the current thread, and go
at the end of the sew order. When the sketch is closed, its Properties
panel makes embroidery of all its regions, or runs along all its curves.

## Objects that follow the sketch

The objects that you make from a sketch stay linked to it. When you change
a dimension or a constraint, the program makes them again. They keep their
stitch settings, their thread and their place in the sew order. Undo takes
back the change and the new objects in one step.

- To change the sketch again, double-click one of its objects, or click
  **Edit Sketch** (right-click menu or Properties panel).
- When you move, turn or scale the sketch, its objects move with it. When
  you move only the objects, they stay at that distance from the sketch.
- If a change opens a region or deletes a curve, its objects stay as they
  are, and the program shows a warning. Close the region again, or click
  **Unlink** to make them plain objects.
- Shaping commands do not take linked objects. Unlink them first. A change
  of the shape of a linked object by hand does not stay after the next
  change of the sketch.

## Drag

With **Select**, drag a point or a line: the sketch moves with it and the
constraints stay true. Drag the edge of a circle to change its size.
