---
id: arranging
title: Arranging and the sew order
group: design
keywords: arrange, rotate, turn, mirror, flip, skew, slant, reference points, center, align, space evenly, distribute, same size, copies, arrangement, mirror merge, array, wreath, kaleidoscope, reflect, group, ungroup, lock, unlock, sew order, sequence, color changes, shortest order, cap order, closest join, click order
---
# Arranging and the sew order

The **Arrange** tab moves and copies objects, and sets the order in which
they sew.

## Transform

- **Rotate 90° Left**, **Rotate 90° Right** (Ctrl+R) and **Rotate 180°**
  turn the selection.
- **Mirror Left-Right** (H) and **Mirror Up-Down** (V) mirror it.
- **Center in Hoop** moves the selection to the hoop center.
- **Mirror Line**: click two points of a line; the selection mirrors
  about it. Hold **Ctrl** to keep the original.
- **Ref Points**: click two points on the selection, then the places
  where they go. The selection moves, turns and scales to fit.
- **Rotate** turns the selection by the angle in the field next to it
  (clockwise). **Skew** slants it by the X and Y angles.

To turn all of the design in the hoop, use **Design > Rotate Design**.

## Align, space and size

Select two or more objects. The first selected object is the one that
the others follow.

- **Align**: **Left**, **Center**, **Right**, **Top**, **Middle** and
  **Bottom** edges.
- **Space and Size**: **Across** and **Down** space three or more objects
  evenly. **Same Width**, **Same Height** and **Same Size** give the
  selection the size of the first selected object.

## Copies

- **Arrangements** opens a window that makes copies of the selection:
  - **Mirror-merge**: a mirror copy to the right, below, or both, with a
    gap.
  - **Array**: rows and columns of copies, with a gap across and down.
  - **Wreath and kaleidoscope**: copies round a center below the
    selection, at a radius. A kaleidoscope has a mirror copy in each
    part.
- **Mirror Right**, **Mirror Below** and **Mirror Both** add mirror copies
  at once.
- **Reflect**: click two points of a line; a mirror copy of the selection
  is added.

## Group and lock

- **Group**: a click on one object selects all of the group. A
  double-click on an object of a group selects that object alone, for
  example to change its stitch type; a click on an object outside the
  group selects groups as a whole again. When a group is selected, the
  Properties panel also has a button for each object of the group.
  **Ungroup** takes the selection out of its groups.
- **Lock**: locked objects cannot be selected or changed. **Unlock All**
  unlocks them.

## The sew order

The **Sequence** panel shows the objects in the order that they sew. An
object that sews later lies on top. Drag objects in the panel, or use the
buttons at its top.

- **To Start**, **Back One**, **Forward One** and **To End** move the
  selection in the order.
- **Back One Color** and **Forward One Color** move it past the objects
  of one color.
- **By Click Order**: Ctrl+click objects in the order you want, then
  click this.

## Fewer color changes and trims

Each color change stops the machine (or changes the needle), and each
trim takes time.

- **Fewer Color Changes** merges blocks of one color where no overlapping
  object is in the way. What is on top stays on top. The Sequence panel
  has the same command (**Merge colors**).
- **Group by Color**: the fewest color changes. It can put an object
  below one that covered it, so check the design after.
- **Shortest Order** sews the objects of each color nearest first, so the
  needle does not go to and fro: less travel and fewer trims. The colors
  and what is on top do not change. The status line tells how many trims
  and how much travel it saved.
- **Closest Join**: each object ends at the point nearest to the next
  one, which starts there. Short gaps then get travel stitches, not trims.

## Caps

A cap moves on its frame when the stitches pull it. **Cap Order** sews
the objects of each color from the center out and from the bottom up,
and lettering from the middle letter out, so the cap fabric does not
move. With a cap frame as the hoop, Design info warns when the order is
not so.

## The start and end points

The stitches of a fill or a satin depend on where the needle is when the
object starts, so a change of the sew order can change them a little.
Reshape shows the start (green) and the end (red) of the selected
object; drag them to change where it starts and ends. **Design > Hoop Options > Auto start and
end** sets where the needle goes before the first stitch and after the
last one.
