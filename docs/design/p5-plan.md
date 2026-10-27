# Plan: roadmap phase P5 (decorative stitches and effects)

Date: 2026-10-23 · Status: Accepted. Milestones 1 to 7 done in software; the exit test (sew-outs) is open.

## Goal

The stitch catalog of a professional digitizer (roadmap P5): outline and
fill stitch types, motifs, patterned stitching, stitch effects and color
blends. Each stitch type and effect is a pure engine function with a
golden test. The exit test: every stitch type and effect has a corpus
design, a golden test and a sew-out photo in `tests/sewouts/`.

P4 passed its sew-out on 2026-10-23. The exit tests of P1 (sew-outs), P2
and P3 (users) are still open. They run in parallel.

Sources for what each stitch type does: the public documentation of
commercial embroidery products and common digitizing practice, in our own words
(clean-room rule). The patterns, motifs and stamps in our libraries are
our own designs.

## Order

Each milestone ends with something a user can try in the app.

| # | Scope | Exit test |
| --- | --- | --- |
| 1 | **Stitch-type framework; outline and column types.** A column type on satin columns: satin, zigzag, E stitch, raised satin, tatami, square, coil, contour. A line can have a width: it then sews as a column of one of these types along the line (outline stitch types). Sculpture run. Object Properties: outline type and column type with their settings. | Corpus design `outline-types` (each type on a line and on a column) with a golden snapshot. |
| 2 | **Curved fills.** Fill types on closed shapes: contour (rows that follow the shape between two sides), spiral, offset (concentric rings), island coil, radial (rows from a center point), maze (a double spiral that never crosses itself); effects Florentine (fill rows bend along a guide line) and liquid (even turning on satin). | Corpus design `curved-fills`. |
| 3 | **Texture fills.** Satin, zigzag, E stitch, raised satin, square, double square and coil as fill types on closed shapes; string (outline and fill), cross stitch, stipple (run, backstitch, stem stitch); laydown (a light fill under the design for towels and fur, also for a whole design). | Corpus design `texture-fills`. |
| 4 | **Motifs.** A motif library of our own designs; motif run, motif fill (rows of motifs), motif column; gradient motifs (the size changes across the fill); 3D warp (a motif fill bent like a ball); create a motif from selected objects; manage the library. | Corpus design `motifs`. |
| 5 | **Patterned stitching.** A program-split library of our own patterns; program-split fill and border; user-defined split (split lines drawn by the user); flexi split (the pattern follows a turning shape); carving stamps (stamp shapes pressed into fills, a Carving stamp panel). | Corpus design `patterns`. |
| 6 | **Stitch effects.** Auto jump (long satin stitches as jumps where the machine allows it), fractional spacing, mitre, cap and lap corners, jagged edge, trapunto, accordion spacing (preset and custom profiles), hand-stitch effect, cording, double underlay, pull compensation as a percent and for each side, satin auto-spacing. | Corpus design `effects`. |
| 7 | **Blends, buttonholes, borders.** Color blends (two colors with accordion spacing in opposite directions), multi blend (layers in a panel: add, edit, reorder), buttonholes, a library of decorative borders. | Corpus designs `blends` and `buttonholes`. |

Then the exit test (roadmap P5 exit): a sew-out sheet for each corpus
design of this phase, sewn on the Topaz 20, with photos.

## Progress

