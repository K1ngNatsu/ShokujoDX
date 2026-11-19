# ADR 0120: Fill letters from a size

Date: 2026-11-19 · Status: Accepted (on pictures only; no sew-out)

## Context

An embroidery font has one shape for each letter. A font with satin
letters has a size range that ends where its columns get too wide (10 %
of their length with stitches of more than 12 mm, ADR 0115): at 25 to
60 mm for most fonts. Above it the letters have long, split stitches,
and the design checks give a warning. Large lettering needed one of the
ten fill fonts (ADR 0117), which are other fonts with other names.

ADR 0119 let the converter choose columns or a fill for each letter. The
measures showed that one choice for each letter is wrong at one end of
the size range: at 6 mm a satin letter is right, and at 60 mm a fill is
right. That choice changed 32 letters of five fonts. A TrueType font does
it by size: with the stitch type Automatic, the layout makes a letter
from columns or as a fill at the size of the lettering.

The owner asked for lane 5b of the review of P17 (2026-11-19).

## Decision

- **A letter can have two shapes.** `EmbLetter::fill_parts` is the letter
  as fills: the outline of the source font
  (`quality::fill_letter`), next to the columns in `EmbLetter::parts`.
  A font with such letters has `EmbFont::fill_letters` (`FillSizes`):
  `from_mm`, the capital height above which the letters sew as fills,
  and `min_mm`, the smallest size at which the fill letters sew well.
- **The change is by the size of the lettering, for the whole text.**
  `EmbFont::parts` gives the columns up to `from_mm` and the fills above
  it (`EmbFont::uses_fill`). So a word is all satin or all fill; it has
  no mix of the two looks. A fill letter is not fixed and has no
  settings of its parts (`LetterSource::fixed` and `tuning` get the
  stitch type and the scale).
- **`from_mm` is the largest size of the columns** (the end of the size
  range before), and not less than 15 mm (`FILL_MIN_HEIGHT_MM`). So
  lettering in the size range that a font had sews as before: no design
  changes its look. Above it, lettering that had a warning and long
  stitches has fills now.
- **The size range goes to 300 mm** (`EmbFont::max_height_mm`), as the
  range of the fill fonts. `EmbFont::column_max_mm` is the end of the
  columns: the quality score, the size range of the columns
  (`fit_size_range`), the audit of a font and the font test sheet use
  it. The score does not change.
- **The stitch type of the lettering selects a shape.** For a font with
  fill letters, **Stitch type** in the Lettering window is there:
  Automatic (by size), Satin (columns at each size; the range ends at
  `from_mm`) or Fill (fills at each size; the range starts at `min_mm`).
  Before, the setting did nothing for an embroidery font.
- **The converter makes the fill letters last**
  (`EmbFont::add_fill_letters`), after the size range of the columns:
  `emb-cli font` and Convert in Lettering > Fonts, with the stitch type
  Auto. `min_mm` is the start of the range of the fill font with the same
  letters (ADR 0117). Not for a fill font (it has fills only), not for a
  font for small text, and not with the stitch type Satin.
- **A font file is checked**: the fill parts are fills with coordinates
  in the range of a font, and the two sizes are in the range of the font
  (`EmbFont::validate`, `check_letter`).
- **The Fonts window** shows the size of the change under the size
  range.

## Consequences

- All fonts were made again. 123 of the 137 fonts have fill letters: all
  fonts with satin letters but the four fonts for small text. Their
  columns and their scores are the same as before (a check of each
  letter), so the designs of the corpus have the same stitches. The
  change to fills is at 25 mm (the heavy display fonts) to 206 mm
  (Monoline Script), at 52 mm for the middle font.
- **The heavy fonts gain the most.** Poster Bold has untidy columns at
  29 mm, the end of its columns, and clean fill letters at 33 mm (the
  pictures of the sew-out kit). With the stitch type Fill, a user gets
  these fill letters from 15 mm.
- For 122 fonts the fill letters are good from the size of the change
  (`min_mm` is not above `from_mm`; it is 15 mm for most fonts). Fat
  Face Serif is the one font with a gap: its columns end at 44 mm, and
  its hairlines are narrower than 2 mm as fills up to 58 mm.
- **The font files are larger.** A fill letter is the outline of the
  letter (about 37 points for each ring). The fonts of the program are
  65 MB as text (42 MB before) and about 4 MB more in the program
  (they are compressed); the font packages are 16 MB (10 MB before).
  The outlines are as fine as the columns (0.02 mm at 25 mm); a coarser
  outline would show at 300 mm.
- **A design with the stitch type Fill and an embroidery font changes.**
  The setting did nothing for such a font; now the lettering sews as
  fills when its letters are made again. The window did not show the
  setting for an embroidery font, so a design has it only if its font was
  a TrueType font before.
- The ten fill fonts stay: their names are in designs, and they are the
  fonts of the filter For large text. A font with satin letters and the
  stitch type Fill gives the same letters as its fill font.
- The font test sheet and the sew-out kit use the sizes of the columns.
  The kit has three new designs (`large/`): the same letters at the end
  of the columns and 15 % above it.
- A letter that a person corrected (ADR 0112) keeps its fill letter: the
  correction is for the columns.
- A fill letter has no border, and its edge is rougher than the edge of
  a satin letter (ADR 0117). At the size of the change the look of a
  text changes from satin to fill. If a sew-out shows that the change
  must be at another size, `from_mm` is one number for each font.
- A font file of an earlier version has no fill letters and reads as
  before. An earlier version of the program reads a new font file as a
  font with columns up to 300 mm (it does not know the new values); no
  released version of the program reads the font packages.
- No sew-out confirms the size of the change.
