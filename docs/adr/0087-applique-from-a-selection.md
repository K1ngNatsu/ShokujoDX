# ADR 0087: Appliqué from a selection

Date: 2026-10-30 · Status: Accepted (extends the appliqué of P3 Milestone 7)

## Context

The owner asked for better appliqué tools after a study of a consumer
embroidery product. A frequent job is a design with a fill in a satin
border, where the fill must become fabric (fewer stitches, a softer
patch). Before this change, that took several steps: weld the shapes,
convert them, and then remove the border or set a cover again. There was
also no way back to the objects as they were, other than undo.

An appliqué from open shapes was not added: partial appliqué (edges with
no cover) does that work already.

## Decision

- **One command: Digitize > Appliqué from Selection
  (`Design::applique_from`).** The fabric is the area of the selected
  fills, closed vectors and appliqué objects, all in one (overlapping
  shapes become one piece, separate shapes are pieces of one object).
  A closed border (a satin ring, or a closed line with a border) round
  them adds the area inside its middle line, so the cut edge of the
  fabric is under the border also when the fill stops at the inner edge
  of the border. With no areas in the selection, the area inside each
  closed border is the fabric.
- **A selected border is the cover.** An edge of the fabric is covered if
  90 % of its points, and the points of the tack-down line, are in a
  selected border (to 0.2 mm). A covered edge gets no cover of its own
  (an open edge, as in partial appliqué). If the borders cover all
  edges, the appliqué has no cover of its own. The borders that cover
  an edge sew after the appliqué, and the appliqué sews in the thread of
  the first of them, so the border hides the placement and tack-down
  lines. The fabric shows the color of the fill that it replaces.
- **Back to Original (`Design::back_to_original`).** A record
  (`Design::applique_origins`, `AppliqueOrigin`) keeps the replaced
  objects, the borders that sewed before them, and the border traces
  that they were part of (the traces end with the command). The record
  goes with the project file. Back to Original puts the objects back in
  the place of the appliqué, with their ids, and the traces again. When
  the appliqué moves, the kept objects move with it; their ids are kept
  (`kept_ids`), so no new object takes one. A delete of the appliqué
  removes the record.
- Objects with a shape link or a sketch link are refused (the link would
  break); remove the link first.

## Consequences

- A fill in a satin border becomes appliqué in one step, with the border
  as the cover. No golden snapshot changes: the command makes new
  objects only when the user runs it.
- Back to Original ignores the changes made to the appliqué after the
  command (it puts back the objects as they were, moved with it).
- A hole of a piece always gets a cover of its own (partial appliqué has
  open edges on the outline only), so a border round a hole that does not
  cover all edges sews over that cover.
