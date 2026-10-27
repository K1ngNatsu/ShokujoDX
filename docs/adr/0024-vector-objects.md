# ADR 0024: Vector objects

Date: 2026-10-23 · Status: Accepted (P6 Milestone 1)

## Context

Roadmap P6 adds vector artwork to the canvas: shapes with no stitches,
which the user can draw, import, shape and later turn into embroidery.
The reference product uses a separate graphics mode for this, and
"vector outline" and "vector fill" stitch types for objects that keep
their shape but sew nothing. Today, imported SVG shapes always become
embroidery objects (fills and lines, or lines only).

There are three ways to keep artwork:

1. A separate artwork layer in the design (a second list).
2. A "vector" setting on the existing object kinds (as the stitch types
   of ADR 0022).
3. A new object kind in the same object list.

## Decision

Option 3: a new object kind, `ObjectKind::Vector`, with one path of one or
more subpaths (closed and open) and a `VectorStyle`: a fill color
(optional, even-odd), a stroke color (optional) and a stroke width.

- It is an object as any other: it has a place in the stack, a group, a
  lock, a selection and undo through `Document::edit` (ADR 0004). The
  Sequence panel shows it in the color block of its thread, as
  "Artwork · no stitches".
- It sews nothing. The stitch plan, the machine files, the stitch count,
  the sew time and the worksheets skip it (`Design::sewn_color_blocks`),
  so a vector between two objects of one color does not add a color
  change. The canvas draws it in its own colors, under the stitches;
  View > Vectors and Dim vectors show, hide or pale it. The vector
  commands are on their own ribbon tab, Artwork (ADR 0010).
- **Convert** changes its kind: a closed path to a fill (with its holes),
  to a satin column, or to an outline run; an open path to a line. The
  reverse conversion keeps the outline of an embroidery object as a
  vector. This takes the place of the "vector" stitch types of the reference product.
- Its colors are not threads. It keeps the `thread` field of all objects
  (its place in a color block of the Sequence panel; an import gives it
  the thread of its color). A conversion gives the new objects the thread
  nearest to the vector color (a color difference of 12 or less), or a
  new thread.

Option 1 was rejected: two lists need two sets of tools for order, group,
lock and select. Option 2 was rejected: every object kind would need a
"no stitches" path through the engine, and a fill with a vector setting
would still carry fill settings that do nothing.

## Consequences

- The project format gets a new object type (`"type": "vector"`). Older
  versions of the app do not read a design with vector objects, as for
  the object kinds of P3 to P5; the format version stays 1 (ADR 0005).
- `ObjectId` is shared: a vector and an embroidery object can be in one
  group, so a logo can keep its artwork next to its stitches.
- Shaping commands (Milestone 2) work on paths, so vectors and fills use
  the same code.
- A closed shape converted to a satin column becomes a turning column
  along its long axis (the two outline points farthest apart), with stitch
  angle lines across it every 2.5 mm. This suits bars, leaves and letter
  strokes; a branching shape needs click-to-satin (Milestone 6).
