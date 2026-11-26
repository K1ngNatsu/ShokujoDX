# ADR 0171: Satin lean and bands of threads

Date: 2026-11-25 · Status: Accepted (P22 Milestone 4,
`docs/design/p22-plan.md`).

## Context

The top stitches of a satin column go square across the column, or
along the rungs. On a knit, square stitches can sink between the rows of
the knit; stitches that lean a little do not. A larger lean is a look.
A column of more than one thread across its width (a stripe of red,
white and blue, for example) was a set of columns drawn by hand, and
their sides did not meet. Both ideas are from zigzag (an open-source
program under Apache-2.0); the code is our own.

## Decision

- **Lean.** `SatinParams::lean_deg` (0 by default, from −45° to 45°,
  `object::MAX_LEAN_DEG`; a positive angle moves the end of each top
  stitch on rail B forward along the column): the angle of the top
  stitches from square. `satin::Column::leaned` makes the pairs of the column about
  0.5 mm apart along the rails (`LEAN_STEP_MM`), and moves the point of
  each pair on rail B along rail B by the width there times the tangent
  of the lean. The points on rail B increase strictly, and near an end of
  the column a moved point stops at the end of rail B: the stitches fan
  from the corner, and no needle point goes out of the column. The ends
  of the column stay. The underlay does not lean: `satin_with` leans the
  column of the top stitches only. Lettering has the setting through
  `Lettering::satin` (`apply_params` copies it to the satin parts).
  Object Properties has a Lean slider for satin, raised satin and zigzag
  columns.
- **Bands.** A column of bands is a group of satin columns with the same
  rails, each with a `ColumnBand` (`index`, `count`, `overlap_mm`) in
  `SatinParams::band`: the same plan as the tartan and gradient fills
  (ADR 0047). `SatinGeometry::band` makes the rails of a band: pairs
  about 0.5 mm apart (at the rungs, and between two rungs at the same
  part of the length of each rail), each cut at the fractions of the band
  (`ColumnBand::fractions`: equal widths, each band `overlap_mm / 2` into
  the band next to it on each side that has one), with a rung every
  fourth pair. The first band sews the underlay of the whole column; the
  other bands have no underlay (`satin_with` and
  `object::underlay_layers`). `SatinParams::banded` gives the settings of
  a band: pull compensation only out at the two sides of the column (the
  first band at rail A, the last band at rail B), and no pull in percent
  (it is a part of the width of the whole column). The stitch engine and
  the Ink/Stitch writer use it.
- **The window** Edit > Stitch Types > Bands (`column_bands.rs`): a
  thread for each band (two to six, from rail A), the overlap, Make
  bands and Remove bands; each is one undo step. Make bands keeps the
  selected column as the first band and adds the others after it in the
  sew order, in a new group; the old bands of the column go. The
  selection can be one column or all the bands of one column. Object
  Properties shows "Band 2 of 3" for a band; the Lean of a band changes
  the lean of all the bands of its column (`column_bands::bands_of`), so
  that their stitches meet; a band after the first shows no underlay
  setting.
- **Ink/Stitch.** The writer writes each band as a satin column with the
  rails of its band and the settings of `banded`. Ink/Stitch has no lean:
  a leaned column gives the warning "Lean".
- **Swatches.** `SwatchSetting::Lean` (columns, lines with a border,
  lettering) and `SwatchSetting::BandOverlap` (columns of bands). A copy
  of a group in a sheet of swatches is a new group of its own. The corpus
  design `satin-lean-bands` has columns with a lean of 0°, 8° and 15° and
  a column of three bands with an overlap of 0.3 mm; it is the start
  design of the sew-out (`emb-cli swatch` on it).

## The audit of P22

The rails of a band have at most about 20,000 pairs (`MAX_PAIRS`): a band
of a column 1.5 km long (a wrong value; the stitch engine refuses it) made
an Ink/Stitch file of 120 MB, and makes one of 0.8 MB now. A band
uses `SatinParams::active_band` and `ColumnBand::sews_underlay` in each
place that sews or writes it.

## Consequences

- The exit test in software is met: the mean angle of the top stitches
  to the square direction is the lean within 1°, with no needle point out
  of the column and no pairs that cross (`satin::pairs_cross`;
  `the_top_stitches_lean_by_the_angle`); bands with no overlap meet within
  0.1 mm, and the overlap puts two layers only in the overlap
  (`bands_cover_the_column`). The golden snapshots of the designs of
  before do not change (lean 0, no bands). The sew-out of the swatches is
  the owner's.
- A band of a column has the outline of the whole column on the canvas,
  as a layer of a tartan has the shape of the whole fill.
- The bands sew one after the other from the start of the column. Bands
  of different threads have a color change between them, so the travel
  to the start of the next band is a trim.
- Pull compensation in percent does not apply to a band.
