# ADR 0070: Drawing help and sketch edits

Date: 2026-10-27 · Status: Accepted (P14 Milestone 3)

## Context

Sketch mode (ADR 0069) needs the help of CAD programs while the user
draws, and the edits that change curves that are already in the sketch:
trim, extend, split, fillet, chamfer, offset, mirror and patterns. A
parametric sketch must stay true after each edit: when a dimension changes
later, the result of the edit must follow.

## Decision

1. **Drawing help in the engine** (`emb_sketch::infer`). A click near a
   point uses the point (coincident); a click near a curve makes a new
   point with an on-curve constraint; a line within 4° of horizontal or
   vertical becomes so (its free end moves there); a line that starts or
   ends at the end of an arc in the direction of the arc becomes tangent
   to it. The canvas shows the marks of these constraints (H, V, T, O)
   next to the pointer before the click. Alt, or a check box of the
   Properties panel, stops them; a click on a point still uses the point,
   so a chain still closes.
2. **Edits in the engine** (`emb_sketch::edit`, methods of `Sketch`), each
   with the constraints that keep its result true:
   - Trim removes the piece between the curves that cut the curve (a
     curve that nothing cuts goes away; a circle with two cuts becomes an
     arc). Extend moves an end to the next curve that the extension meets.
     The new ends get an on-curve constraint to the curve that cut them.
     A middle piece of a line stays in line with the first piece (two
     on-curve constraints); a piece of an arc keeps the center and an
     equal radius. Split does the same at one point.
   - Fillet and chamfer cut the corner of two lines. The corner stays as
     a point on both lines, so the dimensions to the corner stay; a length
     of a line becomes the distance from its other end to the corner. A
     fillet arc is tangent to both lines and has a radius dimension; a
     chamfer has two distances from the corner.
   - Offset copies a chain of curves (curves that follow on at their ends)
     at a distance: outside of a closed chain, else on the left. A line
     stays parallel at a new dimension, the distance from a point to a
     line (`Constraint::LineDistance`). An arc keeps the center of its arc;
     tangent curves stay tangent, and an arc with no tangent gets a radius.
   - Mirror copies curves about a line with a symmetric constraint for each
     point (a point on the line is shared).
   - Rectangular and circular patterns copy curves with the constraints
     between them (a turned copy has no horizontal or vertical
     constraints). The places of the copies are free: a parameter can link
     them in Milestone 6.
   - Relax removes the constraints that conflict, dimensions first, until
     none conflicts, and returns them for the list.
3. **A tangent at a shared end is a right angle.** When the line and the
   arc (or two arcs) share an end, the tangent equation is "the radius at
   the end is at 90° to the line" (or "the two centers and the end are in
   line"), not "the distance from the center to the line is the radius".
   The distance form has a double root: an error of 10⁻¹⁰ lets the end
   slide by about 10⁻⁵ mm. The new form is exact to the solver tolerance.
4. **The app** (`emb-app/src/sketch.rs`). The Sketch tab gets a Modify
   panel (Trim, Extend, Split, Fillet, Chamfer, Offset), a Copy panel
   (Mirror, Rectangular Pattern, Circular Pattern) and Relax. An edit that
   the engine refuses changes nothing and makes no undo step
   (`Document::try_edit`); its message is translated (`edit::MESSAGES`).
   A fillet or a chamfer opens the value box of its new dimension. The
   values of the tools (radius, distances, columns, rows, copies, angle)
   are in the Properties panel. The Dimension tool waits after a line: a
   second line makes an angle, a point a distance to the line, an empty
   place the length.

## Consequences

- An equal-length constraint on a line that a fillet shortens still
  compares the shorter line.
- The copies of a pattern do not follow the first curves when they move.
- A tangent to a full circle (no ends) keeps the distance form.
- Not tested in the real app on 2026-10-27: a window of the Windows text
  input kept the front of the screen. The automated tests drive the tools
  through the app state and the mouse.
