# Report: the 3D stitch view, Milestone 5 (the canvas on tiles)

Date: 2026-11-19 · The second part of P18 (`docs/design/p18-view-plan.md`),
ADR 0126. Items 1 to 6 are done. Of the budgets, pan and zoom are met; the
first view, a change of one object and a change of the light are slower
than their budgets (see below). The numbers of a small computer are open.

## What the milestone made

| Item | What there is now |
| --- | --- |
| 1. A tile cache | `emb-app/src/stitch_tiles.rs`: the tiles of all open designs and views by design, lay, light, shown strands, level and place; 256 MB, the tiles not used for the longest time go first. |
| 2. Threads in the background | One thread lays each new plan (`surface::relay` after a change); up to 12 threads make the tiles: the level of the screen from the middle, then the level under it, then the level over it. Until a tile is there: a tile of another level, else the look of today. |
| 3. A change of the design | The tiles that the change does not reach stay; in the others only the part that changed is made again (a hole with the look of today until then). A drag shows the moving objects as today and the tiles for the rest. |
| 4. The stitch player | The tiles up to a step of 1,000 records, the stitches after it as today. |
| 5. The other marks | Drawn over the tiles as before (the jumps too). |
| 6. Views | The canvas, the split view, the views, the other design and the overview use the cache. |

A new setting, **View > Thread Look > Fine surface** (on), gives the look
of today in the 3D look when it is off.

## Exit test

### No missing tile after the work is done

Test `every_corpus_design_has_all_its_tiles` (release): all 80 corpus
designs, in a view of 1,200 × 900 pixels, at the zoom that fits and at 4
and 16 times it. Each view had all its tiles when the threads were done.
The time until all tiles of the level of the screen showed:

| Zoom | Median | 90 % of the designs | Largest |
| --- | ---: | ---: | ---: |
| The design fits | 121 ms | 188 ms | 416 ms |
| 4 times | 30 ms | 52 ms | 64 ms |
| 16 times | 96 ms | 185 ms | 322 ms |

The whole test takes 45 s.

### The budgets of the plan

The development computer: 32 processor threads, an NVIDIA GeForce RTX
5070 Ti. The synthetic designs of Milestone 1 (`emb_render::stress`,
about 200 × 180 mm).

**Pan and zoom** (`ShokujoDX --view-bench`, a window of 1,280 × 720, the
view moves and zooms for 3 s after the first full view):

| Stitches | 3D meshes (today) | Surface on tiles | Frames with all tiles |
| ---: | ---: | ---: | ---: |
| 10,656 | 1,455 frames/s | 3,225 frames/s | 96 % |
| 106,576 | 192 frames/s | 826 frames/s | 91 % |
| 787,408 | 24.5 frames/s | 122 frames/s | 60 % |

The budget (60 frames in a second with 1,000,000 stitches) is met. A frame
of a pan with all tiles made needs 0.04 ms of work on the processor. The
software renderer: 10.4 frames in a second with 10,656 stitches (4.0 with
the 3D meshes), about 1 with more stitches; a frame with almost nothing in
it takes about 67 ms with this renderer, so its budget (30 frames in a
second) is not in reach of any look.

**The work of the tiles** (test `tile_budgets`, a view of 1,920 × 1,080):

| What | Budget | Measured |
| --- | ---: | ---: |
| First full view, 10,000 stitches | | 145 ms (lay 15 ms) |
| First full view, 100,000 stitches | 0.3 s | 0.39 s (lay 0.13 s) |
| First full view, 1,000,000 stitches | | 1.9 s (lay 0.88 s) |
| New tiles after a change of one object (2,000 stitches) in 200,000 | 0.1 s | 0.13 s (lay 0.06 s) |
| A change of the light, 100,000 stitches | 0.1 s | 0.24 s |
| Memory of the tiles of one view (with the levels under and over it) | 256 MB for all | 15 to 47 MB |

The look of today shows until the first full view; a change shows as
today where it is until its tiles are there; after a change of the light
the tiles in the light before show until the new ones are there. So the
canvas answers at once each time; the budgets are for the new look.

### Why the budgets that are not met are not met

The time of a tile is mostly in its pixels: a tile of the whole view
(64 mm at 4 pixels for each mm) is made at 16 pixels for each mm, about
1,000,000 pixels. The densest tile of the 100,000 stitches (test
`block_steps`, one processor) takes 0.26 s: the stitches into the pixels
105 ms, the slopes, gaps and shadows 59 ms, the halving 25 ms, the light
6 ms. The first measure was 786 ms for the first view; these changes made
it 0.39 s:

- The direction of the light was computed for each pixel two times (with
  the slow trigonometry of `emb_geom::math`); now one time for each block.
- A shadow line stops where it is above the highest point of the squares
  of 16 pixels in front of it (the same shadows).
- The apron of a block is the longest shadow of the design, not 4 mm.
- 12 tile threads in place of 6.
- A change makes again only the part of a tile that it reaches, in small
  parts on all threads (`TextureHandle::set_partial`): 0.47 s → 0.13 s.

What is left to choose (the owner, before or in Milestone 9):

1. **A change of the light** needs all tiles again, because the shadows are
   made at the detail (ADR 0125). Keep the surfaces of the tiles of the
   screen and light them again (fast, the shadows less exact for a moment),
   or the light step on the graphics card (Milestone 9, item 4).
2. **The first view** is near its budget; the lay (0.13 s, one thread) is a
   third of it.
3. **The small computer**: its numbers are open (`ShokujoDX --view-bench`
   and `--renderer=software` on it).

## What changed in the pictures

- The cells of the lay are at fixed places now (`floor(p / cell)`), so a
  change of the design keeps them: 37 surface pictures of the bench
  (`view_bench__pictures.snap`) changed by a fraction of a pixel; the
  pictures of the look of today did not change.
- Of the eight satin columns, the two that go toward the light (90° and
  112.5°) are 0.8 % apart now (1.6 % before); their values are near the
  weakest part of the highlight, so a small change of the cells moves
  them. The test asks 0.5 %; the twist of the fibers (35°) did not change
  (a scan of 33° to 42° gave pairs from 0.5 % to 1.6 % apart). The photos
  set the twist.

## Pictures

`cargo test --release -p emb-app -- --ignored write_tile_pictures` puts
the tiles of three views together as the canvas shows them
(`target/tmp/tiles`): no seams at the joins of the tiles. The shadow of a
puff column has small steps at its edge: they are the ends of its
stitches (0.4 mm apart), not the tiles.

## Open

- The numbers of a small computer (Milestone 1, item 4).
- The budgets that are not met (see above).
- The photos of the sew-out of the reference set (Milestones 1 and 4).
