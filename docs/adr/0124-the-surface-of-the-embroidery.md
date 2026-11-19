# ADR 0124: The surface of the embroidery

Date: 2026-11-19 · Status: Accepted (Milestone 3 of the second part of
P18, `docs/design/p18-view-plan.md`; the main decision of the part)

## Context

The 3D thread look of today (ADR 0040) lights each stitch alone, as a
cord, and paints the stitches in sew order. Each stitch is at the same
height; only the order of the paint shows what is on top. So a border on
a fill is as high as the fill, underlay does not lift a column, puff is
flat, and a satin that sews up looks darker than one that sews down
(ADR 0123). The canvas makes a mesh of all stitches for each frame, and
the pictures light each pixel of each stitch on one processor.

## Decision

The view makes a **surface** of the embroidery one time and lights it.

- **Strands** (`emb_render::strand`): the stitches of a stitch plan with
  the color, the kind and the weight of their thread and the foam under
  them (puff columns and puff lettering, `strand::lifts` from the objects
  of the plan, 2 mm). The plan does not mark which stitches are underlay,
  and the surface does not need it: the underlay sews first, so the
  stitches of the top lie on it. A role for each stitch (underlay, top,
  travel) waits for a step that needs it; it needs a change of the stitch
  engine.
- **Lay** (`surface::lay`): the strands are laid in sew order on a grid
  of 0.1 mm (larger cells for a design of more than 16 million cells).
  - A stitch is a flat oval, 0.55 times as thick as it is wide (the
    width of its weight, ADR 0123). Along it, it comes out of the fabric
    at its needle point and is at its full height after 0.35 mm (a
    quarter circle); a short stitch is an arch.
  - A stitch lies on what is under its middle line as a thread under
    tension: a straight line from each needle point (on the fabric)
    over the highest things that it crosses (the upper hull). A thread
    beside it (less than half a width into it) is not under it, and the
    stitch just before it is not under it either (the two share a needle
    hole): such threads lie side by side. These rules come from the
    first models: with a slow slope from each high point, or with
    samples at the sides of a stitch, satin stitches in a curve and the
    parts of a split satin lay on each other, and lettering became 5 to
    14 mm high.
  - Thread presses thread: on `n` layers a thread adds its thickness
    divided by `1 + 0.15 n` (the grid counts the layers; the density map
    of ADR 0044 is not used, so the surface needs no second pass).
  - Foam is a floor under a stitch from 0.3 mm after its needle points;
    appliqué fabric and stones are pieces 0.3 mm thick under all stitches.
  - The result (`Laid`) keeps the height of the middle line of each
    stitch at steps of one cell. Laying is in sew order on one
    processor.
- **Tiles** (`Laid::surface`): the surface of an area at a zoom, in tiles
  of 256 × 256 pixels, made on all processors. For each pixel: the
  height (the highest thread there; a thread sewn later wins at the same
  height), the color, the direction and the material of the thread on
  top, and the part of the pixel that threads cover (the union of the
  parts of each thread). A pixel depends only on the laid stitches, so
  the same input gives the same bytes on each system, with each number of
  processors and in each part of an area (tests).
- **The first light** (`surface::light`): the light of the 3D thread look
  of today (`Lighting::shade_normal`, the same constants) on the normal
  of the surface, with the highlight of the kind of thread, over the
  fabric of today in linear light. No shadows yet: Milestone 4 gives the
  light of the surface (sheen by direction, shadows from the heights,
  zoom levels).
- **The bench** (ADR 0122) makes a second picture of each reference
  design at each zoom, `<design>-<zoom>-surface`, and measures the lay,
  the tiles and the light; `--stress` measures them at 10 pixels for each
  mm. The canvas and the pictures of the program do not change yet
  (Milestones 5 and 9).

## Consequences

- The exit test reads the heights from the surface: a border on a fill
  0.85 mm and the fill 0.41 mm; a satin with two layers of underlay
  0.63 mm and one with none 0.21 mm; puff 2.3 mm (2.7 mm at the caps),
  and no other design of the set higher than 1.14 mm.
- The surface of 100,000 stitches at 10 pixels for each mm takes 0.17 s
  on the development computer (lay 136 ms, tiles 16 ms, light 22 ms;
  the budget is 1 s). With 800,000 stitches the lay takes 0.95 s, on one
  processor; the data of the laid stitches is then 195 MB (the heights
  of 30 points for each stitch and the grid). Milestone 5 needs a smaller
  form of it for its budget of memory.
- A satin looks the same in each sew direction; the zoom of the whole
  design keeps the colors of the threads.
