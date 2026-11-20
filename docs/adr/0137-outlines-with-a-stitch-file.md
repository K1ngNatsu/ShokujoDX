# ADR 0137: Outlines with a stitch file

Date: 2026-11-20 · Status: Accepted (P18 Milestone 7,
`docs/design/p18-plan.md`)

## Context

Recognize Objects (ADR 0134) makes objects from the stitches of a machine
file; the shapes of these objects come from the needle points (ADR 0132,
ADR 0133). Many programs can also save the outlines of a design as a
vector file. With the outlines and the machine file of the same design,
the objects can have the exact shapes of the design.

## Decision

- **Engine**: `emb_art::part_outlines`.
  - An `Outline` is a path, an area (a shape with a fill) or a line (a
    stroke only), the stroke width and the name.
  - Alignment: `fit_by_size` puts the center of the box of the outlines on
    the center of the box of the stitches, with a scale when the sizes
    differ by more than 5 % (a file in other units); `fit_two_points`
    gives the move, turn and scale of two pairs of points.
  - `split_areas` makes each area of an outline (an outer shape and its
    holes) an outline of its own.
  - `with_outlines(stitches, made, outlines)`: a made object goes with the
    outline that has 70 % or more of its needle points in it (in the area,
    or 0.8 mm or less from its edge; near a line: half the stroke width
    and 1 mm). When more outlines have nearly the same part, the smallest
    one takes it (a detail on a larger shape). The made object that covers
    most of the box of the outline decides the kind and gives the
    settings; the outline takes the made objects of that kind only, so a
    piece of another object inside it stays.
  - The objects: an area with fill stitches is a fill of the outline with
    the settings of the fill; an area with column stitches is a satin
    column of the outline (`SatinGeometry::from_outline`, with the ends and
    the rungs of the recognized column as the angle lines), a ring column
    for an area with one hole, else the satin of Convert (ADR 0041); its
    pull compensation is half the difference between the width of the
    stitches and the width of the outline. A line with run stitches is a
    run along the line; with column stitches, a satin column of the width
    of the stitches along it. The outline object takes the records of the
    made objects that it replaces and their place in the sew order; the
    other made objects stay, so no stitch is lost. The second value is the
    outline of each made object.
- **Window**: Recognize Objects has **Outlines…** (a vector file: SVG, PDF,
  AI, EMF, WMF, read as vectors), the name of the file with the number of
  outlines and of made objects, **Fit by Size**, **Align by Two Points**
  (click an outline point and the same point of the stitches, two times;
  each click snaps to the nearest outline point or needle point within
  3 mm) and **Remove Outlines**. The outlines are drawn on the picture of
  the file (the ones that made objects in a darker color), and the list
  marks the objects of outlines. Accept is as before (one undo step).
- **Exit test** (automated part, `crates/emb-cli/tests/outlines.rs`): each
  corpus design (lettering as its parts) is saved as outlines with Export
  Outlines and as DST; the DST opens as stitch objects, the outlines go on
  them by their sizes, and each outline becomes objects. The measured
  objects are the fills of rows, the satin columns, the runs and the lines
  with a satin border (not the travel runs inside letters: their stitches
  go with the column over them, ADR 0131).

## Consequences

- 92.8 % of the 1,819 measured objects become objects of their kind, and
  all the fills of their kind have the angle of their source within 5°.
  Most of the others are satins with a jagged edge or a hand-stitch effect,
  and columns of letters whose underlay goes out of their outline.
- The outlines of another program can have more shapes than the machine
  file (a background box, notes): an outline that takes no stitches makes
  nothing, and two points align the outlines when their boxes differ.
- Open: the exit test of the owner (a machine file and the outlines of a
  design of another program).
- The audit of P18 (2026-11-20): each stitch object (one for each color)
  is matched alone, so an outline round the design gave its shape to a
  small object of each color. An outline takes objects only if they
  cover a quarter of its box (`MIN_COVER`); the exit test gives 92.5 %
  (92.8 % before).
