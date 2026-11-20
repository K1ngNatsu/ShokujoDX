# Report: the 3D stitch view, Milestone 8 (the viewer)

Date: 2026-11-19 · The second part of P18 (`docs/design/p18-view-plan.md`),
ADR 0129. Items 1 to 6 are done. The speed part of the exit test passes;
the check of the owner is open.

## What the milestone made

| Item | What there is now |
| --- | --- |
| 1. Tilt and turn | **View > Tilt View** shows the embroidery from an angle on the canvas (the tools wait until **Close**); **View > Tilt Window** shows it in a window next to the canvas. A left drag tilts (0° to 75°) and turns, a right or middle drag moves, the wheel zooms, a double-click fits. **Heights** makes the heights 1 to 10 times larger. The picture comes from the surface of the part of the design that the view can show, with the processor; while the view moves, it has half the pixels on each side. |
| 2. The light by hand | Hold **L** and drag on the canvas, a split view or the tilt view: the light comes from the pointer (straight above in the middle of the view, 10° high at the edge). Presets in **View > Lighting** and the Thread Look menu: from the top left (the default), flat from above, low from the side, two lights (a second light from the opposite side with half the light). The light stays in the room when the design turns in the tilt view. |
| 3. A magnifier | **View > Magnifier**: a round window of 220 points at the pointer with the thread at 20 times its real size. |
| 4. Compare | **View > Compare 3D**: the stitch view on the canvas and the 3D look in the split view; **Same Place** gives a split view or a new view the place and the zoom of the canvas. **View > Before and After** keeps the stitches of now; after a change, a line that you drag shows them on its left and the design now on its right. |
| 5. Check views | **View > Fabric Shows Through** marks the places of a fill or a satin where more than 25 % of the fabric shows, from the laid threads (their widths, heights and gaps). **View > High Thread** marks the places where the thread is more than 1.2 mm high over the fabric (foam, padding and pieces do not count). The limits are in Options; a line on the canvas counts the places. |
| 6. Product and turntable | With the 3D look, the stitches on a drawn product (the picture on the product, the sides of the turntable, the other designs on the canvas) are their surface in the light of the product: heights, sheen, and their shadows and gaps on the product. |

## Exit test

The plan: *the owner checks a design with the low light and the tilt,
finds a place where the fabric shows through, changes the density and
sees the place closed; the tilt view of a design with 100,000 stitches
moves at 20 frames in a second or more on the development computer.*

**Speed** (test `tilt::tests::tilt_budget`, release build, the stress
design of 100,000 stitches in a view of 1280 × 800 pixels):

| Step | Time |
| --- | ---: |
| The lay (one time; the canvas keeps it) | 130 ms |
| The surface of the view (784 × 704 pixels at 4 px/mm, one time) | 53 ms |
| A frame while the design turns (the light again, a picture of 640 × 400) | 22 ms (45 frames in a second) |
| A frame while the view tilts (the picture of 640 × 400) | 1.6 ms |
| The still picture (1280 × 800) | 3.9 ms |

The tilt view moves at 45 frames in a second or more: the speed part
passes.

**The place that closes**: the test
`checks::tests::a_place_where_the_fabric_shows_closes_with_more_density`
does what the owner does in the program: a fill with its rows 0.9 mm
apart shows the fabric (a place with more than 30 % open); with the rows
0.4 mm apart, the check finds no place. Open: the owner does it with a
design of their own, with the low light and the tilt.

## Numbers

The check map of the corpus designs (cells of 0.25 mm; a surface of the
design at 4 px/mm, made at 16 px/mm and halved) takes 5 to 140 ms on a
thread. With the default limits:

- **Fabric shows through**: only `curved-fills` (124 places of 1 to 2 mm²,
  up to 70 % open: the rows of contour, offset and Florentine fills open
  up at sharp turns) and `effects` (7 small places). Fills that are open
  by design are not checked (see ADR 0129); before this rule, the blends
  and the textures of `texture-fills` showed hundreds of places.
- **High thread**: the highest thread of the corpus designs is 0.4 to
  1.0 mm in most designs, up to 1.3 mm at the crossings of satin letters,
  1.5 mm in lace and 1.8 mm in `effects` (objects on top of each other).
  Over 1.2 mm: `view-lace` and `lace-ornament` (7), `effects` (15),
  `curved-fills` (2), `lace-edge` (3), one letter of `embroidery-fonts-4`.

**The tiles of the canvas** (`tile_budgets`): the first view of 100,000
stitches shows drafts in 0.21 s and has all tiles in 0.44 s, as after
Milestone 7 (with one light the light step does the same work).

## What changed in the pictures

- The reference set has 36 new pictures: each design in a low light from
  the side at the close zoom (`<design>-close-low`) and tilted 55°,
  turned 20°, with the heights four times larger (`<design>-tilt`).
- The 105 pictures before did not change.
- The pictures of the products (the picture on the product, the
  turntable) change with the 3D look: the stitches are the surface (no
  test keeps their hash).

## Limits

- The tilt view is a parallel projection of a height field: an overhang
  (a thread over a gap) is solid, and the front face of a high part has
  the color of its top, darker. With the heights made larger, the plies
  of a satin look like ridges.
- When the design turns, the light is made again for the whole surface;
  a design much larger than the stress design turns more slowly (the
  surface has at most 4 million pixels).
- Lettering is not in the check of the fabric that shows through (its
  letters are not fill or satin objects of the design), and the check
  does not know the pull of the fabric.
- The magnifier is made in the frame from the lay: on a slow computer a
  move of the pointer over a dense design can take some milliseconds.
- Before and After compares with the stitches when it started; to
  compare with a later state, turn it off and on again.

## Open

- The check of the owner with a design of their own (the exit test).
- The compare of Milestones 4, 6 and 7 with photos and sew-outs.
- The numbers of a small computer (Milestone 1, item 4).
