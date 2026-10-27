# ADR 0073: Sketch transforms, dimension kinds, centerlines, relax mode

Date: 2026-10-27 · Status: Accepted (P14, the owner's third request)

## Context

The owner asked for the next sketch features of the reference CAD program
(the table in `docs/design/p14-plan.md`): a tangent
arc by a drag from the end of a line; move, copy, rotate and scale with
values; horizontal and vertical dimensions by the place of the click; arc
length; centerlines; the constraints of a part when the pointer is on it;
and relax mode.

## Decision

1. **Tangent arc by a drag.** With the Line tool, a drag from the end of a
   line makes a tangent arc to where the drag stops (the Tangent Arc tool
   on a copy of the sketch shows it while the drag goes on); the chain of
   lines then goes on from the end of the arc.
2. **Move, Copy, Rotate, Scale** (`Sketch::transform_parts`). They take
   the selected parts. Move and Copy: a base point, then where it goes
   (snaps work), or X and Y in the Properties panel. Rotate: a click on
   the center, by the angle of the panel (counterclockwise on the screen;
   the y of the design goes down). Scale: a click on the base point, by
   the factor of the panel; the radii and the dimensions between the
   parts (not the angles) scale too. A move goes as a drag: as near as
   the other constraints let it. A copy has the constraints between the
   copied curves. A dashed preview shows the result at the pointer.
3. **The kind of a distance by the place of the label** (`orient`): above
   or below the two points (between their x) a horizontal distance, left
   or right (between their y) a vertical distance, else along them; a
   right click while the dimension waits for its place chooses the kind
   (Aligned, Horizontal, Vertical; Radius, Diameter, Arc Length), and the
   place then does not change it.
4. **Arc length** is a dimension (`Constraint::ArcLength`): the radius
   times the angle of the arc from its start to its end.
5. **Centerlines** (`Curve::centerline`, also construction lines): a
   distance from a point to a centerline is a diameter (the value is the
   diameter; the equation uses half of it). A change to or from a
   centerline keeps the geometry (the values change by 2).
6. **Hover**: the constraints of the part under the pointer light up (and
   show when the marks are off); the parts of the constraint under the
   pointer light up.
7. **Relax mode** (a toggle on the Sketch tab): during a drag the driving
   dimensions are driven, so the drag changes them; after it they drive
   again with their new values. The other constraints stay.

## Consequences

- A move of parts that a fixed point or a dimension to other parts holds
  goes only as far as the constraints let it; the message says so.
- Rotate turns by the value of the panel; a turn by two clicks (from one
  direction to another) is not there.
