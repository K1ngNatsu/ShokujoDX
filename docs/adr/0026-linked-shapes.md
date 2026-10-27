# ADR 0026: Linked shapes

Date: 2026-10-23 · Status: Accepted (P6, owner request)

## Context

Flatten, Divide and Remove overlaps (Milestones 2 and 3) cut objects
once. After a move of one of the objects, the cut no longer fits: the
owner had to undo, move and do the command again. The owner asked
(2026-10-23) for the objects to stay linked, so that a drag after a gap
or a flatten keeps the cut right. Border traces (ADR 0025) have the same
need.

## Decision

- **A record keeps the cut objects as they were** (`Design::shape_links`,
  `emb_model::link::ShapeLink`, with the same `TraceCut` as a border
  trace): each original and the ids of its pieces, and, for Remove
  overlaps, the objects that cut.
- **A move makes the cut again.** Each move, scale, turn and mirror goes
  through `Design::transform_objects`, or (a drag on the canvas, which
  places the objects again each frame) calls
  `Design::linked_objects_moved` with the step of the frame. The stored
  original of a moved piece moves by the same step; then each record with
  a moved object puts its originals back and cuts them again.
- **The pieces keep their ids** when the cut is made again (while there
  are as many), so the selection and the drag on the canvas go on.
- **The stitch settings, the thread and the functions of the first
  piece go back to its original** before each new cut, so changes of
  settings stay. A change of the shape of a piece by hand does not stay:
  the user unlinks first (Artwork > Unlink, which ends the record and
  keeps the objects as they are).
- **Linked is the default** for Flatten, Divide and Remove overlaps
  ("Keep linked" in the Shaping panel). The other shaping commands make
  one result from several objects, which cannot follow a move of one of
  them; they stay plain. Border traces are always linked.
- **An object is in one record at most.** Shaping commands refuse linked
  objects ("unlink them first"). So do the commands that replace objects
  with new ones (Join, Break apart, the knife, Split at needle), because
  the record would lose them.
- **One object stays one object** (owner report, the same day). A cut
  that leaves several separate areas of an object gives one object with
  all of them (one fill can have several outer shapes, as Join makes),
  not one object for each area; only Divide makes an object of each
  piece. To fill makes one fill of a vector with several shapes. The
  fill engine keeps an outer shape that is inside a hole (an island).
- **Conversions keep the link** (`Design::convert_linked`): To fill, To
  satin, To outline and To vector change a linked object in place, with
  its id, and change its stored original the same way, then cut again.
  A conversion that would make more than one object of a linked object
  is refused.
- **New object ids never take the id of a kept original**, so an
  original can always come back with its own id.

## Consequences

- Each drag frame of a linked object makes the cut again; this is a few
  boolean operations, fast for normal designs.
- Changes that are not moves (Reshape of a linked object) do not make
  the cut again; the next move does.
- The project format gets the `shape_links` list; designs without links
  do not write it.
