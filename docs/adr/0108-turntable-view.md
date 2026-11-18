# ADR 0108: A turntable view of the product

Date: 2026-11-18 · Status: Accepted

## Context

The owner wants a customer to see the product from more sides, as a
simple 3D view that turns in steps (2026-11-17), and fast on a PC with
no graphics card. Phase P16 Milestone 8 (`docs/design/product-plan.md`)
adds it. The project has no 3D models of garments, and no pictures of
garments of other companies go in the repository.

## Decision

- **A body from the drawing** (`emb_render::turntable::Body`). The
  outline of the front and the back of a drawn product is "blown up": a
  part of it has the half depth `ratio × √(2u)`, where `u` is the
  solution of ∇²u = −1 in the outline (u = 0 at the edge). A part is
  round across its own width, so a sleeve is thinner than the body. The
  ratio is a property of the kind (a garment 0.5: a T-shirt is about
  250 mm deep; a towel 0.05). The bottom of a garment or a hat is open
  (the shape goes on down when it is made), so the hem is a tube and
  not the edge of a closed bag. A cap has its own shape: a round crown
  open at the bottom, and a brim that comes forward from the bottom of
  the crown.
- **The design is exactly on it.** The front of the body has the flat
  coordinates of the canvas: its colors are the picture of the front
  side with the designs (`product::turn_side`, as the saved picture but
  with no flat round edges, because the body has its own shape). The
  back of the body has the picture of the back side (a product with one
  side: the product with no design).
- **Views.** A view turns the body round the vertical axis through the
  middle of the product and draws it from the front with no
  perspective. Each row of the body is a closed curve (the front, then
  the back), drawn with a depth buffer, so no view has holes. A studio
  light from the top left in front shades the turned surfaces; a surface
  that faces the viewer is as bright as in its flat picture. A view is
  made at twice its size and made smaller, for smooth edges.
- **Steps made before.** When the design or the product changes, a
  background job (`emb-app/src/turntable.rs`) makes the 24 views (15°
  apart), the front first. A turn shows a finished view only, so it is
  fast with the software renderer too. The views show in the
  **Turntable** window (Design > Turntable); the canvas stays flat,
  because the design is the work there (as for the curve of a cap, ADR
  0105).
- **Save.** Save GIF writes the 24 views as an animated GIF that plays
  again and again (150 ms each); Save Four Views writes the front, two
  quarter turns and the back next to each other (PNG).

## Consequences

- No 3D model files, no graphics card and no new dependency.
- At an angle, the cloth at the far edge of a part is stretched (the
  soft body has a steep side); a very round product (a beanie) shows it
  most.
- A photo of a product cannot turn (it has no outline and no back).
- A garment on a body or a mannequin needs real 3D models; it is not in
  this plan.
