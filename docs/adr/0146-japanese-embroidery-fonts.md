# ADR 0146: Japanese embroidery fonts

Date: 2026-11-20 · Status: Accepted (P20 Milestone 6,
`docs/design/p20-plan.md`)

## Context

The converter made embroidery fonts with Latin characters only
(`embfont::default_chars`). A Japanese embroidery font needs the kana,
Japanese punctuation and the kanji for general use: the 2,136 jōyō kanji.
Such a font is large: each kanji has many columns, and its route has many
travel runs. The fonts with Japanese characters that the owner can use are
the fonts of the computer (TrueType); an embroidery font sews the same
letters each time, with routes and a quality score (ADR 0113, 0115).

## Decision

1. **The characters** (`emb_letter::japanese`, new): the reference set of
   the lettering bench (moved there from `emb_cli::lettering_bench`;
   `tools/make-test-font-jp.py` reads it there), all kana (`kana`),
   Japanese punctuation, marks and the full-width signs of the keyboard
   (`JAPANESE_MARKS`), and the 2,136 jōyō kanji (`JOYO_KANJI`, the table of
   the Cabinet of Japan of 2010 in its order, read from the English
   Wikipedia list and checked: 2,136 kanji, each one time, the 80 kanji of
   the first school year in it). The table has the forms 塡, 剝, 𠮟, 䇳 and
   頬; `KANJI_FORMS` adds the forms that keyboards and most fonts have (填,
   剥, 叱, 箋) and 頰. `SHOP_KANJI` adds two kanji that are not jōyō kanji
   but that a shop sews and that the reference set has: 繍 (刺繍,
   embroidery) and 龍 (a dragon). The kanji of names that are not jōyō
   kanji (the jinmeiyō kanji) are not in this milestone.
2. **The converter**: `emb-cli font --sets latin,kana,kanji` (latin is the
   default) and `--weight <n>` for a variable font (`Font::from_bytes_with`
   with the axis values; ttf-parser reads variable fonts with its feature
   `variable-fonts`, which adds no crate). The route of all letters uses all
   processors (`EmbFont::route_until`, ADR 0116): about two minutes for one
   font.
3. **Large sizes**: a font with Japanese, Chinese or Korean letters gets no
   fill letters (ADR 0120). In Auto, its wide columns are tatami columns
   that follow the strokes (ADR 0142, 0143), not one fill for each letter,
   so it goes up to 300 mm with its columns. `EmbFont::satin_max_mm` (new,
   optional in the file) keeps the largest size of its satin columns: the
   end of the stitch type Satin, and `column_max_mm` for the score, the
   audit and the test sheets.
4. **The quality score** of a font with Japanese letters also measures the
   reference set (`quality::measured_in`): their coverage, their columns
   that are too narrow at the smallest size and too long at the largest
   satin size, the area outside them and their two layers.
5. **Two fonts in a font package** (`data/font-packages/
   shokujodx-fonts-japanese.zip`, made by `tools/make-embroidery-fonts.py`
   with their OFL texts; File > My Assets > Import a package): Noto Sans
   JP (a gothic, the Bold weight of the variable font) and Zen Maru Gothic
   (a rounded gothic, Bold), from the google/fonts commit of the tool. Noto
   Sans JP reserves the name "Source" and Zen Maru Gothic reserves none, so
   both keep their names. They are not routed with `--extend`: the stroke
   finder takes the ends to the corners (ADR 0143), and the end steps of
   `--extend` would make the cut ends of crossings long again. (ADR 0154
   changed this: the tool passes `--extend` to all fonts, and the converter
   leaves the ends of kana and kanji as they are.)
6. **The app reads a large embroidery font only when it is used**: for a
   font file larger than 4 MB in the fonts folder of the user, the font
   lists read its header (`EmbFont::load_header`: the names, the marks and
   the score); the letters are read when lettering uses the font.
7. **The sew-out kit** of P20 (`tools/make-sewout-kit-p20.py`) has a sheet
   of each font at 20 mm. `emb-cli font-audit` takes `--weight` too, so it
   compares the letters with the outline of the same weight.
8. **Two faults of the stroke finder** that the proof pictures of Zen Maru
   Gothic showed (ADR 0143):
   - A stroke that crosses the same stroke two times (the diagonal of ぬ
     through its loop) lost its part between the crossings: the end of a
     cut piece was trimmed to the last point in the first half of the
     piece that is in the other stroke (`trim_start_out_of`, made for ends
     that go across a stroke and out again). A cut at a crossing now stops
     where the piece first comes out of the other stroke
     (`stroke::trim_start_at_exit`). ぬ: 94.7 % covered before, 98.5 %
     now.
   - The end trim (`stroke::trim_ends`) tested a point against the two
     rails of a column as one ring; a column that goes round and over
     itself has no one ring. It tests the quads between the pairs of points
     now.

## Consequences

- Noto Sans JP: 2,639 letters (Latin, all kana, all 2,136 jōyō kanji and
  the forms of item 1), score 89, 6.5 mm to 300 mm (satin columns to
  49 mm). Zen Maru Gothic: 2,629 letters (all jōyō kanji in the forms of
  JIS X 0208; not 𠮟, 塡, 剝, 䇳, 頰, ゕ, ゖ), score 88, 7.5 mm to 300 mm
  (satin to 61 mm). The exit test of the plan (a package with the kana and
  the jōyō kanji, a score of 65 or more) is met; the sew-out of the test
  sheets is the owner's.
- The font files are large (33 MB and 30 MB of text; the package is
  11.6 MB). They are in a package, not in the program.
- The bundled fonts and the other packages do not change (they have no
  Japanese letters; the new field is not in their files). The fixes of
  item 8 change the letters that lettering makes from TrueType fonts a
  little (see the commit).
- Open: in Zen Maru Gothic, あ is covered 92.6 %: the stroke finder does
  not find the lower part of its inner stroke (the letter as TrueType
  lettering has the same gap). A second pass for each letter with a large
  part that no column covers did not find it either, and it gave other
  letters two layers (よ: 9.6 %), so it is not kept. A letter correction of
  the font (ADR 0112), or a change of the stroke finder, fixes it later; all
  other letters of the reference set have no gap in either font.
- The test `tests/japanese_font.rs` converts the reference set of the test
  font: no fill letters, the range to 300 mm, a score of 65 or more with
  the Japanese characters, the header of the file.
- The audit of P20 (2026-11-20) found that the route left out parts of
  letters with 32 to 64 parts (ADR 0144). The two fonts were made again:
  in Noto Sans JP, 醸 had no stitches and 鬱 had 36 of its 56 columns;
  Zen Maru Gothic had no such letter. The scores stay 89 and 88.
