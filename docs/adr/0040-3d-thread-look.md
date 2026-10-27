# ADR 0040: The 3D thread look with CPU meshes and pixel shading

Date: 2026-10-24 · Status: Accepted (P10 Milestone 3)

## Context

Roadmap P10 asks for a thread preview with light and fabric texture (as
the 3D thread look of the reference product). The canvas draws the stitches as an egui
mesh that the app makes again each frame (2D thread look, P3). The
design picture and the virtual decoration picture are made pixel by
pixel in `emb_render::raster` (no GPU, so `emb-cli` makes them too).

A GPU shader (a wgpu paint callback) would give per-pixel light on the
canvas, but it ties the canvas to wgpu, needs its own pipeline and does
not help the pictures, which must not need a GPU.

## Decision

1. One shading model in the engine crate `emb-render`
   (`emb_render::thread3d`): a stitch is a round cord on the fabric. A
   light with a direction, a height and a strength gives each point across
   (`s`) and along (`t`) the thread a brightness (ambient and diffuse) and
   a highlight; the thread is darker near the needle points; it throws a
   shadow away from the light. The fabric textures (plain weave, twill,
   knit, piqué, terry, felt, smooth) are procedural values that repeat in
   a 4 mm tile; the fabric id of the design selects one. Trigonometry goes
   through `emb_geom::math` (ADR 0003).
2. The canvas uses CPU meshes, as before: 5 vertices across each stitch,
   3 rows along it when the thread is at least 12 screen points wide (else
   2), and a shadow quad. A table (`ShadeTable`) keeps the shading for 64
   stitch directions; the colors of a direction are kept while the
   stitches have the same color. The fabric is a texture tile with
   `TextureOptions::LINEAR_REPEAT`, under the stitches where the design has
   a background color; it fades out when its threads are smaller than the
   screen pixels (wgpu in egui has no mipmaps).
3. The pictures shade each pixel with the same model, with the twist of
   the plies and the shadow of each stitch, over the fabric texture.
   `raster::Look` selects the look; the old functions keep the flat look.

## Consequences

- The canvas, the PNG design picture, the decoration picture and
  `emb-cli output` show the same light. The SVG picture, the worksheets,
  the library thumbnails and the wizard preview keep the flat look.
- The 3D mesh has about 1.7 times the triangles of the 2D mesh; at 4:1
  zoom it builds in 1.5 to 1.8 times the time of the 2D mesh (1.7 ms for
  100 000 stitches, release build). At small zoom both looks draw thin
  lines.
- No new dependency and no `unsafe` code.
