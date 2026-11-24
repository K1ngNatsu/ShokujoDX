# ADR 0156: The work areas of the font packages

Date: 2026-11-23 · Status: Accepted (P21 Milestone 10,
`docs/design/p21-plan.md`). ADR 0157 found the cause of the faults at
balls and at square dots in the stroke finder, and removed five of the
corrections of decision 3.

## Context

After Milestone 9, the owner asked which fonts of the font packages are
ready to go in the program. Eighteen fonts of the packages had a score
of 85 (`quality::BUNDLE`) or more. On their proof pictures, ten were
clean and eight had faults in a few letters. The faults were of four
kinds, and the owner asked to fix the four work areas in sequence:

1. **Crossings of strokes of the same width.** In a narrow "X" (Khand,
   Compact Heavy) the sides of the crossing are about 1.6 widths long,
   longer than `split::BRIDGE_MAX` (1.3), so the crossing had no bridge,
   and the crossing was in short pieces.
2. **Balls at the ends of strokes** (the terminals of "S", "9" and "r" in
   Bookish Slab): the column of the stroke stopped before the ball, and
   most of the ball was open.
3. **Single letters** with a gap, a spike or a joint that the stroke
   finder does not make well (Sturdy Serif "k" and "3", Rubik Mono One
   "A" and "3", Fat Face Serif "u" and "y", Compact Heavy "X", Jazz Hand
   "t" and "w", Angular Sans "r" and "k"), and square dots: the stroke
   finder gave the dots of Jazz Hand "i", "j" and ":" ends that go out of
   the dot (16 needle points out of the outline in each "i" and "j").
4. **The score.** The narrow measure counted the pointed end of a column
   (a column cut at the fork of a "Y" or at a crossing, the point of a
   serif) as too narrow at the smallest size. In most fonts this was most
   of the narrow measure: a sans font lost about 10 points for columns
   that come to a point as they must.

## Decision

1. Longer bridges between two concave corners in Latin letters (2.0
   widths) were tried. Khand "X" and "x" got one whole stroke and two
   arms (8 to 6 parts, 2 to 0 spikes in "x"), but the arms of "x" fan
   from a corner, and the fonts with slab serifs and some scripts got
   more trims (Zilla Slab "X" 0 to 2 at each size, Geo Slab "x" 0 to 3,
   Sacramento "K" 1 to 3; 15 more in all the fonts). A trim costs more
   than the small gain, so the longer bridges are not in the program.
   A width of a column measured square to the column (for the check of
   the pieces) changed no letter that was measured, so it is not in the
   program either. The crossings of the fonts of the packages are as
   before, and Compact Heavy "X" gets a correction (decision 3).
   The trial found a fault of the route: a run keeps `route::EDGE` from
   the edge of each column, so where two columns that are not sewn yet
   only touch (the two sides of a bridge), there was no way across, and
   the letter got a trim. Now a cell near the edge of a column is a way
   if the columns that are not sewn cover all round it
   (`Router::passable`).
2. A step of the tuning that gave a ball a column of its own was tried
   (the largest circle in the outline near the end of a column). It did
   not find the balls well (a column at the joint of the bowl and the
   stem of "9", a ball that the checks refused in "r"), so it is not in
   the program. The balls get letter corrections.
3. The letters of work areas 2 and 3 get letter corrections (ADR 0112):
   each column has the sides of the outline as its rails, a ball has a
   column of its own from the notch where the stroke meets it, and a
   long curved column has rungs square to it. A correction is written as
   the routed letter (the route of `route::route_with_splits_in` with the
   outline of the source font), so the font has the letter of the file,
   and a font of a package can have corrections too:
   `packaging::the_font_packages_have_their_corrections` reads the
   packages, and `Corrections::missing_in` is the check of the fonts of
   the program and of the packages.
4. The narrow measure of the score leaves out the pointed ends of the
   columns: at each end of a column, the measures that are too narrow
   are a pointed end if they are at most `TIP_WIDTHS` (1.0) middle widths
   of the column long (`quality::tips`). A column that is narrow from end
   to end, or narrow in its middle, still counts. The size range of a
   font (`EmbFont::fit_size_range`) does not change: it keeps a smallest
   size at which the pointed ends are not too narrow.
5. The limits of the score stay (`LIMIT` 65, `BUNDLE` 85), and each font
   keeps its place (ADR 0116). The scores of most fonts are higher, so a
   score of 85 is easier to get than before; a font goes from a package to
   the program only when the owner chooses it after a look at its proof
   pictures.

## Consequences

- The corrected letters at 25 mm (needle points out of the outline,
  crossing pairs, open area in mm2), before and after: Sturdy Serif "k"
  open 31.2 to 2.9, "3" 24.8 to 1.3; Rubik Mono One "A" 2, 11, 11.4 to
  0, 0, 0.2, "3" 25, 8, 2.9 to 0, 0, 0.4 (13 parts to 4); Fat Face Serif
  "u" 0, 7, 18.8 to 0, 1, 0.3, "y" 0, 16, 12.2 to 0, 6, 0.1; Compact
  Heavy "X" 8 to 5 parts; Bookish Slab "S" 4, 15, 4.0 to 0, 0, 0.2, "9"
  4, 17, 5.0 to 0, 2, 0.3, "r" open 6.6 to 0.4; Jazz Hand "i" and "j" 16
  to 0 spikes, "t" 2, 10, 3.9 to 0, 0, 0.1, ":" 5, 14, 7.4 to 0, 0,
  0.1, "w" open 1.3 to 0.2; Angular Sans "r" 6, 6, 2.4 to 0, 0, 0.0,
  "k" open 5.7 to 0.1.
- The route: 33,966 trims in the letters of all 129 fonts (34,232
  before; Zen Maru Gothic and Noto Sans JP 208 fewer, Varsity 9 fewer,
  Barlow 1 more). The letter fault bench: the same, but Anton 3 to 2
  trims and 22 to 20 crossing pairs, and Varsity 18 to 15 trims with 55
  to 63 column ends on top (the route puts fewer trims first).
- The scores: the 55 column fonts of the program 84.8 to 94.2 on mean
  (none lower; the lowest is Bold Script, 76); the 72 fonts of the
  packages 80.0 to 90.0 on mean, 18 to 58 with a score of 85 or more,
  none lower. The fill fonts do not change.
- The square dots and the balls are corrected by hand; the causes in the
  stroke finder stay open (`docs/design/open-work.md`).
