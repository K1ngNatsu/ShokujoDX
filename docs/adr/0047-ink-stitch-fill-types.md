# ADR 0047: Tartan, gradient, meander, circular, guided fills and ripple stitch

Date: 2026-10-25 · Status: Accepted (P11 Milestone 4)

## Context

P11 Milestone 4 adds six stitch types that Ink/Stitch has and this
program did not have: tartan fill, linear gradient fill, meander fill,
circular fill, guided fill and ripple stitch. ADR 0022 says that a
stitch type is a setting of the object kind that has the matching
geometry. Two of the new types have more than one color, and an object
has one thread.

Ink/Stitch is GPL-3.0, so a port with a header is allowed (spec, rule 4).
The methods here are our own; they follow what the public Ink/Stitch
documentation says each type looks like.

## Decision

1. **Five fill types and one outline type** (ADR 0022):
   `FillPattern::Tartan`, `Gradient`, `Meander`, `Circular` and `Guided`,
   and `RunningParams::ripple` (`Ripple`) for lines. New settings are not
   written when they have the default value, so old files and the
   golden snapshots stay the same. The points in the settings (gradient
   line, center, guide lines, ripple point and guide) move with the
   object (`ObjectKind::transform`).
2. **A color fill is a group of fills.** A tartan or a gradient of `n`
   colors is `n` fills with the same shape, in one group; each fill has
   the settings of the whole (the sett, or the places of the colors) and
   the number of its own color, and sews only the rows of that color
   (`fill::RowKeep`). Only the first fill has underlay, because the
   colors together cover the shape one time. `tartan::tartan_layers` and
   `tartan::gradient_layers` make the settings; Edit > Stitch types >
   Tartan and Gradient (one window) make the group from the selected fill
   in one undo step, and make it again with new settings.
3. **Tartan.** The sett is its usual text form (`K4 R24 K24 Y4`: a color
   code and a thread count for each stripe; mirrored by default), with
   the width of one count. The codes have colors of our own; the threads
   of a new tartan get these colors. The stripes are centered on the
   middle of the shape. Each color sews tatami rows in its stripes, at
   the fill angle (warp) and square to it (weft), each direction one row
   spacing apart (0.8 mm at the start). **Each part of a stripe starts a
   new needle path**, so the plan puts a trim between the stripes of one
   color: travel would lie on top of the other colors. (The corpus
   design, a 90 mm tartan of four colors and a 60 mm check, has 58
   trims.)
4. **Gradient.** One tatami layer at the fill angle; each row gets one
   color from its place between the start and the end of the gradient
   (by default one side of the shape to the other). Between two stops
   the share of the second color goes from 0 to 1, and row `k` gets it
   when the golden-ratio number of `k` is below the share
   (`fill::gradient_color`). So the rows of all colors together have the
   set density, and the colors mix evenly. A gradient fill alone has no
   underlay (it fades to the fabric).
5. **Meander.** A graph of the corners of a tile pattern (squares,
   triangles, hexagons; one spacing a side; turned by the fill angle)
   inside the shape. The line starts with one step, then steps become
   detours round a tile through free corners, in an order from a fixed
   seed (newest first made long straight combs); then the ends go on,
   and a corner can give its place to a longer way round. A line on the
   tile edges that uses each corner one time cannot cross itself. The
   line uses 85 % to 99 % of the corners in the tests.
6. **Circular.** One spiral round the center (by default the middle of
   the shape), one spacing a turn, cut at the edge; the parts are sewn
   nearest first (`rings::sew_nearest`).
7. **Guided.** Rows that mix two guide lines point by point between
   them (no two rows farther apart than the spacing), and copies of the
   outer guide lines past them. The rows go past the shape and are cut
   at its edge, with staggered needle points. No guide: tatami rows.
   Edit > Stitch types > Guide lines makes the selected lines the guides
   (one or more; Florentine stays for one guide with tatami rows).
8. **Ripple.** Copies of the line, scaled round the point (from 100 %
   to the size of the last line) and moved along the guide. A closed
   line sews as one spiral from ring to ring; an open line sews back and
   forth. Edit > Stitch types > Ripple guide makes the second selected
   line the guide.

## Consequences

- A tartan needs trims (fewer if the stripes are wide); the sew time
  in Design info shows them. A satin border over the edge hides the
  stripe ends.
- Editing one fill of a tartan or a gradient changes that fill only; the
  window makes the whole group again from its settings.
- Meander fills of hexagons leave the middle of each tile open, so they
  look more open than squares and triangles.
- The exit test (sew-outs of each type on the Topaz 20) is open.
