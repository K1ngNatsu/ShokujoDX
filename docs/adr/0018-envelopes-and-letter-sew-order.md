# ADR 0018: Envelopes and the sew order of letters

Date: 2026-10-22 · Status: Accepted

## Context

P4 Milestone 4 adds envelopes (the text bends into a shape such as a
bridge or a pennant), a stitch type and an underlay for each letter, and
the sew order of the letters with their connections. The exit test asks
for a name in a bridge envelope with fewer trims than the default order.

Most trims in lettering are not between letters but in a letter: the
strokes of a letter meet at junctions, and the start of the next stroke is
often more than 1 mm from the end of the last one.

## Decision

- **An envelope moves the top and bottom edges of the text box.**
  `emb_model::Envelope` has a shape (12 shapes: bridge, valley, arch over
  and under, bulge, pinch, pennant left and right, perspective left and
  right, diamond, wave) and an amount (a part of the text height). The box
  is from the start of the first letter to the end of the last one, from
  the capital height to the last baseline, so it does not change when one
  letter moves. Each point keeps its X position and its relative height in
  the box. The parts get points at most 1/80 of the box width apart first,
  so straight lines bend.
- **Envelopes come last.** Layout puts the letters on lines, applies the
  changes by hand, then bends. Envelopes apply only to a straight
  baseline. The warp has an exact inverse, so a move by hand without a new
  layout and a shape changed by hand (ADR 0017) stay right in an envelope.
- **Each letter can have its own stitch type (TrueType fonts) and its own
  underlay** in its `LetterEdit`.
- **Three sew orders.** Text order sews as before (the golden snapshots
  do not change). Center out sews from the middle letter to the right end,
  then from the middle to the left end. Closest join tries each letter as
  the first one, goes to the nearest letter next, and keeps the shortest
  route.
- **Center out and closest join optimize each letter.** For a letter with
  at most nine parts, a search over all orders and both directions of each
  satin column (dynamic programming over the sets of sewn parts) finds the
  fewest trims, then the shortest travel, then the end nearest the next
  letter. A connection needs no trim when it is at most `trim_over_mm`
  long, or when the parts that sew after it cover the whole travel line:
  their stitches hide the travel stitch.
- **Connections.** `trim_over_mm` (default 1 mm) is the longest gap with a
  travel stitch. With the jump connection, longer gaps are jumps, not
  trims, for machines with no trimmer.

## Consequences

- A bridged name in Varsity ("Joshua", 20 mm) sews with 12 trims in
  closest join order and 20 in text order. Over five fonts and four names
  in a bridge, the optimized orders have a third to a half fewer trims.
- A hidden travel stitch can be up to a few millimeters long; the satin or
  fill on top covers it.
- Text order stays as the default, so existing designs sew as before. The
  user picks closest join or center out.
- An envelope turns curves into short straight lines in the letter parts.
