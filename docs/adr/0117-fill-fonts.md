# ADR 0117: Fill fonts

Date: 2026-11-19 · Status: Accepted

## Context

An embroidery font of the program has satin letters: the converter finds
the strokes of each letter and makes a column for each one (ADR 0016,
0100, 0116). Two kinds of lettering do not fit this:

- **Heavy fonts.** In a heavy slab or a fat display font the serifs and
  the strokes are one shape with no clear columns. Slab Heavy has a
  quality score of 59 after all the work on the stroke finder, and a
  table of pen strokes did not help (ADR 0116).
- **Large text.** A satin stitch is good up to about 7 mm (the stitch
  engine splits longer ones). The letters of a jacket back have strokes
  of 20 mm and more. A person digitizes them as fills.

The converter can make all letters of a font as fills (`--stitch fill`,
and the stitch type Fill of Lettering > Fonts > Convert). Such a font had
a score of 50: the score counted each fill letter as half a letter
(ADR 0115), and its size range started at 5 mm.

## Decision

- **A fill font** is an embroidery font with the mark `EmbFont::fill`. The
  converter sets it for the stitch type Fill. Each letter is one fill for
  each of its shapes, with its holes. The letters are not fixed and have
  no route. The Fill settings of the lettering object set the stitches,
  as for each fill letter.
- **The widths of a fill letter** (`quality::fill_widths`). The local
  width of a place in a letter is the diameter of the largest circle in
  the letter that has the place in it (from the distance transform of a
  grid with cells of 0.25 mm at the design size). The width of a hole is
  the diameter of the largest circle in it.
- **The size range** (`EmbFont::fit_size_range`) starts where at most 5 %
  of the area of the letters and digits (`FILL_TAIL`) is narrower than
  2 mm (`MIN_FILL_MM`, the narrowest fill that sews well), and at most
  10 % of their holes are narrower than 1 mm (`MIN_HOLE_MM`), and not
  below 15 mm (`FILL_MIN_HEIGHT_MM`). It ends at 300 mm, as the range of
  fill letters of a TrueType font does. The tail is 5 % of the area, not
  10 % of the length as for columns: the hairlines of a serif font are a
  small part of its area.
- **The quality score** of a fill font has the same three parts
  (ADR 0115). Shape: the part of the outline that the fills cover (all of
  it). Narrow: the part of the area of a letter that is narrower than two
  thirds of 2 mm at the smallest size, and 0.15 for each hole that is
  narrower than two thirds of 1 mm there (`HOLE_CLOSED`). Long: none (the
  rows of a fill have stitches of a set length). The letters of a fill
  font are not "fill letters" of the score (`LetterQuality::fill` is for a
  letter with no column in a font of satin letters).
- **Ten fill fonts are in the program**, made by
  `tools/make-embroidery-fonts.py` (its list `FILL`) from the same source
  fonts as the fonts with their names: Block Sans Fill, Varsity Fill,
  Anton Fill, Archivo Black Fill, Slab Heavy Fill, Poster Heavy Fill,
  Giant Round Fill, Funky Slab Fill, Rubik Mono One Fill and Shrikhand
  Fill. They are the last fonts of `bundled::EMBROIDERY` (from
  `bundled::P17_EMBROIDERY`). A fill font has a name of its own, so a
  design with the satin font of the same source keeps its letters.
- **The Fonts window** marks a fill font ("fill"), says what it is for,
  and has the filter **For large text**. In the order of the quality
  scores, the fill fonts come after the fonts with satin letters: their
  score has another measure, and each of them has a high score.

## Consequences

- The ten fill fonts have scores from 99 to 100 and ranges from 15 mm
  (Varsity Fill from 20.5 mm, Block Sans Fill from 15.5 mm) to 300 mm.
  The score of a fill font says little: a fill covers each shape. The
  size range is what the measure gives. As fill fonts, the fonts with
  thin strokes would start high (Fat Face Serif at 58 mm, the script
  Great Vibes at 67 mm), so they are not in the list.
- Slab Heavy, Shrikhand and the other heavy fonts have a good font in the
  program again. Slab Heavy and Shrikhand with satin letters stay in the
  package of the fonts of earlier versions.
- A fill letter has a rough edge at a small size, and a fill has no
  border. A satin border around fill letters is not made. Between 15 mm
  and about 25 mm the satin font of the same source can look better, if
  it has a good score.
- A font that the user converts with the stitch type Fill is a fill font
  now: it gets the new score and a range from 15 mm.
- The local width is in steps of 0.5 mm at the design size (25 mm), so
  the start of the range of a font with thin strokes is coarse.
- The program has 65 embroidery fonts. The corpus design
  `embroidery-fonts-8` shows the fill fonts.
- No sew-out confirms the fill fonts.
