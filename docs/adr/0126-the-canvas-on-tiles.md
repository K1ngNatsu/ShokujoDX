# ADR 0126: The canvas on tiles

Date: 2026-11-19 · Status: Accepted (Milestone 5 of the second part of
P18, `docs/design/p18-view-plan.md`; the numbers of a small computer are
open)

## Context

Milestones 3 and 4 (ADR 0124, 0125) made the surface of the stitches and
its light for the pictures of the bench. The canvas still made a mesh of
all visible stitches for each frame (ADR 0040), so its cost grew with the
stitches, and it did not have the new look. The plan asks for a cache of
tiles in the app, made by threads in the background, that a change of
the design makes again only where it changed, and that all views use.

## Decision

- **One grid of pixels for each zoom level.** Level `L` has 2^L pixels for
  each mm (levels −4 to 7); pixel `x` has its center at `(x + 0.5) / ppm`
  mm. A tile is 256 × 256 pixels of its level; its texture has one pixel
  of the tiles round it on each side (258 × 258), so the tiles join with
  no seam. A view takes the level nearest to its pixels for each mm.
- **A block of pixels** (`Laid::block`) is made as `Laid::surface_in`
  makes a surface (16 pixels for each mm or more, halved; at most 4
  halvings, so a level under 1 pixel for each mm has less detail), with
  an apron round it as wide as the longest shadow of the design in its
  light (`shadow_reach`, at most 4 mm) and the gaps. So each pixel is the
  same in each block that has it (a test). A block of more than 768
  pixels of its detail on a side is made in parts. The light of a block
  for the canvas is `sheen::overlay`: the threads in the part of each
  pixel that they cover, and a dark layer that gives the fabric under
  them its gaps and shadows (colors of the screen, premultiplied).
- **The lay on fixed cells.** The cell of the grid of the lay is
  `floor(p / cell)` (not from the corner of the design), and the cell is
  0.1 mm times a power of two, so a change of the design keeps the cells.
- **A lay again after a change** (`surface::relay`): the strands before
  the first changed strand and after the last one are found by a number
  of each strand (`key`, not its record). The changed strands are laid
  again, and each strand after them is laid again only if a changed
  strand is in a square of 1 mm under it; it changes only if its heights
  change. The result is the same as a new lay (a test). `Changes` gives
  the areas that changed (squares of 2 mm).
- **The cache** (`emb-app/src/stitch_tiles.rs`): the tiles of all open
  designs and all views by design, lay, light, shown strands, level and
  place, in a budget of 256 MB (ADR 0077); the tiles that were not used
  for the longest time go first. An empty tile has no texture.
- **Threads**: all processors but two (at most 12) make the jobs in a
  queue that each frame makes again: the tiles of the level of the screen
  from the middle of the view, then the level under it, then the level
  over it for the middle of the view. From level 5 a job makes 2 × 2
  tiles (the apron is large for a small tile). A lay is made by its own
  thread, one at a time for each design.
- **Until a tile is there** a view shows a part of a tile of a smaller
  zoom (up to three levels), the four tiles of the larger zoom, or the
  tile in the light before a change of the light; else the look of today
  (the meshes), cut to the places with no tile.
- **A change** keeps the tiles that it does not reach for the new lay or
  the new strands that show. A tile that it reaches keeps its texture
  with a hole at the change (the look of today shows there) and the
  hole is made again in parts of at most 256 pixels of the detail, on all
  threads, and written into the texture (`TextureHandle::set_partial`).
  While the lay of a new plan is not there, the tiles near the objects
  whose stitches differ from the plan of the lay show the look of today.
- **The stitch player** shows the tiles up to a step of 1,000 records and
  the stitches after the step as today (full up to the cursor, dim after
  it). **A drag** keeps the lay that it started with, the objects that
  move are not in the tiles and show as today, and the lay comes after
  the drag. **The view filter**: the hidden strands are not drawn, but
  the shown ones keep their heights on them (the hidden ones still sew).
- **All views use the cache**: the canvas, the split view, the views of
  View > New view, the other design (its own lay) and the overview. The
  selection, needle points, jumps, functions, the density map and the
  other marks are drawn over the tiles as before.
- **A setting**: `LookSettings::surface` (View > Thread Look > Fine
  surface), on by default; off, the 3D look is the meshes of today. The
  tests of the app have it off unless they ask for it.
