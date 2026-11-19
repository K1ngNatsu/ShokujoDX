# Report: the 3D stitch view, Milestone 5 (the canvas on tiles)

Date: 2026-11-19 · The second part of P18 (`docs/design/p18-view-plan.md`),
ADR 0126. Items 1 to 6 are done. After a speed pass (see below) the budgets
of the development computer are met, with a quick picture first for the
first view and for a change of the light; a change of one object is at its
budget. The numbers of a small computer are open.

## What the milestone made

| Item | What there is now |
| --- | --- |
| 1. A tile cache | `emb-app/src/stitch_tiles.rs`: the tiles of all open designs and views by design, lay, light, shown strands, level and place; 256 MB, the tiles not used for the longest time go first. |
| 2. Threads in the background | One thread lays each new plan (`surface::relay` after a change); up to 12 threads make the tiles: the level of the screen from the middle, then (when all of them are there) the level under it and the level over it. Until a tile is there: a draft, a tile of another level, else the look of today. |
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
| The design fits | 124 ms | 207 ms | 482 ms |
| 4 times | 32 ms | 56 ms | 68 ms |
| 16 times | 92 ms | 179 ms | 327 ms |

The whole test takes 56 s. (With the speed pass the levels under and over
the screen are made after the screen; the times are those of the exact
tiles, not of the drafts.)

### The budgets of the plan

The development computer: 32 processor threads, an NVIDIA GeForce RTX
5070 Ti. The synthetic designs of Milestone 1 (`emb_render::stress`,
about 200 × 180 mm).

**Pan and zoom** (`ShokujoDX --view-bench`, a window of 1,280 × 720, the
view moves and zooms for 3 s after the first full view):

| Stitches | 3D meshes (today) | Surface on tiles | Frames with all tiles | First view: drafts, all tiles |
| ---: | ---: | ---: | ---: | ---: |
| 10,656 | 1,645 frames/s | 3,190 frames/s | 95 % | 0.10 s, 0.24 s |
| 106,576 | 182 frames/s | 740 frames/s | 86 % | 0.24 s, 0.56 s |
| 787,408 | 23.3 frames/s | 117 frames/s | 40 % | 1.7 s, 2.9 s |

(The numbers after the speed pass. Before it: 826 and 122 frames in a
second, and 91 % and 60 % of the frames with all tiles; now the levels
under and over the screen come after the screen, so a moving view finds
fewer of them. The tiles use 127 to 136 MB with the surfaces that they
keep for a change of the light.)

The budget (60 frames in a second with 1,000,000 stitches) is met. A frame
of a pan with all tiles made needs 0.04 ms of work on the processor. The
software renderer: 10.4 frames in a second with 10,656 stitches (4.0 with
the 3D meshes), about 1 with more stitches; a frame with almost nothing in
it takes about 67 ms with this renderer, so its budget (30 frames in a
second) is not in reach of any look.

**The work of the tiles** (test `tile_budgets`, a view of 1,920 × 1,080):

| What | Budget | Milestone 5 | After the speed pass |
| --- | ---: | ---: | ---: |
| First full view, 10,000 stitches | | 145 ms | drafts 75 ms, all tiles 175 ms |
| First full view, 100,000 stitches | 0.3 s | 0.39 s | **drafts 0.20 s**, all tiles 0.43 s (lay 0.13 s) |
| First full view, 1,000,000 stitches | | 1.9 s | drafts 1.1 s, all tiles 1.9 s (lay 0.90 s) |
| New tiles after a change of one object (2,000 stitches) in 200,000 | 0.1 s | 0.13 s | **0.10 to 0.11 s** (lay 0.05 s) |
| A change of the light, 100,000 stitches | 0.1 s | 0.24 s | **shown 0.02 s**, all tiles 0.23 s |
| Memory of the tiles of one view | 256 MB for all | 15 to 47 MB | 94 to 97 MB (with the surfaces) |

The look of today shows until the first full view; a change shows as
today where it is until its tiles are there; after a change of the light
the tiles in the light before show until the new ones are there. So the
canvas answers at once each time; the budgets are for the new look.

### Where the time goes

The time of a tile is mostly in its pixels: a tile of the whole view
(64 mm at 4 pixels for each mm) is made at 16 pixels for each mm, about
1,000,000 pixels. The densest tile of the 100,000 stitches (test
`block_steps`, one processor) took 0.26 s in Milestone 5: the stitches into
the pixels 105 ms, the slopes, gaps and shadows 59 ms, the halving 25 ms,
the light 6 ms. Now 0.23 s (the plies, below). The lay of 100,000 stitches
takes 0.12 s on one thread; two thirds of it is to put each stitch on the
grid.

In Milestone 5 (786 ms → 0.39 s for the first view):

- The direction of the light was computed for each pixel two times (with
  the slow trigonometry of `emb_geom::math`); now one time for each block.
- A shadow line stops where it is above the highest point of the squares
  of 16 pixels in front of it (the same shadows).
- The apron of a block is the longest shadow of the design, not 4 mm.
- A change makes again only the part of a tile that it reaches, in small
  parts on all threads (`TextureHandle::set_partial`): 0.47 s → 0.13 s.

### The speed pass

The owner asked for a short speed pass before Milestone 6 (2026-11-19).

| Change | What it gave |
| --- | --- |
| **Quick pictures in a new light.** The cache keeps the surface of each tile that shows (96 MB at most, about 70 tiles), and a change of the light lights it again from its surface and the surfaces round it (`sheen::overlay_around`: the shadows from the heights of its level) until the tile is made again. | The new light shows in 11 to 30 ms (0.24 s before). Its mean light is within 0.1 % of the tile (test). |
| **Drafts.** Where no tile of another level can show (a first view), a draft with half the detail (8 pixels for each mm, a quarter of the work) shows first. | The first view of 100,000 stitches shows in 0.20 s (0.39 s before); all tiles 0.43 s. A draft is within 3 % of its tile in mean light (test). |
| **The levels under and over the screen** are made when all tiles of the screen are there (they take as much work as the screen). | The screen gets all threads. With 20 or 30 threads in place of 12 the first view was slower (the threads shared the cores). |
| **A change reaches as far as its shadows**: the apron of the design in its light (and of the lay before), in squares of 1 mm (2 mm before). | About a quarter fewer pixels to make again. |
| **The lay again** (`surface::relay`): the old grid is copied one time, not three times; the index of the new lay and the copy of the grid are made on a second thread; the index is one array. | 59 → 45 ms for a change in 200,000 stitches. |
| **The plies** are two parabolas in place of the sine of the math library. | A dense tile 254 → 227 ms. All 39 surface pictures of the bench changed a little; the satin values stayed (the nearest two 0.84 % apart). |
| The pixels of a stitch are only those of the rows that it reaches (not all of its box). | The same pixels; a diagonal stitch has fewer pixels to test (the synthetic design has level rows, so no change there). |

What is left:

1. **The lay** is on one thread (0.13 s of the first view of 100,000
   stitches, 0.9 s of 1,000,000). Parallel strips of the design could make
   it faster (the stitches of a strip lie only on the stitches of their
   strip); a stitch across two strips makes them wait for each other.
2. **The small computer**: its numbers are open (`ShokujoDX --view-bench`
   and `--renderer=software` on it).
3. A change of the light and a first view show quick pictures first: the
   exact tiles follow 0.2 to 0.4 s later (a small change of the light and
   the detail when they come).

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
- A faster lay (see above), if the first view of large designs needs it.
- The photos of the sew-out of the reference set (Milestones 1 and 4).
