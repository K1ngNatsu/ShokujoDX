# ADR 0151: A middle line from a Voronoi diagram (a trial): the grid stays

Date: 2026-11-23 · Status: Accepted (P21 Milestone 7,
`docs/design/p21-plan.md`).

## Context

The stroke finder makes the middle line of a letter on a grid of cells
(thinning, `grid::thin`). Many rules of P21 Milestones 1 to 3 are for
faults of the grid: the line is up to one cell off the middle, its ends
bend to a corner, and side branches need rules. The Voronoi diagram of
the segments of the outline gives the exact middle line with no grid,
with the half-width at each point. The owner asked for a trial of it
before the fonts are made again (Milestone 6), with a clear rule: the new
middle line stays only if it is better in the four fonts of the letter
fault bench in spikes, crossing pairs, open area, open corners and
bulges, and the lettering bench of P20 keeps its exit tests.

## The trial

- `boostvoronoi` 0.12.1 (a port of the Voronoi diagram of Boost.Polygon
  to pure Rust; BSL-1.0; no `unsafe` code) made the diagram of the
  segments of the outline in integer coordinates (2 to the power 22
  units for the larger side of the glyph). The edges inside the outline
  (nonzero winding) were the middle line, with the exact half-width at
  each point. The graph of the stroke finder (the prune, the links, the
  chains) and all the steps after it did not change. A shape with
  segments that cross or overlap got the grid.
- Three ways at the corners of the outline: no edge from a corner; the
  edge from each corner where the outline turns 40 degrees or more (the
  grid has a side branch there); the half of that edge nearer to the
  middle line.

The numbers of the trial are in the plan. In short (spikes, lettering in
the source fonts at 25 mm, grid to the best way): Block Sans 17 to 63,
Anton 44 to 28, Pacifico 37 to 12, Varsity 15 to 24; open corners of
Varsity 33 to 67. No way was better in all four fonts, and no way kept
95 % of each Japanese character covered.

## Decision

1. The grid stays. `main` does not get the module and has no new
   dependency.
2. The code of the best way (no edge from a corner) stays on the branch
   `trial/voronoi-middle-line`: `emb_letter::medial` (the diagram to the
   graph of the stroke finder), `stroke::MiddleLine` and `strokes_with`
   (the switch), and `--middle grid|voronoi` for `emb-cli letter-faults`
   and `lettering-bench`.

## Consequences

- The middle line of the diagram is better on round strokes (Pacifico:
  fewer spikes, fewer crossing pairs, less open area; fewer Japanese
  characters with two layers). It is worse at square ends and sharp
  corners, because the prune, the free ends and the mitres were tuned on
  the middle line of the grid: thinning goes far into a square end and
  stops before a sharp corner, and the diagram does the opposite. To use
  the diagram, these steps must be made again for it; that is more work
  than a change of the middle line.
- A middle line from the diagram of each stroke alone (after a split of
  the outline into strokes) has no junctions and no side branches at its
  corners. A later trial of such a split can use the code on the branch.
