# ADR 0075: Sketch regions to embroidery

Date: 2026-10-27 · Status: Accepted (P14 Milestone 4)

## Context

A sketch (ADR 0068, 0069) sews nothing. Milestone 4 makes embroidery from
it: the closed regions that its curves make (where curves cut each other,
with holes and islands) become fills, satins, appliqué or vectors, and its
curves become runs, satin lines, or a satin column between two curves.
Milestone 5 links the objects to the sketch; here they are free objects.

## Decision

1. **Regions in the engine** (`emb_sketch::regions`, `Sketch::regions`).
   The curves that are not construction curves are cut where they cross
   (the crossings of `shape`) and at their ends; the pieces are the edges
   of a plane graph (points nearer than 0.1 µm are one point). Edges with a
   free end go, again and again, so open lines close nothing. Each face is
   a walk that, at each point, takes the edge just clockwise of the way
   back (the face is on the left). Walks with a positive area are the
   regions; walks with a negative area are the outsides of groups of
   curves, and each is a hole of the smallest region round a point just
   outside it (an island). A circle with no cut is one loop. The direction
   at a point is to a point 0.1 mm along the edge, so a curve that touches
   another (a tangent circle) sorts in the right order.
2. **The paths** of a region keep the exact curves: lines, and cubic
   curves of at most 90° for the pieces of arcs and circles (either way
   round). A region gives its outline, its holes, its area less the holes,
   and the curves round it (for Milestone 5).
3. **Conversions in the model** (`emb_model::sketch`): `region_objects`
   with `RegionTo` (Fill; Satin as Convert to satin of a vector: a column
   where the shape lets it, else a fill of satin columns; Appliqué with
   one piece and its holes; Vector), `curve_objects` with `CurveTo` (Run,
   a satin line of a width), and `SketchObject::satin_between` (the two
   curves as the rails, the second one turned if its ends are nearer the
   other way). The stitch settings are the current properties of the
   design (P3) and the thread is the current thread.
4. **The app**: the Regions tool (Sketch tab) picks and unpicks regions by
   a click inside (shaded; the region under the pointer has a light
   shade); the picks are points, so they stay right after the sketch
   changes. The Properties panel (Make Embroidery) and the right-click menu
   make the objects from the picked regions, else from all regions, and
   from the selected curves (Run, Satin Line, Satin Between). A sketch that
   is not open has buttons for all its regions and Runs Along the Curves.
   Each make is one undo step; the new objects go at the end of the sew
   order.
5. **Corpus design `sketch-badge`**: a badge sketch, fully constrained;
   its regions become a satin ring, a fill with the hexagon as a hole and
   a fill of the hexagon (an island), and a construction line becomes a
   satin line.

## Consequences

- Two curves that lie on each other (collinear lines on one line, the
  same circle twice) do not cut each other; the region walk then sees two
  edges. Draw such shapes without overlaps.
- The objects do not change with the sketch yet (Milestone 5; done in ADR 0076).
