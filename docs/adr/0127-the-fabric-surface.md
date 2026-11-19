# ADR 0127: The fabric surface

Date: 2026-11-19 · Status: Accepted (Milestone 6 of the second part of
P18, `docs/design/p18-view-plan.md`; the compare with the photos of the
owner is open)

## Context

The 3D look showed the fabric as a pattern of brightness that repeated
each 4 mm (`thread3d::fabric_value`, P10). It had no height and no
direction, so the light did not change it, the size of the weave did not
come from the fabric, and stitches lay on it as on paper. The plan asks
for a surface of each fabric that the light of Milestone 4 lights, for
stitches that sink into a pile unless a topping keeps them up, for needle
holes, for the same fabric on the product, and for fabrics of two colors.

## Decision

- **One relief for each fabric** (`emb_render::fabric::Fabric`): one
  repeat of about 16 mm (`REPEAT_MM`) that repeats with no seam, in 512 ×
  512 texels (about 32 for each mm). Each texel has the mean height, the
  mean direction of the surface (shorter than 1 where it changes), the
  part of the second color and the light that the low places let in; each
  is the mean of 2 × 2 samples of a pattern. The patterns use no function
  of the math library, so a relief has the same texels on each system.
  The repeat is a whole number of units of the pattern (a multiple of 4,
  for a twill), and the noise repeats in it.
- **Eight weaves** (`emb_model::Weave`): plain weave and twill (warp and
  weft go over and under; the warp of a twill is dense, with the twist of
  its fibers, and its diagonal goes up to the right), knit (columns of
  V-shaped loops), piqué (raised round cells), terry (three loops in each
  cell, open rings on mounds), fleece and felt (noise), leather (pebbles
  between fine creases). Fleece is a weave of its own (before, it was
  felt): it has a pile.
- **The fabric presets** have `weave` and `weave_mm` (the size of the
  weave: the distance from one thread to the next, or the size of a loop,
  a cell or a grain; `Fabric::weave_size`). A design with no fabric has a
  plain weave of 0.25 mm.
- **Its light** (`Fabric::shade` → `Shade`): the diffuse light of the
  threads (`sheen::diffuse`, ADR 0125) times the light of the low places,
  and a soft gloss on leather, divided by its mean in the default light.
  So in the default light the mean color of a fabric is its color (the
  background), and another light makes it lighter or darker as it makes a
  flat thread. A `Shade` keeps zoom levels (means of 2 × 2 texels) and
  gives the mean light of a square of any size (`Shade::at`,
  `Shade::color`). The canvas, the surface pictures (`sheen::light`), the
  pictures of the 3D look (`raster`, `Look::fabric` is a `FabricLook`) and
  the product view (`product::light_factors`) all use it.
- **The canvas** draws the texture of the zoom level whose texels are as
  small as the pixels of the screen or smaller (`fabric::level_for`; the
  graphics stack has no mipmaps), made when that level shows; the light
  makes the levels again. It does not fade out at a small zoom any more:
  the levels have the mean.
- **Two colors** (`Design::fabric_mix`, in each colorway): the weft of a
  plain weave or a twill (a denim: a dark warp and a light weft), else a
  part of the fibers in thin streaks (a heather). The Threads panel sets
  it under the background color.
- **Needle holes**: each stitch makes a dent at both needle points
  (`Fabric::hole`: 0.12 mm and 30 % darker in the middle; 0.2 mm on felt;
  0.4 mm and 75 % on leather and vinyl, where the hole of the needle
  stays). The dent darkens only a place that is lower than 0.15 mm
  (`HOLE_DEPTH_MM`): the fabric and the thread that goes into the hole,
  not a thread that lies over it. It is in `Tile::open`, so the halving
  keeps it.
- **A pile** (terry 1.6 mm, fleece 0.9 mm, `Fabric::pile_mm`): where no
  topping keeps the stitches up (`Ground::sinks`), the pile hides a
  thread where it is higher than the thread (up to 85 %, `PILE_HIDE`, in
  0.25 mm, `PILE_SOFT_MM`). Threads press the pile down by the part of the
  area in 1 mm round a place (about a loop, `PILE_RADIUS_MM`) that they
  cover: not at all under 35 % (`PILE_PRESS_FROM`), up to 90 % lower under
  full cover (`PILE_PRESS`). So a run and the edges of a column sink, a
  wide column keeps its middle, and a fill under the stitches (a laydown)
  lifts and presses them. The topping is the topping of the fabric
  preset unless the design says that the sew-out has none
  (`Design::no_topping`, the Topping check box of the Styles panel; the
  worksheet then shows no topping). The pile is in the finish of a block
  (made with its detail), so the zoom levels keep it.
- **The surface knows its fabric** (`fabric::Ground`, an argument of
  `Laid::block`, `block_with` and `surface_on`); the tiles of the canvas
  have it in their variant (`Ground::key`). With no fabric shown (no
  background, or Fabric texture off), the tiles have no ground: the
  needle holes of a plain fabric, no pile. Where stitches sink, the apron
  of a block also has the reach of the press (1 mm in place of the 0.3 mm
  of the gaps; `surface::apron_for` has the ground).
- **The bench** (`emb-cli view-bench`): the surface pictures are on the
  fabric of the design; `view-towel` also has its pictures with no
  topping (`-no-topping`); `fabric-<id>-<zoom>` puts the lettering of
  `view-towel` on eight fabrics (`view::FABRICS`) with the stitch settings
  of each fabric.

## Consequences

- All pictures of the reference set changed (the fabric in the 3D look
  of today, and the fabric and the needle holes in the surface); 27
  pictures are new. The mean brightness of each area is still the same
  within the limits of the tests at each zoom.
- On a terry pile with no topping a run loses 84 % of its cover and the
  middle of a column 4 mm wide nothing (test). The 8 mm lettering of
  `view-towel` is bold, so the pile hides about a fifth of it, as of the
  20 mm lettering: the edges of the columns and the joins (test).
- On the development computer a relief takes 11 to 19 ms (one time for
  each fabric and size; the last four are kept), its light 5 ms, a
  texture of the canvas up to 4 ms. The tiles of a plain fabric take as
  long as before; on a towel with no topping the first view of 100,000
  stitches shows in 0.22 s and has all tiles in 0.46 s (0.20 and 0.43 s
  on cotton).
- A random fabric (terry, fleece, felt, leather) repeats each 16 mm; at a
  zoom of about 4 pixels for each mm the repeat can show a little.
- Open: the compare with the photos of the owner (the towel with and
  with no topping); appliqué fabric, lace and the holes of cutwork are
  Milestone 7.