| # | State | Notes |
| --- | --- | --- |
| 1 | Done in software | `emb_model::ColumnPattern` (`SatinParams::pattern`): satin, zigzag, E stitch, raised satin (layers), tatami (stitch length), square, coil, contour (lines from the spacing); `LineBorder` (`RunningParams::border`): a line sews as a column of a set width; `RunStyle::Sculpture`. `emb_stitch::column` (one function for each type) and `running::sculpture`. App: Object Properties shows the outline type of a line (Line or a column type, with a width) and the column type of a satin column, each with its settings; Edit > Stitch types sets the type of all selected objects in one undo step. Corpus design `outline-types` (each type on a wavy line and on a tapered column) with a golden snapshot. Fixed: the size fields of Object Properties no longer record a "Resize" step for a straight line (zero height). Not yet tried by hand in the app (the test computer blocks clicks). |
| 2 | Done in software | `emb_model::FillPattern` (`FillParams::pattern`): tatami, contour, offset, spiral, island coil, radial (an optional center), maze, Florentine (a guide line); `FillParams::with_pattern`; `SatinParams::liquid`. The radial center and the guide move with the object. `emb_stitch::curves` (one function for each type: inset levels, a blended spiral, a double spiral for maze, coil rings, contour lines between two sides of the outline cut by the holes, radial lines that start where they have room, Florentine as tatami on a sheared shape) and `Column::make_liquid`. Island coil and maze have no underlay by default (it would show). App: Object Properties shows the fill type and its settings (the center of a radial fill, the guide of a Florentine fill, Liquid for satin columns); Edit > Stitch types > Fill type, and Guide line (a fill and a line: the line becomes the guide). Corpus design `curved-fills` with a golden snapshot. Also (owner requests): the hoop orientation on the production worksheet, Join (Ctrl+J), Esc closes the Lettering window. Not yet tried by hand in the app (the test computer blocks clicks). |
| 3 | Done in software | `FillPattern`: `Columns` (satin, zigzag, E stitch, raised satin and coil across a shape: the two halves of the outline are the rails, a flat edge is cut in its middle, stitches are cut at holes), `Square` (needle points in line), `DoubleSquare` (a second layer turned), `String` (string art from the edge), `CrossStitch` (a grid from the design origin; X, / or +; bold), `Stipple` (a meander round a random spanning tree with a fixed seed; run, backstitch or stem stitch), `Laydown` (one or two open layers). `ColumnPattern::String` (slanted stitches a chord length on) for columns and lines. `emb_stitch::textures` (one function for each type; `laydown_area`). App: the settings of each type in Object Properties; Digitize > Fills > Laydown puts a laydown fill under the selection (or the whole design) with a 2 mm margin, first in the sew order, in a thread of its own. Corpus design `texture-fills` with a golden snapshot; `outline-types` changed (a tenth row for the string column type). Not yet tried by hand in the app. Known: only whole cross stitch cells are sewn (no part crosses at the edge); a column fill on a shape that turns back across the angle (a C at 0°) has stitches cut at the gap. |
| 4 | Done in software | `emb_model::motif`: `Motif` (needle points from the first reference point (0, 0) to the second (1, 0)), 15 built-in motifs of our own design, `Motif::from_points`, `MotifRun`; `RunningParams::motif` (motif run), `ColumnPattern::Motif` with `SatinParams::motif` (motif column), `FillPattern::Motif` (length, height, row offset, gradient, 3D warp). The motif is kept in the object. `emb_stitch::motifs`: place a motif between two points, motif run (repeats fitted to the line), motif column (each repeat as high as the column is wide), motif fill (rows at the fill angle, cut at the edge; gradient; the ball warp `r' = R sin(π/2 · r/R)`). App: Edit > Stitch types > Motifs (the library with previews: use for the selection, create from the selection, rename, delete; `.embmotif` files in the `motifs` folder next to the settings); Motif run as an outline type; the motif settings in Object Properties. Corpus design `motifs` with a golden snapshot; `outline-types` changed (an eleventh row for the motif column, rows 16 mm apart so all fit in the hoop). Not yet tried by hand in the app. |
| 5 | Done in software | `emb_model::split`: `SplitPattern` (a tile of rows of needle points; 10 built-in patterns of our own: brick, diamond, wave, zigzag, herringbone, checker, scales, basket, ripple, grid), `Stamp` (8 built-in carving stamps), `StampPlacement`. `FillPattern::{ProgramSplit, FlexiSplit, UserSplit}`, `FillParams::stamps`, `ColumnPattern::ProgramSplit` with `SatinParams::split_pattern`. Split lines and stamps move with the fill. `emb_stitch::fill::RowSplit`: tatami rows take their needle points from a pattern (on a grid of tile widths, so objects line up), and split lines add points where rows cross them (user split: only there, with the stitch length as the longest stitch). Flexi split: program split across a shape between the two halves of its outline, and along columns. App: Object Properties (pattern and size; split lines; carving stamps with size, turn and place); Edit > Stitch types > Split lines (a fill and lines: the lines become split lines). Corpus design `patterns` with a golden snapshot; `outline-types` changed (a twelfth row for the program-split column; rows 15 mm apart). Also (owner request): right-click Edit for lettering and monograms. Not yet tried by hand in the app. |
| 6 | Done in software | `SatinParams`: `auto_jump`, `spacing_at` (fractional spacing), `auto_spacing`, `accordion` (`Accordion` profiles: five presets and custom points), `jagged_mm`, `hand_mm`, `pull_percent`, `pull_b_mm`; `LineBorder::corner` (`Corner`: mitre, cap, lap); `FillParams`: `trapunto`, `accordion`, `jagged_mm`, `hand_mm`; `RunningParams`: `hand_mm`, `cording`; `Underlay::Double`. All off by default and not written then, so earlier designs sew the same (the golden snapshots did not change). `emb_stitch::effects` (seeded random amounts, hand effect, spaced satin pairs, lap parts), auto jump in the plan (long stitches as jumps), per-side and percent pull, jagged satin and row ends, accordion rows, trapunto travel along the edge, cap corners (`SatinGeometry::from_centerline_capped`), lap corners, cording (a line, a trim and stop to lay the cord, a zigzag over it), double underlay. App: an Effects section in Object Properties of lines, columns and fills (with a custom accordion profile editor); Corners for lines with a width; Double in the underlay list. Corpus design `effects` with a golden snapshot. Not yet tried by hand in the app. Known: on VP3 a jump is part of the next move, so auto jump gives a long stitch; keep columns with auto jump narrower than the machine's longest stitch (12 mm on the Topaz). |
| 7 | Done in software | `emb_model::decor` (ADR 0023): blend profiles (density changes evenly), `BlendRecord` (`Design::blends`), `ButtonholeSpec`, `buttonhole` (running box, two satin legs with a gap for the cut, two bar tacks), `string_positions`, `Frame` with 12 built-in borders on four shapes, `frame_objects`. Accordion profiles now go from the drawn start of a column, also when it is sewn from its end. App: Digitize > Decorate > Borders (round the selection or the design), Buttonholes (one, or a string along the selected line), Blends (make a color blend; the layers of a multi blend: color, density from and to, order, add, remove). Corpus designs `blends`, `buttonholes` and `borders` with golden snapshots. Not yet tried by hand in the app. |

