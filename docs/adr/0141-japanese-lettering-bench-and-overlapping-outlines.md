# ADR 0141: The Japanese lettering bench, and font outlines that overlap

Date: 2026-11-20 · Status: Accepted (P20 Milestone 1,
`docs/design/p20-plan.md`)

## Context

Phase P20 makes Japanese text sew well. Its milestones change the
lettering engine, so they need a measure that CI runs, with a Japanese
font that CI has: the fonts of a computer are not in CI, and their
licenses do not let the project keep them.

The first run of the measure on Noto Sans JP found a fault of the engine.
Noto Sans JP of Google Fonts is a variable font. Its letters are strokes
that overlap (each glyph has the overlap flag of TrueType: 名 is seven
rings). The fonts of Windows that the plan measured (Yu Gothic and others)
have clean outlines. The engine read the rings by even-odd nesting
(`glyph::regions`): a ring whose first point is in another ring is a hole,
and rings that overlap are separate areas. So a letter of such a font had
fill parts that lay on each other (two layers in a fill letter), holes
where strokes overlap, and satin columns outside the letter (on average
14 % of the area of the letter at 15 mm, 153 % for 名). All variable fonts,
and the static fonts made from them, can have such outlines, also Latin
fonts that a user installs.

## Decision

1. **A test font in the repository.** `crates/emb-cli/tests/data/
   noto-sans-jp-bold-subset.ttf`: the Bold instance (wght 700) of Noto Sans
   JP 2.004, kept only for the 248 characters of the reference set, the
   characters of the test words and "H" (131 KB). Its license is the SIL
   Open Font License (`noto-sans-jp-OFL.txt`); the Reserved Font Name is
   "Source", so the subset keeps the name Noto Sans JP.
   `tools/make-test-font-jp.py` makes it from the variable font with
   fontTools (the same bytes at each run). The owner approved the font and
   the download (2026-11-20). The font is test data: no package has it.
2. **The lettering bench** (`emb_cli::lettering_bench`, `emb-cli
   lettering-bench <font> [--face <n>] [--letters]`). For each character of
   the reference set (the kana, the 80 kanji of the first school year, 26
   kanji with many strokes and crossings, punctuation) at 15, 25, 50 and
   80 mm in Auto: the stitch of each part (satin, split satin, tatami
   column, fill, run), the part of the shape that the parts cover, the area
   outside it, the part with two layers (`quality::areas`), the narrowest
   and the widest column (`quality::column_widths`); and the trims of
   eight words. The test `tests/lettering_bench.rs` keeps the report as a
   snapshot: a change of it needs a note in the commit message, as for the
   golden snapshots.
3. **Outlines with the nonzero rule.** `Font::contours` merges rings that
   overlap with the nonzero rule of TrueType and OpenType
   (`emb_geom::region::without_overlaps`): each ring is an outer ring or a
   hole. Rings that do not overlap come back as they are (the nonzero and
   the even-odd areas are the same), so a font with clean outlines gives
   the same letters as before. The function uses `i_overlay` (the back end
   of `geo`, ADR 0002) as a direct dependency of `emb-geom`, with no
   default features (no threads), the same version and features as in
   `geo`; `Cargo.lock` gets no new crate.
4. **`emb-letter` is optimized in debug builds** (`opt-level = 2`, as
   `emb-stitch`, `emb-render` and `emb-app`): the bench takes 4 s in the
   tests (42 s with no optimization), and the stroke finder runs at each
   new letter in the app.

## Consequences

- The baseline of P20 in the test font (`docs/design/p20-plan.md`,
  Milestone 1): at 50 mm and 80 mm, 247 of the 248 characters are all
  tatami; at 15 mm and 25 mm, the parts have two layers on 6.3 % of their
  area on average (24.9 % for ャ), 171 characters over 3 %; 9 characters
  have a column under 0.8 mm at 15 mm; 刺繍 has 26 trims at 15 mm.
- After the merge, the parts outside the letters are 0.4 % on average
  (1.6 % at most), and a fill letter has one layer. The test checks both.
- A font of the computer with overlapping strokes now gives the right
  letters, and its sample in the font lists has no holes where strokes
  overlap.
- A new character in the reference set needs a new test font (the tool),
  and a new snapshot with a note.
