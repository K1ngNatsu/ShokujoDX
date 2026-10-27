# ADR 0032: Hoop placement and hoopings

Date: 2026-10-23 · Status: Accepted (P7)

## Context

P7 adds a hoop that turns by any angle, hoop placement (manual or auto
centering), auto start and end of the design, and large designs that sew
in several hoopings (roadmap P7, `docs/design/p7-plan.md`).

Before P7, the machine coordinates came from the design coordinates in
two steps: the quarter turn of the design (`Design::rotation`) about the
hoop center, then a move of the hoop center to the origin. Many places
in the app and the engine did these two steps themselves.

## Decision

- **One output transform.** `Design::output_transform` takes design
  coordinates to machine coordinates: the quarter turn, then the hoop
  angle turned back about the hoop center, then the hoop center to the
  origin. `Design::hoop_transform` is the part from view coordinates
  (after the quarter turn) to the hoop frame. The stitch plan, the hoop
  checks, the worksheets and the canvas use these functions only.
- **The hoop angle turns the hoop, not the objects.** The canvas shows the
  hoop turned by `HoopPlacement::angle_deg`. The stitch plan turns the
  stitches back, so in the machine file the hoop is straight. The fill
  and satin stitches do not change (their angles are relative to the
  objects); only the entry and exit points can change, because they come
  from the needle position (ADR 0007).
- **Auto centering is a placement setting.** With `HoopPlacement::auto_center`,
  each edit of the design (`Document::edit` and the other edit functions)
  puts the hoop center at the center of the design extents, in the same
  undo step. Manual placement (the default) keeps the hoop where the user
  put it.
- **Auto start and end are part of the plan.** `Design::start_end` sets a
  start point and an end point (design center, hoop center or a point).
  The plan starts with a jump to the start point, so the first object
  starts from there (ADR 0007), and ends with a trim and a jump to the end
  point. No stitch sews at the points. VP3 (the format of the Topaz 20)
  has no jump record: a jump is part of the next stitch, so in a VP3 file
  the start point sets only where the first object starts, and the needle
  stays at the last stitch. DST and the other formats with jumps keep both.
- **Places on the design box are in the hoop frame.** "Left" or "center"
  of the design is on the box of the objects as they sit in the hoop (the
  hoop angle applied), so the left end of a bar on a slant in a turned
  hoop is its left end in the machine file too.
- **Hoopings are made from a layout, not kept.** (Milestone 2.) The design
  keeps the layout of the hoopings (`Design::hooping`). The objects of each
  hooping come from the layout when they are needed (`Design::hoopings`),
  so an edit of the design changes all hoopings and there is no copy to
  keep up to date. Each hooping is a design of its own, with its hoop
  center at the hooping center, so the stitch plan, the formats and the
  checks need no change.
- **Zones from split lines.** The split lines (each one goes on past its
  ends) cut the plane into zones; a zone goes to the field that holds most
  of the design in it. This works for the rows and columns of the auto
  layout and for lines that the user draws (Milestone 3).
- **Cut only what does not fit.** An object that fits the hooping that has
  most of it is not cut, so it has no join. Areas are cut with a small
  join overlap (1 mm), so no fabric shows; lines are cut with no overlap,
  so no stitch sews twice. Satin columns and lettering are not cut in this
  milestone: a warning says to move a split line or to change the object.
- **Two overlaps.** The field overlap (20 mm) is how far the fields of
  two hoopings next to each other overlap in the auto layout: the space
  for the registration marks. The join overlap is how far a cut area goes
  into the next hooping.
- **Multi-position hoops are hoopings with a fixed place.** (Milestone 4.)
  A hoop preset can have positions; each one is a hooping whose field is
  the field of the position, and the hoopings of one hoop move and turn
  together. So the cut, the output and the sheet are the same as for
  multi-hooping. There are no registration marks: the fabric stays in the
  hoop.
- **Registration marks** are crosses in a thread of their own, where both
  fields have space and the design does not (when it can). The first
  hooping sews them last; the next hooping sews them first, and the color
  change stops the machine, so the user sees if they sew on the old marks
  before the design goes on.

## Consequences

- A design file of P6 opens with angle 0, manual placement and no auto
  start and end, so its stitches do not change.
- Code that needs machine coordinates must call `output_transform`; a
  test checks that the plan and the hoop checks agree for a turned hoop.
