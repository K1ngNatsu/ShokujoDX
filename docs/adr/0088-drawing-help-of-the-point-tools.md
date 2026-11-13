# ADR 0088: Drawing help of the point tools

Date: 2026-10-30 · Status: Accepted (extends the draw tools of P2)

## Context

The point tools (Line, Shape, Complex fill, the columns, Appliqué and the
others) took left clicks for corners and right clicks for curve points,
and snapped to the grid, the guides and the nodes of other objects. Sketch
mode (P14) has much more help. The owner asked to bring the main help of
CAD and vector programs to the everyday draw tools, without sketch
constraints.

## Decision

- **One place for the next point: `AppState::draw_target`.** The click,
  the drag and the preview all use it, so the preview shows the point that
  a click places. With Shift, the line from the last point keeps a step of
  15° on the screen (the view after the design rotation), at the length
  of the pointer along that line. Else the point goes through
  `AppState::snap_hit`.
- **The object snap finds more: `SnapHit` and `SnapKind`.** With "Snap
  objects" on, the order is node, midpoint (of a line, or of a curve at
  its middle), center (of the box of a closed shape), edge (the nearest
  point of an outline), each within the snap distance. A part of another
  object wins over the grid and the guides. If there is none, the point
  lines up with the X or the Y of a point of the object being drawn. The
  preview shows a mark and the name of what the point snaps to, and a
  dashed line to a point that it lines up with. `AppState::snap` keeps
  its result type (a point) for the other callers.
- **Typed lengths: `DrawState::typed`.** While a point tool has points,
  typed text that starts with a number is the length of the next point
  (`workspace::parse_length`: units, fractions and `+ - * /`), with an
  angle in degrees after `<` (counterclockwise from the right, as the
  Measure tool shows it). `draw::take_typed` takes the text events and
  the keys of the text before the shortcuts (so the "h" of an "in" text
  does not mirror the selection); Enter, Backspace and Esc work on the
  text first. With no angle, the point goes toward the pointer.
- **Drag-out handles: `DrawState::tangents`.** A left drag in a tool that
  takes curve points places a curve point with the tangent of the drag
  (the curve leaves toward the end of the drag). The tangents are a list
  of (point, tangent), and `Path::through_points_with` asks it for the
  tangent of a point, so `build_with` does not need a new point type, and
  all parts of a tool (rails, holes) take handles. A drag shorter than
  4 screen points is a corner. Continue Line turns the handles round when
  it adds the points before the start of the line.

## Consequences

- No design file changes: the help only changes where the points go.
- The snap looks at the outlines of the objects near the pointer (by
  their boxes) each frame, so a very large design can make the pointer
  slower while the object snap is on.
- The handles cannot change after the point is placed (Backspace removes
  the point and its handle); Reshape changes them after the object is
  made.
