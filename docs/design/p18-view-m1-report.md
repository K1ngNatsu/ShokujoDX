# Report: the 3D stitch view, Milestone 1 (measures and a reference set)

Date: 2026-11-19 · The second part of P18 (`docs/design/p18-view-plan.md`),
ADR 0122. Items 1 to 4 are done; item 5 (the sew-out) and the numbers of
a small computer are open.

## What the milestone made

| Item | What there is now |
| --- | --- |
| 1. A reference set | Twelve corpus designs `view-*`, each for one item of the list of faults (table below). |
| 2. `emb-cli view-bench` | The pictures of the set at three zooms, the time and the data of each step; `--out`, `--stress`, `--sew`. The test `tests/view_bench.rs` keeps a hash of each picture; CI keeps the pictures. |
| 3. A picture compare | `emb-cli view-compare <a> <b> <out>`: side by side, the difference, and the difference of the brightness of each area (linear light); `--place` for a photo. |
| 4. The numbers of today | This report, from `emb-cli view-bench --stress` and `ShokujoDX --view-bench` (also with `--renderer=software`). |
| 5. The sew-out | Open: the owner sews the set (`out/p18-view-bench/sew`, made with `--sew`) on two fabrics and takes the photos. |

## The reference set

Each design has natural white cotton under it, except `view-towel` (a
light blue towel, with the fabric setting `towel`) and `view-dark` (black
cotton). The close picture is a 16 mm square at the place in the table.

| Design | Fault of the plan | What it has | Close picture at |
| --- | --- | --- | --- |
| `view-satin-angles` | 2. Sheen by direction | Eight satin columns of one color, 0° to 157.5° in steps of 22.5° | the column at 0° |
| `view-fill-border` | 1. Height | A tatami circle with a satin border on its edge | the border at 45° |
| `view-underlay` | 1. Height | Satin columns and fills with no underlay and with two layers | the two columns |
| `view-lettering` | 4, 7. The shape of a stitch, small zoom | Round Sans at 6 mm, Soft Round at 60 mm | the small text |
| `view-fill-over-fill` | 1. Height | A dense fill (0.3 mm) over a fill | the edge of the top fill |
| `view-puff` | 1, 6. Height, what is not thread | Puff letters and a puff column | the column |
| `view-applique` | 6. What is not thread | An appliqué circle with its cover and a triple run | the edge |
| `view-motif-fill` | 2. The pattern of a fill | A motif fill of hearts | the middle |
| `view-run` | 4. The shape of a stitch | Single run, triple run, backstitch, a run on a circle | the triple run |
| `view-blend` | 8. Colors | A blend of red and gold in one area | the middle |
| `view-towel` | 5. The fabric | Names on a towel, 20 mm and 8 mm | the small name |
| `view-dark` | 8. Colors | White and navy thread on black | the gap between the fills |

The thread kinds (fault 3) need the thread kinds of Milestone 2; the
point of view (fault 9) has no design. Fault 10 (no proof) is this
milestone.

## The numbers of today

The development computer: 32 processor threads, an NVIDIA GeForce RTX
5070 Ti, Windows 11, release build of 2026-11-19.

### The canvas (`ShokujoDX --view-bench`)

A window of 1280 × 720 pixels with no vertical sync; the view moves and
zooms (the whole design to 4 times) for 3 s. Frames/s is the number of
frames in a second; "slow" is the time that 95 % of the frames are
faster than.

