# ADR 0143: Crossings, junctions, and one stitch for the text

Date: 2026-11-20 · Status: Accepted (P20 Milestone 3,
`docs/design/p20-plan.md`). Decision 1 replaces the stitch of each column
of ADR 0142.

## Context

After Milestone 2, the owner looked at pictures of 刺繍東京 at 50 mm and
80 mm and asked two things:

- The strokes have bulges where they cross. The stroke finder makes the
  rails of a column with lines across the stroke from its middle line to
  the outline. Where another stroke crosses, the line on that side goes on
  along the other stroke to its limit, so the column is wider there. Both
  columns went through the crossing: two layers of thread (6.3 % of the
  area of the parts on average in the bench, 24.9 % for ャ).
- The letters look asymmetric. With a stitch for each column (ADR 0142),
  strokes of almost the same width got satin or tatami: at 80 mm the
  strokes of 東 are 9.3 to 13.6 mm, and seven were tatami columns and two
  satin columns.

## Decision

1. **One stitch for the text** (`glyph::auto_stitches`, called by the
   layout for the letters with the stitch type Auto; not for a letter with
   its own shape, not for the underline). The typical width of the satin
   columns of the standard type of the text (the width that half of their
   length is narrower than, `glyph::column_width`) decides: up to 12 mm
   (`AUTO_MAX_SATIN_MM`) all stay satin, with their stitches longer than
   7 mm split, and only a column wider than `AUTO_ALONE_MM` (18 mm) is a
   tatami column; wider, all are tatami columns. `Font::parts` and
   `EmbFont::parts` give the satin columns, and the layout chooses. The
   bench chooses for each character alone, as for a text of one
   character.
2. **Crossings** (`stroke::cut_crossings`). Where the middle lines of two
   pieces cross and each goes on past the other by at least
   `CROSS_PAST` (1.5) half-widths of the other, the longer piece goes
   through and the shorter is cut into two pieces. Each new end stops a
   little inside the edge of the piece that goes through
   (`trim_start_out_of`, as the end of a T). A closed stroke and the
   stroke of a dot are not cut.
3. **The width where strokes meet** (`stroke::rails`). A side of a line
   across a stroke that ends in another stroke (nearer than `INTO_OTHER`,
   0.8 of its half-width, to its middle line) takes the width of that side
   before and after it, along the stroke. The column has the width of its
   stroke at a crossing or a junction, not the size of the junction.
4. **Ends in other columns** (`stroke::trim_ends`, after all rails, and
   again by `stroke::trim_column_ends` after the second pass of the stroke
   finder and the round parts). The end of a column whose pairs of rail
   points lie in other columns (three points across, `SIDE_IN` of the
   width in from the sides, deeper than `END_OVERLAP`, 8 % of the width)
   is cut back to the first pair that is not in them. Up to `END_LEAD`
   (0.6 of the width) at the end can be out of them before the column goes
   into them: at a corner, the end of one column lies on the side of the
   other. Each side is then cut where it comes out of them, so the end
   follows a side that is not square to the column (strokes that meet at
   a slant), with at most one width of slant. A column that lies all in
   others goes. The columns are cut in order, so a cut column does not cut
   the one after it at the same place (a corner stays covered).
5. **Dots.** A short stroke gets more rail points (a step of at most a
   twelfth of its length), and `GlyphShapes::new` gives a round part one
   column to its edge (`tune::round_parts`, as the converter does).
6. **The ends of strokes to their corners** (owner, after the pictures of
   this milestone: the columns did not reach the corners of many stroke
   ends). Near an end that the font cuts at a slant, the middle line bends
   to the sharp corner, and the last line across the stroke was square to
   it. At a free end of a pen stroke (not an end at the cut of a corner,
   `Piece::free`), each rail now follows its side of the outline
   (`stroke::trace_side`) from a pair `END_BACK` (0.8) widths back to the
   corner of the end, where the outline turns more than `END_TURN_DEG`
   (70°) from the side; the pairs between are at the same parts of the two
   paths, so the stitches turn to the slant of the end. Where the two paths
   come to one point, the column ends where it is still `END_MIN_WIDTH`
   (30 %) of the stroke wide (no stitches shorter than 0.3 mm).
7. **The sew-out** (item 4 of the plan). `tools/make-sewout-kit-p20.py`
   makes `out/p20-sewout` (not in the repository): 20 characters at 25 mm,
   4 at 50 mm, the words 刺繍 and 東京 at 40 mm, in the test font, with
   the machine files, the worksheets, the pictures and a checklist. `emb-cli
   font-sheet` now also takes a TrueType or OpenType font.

Tried and not kept:

- `tune::extend_ends` for the letters of TrueType fonts (it covers the
  corners of slanted ends): it makes the cut ends long again, through the
  other strokes (two layers 1.9 %, 12.5 % for サ).
- Free ends that go straight from the body of the column to the outline:
  hooks and short strokes lost more than the slanted ends got (95.8 %
  covered on average, 85 % for ふ). Following the outline (decision 6)
  covers both.

## Consequences

- The bench at 25 mm: two layers 1.4 % of the area of the parts on
  average (6.3 % before), 5.1 % at most (本; 24.9 % before), 17 characters
  over 3 % (170 before). Covered 98.9 % on average (97.2 %), 95.9 % at
  least (愛; 83.6 % before): no character under 95 %. Fewer thin columns:
  at 15 mm, 4 characters have a column under 0.8 mm (9 before).
- The exit test of the plan: at least 95 % covered for each character is
  met; at most 3 % two layers for each character is not met yet. The
  characters over 3 % have junctions of three or four strokes at a slant
  (本, 桜, 海, 繍). The bench test checks the coverage and keeps the levels
  of today for two layers (under 2 % on average, none over 6 %).
- A cut crossing is two columns, so a letter has more parts and more trims
  until the letters of TrueType fonts get routes (Milestone 4): 刺繍 at
  15 mm 29 trims (26), 東京 19 (17).
- The engine is the same for all scripts. Latin, Arabic and Devanagari
  letters of TrueType fonts sew fewer stitches where strokes cross and
  reach the corners of their ends (the golden designs `lettering-names`
  2,498 to 2,443 stitches, 17 trims; `shaped-text` 6,913 to 6,679, 37 to
  41 trims; `mitre-corners` 7,544 to 7,532). Auto-trace and Click to
  stitch use the stroke finder: the designs `autotrace`, `rhinestones` and
  `smart-logo` change a little, and the picture bench covers 98.1 %
  (98.0 %).
- The bundled embroidery fonts are not made again yet. Their letters are
  routed (ADR 0113: the route cuts columns at junctions). The rule of
  which stroke goes through (item 3 of the plan) is chosen after the
  sew-out; then the fonts are made again with the final stroke finder, as
  CLAUDE.md says.
