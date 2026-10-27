# ADR 0072: Dimension placement, driven dimensions, more snaps and tools

Date: 2026-10-27 · Status: Accepted (P14, the owner's second request)

## Context

The owner asked for the sketch features of the reference CAD program, under the
clean-room rules, and named three problems: the Dimension tool did not
show what a click dimensions, the arc tools did not show the first click,
and the drawing help had no midpoint or alignment snaps. The plan
`docs/design/p14-plan.md` now has a table of the features of that
program, in our own words, with their state.

## Decision

1. **The Dimension tool shows a ghost.** While the pointer moves, the
   dimension that the next click makes shows at the pointer, half
   transparent. When its parts are picked (a circle, two points, a point
   and a line, two lines), it waits for a click that places the label; a
   line and then an empty place makes its length there.
2. **Label places** are `Sketch::label_at` (sketch coordinates, kept in
   the file). A length or a distance takes the offset of its label from
   its line; a radius points its leader at the label. Select drags a
   label (one undo step); Reset Label Place removes the place.
3. **Driven dimensions** (`SketchConstraint::driven`) are not equations;
   their value follows the sketch after each solution (`update_driven`)
   and shows in parentheses. `Sketch::add_dimension` makes a new dimension
   driven when the sketch sets that size already (the analysis finds it
   repeated or in conflict), so a dimension never makes a conflict by
   itself. Make Driven and Make Driving change it.
4. **More snaps** (`infer::snap`): a point, then the middle of a line (a
   midpoint constraint), then the place where two curves cross (on both
   curves), then a curve. A free click takes the x or the y of a point in
   line with it (the place only, with a dashed line), as the other
   program shows.
5. **More hints for lines**: tangent to a circle that the end touches in
   its direction; perpendicular to the line before it in the chain;
   parallel or perpendicular to the line that the pointer went over last.
6. **Previews**: a draw tool draws the shape that the next click makes by
   running the tool on a copy of the sketch, so each preview is exactly
   the result. The points of the clicks show at once.
7. **New tools**: center rectangle (diagonals as construction lines, the
   center on one), three-point rectangle (perpendicular and parallel
   sides), slot from two centers (half circles tangent to the sides, equal
   radii, a construction line between the centers), polygon round its
   circle (the sides tangent to it). A new constraint, collinear (the two
   ends of one line on the other line).
8. **Use**: a drag of a part of the selection moves all of it; F8 shows
   and F9 hides the marks of the constraints (also a check box).

## Consequences

- The program does not ask "make it driven?" as a question; it makes the
  dimension driven and says so on the message line.
- A placed label stays where it is in sketch coordinates; when the sketch
  changes a lot, the user can reset it.
