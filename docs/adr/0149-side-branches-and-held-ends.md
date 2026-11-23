# ADR 0149: Side branches by the width of their stroke, and shorter held ends

Date: 2026-11-23 · Status: Accepted (P21 Milestone 3,
`docs/design/p21-plan.md`).

## Context

The graph of the middle lines removes short branches from line ends to
junctions (the branches to the corners of a stroke, `stroke::Graph::prune`).
It measured a branch against the half-width of its junction: where strokes
cross, the circle in the crossing is larger than the strokes, so the short
arms of a crossing (the bar of a "t") were removed, and the stem covered
them with a bulge. Branches went one at a time, so of two branches of one
junction only the first went. The end of a column at a junction held its
width for 1.5 half-widths and did not look at the outline there, so the
places where a stroke widens into another (the slab serifs of Varsity) had
no stitches.

## Decision

1. A branch is a side branch if it is short and stays near its junction,
   measured with the half-width of the strokes that it hangs on
   (`Graph::side_radius`: the middle half-width of the other edges of the
   junction, from one to three half-widths of the junction away), not the
   half-width of the junction.
2. The side branches of a junction are found first and go together; where
   all its branches are side branches, the longest stays.
3. An end at a junction or a crossing holds its width for 0.5 half-widths
   (`HOLD`, 1.5 before); farther from the end, its rails follow the
   outline, with the openings of ADR 0147.
4. The rails go round a short arm that is still a side branch only if it
   is at most `STUB_NEAR` 2.0 half-widths from the middle line (2.2
   before): the arms that are strokes now need it no more.

## Consequences

The bench, lettering in the four source fonts at 25 mm (after Milestone 2
→ after Milestone 3):

| Font | Spikes | Rail points out (ends) | Crossing pairs | Bulges (mm) | Open area (%) | Corners open |
| --- | --- | --- | --- | --- | --- | --- |
| Block Sans | 20 → 17 | 100 → 38 | 20 → 27 | 2.3 → 0.0 | 0.98 → 0.57 | 46 → 43 of 233 |
| Anton | 50 → 44 | 105 → 84 | 48 → 40 | 4.3 → 4.3 | 1.84 → 1.66 | 56 → 54 of 330 |
| Pacifico | 33 → 37 | 111 → 76 | 41 → 40 | 17.8 → 13.5 | 0.92 → 0.56 | (none) |
| Varsity | 15 → 15 | 53 → 30 | 14 → 11 | 0.0 → 0.0 | 0.80 → 0.47 | 40 → 33 of 531 |

- The bar of Anton "t" and the arms of Block Sans "E", "F" and "k" have
  no open corner. Varsity "X" and "x" have 3 and 2: small open triangles
  where the diagonals widen into the serifs.
- The lettering bench of P20: more coverage (lowest at 15 mm 95.6 % to
  96.1 %), less outside the shape (highest 0.8 % to 0.6 %), but more
  characters with more than 3 % of two layers (8 to 18 at 10 mm; the mean
  stays 1.2 %, and no character is over 4.7 %): the arms that are strokes
  and the ends that follow the outline lie a little more on the strokes
  they meet.
- `shaped-text` has two trims more (19): more parts to route.
