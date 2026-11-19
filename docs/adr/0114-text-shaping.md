# ADR 0114: Text shaping for lettering

Date: 2026-11-18 · Status: Accepted

## Context

The layout of lettering took one glyph for each character, from left to
right, with the pair kerning of the font (ADR 0015). That is correct for
most Latin text. It is wrong for the writing systems that need shaping
(the review of 2026-11-18; Phase P17 Milestone 6,
`docs/design/p17-plan.md`):

- Arabic letters have a form for their place in the word, they join, and
  the text goes from right to left.
- Devanagari letters make conjuncts, and some vowel signs are written
  before their consonant.
- Latin fonts have ligatures ("fi"), and script fonts have forms of a
  letter that depend on the letters next to it.

## Decision

- **Two libraries in Rust** do the work, in the new module
  `emb_letter::shape`:
  - `unicode-bidi` 0.3 (MIT or Apache-2.0): the Unicode bidirectional
    algorithm gives the runs of a line and their directions.
  - `harfrust` 0.12 (MIT): a port of HarfBuzz (it follows HarfBuzz
    13.0.0) gives the glyphs of each run and their places, from the
    substitution and positioning tables of the font. It forbids unsafe
    code and has no C code (rule 9).

  Both are in the dependency tree of the program already (`harfrust`
  from the UI library, `unicode-bidi` from another package), so the
  program gets no new package.
- **Runs.** A line is cut into the runs of the bidirectional algorithm,
  and each run into parts with one script (digits, spaces and marks go
  with the letters next to them). Each part is shaped with its direction
  and script. The glyphs come back in visual order, from the left end of
  the line.
- **A source of letters can shape** (`LetterSource::shape`). A TrueType
  or OpenType font does; an embroidery font does not (it has one letter
  for each character, as before). With a source that shapes, the layout
  takes the shaped glyphs of each line.
- **One letter for each cluster.** A cluster is the characters that
  shape together: a ligature, a conjunct, a base with its marks. It is
  one `Letter` of the lettering object: its text is the characters of the
  cluster, its index is the index of its first character, and its parts
  are the parts of its glyphs, each at its place. So the changes of
  single letters (ADR 0017) work on a cluster as on a letter, and the
  stroke finder works on each shaped glyph.
- **Sew order.** The letters of a line are in the order of the text. Text
  from right to left starts at its right end, so it sews in reading
  order. The kerning by hand and the letter spacing add space in visual
  order (before a letter is at its left).
- **Characters with no letter.** Glyph 0 (the font does not have the
  character) sews nothing. `Lettering::missing_characters` lists such
  characters; the Lettering window, Object Properties and the design
  checks show them, with the two bundled fonts.
- **Two bundled fonts** (`bundled::ARABIC`, `bundled::DEVANAGARI`):
  Tajawal Bold (Arabic, 60 KB) and Poppins Bold (Devanagari, 156 KB),
  both under the SIL Open Font License, from the google/fonts repository
  (the commit of `tools/make-embroidery-fonts.py`), not changed, with
  their license texts in `data/fonts`. Both have even strokes, which
  become satin columns. The program also uses them to show these
  characters in its text boxes.

## Consequences

- Latin text in a TrueType font can change when it is made again:
  ligatures, forms that depend on the letters next to them, and the full
  kerning of the font (not only pairs). The golden snapshots did not
  change: their texts have no ligature, and their kerning is the same.
  Lettering in a design keeps its letters until its text or font changes
  (ADR 0015).
- A change by hand of a character that is now in the middle of a ligature
  does not apply (its index is not the index of a letter).
- The "reference shaping" of the exit test is the output of the shaping
  library for the bundled fonts, written down in a test after a check of
  the sewn words by eye. The library passes the test suite of HarfBuzz;
  no run of HarfBuzz itself was compared.
- The Windows program is about 2.3 MB larger (63.4 MB): the two fonts
  (0.2 MB), and the parts of the shaping library that the UI library did
  not use.
- Stacked text (one letter under the other) does not shape.
- There is no option to turn ligatures off.
- The embroidery fonts (also the tuned ones) do not shape: a tuned
  script font has no ligatures.
- The font test sheet has no character set for Arabic or Devanagari; use
  **My text**.
- Other writing systems that the shaping library knows (Hebrew, Thai,
  other Indic scripts) work with a font of the computer; they were not
  checked.
