---
id: digitizing
title: Drawing objects
group: design
keywords: draw, digitize, line, manual, freehand, shape, fill, complex fill, free shape, column, satin, column pairs, column c, rectangle, round rect, ellipse, star, ring, turning, curve, corner, right click, property bar, reshape, node, handle, start point, end point, angle, typed length, shift, 15 degrees, snap, midpoint, center, edge, line up, drag handle, direction line, stitch direction, hole, knife, borders, buttonholes, blends
---
# Drawing objects

The tools of the **Digitize** tab draw new objects. Each tool stays on
after an object, so you can draw more. Push **Esc** to stop.

## Clicks and keys

- A **left click** makes a corner point.
- A **right click** makes a curve point: the line goes through it
  smoothly.
- **Enter** finishes the object. A click on the first point closes a
  shape.
- **Backspace** removes the last point. **Esc** cancels.
- The status line tells what to do next.
- The property bar sets the settings of new objects, for example the
  stitch length or the column width. These settings are the current
  properties (see [Fabrics, styles and templates](fabrics-and-styles)).
- Snap to the grid, the guides or other objects (View tab) works while
  you draw.

## Drawing help

- **Drag** to make a curve point with a handle: push the mouse button
  where the point goes, and move to where the curve goes. A short drag
  makes a corner.
- Hold **Shift** to keep the line from the last point at a step of 15°.
- The length and the angle from the last point show at the pointer. The
  angle goes counterclockwise from the right: 0° is to the right, 90° is
  up.
- Type a length and push **Enter**: the next point goes that far, toward
  the pointer. Type a length and an angle, for example `25<45`, for a
  point at that angle. You can type units and calculations, for example
  `1 in` or `12+3`. **Backspace** and **Esc** change the text first.
- With **Snap objects** on, the points snap to the nodes, the midpoints,
  the centers and the edges of other objects (a mark with its name
  shows). They also line up with the points that you placed (a dashed
  line shows).

## Lines

- **Line**: a running stitch line. Left click: corner; right click: curve
  point; Enter: finish.
- **Manual**: each click is one needle point. Use it for a few exact
  stitches, for example a tie-down.
- **Freehand**: drag to draw. The program smooths the line.
- **Smart Drawing**: drag to draw. A stroke near a straight line, a
  rectangle, a triangle, a circle or an ellipse becomes that clean shape
  (a level line or rectangle if it is near level). Other strokes stay
  as drawn. **Ctrl+Z** gives back the stroke.

The property bar of a line has the run type (**Run**, **Backstitch**,
**Stem stitch**, **Sculpture run**, **Manual**), **Triple** (each stitch
three times, a bean stitch), the stitch length, and the pass: **Once**,
**Backtrack** (forward and back), **Repeat** (two times), **Redwork** or
**Branching** (all lines of the object as one network). See [Stitch
types](stitch-types).

## Fills

- **Shape**: a filled area (tatami). Click round the area; Enter or a
  click on the first point finishes it.
- **Complex Fill**: an area with holes. Click the outline, Enter; then
  each hole, Enter; then Enter again to finish.
- **Free Shape**: drag round the area. The program smooths the outline.
- **Turning**: a fill whose stitches turn. Click the outline, Enter; then
  click stitch angle lines across the shape (two clicks each); Enter.
- **Smart Fill**: click inside an area that lines or the outlines of
  other objects close all round. The area becomes a fill, with holes for
  the shapes inside it. The lines can cross; gaps up to 0.3 mm close.

## Columns

- **Column**: a satin column. Click the points of one side and push
  **Enter**; then click the points of the other side and push **Enter**.
  While you draw the second side, the satin fills the space between the
  two sides. Right-click for curve points.
- **Column Pairs**: click a point on one side, then the matching point on
  the other side, pair by pair. Each pair sets the direction of the
  stitches there. Enter: finish.
- **Column C**: a column of fixed width along a line. The property bar
  sets the width.
- **Ring**: a satin ring. Drag corner to corner. The property bar sets
  the width.

Columns sew best from about 1.5 mm to 7 mm wide. Wider columns get extra
needle points (Object Properties > **Split stitches over**), or use a
fill.

## Shapes

**Rectangle**, **Round Rect**, **Ellipse** and **Star** draw filled
shapes. Drag from corner to corner. **Ctrl** makes equal sides (a square
or a circle); **Alt** draws from the center. The property bar sets the
corner radius of a round rectangle and the number of points of a star.

## Decorate

- **Borders**: decorative borders (satin, runs, blanket, rope, motifs)
  round the selection, or round the whole design if nothing is selected.
- **Buttonholes**: one buttonhole, or a string of buttonholes along the
  selected line. Set the opening, the leg width, the gap for the cut,
  the bar tack and the density. Cut each opening between the legs after
  sewing.
- **Blends**: two or more colors in one fill or satin column. Each layer
  goes from dense to open, so the colors mix.

See [Stitch types](stitch-types) for the settings.

## Appliqué

**Appliqué** draws the outline of a fabric piece. Click the outline,
Enter; then holes, if any; then Enter again. The program sews a placement
line, stops, sews a tack-down, stops so you can cut the fabric, and sews
a cover satin. See [Tutorial: an appliqué](tutorial-applique).

## Change the shape: Reshape

1. Select the object and click **Reshape** (Digitize > Select).
2. Drag a node to move it. Squares are corners; circles are curve nodes.
3. Drag a handle to change a curve. **Alt** while you drag makes a
   corner. Double-click a node to change it between corner and curve.
4. Double-click the outline to add a node. **Delete** removes the
   selected node. **B** (or right-click > **Break at Node**) cuts a line
   in two at the selected node; a closed line opens there.
5. The green point is where the object starts to sew; the red point is
   where it ends. Drag them to move them.

With a lettering object, Reshape selects one letter: move, turn or scale
it. With a stitch object (from a machine file), Reshape moves, adds and
deletes single needle points.

## Stitch angles, holes and cuts

These tools are on the **Edit > Shape** panel:

- **Angle**: click two points across a fill or a satin to set the stitch
  direction.
- **Direction Line**: click points of a line through the selected satin
  or fill, then push **Enter**. The stitches go across the line, so they
  turn where the line turns. The property bar sets the angle of the
  stitches to the line (90°: square to it). A fill with the satin stitch
  type keeps the line: Object Properties changes its angle or removes it.
  On another fill, the line sets the fill angle.
- **Add Hole**: draw a hole in the selected fill.
- **Knife** (Edit > Split and Join): cut the selected objects along a
  line. Click two points.

See [Changing objects](editing) for all the edit commands.
