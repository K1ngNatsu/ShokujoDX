# ADR 0079: Splines, and sketches from vectors, SVG and DXF

Date: 2026-10-27 · Status: Accepted (P14 Milestone 6)

## Context

Milestone 6 (b) and (c) of the P14 plan: splines through points with
constraints on their points and ends, and sketches made from vector
objects, imported SVG files and DXF drawings, with the constraints that
the program finds.

## Decision

1. **A spline piece is a cubic Bézier curve** (`Geometry::Bezier`: the
   start, two handles and the end are points of the sketch). The points
   and the handles drag and take constraints as all other points, the
   solver needs no new kind of variable, and the curves of SVG files are
   cubic curves already. `Shape::Cubic` gives the point, the direction and
   the nearest point (samples, then Newton steps), and the crossings with
   lines, circles and other cubics (a polyline of 48 pieces, then Newton
   steps on one or both parameters), so regions, snaps and the click
   tests work with splines.
2. **A spline through points** (`Sketch::add_spline`, the Spline tool of
   the Draw panel): one piece between each two clicks, with handles from
   the points before and after (a Catmull-Rom curve), and a tangent
   constraint at each joint so the spline stays smooth when its points
   move. Enter, a right click or a click on the last point ends it; a click
   on the first point closes it. Its ends join the points or curves that
   the first and the last click snapped to. A drag of a point of a spline
   moves its handles with it (as in vector programs).
3. **Constraints on splines**: a tangent between a spline piece and a
   line, an arc or another piece at an end that they share (the directions
   away from the end are opposite: the angle between them is 180°, so the
   solver cannot make a cusp; with an arc, square to its radius); a point
   on a spline piece (the signed distance to the curve). Dimensions do not
   take a spline (give its points dimensions). Trim, extend and split
   refuse spline pieces for now.
4. **Sketches from drawings** (`emb_sketch::import`): lines stay lines; a
   cubic with its handles on the chord is a line, a cubic on a circle
   (within 0.05 mm) is an arc, arcs of one circle in a row join, and a full
   turn is a circle; other cubics are spline pieces. Ends nearer than
   0.05 mm are one point, and so are centers (concentric arcs and
   circles). The program finds horizontal and vertical lines (within 1°),
   tangents where two curves (not two lines) meet smoothly, and equal
   lengths and radii. Then it solves, and removes the constraints that the
   others make true already, again until there are none, so the sketch has
   no repeated or conflicting constraints.
5. **DXF** (text form): LINE, ARC, CIRCLE, LWPOLYLINE with bulges and
   POLYLINE with VERTEX; `$INSUNITS` gives the units (else mm). The y axis
   of a DXF goes up, so the drawing is turned over (an arc keeps its side).
   The format is public; the reader is our own code.
6. **The app**: Vector to Sketch (Artwork tab > Convert, also for the
   vectors of an imported SVG, PDF or EMF; also in Convert of the
   right-click menu) puts one sketch in the place of
   the selected vectors, in one undo step. A DXF file (Import Artwork, or a
   drop on the window) opens as a new sketch in the middle of the hoop.

## Consequences

- A spline piece has no length dimension; constraints between its handles
  and other parts are possible, as for other points.
- Smooth joints of the second order (G2) and spline edits (trim, split)
  are later steps.
