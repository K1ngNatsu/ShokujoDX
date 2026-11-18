# ADR 0107: Photos of the user's products

Date: 2026-11-18 · Status: Accepted

## Context

The user can put a photo of their own product under the design (P7
Milestone 5), but the photo shows only its own color, it has no usual
places for a design, and the user must load and measure it again for
each design. Phase P16 Milestone 7 (`docs/design/product-plan.md`) makes
the photo a product of its own.

## Decision

- **The mask** is `ProductPicture::mask` (`PhotoMask`): a grid of cells
  over the photo (the shape of the photo, at most 1024 cells on its long
  side), each on or off, saved as runs. A large photo has a mask of a
  few kilobytes in the project.
- **Pick a color.** A click picks the cells that touch the click through
  cells of a near color (`emb_render::photo::MaskGrid::pick`). Light and
  folds change the brightness of cloth more than its color, so a change
  of brightness counts less. Lines of another color up to about 6 cells
  wide (seams, stitches) do not stop the pick and are part of the area
  (a closing of the cells of a near color before the flood fill). A
  brush and an eraser change the mask by hand.
- **The color on the photo** (`emb_render::photo::recolor`): the masked
  cloth gets the product color, and each pixel is darker by the ratio of
  its brightness to the brightness of the cloth (a high part, 85 %, of
  the brightness of the masked pixels). Highlights go to white, and the
  edge of the mask is soft. One photo of a white product shows the
  product in each color. The canvas keeps the decoded photo, so a color
  change does not decode it again.
- **Places on a photo** are `ProductPicture::placements`
  (`PhotoPlacement`: a name, a point in mm from the top left corner of
  the photo, and the largest design). The design goes to the first place
  when the photo is put under it.
- **Product templates** are `.embproduct` files (JSON with a format name:
  the `ProductPicture` with its photo, real width, mask and places) in
  the `products` folder next to the settings. **My Products** lists them
  by file name; Export writes one to a file that the user chooses, and
  Import adds one to the folder, as My Machines does (ADR 0099).
- The mask and the places change in the **Product Photo** window
  (`emb-app/src/product_photo.rs`). The cells change in the window while
  the user paints; one click or one stroke is one undo step.

## Consequences

- A photo of a dark product cannot take a light color well (it has
  little light and shade); the help says that one photo of a white
  product shows the product in each color.
- Older projects open with no mask and no places.