- **Speed of the engine**: the direction of the light is computed one
  time for each block (not for each pixel), and a shadow line stops where
  it is above the highest point of the squares of 16 pixels in front of
  it (`Tops`; the same result).
- **The speed pass** (the owner, 2026-11-19, after the first measures):
  - **Quick pictures** show until a tile is made, as provisional entries
    of the cache. Where no tile of another level can show (a first view),
    a draft made with half the detail (`DRAFT_PX_PER_MM`, 8 pixels for
    each mm). After a change of the light, a picture from the surface of
    the tile and of the tiles round it (`sheen::overlay_around`, the
    shadows from the heights of its level): the cache keeps the surfaces
    of the tiles (96 MB at most, the oldest go first).
  - The levels under and over the screen are made when all tiles of the
    screen are there (they take as much work as the screen).
  - A change reaches as far as the shadows of the design in its light and
    of the lay before (`surface::apron_for`), in squares of 1 mm.
  - `relay` copies the old grid one time and makes the index of the new
    lay on a second thread; the plies are two parabolas (no call of the
    math library for each pixel); a stitch tests only the pixels of the
    rows that it reaches.

## Consequences

- The numbers of the development computer (32 processor threads, 12 tile
  threads; `docs/design/p18-view-m5-report.md`): with the graphics card
  the canvas pans and zooms 787,408 stitches at 117 frames in a second
  (23 with the 3D meshes) and 106,576 at 740 (182). After the speed pass
  the first view of 100,000 stitches shows drafts in 0.20 s (budget
  0.3 s; all tiles 0.43 s, the lay 0.13 s of it), new tiles after a change
  of an object of 2,000 stitches in 200,000 take 0.10 to 0.11 s (budget
  0.1 s), and a new light shows in 0.02 s (budget 0.1 s; all tiles
  0.23 s). The tiles of a view use about 100 to 140 MB with their
  surfaces. The software renderer gains less: a frame with almost nothing
  in it takes about 67 ms.
- A first view and a new light change a little when the exact tiles come
  (a draft is within 3 % of its tile in mean light, a picture in a new
  light within 0.1 %; tests).
- All 80 corpus designs show all their tiles at three zooms when the work
  is done (test `every_corpus_design_has_all_its_tiles`).
- The cells of the lay moved to fixed places: 37 surface pictures of the
  bench changed by a fraction of a pixel; the closest two of the eight
  satin columns are 0.8 % apart now (the test asks 0.5 %). The plies of
  the speed pass changed all 39 surface pictures a little.
- Open: the numbers of a small computer; the lay is on one thread (0.9 s
  for 1,000,000 stitches).
- Changed on 2026-11-20 (owner report: the stitch player was jumpy, the
  3D look came seconds after the needle, and a rectangle of the flat look
  showed for a frame at each step). The step was 1,000 records: at 200
  stitches a second the tiles were up to 5 s behind the needle, and a
  tile that a step reached showed the look of today in a rectangle until
  it was made again (in 28 of 470 frames at 2,000 stitches a second, up
  to 56 % of the view). Now a step goes to the cursor when the tiles of
  the step before are made in the canvas (`Scene::lead`; at most
  `HOLD_MAX`, 0.5 s), and a tile that a step forward reaches keeps its
  picture until the new part is there, with the new stitches as today
  over it (`Entry::lags`); a step back goes at once, as before. The
  measure `player_budget` (release, ignored): at 200 stitches a second
  the tiles are 0.02 s behind the needle on average (0.07 s at most), at
  2,000 stitches a second 0.14 s (at most `HOLD_MAX`), and no frame
  shows the look of today in a rectangle.
- Changed on 2026-11-20, the second owner report: after the light by
  hand and a drag of the player slider back and forth, the view was flat,
  low in detail and slow for about a minute after the close of the player.
  Each step of the slider is a new variant. A tile that many small changes
  reached part by part was made again in up to 81 parts for each step (the
  check of `PATCH_MAX` looked at the new part only), and the threads gave
  thousands of tiles and parts of variants that no view showed any more.
  The frame took 24 of them at most and gave each full tile to the screen,
  so the tiles of the view waited behind them. Now the check is on all the
  parts that are not made yet (`patch_of`), and a finished tile of a
  variant that no view shows (`TileCache::alive`) is not kept; a frame
  takes up to `TAKES` (4,096) such results and gives at most `UPLOADS`
  tiles to the screen. On the owner's design (26,568 stitches) all tiles
  were there 21 s after the close, now 0.5 s (`player_slider_budget`,
  release, ignored).
