# ADR 0074: Tangent circle, slot kinds, chamfer kinds, stretch, automatic dimensions

Date: 2026-10-27 · Status: Accepted (P14, the owner's fourth request)

## Context

The next sketch features of the reference CAD program (the table in
`docs/design/p14-plan.md`): a circle tangent to three lines, the
three-point center rectangle, the other kinds of slots, chamfers with two
distances or a distance and an angle, stretch, center points, and
automatic dimensions. The Sketch tab also became too wide for a window of
about 1450 points: the last panels folded.

## Decision

1. **Tangent circle** (`Sketch::add_tangent_circle`): the circles that
   touch three lines (their extensions too) are the solutions of three
   linear equations n·c − σ·r = d for the four sign sets σ; the program
   takes the one with its center nearest to the middle of the three
   clicks, with a tangent constraint to each line. The tool takes lines
   only (not their points).
2. **Three-point center rectangle**: the center, the middle of a side
   (direction and half width), then a corner (half height); the sides
   perpendicular and parallel, the center on a construction diagonal.
3. **Slot kinds** (a choice in the Properties panel, as `SlotKind`):
   center to center, overall (the two ends; the centers a half width in),
   center point (the middle and one center), three-point arc and center
   point arc. A curved slot has two side arcs round the center of its path,
   two end half circles tangent to them with equal radii, and its path as a
   construction arc.
4. **Chamfer kinds** (`ChamferKind`): equal distances, two distances (the
   first on the first line of the corner), or a distance and the angle to
   the first line (an angle dimension).
5. **Stretch**: a drag of a box takes the points in it (also the ends of
   curves); a base point and where it goes move those points as a drag;
   the curves that end at them stretch.
6. **Center points** (`SketchPoint::center`): a format; the point shows as
   a cross. For later milestones (holes, eyelets).
7. **Automatic dimensions** (`Sketch::auto_dimension`): for the selected
   curves (or all), first horizontal and vertical for lines within 0.5°,
   then lengths, diameters of circles and radii of arcs, then the angles
   between lines that meet, each with the size of now and only if it does
   not repeat or conflict (a test on a copy with the analysis). The place
   of the sketch stays free; the message gives the free motions.
8. **The constraint commands are a grid of icons** on the Sketch tab (a
   custom ribbon item, `constraint_icons`), as the other program shows
   them; the names are the tooltips and the screen reader names. So the
   Modify and Copy panels do not fold at about 1450 points.

## Consequences

- The tangent circle chooses its side by the clicks; to get another
  circle of the four, click near it.
- Automatic dimensions do not add a fix: the user places the sketch.
