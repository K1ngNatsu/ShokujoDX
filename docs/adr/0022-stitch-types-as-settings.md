# ADR 0022: Stitch types as settings

Date: 2026-10-23 · Status: Accepted

## Context

Roadmap P5 adds about forty stitch types and effects: outline types
(sculpture run, E stitch, coil and others), fill types (contour, spiral,
motif fill and others) and effects. Each object kind (`Running`, `Satin`,
`Fill`) holds its geometry and its parameters in one enum variant, so an
object cannot have a geometry that does not match its stitch type
(ADR 0004, `emb_model::object`).

Two models are possible: a new object kind for each stitch type, or a
stitch-type setting on the kinds that exist. In the reference product the user
changes the stitch type of a selected object at any time, and the outline
stays the same.

## Decision

- **A stitch type is a setting of the object kind that has the matching
  geometry.** A satin column has a column type, `SatinParams::pattern`
  (`ColumnPattern`): satin, zigzag, E stitch, raised satin, tatami, square,
  coil and contour. Fill types go into `FillParams::pattern`
  (`FillPattern`) in the same way (Milestone 2: contour, offset, spiral,
  island coil, radial, maze, Florentine; Milestone 3: the column types
  across a shape, square, double square, string, cross stitch, stipple,
  laydown; Milestone 4: motif run, motif column and motif fill, with the
  motif kept in the settings; Milestone 5: program split, flexi split, user
  split and carving stamps, with the pattern and stamps in the settings;
  Milestone 6: stitch effects, off by default).
  Settings that are points (the
  center of a radial fill, the guide of a Florentine fill) are in design
  coordinates and move with the object. Each type has its own settings in its enum
  variant (for example the layers of raised satin); `spacing_mm` is the
  repeat distance of the type.
- **A line can have a width.** `RunningParams::border` (`LineBorder`: a
  width and the column settings) makes the line sew as a column of fixed
  width along it, of any column type. These are the outline types of
  the reference product with a width (satin, E stitch, coil, ...). The line
  stays a line: Reshape edits the line, and the width is a setting. The
  engine makes the column with `SatinGeometry::from_centerline` for each
  stitch, so the column follows each change of the line.
- **Sculpture run is a run style** (`RunStyle::Sculpture`): triple and
  single stitches in turn. It is applied after the plan chose the
  direction of the line, as backstitch and stem stitch are.
- **New settings are not written when they have the default value**
  (`skip_serializing_if`), so old project files open unchanged and the
  golden snapshots of earlier designs stay the same.
- **One function for each type** in `emb-stitch` (`column::top`), a pure
  function of the column and the settings, with unit tests and a corpus
  design (`outline-types`) with a golden snapshot.
- In the app, Object Properties shows the outline type of a line and the
  column type of a column, with the settings of the type. Edit > Stitch
  types sets the type of all selected objects in one undo step.

## Consequences

- Changing a stitch type never changes the geometry, and undo restores
  the old type and its settings.
- A change of type resets the settings of the type to its defaults (pull
  compensation and split length stay).
- Types that need data outside the object (motifs, program-split
  patterns, carving stamps) keep a reference to a library item in their
  settings (Milestones 4 and 5).
