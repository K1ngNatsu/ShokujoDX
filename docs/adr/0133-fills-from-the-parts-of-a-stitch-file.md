# ADR 0133: Fills from the parts of a stitch file

Date: 2026-11-20 · Status: Accepted (P18 Milestone 3,
`docs/design/p18-plan.md`)

## Context

Milestones 1 and 2 (ADR 0131, ADR 0132) cut the stitches of a machine file
into parts and make running and satin objects of the runs and the
columns. Milestone 3 makes fill objects of the rows: the area with its
holes, the angle, the row spacing, the stitch length, the offset of the
needle points from row to row, and the underlay. A fill with rows in more
than one direction becomes more than one fill. The native formats of
closed-source programs stay out of scope (rule 3).

## Decision

- **The engine** is `emb_art::part_fills` (not a port), called by
  `emb_art::part_objects::objects`.
- **Rows in layers** (`row_layers`): in a part, a leg is a row if it is
  0.5 mm long or more and it is not a turn (a short leg between two legs
  that go back and forth). A new layer starts where the direction of the
  rows changes by more than 12° and the next row keeps the new direction;
  else the leg is not a row. A layer that a denser layer comes after
  (less than 0.8 of its spacing) is the underlay of that layer; the other
  layers are the rows of fills.
- **Settings** (`rows_shape`): the angle is the mean direction of the rows
  (long rows count more), kept as a whole degree when it is within 0.3°.
  The row spacing is the median distance between two rows one after the
  other. The stitch length is the median length of the stitches inside
  the rows. The offset: the stitch engine puts the needle points of row
  `k` at `(n + (k mod staggers) / staggers) · length`; the number of
  staggers (1 to 6) that fits the needle points best gives a square fill
  (1) or a tatami fill (the others). The pull compensation is 0: the row
  ends have it already.
- **Area** (`rows_area`): in the frame of the rows, the area goes from the
  ends of each row to the ends of the row next to it, and half a row
  spacing out at a row with no row next to it. It is drawn in cells of
  0.1 mm (at most 3 million cells) and traced as Recognize Outlines does
  (ADR 0030), so the outline goes through the ends of the rows. A gap
  across the rows of at most 1.2 row spacings is closed (a row that went
  with another part). Each traced area is one fill object.
- **Groups**: layers of rows of one thread with angles within 3° and row
  spacings within 10 %, one after the other, are one fill (the sections
  of a fill). A run of at most 40 stitches between them is travel; its
  rows at the fill angle are a part of the area. A column part with rows
  at the angle and spacing of the open fill is rows of the fill (a narrow
  section with rows of one stitch).
- **Underlay**: a piece of underlay along the outline (its legs stay at
  one distance from it) is an edge walk. In the other pieces, the legs
  across the inside are rows (in a narrow shape a row is one stitch or a
  few); the rows of one direction over the pieces one after the other are
  a tatami layer (its angle to the top rows, its spacing, and its inset:
  the median distance of its row ends from the outline) if they are
  lighter than the top rows, at most 6 mm apart, and cover 40 % of the
  area inside the inset. When the layers are the layers of Auto, Light,
  Heavy or Double for the shape (`emb_stitch::object::*_fill_underlay`),
  the fill gets that setting; else the layers (Custom), or none.
- **Changes of the parts** (`emb_art::parts`, Milestone 1): rows sew each
  cell once, so a column or rows under rows is lighter by its row spacing
  (1.25 times as open), not by the thread in a cell. A part of at most 16
  stitches that rows cover is underlay too (too short to show its
  spacing), and so is a part of at most 16 stitches between two parts of
  the underlay of a part. The exit test of Milestone 1 goes from 97.1 % to
  97.2 % (rows 97.9 %).
- **The view**: View > Stitch Parts also draws the outline and the holes
  of each fill.
- **The exit test** is the third test of `crates/emb-cli/tests/parts.rs`
  (the corpus as DST and back, the lettering cut into its letter parts).
  For each recognized fill, its source object is the object of most of
  its stitches. Fills of rows (tatami, square, program split, user split)
  with no accordion, jagged edge or hand stitch are measured: the angle
  (in the coordinates of the file, as a line) must be within 5° and the
  row spacing within 10 % of the source; 90 % of their stitches must pass.

## Consequences

- 99.9 % of the 214,527 stitches of the measured fills pass; the misses
  are three fills of fewer than 30 stitches. 98.7 % of the 230,060
  stitches of the fills of rows of the corpus are in a recognized fill.
- Reported, not in the exit test: 96.7 % of the 214 tatami and square
  fills get the type and the stitch length (10 %) of their source; 82.3 %
  of the 226 measured fills get the underlay setting of their source.
  Most of the others are small letters of the fill fonts, where travel
  and short sections give a layer more or a layer less.
- A section of a fill that sews before the first rows of its fill, with
  rows of one stitch, can become a satin; a short run that is not in the
  fill stays a running object over it.
- The objects of 44,000 stitches take 0.13 s after the parts (0.09 s).
- Open: Recognize Objects (Milestone 4), and a test with machine files of
  other programs.
