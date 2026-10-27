# ADR 0016: Our own embroidery font format (`.embfont`)

Date: 2026-10-22 · Status: Accepted

## Context

P4 Milestone 2 adds embroidery fonts: letters that are already embroidery
objects, with a recommended size range. Commercial embroidery fonts use
closed, undocumented formats (the clean-room rule forbids them). A
TrueType font gives only outlines, and the stroke method of ADR 0015 is a
heuristic that can change between versions. Users also need fonts that
they can correct by hand (Milestone 7).

## Decision

- **`.embfont` is a JSON file.** It has a format tag and a version, a
  family and style name, notes (source, copyright and license), the
  design size (`cap_height_mm`: the letters are stored at this capital
  height), the line height, the space width, the recommended size range
  (`min_height_mm`, `max_height_mm`), the letters (for each character: its
  advance and its parts as normal object kinds, satin or fill, in mm, with
  the baseline at Y = 0) and kerning pairs (in mm at the design size).
- **One layout for both kinds of font.** `emb_letter::layout` uses a
  `LetterSource`: a TrueType font makes its letters from glyph outlines,
  an embroidery font scales its stored letters. Lettering objects name the
  kind of font in `FontRef::kind`.
- **Size range.** The converter measures the typical stroke width of the
  font. The minimum height gives satin columns of at least 1.2 mm and is
  never below 5 mm; the maximum gives satin columns of at most 10 mm. A
  TrueType font gets the same range when it is used directly. The
  lettering object keeps the range, and the design checks warn when the
  height is outside it.
- **Starter library.** Embroidery fonts made with `emb-cli font` from OFL
  fonts are in `data/fonts/embroidery/` and are built into the app. They
  are Modified Versions under the SIL Open Font License: a font with a
  Reserved Font Name gets a new name (Lato becomes "Block Sans", Graduate
  becomes "Varsity"), and the license text is next to each file.
- **User fonts** are in the `fonts` folder next to the settings file.

## Consequences

- An embroidery font is independent of later changes to the stroke
  method: a converted font sews the same in each version.
- The files are larger than TrueType files (each letter has its satin
  rails), so the built-in library is small.
- The converter shows a note that the license of a font must permit the
  change; the app cannot check font licenses.
