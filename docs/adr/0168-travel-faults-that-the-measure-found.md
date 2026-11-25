# ADR 0168: Travel faults that the measure found

Date: 2026-11-25 · Status: Accepted (P22, after Milestone 1; the owner:
"Fix the travel faults then begin M2").

## Context

The measure of travel that shows (ADR 0167) found travel in the corpus
that lies on top of sewn stitches or on the fabric:

- a fill of `shaping` (a square less a circle and the circle less the
  square: two rings that meet at two corners) whose travel went across
  its own cut-out part, 19 mm on the fabric;
- 8 to 20 mm of travel on the fabric in lettering of `embroidery-fonts-3`,
  `-4`, `-6`, `-7`, `sewout-towel` and `sewout-cap` (for example at the
  "T" of a script font);
- travel straight across sewn rows in guided fills (55 and 42 mm) and a
  circular fill (37 mm).

## Decision

1. **A line through a corner of the outline.** `Travel::visible` split a
   line only where it crosses an edge, and took the line as inside if its
   middle was inside. A line that goes out of the shape through a corner
   crosses no edge: the line of the `shaping` fill went through the centre
   of the circle, out of one ring at a corner of its arc and into the
   other at the corner where the rings meet. Now the line is also split
   where it touches an edge (`touch_params`), and each piece must be
   inside. A line that touches nothing has one piece, as before.
2. **A travel run of a fixed letter keeps its direction**
   (`Run::fixed`). The plan sews a run that is not fixed from the end
   nearer the needle. As the first run of a lettering object, the needle
   came from the object before it, nearer the far end of the run: the
   plan turned the run, and a straight travel stitch of up to 27 mm went
   back over the fabric to the next part of the letter.
3. **Only straight rows have a direction.** ADR 0166 gave all fills the
   direction of the fill angle (`Sewer::rows`), also the fills with curved
   rows (circular, guided, contour, the texture fills...): their travel
   went straight across the rows at the fill angle (ADR 0166 said that
   these keep the travel of before; they did not). Now only a fill with
   tatami rows (`row_split`) has `Sewer::rows`.
4. **Circular and guided fills go round their sewn rows.** Before their
   top rows, `Sewer::track_sewn` starts a map of the cells (1 mm) that the
   stitches go through (`Sewn`). Their travel between two parts
   (`Travel::edge_path_sewn`) costs `OVER_SEWN_COST` (9) more for each mm
   across the inside that lies on sewn cells, so it goes along the edge or
   under rows that are not sewn yet. If the best way still lies more than
   `MAX_OVER_SEWN_MM` (2 mm) on sewn cells, farther than 1 mm from the edge
   (the turns of the rows), the needle path starts again at the next part:
   the plan trims there, or makes a short travel stitch when the gap is
   short, as for any two runs. This is not used for the other curved
   patterns: a satin fill of a ring has no way round its sewn columns and
   got two more trims (`satin_from_svg_has_no_pile_ups`).

## Consequences

- The travel that shows in the corpus (`tests/travel_shows.rs`): `shaping`
  37.5 mm to 0, the lettering of the five designs 7.5 to 28 mm to 0 (and
  `embroidery-fonts-4` 10.9 to 0.9 mm), `guided-fill` 97 to 0 mm,
  `circular-fill` 37 to 0.2 mm, `lace-edge` 11.7 to 1.4 mm on stitches.
- 21 golden snapshots change, -0.03 % stitches in all. Trims: `shaping`
  10 to 14 (its two rings meet only at corners, so there is no way
  inside from one to the other), `circular-fill` 3 to 5 and
  `guided-fill` 2 to 4 (where all ways go over sewn rows). The lettering
  designs change by a few stitches and keep their trims.
- Open (`docs/design/open-work.md`): in `embroidery-fonts-9`, the route of
  a letter of Fat Face Serif joins two parts with an 11 mm travel stitch
  that goes back over the column sewn before it (4.2 mm shows). The
  route of the font decides this link (`joined`), so the fix is in the
  route and the fonts, not in the travel.
