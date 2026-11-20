# ADR 0129: The viewer of the 3D look

Date: 2026-11-19 · Status: Accepted (Milestone 8 of the second part of
P18, `docs/design/p18-view-plan.md`; the check of the owner is open)

## Context

After Milestone 7 the 3D look showed the surface of the stitches from
straight above, in one light that the user set with sliders. A person
who checks a sew-out turns it in the light, looks at it from an angle
and with a loupe, and compares it with the sew-out before a change. The
plan asks for these tools, for check views that use the surface, and for
the product view with the same surface and light.

## Decision

- **Two lights and the presets** (`thread3d::Lighting::two`,
  `Lighting::lights`, `LightPreset`): a light can have a second light at
  the same height from the opposite side with `SECOND_LIGHT` (half) of
  its light; the parts of the two lights add up to 1, so a flat top in
  the default height keeps the color of its thread. The diffuse light,
  the sheen, the glints, the facets of stones and the shadows (made for
  each light and added by their parts) use both. The presets are from
  the top left (the default), flat from above, low from the side (15°
  from the left) and two lights.
- **The light by hand** (`emb-app/src/viewer.rs`): with **L** held, a
  drag on the canvas, a split view or the tilt view puts the light at
  the pointer: from straight above in the middle of the view to 10° high
  at the edge of the largest circle in it, from the direction of the
  pointer. The canvas shows the circle and a sun while L is held. A text
  field that has the keys keeps the L.
- **The tilt view** (`emb_render::tilt`, `emb-app/src/tilt.rs`): a
  `Relief` is the picture of a surface in a light (`sheen::light`) and
  its heights; `project` shows it from a `Camera` (center, zoom, turn,
  tilt up to 75°, heights times 1 to 10). The projection is parallel:
  each column of the picture is one line across the turned relief, made
  from the front to the back with the highest row filled so far, so a
  high part hides what is behind it; the rows under the top of a point
  (its front face) are darker. The columns go to all processors. A
  thread makes the surface of the part of the design that the view can
  show (any turn), at the detail of the zoom (a power of two, at most 4
  million pixels), from the lay of the tiles (`TileCache::laid_for`).
  The light stays in the room when the design turns
  (`Lighting::turned`), so the relief is lit again for each turn; while
  the view moves, the picture has half the pixels on each side. View >
  Tilt View shows it on the canvas (the tools wait), View > Tilt Window
  in a window.
- **The magnifier**: a round window of 220 points at the pointer with
  the surface of its area at 20 times the real size, made in the frame
  from the lay of the tiles.
- **Compare**: View > Compare 3D shows the stitch view on the canvas and
  the 3D look in the split view; **Same Place** (`SplitView::follow`)
  gives a split view or a new view the place and zoom of the canvas, in
  both directions. View > Before and After keeps the plan, the design
  and the stitches of now (`viewer::BeforeAfter`, its own id in the tile
  cache); the canvas shows them on the left of a line that the user
  drags, and the design now on its right.
- **Check views** (`emb_render::check`, `emb-app/src/checks.rs`): a
  `CheckMap` of cells of 0.25 mm from a surface of the design (made with
  its detail and halved, so a cell has the mean of its pixels):
  - the part of the fabric that shows through an area, `1 − cover` (the
    holes of cutwork do not count), the mean over 0.75 mm, only in the
    areas of fills of rows and satins (`check::areas`), 0.5 mm in from
    their edges. Fills that are open by design are not checked: motifs,
    stipple, cross stitch, tartans and gradients, accordion spacing (a
    blend), a jagged edge, the hand-stitch effect and rows 1 mm or more
    apart. The limit is 25 % (Options);
  - the height of the thread over what is not thread
    (`Laid::floor_in`: the pieces, the padding and the foam on all the
    length of a puff stitch), limit 1.2 mm (Options). `LaidStrand` keeps
    the foam of its strand for it.
  A place is connected cells over the limit, 0.5 mm² or more. A thread
  makes the map for each new lay; the canvas colors the cells (magenta:
  the fabric shows; red: high thread) and counts the places.
- **The product and the turntable** (`product::stitch_surface`,
  `surface_onto`): with the 3D look, the stitches on the drawn product,
  in the picture on the product, on the sides of the turntable and the
  other designs on the canvas are their surface in the light of the
  product (`sheen::overlay` over the product: the threads, and their
  shadows and gaps on it). The flat look keeps the soft shadow.

## Consequences

- The tilt view of the stress design of 100,000 stitches turns at 22 ms
  for each frame (45 frames in a second: a new light and a picture of 640
  × 400 pixels) and tilts at 1.6 ms (the picture only) on the development
  computer (`tilt::tests::tilt_budget`); the surface of the view takes
  53 ms one time.
- The reference set has a picture in a low light from the side at the
  close zoom (`<design>-close-low`) and a tilted picture
  (`<design>-tilt`) of each design. The pictures before do not change:
  with one light the light is the same as before.
- The corpus designs have no place where the fabric shows through but
  the rows of contour, offset and Florentine fills that open at a sharp
  turn (`curved-fills`) and a few small places of `effects`; the high
  thread is over 1.2 mm in lace, in `effects` (objects on top of each
  other) and at a crossing in a few letters.
- Lettering is not in the check of the fabric that shows through (its
  letters are not fill or satin objects of the design).
- Open: the owner checks a design with the low light and the tilt, finds
  a place where the fabric shows through, and closes it with the density
  (the plan's exit test; the program's part of it is the test
  `checks::tests::a_place_where_the_fabric_shows_closes_with_more_density`).
