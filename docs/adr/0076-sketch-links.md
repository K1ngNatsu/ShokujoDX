# ADR 0076: Objects that follow their sketch

Date: 2026-10-27 · Status: Accepted (P14 Milestone 5)

## Context

Milestone 4 (ADR 0075) makes fills, satins, appliqué, vectors and runs
from the regions and curves of a sketch, but the objects are free: after a
change of a dimension, the user must make them again and set their
settings again. Milestone 5 links them to the sketch, as the linked shapes
of ADR 0026 follow their originals.

## Decision

1. **A record for each make** (`Design::sketch_links`,
   `emb_model::sketch_link::SketchLink`): the sketch object, the source
   (`LinkSource`: a region with its key, a curve, or two curves for a
   satin column), how the objects were made (`RegionTo`, `CurveTo`, the
   column width, the vector color) and the ids of the objects.
   `Design::make_from_sketch` makes the objects and the record. Each make
   command of the sketch (the Regions tool, Run, Satin Line, Satin Between,
   the buttons of a closed sketch) goes through it.
2. **A region key** (`emb_sketch::RegionKey`): the pieces of curves round
   the region, each with its direction (the region is on the left of the
   walk), and a point well in the region. Curve ids stay the same when a
   dimension changes, so the same sides find the same region; the
   direction tells a ring from its disc and the two sides of a line
   across a shape. When the curves change (a fillet adds an arc, a trim
   cuts a line), the region is the smallest one round the point of the key
   that has more than half of the curves in common
   (`Sketch::find_region`). After each change the key takes the new sides
   and point, so it follows the sketch step by step.
3. **Made again after each change of the sketch.** `AppState::edit_sketch`
   and `try_edit_sketch` call `Design::refresh_sketch_links` in the same
   edit, so the change and the new objects are one undo step (a drag is
   one step). The objects keep their ids, their places in the sew order
   (one for one in place; if the count changes, all in the place of the
   first), their thread, functions, group and name, and their stitch
   settings (the settings of each old object go to the new object of the
   same type; a vector keeps its style, an appliqué piece its layer and
   fabric color).
4. **A source that is gone** (a region that opens, a deleted curve): the
   objects stay as they are, and the record is lost (`SketchLink::lost`).
   The status line gives a warning; the Properties panel of the object and
   of the sketch shows it, with Unlink. A later change that closes the
   region again makes the objects again.
5. **Moves.** A move, scale, turn or mirror of the sketch (through
   `Design::transform_objects` or a drag, `linked_objects_moved`) moves its
   objects the same way. A move of the objects without the sketch goes
   into the record (`offset`, after the sketch), so they stay where the
   user put them after a change of the sketch; the other objects of the
   same record move with them.
6. **Edit Sketch again**: a double-click on an object made from a sketch,
   Edit Sketch in its right-click menu or its Properties panel, opens the
   sketch. Unlink (Artwork tab, or the Properties panel) ends the records
   of the selected objects, or of all objects of a selected sketch.
7. **Other commands.** Made objects are linked (`Design::is_linked`), so
   shaping and the commands that replace objects refuse them ("unlink
   first"), as for ADR 0026. A conversion (To fill, To satin, To vector)
   keeps the link and changes how the record makes the objects
   (`LinkSource::converted`). A deleted object leaves its record; a
   deleted sketch ends its records (`Design::remove_object`).

## Consequences

- Each change of a sketch with made objects finds the regions again (a
  few milliseconds for normal sketches) and gives the objects new
  revisions, so their stitches are made again.
- Settings in design coordinates (a fill direction line, start and end
  points) stay where they are after a change of size; the user may set
  them again.
- A change of the shape of a made object by hand (Reshape) does not stay
  after the next change of the sketch: unlink it first.
- The project file gets the `sketch_links` list; designs without links do
  not write it.
