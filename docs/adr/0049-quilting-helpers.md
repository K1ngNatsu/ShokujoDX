# ADR 0049: Quilting helpers

Date: 2026-10-25 · Status: Accepted (P11 Milestone 6)

## Context

P11 Milestone 6 asks for echo quilting round shapes, a stipple or meander
fill round shapes and appliqué, and edge-to-edge quilt blocks larger than
the hoop that sew in hoopings with joins that align. The program has
offsets (P6), stipple and meander fills (P5, P11 Milestone 4) and
multi-hooping (P7, ADR 0032). The methods are common quilting practice,
in our own words (other programs have quilting features; their code is
closed, spec rule 3).

## Decision

1. **One window**, Artwork > Quilting, with three parts; the engine work
   is in `emb_model::quilt`.
2. **Echo:** rings round the joined areas of the selected shapes (round
   corners), the first at the distance, then one spacing apart. They are
   **one running line**: each ring starts at its point nearest to the end
   of the ring before. A **border shape** is a selected fill or vector
   that holds all the other selected objects (`quilt::border_of`); the
   rings are cut at its edge, and a step from the border to the next
   piece goes along the border (or out to it first), not across the
   rings. Single or triple stitch, 3 mm.
3. **Background:** the area of the border shape, or of the box round the
   shapes made larger by a margin, without the shapes and a gap round
   them, as one fill with holes (`quilt::fill_of`): stipple or a meander
   of squares, triangles or hexagons.
4. **Quilt block:** the selected lines, copied in rows and columns at the
   size of their box plus a gap across and down, cut at the block edge;
   line ends that meet are joined (`quilt::block_lines`). Or a meander
   over the block (`emb_stitch::meander::meander_lines`, as a line). An
   optional line round the block. The selected lines are replaced. If the
   block is larger than the hoop, the window makes the hooping layout
   (`HoopingLayout::auto`) in the same undo step. The block is lines, not
   a fill: P7 cuts lines at the split lines with no overlap, so the line
   ends of two hoopings are at the same points and the joins align. (A
   fill would be cut with an overlap and each hooping would make its own
   stitches.)

## Consequences

- An echo has one straight step from each ring to the next, where the
  rings are nearest; quilters do the same by hand.
- A tile pattern joins only if its end is level with its start; with a
  gap across, each copy is its own line.
- The exit test (a 300 × 300 mm block in four hoopings with aligned
  joins, sewn) is open; a test checks in software that the corpus block
  is in four hoopings and that each line end on a split line meets a line
  end of another hooping.
