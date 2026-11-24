# ADR 0157: Column ends on the outline, and balls at the ends of strokes

Date: 2026-11-23 · Status: Accepted (P21 Milestone 11,
`docs/design/p21-plan.md`).

## Context

Milestone 10 (ADR 0156) left three items of the letter shapes open, and
the owner asked to finish them:

1. The stroke finder gave the square dots of Jazz Hand "i" and "j" ends
   that went 2.6 mm out of the dot (16 needle points out of the outline
   in each letter). The letters had corrections; the cause was open.
2. A ball at the end of a stroke (the terminals of Bookish Slab "S", "9"
   and "r") was not found: a step of the tuning that looked for the
   largest circle near the end of a column did not find the balls well.
   The letters had corrections.
3. The size range of a font counts the pointed ends of the columns as
   narrow; the score does not (ADR 0156).

The cause of item 1: the middle line of a square dot is a diagonal, and
the walk on the grid ended it a part of a cell past the corner, out of
the dot. A rail of a point is stopped where a ray from the point meets
the outline, but a ray from a point out of the outline, or on it, meets
nothing there, so the rail took the full width of the stroke out of the
letter. The same fault was at the ends of many columns: it made most of
the needle points out of the outline in the letter fault bench (Pacifico
"u" and "y", Anton "e", "g" and "h").

## Decision

1. The middle line of a column ends on the outline: the points out of
   the outline at each end go, and the place where the line leaves the
   outline is its end (`stroke::clip_ends`). The width of a side at a
   point is measured with `stroke::room`: a side that goes out of the
   outline at once (a point on the outline) has no width there, and a
   ray from such a point that meets the next part of the letter (the stem
   under a dot) is not used.
2. A ball is cut from its stroke in the split of a Latin letter
   (`split::ball_cut`): at a concave corner that turns `BALL_MIN_DEG`
   (70) degrees or more but is not a notch, a cut square to the side of
   the stroke, across it, at most `BALL_CUT_MAX` (1.6) widths long, if
   the outline the other way round the corner to the cut is round
   (`BALL_ROUND`: 4 pi area over the square of its outline at least
   0.7), wider than the stroke (`BALL_WIDER`) and at most `BALL_PART`
   (0.35) of the glyph. The cut is a soft link: the glyph keeps it only
   if its columns sew no worse (ADR 0155, decision 6).
3. The column of a short round piece (a dot, or a ball cut from its
   stroke: `stroke::long_axis`) has free ends, so its rails go round to
   the edge of the piece. The column of a ball goes square to its cut,
   away from the stroke (`split::Piece::axis`), so its stitches lie as
   the last stitches of the stroke (the owner's request: the stitches
   of the ball near parallel to the column next to it), if it covers the
   ball within `BALL_COVER_LOSS` (3 %) of the ball's own column. A ball
   that hangs to a side of its stroke (Bookish Slab "r": 11 % less)
   keeps its own column.
4. The corrections of Jazz Hand "i" and "j" and of Bookish Slab "S", "9"
   and "r" go: the letters of the program are as good. The colon of Jazz
   Hand keeps its correction (its dots have the shape of a "D": the
   middle line goes to a corner, a different fault).
5. The size range keeps the pointed ends. Measured on the fonts of the
   program: without them, the smallest size of 15 fonts would be 0.5 to
   3 mm smaller (Pangolin 11 to 8 mm), where 2 to 9 % of the length of
   their columns is too narrow, and their scores would fall by about 5 to
   15 points. In most other fonts the smallest size comes from the
   converter, not from this measure. The pointed ends are a margin at the
   smallest size.

## Consequences

- The letter fault bench (the embroidery fonts and lettering in their
  source fonts, at 25 mm): needle points out of the outline Anton 16 and
  17 to 3 and 4, Pacifico 28 and 29 to 3 and 3, Block Sans and Varsity
  as before; needle points with no pull compensation Anton 24 to 10,
  Pacifico 47 to 9, Varsity 2 to 0; crossing pairs fewer in Anton and
  Pacifico, Varsity 4 and 3 to 7 and 6; open area the same; trims the
  same. Block Sans has a bulge of 0.9 mm (0 before), Varsity runs out of
  the outline 0.4 mm (0.2).
- Bookish Slab as lettering at 25 mm, open area: "S" 4.5 to 0.2 mm2,
  "9" 2.7 to 0.3, "r" 2.9 to 0.3, "j" 1.2 to 0.1, "6" 2.4 to 0.3. The
  stitches of the balls of "S" and "9" lie as the stitches of their
  strokes.
- The letters in all 129 fonts have 34,070 trims (33,966 after Milestone
  10): Zen Maru Gothic 91 more (1 %), the others 13 more. The columns
  stop at the outline, so a stroke that touched the next only with its
  stitches out of the outline (the dots of a colon in Bungee) needs a
  trim now. The lettering bench of P20: stitches out of the outline 0.6 %
  to 0.2 % at the most; the trims of 刺繍 10, 11, 15, 16, 16 to 11, 12,
  16, 16, 16 and of 龍 6 to 7 at 10 mm.
- The scores: the 55 column fonts of the program 94.2 to 94.3 on mean
  (five fonts 1 point lower).
- The golden designs with lettering: stitches -1.5 % to +2.7 %;
  embroidery-fonts-2 and embroidery-fonts-4 1 and 3 more trims.
