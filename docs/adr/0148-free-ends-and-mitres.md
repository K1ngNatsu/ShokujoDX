# ADR 0148: Free ends along the outline, and mitres at corners

Date: 2026-11-23 · Status: Accepted (P21 Milestone 2,
`docs/design/p21-plan.md`).

## Context

After ADR 0147 the middle of each column is on the outline, but its ends
were made as before: a free end was extended along the middle line and its
sides were traced to the corners of the end, paired at the same parts of
their lengths; a stroke cut at a corner was extended to the edge, and the
next piece started where it came out of it. The letter fault bench showed
the faults at the ends: spikes (most of all near ends), fans of stitches,
open triangles at the feet of "A" and the arms of "k", open crescents at
round ends.

## Decision

1. **A free end** (the end of a pen stroke; `stroke::free_end`): from a
   pair of the column a little back from the end (0.8 widths, or the
   middle of a short column), the outline from its point on rail A round
   the end to its point on rail B is the end (at most `END_LOOP_MAX` widths
   long, on one ring; else the end goes into another stroke). The
   direction of the stroke is square to that pair (the middle line bends at
   the end). The points of the end farthest along it are its front: an end
   edge if they are on a straight line (`END_FLAT`, `END_STRAIGHT`), else
   the tip of a round end. The two sides go to the front, and the pairs are
   the points of the two sides at the same places along the stroke (at
   most `END_STEP` of the width apart, and at each point of the outline):
   the stitches stay across the stroke and end on the outline, on the end
   edge of a square end (the last pair is its two corners), on the slant of
   an end cut at a slant, round a round end. Where the two sides come
   nearer than `END_TIP` of the width, the column ends. A side that goes
   back against the stroke (an end that curves back) has no such end; the
   trace of before is then used.
2. **A corner** (a cut of `cut_corners`; `stroke::mitre_at`,
   `close_mitre`): the two pieces end on the mitre, the line from the inner
   corner of the outline (its nearest point on the inner side of the turn)
   to the outer corner (a corner of the outline near the line that halves
   the turn, else the point of the outline on that line). The pairs past
   the line go, and the mitre is the last pair; the pieces are not
   extended and do not start where they come out of each other. A mitre is
   only where it is clear: a turn of at most `MITRE_TURN_MAX_DEG` (110°,
   measured over a long part of the middle line), a mitre at most
   `MITRE_MAX` (1.6) widths of the thinner piece long, at least
   `MITRE_ACROSS_DEG` (25°) off the direction of each piece, in the letter
   (not across a counter), no corner of the outline farther out on the
   line that halves the turn (a sharp point), and not more than half of a
   column cut away. The side of the line that a column keeps is taken from
   a pair back from the end by the length of the mitre (the middle of a
   column that curves round, a "G", can be on the other side). Other
   corners are made as before.
3. **Junctions and crossings** do not change (the end of a column is held
   and cut back in the other column, ADR 0143).
4. **Tails of the route** (`route::TOUCH`, 0.03 to 0): a stroke is a tail
   (the letter goes on into the next letter) only if it ends at the end of
   its place in the line or after it. With the ends on the corners, the
   feet of an "A" reached the edge of its place, and its route started
   with a run from the left foot.

`tune::extend_ends` (the ends of the embroidery fonts made longer) and
`trace_side` stay: the first for Milestone 6 (the fonts are made again),
the second where a free end has no outline round it. (Milestone 6 kept
`tune::extend_ends`, with a check on the needle points, and lettering in
TrueType fonts uses it too: ADR 0154.)

## Consequences

The bench, lettering in the four source fonts at 25 mm (after Milestone 1
→ after Milestone 2):

| Font | Spikes | Out with no pull compensation | Crossing pairs | Sides that go back (mm) | Bulges (mm) | Open area (%) | Corners open |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Block Sans | 33 → 20 | 55 → 33 | 25 → 20 | 0.9 → 2.0 | 2.2 → 2.3 | 1.34 → 0.98 | 44 → 46 of 233 |
| Anton | 90 → 50 | 128 → 68 | 106 → 48 | 6.9 → 2.6 | 6.1 → 4.3 | 2.49 → 1.84 | 50 → 56 of 330 |
| Pacifico | 73 → 33 | 132 → 57 | 73 → 41 | 5.5 → 12.7 | 17.1 → 17.8 | 1.54 → 0.92 | (none) |
| Varsity | 15 → 15 | 16 → 16 | 47 → 14 | 3.2 → 0.7 | 0.0 → 0.0 | 0.89 → 0.80 | 35 → 40 of 531 |

- Square, slanted and round ends are covered to their corners and tips
  (Block Sans "T", "l", "E"; Anton "H"; the feet of "A"; Pacifico round
  ends), and the curves of "c", "S", "9" have no crossing stitches.
- The open corners are a little more: the tips of very sharp ends (under
  `END_TIP` of the width) and the joints of "x", "w", "W", where pieces
  meet at a sharp point with no mitre. The sides that go back in Pacifico
  are at the round tips (the column turns there). The bulges of Pacifico
  are the short arms that a column covers (ADR 0147).
- The lettering bench of P20: less outside the shape (mean 0.2 % to
  0.1 %), fewer characters with more than 3 % of two layers (13 to 8 at
  10 mm), more coverage.
- One stitch of 11.7 mm (a travel with no trim) is now in an Arabic letter
  of the golden design `shaped-text`: a matter of the route (Milestone 5).
