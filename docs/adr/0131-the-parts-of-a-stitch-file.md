# ADR 0131: The parts of a stitch file

Date: 2026-11-19 · Status: Accepted (P18 Milestone 1,
`docs/design/p18-plan.md`)

## Context

A machine file of another program opens as stitch objects: they have no
shape and no settings. P18 makes objects with stitch types from them in
steps. The first step (Milestone 1) cuts the stitches into parts of one
kind (runs, columns, rows of a fill, other) and finds the underlay of each
part. Milestones 2 and 3 make running, satin and fill objects of the
parts. The native formats of closed-source programs stay out of scope
(rule 3); the test files are the designs of our corpus.

## Decision

- **The engine** is `emb_art::parts` (next to Recognize Outlines, ADR
  0030; not a port). `parts(&StitchDesign)` gives the parts in sew order,
  each with its kind, its stitch records, its thread and the part that it
  is the underlay of.
  1. **Chains**: the stitches between trims, jumps, stops and color
     changes (a stitch longer than 12.5 mm is a jump too).
  2. **Legs**: in a chain, the stitches that go on within 30° of each
     other. A stitch shorter than 0.35 mm (a lock stitch) has no direction.
  3. **Back and forth**: four legs or more in which each goes back along
     the one before it (a cosine below −0.6) are **rows** or a **column**.
     A short turn (one or two legs, at most 4 mm and half the shorter row,
     at least 1 mm) can be between two legs; the turn is tried first,
     because at the end of a row at an angle to the edge the turn stitch
     itself goes back a little. **Rows** have a turn between most legs (a
     stitch along the edge) or three stitches and more in a leg, and their
     legs are within 12° of their mean angle (85 % of their length); else
     the legs are a **column** (a satin goes back from the needle point
     where it came; a wide satin crosses in two or three stitches). Legs
     that go back to where they started (a bean stitch) are a run.
  4. The other legs are a **run**. A run of at most two stitches between
     two parts of one kind goes with them; tiny stitches go with the part
     before them.
  5. **Underlay**: from the last part to the first, a map keeps for each
     cell (0.25 mm) the next column or rows of the same thread over it. A
     part that the later parts cover in 85 % of its cells goes with the
     part that covers most of them: a run always (an underlay run, or the
     travel under a part), a column or rows if it has less than 0.8 of the
     thread in a cell of that part (a zigzag or a fill underlay).
  `shown_kind` gives the kind that the stitches of a part show (the kind of
  the part that it is the underlay of), `kinds_of_records` the kind of each
  record. A stitch is never lost: each stitch of a chain is in a part.
- **The view**: View > Stitch Parts (`emb-app/src/parts_view.rs`) colors
  the stitches of the plan that shows by the kind of their part (blue
  runs, orange columns, green rows, purple other; the underlay paler) with
  a key; a thread finds the parts of each new plan.
- **The exit test** (`crates/emb-cli/tests/parts.rs`): each corpus design,
  with its lettering cut into its letter parts (as Break Apart), is
  written as DST and read again; each stitch of the file is matched to the
  stitch of the plan at its place, and the kind of its source object
  (running, satin and zigzag columns and borders, fills of rows and of
  satin columns) is compared with the kind of its part.

## Consequences

- 97.1 % of the 479,588 stitches of the objects of known kinds are in a
  part of their kind (columns 98.7 %, rows 97.6 %, runs 67.9 %). The runs
  that count as wrong are mostly the travel runs in routed letters and the
  bar runs of buttonholes: they sew under a later column, and the parts
  put them with it, as the plan asks for underlay.
- 113,304 stitches are of objects of other kinds (motif fills and runs,
  contour, spiral and other fills, cross stitch, appliqué): the parts do
  not know them; they are runs, columns or rows by their stitches.
- The parts of 62,000 stitches take 0.17 s (most of it the underlay).
- Open (Milestones 2 and 3): objects from the parts.
