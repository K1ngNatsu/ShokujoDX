# ADR 0015: Lettering objects keep their letter shapes; satin letters come from glyph strokes

Date: 2026-10-22 · Status: Accepted

## Context

Roadmap P4 adds lettering. A lettering object comes from a text and a
font, but the stitch engine (`emb-stitch`) must not read fonts, and a design
must open on a computer that does not have the font. TrueType fonts give
outlines, not satin columns: satin letters need the strokes of each glyph.

## Decision

- A new engine crate, `emb-letter`, reads TrueType and OpenType fonts with
  `ttf-parser` (pure Rust, no `unsafe`) and makes the letter shapes. Font
  files are read by the app, not by the engine crates.
- `ObjectKind::Lettering` keeps the text, the font name and style, the
  settings, a placement transform and the letters. Each letter has parts
  that are normal object kinds (satin or fill). Transforms, hit tests and
  stitches use the parts. The app makes the parts again when the text,
  font or a setting changes, and keeps the placement.
- Satin letters come from the strokes of the glyph (our own method, see
  `docs/design/p4-plan.md`): a distance transform and a thinning on a grid,
  pruning of short branches, straight continuation at junctions, ends
  extended to the shape, rails from lines across the middle line to the
  outline. When the strokes are too wide or do not cover the glyph, the
  automatic stitch type uses a tatami fill of the glyph outline.
- Bundled fonts are under the SIL Open Font License, with the license text
  next to each font file.

## Consequences

- A project file has the letter shapes, so it is larger than one with the
  text only, and a font change on another computer does not change a saved
  design until the user edits the text.
- Every new per-object feature must handle `ObjectKind::Lettering` (often
  by delegating to the parts).
- The stroke method is a heuristic. Some glyphs (script fonts, very bold
  or decorated fonts) sew better as fill; per-letter editing (Milestone 3)
  lets the user correct a letter.
