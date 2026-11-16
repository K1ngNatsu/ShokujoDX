# ADR 0097: Sketch origin, locked sketch objects and the Sketch tab groups

Date: 2026-11-16 · Status: Accepted

## Context

The owner asked for four changes of sketch mode (v0.3 plan, Stage 4):
a fixed construction point at (0, 0) in each new sketch; no scaling of a
sketch outside sketch mode; objects made from a sketch locked to it until
Break Apart; and a Sketch tab with drop-down buttons, as in the reference
CAD program. Before, a made object could move alone (the move went into
the `offset` of its link, ADR 0076), and the Sketch tab had rows of small
buttons for the kinds of each tool.

## Decision

- **Origin.** `Sketch::origin` is a point at (0, 0) with a `Fix`
  constraint. `Sketch::with_origin` makes it; Start Sketch uses it.
  `remove_point` and `remove_constraint` keep the origin and its Fix, a
  join of corners does not merge it away, and Delete in sketch mode skips
  it. `Sketch::is_empty` does not count it (an empty sketch still goes
  away at Finish Sketch). It shows as a ring with a cross. Sketches of
  older versions have no origin.
- **No scale outside sketch mode.** `Transform::is_rigid` is true for
  moves, turns and mirrors. `Design::scales_sketch` is true when a
  transform that is not rigid touches a sketch or an object made from one;
  then `Design::transform_objects` leaves the sketch and its objects as
  they are (other objects of the selection change). The app says why
  (`AppState::warn_sketch_scale`), and the scale handles do not start a
  drag (`refuse_sketch_scale`).
- **Locked objects.** `Design::sketch_objects_moved` moves a sketch and
  all the objects made from it together, also when only a made object
  moved (the canvas drags, nudges and the move commands all call it). The
  offsets of older files stay the same move after the sketch. Reshape does
  not edit the nodes of a made object and says to change the sketch or to
  use Break Apart. **Break Apart** removes the sketch links of the
  selected objects (and splits objects with more parts, as before);
  Unlink still does the same.
- **Sketch tab.** `widgets::split_large` is a large button with an arrow
  at its right: the button runs the command that it shows (the tool in
  use, else the last pick, else the first), and the arrow opens a menu of
  all its commands. Each drop-down is a `Custom` item with its commands,
  so the help, the icon test and the shortcuts still see each command.
  The groups: Sketch (Select, Regions), Create (Line, Circle, Arc and
  Rectangle drop-downs; Fillet, Chamfer, Point), Modify, Pattern,
  Constrain (Dimension, the constraint icons, Automatic Dimensions,
  Parameters, Relax), Format (Construction, Centerline, Center Point) and
  Exit (Finish Sketch). There is no Insert group: the program has no
  insert command in sketch mode, and Stage 4 adds no new features.

## Consequences

- Points drawn at (0, 0) snap to the origin and are fixed with it, as in
  CAD programs. Tests that draw at (0, 0) and are not about the origin
  use a sketch with no origin (`sketching()` in the app tests).
- A made object that the user moved alone in an older version keeps that
  offset; it cannot get a new one.
