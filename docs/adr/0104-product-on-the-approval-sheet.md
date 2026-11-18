# ADR 0104: The product on the approval sheet

Date: 2026-11-18 · Status: Accepted

## Context

The approval sheet (PDF) shows the design, its colors and the order, for
the customer to sign. The customer also wants to see the design on the
product. Phase P16 Milestone 4 (`docs/design/product-plan.md`) adds a
page with the design on the product.

The PDF writer (`emb_render::pdf`) had text, lines, rectangles, areas and
stitches as vectors, and no pictures. The product is a picture (light,
shade and fabric).

## Decision

- **Pictures on pages.** `page::Op::Image` puts a `raster::Picture` in a
  rectangle of the page. The PDF writer puts each one in the file as a
  JPEG image (`/DCTDecode`, quality 90; transparent pixels on white): a
  PDF reader decodes JPEG itself, so the writer needs no compression
  code. The print preview draws the picture as a texture, kept while the
  picture is the same.
- **The product page.** `sheet::product_page` comes after the approval
  page: the header, then one picture for each side (the side that shows,
  or the front over the back with Both sides in the picture), at one
  scale, as large as the page lets them be, each with its caption. One
  over the other makes each side larger on a portrait page than next to
  each other.
- **The picture.** `product::pictures_of_design` makes the pictures from
  a design and its stitch plan, with no app state, so Output design and
  Batch output make the same page. It uses the default 3D thread look at
  2.5 pixels for each mm (about 250 dpi on the page for one side). The
  preview keeps the pictures for a design state (`AppState::sheet_product`).
- **The option.** `OutputProfile::approval_product` (on by default):
  With the product, next to Approval sheet in Output design.

## Consequences

- An approval PDF with a product page is larger: about 0.7 MB for a
  garment with both sides.
- The production worksheet does not show the product; it is for the
  operator.
