# ADR 0039: Rhinestone objects

Date: 2026-10-24 · Status: Accepted (P9 Milestone 3)

## Context

Roadmap P9 adds rhinestones ("bling"): stones of a few standard sizes
that a heat press fixes to the garment. The stones go on a template (a
sheet with a hole for each stone, cut on a vinyl or craft cutter); the
template is Milestone 4. A design can have stones and embroidery
(Milestone 5). The reference product has bling runs, bling fills, manual
stones, bling lettering and overlap removal.

Stones sew nothing. A stone has a size and a color; the sizes are the
usual "SS" (stone size) numbers of the trade, each with a diameter in mm.

## Decision

A new object kind, `ObjectKind::Bling`, with an `emb_model::bling::Bling`:

- **The stone**: an SS size (SS6 to SS34, with their diameters) and a
  color with a name (from a list of common stone colors, or any color).
  One object has one kind of stone. The `thread` field of the object
  keeps its place in the color blocks, as for vectors (ADR 0024); the
  stone color is not a thread.
- **The layout**: single stones (a list of centers), a run along a path,
  or a fill of a closed shape (in rows at an angle, in a honeycomb, or in
  rings that follow the outline). Runs and fills keep their path, so
  Reshape and the transforms change the shape and the stones follow; the
  stones keep their size when the shape is scaled.
- **The spacing** is the gap between the edges of two stones next to each
  other (0.5 mm by default).
- **Overlaps**: `Design::stones` places the stones of all rhinestone
  objects in sew order. Two stones overlap when their edges are nearer
  than 0.2 mm. An object with "Remove overlaps" on (the default) leaves
  out each of its stones that overlaps a stone placed before it (also a
  stone of the same object, at a sharp corner of a run). Overlaps that
  stay show red on the canvas.
- **It sews nothing.** The stitch plan, the machine files, the stitch
  count and the sew time skip it (`ObjectKind::sews` is false). The canvas
  draws each stone as a disk in its color.
- **Convert to rhinestones** makes a run along the outlines of the
  selected shapes, lines and lettering, a run along their center lines
  (one line of stones in each stroke: the usual rhinestone lettering;
  from the center line of click-to-stitch, ADR 0028), or a fill of the
  selected shapes.

## Consequences

- The project format gets a new object type (`"type": "bling"`); the
  format version stays 1 (ADR 0005).
- The template (Milestone 4) and the multi-decoration output
  (Milestone 5) use `Design::stones`, so they leave out the same stones
  as the canvas.
- The stone diameters are the usual values of the trade; a template
  adds a clearance for the template material (Milestone 4).

## Update 2026-10-24 (Milestone 4)

- The template has a hole for each stone of `Design::stones`, the stone
  diameter plus a clearance (0.3 mm by default, a user setting), and four
  registration crosses outside the box of the holes. Its coordinates have
  the design rotation but not the hoop angle: the template goes on the
  garment, not in the hoop. (The appliqué cutter files of P8 have the
  hoop angle, because the pieces go on the fabric in the hoop.)
- SVG and DXF keep each kind of stone apart (a group, a layer), so a
  cutter program can cut one size at a time or all of them.

## Update 2026-10-24 (Milestone 5)

- The registration marks can sew: a running-stitch object of four
  crosses at the end of the sew order, in a thread of its own, that the
  operator pulls out after the press. `Design::template_marks` keeps which
  object it is, and the template takes its marks from it, so the sewn
  crosses and the template crosses are the same design points also when
  the user moves them. The marks are an ordinary object (the user can see,
  move or delete it), not a hidden part of the stitch plan.
- Output design writes the template next to the machine file, and the
  worksheet gives the order of the processes. No multi-decoration machine
  format is written: none has a public description.
