# ADR 0101: A realistic look of the product

Date: 2026-11-17 · Status: Accepted

## Context

The product under the design (P7 Milestone 5, `emb_model::product`) was
flat areas of one color with dark seam lines. The owner found it weak
(2026-11-17): it did not look like cloth, so it was not good enough to
show to a customer. Phase P16 (`docs/design/product-plan.md`) makes the
product view better; Milestone 1 is the look.

The products stay drawn in code at a real size: no pictures of garments
go in the repository, and a product can have any color.

## Decision

`emb_render::product` lights the drawn product with a height field:

- **Round edges.** The height rises from the outline (a chamfer distance
  of the product area) over a width and to a height for each kind
  (`roundness`: little for a towel, much for a cushion or a cap crown). A
  blur makes it smooth where two edges meet.
- **Folds.** `ProductPart::Fold { path, width, height }` is a soft ridge
  or valley (cos² across, low at the two ends), part of the drawing of
  each product. Folds stay out of the usual places of large designs.
- **Seams.** A seam line (3 mm wide or less) is also a small groove.
- **Light.** The light of the 3D thread look (`thread3d::Lighting`, the
  user's light) shades the height field (ambient and diffuse), with a
  little shadow at the edges. Light colors are a little darker on flat
  cloth so the highlights show; light lifts dark colors a little, so a
  black product shows its shape.
- **Fabric.** Each kind has a fabric (`product::weave`: knit, piqué,
  fleece, twill, terry, plain). The fine weave shows only with enough
  pixels (no moiré); a soft mottle of larger cells shows always.
- **Shadow of the embroidery.** The saved picture (`decoration`) has a
  soft shadow under the stitches (blurred cover, away from the light),
  before the stitches.

The light of a product is a brightness factor for each pixel
(`product::Shading`). It does not depend on the color or the place of
the product, so the app keeps it for a kind of product and a light, and
a color drag only paints the colors again.

A photo of the user's own product does not get the light or the fabric:
it has them already. The embroidery shadow is on it too.

## Consequences

- The canvas and the saved picture show the same look; the canvas has no
  soft shadow of the embroidery yet (the plan keeps it for later in
  Milestone 1, if needed).
- A product of a new kind needs a roundness, a fabric and folds.
- The picture costs more time: about half a second for each product at
  4 pixels for each mm (release build). The shading of the canvas is
  made once for each kind.
- No golden snapshot changes: the snapshots do not include the product.
