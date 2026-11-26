# ADR 0180: Stitches that run along a column

Date: 2026-11-26 · Status: Accepted (the owner: "work through the
monogram fix", the open-work row "Monograms that build up thread at
large sizes"). Changes how a satin column places its pairs of needle
points (`satin.rs` notes: "Spacing is measured along the middle") and
when it makes short stitches (P2).

## Context

ADR 0178 measured the monograms that build up thread: at 60 mm, the Oval
and Diamond styles and the large middle of Classic had places of 7 to 12
layers, also on cotton (8.9). The worst place, the foot of "A" in the
Oval style on towel (11.7 layers on 11 mm²), was in one column, not two
columns on each other.

A picture of that column showed why. The tuning of the font takes the
end of the column to a corner of the outline, so the bottom edge of the
foot is a part of one side of the column. The envelope of the style
bends that edge until it runs only 6° from the stitches. The pairs of
the column are one step apart along the middle (0.18 mm for the top
stitches), but only about 0.02 mm apart across the stitches: 30 top
stitches and 10 underlay stitches went through a square of 1.5 mm.

The same fault in a smaller size: where the stitches meet a side at a
small angle (16° at the top corner of "W"), the needle points on that
side are not near each other (0.55 mm), so the rule of short stitches
(needle points nearer than 0.25 mm) did not start, but the lines of the
stitches on that side are 0.15 mm apart.

The column fills already measure their spacing across the stitches
(`Column::across`, P5): their rungs are parallel, and an edge almost
along the stitches did not crowd them.

## Decision

1. **Pairs that the stitches do not need go** (`satin::thin_across`).
   The pairs of the top stitches (`effects::spaced_pairs`, also with the
   E stitch, square and string patterns) and of the zigzag underlay are
   made as before. Then a pair goes when the pair after it is not farther
   across the stitches from the last pair that stays than the step of the
   pairs there (the distance of the middles of the two pairs, not more
   than the step). The distance across the stitches is the larger of the
   distances from the middle of each pair to the line of the other. So
   the pairs that stay are not more than one step apart across the
   stitches. Where the pairs are more than half a step apart across the
   stitches (all columns with stitches across them, also slanted ones up
   to 60° from square), no pair goes and the stitches are the same. The
   first and the last pair stay. `Column::pairs` does not change: the
   width, the knockout, the scores and the split use it as a measure.
2. **Short stitches where the stitches crowd.** A stitch is crowded when
   its needle point is nearer than `SHORT_DIST_MM` to the line of the
   pair two before it (it was: to the needle point two before it). On the
   inside of a tight curve both are near; where the stitches meet a side
   at a small angle, only the line is near.
3. **Not done.** Pairs at a smaller distance across than along (a target
   of one step across in all columns) would change the slanted columns of
   all letters; the rule above changes only the places that crowd. A
   limit of thread that grows with the size of the letters would hide
   these places (ADR 0178).

## Consequences

- The monogram probe (`probe_monogram_warnings`: the templates with four
  sets of initials and each style at 30 and 60 mm, on cotton and towel;
  64 cases): places of build-up in 23 cases (38 before). The worst place
  8.4 layers (11.7 before). On cotton, all clean but one (EAN, Classic
  at 60 mm: 6.0). The Oval "ABC" on towel: 0 (11.7). "Block in a
  square" on towel is clean, so the test of the templates has no known
  place left.
- Still over the limit, on towel at 60 mm (heavy underlay): the top
  corner of "W" in the Classic style (8.4), and places of 6 to 7.6 layers
  in the Diamond, Oval and Classic styles. They are in
  `docs/design/open-work.md`.
- The letters: the letter fault bench and the judged letters are the
  same to the eye (480 of 496 clean, the measures agree with the eye on
  all 496). Rail points out of Anton (TrueType) 61 to 52, of Block Sans
  (TrueType) 41 to 40; open area of Anton (TrueType) 39.9 to 40.5 mm².
  The embroidery fonts at 25 mm do not change.
- The corpus: 28 designs change, with fewer stitches (team-names 7,524
  to 7,489; embroidery-fonts-9 12,355 to 12,206), and one trim fewer in
  `embroidery-fonts-3`. Travel that shows: about the same (0.1 mm more
  and less in three font designs). The picture bench: one column of
  `large-picture` loses 3 of its 198 pairs, and the plan then makes one
  more trim (4 to 5).
- The embroidery fonts are not made again for this change (as after ADR
  0163, `docs/design/open-work.md`): the converter measures the stitches
  when it chooses a split, so the next remake brings it with it.
- Tests: `stitches_along_the_column_do_not_crowd`,
  `short_stitches_where_the_stitches_meet_a_side_at_a_small_angle`,
  `the_monogram_templates_sew_with_no_warning` (no known place).
