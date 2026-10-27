# ADR 0027: Bitmap tools and vector artwork readers

Date: 2026-10-23 · Status: Accepted (P6 Milestone 5)

## Context

Roadmap P6 Milestone 5 adds tools for the backdrop picture (crop,
adjust, color reduction, thread matching) and the import of vector
artwork in PDF, PDF-based AI, EMF and WMF files. Milestones 6 to 8
(auto-trace, smart digitizing, photo stitch) work on pictures too.

## Decision

- **A new engine crate, `emb-art`**, for bitmap work: `adjust`
  (brightness, contrast, unsharp mask), `crop` (rectangle, ellipse,
  polygon; pixels outside become transparent), `reduce` (color reduction
  and thread matching). It depends on `emb-geom`, `emb-model` and `image`
  (pure Rust), not on the app. Later P6 milestones add tracing there.
- **Color reduction is deterministic** (ADR 0003): k-means in Lab color
  space on a 15-bit color histogram, started from the most common color
  and then, each time, the color farthest from the colors so far
  (weighted by its count). Then specks smaller than a size take the
  color round them, and a 3 × 3 rule cleans the edges.
- **The picture tools change the backdrop picture**, in one undo step
  each (the picture is a new PNG in the project file). There is no
  separate "finalize crop" step as in the reference product: undo takes a crop back.
- **PDF and AI through `lopdf`** (MIT, pure Rust, no default features:
  no threads, no clock). We read the drawing operators of the first page
  ourselves: paths, painting, gray/RGB/CMYK colors, the transform, form
  XObjects. Clipping is not used; text and images give warnings. AI files
  that are PDF files inside read; old PostScript AI files and EPS files
  need a PostScript interpreter, which we do not add: the message says to
  save them as PDF or SVG.
- **EMF and WMF through our own readers** of the public [MS-EMF] and
  [MS-WMF] specifications: polygons, polylines, Bezier curves, path
  brackets, rectangles, ellipses, pens and brushes, the world transform
  and the mapping modes. EMF+ records are not read; EMF+ files keep EMF
  records for such readers.
- **All artwork readers give the same result** as the SVG import
  (`emb_formats::artwork::import_artwork`): fills and running lines,
  lines only, or vector objects (ADR 0024).

## Consequences

- `lopdf` brings more pure-Rust crates (decoders, hash functions for
  encrypted PDF files). Its random numbers come from the operating
  system through `getrandom`, a normal Rust interface to the system
  (as the standard library), not a C library.
- The CDR format of a commercial graphics suite has no public specification: not planned.
- `emb-art` joins the list of engine crates that must not use FFI.
