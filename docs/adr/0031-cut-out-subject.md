# ADR 0031: Cut out the subject of a picture

Date: 2026-10-23 · Status: Accepted (P6, owner request)

## Context

After Milestone 8 the owner asked (2026-10-23) for a raw erase that takes
large parts out of a picture, for more polish in the Background step, and
for a better way to find the subject of a picture "without excess weight".
The color tools (click a color, remove the edge color) work on plain
backgrounds; a person in front of a busy background needs more.

Two ways were looked at:

- **A trained segmentation model** (a neural network that finds people or
  the main subject). It gives the best results on hard photos, but it
  needs a model file of about 5 MB to more than 100 MB and an inference
  library; a pure-Rust one exists (`tract`), so the FFI rule (ADR 0001)
  does not stop it, but it adds weight, build time and a file that the
  repository did not make.
- **GrabCut**, a published graph-cut method: the user drags a box round
  the subject, color models of the subject and the background are
  learned, and a minimum cut follows the edges. It needs no data and no
  dependency, and the user corrects it with marks.

## Decision

- **GrabCut now** (`emb_art::cutout`, with our own max-flow of Boykov and
  Kolmogorov in `emb_art::maxflow`): five Gaussians in RGB for each side,
  8 neighbors, five rounds, on a copy at most 320 pixels on its long side;
  at the full size the pixels near the edge are decided by the color
  models; small islands and holes are cleaned up. Deterministic (ADR
  0003).
- **The Background step** of the Picture to design wizard has three groups
  of tools: Smart (Cut out subject: drag a box; Mark keep and Mark remove:
  paint a line, then it cuts again), By color (click a color), By hand
  (erase and restore brushes; Box and Lasso, each to erase inside, keep
  only inside or restore inside). The edge-color button and the clean-up
  settings stay.
- **Polish**: the preview zooms (scroll, about the pointer) and pans
  (right or middle drag), with a Fit button; the removed parts show
  faintly under the checker (can be turned off); a box, a lasso or a
  stroke starts where the button went down; Ctrl+Z in the Background step
  undoes the last change of the picture (also the marks), not a change of
  the design.
- **A trained model stays an option for later**, as an optional download
  that the app finds if it is there, so the program itself stays small. It
  needs its own ADR (the model, its license and its size) and an owner
  decision.

## Consequences

- A cut-out takes well under a second in the release build for the work
  picture of 1200 pixels.
- Very similar colors on both sides of an edge (dark hair on a dark wall)
  need marks; the color tools and the brushes still work after a cut-out,
  and the next mark makes the cut from the picture as it is then.