**Audit (2026-10-23).** The windows and panels of all milestones draw for
each corpus design with no change to the design (`emb-app/src/smoke.rs`).
The audit found and fixed one error of this kind (sliders rounded values
when they showed). Use by hand in the app is still to do.

**Exit test (2026-10-23).** Software part passed: each stitch type and
effect has a corpus design (`outline-types`, `curved-fills`,
`texture-fills`, `motifs`, `patterns`, `effects`, `blends`,
`buttonholes`, `borders`) with a golden snapshot. Sew-out part open: the
files for the Topaz 20 (VP3, worksheet, picture) are in `out/p5-sewout`
with a checklist; photos go to `tests/sewouts/`.

## Decisions for Milestone 7

See ADR 0023.

- **Decoration is plain objects** made by generators in `emb_model::decor`;
  a multi blend is a group with a record, as a monogram is.
- **Blend layers change density evenly** (a profile with 21 points).

## Decisions for Milestone 6

- **Effects are settings, off by default**, on the settings of the object
  kind they change (ADR 0022); with all effects off, the stitches are the
  same as before.
- **Random effects are reproducible**: hand stitch and jagged edges use a
  hash of the point index with a fixed seed (ADR 0003).
- **Cording on a single-needle machine is couching**: the machine sews the
  line, trims and stops; the operator lays the cord; the machine holds it
  down with an open zigzag. (Machines with a cording device: P9.)