| Stitches | Look | Graphics card: frames/s (median, slow ms) | Software renderer: frames/s (median, slow ms) | Mesh ms |
| ---: | --- | --- | --- | ---: |
| 10,656 | stitches | 2,900 (0.31, 0.67) | 7.7 (113, 188) | 0.04 |
| 10,656 | thread look | 1,868 (0.48, 0.95) | 4.9 (185, 274) | 0.12 |
| 10,656 | 3D thread look | 1,531 (0.54, 1.37) | 4.0 (198, 354) | 0.19 |
| 106,576 | stitches | 389 (2.0, 5.0) | 1.6 (379, 1,009) | 0.67 |
| 106,576 | thread look | 246 (3.2, 8.7) | 1.5 (483, 1,012) | 1.22 |
| 106,576 | 3D thread look | 189 (4.3, 13.4) | 1.6 (391, 1,024) | 1.75 |
| 787,408 | stitches | 49 (15.3, 56.6) | 1.4 (1,017, 1,073) | 5.30 |
| 787,408 | thread look ¹ | 31 (23.2, 65.5) | 1.6 (390, 1,050) | 8.82 |
| 787,408 | 3D thread look ¹ | 24 (37.1, 78.3) | 0.9 (1,055, 1,062) | 11.79 |

¹ The frames with all stitches in the window are drawn as stitches
(`canvas::fitted_mesh`, see below).

**What the numbers say.** With the graphics card, the canvas of today
meets the budget of the plan (60 frames in a second) up to about 100,000
stitches, and not with 800,000 (24 to 49). The software renderer is far
below the budget (30 frames in a second) at each size: it draws each
triangle on the processor, and a frame with 10,000 stitches has 20,000
to 100,000 triangles. A canvas on tiles (Milestone 5) draws a few
textures in place of the triangles, so the software renderer gains the
most from it.

### The pictures (`emb-cli view-bench --stress`)

One processor thread. The whole design at 2 pixels for each mm and at 8
pixels for each mm (about 200 × 180 mm: 400 × 360 and 1,600 × 1,440
pixels).

| Stitches | Segments ms | Flat 2 px/mm | 3D 2 px/mm | Flat 8 px/mm | 3D 8 px/mm |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 10,656 | 0.1 | 3 ms | 8 ms | 22 ms | 200 ms |
| 106,576 | 0.6 | 29 ms | 33 ms | 204 ms | 1.3 s |
| 787,408 | 4.4 | 213 ms | 215 ms | 1.5 s | 9.2 s |

The budget of the plan for a design picture of 4,000 × 4,000 pixels with
100,000 stitches is 2 s. The 3D look of today needs about 1.3 s for 2.3
million pixels, so about 9 s for 16 million.

The reference set: a design picture at 8 pixels for each mm takes 11 to
132 ms in the 3D look (`view-lettering` is the largest), and the close
picture (512 × 512 pixels) 11 to 75 ms. The stitch plan of each design
takes less than 2 ms. The full table is in `report.txt` of the run.

### Memory

A picture keeps 4 bytes for each pixel (1 MB for a close picture, 8.7 MB
for the whole stress design at 8 pixels for each mm). The segments of
800,000 stitches are 30 MB (40 bytes for each segment). The plan expects
about 12 bytes for each pixel for a surface before the light.

## A fault found: very large designs stopped the program

With 787,408 stitches the program stopped in the 3D thread look: the
mesh needed 165 MB of vertices, egui-wgpu doubled its vertex buffer to
329 MB, and the graphics library allows 256 MB for one buffer. A frame
that needs more than 128 MB of vertices can do this: about 340,000
stitches that show at one time in the 3D thread look, or 670,000 in the
flat thread look. The canvas now uses a lighter mesh for such a frame
(ADR 0122); the numbers above are with this change. What's New tells the
users.

## Open

- **The sew-out (item 5).** Sew the twelve designs of
  `out/p18-view-bench/sew` (VP3 for the Topaz 20, with a worksheet each)
  on white cotton and on a second fabric; `view-towel` and `view-dark`
  have their own fabric. Take the photos in one light from the top left
  and with the camera straight above. Then `emb-cli view-compare` puts
  each picture next to its photo; the result goes in this report.
- **A small computer.** Run `ShokujoDX.exe --view-bench` and
  `ShokujoDX.exe --view-bench --renderer=software` on a small computer;
  each writes `view-bench.txt` in the settings folder. Its budgets are
  set from these numbers.
- **The pictures of today** are not in the repository: CI keeps them
  (the artifact `view-bench`), and `emb-cli view-bench --out <folder>`
  makes them again.
