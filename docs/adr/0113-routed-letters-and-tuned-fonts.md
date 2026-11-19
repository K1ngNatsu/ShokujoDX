# ADR 0113: Routed letters and tuned fonts

Date: 2026-11-18 · Status: Accepted

## Context

The bundled embroidery fonts are conversions of outline fonts. Phase P17
Milestone 5 (`docs/design/p17-plan.md`) wants ten fonts that are as good
as fonts that a person digitized. An audit of the converted fonts
(`emb-cli font-audit`, proof pictures of each letter) showed where they
are not:

- **Trims in the letters.** The stitch engine sews the parts of a letter
  nearest first, and it trims each gap that is longer than 1 mm (ADR
  0018). A converted font had about one trim in each letter (Block Sans:
  93 trims in 81 letters at 21 mm). A person hides the travel under the
  strokes that sew later, and a letter has no trim.
- **Corners with no stitches.** The stroke finder ends a column where its
  stroke meets another stroke. At a sharp corner (the top of "N", the
  corners of "Z") the end turns to the other stroke, a part of the
  outline has no stitches, and the two columns do not touch.
- **Round dots** had a square column that covered about 85 % of them.
- **Script letters** did not end where the next letter starts, so most
  joins of a script had a trim.

## Decision

- **The audit** (`emb_cli::fontaudit`, `emb-cli font-audit`): for each
  letter, the part of the outline of the source font that its parts
  cover, the part outside the outline, the part that two parts cover, the
  narrowest and widest column, and the trims in the letter at the
  smallest, middle and largest size (from the stitch engine). It also
  gives the trims of a sample sentence. `--proof` writes the geometry of
  the letters; `tools/font-proof.py` draws the proof pictures from it.
- **Column ends** (`emb_letter::tune`, with the outline of the source
  font). An end goes on into a part of the outline with no stitches, as
  far as the outline, one rail more than the other at a slanted edge. An
  end that turns to the column that it meets is cut back to the straight
  body of its column and goes on straight; the ends of the other columns
  then take the part that it had. A small round part of the outline with
  one column gets a column whose rails are the two sides of the part. A
  change stays only if the outline then has fewer cells with no stitches
  and the parts do not go much more outside it.
- **The route of a letter** (`emb_letter::route`). A search over all
  orders and both directions of each column takes the route with the
  fewest trims, then the shortest travel. Between two parts, a **travel
  run** goes from the end of the first one to the start of the next one
  through the columns that sew later, down their middle, so their
  stitches cover it (a run with stitches of 1.5 mm). A run can leave a
  column at each point of its end edge and come to a column at each point
  of its start edge: a stitch along such an edge lies on the end stitches
  of the column.
- **Cuts.** If a letter still has a trim, a column is cut at a junction
  or a crossing where that gives fewer trims: the stem of "H" at its bar
  (each half sews to the bar), one stroke of "X" where the other one
  crosses it. The two halves overlap by 1 % of the capital height. At
  most two cuts in a letter.
- **Flow.** A route that starts on the left and ends on the right is
  better. A letter whose last stroke ends after its place in the line (a
  script letter) ends at the end of that stroke, and starts with a run
  from the place where the last stroke of the letter before it ends, so
  the letters of a word join with no trim.
- **A routed letter is fixed** (ADR 0112): the stitch engine sews it as
  the font has it. For a fixed letter, the engine joins two parts with no
  trim if their edges (the end of the first one, the start of the next
  one) are near or the way between them is under the parts that sew
  later.
- **Tuned fonts** (`EmbFont::tuned`, `bundled::TUNED`,
  `TUNED` in `tools/make-embroidery-fonts.py`): Block Sans, Friendly
  Serif, Pacifico, Atkinson Hyperlegible, Varsity, Bebas Neue, Soft
  Round, Geo Slab, Brush Hand and Vintage Script. The converter runs with
  `--extend --route --tuned`, and each letter of the basic set was looked
  at on proof pictures. A letter with more than eight parts gets a simple
  route (each time the nearest part that needs no trim). The font lists show the tuned fonts first, and
  the Fonts window marks them. The other bundled fonts do not change in
  this milestone.

## Consequences

- The numbers of the ten fonts are in `docs/design/p17-plan.md`. Over
  the ten fonts, 1263 trims in the basic letters become 159. For
  Block Sans: 93 trims in 81 letters become 11 (the dots of "i" and "j",
  the two parts of ":" and the like stay), and the sample sentence of 45
  letters goes from 92 trims to 45, one for each gap between two letters.
  For the script Pacifico the sentence goes from 115 trims to 30.
- A tuned font has travel runs as parts of its letters
  (`ObjectKind::Running`), and columns that were cut. Code that reads the
  parts of a letter must accept runs.
- The seam of a cut column can show as a fine line. No sew-out confirms
  the cuts yet: the owner sews the test sheets (the exit test).
- The golden snapshots of the designs with tuned fonts change.
- The route is made when the font is made, for the capital height of the
  font. At a very large size a gap that the font joins with one stitch
  can be longer than the trim limit; the audit shows the trims at the
  largest size.
- Routing a font takes about 5 to 30 seconds (the letters in parallel
  threads), so only the font tool does it, not the program.
- The proof pictures are a development tool (`tools/font-proof.py`); the
  program does not make them.
