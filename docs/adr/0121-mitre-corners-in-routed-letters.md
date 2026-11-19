# ADR 0121: Mitre corners in routed letters

Date: 2026-11-19 · Status: Accepted (on pictures only; no sew-out)

## Context

The Mitre Corners setting of lettering (ADR 0086) makes two columns that
meet at a corner one column that turns (`mitre::join_parts`). It looks at
all the ends of the columns of a letter and joins the nearest pairs, in
each direction that fits.

A routed letter is fixed (ADR 0113): the font gives the order and the
direction of its parts, with travel runs between them, and the stitch
engine sews them as they are. A join of two parts that do not sew one
after the other breaks the route. So the layout did not join the parts of
a fixed letter, and since all embroidery fonts of the program and of the
packages are routed (ADR 0116), the setting did nothing for them; the
check box was not available. The review of P17 named this as a lost
setting (lane 6): the route joins the corners when the font is made, or
the layout joins two parts that sew one after the other.

## Decision

- **The layout joins parts that sew one after the other**
  (`mitre::join_in_order`): two satin parts, where the end of the first
  one meets the start of the second one at a corner, become one column
  with a mitre corner (the same `join_cols` and the same rules as ADR
  0086: a turn of 30° to 120°, the stitches in the footprint). The joined
  column sews from the start of the first part to the end of the second
  part. So the order and the directions of all other parts stay, the
  travel runs stay, and the letter is still fixed.
- **Not in the font.** A font with mitred corners is another look for
  each user of the font. The setting is off by default (ADR 0086), and a
  design with the setting off does not change.
- **No ring from a fixed column.** `join_parts` makes a ring of a column
  whose end meets its start. A ring starts at the place nearest the
  needle, not at the place of the route, so a fixed letter keeps the
  column.
- **The settings of a part** (`PartTuning`, ADR 0112) go with the parts:
  the joined column has the settings of the part that sews first.
- **The check box is there for each font** (it was not available when
  all letters were fixed).

## Consequences

- With the setting on, a routed font has mitre corners at about as many
  places as a font with no route (all letters and digits at 20 mm):
  Varsity 24 corners (25 with no route), Anton 9 (9), Block Sans 5 (9),
  Pacifico 5 (6). The corners that stay a lap are those where the route
  sews something between the two strokes, or sews a stroke toward the
  other end (the second corner of "E", the "B", the "7" of Block Sans).
- No letter gets a new trim: all letters and digits of four fonts have
  not more trims with the setting on than with it off (a test; Geo Slab
  has one trim less), and no stitch
  of the mitred letters is more than 0.3 mm out of the lapped letters
  (the test of ADR 0086, now for three routed fonts).
- The corpus design `mitre-corners` has a new golden snapshot: its
  letters in Block Sans have mitre corners now ("E", "L" and "F").
- A design that has the setting on with an embroidery font of the
  program changes when its letters are made again: before, the setting
  did nothing there.
- To reach the other corners, the route must know of the setting when
  the font is made (an order that sews the two strokes of a corner one
  after the other). That makes a second route for each letter; it is not
  done.
- No sew-out confirms the look.
