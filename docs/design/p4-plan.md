# Plan: roadmap phase P4 (lettering)

Date: 2026-10-22 · Status: Done. Milestones 1 to 7 done in software; the exit test (sew-outs) passed on 2026-10-23.

## Goal

Lettering is the main reason many home users buy embroidery software
(roadmap P4). The exit test: names at 10 mm and 25 mm sew legibly on towel
and on a cap, and a list of 20 team names exports in one step.

The exit tests of P1 (sew-outs), P2 and P3 (users) are still open. They
run in parallel.

## Order

Each milestone ends with something a user can try in the app.

| # | Scope | Exit test |
| --- | --- | --- |
| 1 | **TrueType lettering.** A new engine crate `emb-letter`: read TrueType and OpenType fonts, glyph outlines, kerning (`kern` and OpenType pair kerning). Letters as satin (strokes found in the glyph shape), as tatami fill (the glyph outline with holes), or automatic (fill when the strokes are too wide for satin). The lettering object keeps the text, the font, the settings and the letter shapes, so a design opens without the font. Lines, letter spacing, line spacing, justification, height. Letters sew left to right with a travel or a trim between them. Lettering tool and Object Properties; bundled OFL fonts and the fonts of the computer. | Type a name in a TrueType font at 25 mm and at 10 mm, see satin letters in the Stitches view, and export. |
| 2 | **Embroidery fonts.** Our own font format (letters as embroidery objects, with baseline, advance, kerning pairs and a recommended size range). Convert a TrueType font to an embroidery font; a font list with previews and filters; a starter library from OFL fonts; warnings below the minimum and above the maximum size. | Convert a font, letter with it, and see a warning at a size below its range. |
| 3 | **Baselines and letter editing.** Arc, circle and custom-path baselines; manual kerning; move, scale and rotate one letter on the canvas; reshape one letter (a user-refined letter, kept in the design). | Put a name on a circle and change one letter by hand. |
| 4 | **Effects and sequence.** Envelopes (bridge, pennant, perspective, diamond and others), lettering art; stitch type and underlay for each letter; the sew order of the letters (left to right, center out, closest join) and their connections. | A name in a bridge envelope with fewer trims than with the default order. |
| 5 | **Monogramming.** Monogram styles (one to three letters), borders, ornaments, monogram templates. | Make a three-letter monogram with a border from a template. |
| 6 | **Team names and keyboard collections.** Name lists, name drop, export each name as a file or all in one layout; keyboard design collections (a design for each key). | A list of 20 names exports in one step. |
| 7 | **Custom lettering.** Create a letter from selected objects, the alphabet creator, manage alphabets (rename, delete, import, export). Low priority (owner request): import bitmap fonts, BDF (`.bdf`) and Windows raster fonts (`.fon`), as letters from their pixels. | Make a three-letter alphabet and letter with it. |

Then the exit test (roadmap P4 exit).

## Progress

