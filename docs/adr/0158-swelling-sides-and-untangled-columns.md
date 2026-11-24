# ADR 0158: Sides that swell, and columns with no crossing stitches

Date: 2026-11-24 · Status: Accepted (P21 Milestone 12,
`docs/design/p21-plan.md`).

## Context

After Milestone 11 the owner asked to work through the largest measured
faults of Latin letters, from the most improvement to the least: open
areas (Anton "a" and "u"), bulges (Pacifico), crossing stitches at
junctions ("e"), sharp corners that the columns do not reach, columns
that end on top of other columns, and needle points out of the outline.

The letter fault bench has four fonts, so the order came from a survey:
the faults of the letters A-Z, a-z and 0-9 of the 127 column fonts of
the program and of the packages, as lettering in their source fonts at
25 mm (7,874 letters). Open area was in 1,770 letters of all 127 fonts;
crossing pairs in 928 letters; open corners in 1,409 letters but only
0.5 mm2 a letter; bulges 59 % in ten script fonts; needle points out of
the outline 46 % in a few heavy fonts; columns on top in 951 letters.

## Decision

1. **A side that swells.** A rail looks for the outline only within
   `FOOT_REACH` half-widths of the middle line (so it does not go into
   another stroke at a junction). Where a stroke swells (the outer side
   of the bowl of a heavy "3", the stems of a heavy "u", the leg of
   "k"), the outline is farther, and the rail went straight across, with
   the swell open. Where no foot is in that reach, a foot as far as
   `FOOT_FAR` (2.5) half-widths is taken if the line to it stays in the
   letter (not across the eye of an "e"); the check of the outline
   between two feet keeps the openings into other strokes. Measured:
   2.5 left 38 % less open area; 3 and 4 more crossing stitches. If the
   outline of a column (its two sides) crosses itself more with the far
   feet than with the near feet only, the column has the near feet
   (`stroke::sides_cross`): in the pilcrow of Great Vibes a far foot was
   paired with the wrong part of a loop, and the column crossed itself
   40 times.
2. **Valid areas for the second pass.** The second pass for lost strokes
   (`glyph::add_missed_strokes`) made the area of each column from its
   two rails; a column that crosses itself gave a ring that crosses
   itself, and a boolean operation of the `geo` library did not end on
   it (the conversion of Great Vibes did not end). The areas are now made
   valid first (`region::fill_nonzero`, the nonzero rule of the overlay
   library).
3. **No crossing stitches.** Where a column turns fast, or its sides
   have different lengths (a slanted end, a curl, a crossbar), two pairs
   across it next to each other crossed. The columns of lettering get
   rungs on which one side stays where a pair would cross the pair
   before it (a pivot: `Column::untangled_rungs`, applied by
   `glyph::untangle`). The satin engine does not change for other
   objects (the owner keeps changes of the stitch engine narrow). A
   pivot moves the side that stays a millionth of the rails, and the
   grid of a font file (0.01 mm) takes it away; a larger move makes the
   next pair cross, and rung points off the grid of the rails were
   matched to the wrong place of a rail that loops (Pacifico). So the
   layout untangles the columns of each letter last, after the scale,
   bold and italic (`glyph::untangle_parts` in `layout::letter_parts`),
   for TrueType and embroidery fonts; the font files keep their grid.
   The letter fault bench measures the letters of the embroidery fonts
   as the layout sews them.
4. Tried and left out: the second pass for lost strokes
   (`glyph::add_missed_strokes`) also for letters that the columns cover
   well, for the foot of a stem past a junction (Anton "a"). Open area 8 %
   less, but 9 % more area with two layers, 5.5 % more crossing pairs and
   22 more trims.
5. Not changed:
   - Open corners: the tips of acute corners (0.5 mm2 a letter); satin
     cannot go into a tip that sharp, and the thread covers it.
   - Bulges: the measure counts a column wider than 1.5 times its middle
     width; in calligraphic scripts that is the swell of the stroke, and
     the columns follow the outline.
   - Needle points out of the outline: most are in counters that are
     narrower than a thread in very heavy fonts (Shrikhand "p", Poster
     Heavy "n", Giant Round "m"), where pull compensation pushes the
     points into the counter; a pull compensation for each part near
     such a counter is a feature of its own. (Corrected in ADR 0160: the
     cause was the split of the outline, a cut along the stem and the
     moved end of a piece, not pull compensation.)
   - Columns on top: the route takes fewer trims first (ADR 0153).

## Consequences

- The survey (7,874 letters of 127 fonts as lettering at 25 mm): open
  area 5,837 to 3,581 mm2 (letters with more than 0.5 mm2 open: 1,770 to
  1,502); crossing pairs 4,507 to 806 (letters with any: 928 to 440);
  open corners 748 to 684 mm2; area with two layers 3 % less; trims 787
  to 783; needle points out of the outline 1,102 to 1,107. The pivots
  make short fans (175 to 391 mm), and 35 letters have more stitches
  that pass over a notch (128 to 223).
- The letter fault bench (embroidery font / lettering): crossing pairs
  Block Sans 11 / 12 to 3 / 4, Anton 19 / 14 to 8 / 13, Pacifico 12 / 10
  to 3 / 0, Varsity 7 / 6 to 1 / 1; open area of Anton 1.09 % / 1.07 %
  to 0.31 % / 0.28 % (its bowls and stems); the other measures as before.
- The fonts: 34,070 to 33,949 trims in the letters of all fonts; the
  column fonts of the program 94.3 to 94.2 on mean (six fonts 1 or 2
  points lower).
- The golden designs with lettering: stitches -2.3 % to +2.0 %, the same
  trims. The lettering bench of P20: one trim fewer in 刺繍 at 10, 15 and
  25 mm, the other words the same; stitches out of the
  outline at the most 1.0 % (森; 0.2 % before).
- The audit of P21 (2026-11-24): `Column::untangled_rungs` makes at most
  20,000 pairs between two rungs (`UNTANGLE_MAX_PAIRS`). With a step of 0
  it made a pair at each billionth of a mm and asked for 34 GB of memory,
  and the program stopped. The step is a part of the height of the
  lettering, so lettering with a height below 0 (a project file with a
  wrong number) did that. The test for two pairs that cross is one
  function (`emb_stitch::satin::pairs_cross`) for the score, the bench,
  the choice of a split and the untangled rungs.
- The audit of P21, the area of columns (`region::fill_nonzero`, decision
  2): where the two rails of a column lie on each other, the overlay
  gives rings with no area and rings with thousands of sides of almost
  no length, and `region::offset` did not come back from such a region
  (the picture at "K" of the font Webdings: the layout waits for this
  step, so the program did not answer). The area has no ring with no
  area now, and a ring with a side shorter than a millionth of the size
  of the rings is thinned by that distance. This changed two letters of
  the font packages: the pilcrow of Neucha (a fill letter now: its
  columns crossed themselves) and one column of the pilcrow of Great
  Vibes; the fonts of the program are the same files.
