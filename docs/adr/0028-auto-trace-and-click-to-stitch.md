# ADR 0028: Auto-trace and click-to-stitch

Date: 2026-10-23 · Status: Accepted (P6 Milestone 6)

## Context

Roadmap P6 Milestone 6 turns a backdrop picture into artwork and
embroidery: auto-trace makes vector shapes of the color areas, and a
click on a color area makes a fill (with or without its holes), satin
columns, a center line or an outline. The P6 plan asked that the
click-to-satin center line and the satin letters (ADR 0015) share one
method.

## Decision

- **Trace from the colors of the picture.** A picture with 32 colors or
  fewer (a prepared picture, flat artwork) is traced with its own
  colors (`emb_art::reduce::exact`); a picture with more colors is first
  reduced with the Prepare colors settings, and the message says so.
- **Each edge between two colors is traced once** (`emb_art::autotrace`).
  The pixel sides between colors make chains from node to node (a node
  is a pixel corner where three or four edges meet). Each chain is
  smoothed once, and the two areas on its sides use it in opposite
  directions. So traced areas have no gap and no overlap, and an area
  traced alone (a click) is the same as in a whole trace.
- **Smoothing keeps corners.** A turn of more than 60° over four pixel
  sides is a corner. Between corners, the middle points of the pixel
  sides are averaged over a few pixels (this removes the steps of a
  slanted edge), then the curve fit of `emb_geom::simplify` (our own code
  of the method that Schneider published) makes the fewest cubic curves
  within 0.8 px. A straight part is a line.
- **Two pixels of one color that touch only at a corner** are one area
  if their color is the less common of the two colors at that corner. A
  thin diagonal line of a detail color stays whole; the background
  behind it is cut.
- **The background** is the most common color at the edge of the
  picture. Auto-trace leaves it out unless the user asks for it.
- **`emb-art` depends on `emb-letter`** for the stroke finder
  (`emb_letter::stroke::strokes`), so click satin and the satin letters
  share one method. This is simpler than moving the grid functions to a
  lower crate, and both are engine crates with no UI. Click satin then
  pulls rail points that are outside the area back to its outline: the
  stroke finder gives a column end the width just before it, which is
  right for the square ends of letters but sticks out of a round end.
  The stroke finder itself does not change, so letters sew as before.
- **Click to stitch is one tool** (Artwork > Trace > Click to stitch)
  with five buttons that choose what it makes. Each click is one undo
  step; the tool stays on for the next click. The new objects take the
  design thread nearest to the area color, or a new thread.

## Consequences

- Averaging moves a small curve a little to its inside (about 0.3 px at
  a radius of 15 px). At the resolution of a logo picture this is under
  0.1 mm, less than the pull compensation of a fill.
- Very thin areas (one or two pixels wide) can get sharp tips at their
  ends; reduce or enlarge the picture first.
- The trace does not yet join areas into larger shapes or choose stitch
  types: that is smart digitizing (Milestone 7).
