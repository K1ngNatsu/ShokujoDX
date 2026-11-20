# ADR 0142: The stitch of each column in Auto lettering

Date: 2026-11-20 · Status: Accepted (P20 Milestone 2,
`docs/design/p20-plan.md`). Decision 2 (a stitch for each column) is
replaced by one stitch for the text (ADR 0143).

## Context

With the stitch type Auto, a letter of a TrueType or OpenType font was
satin only if its widest column was not wider than 8 mm
(`glyph::AUTO_MAX_SATIN_MM`); else all of the letter was one tatami fill.
The width was the widest point of the stroke finder (`Strokes::max_width`).
At a junction, the lines across a stroke go into the other stroke, so that
point is the size of the place where the strokes meet, up to two times the
width of the stroke (木 at 50 mm: 17.5 mm, and its strokes are 8 to 10 mm).
In the lettering bench (ADR 0141), 247 of the 248 characters of the
reference set were all tatami at 50 mm and at 80 mm, and 森, 木 and ・ at
25 mm. The owner saw a Japanese character in tatami that should sew well
as satin.

The satin columns of the program split a stitch longer than 7 mm
(`SATIN_SPLIT_MM`, the default `SatinParams::split_mm`), and the column
type Tatami (`ColumnPattern::Tatami`) sews rows across a column that
follow it.

## Decision

1. **The width of a column between its two sides**
   (`glyph::column_width`): the largest width that the column keeps over a
   length of `glyph::KEEP_LENGTH` (1.5) times that width, measured along
   the middle of the column (`quality::column_pairs`). Where a column
   crosses or meets another stroke, it is wider over about the width of
   that stroke: that width does not count. A column shorter than its width
   (a dot) has its middle width.
2. **The stitch of each column in Auto** (`glyph::auto_column`), not of
   the whole letter: a satin column up to `AUTO_MAX_SATIN_MM`, now 12 mm
   (`quality::LONG_MM`, the longest satin stitch of the quality score),
   with its stitches longer than 7 mm split; a wider column is a tatami
   column (stitches of `AUTO_TATAMI_STITCH_MM`, 3.5 mm, the spacing of the
   type: rows 0.4 mm apart, no short stitches; `SatinParams::with_type`
   keeps the pull compensation and the underlay). A letter that the
   columns cover less than `AUTO_MIN_COVERAGE` (85 %) stays a fill letter,
   as before (`GlyphShapes::columns_cover`, `auto_parts`).
3. **Embroidery fonts** get the same choice in Auto
   (`EmbFont::parts`): a satin column of the standard type that is wider
   than 12 mm at the size of the lettering is a tatami column. The fill
   letters above the largest size of the columns (ADR 0120) stay. The
   converter and the bitmap fonts choose satin or fill for each letter by
   the coverage only, as before; the stitch of each column is chosen at
   the layout, from the size.
4. **The lettering keeps a tatami column.** `Lettering::apply_params`
   gives the satin settings of the object to each column, but a tatami
   column of Auto stays a tatami column (with the spacing of its type)
   if the columns of the object are satin. If the object has another
   column type, all columns get it.
5. **The bench** shows the width of the widest column between its sides,
   the columns wider than 8 mm (`lettering_bench::WIDE_MM`, the limit of
   the plan) and of them the satin columns with no split stitches. Its
   test checks the exit test of the milestone. The widest place of a
   column (for split stitches) is now the longest stitch across it: the
   measure of Milestone 1 used only pieces of 0.5 mm or more, but it
   measured in pieces of 0.25 mm, so it gave the middle width.

Not done: the plan says that a part of a letter that no column covers is
a fill only if it is large. The stroke finder covers each character of the
reference set more than the limit (the lowest is 88 %, 、), so no letter
needs it now; a letter under the limit is a fill letter, as before.

## Consequences

- The test font (Noto Sans JP Bold): no character of the reference set is
  all tatami from 15 mm to 80 mm. At 50 mm: 1,073 satin columns with split
  stitches and 176 with none; at 80 mm: 836 tatami columns, 394 satin
  columns with split stitches, 19 with none. Each column wider than 8 mm
  has split stitches or is a tatami column.
- The letters at 50 mm and 80 mm are columns now, so their numbers are the
  numbers of columns: two layers on 6.3 % of the area on average (crossings,
  Milestone 3), the trims of the columns (刺繍: 25 at 50 mm, 6 before as a
  fill; Milestone 4 gives routes), and thin columns of the stroke finder
  (0.18 mm in り at 50 mm; Milestone 5).
- At 80 mm a letter can have satin and tatami columns. The sew-out of
  Milestone 3 shows if a letter must have one stitch for all its columns.
- Latin letters of TrueType fonts change in the same way at large sizes:
  a letter with a stroke wider than 8 mm at a junction was a fill, and is
  now satin columns, or tatami columns for strokes wider than 12 mm. The
  golden snapshots did not change (no corpus design has such lettering).
- `LetterStitch::Auto`, the Lettering help and What is new say it.
