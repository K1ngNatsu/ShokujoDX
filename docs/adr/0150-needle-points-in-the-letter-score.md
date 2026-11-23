# ADR 0150: Needle points in the letter score, and the fault bench in CI

Date: 2026-11-23 · Status: Accepted (P21 Milestone 4,
`docs/design/p21-plan.md`).

## Context

The checks of the converter (the coverage of Auto, the score of a letter,
the choice of columns or fill, the tuning of the ends) measured the area
between the rails of the columns, not the stitches. A column whose end
sticks out of the letter, or whose stitches cross on the inside of a
curve, had a good score. The letter fault bench of P21 measured the needle
points, but only by hand.

## Decision

1. `emb-letter` uses `emb-stitch` (a new dependency between two engine
   crates; `emb-stitch` does not use `emb-letter`). The dependency
   direction in the spec says so.
2. The score of a letter (`quality::LetterQuality`) has two more measures
   from `quality::needle_faults`: `spikes`, the part of the needle points
   of the columns more than `SPIKE_MM` (0.2 mm) out of the outline, sewn
   with the default satin settings and no pull compensation, and
   `crossing`, the part of the pairs of the columns that cross the pair
   after them. Up to 1 % of each takes nothing (`SPIKES_FREE`,
   `CROSSING_FREE`); 10 % more spikes or 20 % more crossing pairs take all
   of the shape score (`SPIKES_ALL`, `CROSSING_ALL`). So the font score,
   the choice of columns or fill for each letter (`EmbFont::choose_fills`)
   and the size range see them.
3. The fault bench is a snapshot test (`crates/emb-cli/tests/letter_faults.rs`,
   the letters of `letter_faults::SNAPSHOT_CHARS` as lettering in the four
   source fonts; 28 s in a debug build). The open area of the bench is
   filled on one grid (`grid::fill_into`, `tune::outline_mask`): the same
   numbers, about twice as fast.

The coverage of Auto (`glyph::AUTO_MIN_COVERAGE`) and the second pass of
the stroke finder do not change: the thin bands of ADR 0147 already keep
a missing part out of the slivers.

## Consequences

- A letter with spikes or crossing stitches has a lower score than the
  same letter with none (`quality::tests`).
- The scores of the four bundled fonts with a source font in the program,
  measured again: Block Sans 88 to 84, Anton 82 to 72, Pacifico 84 to 68,
  Varsity 98 to 96 (their files have the new scores; the test
  `the_score_in_the_file_is_the_score_of_the_font` checks them). They were
  made with the stroke finder of before P21, and the new measures see
  their spikes. All are at the limit (65) or more. The other bundled
  fonts keep the scores of their files until Milestone 6 makes all fonts
  again and measures them.
- A change of the stroke finder, the ends, the route or the stitch engine
  that makes the letters worse shows in CI.
