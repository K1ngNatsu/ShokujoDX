# ADR 0103: The front and the back of a garment

Date: 2026-11-18 · Status: Accepted

## Context

A shop often sews a logo on the front of a garment and a print or a name
on the back. The product view (P7 Milestone 5) had a T-shirt back as a
product of its own, and no back for the polo or the sweatshirt. Phase P16
Milestone 3 (`docs/design/product-plan.md`) gives a garment two sides.

## Decision

- **A side is a kind.** Each side of a product is a `ProductKind`, with
  its own drawing, placements, roundness and fabric: `TShirt` and
  `TShirtBack` (as before), `Polo` and `PoloBack`, `Sweatshirt` and
  `SweatshirtBack`. `ProductKind::turned` gives the other side,
  `side` and `on` go between a kind and a `Side`. `ProductView::kind` is
  the side that shows. Projects of earlier versions open with no change
  (a T-shirt back is still a T-shirt back).
- **The open design is on the side that shows.** Turn Product (the
  Product window, the Design tab and Ctrl+Shift+T) shows the other side
  and moves the product so that the open design is at the placement of
  the same name there, else at a free place. The design itself does not
  change.
- **Each other design has a side** (`ProductExtra::side`, front by
  default). It shows on the canvas only when its side shows. A new extra
  goes on the side that shows; the list can put it on the other side.
- **Both sides in one picture.** With `ProductView::both_sides`, Save
  picture on product draws the front and the back of the garment next to
  each other, each with its designs.
- The list of products to choose from (`ProductKind::PRODUCTS`) has the
  fronts only, with the names of the products (`product_name`).

## Consequences

- To see a back print on the canvas, turn the product; the open design
  then sits on the back. The saved picture with both sides shows each
  design on its own side.
- A new garment with two sides needs two kinds and a `turned` pair.
