# ADR 0106: Two-tone products and garment colors

Date: 2026-11-18 · Status: Accepted

## Context

Many blank garments have parts in a second color: the collar and cuffs
of a polo, the rib of a T-shirt, the front panels or the brim of a cap.
A customer wants to see the design on the garment that they buy. Phase
P16 Milestone 6 (`docs/design/product-plan.md`) adds a second color and
the usual garment colors.

## Decision

- **Trim parts.** A trim part is an area of the drawing with the shade
  `Shade::Trim { group, dark }`. `group` is the index of its name in
  `ProductKind::trims` (for example "Collar" and "Sleeve cuffs" of a
  polo); the two sides of a garment have the same names. A trim part
  lies on the parts in the product color, so a product with no trim
  color looks as before. The shape of a trim part can go a little past
  the edge of the product: the drawing cuts it at the edge.
- **The trim color** is `ProductView::trim` (`Trim`: a color and the
  names of the parts that are off). `ProductView::part_color` gives the
  color of a part, or no color for a trim part that does not show. The
  parts that are off are names, so they stay correct when the user
  changes the product.
- **Lines on trim parts.** On a two-tone product, each piece of a seam or
  an edge is darker than the area under it, so a seam on a white cap
  front is light gray, not dark blue. A product with one color draws its
  lines as before.
- **Garment colors.** `product::SWATCHES` has 18 usual colors of blank
  garments with generic names (no brand names or brand color numbers).
  The Product window shows them as small swatches under the product
  color and under the trim color.
- The shading of the product does not change with a trim color, so a
  color change on a two-tone product stays fast (a trim adds less than
  3 ms to a picture of the product on the canvas).

## Consequences

- The window has **Trim color** (on or off), the trim color, its
  swatches and a check box for each trim part.
- A new trim part needs a name in `ProductKind::trims`, an area with its
  group in the drawing, and a translation of its name. A test checks
  that each name has a part on a side of the product.
- The trim color goes into the project file only when it is set; older
  projects open with no trim color.
