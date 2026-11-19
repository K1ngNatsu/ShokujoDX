# Report: the 3D stitch view, Milestone 6 (the fabric)

Date: 2026-11-19 · The second part of P18 (`docs/design/p18-view-plan.md`),
ADR 0127. Items 1 to 5 are done. The exit test has its pictures and a
test; the compare with the two photos of the owner (the towel with and
with no topping) is open.

## What the milestone made

| Item | What there is now |
| --- | --- |
| 1. A surface for each fabric | `emb_render::fabric`: a relief of one repeat (about 16 mm, 512 texels on a side) with a height, a direction and a second color for each texel, for eight weaves (`emb_model::Weave`: plain, twill, knit, piqué, terry, fleece, felt, leather) at the size of the fabric preset (`weave`, `weave_mm` in `data/fabrics.toml`). The light of Milestone 4 lights it (`Fabric::shade`); in the default light its mean is the fabric color. |
| 2. Stitches sink in | On terry and fleece with no topping, the pile hides a thread where it is higher than the thread; threads press it down where they cover the area round a place. The topping of the fabric keeps all stitches on top; **Topping** in the Styles panel (`Design::no_topping`) says that the sew-out has none (the worksheet then shows none). |
| 3. Needle holes | A dent at each needle point, 0.12 mm (0.2 mm on felt, 0.4 mm and darker on leather and vinyl); a thread that lies over a hole does not show it. |
| 4. The fabric of the product | The product view, the turntable and the product page light the same relief (`product::light_factors`). Sweatshirts and hoodies are fleece now (they were felt). |
| 5. Two colors | `Design::fabric_mix` (in each colorway; **Second color** in the Threads panel): the weft of a woven fabric (a denim: a dark warp and a light weft) or a part of the fibers (a heather). |

The canvas draws the texture of the zoom level whose texels are as small as
the pixels of the screen (no mipmaps in the graphics stack), so the fabric
has its mean color at each zoom and no longer fades out. The 3D pictures of
today (`raster`) and the surface pictures use the same fabric.

## Exit test

### The towel with and with no topping

`view-towel` (navy lettering on a light blue towel: Block Sans 20 mm and
Round Sans 8 mm, the stitch settings of the fabric `towel`) at the three
zooms of the bench: `view-towel-<zoom>-surface` with the topping of the
fabric, `view-towel-<zoom>-no-topping` with none (`out/p18-view-m6`, made
by `emb-cli view-bench --out out/p18-view-m6`). With no topping the loops
cover the edges of the columns and the joins of the 8 mm letters (they look
thinner and broken), and the edges of the 20 mm letters are ragged; with
the topping both are clear.

The test `thin_stitches_sink_into_a_towel_with_no_topping` measures the
part of each lettering that threads cover: with no topping the pile hides
19 % of the 8 mm lettering and 18 % of the 20 mm lettering; with the
topping nothing. The 8 mm lettering is bold (its columns are as wide as
the area that presses the pile), so it loses about as much as the large
one, mostly at the edges. A line loses more: on terry with no topping a
run loses 84 % of its cover and the middle of a column 4 mm wide nothing
(test `a_pile_hides_a_line_more_than_a_wide_column`).

Open: the owner compares the two pictures with the two photos of the
sew-out (Milestone 1, item 5).

### Each fabric at three zooms

`fabric-<id>-<zoom>` (`out/p18-view-m6`): the lettering of `view-towel`
with the stitch settings and the topping of each fabric, at the whole
(2 pixels for each mm), the normal (8) and the close zoom (32):

| Fabric | Weave, size | Colors | Thread |
| --- | --- | --- | --- |
| cotton | plain, 0.25 mm | natural white | navy |
| denim | twill, 0.45 mm | indigo warp, white weft | gold |
| knit | knit, 0.7 mm | heather gray (two grays) | white |
| pique | piqué, 1.0 mm | navy | white |
| towel | terry, 1.0 mm | light blue | navy |
| fleece | fleece, 0.5 mm | charcoal | white |
| felt | felt, 0.4 mm | red | white |
| leather | leather, 0.7 mm | brown | cream |

What they show: the weave at the close zoom (the over and under of the
threads, the diagonal of the denim, the V loops of the knit, the round
cells of piqué, the loops of the towel, the pebbles of leather), a texture
of the weave at the normal zoom (the denim keeps its diagonal lines), and
the mean color at the whole zoom. The needle holes show as dark dents at
the ends of the columns on leather.

## Numbers

The development computer (32 processor threads), release build.

| What | Time |
| --- | ---: |
| A relief (one time for each fabric and size; the last four are kept) | 11 to 19 ms |
| Its light (each change of the light) | 5 ms |
| A texture of the canvas (a zoom level, 512 × 512 at most) | 4 ms |
| First view of 100,000 stitches on cotton: drafts, all tiles | 0.20 s, 0.43 s |
| The same on a towel with no topping | 0.22 s, 0.46 s |
| A change of the light, all tiles: cotton, towel | 0.23 s, 0.27 s |
| A change of one object in 200,000 stitches (lay, then tiles): cotton, towel | 0.10 s, 0.11 s |

The budgets of Milestone 5 still hold on cotton (the tiles have only the
needle holes there, with the same apron). On a fabric where stitches sink,
the apron of a block has the reach of the press of the pile (1 mm), so its
tiles take about 10 % longer.

The surface and its light for the pictures of the bench: the light of a
close picture (512 × 512) takes about 12 ms.

## What changed in the pictures

- All 78 pictures of the reference set changed: the fabric of the 3D look
  of today (the old brightness pattern is gone), and the fabric and the
  needle holes of the surface. 27 pictures are new (24 fabric pictures and
  the towel with no topping). The tests of the mean light of each area at
  each zoom and of the eight satin angles pass as before.
- The fabric in another light: a low light shows the relief more (test);
  in the default light the mean color of each fabric is its color (test).

## Limits

- A fabric of random loops or fibers repeats each 16 mm; at about 4
  pixels for each mm the repeat can show a little (the towel of the
  product view).
- The pile has no height of its own in the surface: it hides a thread,
  but it does not put a shadow on it.
- The second color of a heather is in thin streaks down the fabric, not
  along the yarn of each loop.
- Appliqué fabric, lace and the holes of cutwork are Milestone 7.

## Open

- The compare of the towel pictures with the photos of the owner.
- The numbers of a small computer (Milestone 1, item 4).