- **Auto jump** leaves satin stitches unsplit and lets the plan sew the
  parts longer than the machine allows as jumps.

## Decisions for Milestone 5

- **One mechanism for patterned rows.** Program split, user split and
  carving stamps all change where the needle goes in along tatami rows
  (`RowSplit`). Program-split tiles sit on a grid from the design origin,
  so patterns of objects next to each other line up.
- **Flexi split is a program split in column coordinates**: measured from
  rail A along each stitch, so the pattern bends with the column (and
  with a shape filled as a column between the halves of its outline).
- **Stamps belong to the fill** (in its settings, moving with it), with a
  size, a turn and a place. The stamp lines get needle points in every
  row, and grid points near them are left out, so the line shows.

## Decisions for Milestone 4

- **A motif is one line of needle points between two reference points**,
  (0, 0) and (1, 0). Placing it is a scale and turn from those two points
  to two points on the line, so motifs follow curves chord by chord.
- **The object keeps its motif** (as lettering keeps its letters): a
  design opens and sews on a computer that does not have the motif.
- **Repeats fit the line or column exactly**: the length setting is the
  wish, and the engine takes the nearest length that ends at the end.
- **3D warp is a radial map** of the motif rows round the middle of the
  shape (`r' = R sin(π/2 · r/R)`): large in the middle, small at the edge,
  like a ball seen from the front.
- **Create motif** uses the stitches of the selected objects; the left
  and right middle of their box are the reference points.

## Decisions for Milestone 3

- **Satin and the other column types fill a shape as one column.** The
  outline is cut into two halves at its first and last points across the
  fill angle; the halves are the rails. So one setting (the fill angle)
  turns the stitches, as in a turning fill. Stitches are cut where they
  cross a hole or leave the shape.
- **Square is tatami with the needle points in line**, as the public
  help describes it (stitches of one length in straight rows). The
  toothed square stays the column type for borders.
- **Stipple is reproducible.** The random tree uses a fixed seed, so the
  same object always sews the same way (ADR 0003).
- **Laydown is a fill type and a command.** The command joins the areas
  of the objects, moves them out by 2 mm, drops the holes, and puts the
  result first in the sew order in a new thread of the fabric color.

## Decisions for Milestone 2

- **Fill types are a setting** (`FillParams::pattern`), as the column types
  are (ADR 0022). A change of type keeps the angle and the underlay, but
  island coil and maze start with no underlay.
- **Florentine is tatami on a sheared shape.** The shape is sheared so
  that the guide line becomes straight, filled with tatami rows along the
  guide, and sheared back. So the rows keep the tatami needle pattern and
  the travel stays inside the shape. The guide comes from a line that the
  user selects with the fill (Edit > Stitch types > Guide line).
- **Radial lines start where they have room.** A line starts where the
  lines next to it are one spacing apart, so the middle does not get too
  many stitches.
- **Liquid** replaces the rungs of a column with a dense list whose
  pairing is a moving mean over about two column widths.

## Decisions for Milestone 1

See ADR 0022.

- **The stitch type is a setting, not a new object kind.** A satin column
  has a column type (`SatinParams::pattern`); the geometry stays the same,
  so the user can change the type at any time. Fill types (Milestones 2
  and 3) go into `FillParams` in the same way.
- **A line with a width** (`RunningParams::border`) sews as a column of
  fixed width along the line, with its own column settings. The line
  stays a line: Reshape edits the line, and the width is a setting.
- **New settings have defaults** and are not written when they have the
  default value, so old project files open unchanged and the golden
  snapshots of earlier designs do not change.
</content>
</invoke>
<invoke name="mcp__ccd_turn__update_plan">
<parameter name="advance">true