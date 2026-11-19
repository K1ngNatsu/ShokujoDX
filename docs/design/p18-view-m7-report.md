# Report: the 3D stitch view, Milestone 7 (what is not thread)

Date: 2026-11-19 · The second part of P18 (`docs/design/p18-view-plan.md`),
ADR 0128. Items 1 to 7 are done. The exit test has its reference designs
and their pictures; the compare with the sew-outs of the owner (appliqué,
puff, cross stitch, rhinestones on a transfer) is open.

## What the milestone made

| Item | What there is now |
| --- | --- |
| 1. Appliqué fabric | A piece 0.3 mm thick with the relief of its weave and a cut edge (lower in its last 0.15 mm), under its cover. The weave is a setting of the appliqué object (**Fabric surface** in the appliqué window; twill if not set). |
| 2. Puff | The foam rises round in 0.8 mm after the needle points, so a column is round across; the cap stitches close the ends lower. |
| 3. Rhinestones | At their real size, 0.9 of their radius high: a flat table and eight facets. Glass in the light: less diffuse light, a sharp highlight on a facet that faces the light, and the light of the foil at the back on the facets on the far side. |
| 4. Cutwork and lace | The cut lines of the cutting needles are joined again and cut holes (a slit for an open line); they are not thread. A hole shows what is behind the fabric (a dark gray; transparent in a picture with no fabric). Lace has no fabric: no fabric texture, no shadow on it. |
| 5. Cross stitch | A ninth weave, Aida (blocks with a hole at each corner), and the fabric preset **Aida cloth (cross stitch)**. A grid of 1.814 mm with its corners at the holes puts the needle points in them. |
| 6. Trapunto and quilting | A fill with trapunto has padding under it (a dome 1.5 mm high). The fabric preset **Quilt (top, batting and backing)** has a loft of 2 mm: the fabric rises round in 3 mm from the nearest thread, so it is higher between the lines of stitches. |
| 7. Metallic and glow thread | Metallic thread has small glints of its foil at 6 pixels for each mm and more, which change with the light. **View > Thread Look > Lights off** shows a dark room: only glow thread shows, in its own light (with the fine surface). |

A thread is now round across from the height of what is under it (a piece
or padding), not from the fabric: its edges stay on top of them.

## Exit test

The reference set (`emb_cli::view::REFERENCE`) has a design for each item;
`emb-cli view-bench --out out/p18-view-m7` makes each at three zooms (the
whole design, 8 and 32 pixels for each mm), and the test
`tests/view_bench.rs` keeps the hash of each picture:

| Item | Reference design | Pictures |
| --- | --- | --- |
| 1. Appliqué fabric | `view-applique` (a blue circle with a navy cover) | `view-applique-<zoom>-surface` |
| 2. Puff | `view-puff` (the word UP and a bar) | `view-puff-<zoom>-surface` |
| 3. Rhinestones | `view-rhinestones` (new: crystal SS10, SS16 and SS20 in rows, gold SS8 in a ring, on a black knit) | `view-rhinestones-<zoom>-surface` |
| 4. Cutwork, lace | `view-cutwork` (new: a circle and a leaf cut in white cotton, navy satin over the edges), `view-lace` (new: the lace ornament on water-soluble stabilizer) | `view-cutwork-<zoom>-surface`, `view-lace-<zoom>-surface` |
| 5. Cross stitch | `view-cross-stitch` (new: a heart on Aida, two half crosses) | `view-cross-stitch-<zoom>-surface` |
| 6. Trapunto, quilting | `view-quilting` (new: a trapunto heart and echo lines 6 mm apart on the quilt fabric) | `view-quilting-<zoom>-surface` |
| 7. Metallic, glow | `view-thread-kinds` (a metallic and a glow column) | `view-thread-kinds-<zoom>-surface`, `view-thread-kinds-<zoom>-lights-off` |

What they show: the stones have a bright table and one bright facet each,
and shadows on the knit; the holes of the cutwork are dark inside their
satin; the lace ornament has nothing between its threads; the crosses lie
in the grid of the Aida with their corners in its holes; the heart of the
quilt stands up over its padding and the batting puffs up in round
channels between the echo lines; with the lights off only the glow column
shows.

Open: the owner compares appliqué, puff, cross stitch and rhinestones on a
transfer with their sew-outs (the plan names these four).

## Numbers

The development computer, release build. The surface and its light for the
pictures of the bench, ms:

| Design | Stitches | Lay | Surface, normal | Light, normal | Surface, close | Light, close |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| view-applique | 839 | 2.4 | 19.7 | 9.7 | 11.8 | 11.2 |
| view-rhinestones | 75 | 0.7 | 19.8 | 10.1 | 9.9 | 11.5 |
| view-cutwork | 1,323 | 2.7 | 14.4 | 8.8 | 15.7 | 11.1 |
| view-lace | 12,905 | 24.1 | 47.5 | 10.8 | 15.6 | 6.2 |
| view-cross-stitch | 217 | 0.3 | 7.9 | 7.2 | 9.5 | 11.3 |
| view-quilting | 1,041 | 3.6 | 54.3 | 12.7 | 15.7 | 11.2 |

The batting of a quilt costs most (a distance from the threads in each
block, and a wider apron). The tiles of the stress designs (no pieces) take
as long as before (`tile_budgets`, below).

**The tiles of the canvas** (test `tile_budgets`, the synthetic designs on
woven cotton): the first view of 100,000 stitches shows drafts in 0.20 to
0.21 s and has all tiles in 0.43 s, as after Milestone 6; the work of the
pieces, of the floor under the threads and of the slope of the fabric is
done only where there are pieces or a fabric with a height.

## What changed in the pictures

- The reference set has 33 new pictures (five new designs at three zooms
  in two looks, and three pictures with the lights off).
- Of the 105 pictures before, 8 changed: the surface of `view-applique`
  at the three zooms (the weave of the appliqué fabric, the edges of the
  threads on it), of `view-puff` at the three zooms (round over the foam)
  and of `view-thread-kinds` at the normal and the close zoom (the glints
  of metallic thread). The 3D look of today and the fabric pictures of
  Milestone 6 did not change.
- The light of each area at each zoom level stays within 3 % (the test of
  Milestone 4), but for cross stitch on Aida: 4 %. The crosses cover the
  middles of the blocks of the cloth, so the fabric that shows is the
  darker grooves; at a small zoom a pixel has the mean of all of its
  fabric.

## Limits

- A hole shows a flat dark color; on the canvas it does not show the hoop
  or the product under the fabric.
- The glow of glow thread does not light the fabric round it.
- Lights off is only in the fine surface (the meshes of today do not have
  it).
- The fabric of the program has no grid of its own: the Aida lines up with
  a grid of crosses only where the grid starts at a hole (the reference
  design does).
- At a small zoom cross stitch on Aida is up to 4 % lighter (see above).
- A pad is the shape of the fill; a trapunto area that is not a fill (an
  outline with stuffing) has no padding.

## Open

- The compare with the sew-outs of the owner (appliqué, puff, cross stitch,
  rhinestones).
- The numbers of a small computer (Milestone 1, item 4).
