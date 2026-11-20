# Plan: Japanese lettering (phase P20)

Date: 2026-11-20 · Status: Accepted (owner, 2026-11-20: before v0.4.0).
Milestones 1 and 2 are done (ADR 0141, ADR 0142).

## Goal

Hiragana, katakana and kanji look very good in embroidery, and the owner
wants them to sew well. The owner saw a Japanese character in tatami that
should sew well as satin. This phase makes the lettering engine sew
Japanese text as a digitizer does: satin columns that follow the strokes,
no lumps where strokes cross, few trims, sizes that are clear, and a
Japanese embroidery font of the program.

The engine is the same for all scripts, so Chinese and Korean text gets
the changes too. The tests and the exit tests are for Japanese.

## What the measures of 2026-11-20 show

The measures used the 30 fonts with Japanese characters on the owner's
computer (Yu Gothic, MS Gothic, Kozuka Gothic and Mincho, Malgun Gothic,
Microsoft JhengHei and YaHei, SimSun), and 149 characters: 46 hiragana,
46 katakana and 57 common kanji.

| Finding | Number |
| --- | --- |
| The stroke finder covers the shape of each character with satin columns | 85 % to 98 % of each character; not one character below the limit of Auto (85 %) |
| **The tatami comes from one width rule.** In Auto, a character is satin only if no column is wider than 8 mm (`glyph::AUTO_MAX_SATIN_MM`); else all of it is one tatami fill | At 50 mm: 128 of 149 characters in Yu Gothic Bold, 79 of 149 in MS Gothic |
| Most of these characters have only one or two wide columns | Yu Gothic Bold at 50 mm: 92 of the 100 characters with a column wider than 8 mm have one or two; the middle column is 7.1 mm |
| The width is measured at the widest point of the middle line of a stroke. Where strokes meet, that point is the size of the place where they meet | MS Gothic at 50 mm: 37 characters have a column wider than 8 mm, but 79 become tatami |
| **Crossings have two layers of satin.** Both columns cover the place where two strokes cross, and the column that goes through is wider there | 10 % to 16 % of the area of 刺, 繍, 東 and サ; 9 % of 龍; near 0 % of う and り |
| **Letters of a font of the computer have no route.** The route of the letters with few trims (ADR 0113) is only for the embroidery fonts that the converter makes | Yu Gothic Bold at 20 mm: 刺繍東京 44 trims, 22 with routes; 龍 17 and 8; ありがとう 14 and 9 |
| The converter takes only Latin characters (`embfont::default_chars`) | No Japanese embroidery font can be made today |
| The height of lettering is the height of a Latin capital | A kanji is about 1.3 times the height that the user types |
| A thin stroke stays a satin column at each size | Kozuka Mincho at 25 mm: the middle column is 2.2 mm, so at 10 mm many columns are under 1 mm |

## Milestone 1: a Japanese reference set and its measure (S)

The other milestones need a measure that CI runs, as the fonts have their
quality score (ADR 0115).

1. A reference set: the 46 hiragana, the 46 katakana, the 80 kanji of the
   first school year, the 57 kanji of the measures above, and words that
   a shop sews (names, 刺繍, 東京, ありがとう).
2. A Japanese test font in the repository: a subset to the reference set
   of a font with the SIL Open Font License, in `tests/data` with its
   license text. The candidates are Noto Sans JP, Zen Maru Gothic and
   M PLUS 1p. Before the download, the owner approves the font and the
   download, and the license is checked for Reserved Font Names (a subset
   is a Modified Version).
3. A measure (`emb-cli lettering-bench`, and a test that keeps its
   numbers): for each character at 15, 25, 50 and 80 mm, the stitch of
   each part, the part of the shape that two parts cover, the part outside
   the shape, the narrowest and the widest column, and the trims of each
   word.

**Exit test:** the measure runs in CI and gives the numbers of today (the
baseline of the milestones below).

