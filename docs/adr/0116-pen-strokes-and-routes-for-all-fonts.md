# ADR 0116: Pen strokes in the stroke finder, and routes for all fonts

Date: 2026-11-18 · Status: Accepted

## Context

The quality scores of the embroidery fonts (ADR 0115) and the proof
pictures of their letters (ADR 0113) showed two causes of bad letters that
are not in the font, but in the converter:

- **The columns were not the strokes of a pen.** At a junction, the stroke
  finder joined the two lines that were the straightest there, measured
  near the junction. The middle lines bend near a junction, so the stem of
  a "p" could go on into the bowl, and the rest of the stem was a short
  column or was lost. At a corner with a small turn (the "7", the foot of
  an "L" in a slanted font), one column went round the corner, and its
  stitches fanned out there.
- **Trims.** Only the ten tuned fonts were routed (ADR 0113). The other
  117 fonts had about 1.4 trims in each letter.

The owner asked for letters that sew as a pen writes them, and for a trial
in three steps: better strokes at junctions and corners, the writing order
in the route, and a table of pen strokes for each letter (templates).

## Decision

- **The direction of a stroke at a junction** (`Graph::direction`) is the
  chord of its middle line from 0.5 to 2 half-widths from the junction
  (`TANGENT_FROM`, `TANGENT_TO`), not the first cells of the line. Two
  lines join into one stroke if the angle between them is 140 degrees or
  more (`STRAIGHT_DEG`, as before) after a penalty for different widths
  (`WIDTH_DEG`: 30 degrees for a line that is much narrower than the
  other). So a stroke goes on into the line of the same width.
- **A stroke that meets another stroke** is cut back only where it is in
  the other stroke with its whole width (`trim_start_out_of` with the
  half-width of the stroke). The end of a stroke that goes on to the
  outline is not cut back by more than the part that was added.
- **A corner of the outline cuts a stroke** (`outline_corners`,
  `cut_corners`). A turn of the middle line of 50 degrees or more
  (`CORNER_AT_OUTLINE_DEG`; 70 degrees with no corner, as before) is a cut
  if the outline has a sharp corner on the outer side of the turn, not
  more than 2.1 half-widths away, with its two sides along the stroke
  before and after the turn, and the turn is the part of the stroke that
  is nearest to that corner. The corners of the end of a stroke and the
  round turns of an "S" or a "C" are not cuts.
- **The writing order in the route** (`route::PEN`). Between routes with
  the same trims, a column that sews up, or to the left if it is more
  level than upright, costs as much as 0.5 mm of travel for each mm. So
  stems sew down and bars sew to the right where that costs little
  travel. The trims come first, as before.
- **All fonts are routed.** `tools/make-embroidery-fonts.py` makes each
  font of the program and of the packages with `--extend --route` (ADR
  0113). The mark `--tuned` stays for the ten fonts whose letters were
  looked at on proof pictures (the lists show them first). The tool makes
  some fonts at the same time (`--jobs`).
- **A font keeps its place.** A change of the converter changes the
  scores. A font of the program stays in the program and a font of a
  package stays in its package, as long as its score is not below the
  limit (the tool stops for such a font). The rule of ADR 0115 (in the
  program from a score of 85) is for a new font.
- **No stroke templates.** The third step of the trial was a table of the
  pen strokes of 39 letters and digits, matched to the middle lines of a
  glyph. It fitted 419 of 624 letters of 16 test fonts and gave no gain
  (mean score 83.2 and 83.1, the same coverage, 94 and 101 trims): after
  the changes above the rules find the pen strokes of plain fonts, and
  the bad letters of heavy serif fonts are not a question of which
  strokes there are. The templates are not in the program.
- **No travel stitch to the place of the needle** (`plan::SAME_PLACE_MM`).
  A run that starts less than 0.1 mm from the needle gets no travel
  stitch: in a machine file (unit 0.1 mm) such a stitch can have no
  length. One letter of the new fonts had such a stitch, and a machine
  file of it did not open as the same stitches.

## Consequences

- **Scores.** Over the 127 fonts the mean score goes from 78.8 to 82.3,
  and 52 fonts have 85 or more (40 before). The part of the outlines that
  the letters cover goes from 97.0 % to 97.8 %. The lowest score of a font
  of the program is 71 (66 before). Three fonts go down by 1 to 3 points
  (Friendly Serif 85 to 82, Sturdy Serif 81 to 78, Damion 84 to 83): the
  serifs are columns of their own now, and they are narrow.
- **Trims.** The 7,874 letters and digits of the 127 fonts have 683 trims
  at 15 mm (9,889 before). A text of the 62 letters and digits of each
  font has 5,726 trims (11,613 before): most of them are the trim between
  two letters. The ten tuned fonts have 60 trims in their letters (71
  before). The fonts have more parts (32,070; 20,524 before): travel runs
  and cut columns.
- **The Mitre Corners setting** of lettering (ADR 0086) does not change a
  routed letter (ADR 0113), so it does nothing for the embroidery fonts of
  the program and of the packages. It stays for TrueType fonts and for
  fonts that the user converts. The setting is not available when all the
  letters of the lettering are fixed.
- **Twelve fonts of the packages** have a score of 85 or more now (Mali,
  Barlow, Rubik Mono One, Neucha, Machine Sans, Barlow Semi Condensed,
  Khand, Humanist Sans, Game Sans, Wide Heavy, Shadows Into Light, Cabin
  Condensed), and Fat Face Serif (a font of earlier versions) is at the
  limit of 65. They keep their places; the owner decides if they move.
- **A join under the strokes** can be one long stitch (9.8 mm in the
  corpus design `embroidery-fonts-6`). The parts that sew later cover it.
- **Picture to design** uses the stroke finder for narrow areas
  (`emb_art::click`). Two pictures of the bench change by a few stitches
  (`large-picture`: 2,853 stitches and 5 trims, 2,849 and 4 before).
- **The golden snapshots** of the designs with lettering change: the
  designs of the fonts have about half of the trims.
- **Tests that need a font with no route** (the corner test, the test of
  the closest-join order) convert a bundled TrueType font.
- The letters of the ten tuned fonts were looked at again on proof
  pictures. The other 117 fonts were measured, not looked at letter by
  letter. No sew-out confirms the new letters.