| # | State | Notes |
| --- | --- | --- |
| 1 | Done in software | `emb-letter` (new engine crate): `font` (names, metrics, outlines, kerning from the OpenType "kern" feature and the `kern` table), `grid` (fill, distance transform, thinning), `stroke` (satin columns from glyph strokes), `glyph` (fill regions), `layout` (lines, kerning, spacing, alignment, automatic stitch type), `bundled` (Lato Regular and Bold, Pacifico). `emb_model::Lettering` (`ObjectKind::Lettering`); `emb-stitch` sews the letters in order and trims gaps over 1 mm. App: Lettering tool (Digitize > Lettering) with a window for text, font and height; Object Properties for lettering; fonts of the computer found in the background; Break apart makes the parts plain objects. Corpus design `lettering-names` (25 mm and 10 mm names) with a golden snapshot. In Lato Bold, every glyph of A–Z, a–z and 0–9 sews as satin, and the strokes cover 86 % to 100 % of each glyph. Not yet tried by hand: the Lettering window and Object Properties (the test computer blocks clicks); sew-outs open. Known: a small peak above the apex of "A". |
| 2 | Done in software | `emb_letter::embfont`: the `.embfont` format (JSON: letters as satin and fill parts at a 25 mm design size, kerning pairs, line height, space, size range, notes), `convert` (200 characters: ASCII, Latin-1, common marks). `emb_letter::source`: `LetterSource` for TrueType and embroidery fonts, `size_range` (satin columns 1.2 to 10 mm; at least 5 mm). `FontRef::kind`, `Lettering::range_mm` and the size warning in the design checks. Starter library in `data/fonts/embroidery/`: Block Sans (from Lato Bold), Anton, Pacifico, Varsity (from Graduate), built with `emb-cli font`; Anton and Graduate also bundled as TrueType. App: embroidery fonts first in the font lists (bundled and the user's `fonts` folder); Fonts window (Digitize > Lettering > Fonts) with search, kind filter, preview, size range and notes, "Use for the selected lettering" and Convert to embroidery font (in the background). New lettering uses Block Sans. The stroke method keeps short thick arms (Anton "T"), so more letters sew complete. Corpus design `embroidery-fonts`. |
| 3 | Done in software | `emb_model::Baseline` (straight, arc, circle, path; letters move as rigid shapes onto it) and `emb_model::LetterEdit` (kerning, move, size, turn, shape, kept by character index; `retarget_edits` keeps them through text changes). Each letter keeps its frame, so a letter with changed nodes keeps its shape (`keep_shape`), scaled with the height. `emb_letter::layout` applies the baseline, manual kerning and the edits. App: Reshape tool on lettering (`letter_edit`): diamonds to move letters (Shift: kern), turn and size handles on the selected letter, its nodes to reshape it, the radius handle of an arc or circle, the nodes of a line baseline. Object Properties: baseline list and radius, alignment for every lettering, the values of the selected letter, Reset letter, Reset all letters. Digitize > Lettering > Lettering on line. Corpus design `lettering-circle` (a badge: text over and under one circle, a name with changed letters). Not yet tried by hand in the app (the test computer blocks clicks). |
| 4 | Done in software | `emb_model::envelope`: 12 envelope shapes with an amount; the text box bends with an exact inverse, so moves and shapes by hand stay right. `LetterEdit::stitch` and `underlay` (a stitch type and an underlay for one letter); `Lettering::{order, connection, trim_over_mm}`. `emb_letter::layout_with_box` bends the letters after the edits. `emb-stitch`: sew orders text (as before), center out and closest join; the optimized orders search the order and direction of the parts of each letter for the fewest trims (a travel is free where the next parts cover it). App: Object Properties > Lettering art (a gallery of the envelopes, amount), Sew order (order, trim length, jumps with no trims), underlay of satin and fill, and the stitch type and underlay of the selected letter; the Reshape handles follow the envelope. Corpus design `lettering-bridge`; exit test `closest_join_saves_trims` (12 trims in closest join, 20 in text order). |
| 5 | Done in software | `emb_model::monogram`: styles, borders, ornaments, `MonogramSpec`, `MonogramTemplate` (six built in; JSON), `MonogramRecord` in `Design::monograms`; a new oval envelope. `emb_letter::monogram`: `build` (the initials as one lettering object with letter edits or an envelope for the style, ornaments of our own design, a border fitted round them) and `add_to_design` (one group, a thread for each color, a trim after each object but the last; replaces a monogram in place). App: Monogram window (Digitize > Lettering > Monogram) with initials, templates, style, font, height, border, ornaments, colors and a preview; Add, Change (with a monogram selected), Save as template (the `monograms` folder). Corpus designs `monogram` (exit test: "JMK" from "Classic in a circle") and `monogram-templates`. Also in this milestone (owner requests): double-click lettering to edit its text, Space repeats the last ribbon command, Lettering on line waits for a click on the line. |
| 6 | Done in software | `emb_model::teamnames` (columns, rows, quantities, CSV and pasted import; `Design::team_names`) and `emb_model::keyboard` (`.embkdc` collections). `emb_letter::teamnames`: the design of a row (width limit), all rows in one design in a grid, color by color. `emb_letter::keyboard`: type text with a collection (color by color), a key design from objects, the built-in shadow collection. App: Digitize > Personalize > Team names (columns, rows, show a name, import, Save each name, All in one design) and Keyboard designs (type, map the selection to a key, new collection). Exit test `twenty_names_export_in_one_step`. Corpus designs `team-names` and `keyboard-designs`. Also: a drag on the canvas is again one undo step while Object Properties shows lettering. |
| 7 | Done in software | `emb_letter::embfont`: `EmbFont::empty`, `set_letter`, `remove_letter`, `letter_from_objects` (the selection between a baseline and a capital height). `emb_letter::bitmap`: BDF and Windows `.fon` (FNT 2 and 3, raster) readers, pixel outlines, `to_embfont` (satin or fill); `emb-cli font` reads them too (checked with MS Sans Serif, MS Serif, Courier and Small Fonts). App: Digitize > Lettering > Alphabets (alphabet creator with guides, Create letter; new, rename, delete, import, export, delete a letter, letter with the alphabet). Exit test `make_a_three_letter_alphabet_and_letter_with_it`; corpus design `custom-alphabet`. |

**Exit test (2026-10-22).** Software part passed: the exit test of each milestone has an automated test, and a list of 20 team names exports as 20 machine files in one step (`twenty_names_export_in_one_step`). Sew-out part open: the files for the Topaz 20 are made with `emb-cli corpus` and `emb-cli output` from the corpus designs `sewout-towel` (towel preset, closest join) and `sewout-cap` (cap preset, center out): "Anna" at 25 mm and "Joshua" at 10 mm in Block Sans and Pacifico, with worksheets. The owner sews them and reports legibility, coverage and trims.

**Sew-out (2026-10-23).** Passed. The owner sewed `sewout-towel` and `sewout-cap` on the Topaz 20 and reported the result as excellent.

## Decisions for Milestone 7

See ADR 0021.

- **An alphabet is an embroidery font** in the user's `fonts` folder.
- **A letter from objects**: the selection between the baseline and the
  capital height of the alphabet creator (with guides on the canvas).
- **Bitmap fonts** (BDF, Windows `.fon`) become embroidery fonts: the
  pixel outline of each glyph goes through the satin and fill method of
  TrueType letters. The largest size is used.

## Decisions for Milestone 6

See ADR 0020.

- **The list is part of the design** (undo, project file); columns are
  lettering objects of the design.
- **Save each name** writes the machine files only (in the formats of the
  output profile), named after the design and the name.
- **All in one design** sews color by color over all names (the name
  drop); it opens in a new tab and can be larger than the hoop (P7 splits
  it).
- **Keyboard design collections** are our own JSON format; typed key
  designs sew color by color.

## Decisions for Milestone 5

See ADR 0019.

- **A monogram is a group of normal objects** (initials, ornaments,
  border), each in its color; the design keeps a record of the spec, so the
  window can change a selected monogram in place.
- **Styles use letter edits and envelopes** (a new oval envelope).
- **Ornaments and borders are our own shapes**, drawn in code.
- **Templates** are named specs in JSON: six built in, the user's in the
  `monograms` folder.
- **Known:** some letters of Block Sans have small notches where strokes
  meet ("M", "N"); this comes from the stroke method of Milestone 1 and
  shows in monograms too.

## Decisions for Milestone 4

See ADR 0018.

- **Envelopes bend the text box.** The top and bottom edges move; each
  point keeps its X and its relative height. The box comes from the
  letter advances and the capital height, not from the parts, so a move
  of one letter does not change it. Envelopes need a straight baseline.
- **Lettering art** is the gallery of envelope shapes in Object
  Properties, each with a small picture.
- **Text order stays the default** and sews as before. Center out and
  closest join optimize the parts of each letter: an exact search (up to
  nine parts) for the fewest trims, where a travel stitch is free when the
  parts that sew after it cover it.
- **Connections.** A gap up to `trim_over_mm` (1 mm) gets a travel stitch;
  longer gaps get a trim, or a jump for machines with no trimmer.

## Decisions for Milestone 3

See ADR 0017.

- **Rigid letters.** A baseline moves and turns each letter as one shape;
  letters do not bend. The middle of the advance of each letter is on the
  baseline.
- **Arc and circle.** An arc goes through the origin of the lettering; a
  circle has its center there, so texts over and under one circle keep
  the same center. Clockwise text stands outside the circle,
  counter-clockwise text hangs inside it. A change from a straight line to
  a curve centers the text and keeps its middle where it was.
- **Line baselines.** The user draws a line and uses Lettering on line;
  the line becomes the baseline (no longer an object). Alignment is to the
  start, middle or end of the line.
- **Edits by character index.** Kerning by hand moves the letters after
  it on the line; move, size and turn change one letter and need no font.
  A changed shape is kept in the frame of the letter and used again while
  the character at that index is the same.
- **Reshape tool.** Lettering shows a diamond for each letter; the nodes
  of one letter show only when it is selected, so the canvas stays clear.

## Decisions for Milestone 2

See ADR 0016.

- **Our own format, JSON.** An embroidery font keeps letters as normal
  satin and fill object kinds, so a later milestone can edit them by hand,
  and a converted font sews the same in each later version of the program.
  Coordinates are rounded to 0.01 mm; the four starter fonts are 0.5 to
  0.9 MB each.
- **One layout for both kinds.** A `LetterSource` gives the letters,
  advances, kerning and size range of either kind; the stitch type setting
  is for TrueType fonts only (the letters of an embroidery font keep their
  stitch types).
- **Size range from the stroke width.** The middle stroke width of a few
  plain letters ("IlHnoiu") gives the range: satin columns from 1.2 mm to
  10 mm, never below 5 mm. Automatic TrueType lettering has no maximum
  (fill takes over wide strokes). Outside the range, the design checks
  show a warning; the size stays as the user set it.
- **License names.** The starter fonts are Modified Versions under the SIL
  OFL. Lato and Graduate have Reserved Font Names, so their embroidery
  fonts are "Block Sans" and "Varsity"; each `.embfont` has an OFL text
  next to it and the source copyright in its notes. The Convert form asks
  the user to check the font license.
- **Spur test.** A branch from a line end to a junction is removed only
  if it is short and also stays within 1.5 junction half-widths (counting
  its own half-width). A corner branch does; a short thick arm does not.
  This changes the letters of TrueType lettering (the `lettering-names`
  snapshot).

## Decisions for Milestone 1

See ADR 0015.

- **A new engine crate, `emb-letter`.** It reads fonts with `ttf-parser`
  (pure Rust, no `unsafe`, MIT or Apache-2.0) and makes letter shapes. It
  depends on `emb-geom` and `emb-model`; `emb-stitch` does not need it.
- **The object keeps the letter shapes.** `ObjectKind::Lettering` has the
  text, the font name, the settings, a placement (from the lettering space
  to the design) and, for each letter, satin and fill parts as normal
  object kinds. The app makes the shapes again when the text, the font or a
  setting changes. A design with a font that the computer does not have
  still opens, shows and sews.
- **Satin letters from strokes (our own method).** The glyph is drawn into
  a grid of about 200 cells for the capital height. A distance transform
  and a thinning give the middle line of each stroke. Short side branches
  (from corners and serifs) are removed. At a junction, the two branches
  that continue most straight become one stroke. A stroke end goes to the
  end of the glyph shape. The rails are where lines across the middle line
  meet the glyph outline. A loop (as in "O") is a ring satin.
- **Automatic stitch type.** A glyph sews as satin when its widest stroke
  is at most 8 mm wide and the satin strokes cover at least 85 % of the
  glyph shape; otherwise as fill.
- **Height** is the capital height (the height of "H") of the font.
- **No shaping.** One character is one glyph (no ligatures and no
  right-to-left text) in Milestone 1.
- **Bundled fonts.** Lato (regular and bold) and Pacifico, under the SIL
  Open Font License, are in `data/fonts/` with their license texts. The
  app also lists the fonts of the computer.