**Done (2026-11-20, ADR 0141).** The reference set has 248 characters (80
hiragana, 57 katakana, the 80 kanji of the first school year, 26 more
kanji, 5 punctuation marks) and eight words. The test font is the Bold
instance of Noto Sans JP 2.004, kept for these characters
(`crates/emb-cli/tests/data/noto-sans-jp-bold-subset.ttf`, 131 KB, made by
`tools/make-test-font-jp.py`; the owner approved the font and the
download). `emb-cli lettering-bench` prints the measures, and
`tests/lettering_bench.rs` keeps them as a snapshot (4 s).

The first run found a fault: the strokes of Noto Sans JP overlap (a
variable font), and the engine read overlapping rings by even-odd nesting,
so letters had holes where strokes overlap, fill parts on each other and
columns outside the letter (14 % of its area on average at 15 mm). Font
outlines now use the nonzero rule (`region::without_overlaps`); fonts with
clean outlines do not change.

The baseline in the test font:

| Measure | 15 mm | 25 mm | 50 mm | 80 mm |
| --- | --- | --- | --- | --- |
| Characters all tatami (of 248) | 0 | 3 | 247 | 247 |
| Two layers, mean (highest) | 6.3 % (24.9 %, ャ) | 6.2 % (24.9 %) | 0 (all fill) | 0 (all fill) |
| Characters with two layers on more than 3 % | 171 | 168 | 0 | 0 |
| Characters with a column under 0.8 mm | 9 | 6 | 0 | 0 |
| Parts outside the letter, mean (highest) | 0.4 % (1.6 %) | 0.4 % (1.6 %) | 0 % | 0 % |
| Trims of 刺繍 / 東京 / 龍 | 26 / 17 / 17 | 27 / 14 / 18 | 6 / 4 / 2 | 6 / 4 / 2 |

(At 50 mm and 80 mm the trims are few because the letters are fills.)

## Milestone 2: the stitch of each column (M)

This milestone fixes what the owner saw.

1. In Auto, the stitch is chosen for each column, not for the whole
   character:
   - up to the satin limit (8 mm): a satin column;
   - to a second limit (about 12 mm): a satin column with split stitches
     (`SatinParams::split_mm`), so that no stitch is too long;
   - wider: a column with the column type Tatami
     (`ColumnPattern::Tatami`), so the rows follow the stroke.
   A part of the shape that no column covers is a fill only if it is
   large; small parts count as covered (as `glyph::add_missed_strokes`
   does).
2. The width of a column is its real width between its two sides, not
   the size of the place where it meets another stroke.
3. The same choice applies in the converter (the fill letters above the
   largest size of the columns, ADR 0120, stay for Latin fonts).

**Exit test:** with the test font, in Auto from 15 mm to 80 mm, no
character of the reference set is all tatami, and each column wider than
8 mm sews with split stitches or as a tatami column. The lettering of the
corpus designs keeps its stitches, or the snapshots change with a note.

**Done (2026-11-20, ADR 0142).** The width of a column is the largest
width that it keeps over 1.5 times that width (`glyph::column_width`), so
the place where it crosses or meets another stroke does not count (木 at
50 mm: 17.5 mm at the junction, 10.1 mm for the stroke). In Auto, each
column gets its stitch (`glyph::auto_column`): satin up to 12 mm
(`AUTO_MAX_SATIN_MM`, now the longest satin stitch `quality::LONG_MM`),
with the stitches longer than 7 mm split (the auto split of all satin
columns); a tatami column above it. Embroidery fonts get the same choice
in Auto below their fill letters, and the lettering keeps a tatami column
when it applies its satin settings. The exit test is in
`tests/lettering_bench.rs`; the golden snapshots did not change.

Not done: a fill for a large part that no column covers. The stroke
finder covers each character of the reference set more than the limit
(the lowest is 88 %), so a letter under the limit stays a fill letter.

| Measure | 15 mm | 25 mm | 50 mm | 80 mm |
| --- | --- | --- | --- | --- |
| Characters all tatami (of 248) | 0 (0) | 0 (3) | 0 (247) | 0 (247) |
| Satin / split satin / tatami columns | 1,249 / 0 / 0 | 1,241 / 8 / 0 | 176 / 1,073 / 0 | 19 / 394 / 836 |
| Columns wider than 8 mm with no split stitches | 0 | 0 | 0 | 0 |
| Two layers, mean | 6.3 % | 6.3 % | 6.3 % | 6.3 % |
| Trims of 刺繍 / 東京 / 龍 | 26 / 17 / 17 | 27 / 14 / 18 | 25 / 17 / 18 | 27 / 18 / 18 |

