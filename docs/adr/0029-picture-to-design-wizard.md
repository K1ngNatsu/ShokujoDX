# ADR 0029: Picture to design wizard

Date: 2026-10-23 · Status: Accepted (P6 Milestone 7, owner request)

## Context

After Milestone 6, the owner tried auto-trace on photos (2026-10-23).
The trace was rough on complex pictures, there was no way to remove a
background, and the trace and click tools were hard to find and to
understand: the user could not see what each setting would do. The work
has a fixed order (background, detail, colors, stitches), and each step
needs its own preview. The owner proposed a step-by-step wizard. This is
also what the P6 plan calls guided smart design (Milestone 7).

## Decision

- **One wizard, Artwork > Trace > Picture to design**, with six steps
  that each show their result next to their settings: Picture,
  Background, Simplify, Colors, Stitches, Finish. The step bar and
  Back and Next go to any step; a change makes only the later steps again
  (each result is kept with a key of its inputs).
- **Background removal** (`emb_art::background`): a click removes the
  color under it, only the area that touches the clicked pixel or the
  color everywhere, within a tolerance (Lab distance); a button removes
  the most common edge color where it touches the edges. The connected
  removal follows a color that changes slowly, so a shaded wall goes and a
  sharp edge stops it (made edge-aware for photos with grain in ADR 0030).
  Undo and Start again work inside the wizard; clear edges are cut off.
- **Detail is a pixel size in mm** at the design width. The picture is
  resampled to that size (colors averaged with their transparency, so
  the background does not bleed into the edges) and color-reduced
  (`emb_art::reduce`); the preview shows the pixels sharp, so the user
  sees what the trace gets.
- **Colors**: each color can be left out or merged into another color;
  the palette indexes stay, so choices and sew order stay when only the
  merges change. A new color reduction with other colors resets them.
- **Stitches** (`emb_art::smart::digitize`): Auto (a satin for a long
  area narrower than 5 mm, else a fill), fill, satin, outline or artwork
  (a vector). Fills grow by half the overlap amount (0.3 mm), so colors
  that touch overlap and no fabric shows when the stitches pull. The
  colors sew in the order of the list (the most common first; arrows
  change it); the areas of one color from the nearest one on.
- **Finish** shows a picture of the stitches and adds the objects in one
  undo step, placed over the backdrop picture (scaled about its center
  if the width changed), with the nearest chart threads of the Threads
  panel, grouped, and the backdrop hidden (both can be turned off).
- **The engine work is pure** (`emb_art::background`, `emb_art::smart`),
  so the corpus design `smart-logo` runs the same steps as the wizard.

## Consequences

- Instant smart design (one click, no choices) is the wizard with its
  defaults: open it and press Finish on the last step.
- The wizard works on the picture at most 1200 pixels on its long side,
  so each step stays quick; the detail step cannot ask for more pixels.
- Photo stitch (rows that follow the brightness) is still Milestone 8;
  the wizard makes flat color areas.
