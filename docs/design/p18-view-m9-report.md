# Report: the 3D stitch view, Milestone 9 (all pictures, quality, audit)

Date: 2026-11-19 · The second part of P18 (`docs/design/p18-view-plan.md`),
ADR 0130. Items 1, 2, 3, 5 and 6 are done; item 4 (the graphics card) is
decided: not now. The test with users of the exit test is open.

## What the milestone made

| Item | What there is now |
| --- | --- |
| 1. All pictures | `emb_render::picture` makes each picture of the program from the surface, in blocks on all processors: the design picture (Output Design, `emb-cli output`, Share), the approval sheet and the quote, the library and the thumbnails of the file manager (at the fast detail), the kiosk (the tiles of the canvas), the stitch pictures of Picture to Design and the preview of the Fonts window (the letters sewn). The production worksheet keeps the stitches as lines for the operator. |
| 2. Export | The PNG design picture at 150, 254, 300 or 600 dots for each inch, with the fabric color or with no background (for a web shop), and a sew-out animation (GIF, 40 steps). The same in `emb-cli output` (`--dpi`, `--clear-picture`, `--sewout-gif`). |
| 3. Quality | Fast (the stitches one by one), Good (the surface with no shadows from the heights) and Best, in the Thread Look menu. A release build chooses it one time at the first start from a short measure; the software renderer starts with Fast. |
| 4. The graphics card | Not now (ADR 0130): the processor meets the budgets on the development computer. The question stays open until the numbers of a small computer are there. |
| 5. Help | The topic *The 3D view* in English, German, Spanish and Japanese, and the entries of What Is New. |
| 6. Audit | A probe of wrong values, the memory of ten open designs, the rules of the spec (below). |

## The budgets of the plan

The development computer, release build:

| What | Budget | Now |
| --- | --- | --- |
| First full view of 100,000 stitches | 0.3 s | drafts in 0.21 s, all tiles in 0.44 s (Milestone 5) |
| New tiles after a change of one object (200,000 stitches) | 0.1 s | 53 ms |
| A change of the light, window of 1920 × 1080 | 0.1 s | the screen in 16 ms, all tiles in 0.25 s |
| A design picture of 4000 × 4000 pixels, 100,000 stitches | 2 s | 0.54 s (4000 × 3592 pixels; 0.12 s of it the lay) |
| Memory of the tiles of all open designs | 256 MB | 234 MB for ten designs of 100,000 stitches |
| The tilt view of 100,000 stitches (Milestone 8) | 20 frames in a second | 45 |

The first full view is in the budget with the drafts; all tiles of the
level of the screen take 0.44 s. Pan and zoom of 1,000,000 stitches is the
measure of `ShokujoDX --view-bench` (Milestone 5).

## The audit

- **Wrong values** (tests `picture::tests::wrong_values_make_a_picture_and_no_error`
  and `stitch_tiles::tests::a_design_of_one_km_shows_its_tiles`): a thread
  weight of 0 and of 65,535, a stitch of 0 mm, a stitch 1 km long and a
  design 1 km wide make pictures and tiles with no error. The probe found
  one fault: the index of a lay had a square of 2 mm for all of the design,
  so a design 1 km wide asked for 1 TB of memory. The index now has at most
  4 million squares (larger ones for a larger design).
- **Ten open designs** (test `stitch_tiles::tests::ten_open_designs`): ten
  designs of 100,000 stitches, each shown, keep 234 MB of tiles (the budget
  is 256 MB). Their lays were 444 MB (44 MB each): the lays now have a
  budget of their own (256 MB; the lay that no view used for the longest
  time goes first), and ten designs keep 222 MB of lays. A design whose
  lay went out is laid again when it shows (about 0.13 s).
- **The rules of the spec**: the engine of the view is in `emb-render`
  (no UI crate, no C library, `#![forbid(unsafe_code)]`); the trigonometry
  of the new engine code goes through `emb_geom::math`; lengths are mm as
  `f64`; each decision has an ADR (0122 to 0130); each milestone has tests
  and a report; the new texts are in the four languages.

## Quality

The measure of the first start lays a design of 20,000 stitches and makes
two tiles (256 × 256 pixels at 8 pixels for each mm) for each thread of
the tile cache, on those threads at one time, and divides the time by the
number of tiles. The first full view of a design of 100,000 stitches
takes about 0.12 s for each ms of it. The development computer, and the
same computer with the program on fewer cores (the speed of a core of a
weaker computer is not in these numbers):

| Cores | Measure | First drafts, 100,000 stitches | All tiles | A change of one object (200,000) | Tilt view, turning |
| --- | ---: | ---: | ---: | ---: | ---: |
| 16 | 4.0 ms | 0.21 s | 0.44 s | 0.05 s | 45 frames/s |
| 4 | 7.6 ms | 0.55 s | 0.98 s | 0.11 s | 27 frames/s |
| 2 | 13.3 ms | 0.86 s | 1.59 s | 0.23 s | 18 frames/s |
| 1 | 25 ms | | | | |

Best is under 20 ms (the full view in about 2.5 s or less), Good under
50 ms, else Fast. An older laptop with 4 cores, each about half as fast
as a core of the development computer, measures about 17 ms (Best); a
dual-core of that speed about 33 ms (Good). A debug build does not
measure. The choice is made one time; the Thread Look menu changes it.

## What changed in the pictures

- The snapshots of the view bench and the golden snapshots did not change
  (they do not keep the design picture of the program).
- The design picture, the approval sheet, the library thumbnails and the
  pictures of Picture to Design now have the surface of the 3D look.

## Limits

- The approval sheet has a picture (an image in the PDF with a soft mask,
  so it lies on the color of the box), not lines: the file is larger, and
  the picture does not get sharper when you zoom the PDF past 254 dots for
  each inch.
- The sew-out animation has the plain fabric color (no texture): a GIF of
  a texture is several times larger. It is 0.9 to 3.4 MB for the
  reference designs.
- The sew-out animation makes each step again from the start of the
  surface: a large design takes some seconds.
- The quality measure is short and on one processor; on a computer with a
  slow graphics card and a fast processor it can choose Best when the
  screen is slow. The user can change it.

## Open

- The test with users of the exit test: five test users get twelve pairs
  of a photo of a sew-out and a picture of the view and say which is the
  photo and what gave it away.
- The numbers of a small computer (Milestone 1, item 4), and with them
  the question of the graphics card.
- A start and an edit with the software renderer on a computer with no
  graphics driver.
- The compares with photos and sew-outs of Milestones 4, 6, 7 and 8.
