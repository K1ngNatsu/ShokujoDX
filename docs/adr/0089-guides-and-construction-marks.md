# ADR 0089: Guides and construction marks

Date: 2026-10-30 · Status: Accepted (extends the guides of P2)

## Context

Guides were solid lines from the rulers, with a place (`Guide::pos`) and
a direction only. The owner asked for reference markers on the rulers,
dashed construction lines, and a right-click menu on a marker to change
it (delete, color, dashed or solid, move, and other useful changes).

## Decision

- **A style for each guide and mark: `GuideStyle`** (a color or the guide
  color of the Options, solid or dashed, locked). The default is dashed;
  a guide in an older file has no style, so it shows dashed too. A
  locked guide does not move with a drag; the menu and the edit window
  still change it.
- **Construction marks: `Design::marks`** (`ConstructionMark`,
  `MarkShape::Line` through a point at an angle, `MarkShape::Point`).
  They are in view coordinates (after the design rotation), as the
  guides are, and do not sew. The View tab has the tools Construction
  Line (two points, with the drawing help of ADR 0088) and Construction
  Point. Remove Guides removes the marks too.
- **Snap.** With Snap to Guides, a point goes first to a construction
  point or a place where construction lines and guides cross
  (`guides::snap_point`, before the grid), and else onto a construction
  line near it (`guides::snap_line`, after the guides). The snap marks
  show `SnapKind::Mark` and `SnapKind::Intersection`.
- **Ruler markers.** Each guide has a triangle on its ruler (an outline
  if it is locked). A drag of a marker moves the guide along the ruler
  (it goes only when dropped off the canvas; a guide line dropped on a
  ruler goes, as before), with its place from the hoop center at the
  marker. A double click opens the edit window; a right click opens the
  menu with any tool. A click on a ruler is never a point of a tool.
- **The menu and the edit window (`emb-app/src/guides.rs`).** The menu
  (a marker, or a guide line or a mark under the pointer with the Select
  tool and no object there) has Position…, Move to Hoop Center, Color
  (six colors, the guide color, any color), Dashed, Locked, Duplicate (a
  copy one grid step away; a line is parallel) and Delete. The edit
  window takes the place from the hoop center in the units of the
  Options (`workspace::parse_length`), and the angle of a line. Each
  change is one undo step.

## Consequences

- Guides in older files show dashed. No stitches change.
- A mark under an object has no menu with the Select tool (the object
  menu wins); Remove Guides, or a move of the object, reaches it.
