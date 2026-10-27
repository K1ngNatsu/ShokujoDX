# ADR 0053: Picture to design: layers, fill angles, small areas, checks

Date: 2026-10-25 · Status: Accepted (P12 Milestone 2)

## Context

Picture to design made each color area a fill with a hole for each
detail in it, sewed all fills at the angle of the current fill settings,
and made fills with underlay of the smallest areas. A fill with many holes
breaks into parts, needs travel round each hole, and a gap opens at each
hole when the stitches pull. Fills that touch at one angle look like one
area. Problems showed only after Finish, in Design info. Plan:
`docs/design/p12-plan.md`, Milestone 2. The methods are common digitizing
practice, in our own words.

## Decision

All in `emb_art::smart::digitize`, with new settings of `SmartSpec` (and
of the Stitches step):

1. **Layers** (`layer_max_mm2`, 25 mm² at the start; 0: off). A hole of
   an area is no hole if it is at most this size and the areas in it fill
   it (90 % of its area or more) and sew later (their colors are later in
   the sew order). The area sews over it and the details sew on top. A
   hole with a color that sews earlier, that is left out, or with clear
   pixels stays a hole. To find the areas in a hole, each traced area has
   a point well inside it (`Area::inside`: the pixel farthest from its
   edge, from the distance map of ADR 0052), in a grid of 5 mm cells.
2. **Fill angles** (`fill_angles`, on at the start). The long axis of an
   area comes from the second moments of its outline and holes; an area
   less than 1.3 times as long as wide has none and keeps the angle of the
   current fill settings. A fill that touches a fill made before it (the
   areas share a pixel side, `AreaMap::touching`) gets the angle nearest to
   its axis that is 30° or more from those fills (`ANGLE_APART_DEG`); the
   tries are 0, ±30, ±45, ±60 and 90° from the axis.
3. **Small areas** (`small_mm2`, 4 mm² at the start). Auto makes a smaller
   area a satin, or a run along its middle if it is narrower than 0.8 mm
   (`RUN_MAX_MM`). A fill this small (no satin is possible, or the color is
   a fill) has no underlay. Specks smaller than the speck size of Simplify
   still go into the color round them before this.
4. **Checks on the Finish step** (`wizard::Check`, made by the background
   job with the stitch picture): thread build-up over the density limit of
   the Options, more than 30 trims, objects smaller than 1 mm, stitches
   longer than 7 mm, and two kept colors nearer than ΔE 8. Each has a fix
   button where a change of the settings helps (less overlap or no
   details on top; less detail; larger specks; Auto satin up to 5 mm;
   merge the less common color into the other). A fix changes the
   settings, and the job makes the stitches and the checks again.

## Consequences

- On the bench (ADR 0052) the stitches go down on four of the six old
  pictures (by up to 12 %) and stay the same on two, at the same or a
  smaller color difference; the new `badge` picture (a disk with 18 small
  details) has 9 % fewer stitches with layers than without, and its stitch
  plan is about 4 times quicker. The trims stay the same: they come from details of one color
  that are apart, not from holes.
- The golden snapshot of `smart-logo` changed: its leaf sews along its
  long axis.
- Details on top add layers of thread where they are. The build-up check
  shows it if it is too much.
