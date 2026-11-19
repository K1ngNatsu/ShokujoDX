# Report: the 3D stitch view, Milestone 3 (the surface)

Date: 2026-11-19 · The second part of P18 (`docs/design/p18-view-plan.md`),
ADR 0124. Items 2 to 6 are done; item 1 is done without the role of
each stitch (see below).

## What the milestone made

| Item | What there is now |
| --- | --- |
| 1. Strands | `emb_render::strand`: each stitch of the plan with its thread and the foam under it (puff). No role: the plan does not mark underlay, and the heights come from the sew order. |
| 2. A stitch as a shape | A flat oval (0.55 of its width thick); it rises from its needle points in 0.35 mm; a short stitch is an arch. |
| 3. The height field | `surface::lay`: in sew order on a grid of 0.1 mm; a stitch lies on what it crosses as a taut thread; thread presses thread (1 / (1 + 0.15 × layers)). |
| 4. The shape of a column | From the taut thread: over underlay a column is round, its edges go down steeply to the needle points; underlay makes it higher. |
| 5. Tiles | `Laid::surface`: tiles of 256 × 256 pixels on all processors; the same bytes on each system and in each part of an area. |
| 6. The first light | `surface::light`: the light of today on the normals of the surface, over the fabric in linear light. |

## Exit test

**Heights** (test `the_surface_has_the_heights_of_a_sew_out`, mm):

| Design | Place | Height |
| --- | --- | ---: |
| view-fill-border | the border (highest) | 0.85 |
| view-fill-border | the fill (highest in the middle) | 0.41 |
| view-underlay | satin, no underlay (mean of the middle) | 0.21 |
| view-underlay | satin, two layers of underlay | 0.63 |
| view-underlay | fill, no underlay / two layers | 0.17 / 0.33 |
| view-fill-over-fill | the dense fill on top / the fill under it | 0.56 / 0.27 |
| view-puff | the column (mean) / the highest point | 2.28 / 2.72 |
| view-satin-angles | a column (mean of the middle) | 0.54 |
| all other designs | the highest point | 0.49 to 1.14 |

A satin of 40 weight polyester is 0.4 mm wide and 0.22 mm thick; a
sew-out of the set can confirm the heights (a caliper or a photo from the
side).

**Speed** (`emb-cli view-bench --stress`, release build, 32 threads):

| Stitches | Lay | Tiles at 10 px/mm | Light at 10 px/mm | 3D picture of today at 8 px/mm |
| ---: | ---: | ---: | ---: | ---: |
| 10,656 | 16 ms | 6 ms | 17 ms | 239 ms |
| 106,576 | 136 ms | 16 ms | 22 ms | 1.7 s |
| 787,408 | 952 ms | 98 ms | 22 ms | 12.3 s |

The surface of 100,000 stitches at 10 pixels for each mm is ready in
0.17 s (budget: 1 s). The lay is in sew order on one processor and is
the largest step. Its data: 19 MB for 10,000 stitches (mostly the grid
of the area), 41 MB for 100,000, 195 MB for 800,000. A tile at 10 pixels
for each mm keeps 10 bytes for each pixel (35 MB for the whole stress
design).

## What the pictures show

The bench writes a second picture of each design at each zoom
(`<design>-<zoom>-surface.png`; 78 pictures with those of today).

- A satin looks the same in each sew direction: the fault of Milestone 2
  is gone (each stitch has its height, not the order of the paint).
- Underlay shows through as small bumps on the column and on the fill
  with two layers of underlay; a tatami fill shows its needle points.
- Puff letters stand up with steep sides; the large letters at the zoom
  of the whole design have a body with light and shade.
- At the zoom of the whole design the threads keep their colors: the
  part of a pixel that threads cover is the union of the parts of each
  thread. A first version took the largest part of one thread, and a
  dense satin was too light (navy looked gray).

**Not yet**: no shadows, the same sheen in each direction of a thread
(the satin columns at eight angles are all as bright), and the normals at
2 pixels for each mm come from the heights 0.5 mm apart. These are the
items of Milestone 4.

## Two models that did not work

The first model lifted a stitch from each high point under it with a
slow slope (0.12 mm for each mm) and looked at three places across it.
In the curves of letters, satin stitches near the inner edge lay on the
one sewn two before, and the parts of a split satin lay on each other;
each new stitch was higher, and the 60 mm letters were 14.5 mm high
(puff 39.7 mm, because the foam was added on top of each puff stitch
under it). The taut thread held at the fabric at both needle points, the
middle line only, and the foam as a floor make all heights plausible.

## Open

- A role for each stitch (underlay, top, travel): it needs the stitch
  engine to mark its runs. No step needs it yet.
- The lay of very large designs: one processor and 195 MB for 800,000
  stitches. Milestone 5 (the tile cache with a budget of memory) needs a
  smaller form, for example the heights of fewer points for a long
  stitch.
- The program does not show the surface yet (Milestone 5 for the
  canvas, Milestone 9 for all pictures).
- The sew-out of the reference set (Milestone 1, item 5) is still open.
