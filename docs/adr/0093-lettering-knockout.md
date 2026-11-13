# ADR 0093: Lettering knockout

Date: 2026-11-13 · Status: Accepted

## Context

The owner put two text boxes on top of each other. The letters of the back
text sewed under the letters of the front text. Flatten and Remove
Overlaps did not use lettering (`shaping::takes_part`), because a cut
makes plain fills of the letters, and the text then cannot change. Break
Apart and then Flatten works, but the text is lost, and a cut satin
column becomes a fill.

## Decision

- A lettering object can have a **knockout** (`Lettering::knockout`): an
  overlap in mm and the cut area in design coordinates (`area`). The
  objects that cut are all the objects that sew after the lettering and
  sew an area (`knockout::cuts_lettering`: fills, satins, appliqué and
  lettering; vector objects do not sew, so they do not cut). The letters
  stay as they are; only the stitches change. So the text, the font and
  the other settings stay open to change, and the cut follows each change.
- `Design::settle_knockouts` makes the area again after each edit
  (`Document::edit`, `try_edit` and `edit_merged` call it, as
  `settle_hoop`). The area is the union of the areas of the objects in
  front whose box touches the box of the lettering (lettering by its
  letters, `shaping::cut_area_of`), made smaller by the overlap. A cache of
  these objects and their revisions (`Design::knockout_seen`, not saved)
  stops the work when nothing changed, and the lettering changes only when
  its area changes, so the stitch cache keeps it.
- The stitches (`emb_stitch::knockout`) cut each part of each letter: a
  fill keeps its area outside the cut; a satin column stays a satin
  column. Its rungs are sampled each 0.15 mm; each rung keeps its longest
  part outside the cut, and a new column starts where a rung has no part
  or where its part does not touch the part of the rung before it. A part
  of a line outside the cut stays. A letter with no part left does not sew.
- **Remove Overlaps** uses lettering as a cutter. A lettering gets a
  knockout if it is under a selected cutter, or if it is selected and an
  object in front covers it (`Design::covered_in_front`). So the command
  works with the text in front selected, with both texts selected, and
  with only the text behind selected. **Flatten** first cuts the other
  selected objects (also by the selected lettering in front of them), then
  gives a knockout to each selected lettering that the results or the
  lettering in front of it cover. A link of Flatten or Remove Overlaps
  keeps the lettering as a cutter.
- The other shaping commands (Weld, Intersect and so on) do not use
  lettering; with too few other objects they say to use Break Apart first.
- Object Properties > Lettering > Knockout has a check box (Sew round
  objects in front) for each lettering, and the overlap.

The first version kept a list of the objects that cut, from the selection
of the command. The owner found that the knockout then missed text: with
both texts selected, Remove Overlaps cut nothing, and a text added in
front later did not cut. So all the objects in front cut now.

## Consequences

- A knockout is a setting of the lettering, always live. A Remove
  Overlaps without a link still keeps the lettering live; the other
  objects are cut once.
- An object put in front of a lettering with a knockout cuts it with no
  command. If an object in front must not cut, move it behind the
  lettering in the sew order, or turn the knockout off.
- The border trace and the cut of other objects use the letters before
  the knockout.
