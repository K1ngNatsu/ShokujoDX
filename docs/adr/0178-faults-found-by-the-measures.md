# ADR 0178: Faults found by the measures

Date: 2026-11-26 · Status: Accepted (the owner: "work through the faults
found by measurements you noted previously", the list of
`docs/design/open-work.md`). Changes ADR 0112 (how a fixed letter joins
its parts) and ADR 0167 (the edge rule of the travel measure), and makes
the decimal comma of the fields of Object Properties (2026-11-24) a rule
for all number fields.

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
- Tests: `a_fixed_letter_goes_along_the_edges_where_two_columns_meet`
  (no point of the travel in the first column, with the needle at either
  end of its end edge), `the_turns_of_another_object_do_not_hide_travel`,
  `number_fields_take_a_decimal_comma` and
  `a_typed_decimal_comma_sets_the_field` (a click, `2,5`, Enter).
