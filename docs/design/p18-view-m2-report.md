# Report: the 3D stitch view, Milestone 2 (light as light, and threads with a kind)

Date: 2026-11-19 · The second part of P18 (`docs/design/p18-view-plan.md`),
ADR 0123. All five items are done.

## What the milestone made

| Item | What there is now |
| --- | --- |
| 1. Linear light | `emb_render::light`: the pictures, the design on the product and a picture on a PDF page mix colors in linear light. A fully covered pixel keeps its color (test `the_flat_look_keeps_its_colors`). |
| 2. Kind and thickness | `Thread::kind` (polyester, rayon, cotton, metallic, wool, glow) and `Thread::weight` (40 by default, 8 to 150); `thread::width_mm` gives the width of each weight. A project file of an earlier version reads with the defaults and is written the same. |
| 3. The charts | `data/threads/kinds.toml`: 43 of the 75 charts have a kind, each with its source (the name of the line). No chart name gives a weight. |
| 4. The Threads panel | Kind and Weight of the current thread; for a weight that is not 40, a note with the spacing to use. Undo works for both. |
| 5. The model of today uses them | The width of each stitch from its weight (pictures, flat and 3D canvas), the highlight from its kind (3D look). |

## Exit test

- **Two widths:** a 40 weight and a 60 weight thread are 16 and 13
  pixels wide at 40 pixels for each mm (test
  `a_thread_has_the_width_of_its_weight`); on the canvas the 60 weight
  thread is 0.82 times as wide (test
  `the_weight_gives_the_width_and_the_kind_the_highlight`).
- **A matte thread has no highlight:** in the 3D picture the brightest
  green of a thread of color (120, 60, 60) is at most 80 for cotton and
  wool (the light alone), and more than 100 for polyester; rayon and
  metallic are brighter again (test `a_matte_thread_has_no_highlight`).
- **The pictures of the reference set change only at the edges of
  threads:** see the next section. The compare pictures are made with
  `emb-cli view-compare <before> <after> <out>`; the pictures of before
  are those of Milestone 1 (`emb-cli view-bench` of commit `c587959`).

## The reference set before and after linear light

The brightness of each area of a 4 × 4 grid, the pictures of Milestone 1
against the pictures of now (`--grid 4x4`; a plus is lighter now):

| Design | Close (32 px/mm) | Normal (8 px/mm) | Whole (2 px/mm) |
| --- | --- | --- | --- |
| view-satin-angles | 0.2 % (largest +0.4) | 0.1 % (+0.2) | 0.1 % (+0.2) |
| view-fill-border | 0.5 % (+1.7) | 0.9 % (+1.3) | 2.6 % (+3.6) |
| view-underlay | 0.5 % (+1.3) | 2.1 % (+5.4) | 8.3 % (+18.1) |
| view-lettering | 0.4 % (+1.7) | 0.3 % (+0.5) | 1.0 % (+1.8) |
| view-fill-over-fill | 1.0 % (+1.2) | 1.8 % (+2.1) | 6.1 % (+8.2) |
| view-puff | 0.3 % (+0.6) | 0.4 % (+0.7) | 0.9 % (+2.3) |
| view-applique | 0.1 % (−0.3) | 0.3 % (+0.6) | 1.5 % (+2.7) |
| view-motif-fill | 0.3 % (+0.4) | 0.9 % (+1.0) | 2.9 % (+3.4) |
| view-run | 0.1 % (+0.1) | 0.2 % (+0.4) | 1.0 % (+2.1) |
| view-blend | 1.3 % (+1.7) | 3.5 % (+5.5) | 12.9 % (+21.4) |
| view-towel | 0.6 % (+2.3) | 0.6 % (+1.8) | 2.0 % (+6.0) |
| view-dark | 0.6 % (+1.8) | 2.3 % (+2.9) | 11.7 % (+16.6) |

At the close and the normal zoom the pixels that changed are at the
edges of the threads and in their shadows (1 to 34 % of the pixels of a
close picture, nearly all by a few values). At the zoom of the whole
design a thread is 0.8 pixels wide, so each pixel is a mix of thread and
fabric, and the mix of light is lighter than the mix of numbers: a
dark thread on a light fabric (blend, dark, underlay) changes most.

**The zoom levels do not agree yet.** The brightness of the whole
picture and of the normal picture of one design differ by 3 to 25 %
(mean of the areas), before and after linear light: at 2 pixels for
each mm the 3D look draws each thread as a flat line (it is less than a
pixel), and at 8 pixels for each mm with light and shade. This is fault
7 of the plan; Milestone 4 makes the zoom levels from the surface.

## The thirteenth reference design

`view-thread-kinds`: eight satin columns of color (110, 150, 200), one
for each kind at weight 40, then polyester at 30 and at 60, all sewn
from the top down. The mean brightness and the brightest 1 % of each
column in the normal picture (luminance of the screen values, 0 to 255):

| Thread | Mean | Brightest 1 % |
| --- | ---: | ---: |
| Polyester 40 | 165.0 | 195.3 |
| Rayon 40 | 166.5 | 198.0 |
| Cotton 40 | 158.9 | 186.6 |
| Metallic 40 | 170.0 | 204.7 |
| Wool 40 | 158.9 | 186.6 |
| Glow 40 | 162.6 | 190.8 |
| Polyester 30 | 167.5 | 194.2 |
| Polyester 60 | 160.4 | 195.7 |

Cotton and wool are the same here: wool is thick, but its weight is set
apart (a wool thread is usually 12 weight). The 60 weight column shows
more gaps between its stitches than the 30 weight column, as a sew-out
with the same spacing does; the note in the Threads panel tells the user.

## A fault found: the sew direction of a satin

The first layout of `view-thread-kinds` had the columns side by side, and
they were dark and light in turns (128 and 178 of 255 with the same
thread). The stitch engine starts each column at the end nearest the
needle, so the columns sewed down and up in turns. The 3D look paints
each stitch over the one before, with a shadow away from the light: in a
column that sews down, the lit upper edge of each new stitch shows; in a
column that sews up, its dark lower edge shows. A sew-out does not show
this. Milestone 3 lays the stitches in a surface by height, not by the
order of the paint, and removes it. The reference designs `view-underlay`
(its two columns) and `view-satin-angles` show it today.

## Open

- The canvas is not in linear light: the graphics library mixes the
  colors of its meshes as screen numbers. The canvas on tiles
  (Milestone 5) shows pictures of `emb-render`.
- The weights of the threads of the charts: no chart of the program
  gives them. A chart that gives them can go in `kinds.toml` with its
  source.
- The owner's sew-out of the reference set (Milestone 1, item 5) is
  still open; it now has thirteen designs.
