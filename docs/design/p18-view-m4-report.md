# Report: the 3D stitch view, Milestone 4 (the light on the surface)

Date: 2026-11-19 · The second part of P18 (`docs/design/p18-view-plan.md`),
ADR 0125. Items 1 to 7 are done; the compare with photos and the owner's
answer for each picture are open (no sew-out yet).

## What the milestone made

| Item | What there is now |
| --- | --- |
| 1. Sheen by direction | `emb_render::sheen`: a highlight on fibers; the fibers are 35° from the thread (the twist of the plies); strength and width by kind. |
| 2. Shadows from the heights | A soft shadow along the line to the light, made at the detail for the light of the surface and halved. The black copy of each stitch is only in the look of today. |
| 3. Less light in the gaps | A pixel lower than the heights round it is darker (up to 35 %). |
| 4. Plies and changes | A pattern along each thread (0.35 mm a turn); ±6 % width and ±4 % brightness for each thread, from its place. |
| 5. Zoom levels | `Surface::half`: means of the level under it, threads and fabric apart, with the spread of the directions. |
| 6. Smooth edges | The cover of each pixel; at a small zoom the mean of the detail. |
| 7. The bright end | A tone curve above white that keeps the ratio of the colors. |

## Exit test

**The eight satin columns** (`view-satin-angles`, mean light of the
middle of each column at 8 pixels for each mm; light from the top left):

| Column angle | Light | Order |
| ---: | ---: | ---: |
| 0° | 0.435 | 1 |
| 22.5° | 0.421 | 2 |
| 157.5° | 0.379 | 3 |
| 45° | 0.364 | 4 |
| 135° | 0.315 | 5 |
| 67.5° | 0.302 | 6 |
| 112.5° | 0.297 | 7 |
| 90° | 0.292 | 8 |

Eight values, at least 1.6 % apart (test
`the_eight_satin_angles_have_eight_brightness_values`). The order of the
photo is open: the owner sews the set and takes the photo in the light
of the plan (one light from the top left, the camera straight above).
The twist of the fibers (35°) and the strength of the sheen are the two
values that the photo sets; a twist of 20° to 30° gave two columns
within 0.5 %.

**The zoom levels** (test `the_zoom_levels_keep_the_light`): the light of
each design at 16 pixels for each mm, made smaller in linear light,
against the levels halved from it, on a grid of 4 × 4 areas. The largest
difference of an area, before and after the tone curve:

| Design | 8 px/mm | 4 px/mm | 2 px/mm | 2 px/mm after the tone curve |
| --- | ---: | ---: | ---: | ---: |
| view-satin-angles | 0.0 % | 0.0 % | 0.0 % | 0.0 % |
| view-fill-border | +0.3 % | +0.2 % | +0.2 % | +0.2 % |
| view-underlay | −0.2 % | −0.6 % | −0.9 % | −0.9 % |
| view-lettering | 0.0 % | 0.0 % | 0.0 % | 0.0 % |
| view-fill-over-fill | −0.5 % | −1.3 % | −1.8 % | +5.3 % |
| view-puff | −0.3 % | −0.3 % | −0.3 % | −0.3 % |
| view-applique | −0.2 % | −0.2 % | −0.4 % | −0.4 % |
| view-motif-fill | 0.0 % | +0.1 % | 0.0 % | +0.2 % |
| view-run | 0.0 % | 0.0 % | 0.0 % | 0.0 % |
| view-blend | −0.2 % | −0.6 % | −1.5 % | −1.3 % |
| view-towel | 0.0 % | 0.0 % | 0.0 % | −0.1 % |
| view-dark | +0.6 % | +1.0 % | +1.1 % | +2.8 % |
| view-thread-kinds | 0.0 % | 0.0 % | 0.0 % | +0.1 % |

The light is within 3 % everywhere. After the tone curve the white dense
fill differs by 5.3 %: at the detail its highlights are above white and
are cut pixel by pixel; at a small zoom their mean is not cut.

**The compare with photos**: open (no photos). The bench writes the
pictures to compare (`<design>-<zoom>-surface.png`); `emb-cli
view-compare` puts each one next to its photo.

## How the zoom levels came to agree

The first measure gave 4 to 13 % differences at 2 pixels for each mm. In
order:

1. **The weave** (−6 to +8 % on plain fabric): one sample of the weave
   in a pixel of 0.5 mm, the size of the pattern, took the same part of
   the pattern in each pixel. Now the fabric is the mean of samples at
   most 0.12 mm apart.
2. **The shadows** (up to +5 %): they were made at each level from the
   mean heights, so the shadows of single threads went away. Now they
   are made at the detail and halved.
3. **The compare itself** (up to −13 % on the dark fabric): it sampled
   the small picture between its pixels, which mixes the numbers of the
   screen. Now it measures each picture on its own pixels, and the test
   makes the large picture smaller in linear light.
4. **The light of threads and fabric apart**, the diffuse light at the
   detail, and the tone curve only above white.

## Speed and memory

| Stitches | Lay | Surface at 10 px/mm (made at 20, halved) | Light |
| ---: | ---: | ---: | ---: |
| 10,656 | 16 ms | 50 ms | 71 ms |
| 106,576 | 141 ms | 76 ms | 76 ms |
| 787,408 | 990 ms | 216 ms | 73 ms |

The surface of 100,000 stitches with its light: 0.29 s (budget 1 s). The
light of 3.6 million pixels: 73 ms (budget for a change of the light at
1920 × 1080: 0.1 s); this is with the light of the surface; another light
makes the shadows at the level and takes longer. A surface keeps 20
bytes for each pixel (the plan expected about 12).

## What the pictures show

The pictures of the reference set (`out/p18-view-m4`, 78 pictures):

- The strokes of the 60 mm letters and the two sides of the border on
  the fill are lighter or darker by the angle of their stitches; the bowl
  of the "g" changes round its curve.
- At the close zoom the plies show as a twisted cord.
- Puff letters and the puff column have a soft shadow to the bottom
  right; the fill under the dense fill shows in the gaps.
- The highlight of polyester on navy and red threads is strong (the navy
  border shows a light blue side). The photos set its strength.

## Open

- The photos of the sew-out of the reference set (Milestone 1, item 5),
  the compare of each picture with its photo, and the owner's answer for
  each one ("like the sew-out", "near", "not like it").
- The light step in another light than that of the surface makes the
  shadows at the level; the canvas (Milestone 5) needs a plan for a
  change of the light (make the shadows at the detail again, or keep the
  detail level).
- 20 bytes for each pixel and the lay of very large designs (Milestone 5,
  the budget of memory).
