# ADR 0178: Faults found by the measures

Date: 2026-11-26 · Status: Accepted (the owner: "work through the faults
found by measurements you noted previously", the list of
`docs/design/open-work.md`). Changes ADR 0112 (how a fixed letter joins
its parts) and ADR 0167 (the edge rule of the travel measure), makes
the decimal comma of the fields of Object Properties (2026-11-24) a rule
for all number fields, and decides the three places of thread that
builds up that the open-work list gave to the owner (the owner gives
such technical choices to the developer).

## Context

These faults were still open. The measure of travel that shows (ADR
0167) and an agent test of the MCP server (ADR 0176) found the first two:

- In `embroidery-fonts-9`, a letter "a" of Fat Face Serif at 42.5 mm had
  a travel stitch of 11 mm that went back across the column sewn before
  it (4.2 mm of it on top of stitches). The font joins the two columns
  with no trim: the end edge of the first and the start edge of the
  second touch. The route of the font takes that a travel stitch along an
  edge lies on the end stitches of its column, so the needle can leave or
  come to a column at each point of the edge. The stitch engine made one
  straight stitch from the needle to the first point of the next column.
  The needle was at one end of the end edge, and the first point at the
  far end of the start edge (11 mm wide), so the stitch went across the
  first column.
- On a patch with lettering on a fill, with a trim distance of 8 mm for
  the lettering, a white travel stitch went 2 mm across the green fill
  from one letter to the next. The measure gave 0 mm: a travel stitch
  that crosses the stitches under it does not show at the edge of an
  area, where the needle path turns nearer than 1.2 mm, and the turns at
  the sides of the letters were near.
- The fields of Object Properties took `0,4` since 2026-11-24, but the
  other number fields of the windows (160 drag values and sliders) read
  only `0.4`. German and Spanish users type a comma.
- Thread that builds up (the density map, ADR 0044; the limit is 5
  layers on 4 mm² or more): the lace ornament template had twelve places
  (up to 12.7 layers), puff letters with points have places at the tops
  of letters (7 to 8 layers at 20 and 30 mm), and the monogram styles at
  60 mm have places of 7 to 12 layers. Measured for this ADR: a satin of
  lace on its mesh has about 5 layers with no joint; in the ornament,
  two side branches started on the middle line of their spoke and the
  spokes started inside the center ring (three satins on one place over
  the mesh). A puff column alone has 3 to 4 layers at its end (dense
  stitches that meet, and the end stitches that cut the foam); the
  columns of the fonts do not lie on each other at a joint (the ends are
  cut back, ADR 0153), so fewer stitches where they overlap changed
  nothing.

## Decision

1. **Along the edges.** Where a fixed letter joins two parts with no trim
   (`FixedJoin::Near`, the nearest points of the two edges), the travel
   goes from the needle along the end edge of the part before to its
   nearest point, to the nearest point of the start edge of this part,
   and along that edge to the first point (`object::along_edges`; the two
   points are travel of the run). Not when the straight line goes within
   0.3 mm of the two points already (`EDGE_WAY_MM`), and not when the
   parts that sew from now on cover the straight line.
2. **Only the turns of the object under the travel.** A turn of the
   needle path hides travel at the edge of an area only if it is a turn of
   the object of the stitch on top at that point (`Near::turn_near`). The
   turns at the sides of the letters no longer hide travel on a fill sewn
   before them.
3. **A decimal comma in all number fields.** Each drag value and slider
   of the program is made with `workspace::drag` or `workspace::slider`,
   which give egui the parser `workspace::parse_field`: a decimal comma or
   point, `+ - * /` and a degree sign, as the typed lengths; in a field of
   whole numbers, a comma, a point or a space between groups of three
   digits groups the thousands (`1,000`). A field with its own parser (a
   length with its unit) sets it after. The test
   `all_number_fields_go_through_the_parser` finds a drag value or a
   slider made another way. The help (Getting started, in four languages)
   says it.
4. **Thread that builds up.**
   - The lace ornament template is lighter: the spokes start at the
     outer edge of the center ring and the side branches at the side of
     their spoke, each 0.3 mm into it (`lace::JOIN`), and the loop lies
     on the outer half of the border ring, not on the mesh.
   - Free-standing lace and puff are dense by design: Design info allows
     `DENSE_BY_DESIGN` (3) layers more than the limit of the settings in
     a lace design and at a puff object (`info::build_up_limit`). The
     density map shows the limit of the settings, as before.
   - Monograms: not changed. The places of the large styles are in one
     column whose end the tuning takes to a corner and the envelope
     bends (see `docs/design/open-work.md`); a limit that grows with the
     letters would hide them.

## Consequences

- `embroidery-fonts-9`: travel on top of stitches 4.6 mm to 0;
  `embroidery-fonts-4`: travel on the fabric 0.9 mm to 0.4 mm. Eight
  golden snapshots of lettering (`embroidery-fonts`, `-2`, `-3`, `-4`,
  `-7`, `-9`, `lettering-bridge`, `view-towel`) have 1 to 5 more stitches
  (the points along the edges) and the same trims; the 3D pictures of
  `view-towel` and of the fabrics (the same lettering) change.
- `lace-edge`: travel on top of stitches 1.4 mm to 11.6 mm: travel of the
  mesh over the mesh, which the turns of the scallop satins sewn after it
  hid. Lace has no warning for travel (all of lace is thread), so no
  warning comes of it.
- A number typed with a comma no longer leaves the field as it was.
- The lace ornament: no warning (12 before); 336 fewer stitches, and the
  loop is 1.5 mm higher (the golden snapshots `lace-ornament` and
  `view-lace`, the travel measure and the 3D pictures of `view-lace`).
  Puff letters in Block Sans at 30 mm: no warning. The puff tutorial
  expects no warning now, and it and the lace tutorial say why (four
  languages); Options says the limit of lace and puff.
- Tests: `a_fixed_letter_goes_along_the_edges_where_two_columns_meet`
  (no point of the travel in the first column, with the needle at either
  end of its end edge), `the_turns_of_another_object_do_not_hide_travel`,
  `number_fields_take_a_decimal_comma` and
  `a_typed_decimal_comma_sets_the_field` (a click, `2,5`, Enter),
  `lace_and_puff_have_a_higher_limit_of_build_up`.