(In brackets: the baseline of Milestone 1. At 50 mm and 80 mm the letters
are columns now, so they have the two layers of the crossings and the
trims of the columns, as at 15 mm: Milestones 3 and 4.)

## Milestone 3: crossings and junctions (L)

This milestone gives the largest gain on fabric, and it needs the most
care.

1. Where two columns cross, one column goes through, and the other is
   cut into two columns that stop at its sides, with a small overlap so
   that there is no gap. The column that goes through has the width of
   its stroke at the crossing, not the width of the crossing.
2. Where a stroke ends on another stroke (the T and L shapes of kanji),
   the end of its column meets the side of the other column, as the
   route cuts a column at a junction (ADR 0113).
3. The stroke that goes through is the longer stroke, or the stroke that
   sews first in the order of the route, so that the cut stroke sews on
   top. The rule is chosen with the sew-out of item 4.
4. A test sheet of the reference set at 25 mm and 50 mm for a sew-out
   (owner).

**Exit test:** with the test font, two layers on at most 3 % of each
character of the reference set (today 5 % to 16 % on kanji), and at least
95 % of each shape covered. The sew-out of the test sheet has no lumps at
crossings and no gaps (owner).

## Milestone 4: routes for the fonts of the computer (M)

1. Lettering in a TrueType or OpenType font gets the route of each
   character (ADR 0113): the order and the directions of its parts with
   the fewest trims, and travel runs under the strokes that sew later.
2. The route of a character is made the first time the character is used
   and kept for the font, in a background thread: typing stays quick, and
   a new character shows with no route for a moment. A route takes about
   0.5 s for a kanji today.

**Exit test:** with the test font, the trims of the reference words are
at most half of today (刺繍東京 at 20 mm: at most 22). A new kanji shows
at once while its route is made.

## Milestone 5: sizes and thin strokes (S)

1. The height of lettering in Japanese is the height of its characters:
   for a font with Japanese characters, the height of the ideographic box
   of the font, not the height of a Latin capital. Lettering that is in a
   saved design keeps its size.
2. A column that is narrower than the narrowest satin (about 0.8 mm) sews
   as a run, or as a bean stitch where it is a stroke of its own.
3. The Lettering window shows the smallest size at which the characters
   of the text sew well (from the narrowest column), as it does for
   embroidery fonts.

**Exit test:** 刺繍 at a height of 20 mm is 20 mm high (± 5 %); at 10 mm,
no satin column of the reference set is narrower than 0.8 mm.

## Milestone 6: a Japanese embroidery font (L)

1. The converter takes sets of characters: Latin (as today), kana, and the
   2,136 jōyō kanji with Japanese punctuation. A route of all characters
   uses all processors.
2. The quality score (ADR 0115) counts the two layers and the narrow
   columns of Japanese characters.
3. One or two tuned Japanese embroidery fonts (a gothic, and a rounded
   gothic) from fonts with the SIL Open Font License, made by
   `tools/make-embroidery-fonts.py` with their license texts, in a font
   package (File > My Assets > Import a package), as the font packages of
   P17. All the kanji make a large file; the package keeps the program
   small.

**Exit test:** a font package with the kana and the 2,136 jōyō kanji,
with a quality score of 65 or more. A test sheet sews well on the owner's
machine.

## Order

Milestone 1 first (the measure). Then Milestone 2, which fixes what the
owner saw, and Milestone 4, which does not depend on it. Milestone 3 after
Milestone 2, because it changes the same columns. Milestone 5 at any time.
Milestone 6 last, because it uses all the others.

## Not in this phase

- Vertical text with the vertical forms of punctuation (the OpenType
  feature `vert`). Stacked text (ADR 0098) puts the characters under each
  other already.
- Furigana (small kana over kanji).
- A stroke order of calligraphy (the order in which a brush writes).
  The route finds the order with the fewest trims.
- Tests with Chinese and Korean text. They get the changes, but this
  phase does not measure them.
