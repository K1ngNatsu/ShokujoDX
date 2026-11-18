# ADR 0112: Letter corrections and font test sheets

Date: 2026-11-18 · Status: Accepted

## Context

The bundled embroidery fonts are conversions of outline fonts (ADR 0016,
0045, 0100). A person cannot make one letter of them better for good: a
new conversion (after each change of the stroke finder) writes all the
letters again. There was also no quick way to sew all the letters of a
font at the sizes of its range (Phase P17 Milestone 4,
`docs/design/p17-plan.md`).

Two facts of the program made a correction of a letter empty:

- The stitch engine found the sew order of the parts of each letter, and
  the direction of each column, from the place of the needle. The order
  of the parts in the font had no effect.
- A lettering object gives its satin and fill settings to all its parts
  (`Lettering::apply_params`). A setting of one part in the font was
  lost.

## Decision

- **A letter can keep its order** (`EmbLetter::fixed`, `Letter::fixed`). A
  fixed letter sews its parts in the order of the font, each column and
  each run from its start to its end. A fill starts near the needle and
  ends near the start of the next part. A connection is a travel stitch
  if it is short or if the parts that sew after it cover it; else it is a
  trim (or a jump), as for other letters. The letter order of the object
  (text, center out, closest) still sets the order of the letters. The
  corner option of the object does not join the columns of a fixed
  letter: the font sets its joins.
- **A part can have its own settings** (`PartTuning` in `EmbLetter::tuning`
  and `Letter::tuning`): an underlay, a value added to the pull
  compensation of the object, and a value added to the fill angle of the
  object. `apply_params` applies them after the settings of the object,
  so a change of the object keeps them. The underlay that the user gives
  to a letter in a design comes first.
- **A correction file for each font** (`emb_letter::correction`, JSON). A
  correction of a letter is a new shape (`parts`, always applies), or
  steps on the converted letter: `order`, `reverse`, `join` (two columns
  become one with a mitre corner, ADR 0086), for a letter with `of`
  parts. It can also set `tuning` and `advance_mm`. A letter with a shape
  or with steps is fixed.
- **The bundled fonts.** The files are
  `data/fonts/embroidery/corrections/<name>.json`.
  `tools/make-embroidery-fonts.py` gives the file to `emb-cli font
  --corrections`, which applies it after the conversion. A correction
  that does not apply (the new conversion gives another number of parts)
  stops the tool with the letter and the cause. A test
  (`the_bundled_fonts_have_their_corrections`) fails if a bundled font
  does not have a correction of its file.
- **The corrections of the user.** The Alphabets window puts a letter of
  an embroidery font in the design as objects (**Place Letter**) and
  writes the selected objects as the correction of the letter (**Save
  Correction**), always as a shape, to `corrections/<name>.json` in the
  fonts folder of the user. The font library applies this file when it
  reads the font (`FontLibrary::corrections_dir`), also to a bundled
  font. The file has the same name and format as the files of the
  repository, so a correction that the owner makes and sews can go into
  the program with a copy of the file.
- **Font test sheet** (`emb_letter::sheet`, window `font_sheet.rs`,
  `emb-cli font-sheet`). A new design with one group for each size: a
  label with the name of the font and the size (a bundled font for small
  text, 5 mm), then the characters in rows that fit a width. The groups
  are lettering objects, so they sew as the lettering of the user does.
  The sizes are the smallest, the middle and the largest size of the
  range; the middle is the size with the same ratio to both ends. The
  window starts with the sizes whose sheet fits the hoop.

## Consequences

- A design of an earlier version does not change: its letters are not
  fixed and have no settings. The bundled fonts have no corrections yet,
  so the fonts and the golden snapshots do not change (the 30 fonts were
  made again and are the same bytes).
- A lettering object keeps its letter shapes (ADR 0015). A correction
  changes lettering that is in a design when the letters are made again
  (a change of the text, the font or a layout setting).
- Steps are small and a person can read and write them, but they depend
  on the conversion. A shape does not, but it does not get better with
  the stroke finder.
- The label of a test sheet is above each group, not next to each row,
  so the rows have the full width.
- The fill angle of a part turns with the placement of the lettering, as
  the fill angle of the object does; it does not follow the turn of one
  letter on an arc (as before).
