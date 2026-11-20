# ADR 0134: Recognize Objects

Date: 2026-11-20 · Status: Accepted (P18 Milestone 4,
`docs/design/p18-plan.md`)

## Context

Milestones 1 to 3 (ADR 0131 to ADR 0133) make running objects, satin
columns and fills of the stitches of a machine file in the engine.
Milestone 4 gives the user one step to change a stitch object into these
objects, with a look at the result first.

## Decision

- **Engine**: `emb_art::part_objects::recognize(kind)` makes a stitch list
  of a stitch object (`stitch_design_of`: a jump to the start of each of
  its parts, a trim where the object has one) and the objects of its parts.
  `stitches_of` makes a stitch object of the records that stay stitches;
  `Made::stitch_count` counts the stitches of an object.
- **Command**: Artwork > Convert > **Recognize Objects** (`Cmd::RecognizeObjects`,
  next to Recognize Outlines) works on the selected stitch objects.
- **Window** (`emb-app/src/recognize_objects.rs`, docked like the other
  command windows):
  - Two pictures next to each other: the stitches of the file, and the
    objects sewn again with the plan options of the design (runs blue,
    satins orange, fills green, stitches gray).
  - Red cells where the two differ: cells of 1 mm (or more, for at most 80
    cells on the long side) where one side has 0.8 mm of thread for each
    mm of the cell side and the other side less than 0.4 of it. The window
    gives the stitch counts and the area that differs.
  - A list of the objects: the type and its settings (as the Sequence
    panel shows them, `sequence_panel::kind_detail`) and the stitch counts
    before and after. A box keeps an object as stitches; the row under the
    pointer shows the lines of its object in the picture.
  - Accept makes the change as one undo step (`actions::replace_with`:
    the objects get the thread, the group and the functions of the stitch
    object, the stop on the last one). Stitches kept one after the other
    are one stitch object. Recognize All and Keep All as Stitches set the
    boxes; Cancel changes nothing.
- **Open**: Options > **Recognize objects when a file opens**
  (`Settings::recognize_on_open`, off by default) makes the objects at
  once when a machine file opens (`recognize_objects::recognize_all`).
  The needle pins of a file (TBF, U01) belong to its threads, so they
  stay.
- **Changes of the fills** (ADR 0133), found with the window:
  - A turn is any leg between two rows that go back and forth: where the
    edge of a fill is almost along the rows, a turn is longer than the
    rows.
  - The line of a row is the median of its needle points, and its length
    comes only from the needle points on that line. Where the turn goes
    on in the direction of the row, it is a part of the leg, and the area
    went out by half a row there.
- **Exit test**: the owner sews a machine file of a logo with lettering,
  changed into objects, at 150 % of its size. The automated part is the
  fourth test of `crates/emb-cli/tests/parts.rs`: six corpus designs with
  lettering and logos, as DST and back, changed into objects and scaled to
  150 %, have the density of the first size (thread for each mm² that has
  thread) within 15 %; the stitch objects scaled to 150 % have about two
  thirds of it.

## Consequences

- At 150 %, the objects have 0.89 to 1.14 of the density of the first
  size; the stitch objects 0.67 to 0.75. The densest are designs with
  lettering: the Auto underlay of a wider column has more layers.
- The exit tests of the earlier milestones after the changes of the fills:
  parts 97.2 %; columns 98.9 %; fills 100.0 % (98.6 % of the stitches of
  fills of rows in a fill; type and stitch length 97.3 %, underlay setting
  78.4 %, reported).
- Open: the sew-out of the owner (the exit test), and machine files of
  other programs.
