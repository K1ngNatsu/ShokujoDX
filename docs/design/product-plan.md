# Plan: a better product view (phase P16)

Date: 2026-11-17 · Status: Accepted (owner, 2026-11-17). Milestones 1 to 8 done (ADR 0101 to 0108); the owner checks the exit tests at the end of the phase.

## Goal

**Design > Product** puts the design on a drawn product at its real size
(P7 Milestone 5). The owner found it weak (2026-11-17): the products are
flat shapes of one color, with no light, folds or fabric; one design at
one place only; and the picture is not on the approval sheet. The
reference product has a library of garment pictures, product colors of up
to three layers, the user's own product templates, and approval sheets
with the design on the product.

This plan makes the product view good enough to show to a customer. It
has eight milestones. Each milestone is small, can be released alone, and
keeps the projects of earlier versions (new fields have defaults).

What stays:

- The products are drawn in code at a real size (adult size M), so the
  design shows at its true size. No pictures of garments of other
  companies go in the repository.
- The product does not sew and does not change the stitches.
- One shading model: the light of the 3D thread look
  (`thread3d::Lighting`) lights the product too.

## Milestone 1: a realistic look (M)

The drawn product looks like cloth.

1. **Light and shape.** A height field of the product: the body is round
   at its edges (more for a cushion or a cap crown, less for a towel),
   folds are soft ridges and valleys, and seams are small grooves. The
   light of the 3D thread look shades it, and the edges get a little
   ambient shadow. The folds are parts of the product drawing
   (`ProductPart::Fold`).
2. **Fabric.** Each product has its fabric (`Weave`): knit for a T-shirt,
   piqué for a polo, fleece for a sweatshirt, twill for a cap and a tote,
   terry for a towel, plain weave for a cushion. The fine weave shows
   only where the picture has enough pixels for it (no moiré); a soft
   mottle of larger cells shows always.
3. **Shadow of the embroidery.** The saved picture gets a soft shadow
   under the whole embroidery, away from the light, so the design sits on
   the cloth.
4. **Speed.** The app keeps the shading of a product kind, so a change of
   the product color does not make it again.

5. **Shapes and lines** (owner, 2026-11-18). The garments have the
   proportions of an adult M laid flat (shoulders, chest, sleeve angle
   and opening, armhole seams; the polo collar lies on the chest), and a
   seam (a line of 3 mm or less) stops at the edge of the product.

Later in this milestone, if the canvas needs it: the soft shadow on the
canvas too.

**Exit:** the eight products look like cloth in light and dark colors
(the owner checks the pictures of `write_products`); a color drag on the
canvas stays smooth.

## Milestone 2: several placements (M)

A product shows more than the open design, for example a left chest logo
with a name on the back or a sleeve.

- `ProductView::extras`: other designs (a project or a machine file) at a
  placement or a free place. The app stitches each one once and keeps
  its stitches; the extras show on the canvas and in the saved picture,
  and do not change the open design.
- The Product window lists them: add, remove, place, show or hide.
- A warning if an extra is larger than the usual size of its place.

**Exit:** a polo with a left chest logo and a sleeve name, saved as one
picture.

## Milestone 3: front and back (S)

- A garment has a front and a back (T-shirt, polo, sweatshirt) and the
  product view knows which side shows. Each placement and each extra is
  on a side.
- **Both sides** in the saved picture: front and back next to each other.
- A switch in the Product window, and a key, to turn the product.

**Exit:** a T-shirt with a chest logo and a back print in one picture.

## Milestone 4: the product on the approval sheet (S)

- The approval sheet (PDF) gets a page with the design on the product
  (both sides with Milestone 3), after the page with the design at its
  real size.
- An option in Output Design: with or without the product.

**Exit:** an approval PDF with the product page opens in a PDF reader
and prints on A4 and Letter.

## Milestone 5: better products (M)

- The cap again: a front view of a six-panel cap with its seams, the
  button, the front panels, a curved brim, and the area of a cap frame
  (the design shows a little curved, as on the cap).
- New products: long-sleeve T-shirt (sleeve placements), hoodie, apron,
  beanie (cuff placement), and the front of a bag with a flap.
- Each new product has placements, folds and a fabric, and a test that
  its placements are on it.

**Exit:** each product looks right to the owner, and a test design at
each placement is on the product.

## Milestone 6: more colors (S)

- Two-tone products: a second color for trim parts (collar, cuffs, rib,
  the front panels or the brim of a cap): `ProductView::trim`, and a
  part shade `Trim`.
- Swatches of the usual blank-garment colors (black, white, navy, royal
  blue, red, heather gray, forest green, maroon, and so on), with
  generic names only.

**Exit:** a navy cap with a white front and a gray polo with a black
collar.

## Milestone 7: your own product photos, better (M)

- A mask for the photo: the part of the photo that takes the product
  color. A click on the product picks a color range; a brush adds or
  removes. The color goes on with the light and shade of the photo, so
  one white photo works in any color.
- Placements on a photo: the user marks the usual places once.
- Saved product templates: the photo, its real width, its mask and its
  placements, in a `products` folder next to the settings, with Export
  and Import (as My Machines).

**Exit:** a photo of a real shirt of the owner, saved as a template,
shows a design at its left chest in three colors.

## Milestone 8: a turntable view (M)

The product turns in steps, so the customer sees it from more sides. It
needs Milestone 3 (a front and a back).

- **A 3D shape from the drawing.** The outline of the front and of the
  back of each drawn product gets a soft round body (the round edges of
  Milestone 1, in 3D): a "pillow" of the product at its real size. A cap
  gets its own shape (a crown and a brim). No model files.
- **The design is exactly on it.** The front of the shape has the same
  flat coordinates as the canvas, so the placements, the sizes and the
  3D thread look do not change.
- **Steps made before.** When the design or the product changes, a
  background job draws the product at 24 angles (15° apart) in the
  engine (`emb-render`, no graphics card). A turn only shows another
  finished picture, so it is fast on a slow PC too.
- **Save.** An animated picture (GIF) of the turn, or a strip of three
  or four angles.

Not in this milestone: a garment on a body or a mannequin (it needs
real 3D models).

**Exit:** a T-shirt with a chest logo and a back print turns in steps in
the Turntable window (the canvas stays flat, ADR 0108) on a PC with the
software renderer, and saves as a GIF.

## Not in this plan

- Fabric that bends under the design (a design that follows a fold).
- 3D products that turn freely (Milestone 8 turns in steps).
- Pictures of brand garments.
