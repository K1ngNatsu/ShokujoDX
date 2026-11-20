# ADR 0130: All pictures from the surface, and the quality of the view

Date: 2026-11-19 · Status: Accepted (Milestone 9 of the second part of
P18, `docs/design/p18-view-plan.md`; the test with users is open)

## Context

After Milestone 8 the canvas, the tilt view and the product showed the
surface of the stitches, but the other pictures of the program drew each
stitch alone (`raster::picture_look`): the design picture, `emb-cli
output`, the approval sheet, the library and the file manager, the kiosk,
the wizard and the font previews. The plan also asks for an export for
web shops, a quality setting for slow computers, a decision about the
graphics card, a help topic and an audit of the part.

## Decision

- **One engine for the pictures** (`emb_render::picture`): an
  `Embroidery` is the laid stitches of a design (with the foam, the cut
  lines and the pieces: `of_design`), of a machine file (`of_stitches`)
  or of stitch segments (`of_segments`). `Embroidery::picture` makes a
  picture of any area in blocks of 256 pixels on all processors
  (`Laid::block_from`: a grid of pixels from any corner, with the apron of
  the tiles, so a pixel is the same in each block and the memory is
  small). The blocks are lit with the fabric (`sheen::light_on`), over a
  plain color (`sheen::overlay`, so the shadows fall on it) or clear: then
  the threads and their shadows and gaps as a dark layer on the clear
  background, so the picture has them on any background.
  `detail` is the least number of pixels for each mm that the surface is
  made with: `FULL_DETAIL` (16, as the canvas) or `FAST_DETAIL` (4).
- **Who uses it**: the PNG design picture of Output Design, of `emb-cli
  output` and of Share (`picture::design_picture`; the flat look keeps
  `raster`); the approval sheet and the quote (`SheetData::picture`, an
  image in the box on a clear background; the PDF writer gives a picture
  with a clear part a soft mask, `pdf::jpeg_and_mask`, so the picture lies
  on the color of the box with no edge; the production worksheet keeps the
  stitches as lines for the operator); the library and the thumbnails of the file manager
  (`emb_library::preview::picture`, at `FAST_DETAIL`); the kiosk (the
  tiles of the canvas); the stitch pictures of Picture to Design; the
  preview of the Fonts window (the letters sewn). The product page and
  the turntable use the surface since ADR 0129.
- **Export**: the PNG design picture at 150, 254, 300 or 600 dots for
  each inch (`OutputProfile::picture_dpi`, `emb-cli output --dpi`), with
  no background (`clear_picture`, `--clear-picture`), and the sew-out
  animation (`sewout_gif`, `--sewout-gif`): 40 pictures of the design
  with the stitches up to each step (`Embroidery::sewing`, the records
  that `show` keeps), a GIF of 480 pixels at most.
- **Quality** (`viewtools::Quality`): **Fast** is the look of the
  stitches one by one (no tiles, `LookSettings::surface` off), **Good**
  is the surface with no shadows from the heights
  (`Lighting::shadows` off: the finish of a block, the light step and the
  apron leave them out), **Best** is all of it. The pictures have the
  shadows at each quality. At the first start of a release build the
  program lays a design of 20,000 stitches and makes two tiles of it for
  each thread of the tile cache, on those threads at one time (so the
  threads of a core, the memory and a laptop that slows down count), and
  divides the time by the number of tiles. Best is under 20 ms, Good
  under 50 ms, else Fast: the first full view of 100,000 stitches takes
  about 0.12 s for each ms (the development computer: 4.0 ms; on 4 of its
  cores 7.6 ms, on 2 cores 13.3 ms). With the software renderer it chooses
  Fast (`startup::is_software`). It chooses one time
  (`LookSettings::measured`); the user can change it in the Thread Look menu.
- **The graphics card: not now.** The plan leaves the decision to the
  owner after the measures. On the development computer the processor
  meets each budget of the plan (a change of the light: 16 ms for the
  screen; the tilt view: 45 frames in a second; a picture of 4000 × 3592
  pixels of 100,000 stitches: 0.54 s), and a second way to light the
  surface would need a test that the two ways give the same pictures. The
  question stays open until the numbers of a small computer are there
  (Milestone 1, item 4): if the light step is too slow there, a shader can
  light the same surface (a paint callback of the UI library).
- **Audit**: the index of a lay has at most 4 million squares (larger
  squares for a larger design): a design 1 km wide asked for a terabyte
  before. The lays of the tile cache have a budget of 256 MB
  (`LAY_BUDGET`, the lay that no view used for the longest time goes
  first), as the tiles have (`BUDGET`). A thread weight of 0 is clamped
  (`thread::width_factor`), and an object larger than 3 m does not sew
  (`emb_stitch::MAX_OBJECT_MM`).
- **Help**: the topic *The 3D view* (`docs/help/3d-view.md`, in the four
  languages): what the view shows, the kinds of thread, the light, the
  checks, the quality and the pictures.

## Consequences

- The pictures of the program look as the canvas does. A design picture
  of 4000 × 3592 pixels of the stress design of 100,000 stitches takes
  0.54 s (0.12 s of it the lay; the budget of the plan is 2 s), a
  thumbnail of 256 pixels at the fast detail 0.18 s.
- The settings have `look.measured`; a settings file of before measures
  one time.
- "Fine surface" is now the quality (Good or Best); the help and the
  translations say so.
- Ten open designs of 100,000 stitches that all show in one minute keep
  234 MB of tiles and 222 MB of lays (five lays); a design whose lay went
  out is laid again (about 0.13 s) when it shows.
- Open: the test with users of the plan (twelve pairs of a photo of a
  sew-out and a picture), the numbers of a small computer, and a start
  with the software renderer on a computer with no graphics driver.
