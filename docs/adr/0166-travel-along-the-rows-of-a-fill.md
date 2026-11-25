# ADR 0166: Travel along the rows of a fill

Date: 2026-11-25 · Status: Accepted (found in the pictures of the
tutorial "A badge in sketch mode", P19 Part 2 Milestone 5; the owner:
"look into it").

## Context

A tatami fill with a hole sews in sections (`emb_stitch::fill`). Between
two sections, the needle goes with travel stitches inside the shape
(`Sewer::go_to`, `Travel::edge_path`): the shortest path where a step
along the edge costs its length and a step across the inside costs
three times its length.

The edge of a hole and the outer edge do not join, so the travel from
one to the other must cross the inside. It went in a straight line, at
any angle to the rows. In the corpus design `sketch-badge` (a disc with
a hexagon hole), one travel run went from the outer edge to a corner of
the hexagon almost at a right angle to the rows, on top of 51 rows that
were already sewn; another went under 51 rows that sewed after it. The
3D look showed both as a ridge through the fill, and travel on top of
sewn rows also shows in a sew-out. In the 138 tatami fills of the
corpus, travel inside the fills crossed 356 rows that were sewn before
it and 364 rows that were sewn after it (the two designs whose underlay
the measure counts as travel left out).

## Decision

In a fill, the travel goes along the rows where it can.

- `Sewer::rows` is the direction of the rows. A fill sets the direction
  of its top rows for all its layers (`object::fill_runs`), so the
  travel of the underlay also lies along the top rows; `sew_tatami` sets
  its own rows when the sewer has none.
- `Travel::edge_path_rows`: a step across the inside costs
  `1 + (ACROSS_ROWS_COST − 1) × sin`, where `sin` is the sine of its
  angle to the rows and `ACROSS_ROWS_COST` is 6. A step along the rows
  lies between two rows and costs as much as a step along the edge.
- The search can go along the rows: from the start and to the end of
  the travel as far as the edge (`Travel::row_end`), and along the
  lanes (`Travel::lanes`): the lines along the rows from the nodes that
  are farthest across the rows on their part of a ring (the top and the
  bottom of a hole, where the sections split), as far as the edge. A
  lane is the line between two sections. A search takes only the lanes
  that start near the way from its start to its end (`LANE_REACH`,
  `LANE_MARGIN_MM`); far lanes make no better way and make the search
  slower.

## Consequences

- In the corpus, travel inside the tatami fills crosses 13 rows that are
  sewn before it (356 before) and 58 rows that are sewn after it (364);
  the travel across the inside is 2,135 mm (2,459 mm). The ridges of the
  sketch badge, of `fill-with-holes`, of `border-trace` and of the
  counters of the fill letters are gone in the 3D look.
- The travel is a little longer: the corpus has 0.16 % more stitches
  (40 golden snapshots change, only their stitches; no trim, jump or
  color changes). The test `travel_between_sections_goes_along_the_rows`
  checks a disc with a hexagon hole and a box with two round holes: no
  travel on top of sewn rows.
- Sewing is slower: the corpus about 10 % (0.27 s to 0.30 s), a fill of
  100 round holes about 30 % (1.0 s to 1.3 s). The plan is made on a
  thread, and the stitches of each object are kept, so only a changed
  object is sewn again.
- Other fill patterns that do not use `sew_tatami` keep the travel of
  before.
