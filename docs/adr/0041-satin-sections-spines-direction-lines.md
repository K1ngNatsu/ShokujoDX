# ADR 0041: Satin from shapes: sections, spines, direction lines and a shorter sew order

Date: 2026-10-24 · Status: Accepted (owner report during P10)

## Context

The owner sewed designs made from SVG files on the reference machine
and reported three problems:

1. Satin of an SVG shape makes areas with heavy overlap. They stress the
   machine and make thread build up.
2. There are too many jumps, in an order that goes to and fro, and the
   jumps cross stitches that are sewn or not yet sewn.
3. A beta tester asked for a stitch direction line: a line drawn through
   a shape, with the stitches across it (square to it or at an angle that
   the user sets).

A probe of seven SVG shapes (a C, an O, an E, a swoosh, a heart, a star and
nine dots) found the causes:

- The fill with the satin stitch type (and the other column types) cut
  the outline into two halves at its extreme points across the fill angle
  and used the halves as the rails of one column. The rails pair points
  by length, so in any shape with bends, arms or holes the stitches fanned
  out, went outside the shape and were cut into pieces with travel
  between them. In the E, a 0.5 mm cell had up to 250 stitches.
- Convert to satin made a turning column along the straight line between
  the two points farthest apart. In a C that line is outside the shape,
  and in a bar it is the diagonal. Each subpath became its own column, so
  the hole of an O was a second column on top of the first.
- Where an edge goes almost along the stitches, the satin spacing was
  measured along that edge, so up to ten times too many stitches went
  there.
- The sew order was the order of the subpaths in the file.

## Decision

1. **Column fills cut the shape into sections** (`emb_stitch::textures`).
   Scan lines at the fill angle, 0.25 mm apart, cut the shape (holes too)
   into sections as for tatami; in a section each line crosses the shape
   one time. The line ends are the rails and each line is a rung, so no
   stitch goes outside the shape or over another section. The spacing of
   a section is measured across the stitches (`Column::across`). Sections
   go a quarter spacing into the next section, because the first and the
   last satin stitch are slanted. They sew nearest first, with travel
   inside the shape from and to the middle of their end lines.
2. **Spines** (`emb_geom::spine`). The two grid cells farthest apart by
   paths inside the shape are its ends. The middles of the points at the
   same fraction of the two sides are a first line; one width from each
   end, the line goes on as it bends there to the outline, and a second
   pass from those ends gives the spine. `cross_lines` makes lines across
   a shape at an angle to any guide line, to the nearest edge on each
   side; a line that would cross the one before it is left out.
3. **Convert to satin** (`emb_model::vector`). Each closed shape with its
   holes becomes one object: a turning column along its spine (no hole),
   a ring column (one hole), or else a fill with the satin stitch type at
   the angle of the shortest stitches. A column is used only if the
   stitches that the engine will make stay inside the shape, the widest
   one is at most 3 times the median one, and the column covers 90 % of
   the shape (`sews_well`). The new objects go nearest first.
4. **Direction lines.** `FillParams::direction` (`DirectionLine`: points
   and the angle of the stitches to the line) makes the column types go
   across the line, with sections along it; the parts that the lines do
   not reach get sections at the angle of the whole line. On a satin
   column, the tool makes new stitch angle lines (`SatinGeometry::
   with_direction`); the rails stay. On other fills it sets the fill
   angle. The line moves, turns and mirrors with its object.
5. **Shortest order** (`emb_stitch::order`, Arrange > Sew order and
   the Sequence panel). In each color block, the next object is the one
   nearest to the needle of those that can go next; an object does not go
   before an object that it overlaps (by boxes), so the colors and what is
   on top do not change. The stitch engine sews each chosen object to find
   where the needle is next. The app keeps the new order only if it has
   fewer trims, or as many and shorter moves.

## Consequences

- On the probe shapes, the most stitches in one cell went from 250 to 15
  or fewer, the satin fills need 30 % to 60 % fewer stitches, and each
  shape sews with no trim inside it. On the nine dots, the moves between
  objects went from 515 mm to 221 mm. `emb-cli/tests/svg_satin.rs` keeps
  this from an SVG file to the stitch plan.
- Golden snapshots `patterns`, `texture-fills` and `vectors` changed: they
  have column fills and a bar converted to satin (now square across the
  bar, before 8.5° to the diagonal).
- A section ends half a scan line past its last line; at a round tip that
  can be up to about 0.1 mm outside the shape. Pull compensation is more
  than that.
- The sew order and the satin of a design that a user made before do not
  change until the user converts again or uses Shortest order; column
  fills sew the new way at once.
- The quality test of Convert to satin is a choice of numbers (1 % of
  stitches outside, a width ratio of 3, 90 % cover). A sew-out on the
  reference machine must confirm the result; the fill with the satin
  stitch type is the safe choice when a column fails the test.
