# ADR 0105: Better products

Date: 2026-11-18 · Status: Accepted

## Context

The owner found the cap odd (2026-11-17) and wanted more products to show
a design on. Phase P16 Milestone 5 (`docs/design/product-plan.md`) draws
the cap again and adds new products.

## Decision

- **The cap** is a six-panel cap from the front and a little from above:
  the crown with the front seam and the side seams up to the button, two
  eyelets, a curved brim narrower than the crown, and four rows of
  stitches square to the front edge of the brim. Its round edge is less
  high than before (70 mm wide, 20 mm high), so the brim does not look
  puffy.
- **A curved design on the cap.** `ProductKind::warp` puts a point of a
  flat design on the product: for a cap, a cylinder round the crown
  (radius 115 mm) makes the design a little narrower at its sides, and
  its sides a little lower. The saved picture and the approval sheet use
  it; the canvas shows the design flat, because the design is the work on
  the canvas. The other products are flat.
- **New products**, each with its drawing, placements, folds, roundness
  and fabric:
  - a long-sleeve T-shirt, front and back (sleeves at 48°, a hemmed
    cuff; upper and lower sleeve placements);
  - a hoodie, front and back (the sweatshirt body; the hood with its
    lining and drawstrings at the front, the hood on the back, and a
    pocket across the front; the hood is a placement on the back);
  - a bib apron (neck strap, waist ties, a pocket in two halves; bib,
    center and pocket placements);
  - a knitted beanie with a cuff (knit ribs as small folds; cuff and
    above the cuff);
  - a messenger bag (the flap with a row of stitches, a strap and two
    closing straps; the flap placement).
- The sweatshirt and the hoodie share one drawing (`fleece_top`), and
  the long sleeves share their creases (`sleeve_creases`).

## Consequences

- There are 17 kinds of product sides and 12 products to choose from.
- A new product kind needs a drawing, placements inside its outline (a
  test checks each placement of each kind), a roundness, a fabric, and a
  fabric for the lettering kiosk (`kiosk::fabric_of`).
