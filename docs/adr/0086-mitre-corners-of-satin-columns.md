# ADR 0086: Mitre corners of satin columns

Date: 2026-10-29 · Status: Accepted (extends ADR 0015 and ADR 0085)

## Context

The owner and a colleague saw that many square corners sew as a stop and
a start: one satin column ends, and the next column starts over its end
(a lap). This is so in the letters of the satin fonts (the strokes are
cut at each corner, ADR 0015) and at the bar tacks of a buttonhole. A
mitre corner, where one column turns round the corner, sews with no stop
and does not build up thread at the corner. They asked for a choice
between the two, and asked to control the gap of a buttonhole (that is
the setting "Gap for the cut" of ADR 0085).

Lines with a width already had Mitre, Cap and Lap (`LineBorder::corner`),
and a column from a centerline turns at its corners already.

## Decision

- **One method for all columns: `emb_model::mitre`.** `join` makes the
  end of one column and the start of the next one column. The corner
  points are where the line of each rail at the end meets the line of the
  same rail of the next column (the outer corner and the inner corner).
  A long straight last segment (one width or more: a drawn column, a
  buttonhole) is the line. Else the line goes through the rail points two
  and three widths from the end: the ends of font strokes are ragged (they
  narrow and spread for one to two widths), and a line from the last
  segment put the inner corner of an "E" 3.5 mm down the stem, so the
  stitches went into the open part of the letter. The rail points past
  the corner are removed, a rung goes from the outer to the inner corner,
  and a rung half a column width before and after the inner corner keeps
  the stitches straight up to the turn
  (the stitches tilt over that length, so they do not all meet at the
  inner corner). `close` makes a ring of a column whose end meets its
  start. `join_parts` joins all the satin parts of a list whose ends are
  nearer than 0.9 widths, nearest first.
- **The stitches stay in the footprint.** The inner edge near the corner
  must stay in the area of the two columns (to 5 % of the width, at least
  0.1 mm), else the corner stays a lap. The outer edges can meet out of
  the columns (a corner that turns more than 90°, as at the top of a
  "7"): then the corner is cut across from the end of one outer edge to
  the start of the other, and the cut must stay in the columns too. The
  outer corner can fill a notch between two lapped columns, as the letter
  does. A test (`emb-cli/tests/mitre.rs`) checks that no stitch of mitred
  letters is more than 0.3 mm out of the lapped letters.
- **Only real corners.** A turn of less than 30° is not a corner. A turn
  of more than 120° (the apex of an "A") makes a mitre that is too long,
  so it stays a lap. A stroke that ends on another stroke (a "T") has no
  end near the ends of that stroke, so it does not change.
- **Lettering: `Lettering::mitre_corners`** (off by default, so no design
  changes). The layout joins the parts of each letter from the font
  after it scales them, for TrueType fonts and for embroidery fonts. A
  letter shape that the user changed by hand keeps its parts. The check
  box is in the Satin settings of the lettering.
- **Buttonholes: `ButtonholeSpec::mitre`** (off by default) for the
  square ends, the round end and the keyhole: the legs and the bar tacks
  are one satin ring with short stitches. The stretch shape keeps its bar
  tacks (its legs have open spacing, and the ring has one spacing). Bar
  tacks hold the ends of the cut better, so they stay the default.
- **Join (Ctrl+J)** joins two satin columns that meet at a corner with a
  mitre, after it joins the columns whose ends touch.

## Consequences

- Square letters such as "L", "E" and "7", and buttonholes, can sew with
  no stop at the corners. The look is not tried on a machine yet.
- The corpus design `mitre-corners` (each look side by side) has a new
  golden snapshot. The other snapshots do not change, because the
  settings are off by default.
- A corner of a letter whose two strokes do not come near each other
  (the strokes of some fonts stop far from the corner) stays a lap.
