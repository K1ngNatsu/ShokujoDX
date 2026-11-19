# ADR 0125: The light on the surface, and its zoom levels

Date: 2026-11-19 · Status: Accepted (Milestone 4 of the second part of
P18, `docs/design/p18-view-plan.md`; the owner's compare with photos is
open)

## Context

The surface of Milestone 3 (ADR 0124) has the heights of the stitches;
its first light was the light of the 3D thread look of today on its
normals. A sew-out is light or dark by the angle of its stitches to the
light, has shadows and darker gaps, and the same mean brightness at each
distance. The plan asks for sheen by direction, shadows from the heights,
less light in the gaps, the twist of the plies, zoom levels made from the
surface, smooth edges and a tone curve.

## Decision

- **Zoom levels** (`Surface::half`): a surface for a zoom of less than
  16 pixels for each mm is made at 16 or more (at most 8 million pixels)
  and halved. A pixel of a level has the mean height and cover of the
  four pixels under it; over the part that threads cover it has the mean
  color (in linear light), slope, gaps, plies, diffuse light and shadow
  of the threads, and the mean of their directions as the vector of two
  times their angle (shorter where the directions differ: the spread);
  over the rest it has the light of the fabric. The material is the one
  that covers most. Threads and fabric have separate means: at a small
  zoom the dark gap beside a thin bright thread must not darken the
  thread.
- **The detail of each pixel** (`Tile`, 20 bytes for each pixel): the
  height, the normal (the mean direction of the surface), the color, the
  thread direction, the material, the cover, the gaps (a pixel lower than
  the mean of the heights 0.3 mm round it is up to 35 % darker at
  0.15 mm lower), the plies (a pattern along the thread, 0.35 mm a turn),
  and, for the light of the surface (`Surface::light`), the diffuse light,
  the shadow and the light of the fabric. Each thread gets a small change
  of its width (±6 %) and brightness (±4 %) from the place of its stitch.
- **Light-dependent values at the detail.** The shadow (a line to the
  light over the heights; soft over 0.08 mm; at most 4 mm long) and the
  diffuse light are made at the detail for the light of the surface and
  halved with the rest, so a small zoom keeps the shadows of the threads
  and the dark sides of their round sections. A picture in another light
  makes them from the heights of its level (less exact at a small zoom);
  `Laid::surface_in` makes a surface for another light.
- **The light** (`emb_render::sheen`):
  - Diffuse: 0.62 + 0.537 × (normal · light), so a flat top in the
    default light (45° high) has the color of its thread.
  - Sheen: a highlight on fibers, the power of the sine of the angle
    between the fiber and the light (seen from above). The fibers on top
    of a thread are 35° from it (the twist of its plies). Strength and
    power by kind: polyester 0.30 and 6, rayon 0.40 and 8, metallic 1.2
    and 20 (in its own color), glow 0.18 and 4, cotton 0.03 and 2, wool
    0.02 and 2. Where the directions spread, the highlight goes toward
    its mean over all directions.
  - The light is linear (`sheen::light_values`); the threads mix with
    the fabric in the part of the pixel that they cover; then the tone
    curve and the colors of the screen (`Light::picture`). The tone curve
    acts only above white and keeps the ratio of the three colors, so a
    highlight does not turn the thread white.
- **The fabric** is averaged over the pixel at a small zoom (samples at
  most 0.12 mm apart): one sample in a pixel as large as the weave gave
  the same part of the weave in each pixel, 4 to 8 % too light or dark.
- **The compare** (`emb_render::compare`) takes the brightness of each
  area of the second picture from its own pixels when it fills the frame
  (a sample between a bright and a dark pixel mixes the numbers of the
  screen and is too dark), and `compare::smaller` and `Light::smaller`
  make a picture smaller in linear light, for exact tests of the levels.

## Consequences

- The exit test: before the tone curve, the mean light of each area of a
  4 × 4 grid of each design of the reference set at 8, 4 and 2 pixels for
  each mm is within 1.8 % of the picture at 16 pixels for each mm made
  smaller (test `the_zoom_levels_keep_the_light`, all thirteen designs).
  After the tone curve, all are within 3 % except the white dense fill of
  `view-fill-over-fill` (5.3 %): its highlights are above white at the
  detail and are cut there, not in the mean of a small zoom.
- The eight satin columns have eight values, at least 1.6 % apart (test).
  With a twist of 20° to 30° two of them were within 0.5 %, at 15° or 35°
  they are 1.6 % apart; the photo of the sew-out decides the twist and the
  strength of the sheen.
- The surface of 100,000 stitches at 10 pixels for each mm and its light
  take 0.29 s; the light of 3.6 million pixels in the light of the surface
  takes 73 ms.
- The canvas and the pictures of the program do not use the surface yet
  (Milestones 5 and 9).
