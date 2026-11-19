# ADR 0128: What is not thread in the 3D view

Date: 2026-11-19 · Status: Accepted (Milestone 7 of the second part of
P18, `docs/design/p18-view-plan.md`; the compare with the sew-outs of the
owner is open)

## Context

The surface of the 3D view (ADR 0124 to 0127) laid threads on a fabric.
Appliqué fabric and rhinestones were flat colored areas 0.3 mm high, the
cut lines of cutting needles showed as thread and cut nothing, lace showed
a fabric, a trapunto fill and a quilt were flat, puff columns had flat
tops and steep sides, and metallic and glow thread had only their sheen.
The plan asks for each of these to look as it does on a sew-out.

## Decision

- **Pieces** (`surface::Piece`, `PieceKind`): what is not thread is a
  piece with rings (even-odd), a color and a kind, in the coordinates of
  the strands. `strand::pieces(design)` makes them from the design (in
  output coordinates; the canvas moves them with `Piece::transformed`):
  - **Appliqué fabric** (`Fabric`): `PIECE_MM` thick, with the relief of
    its weave (`emb_model::AppliqueParams::weave`, twill if not set;
    **Fabric surface** in the appliqué window) and a cut edge: the fabric
    is lower in the last 0.15 mm (cut threads).
  - **Rhinestones** (`Stone`): at their real size, 0.9 of their radius
    high, a flat table and a crown of eight facets (planes). In the light
    step (`sheen`) a stone is glass: less diffuse light, a sharp highlight
    on a facet that faces the light, and the light that the foil at its
    back sends through the facets on the far side.
  - **Padding** (`Pad`): under a fill with trapunto (`FillParams::trapunto`),
    a dome 1.5 mm high that rises in 2.5 mm from its edge (a chamfer
    distance on the cells of the lay, kept as a `Floor`). The stitches lie
    on it and the fabric round them has its height.
  - **Holes** (`Hole`): the cut lines of cutting needles (ADR 0051), joined
    again from the parts of each needle; a closed line is a hole, an open
    one a slit 0.25 mm wide. The cut lines are not thread
    (`strand::Marks::cuts`). A hole is 1 mm deep (the edge puts its shadow
    in it), and `Tile::behind` is the part of a pixel that is a hole:
    there the view shows what is behind the fabric (`sheen::BEHIND`, a dark
    gray; transparent in a picture with no fabric, so a product shows).
- **The floor under a thread**: a thread is round across from the height
  of what is under it at the pixel (the pieces and the padding), not from
  0, so its edges stay on a piece or on padding.
- **Lace** (`Ground::lace`, a fabric with the underlay weight lace): no
  fabric shows (the canvas draws no fabric texture) and nothing puts a
  shadow on it.
- **A quilt** (`Ground::loft_mm`, `Fabric::loft_mm`; the new preset
  `quilt`, 2 mm): the fabric is higher far from the stitches, a round rise
  in 3 mm from the nearest thread (a chamfer distance in the finish of a
  block); the apron of a block has this reach. The slope of the fabric
  makes its light (the fabric pixels only, so a fabric beside a thread does
  not change).
- **Cross stitch**: a ninth weave, Aida (`Weave::Aida`, blocks with a hole
  at each corner, 1.814 mm, 14 to the inch), and the preset `aida`. A grid
  of the same size whose corners are at the holes puts the needle points
  in them.
- **Puff**: the foam under a stitch rises round from `FOAM_EDGE_MM` after
  each needle point in 0.8 mm, so a column is round across; the cap
  stitches (ADR 0050) are short and close the ends lower.
- **Metallic thread**: at 6 pixels for each mm and more, small glints of
  the foil (a place each 0.12 mm, about half of them, each turned at
  random up to ±34°), so they change with the light.
- **Lights off** (`Lighting::lights_off`, View > Thread Look > Lights
  off): the light of a dark room (4 %) on all but glow thread, which shows
  in its own color as its own light. The fabric of the canvas is dark too.
  Only with the fine surface: the meshes of today do not have it.
- Materials: `material::STONE` is 2 and thread starts at 3.

## Consequences

- The reference set has five new designs (`view-rhinestones`,
  `view-cutwork`, `view-lace`, `view-cross-stitch`, `view-quilting`, and
  their golden snapshots), and `view-thread-kinds` has pictures with the
  lights off. `view-puff` and `view-applique` show items 1 and 2.
- The surface keeps one more byte for each pixel (21).
- Of the pictures of the reference set before, the surface of
  `view-applique`, `view-puff` and (glints) `view-thread-kinds` changed;
  see the report. The tiles of a design with no pieces take as long as
  before: the new work is done only where there are pieces or a fabric
  with a height.
- Cross stitch on Aida at a small zoom is up to 4 % lighter than at the
  detail: the fabric that shows is the grooves between the blocks, and a
  small pixel has the mean of all of its fabric (the test allows it).
- A hole shows a flat dark color, not the product or the hoop under it on
  the canvas.
- Open: the compare of appliqué, puff, cross stitch and rhinestones with
  the sew-outs of the owner.
