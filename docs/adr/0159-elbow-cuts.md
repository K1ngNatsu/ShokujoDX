# ADR 0159: Cuts at elbows

Date: 2026-11-24 · Status: Accepted (P21 Milestone 13,
`docs/design/p21-plan.md`).

## Context

After Milestone 12 the owner asked to go on with the letter shapes. A new
look at the survey of Milestone 12 (the letters A-Z, a-z and 0-9 of the
127 column fonts, as lettering in their source fonts at 25 mm) gave the
largest group of faults: the letters with diagonal strokes (A, K, M, N, V,
W, X, Y, Z, k, v, w, x, y, z, 4 and 7) are 27 % of the letters, but they
have 50 % to 63 % of the needle points out of the outline, of the area
with two layers and of the open corners, and 37 % of the open area. Many
of the letters with the most open area (Archivo Black "Z" and "z",
Display Heavy "z" and "4") show one cause: an elbow, where two strokes
meet at a corner and both end there (the corners of a "Z", "7" and "4",
the top of a "P").

An elbow has one concave corner (inside) and one convex corner (outside).
The split of the outline (ADR 0152) cuts a sharp concave corner (110
degrees or more, a notch) along the line that halves it, if that line
meets the outline across at 45 degrees or more. At an elbow of a steep
diagonal, that line meets the outline near the outer corner at a small
angle, so there was no cut; an elbow of 80 to 110 degrees was not a notch.
With no cut, the stroke finder took the elbow as one shape, and its middle
line goes round the inside of the corner: the outer corner had no
stitches (an open triangle), or the two columns lay on each other.

## Decision

1. **A cut at an elbow** (`split::elbow_cut`). A concave corner with no
   bridge that turns `ELBOW_DEG` (80) degrees or more, and gets no cut
   from a notch, is an elbow if the outline has a convex corner across
   (the outer corner) at the width of each stroke from the side of the
   other stroke at the concave corner (within `ELBOW_FIT`, 25 %). The
   widths are measured square to each side, `ELBOW_SPAN` (half the
   typical width) from the corner. This check keeps the cut out of the
   corners of serifs: the corner of a slab serif is farther from the side
   of the stem than the width of the stem.
2. **A butt joint, not a mitre.** The cut goes on along one side at the
   concave corner to the outline across (the shorter of the two), so one
   stroke goes on to the outer corner and the other ends square to its
   side. A mitre (a cut from the concave corner to the outer corner) was
   tried first: its pieces have pointed ends (half the angle of the
   corner), and satin stitches go out of them (4,275 needle points out of
   the outline in the survey, 1,107 before).
3. **Kept only if the columns sew no worse.** The cuts at elbows are
   optional links, as the bridges to curves and the cuts of balls (ADR
   0155, 0157): a glyph keeps them only if its columns have no more faults
   (`glyph::split_faults`) than without them (`stroke::Optional`). At a
   short hook at the end of a stroke (the arms of Black Ops One "C") the
   cut makes a piece that is near square, and its column does not cover
   it. Without this check, the survey had 45 mm2 more open area (Candy
   Sans "5" 12.7 mm2, Black Ops One "C" 10 mm2) and 28 more needle points
   out of the outline (2 fewer trims).
4. Tried and left out: a notch from 100 degrees (the notches of the "W"
   of Slab Heavy turn 104 to 107 degrees). Open area 18 mm2 less, but 108
   more needle points out of the outline and 5 more trims.
5. Kana and kanji do not change: their split has no notches.
6. **Dense columns in a stitch file are columns** (`emb_art::parts`, ADR
   0131). The exit test of the outlines of P18 Milestone 7 (ADR 0137) fell
   from 90.4 % to 89.0 % with the cuts: the arms of the letters at 10 mm
   are columns of their own, and their stitches were found as runs. The
   cause was in the parts: two legs back on nearly the same line (0.12
   mm) were a bean stitch. In a narrow column of a small letter (needle
   points 0.18 mm apart on a side, with short stitches) each stitch goes
   back on nearly the same line too. A bean stitch goes back into its
   needle holes: now a sequence is a bean stitch only if at least 0.3 of
   its legs (after the first two) end in a needle hole of an earlier leg
   (`BEAN_HOLES`). A
   first try (the middles of the legs on one line) made a bean stitch
   along a curve a column.

## Consequences

- The survey (7,874 letters): open area 3,581 to 3,158 mm2 (letters with
  more than 0.5 mm2 open: 1,502 to 1,394); area with two layers 54,792 to
  50,609 mm2; crossing pairs 806 to 750; columns on top 1,238 to 1,175;
  open corners 1,951 to 1,882; trims 783 to 780; needle points out of the
  outline 1,107 to 1,114. Fans 391 to 449 mm: the bowls of a "B" or a "P"
  that end square on the stem turn at their outer corners.
- The open area of the letters "Z", "z", "W", "B", "e", "w", "F", "4",
  "D", "M", "7" and "G" changed most: Archivo Black "Z" 9.5 to 1.8 mm2,
  Display Heavy "z" 12.8 to 0.8 mm2 and "4" 15.6 to 1.1 mm2.
- The letter fault bench (embroidery font / lettering): crossing pairs
  Block Sans 3 / 4 to 1 / 1, Anton 8 / 13 to 7 / 10; columns on top Anton
  10 / 8 to 6 / 5, Varsity 63 / 59 to 52 / 52; fans 0.3 to 0.5 mm more in
  each font; the open area of Varsity 0.06 % / 0.03 % to 0.07 % /
  0.04 %; Pacifico the same.
- The fonts: 33,949 to 33,937 trims in the letters; the column fonts of
  the program 94.2 to 94.5 on mean. Fat Face Serif (a font of a package)
  90 to 79: its typical stroke is wider, so the converter starts its size
  range at 29.5 mm (36.5 mm before), where 6.4 % of the length of its
  columns is too narrow (the size range allows 10 %, `RANGE_TAIL`). Its
  letters are better (more cover, less area with two layers, 5 fewer
  trims), and it stays in its package.
- The tests of the parts of P18 (the corpus as DST and back): the
  outlines 90.4 % to 92.6 % of the objects of their kind; the stitches in
  parts of their kind 96.8 % to 97.2 %; the stitches of columns in a satin
  94.0 % to 94.9 %; the runs with the stitch length and the bean setting
  of their source 77.6 % of 125 (the same). The measured columns with the
  width and the spacing of their source 95.7 % to 93.5 % (the limit is 90
  %): with the cuts alone 94.7 % (more columns meet at corners in the
  design with Mitre Corners), and the dense columns that were runs before
  are measured now, and their width is often not found well (the short
  stitches).
- The golden designs with lettering: the same trims; stitches 0.4 % to
  4.1 % more (each new piece has its underlay), and the design with Mitre
  Corners 7.5 % more.
- Not changed: an elbow whose outer corner is cut by a short edge that is
  not a side of a stroke (Fat Face Serif "7"), and an elbow that turns
  less than 80 degrees (the flag of Candy Sans "F"). The slab letters "L"
  and "I" keep the columns of before (the check of decision 3).
- The audit of P21 (2026-11-24): the rule for a bean stitch finds the
  needle holes in a grid with cells of 0.06 mm. It compared each leg with
  all the legs before it: a bean stitch of 100,000 stitches took 7 s
  (0.07 s now). The parts are the same.
