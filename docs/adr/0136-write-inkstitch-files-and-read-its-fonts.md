# ADR 0136: Write Ink/Stitch files and read its fonts

Date: 2026-11-20 · Status: Accepted (P18 Milestone 6,
`docs/design/p18-plan.md`)

## Context

Milestone 5 (ADR 0135) reads the SVG files of Ink/Stitch. Milestone 6 is
the other direction: a design of the program as a file that Ink/Stitch
opens with the same objects, and the lettering fonts of Ink/Stitch
(digitized by persons) as embroidery fonts of the program.

## Decision

- **Writer**: `emb_formats::inkstitch::write::write_inkstitch(design,
  underlay_of)` gives the SVG text and the notes. One unit of the file is
  one mm (`viewBox` in mm), the design is on a page with a margin, and the
  objects are in sew order in one layer. The names and the meanings of the
  settings are those of the reader (ADR 0135).
  - Fills: the fill types of Ink/Stitch (tatami, square as one stagger,
    contour with its strategy, guided with its guide lines in a group with
    the guide marker, circular with a target point command, meander, cross
    stitch, a gradient group as one shape with a linear gradient paint);
    the other types are tatami, with a note. Angle (the other way round),
    row spacing, stitch length, pull compensation, an accordion of two
    points as the end row spacing, and the underlay rows (angles, the
    spacing and inset of the first layer).
  - Satin columns: the rails and the rungs (each rung goes out past the
    rails). A column with no rung gets one in the middle, and a column with
    two gets a third in the middle of its first section: with no rung and
    the same number of nodes, Ink/Stitch pairs the nodes, and with two
    rungs it takes the two longest subpaths as the rails. A rung in the
    middle of a section does not change the pairing. Spacing, pull
    compensation for each rail, short stitches, split length,
    `reverse_rails` none, and the underlay as center walk (two walks for
    each layer), contour and zigzag. E stitch and zigzag are satin methods;
    the other column types are satin, with a note.
  - Lines: run (stitch length, bean stitches, two passes), zigzag (stroke
    width and spacing), a satin or E stitch border wider than 1 mm as a
    satin column of the stroke width along each subpath, ripple (line
    count, target point, guide line), manual stitches, cut lines.
  - Stitch objects are manual stitches, one path for the subpaths between
    two trims. Lettering is the parts of its letters in a group. Appliqué
    is the placement lines (stop), the tack-down lines (stop) and a satin
    column of the cover width along each edge. Vector objects have
    `ignore_object`. Cross stitch and rhinestones are not written (a note);
    sketches sew nothing.
  - Trims, stops and lock stitches are the settings `trim_after`,
    `stop_after` and `ties`; start and end points are commands with
    symbols and connectors. When all threads are of one brand that is a
    chart of the program, the file names it as its thread palette.
  - The engine gives the underlay layers that an object sews:
    `emb_stitch::object::underlay_layers` (the Auto, Light and Heavy
    settings depend on the shape and the width). The writer takes it as
    an argument, because `emb-formats` does not depend on `emb-stitch`.
- **Commands**: Design > Output > **Export for Ink/Stitch** (also in the
  File menu; `Cmd::ExportInkstitch`, `AppState::export_inkstitch_to`) and
  `emb-cli inkstitch <design> <out.svg>`.
- **Fonts**: `emb_letter::inkstitch::convert_inkstitch_font(dir)` reads a
  font folder of Ink/Stitch (`font.json`, `LICENSE`, `ltr.svg` or `→.svg`
  or a folder of them) with `emb_formats::inkstitch::import_inkstitch_glyphs`
  (the objects of each glyph layer `GlyphLayer-<name>`, the hidden layers
  too, and the baseline guide). Each glyph with a name of one character is
  a letter: its parts in the order of the font (`fixed`), placed at x 0 on
  the baseline, with the underlay, the pull compensation and the fill angle
  of each part as its tuning (`PartTuning`, against the default settings
  of lettering). The advance is `horiz_adv_x`, else
  `horiz_adv_x_default`, else the right edge; the kerning of Ink/Stitch is
  subtracted, so it changes sign; leading, word spacing and the scale
  range give the line height, the space and the size range (a range of
  one size is 0.8 to 1.25 of the capital height). An upper-case or
  lower-case font gets the letters of the other case. The license name
  and the first paragraph of `LICENSE` go in the notes. The baseline guide
  is measured from the top or from the bottom of the page in different
  versions of Inkscape: the converter takes the one where most glyphs
  stand. `EmbFont::from_letters` makes the font. `emb-letter` now uses
  `emb-formats` (the reader of Ink/Stitch files).
- **Commands for fonts**: Lettering > Fonts > **Ink/Stitch Font…** (choose
  the folder; the conversion runs in a thread, and a note shows the
  license and what the converter could not keep) and
  `emb-cli font-inkstitch <folder> <out.embfont>`.
- **Sources (rule 4)**: the file names, the keys of `font.json` and the
  layout rules are from the source of Ink/Stitch, commit 42e771e9
  (`lib/lettering/font.py`, `font_variant.py`, `glyph.py`).
- **No bundled fonts yet**: the fonts of Ink/Stitch are in the repository
  `inkstitch/embroidery-fonts`, each with its own license (for example
  Cats: SIL OFL 1.1; DejaVu Serif and Abécédaire AGS: CC BY-SA 4.0). The
  owner chose a test only (2026-11-20): the converter was tried on Cats,
  and nothing of it is in the repository. A font can be bundled later only
  with a license that permits it, with its license text and its source.

## Consequences

- The automated parts of the exit test (`crates/emb-cli/tests/inkstitch.rs`):
  each of the 85 corpus designs, written for Ink/Stitch and read again,
  gives the same objects, kinds and colors in the same order (2,768
  objects); fills and columns keep their angle, spacing, pull compensation
  and underlay; an appliqué gives its three steps with two stops. The font
  Cats (115 letters) converts, and its test sheet sews at three sizes. The
  exit test of the owner (a written design opened in Ink/Stitch, a
  converted font sewn) is open.
- Ink/Stitch has settings that the program does not have (random widths
  and splits, push compensation) and the program has settings that
  Ink/Stitch does not have (liquid, accordion, puff, other column and fill
  types): the notes of the writer and of the reader list them.
