# ADR 0100: A second pass of the stroke finder

Date: 2026-11-17 · Status: Accepted

## Context

A TrueType letter becomes satin columns through the stroke finder
(`emb_letter::stroke::strokes`): a skeleton of the glyph, its chains, and
two rails for each chain. In the stitch type Auto (TrueType lettering and
the conversion to embroidery fonts), a letter is satin only if its strokes
cover at least 85 % of its area (`glyph::AUTO_MIN_COVERAGE`); else the
whole letter is a fill.

The owner found a fill "J" in a satin font (GothicE, converted to an
embroidery font). An audit of all the fonts (2026-11-17) found the same
in each bundled embroidery font: for example "p" and "q" of Anton, "p"
and "u" of Pacifico, ten letters of Block Sans, and 22 letters of
GothicE. The cause is not the threshold: the first pass loses a stroke
where strokes meet (the stem of a "p", a leg of an "x"), so the coverage
falls under 85 %. A lower threshold would sew letters with no stem.

## Decision

`GlyphShapes::new` runs a second pass (`glyph::add_missed_strokes`) for
each letter whose first pass covers less than 85 %. Each part of the
glyph that the strokes do not cover is one of these:

- **Small** (less than 3 % of the glyph): a notch where strokes meet. It
  counts as covered.
- **Broad** (a circle fits in it with a radius of 0.25 of the widest
  stroke, or 0.035 of the capital height, the smaller): a lost stroke. The
  stroke finder runs on the part (a little larger, so the strokes overlap
  at the join), and its strokes that cover at least 15 % of the part are
  added.
- **A sliver** (not broad, and at least 30 % of its edge touches the
  strokes): the stroke is a little narrower than the outline there. No
  stitches; it counts as covered.
- **Else a thin stroke on its own** (a hairline): it gets strokes as a
  broad part does.

The coverage is then the area that the strokes cover, with the small parts
and the slivers, over the area of the glyph. The rule of Auto does not
change.

A letter that the first pass covers well enough does not change, so
TrueType lettering that was satin before sews the same.

## Consequences

- Of the four bundled fonts made from the fonts in the repository, Block
  Sans keeps one fill character (×) of ten, Anton none of three, Pacifico
  none of eight, Varsity one (×) of three. GothicE has none of 22. Only the
  former fill letters change.
- The golden designs `sewout-towel` and `sewout-cap` change: the "u" of
  Pacifico in "Joshua" is satin now (2 more trims). They need a new
  sew-out.
- The other 26 bundled fonts come from the google/fonts repository
  (`tools/make-embroidery-fonts.py`); they get the change when they are
  made again. An embroidery font that a user converted gets it when the
  user converts the TrueType font again.
- A letter can have a stroke of the second pass that overlaps a stroke of
  the first pass a little at the join.
