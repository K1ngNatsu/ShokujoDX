# ADR 0145: The height of Japanese text, and thin strokes

Date: 2026-11-20 · Status: Accepted (P20 Milestone 5,
`docs/design/p20-plan.md`)

## Context

The height of lettering is the height of a Latin capital ("H"). In a font
with Japanese characters, a kanji is higher than a capital: 刺繍 at a
height of 20 mm in the test font (Noto Sans JP Bold, capital height 733 of
1000 units) was 25.7 mm high. A user who types 20 mm wants characters
20 mm high.

At small sizes, some columns of Japanese characters are too narrow for
satin (the bench at 15 mm: 0.44 mm in 校). The Lettering window shows no
size for the characters of a TrueType font; the size range of a TrueType
font comes from Latin letters ("IlHnoiu").

## Decision

1. **The height of the characters.** `Lettering::character_height` (new
   lettering: `true`; a design saved before: `false`, so its lettering
   keeps its size). With it, the height of text with Japanese, Chinese or
   Korean characters (`layout::is_cjk`: kana, kanji, Hangul syllables) is
   the height of those characters (`layout::height_unit`):
   `LetterSource::character_height`, the middle height of the kanji
   国永東字書漢日 that the source has (at least three;
   `font::character_height_of`), from the glyph boxes of a TrueType font
   or the letters of an embroidery font. Text with no such character, and
   a source with no kanji, keep the capital height. The layout, stacked
   text and the lettering bench use it (the bench sizes are the heights of
   the characters now).
2. **Thin strokes** (`glyph::thin_run`, in `glyph::auto_stitches`, the
   stitch type Auto). A satin column narrower than `AUTO_MIN_SATIN_MM`
   (0.8 mm, the narrowest satin of the fonts for small text) in its middle
   (the width that half of its length is narrower than, measured in steps
   of 0.25 mm as the bench does) sews as a run along its middle line: a
   bean stitch (three passes) if it is at least `AUTO_BEAN_MM` (2 mm) long,
   a stroke of its own; else a single run. Only in the letters of a
   TrueType or OpenType font: the letters of the embroidery fonts were
   checked at their sizes (with it, a few narrow parts of the corpus
   designs `embroidery-fonts`, `team-names` and `sewout-cap` became runs).
3. **The smallest size of the text** (`LetterSource::text_size_range`). For
   text with Japanese, Chinese or Korean characters in a TrueType font, the
   range starts at the height at which the columns of these characters are
   at least 0.8 mm wide, all but the narrowest `quality::RANGE_TAIL` (10 %)
   of their length, and not below 5 mm. `relayout` keeps it in
   `Lettering::range_mm` for lettering with `character_height` (the design
   checks warn below it). The Lettering window shows "The characters of
   this text sew well from a height of N mm" (in the warning color when the
   height is less). The tooltip of the height says what it is.
4. **The bench** has 10 mm too: the sizes are 10, 15, 25, 50 and 80 mm. Its
   test checks the exit test (at 10 mm no satin column under 0.8 mm; 刺繍 at
   20 mm is 20 mm high, ± 5 %), and the coverage of Milestone 3 from 15 mm
   (at 10 mm, the thin strokes are runs, which cover a line, not an area).

## Consequences

- 刺繍 at 20 mm is 20.1 mm high (25.7 mm before); 東京 20.0 mm; ありがとう
  19.7 mm; HELLO in the same font keeps the capital height.
- At 10 mm, the narrowest satin column of the reference set is 0.81 mm; 36
  columns are runs. At 15 mm, 4 columns are runs.
- The smallest sizes of some texts: 刺繍 9.5 mm, 東京 8.5 mm, 書 11 mm,
  ありがとう 7.5 mm.
- The numbers of the bench change, because its sizes are the heights of the
  characters now (a character at "15 mm" is 15 mm high, not about 18 mm):
  for example 刺繍 at 15 mm has 13 trims (14 at 25 mm).
- Lettering of a saved design keeps its size until a person changes it
  in a new lettering object. A new lettering object with Japanese text in
  a font of the program is a quarter smaller than before at the same height.
- The audit of P21 (2026-11-24): `glyph::thin_run` measures a column at
  20,000 places at most (`THIN_MAX_PAIRS`; a column of 5 m at the step of
  0.25 mm). The time of the layout went up with the height of the
  lettering: lettering 1,000 km high did not end in two minutes.
