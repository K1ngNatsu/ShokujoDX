# ADR 0119: A richer quality score, and the best of columns and fill for each letter

Date: 2026-11-19 · Status: Accepted (on pictures only; no sew-out)

## Context

The review of P17 (roadmap, 2026-11-19) found two things about the fonts:

- **The score did not see what the eye sees.** The quality score of a
  font (ADR 0115) had three measures: the part of the shape of a letter
  that has stitches, the columns that are too narrow at the smallest
  size, and the stitches that are too long at the largest size. The
  audit of a font (`emb-cli font-audit`) had two more that the score did
  not use: the area of a letter outside its outline, and the area with
  two or more layers.
- **A font was all columns or all fills.** In the 127 fonts with satin
  letters, no letter or digit was a fill. A letter became a fill only if
  the stroke finder covered less than 85 % of it (ADR 0100), and such a
  letter had a fixed score of 0.5.

Lane 5 of the review: the converter makes both for each letter and keeps
the one with the better score, and the score gets the measures of the
audit. The owner started the lane before a sew-out (2026-11-19), so each
choice here comes from proof pictures and from counts.

## What the measures showed

A table of each letter and digit of the 127 fonts (`emb-cli font-quality
--letters`), with the same measures of the fill letter of the character:

- **Outside.** Half of the 7,874 column letters have less than 0.7 % of
  their area outside the outline, and 95 of 100 have less than 2.4 %.
  The letters with more have a fault on the proof pictures: points that
  stick out of the shape (the "r" of Round Heavy, 6 %), a column across
  a corner, or a closed hole (the "A" of Friendly Hand, 32 %). The score
  gave the "r" 100 of 100.
- **Two layers.** Half of the letters have less than 4 %, and 95 of 100
  less than 19 %: the places where the strokes meet. The letters with
  much more have two columns on one stroke (the "t" of Kanit, 58 %; the
  "u" of Geo Slab, 45 %, with a score of 85).
- **Trims.** A count of the trims from the parts of a letter gave 11,846
  for the fonts; the stitch engine makes 683 (it hides short moves under
  the stitches). Only the stitch engine can count them, and `emb-letter`
  does not use it.
- **Fill letters.** With a fill letter at a score of at most 0.5, about
  140 letters of 35 fonts had a better fill letter. Most of them are in
  heavy fonts (Slab Heavy, Giant Round, Poster Heavy) with a size range
  from 5 or 6 mm. A fill does not sew well in a letter of that size: the
  fill fonts start at 15 mm for this reason (ADR 0117). And a letter
  with its full shape, but with columns that are too narrow or too wide
  for the size range (a dot, whose ends are narrow; a wide stem), also
  had a better fill letter by the score: a fill does not repair that.

## Decision

- **The score has five measures.** `LetterQuality` and `Quality` have
  `outside` (the area of the parts outside the outline, as a part of the
  area of the outline) and `overlap` (the part of the area of the parts
  with two or more layers); `quality::areas` makes them with the
  coverage, on the same grid. Up to 2 % outside takes nothing from the
  score of a letter, and 20 % more takes all of it (`OUTSIDE_FREE`,
  `OUTSIDE_ALL`). Up to 20 % with two layers takes nothing, and 50 % more
  takes all of it (`OVERLAP_FREE`, `OVERLAP_ALL`). The limits of a font
  (65 and 85) stay.
- **The trims are not in the score.** The route of a letter (ADR 0113)
  leaves few of them, and the audit counts them with the stitch engine.
- **A fill letter has the measures of a fill** in each font, not only in
  a fill font: the part of its area that is too narrow at the smallest
  size, and its holes (`quality::fill_widths`, ADR 0117). In a font with
  a smallest size below 15 mm (`FILL_MIN_HEIGHT_MM`), all of it counts as
  too narrow. In a font of satin letters it has not the look of the other
  letters, so its score is at most 0.5 (`FILL_LETTER`, as before).
- **The converter chooses for each letter** (`EmbFont::choose_fills`),
  after the ends of the columns, the corrections and the route. For each
  letter with columns it makes the fill letter of the character
  (`quality::fill_letter`). The fill letter takes its place if its score
  is 0.05 (`FILL_GAIN`) more than the **shape score** of the letter with
  columns (`LetterQuality::shape_score`: the coverage, the part outside
  and the layers, without the narrow and the long measure). So a fill
  repairs a shape; it does not repair a size. A letter that a person
  corrected (ADR 0112) stays as it is. The size range does not change.
- **Where it runs.** `emb-cli font` with the stitch type Auto (the
  default; `tools/make-embroidery-fonts.py` uses it for all fonts but the
  fill fonts), and Convert in Lettering > Fonts with the stitch type
  Automatic (`fonts::convert_font`). The stitch type Satin keeps all
  columns.
- **The Fonts window** shows the two new measures of a font with satin
  letters, and the part of its letters that are fills.

## Consequences

- All fonts were made again. The mean score of the 127 fonts is 81.6
  (82.3 before). 72 fonts keep their score, 53 have 1 to 5 points less
  (Slab Heavy 59 to 54, Poster Bold 78 to 74, Bouncy Brush 70 to 66,
  Funky Slab 79 to 75) and 2 have 1 point more. 49 fonts have 85 or more
  (52 before). The lowest font of the program has 71, as before. No font
  of the program or of a package falls below 65, so each font keeps its
  place (ADR 0116). No size range changed.
- **No letter of a font of the program changed**: only the scores in the
  font files. The corpus designs of the fonts have the same stitches.
- **32 letters of five package fonts are fills now**, in fonts with a
  smallest size of 19.5 mm or more: Fat Face Serif (17: "P", "g" and 15
  marks and letters with accents), Great Vibes (9: "5", "e", "r", "?"
  and letters with accents), Swash Serif ("4", "5", "V"), Playball ("x"
  and the bullet) and Allura (the bullet). On the proof pictures, the
  columns of these letters left a tenth of the shape or more open (the
  "e" of Great Vibes 26 %); the fill letters have the full shape. A word of such a font can have satin
  letters and fill letters side by side; the score counts this with the
  0.5 of a fill letter.
- **The choice per letter does little for the heavy fonts.** Their poor
  letters are poor at each size, and their size ranges start at 5 to
  9 mm, where a fill is not an answer. A font file has one shape for each
  letter. The better answer is a choice by size: the font keeps the
  columns and the fill letter of each character, and the layout takes the
  fills from the size at which the columns are too wide, as a TrueType
  font does with the stitch type Automatic. Then a word is all satin or
  all fill, and each satin font goes up to 300 mm. This is a change of
  the font format and of the layout; it is a proposal in the roadmap, not
  a part of this decision.
- The letters with a low score from the new measures (two columns on one
  stroke, points outside the shape) are faults of the stroke finder. The
  table of `emb-cli font-quality --letters` lists them for later work.
- A font that the user converts with Automatic gets the choice too. For
  a font with a size range from 15 mm, some of its letters can be fills.
- A font file of an earlier version has no `outside` and `overlap`; they
  read as 0, and its score stays the score in the file until the font is
  converted again.
- No sew-out confirms the limits of the two new measures, the 0.5 of a
  fill letter or the 15 mm.
