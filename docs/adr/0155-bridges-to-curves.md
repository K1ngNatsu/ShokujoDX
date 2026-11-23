# ADR 0155: Bridges to curves, and the joints of serif fonts

Date: 2026-11-23 · Status: Accepted (P21 Milestone 9,
`docs/design/p21-plan.md`).

## Context

On the proof pictures of Friendly Serif (made from Bree Serif), the owner
found joints that were not symmetrical in "M", "N", "V", "X" and "Y", and
a large area with two layers where the stem of "n" meets its curve. The
causes:

- A slab serif on a slanted stroke meets the stroke at a sharp corner on
  one side and over a curve (a bracket) on the other side. The split of
  ADR 0152 joins two concave corners only, so one serif was a piece and
  the next was not.
- The shoulder of "n" has a sharp notch on its outside, and its inside
  curve goes into the stem with no corner. It had no bridge; the stroke
  finder made three columns on top of each other there.
- The cut from one top notch of "M" was a few percent longer than
  `NOTCH_MAX`, and the cut from the other was not.
- The fork of "Y" turns 109 degrees, just under `NOTCH_DEG`.

## Decision

1. In the split with notches (Latin letters and digits), a concave corner
   with no bridge and no cut gets a bridge to a vertex of the outline
   where the outline turns into the shape over a curve (a soft bridge,
   `split::soft_bridges`), on each side where there is one: the fork of
   "Y" gets two. It has the checks of a bridge, with the direction before
   the curve measured over `SOFT_SPAN` of the typical width, and it is at
   most `SOFT_MAX` widths long. It does not end near the end of another
   bridge (`SOFT_NEAR`).
2. Where the stroke that ends is square to the bridge, a short curve
   between two straight edges (a bracket, `split::bracket_start`) goes
   with the stroke that goes through: the stem of "n" stays straight, and
   its serif takes the bracket.
3. The cuts from notches come before the soft bridges. A cut can be
   `NOTCH_MAX` = 2.5 widths long, and it meets the outline across at
   `CUT_END_MIN_DEG` or more (a cut along a stroke is not a cut between
   two strokes: the shoulder of "n").
4. In a piece of a split, the branches of the middle line at its ends can
   be cut back up to `PIECE_PRUNE` times the usual length, if the columns
   cover the piece as well and no line across a column gets more than
   `PIECE_WIDER` times longer (a stroke that ends at a slant: the "X"; a
   serif on the two sides of a stem is not a branch).
5. A short piece of a stroke (`SIDE_PIECE`: a flag serif) does not cut
   back the extended end of another stroke: the stem goes on to the top,
   and the flag stops at the stem. An extension of a middle line stops at
   the outline (`stroke::in_outline`): the flag of "l" went out of the
   letter. A column with a rail of no length goes (`Rails::sewable`: the
   apex of Varsity "a" had one, which an Ink/Stitch file cannot keep). The
   route does not cut a closed column (the cut lost the part where the
   ring closes: a gap in the bowl of "g"), and `quality::cut_column` does
   not leave a half with a rail of no length.
6. For each glyph, a split with soft bridges is kept only if its columns
   have no more faults (at 25 mm, `glyph::split_faults`: needle points out
   of the outline, pairs across a column that cross the next pair at half
   each, open area in mm2) than the split without them. In the bowls of a
   condensed font (Anton) the pieces sew worse.

## Consequences

- Friendly Serif as lettering in Bree Serif at 25 mm, eighteen letters:
  spikes 37 to 3, open area 33.3 to 10.1 mm2, columns on top 25 to 19;
  "X" 8 to 0 spikes, "Y" 4 to 0, "n" 6 to 0.
- The letter fault bench: spikes of Block Sans, Anton, Pacifico and
  Varsity 2, 25, 27, 0 to 2, 17, 29, 0; open area less or the same in
  each font. Pacifico "k" has a bulge that it did not have (2.3 mm).
- Kana and kanji: fewer trims in the words (龍 9 to 6 at 10 mm), and at
  10 mm 9 characters (6 before) with more than 3 % in two layers.
- The exit test of the outlines (ADR 0137) does not measure parts with a
  box under 1 mm2: the small pieces of serifs have a few stitches, and no
  outline takes them (15 of 1841 parts; 90.5 % of the others).
- All the fonts are made again: the mean score of the fonts of the
  program 85.5 to 84.8 (the point of the stem at a fork is narrow at the
  smallest size); no font below the limit.
