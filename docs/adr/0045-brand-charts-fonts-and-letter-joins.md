# ADR 0045: Brand thread charts, more embroidery fonts, and joins of satin letters

Date: 2026-10-25 · Status: Accepted (P11 Milestone 2)

## Context

P11 Milestone 2 asks for more thread charts and 20 or more bundled
embroidery fonts, also fonts for small text. The program had the Brother PEC
chart only (the earlier rule: only with the permission of the maker) and
ten embroidery fonts of 0.5 to 1 MB of JSON each in the program. The
density map (ADR 0044) showed 6 to 9 layers of thread at the joins of the
satin letters of the bundled fonts.

## Decision

1. **Thread charts from Ink/Stitch.** The owner accepted the palettes of
   Ink/Stitch (GPL-3.0, the license of this project) as the source
   (2026-10-25). The 74 brand charts of commit 9a5b983 (all but RAL, a
   paint standard) are in `data/threads/inkstitch/`: the data of Ink/Stitch
   with a new name line (the brand, with no "Ink/Stitch:") and a source
   line. `ThreadChart::builtin` has Brother PEC first, then these.
2. **Joins of satin letters** (`emb_letter::stroke`). A stroke that ends
   where another stroke goes through (the stem of a "T"), a piece that
   starts at a corner after another piece, and an end that was extended to
   the edge of the shape into an earlier piece (the legs of an "A" with a
   flat top) are cut back until their middle line is out of the other
   stroke but 30 % of its half-width (`JOIN_OVERLAP`), so the columns still
   meet with no gap. At most half of a stroke goes. On the lettering corpus
   designs the places over 5 layers went from 65 to 20, and the stitches
   from 3 to 5 % fewer. TrueType lettering changes at once; the embroidery
   fonts were made again.
3. **Fonts for small text.** `emb-cli font --small-text` (and
   `embfont::convert_with`) gives a size range from 4 mm and from satin
   columns 0.8 mm wide (else 5 mm and 1.2 mm); the font has
   `small_text: true`. Four bundled fonts are for small text: Atkinson
   Hyperlegible (Bold), Soft Round (Varela Round), Round Sans (Paytone One)
   and Round Heavy (Lilita One). The Fonts window can show them only.
4. **Thirty bundled embroidery fonts**, made by
   `tools/make-embroidery-fonts.py` from OFL fonts: the first four from
   `data/fonts`, the others from google/fonts commit e7db6ca. A font with
   a Reserved Font Name has a new name: Bold Script (Lobster), Slab Heavy
   (Alfa Slab One), Casual Script (Courgette), Soft Round (Varela Round),
   Round Sans (Paytone One), Round Heavy (Lilita One), Poster Bold (Passion
   One), Geo Slab (Arvo), Friendly Serif (Bree Serif), Slab Round (Patua
   One), Fat Face Serif (Abril Fatface), Brush Hand (Kaushan Script), Retro
   Script (Cookie), Vintage Script (Grand Hotel) and Retro Display
   (Righteous). Scripts with hairline strokes (Pinyon Script from 48 mm,
   Sacramento from 25 mm) were left out; Great Vibes (from 27.5 mm) stays
   for large names.
5. **The fonts are compressed in the program.** `emb-letter/build.rs`
   deflates each `.embfont` (flate2, pure Rust); `bundled::embroidery_text`
   inflates a font once, when it is first used. The files in the
   repository stay JSON, so changes can be read. The converter now writes
   coordinates such as 3.32, not 3.3200000000000003 (15 % smaller files).
   The release program went from 50 to 45 MB with three times the fonts.

## Consequences

- The golden snapshots of the lettering designs changed (the joins and
  the fonts made again); `embroidery-fonts-3` and `-4` are new, with ten
  of the new fonts each.
- Satin columns 0.8 mm wide are narrow for a home machine: the exit test
  (4 mm and 6 mm text of each small-text font on the Topaz 20) shows if the
  small-text range must start higher.
- A chart of Ink/Stitch that changes upstream does not change here until
  the files are copied again.
