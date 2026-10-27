# ADR 0038: Cross-stitch objects

Date: 2026-10-24 · Status: Accepted (P9 Milestone 1)

## Context

Roadmap P9 adds a cross-stitch editor. The reference product has a separate
cross-stitch program for it: the user paints crosses on a grid, cell by
cell, with full, half, quarter and three-quarter crosses and backstitch
lines. The program already has a cross-stitch fill (P5, ADR 0022): a fill
object whose area becomes full crosses. That fill has no control of
single cells, no partial crosses and no backstitch.

There are two ways to keep the cells:

1. Fill objects with the cross-stitch fill, one for each region of cells.
2. A new object kind that keeps the cells of a grid.

## Decision

Option 2: a new object kind, `ObjectKind::CrossStitch`, with a
`emb_model::cross::CrossStitch`:

- **The grid** is a corner point and two cell vectors (`origin`, `u`,
  `v`). Cell (column `c`, row `r`) has its top-left corner at
  `origin + c·u + r·v`. A new grid has `u = (cell, 0)` and
  `v = (0, cell)`. A transform (move, turn, mirror, scale, skew) changes
  the corner and the vectors, so all transforms are exact and the cells
  stay in place on the grid.
- **The cells** are a list sorted by row and column, with one entry for
  each cell: full cross, half cross (`/` or `\`), quarter cross (from a
  corner to the center) or three-quarter cross (a half cross and a
  quarter cross from one of the two other corners).
- **Backstitch lines** are pairs of points on the half grid (2 steps =
  one cell), so a line can go along the cell sides, across the cells or
  to their centers.
- **One object has one thread**, as all objects. A cell has one color: a
  paint tool that puts a cell into one object takes it out of the other
  cross-stitch objects on the same grid.
- **The stitches** (`emb_stitch::cross`): all needle points are cell
  corners or cell centers. Rows of full crosses sew as a zigzag (the
  default: the two legs in turn out and back, with no travel stitches
  between the cells; the top leg changes from cell to cell), or the
  bottom legs out and the top legs back (all top legs go the same way,
  with a travel stitch along the side of each cell). A row goes out to
  its far end and back, so the partial crosses at the far end are on the
  way. Pieces join with travel stitches along cell sides that have
  crosses on one side at least; else the machine trims. Then the
  backstitch lines, joined into long lines.

The object has no paths (`ObjectKind::paths` is empty): Reshape, shaping
and offsets do not work on it. Its box and its hit test come from the
cells. Convert makes cells from fills, appliqué, satin columns and closed
vectors (each cell whose center is inside the shape).

Option 1 was rejected: a cell grid is the natural form of a cross-stitch
chart (Milestone 2 prints it as a chart), and paint and erase work on
single cells, not on outlines.

## Consequences

- The project format gets a new object type (`"type": "cross_stitch"`).
  Older versions of the app do not read a design with cross-stitch
  objects; the format version stays 1 (ADR 0005).
- The cross-stitch fill of P5 stays: it is the quick way to give a shape
  a cross-stitch look.
- The P9 commands are on a new ribbon tab, Specialty (ADR 0010). A
  right drag pans the view with these tools too; Shift+drag erases with
  the Cross stitch tool.
- A travel stitch along a cell side shows between the crosses. The
  sew-out of Milestone 1 shows if travel under the legs is necessary.

## Update 2026-10-24 (Milestone 2)

- Picture to cross stitch is a third choice of the Picture to design
  wizard (ADR 0029), not a separate window, so its background removal and
  cut-out work for cross stitch too. It makes one cross-stitch object for
  each color, on the grid of the cross-stitch tools.
- The chart is a set of `emb_render::page` pages, as the worksheets: the
  same PDF writer, and no font is needed for the symbols (they are
  drawn).
