# ADR 0069: Sketch objects and sketch mode

Date: 2026-10-27 · Status: Accepted (P14 Milestone 2)

## Context

The sketch model and solver (ADR 0068) need a place in the design and a
way to draw and edit them in the app. CAD programs have a sketch mode: the
user goes into a sketch, draws and constrains it, and goes out again.

## Decision

1. **An object kind with no stitches.** `ObjectKind::Sketch` holds an
   `emb_model::sketch::SketchObject`: the sketch (`emb_sketch::Sketch`)
   and its placement. It sews nothing, as a vector object (ADR 0024).
   The object keeps the path of each curve in design coordinates, made
   again after each change, so the box, the hit test, the transforms and
   the drawing work as for other objects. The project file keeps the
   sketch and the placement; the paths are made again when the file
   opens.
2. **Sketch coordinates and a placement.** The sketch has its own
   coordinates, so its horizontal and vertical constraints stay true when
   the object turns. A move, a turn or a mirror of the object goes into
   the placement. A scale scales the sketch itself, its radii, its fixed
   places and its dimensions (not its angles), so a dimension stays real
   mm. A scale that is not the same in x and y stays in the placement.
3. **Sketch mode is app state** (`AppState::sketch`, `emb-app/src/sketch.rs`):
   the open sketch object, the tool, the clicks, the picks of a constraint
   command, the selection of points, curves and constraints, and the
   cached analysis. Start Sketch (Digitize > Shapes) opens the selected
   sketch or makes a new one; a double-click opens a sketch. The Sketch
   ribbon tab shows only in sketch mode. The other objects are dim and take
   no clicks. Finish Sketch closes it; an empty sketch is removed.
4. **Each change is one undo step.** `AppState::edit_sketch` changes the
   sketch through `Document::edit` and solves it; a drag uses
   `edit_merged`, so one drag is one step. An undo that removes the open
   sketch ends sketch mode.
5. **Commands.** Draw tools: a chain of lines (a click on the first point
   closes it), rectangle (with horizontal and vertical sides), circle,
   polygon (corners on a construction circle, equal sides), three-point,
   center and tangent arcs, point, and construction curves. A click near a
   point of the sketch uses that point. A constraint command takes the
   selected parts, or waits for clicks. The Dimension tool makes a length,
   a diameter, a radius, a distance or an angle with the value as drawn,
   and opens a box for the value (in the units of the Options; an angle in
   degrees).
6. **The state on the screen.** Free parts are blue, fixed parts green,
   the parts of a conflict red, construction curves dashed orange, and
   repeated constraints gray. The status line gives the free motions.

## Consequences

- No embroidery comes from a sketch yet: Milestone 4 makes objects from
  its regions, Milestone 5 links them.
- Constraints that the program adds while the user draws (other than
  shared points and the sides of rectangles and polygons) come in
  Milestone 3.
- A sketch object has the machine functions of other objects in the
  Properties panel, with no effect, because it sews nothing.
