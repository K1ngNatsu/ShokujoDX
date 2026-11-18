# ADR 0102: Other designs on the product

Date: 2026-11-18 · Status: Accepted

## Context

The product view (P7 Milestone 5) showed only the open design. A shop
often sews more than one design on one product, for example a logo at
the left chest and a name on a sleeve, and the customer wants to see
all of them in one picture. Phase P16 Milestone 2
(`docs/design/product-plan.md`) adds the other designs.

## Decision

- **A copy in the project.** `ProductView::extras` is a list of
  `ProductExtra`: the file name, the place (`at`, the center of the design
  in product coordinates), Show, and a copy of the design. A machine file
  becomes a design of stitch objects (as File > Open does). The copy has
  no product and no backdrop of its own. The project keeps the copy, so
  the product shows the same when the file moves or changes.
- **A key from the contents.** `ProductExtra::key` is a hash (FNV-1a) of
  the saved text of the copy. The same design has the same key on each
  run, so the app stitches each design once (`product::Extras`, by key)
  and keeps its canvas picture. An undo that brings a design back finds
  its stitches again.
- **Places.** A new extra goes to the first placement with no design
  (the open design or an extra), else to the middle of the product. A
  change of the product kind moves each extra to the placement of the same
  name, if the new kind has it. The T-shirt and the sweatshirt get a
  Sleeve placement.
- **Drawing.** On the canvas each extra is a picture with the thread look
  (at most 8 pixels for each mm, and 4096 pixels on a side), over the
  product and under the open design. The extras are not objects: they do
  not select, move with the mouse or sew. The saved picture on the product
  has their stitches, with the soft shadow.

## Consequences

- A project with extras is larger by the size of each copy.
- The extras do not follow a later change of their files. To show the
  new version, remove the extra and add the file again.
- Projects of earlier versions open with no extras (the field has a
  default), and their files do not change when they have none.
