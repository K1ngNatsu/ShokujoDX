# ADR 0025: Border trace

Date: 2026-10-23 · Status: Accepted (P6 Milestone 4)

## Context

The owner asked (2026-10-22) for one command that puts a border round a
selected object in a thread of its own, with a thickness, and cuts the
object and its border out of the objects that sew before it, so no area
sews twice. Two options: a gap (no border stitches, only the cut, so the
fabric shows as a line round the object), and the sew order of the
object and its border (fill then border, or border then fill). A change
of the thickness or the gap must update the cut.

Offsets and Remove overlaps (Milestone 3) can make each part, but the
result is plain objects: a later change needs the user to put the cut
objects back by hand.

## Decision

- **A record links the parts** (`Design::border_traces`,
  `emb_model::trace::BorderTrace`): the traced objects, the border object
  (none with no border), the settings (`TraceSpec`: border width, gap
  width, border stitch, thread, sew order), and for each cut object a
  copy of it as it was before the cut, with the ids of its pieces.
- **A border, a gap, or both** (changed the same day at the owner's
  request; the first form had one or the other). The gap is round the
  border, or round the objects with no border. The border stitch is a
  column of any column type, or a run or a triple run in the middle of
  the border width. Files in the first form (`mode`, `width_mm`) still
  read.
- **A change puts the cut objects back and cuts them again.** The stitch
  settings of the first piece go back to the object first, so a change
  that the user made on a piece stays. Removing a trace puts the cut
  objects back and removes the border; the traced objects stay.
- **Geometry.** A column border is a line with a width (`LineBorder`)
  whose inner edge is 0.4 mm over the edge of the objects, so no fabric
  shows between them. With no gap, the objects under it are cut 0.4 mm
  less far, so they go under the border; with a gap, they are cut at the
  outer edge of the gap. The border sews just after the traced objects
  or just before them, and is in one group with them, so they move
  together.
- **The trace follows moves** of its objects and of the cut pieces (ADR
  0026).
- **Only objects under the traced objects are cut**: the objects with an
  area that sew before them (as Remove overlaps). An object can be in
  one trace only.

## Consequences

- The project format gets the `border_traces` list; designs without
  traces do not write it.
- A copy of each cut object is in the file twice (as the record copy and
  as its pieces). The files are a little larger; the update is exact.
- If the user deletes a traced object or a piece, the trace still
  updates and removes with what is left.
