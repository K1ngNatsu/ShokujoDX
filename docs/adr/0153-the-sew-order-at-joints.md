# ADR 0153: The sew order at joints, runs in the outline, and routes from right to left

Date: 2026-11-23 · Status: Accepted (P21 Milestone 5,
`docs/design/p21-plan.md`).

## Context

The route of a letter (ADR 0113, 0144) took the order with the fewest
trims, then the shortest travel. It did not look at the joints: in about
a third of them a column that ends in another column sewed after it, so
its end lay on top. Its travel runs went under any part that sews later,
also where a part goes out of the outline (a fan over the opening of an
"e"). And it routed each letter as if the text went from left to right:
in Arabic the needle went from the end of a letter back to the far side
of the next one, with stitches up to 11.7 mm and no trim.

## Decision

1. The route finds the joints of a letter from the shapes of its parts:
   the middle of an end of a column in another column (`route::joints`),
   not where each of two columns ends in the other (the two legs at a
   cut from a notch, ADR 0152). The cost of a route is its trims, then
   the columns that end on top of a column sewn before them
   (`Routed::on_top`), then its travel, in the search of all orders and
   in the beam search.
2. `route_with_splits` keeps a cut that gives fewer trims, or as many
   trims and fewer columns on top, and also tries a cut in the middle of
   a column with each end in another column (a stem between two serifs).
3. With the outline of the letter (`route_in`, `route_with_splits_in`;
   `Font` and the letter fault bench give it), a travel run goes only
   through cells in the outline, from points of the edges in it.
4. A letter of a script from right to left (`route::is_rtl`) is routed
   as its mirror image and mirrored back (`route_with_splits_dir`): `Font`
   by the character of the glyph, the converter by the character of the
   letter.
5. `NO_RUN` and `EDGE` do not change (at 80 mm a gap with no run is about
   1 mm).
6. The letter fault bench counts the trims, and does not count a joint
   where each column ends in the other.

## Consequences

- Columns on top (lettering in the source fonts at 25 mm): 16 to 5 in
  Block Sans, 29 to 11 in Anton, 19 to 10 in Pacifico, 85 to 51 in
  Varsity; trims 2, 2, 2 and 20 (one more in Varsity, where a run went
  out of the letter). Runs out of the outline: 0.2 mm or less in each
  font (before: up to 2.3 mm).
- The eight words of the lettering bench of P20 have 48, 49, 54, 54 and
  54 trims at 10 to 80 mm: no more than before the split.
- The longest stitch of the corpus design `shaped-text`: 11.7 mm to 6.2
  mm.
- Fewer columns on top need more trims: the plan has the other orders
  that were measured. A butt joint (an end on top that goes back to the
  edge of the column under it) was measured and is not used: the gaps
  were trims.
- The converter does not have the outline of the letters yet; P21
  Milestone 6 gives it.
