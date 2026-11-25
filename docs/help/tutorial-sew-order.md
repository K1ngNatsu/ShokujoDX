---
id: tutorial-sew-order
title: "Tutorial: sew order, trims and travel"
group: tutorials
path: drawing
time: 15
level: practice
before: tutorial-logo-by-hand
keywords: tutorial, sew order, sequence, order, trims, trim, jumps, travel, color changes, fewer color changes, group by color, shortest order, hidden travel, trim after, sequence panel, to start, to end
---
# Tutorial: sew order, trims and travel

The machine sews the objects in the order of the design. Each change of
color stops a single-needle machine, and each trim cuts the thread and
takes time. In this tutorial you make a design sew with fewer color
changes and less travel, and you learn where the trims come from.

## Open the start design

1. Click [Open the start design](sample:sew-order). It has six dots: red,
   blue, red, blue, in that sew order.
2. The status line shows **Colors: 6** and **Trims: 6**. The machine
   changes the thread five times.
3. Click **View > Show > Jumps**. The lines between the dots show where
   the machine travels from one object to the next.

![Six dots in a bad sew order, with the jumps](sew-order-before)

## Fewer color changes

Click **Arrange > Sew Order > Fewer Color Changes**. The red dots now sew
one after the other, then the blue dots. The status line shows
**Colors: 2**: the machine changes the thread one time.

The status line still shows **Trims: 6**. The dots are far apart, so
the machine trims the thread after each dot: a long jump thread would
show on the fabric. Objects that touch, or a hidden travel (below), sew
with no trim between them.

**Fewer Color Changes** keeps an object on top of an object that it
covers. **Group by Color** makes the fewest color changes, but it can
put an object under an object that covered it.

## A shorter way

Click **Arrange > Sew Order > Shortest Order**. The program finds the
order and the start and end points with the shortest travel, and keeps
the colors together. The message in the status line shows the trims and
the mm of travel before and after.

![The dots in the shortest order](sew-order-after)

## More ways to change the order

- **Arrange > Sew Order > To Start**, **Back One**, **Forward One** and
  **To End** move the selected objects in the sew order.
- **By Click Order** sets the order: click the objects in the order to
  sew them.
- The **Sequence** panel lists the objects in sew order, by color. Drag
  an object up or down. The **Panels** menu on the **View** tab shows the
  panel.

## Travel instead of a trim

Between two objects of the same color, the machine can trim, or travel
with a run under the stitches of a later object. A hidden travel saves
the time of a trim, and it does not show. Select an object, and in
Object Properties under **Machine Functions** set **Trim after** to
**Hidden travel**. Use it only where a later object covers the way from
this object to the next.

## Check it

- **View > Stitch Cursor > Stitch Player** plays the sew-out: look at the
  jumps between the objects.
- **Design Info** shows the trims and the color changes.

## If it does not look right

- **An object that was on top is under now.** Click **Undo**, and use
  **Fewer Color Changes** in place of **Group by Color**.
- **A trim shows on the fabric.** Move the start point of the next
  object nearer to the end of the one before it, or use a hidden travel.

## Next

- [Tutorial: a name on an arc](tutorial-lettering)
- [Arranging](arranging)
