# ADR 0091: Scale from point

Date: 2026-11-13 · Status: Accepted (changes the scale handles of the canvas)

## Context

The scale handles kept the opposite side of the selection in place (Alt:
the center). So a design moved away from the hoop center each time its
size changed, and the owner had to use Center in Hoop after each change.
The W and H fields of Object Properties already kept the center.

## Decision

- A setting `Settings::scale_from` (`manipulate::ScaleFrom`): one of nine
  points of the selection box, in screen directions. The default is the
  center. Object Properties > Position and size has a picker of the nine
  points (Scale from); the choice is saved with the settings.
- The handles and the W and H fields keep that point in place. For a
  handle drag, an axis where the point is on the side of the handle uses
  the opposite side (a point on the handle cannot stay). Alt uses the
  opposite side (the old behavior).
- The canvas shows the point as a small cross on the selection box; during
  a drag it shows the point that the drag uses.
- The design rotation turns the screen directions to design directions
  (`ScaleFrom::in_design`).

## Consequences

- A drag of a handle by a distance changes the size two times as much as
  before when the point is the center (both sides move).
