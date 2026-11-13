# ADR 0082: Hidden travel

Date: 2026-10-29 · Status: Accepted

## Context

A design from a photo (Picture to design, color areas) has many areas of
one color that are apart, with areas of other colors between them. Each
gap longer than the trim distance of the machine gets a trim: the owner's
four photos gave 61 to 271 objects and nearly as many trims. A trim takes
time on the machine and leaves a thread end. Where the colors that sew
later cover the gap, a run of travel stitches under them is not seen
(the P12 audit: "Travel between them under later colors is not done").

## Decision

- A new trim mode `TrimMode::Hidden` ("Hidden travel" in the Machine
  Functions panel): as `Auto`, but a long gap to the next object of the
  same color gets travel stitches when objects that sew later cover all of
  the way; else a trim.
- The stitch plan makes a grid of cells (0.25 mm, at most 2000 cells on a
  side) only if an object has this mode. Each cell keeps the last place in
  the sew order of an object that covers it: a dense fill (tatami and the
  other patterns that cover all of the shape, rows at most 0.8 mm apart)
  or a satin column. Running stitches, open fills, appliqué and lettering
  do not cover (`emb_stitch::hidden`).
- The way is a straight line if each point on it is covered by an object
  that sews later; else the shortest way through such cells (A* with eight
  neighbors), with its corners cut where a straight line is hidden. Its
  ends can be 0.6 mm from a covered cell (they are on the edge of an
  object). A way longer than 60 mm gets a trim. The travel stitches are at
  most 2.5 mm long.
- The objects of Picture to design (color areas) get the mode
  (`emb_art::smart::functions`), in the wizard, the bench and the corpus.
  Other objects keep `Auto`; the user can set the mode on any object.

## Consequences

- Fewer trims for designs from photos: the owner's four photos have 52,
  226, 102 and 112 trims with `Auto` and 42, 154, 72 and 65 with hidden
  travel (19 to 42 % fewer), and a few stitches fewer (no lock stitches
  at the trims). The pictures of the bench snapshot and the `smart-logo`
  golden snapshot do not change: their gaps lie over colors that sew
  earlier or over the fabric.
- A project file with the new mode does not open in an older version of
  the program.
- The travel lies under the later objects: if the user deletes or moves
  one of them, the plan finds a new way or trims, because the plan makes
  the grid again each time.
