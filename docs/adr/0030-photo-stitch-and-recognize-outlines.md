# ADR 0030: Photo stitch and recognize outlines

Date: 2026-10-23 · Status: Accepted (P6 Milestone 8)

## Context

Roadmap P6 Milestone 8 adds photo digitizing (a portrait becomes a
recognizable stitch picture) and stitch processing (a stitch object can
be scaled and keep its density). The reference product has three photo styles:
rows whose width follows the brightness, a multi-color kind, and a
texture of random runs. The owner asked (2026-10-23) that Picture to
design opens from a picture at once, and found that the wizard did not
cope well with a person on a busy, noisy background.

## Decision

- **Photo stitch is a path of the Picture to design wizard** (ADR 0029):
  the Picture step asks for color areas or a photo stitch; the photo
  path is Picture, Background, Photo stitch, Finish. The background
  tools are the same.
- **Three styles** (`emb_art::photo`, our own methods):
  - Rows (one thread): parallel rows at an angle you choose; along a row
    the needle zigzags as wide as the row spacing where the picture is
    dark and runs straight where it is light, with shorter stitches where
    it is dark.
  - Color rows: the picture reduced to 2 to 8 thread colors; each color
    gets its own rows (at its own angle) whose zigzag width is the part of
    that color round each point. Light colors sew first.
  - Scribble (one thread): up to four points in each cell, four times
    the square of the darkness, joined nearest to nearest with short
    stitches.
- **Settings**: design width, row (or point) spacing, row angle,
  contrast, brightness, the thread (or the number of colors), noise
  smoothing (a median). The preview shows the stitches.
- **Photo stitches are stitch objects** (`ObjectKind::Stitches`, one for
  each color), as stitches from a machine file. A gap up to 2 mm is one
  stitch; a longer gap is a jump with a trim, because a VP3 file has no
  jump without a trim (it would sew the jump).
- **Recognize outlines** (`emb_art::recognize`, Artwork > Convert): the
  area that the stitches of a stitch object cover is drawn into a grid of
  0.2 mm (a thread is 0.4 mm wide), gaps up to two cells close, and the
  area is traced (`autotrace`). The stitch object becomes a fill in its
  thread with the current fill settings. Then scaling makes the stitches
  again with their density: this is "process stitches".
- **Picture to design asks for a picture** when the design has no
  backdrop, and is the first command of Artwork > Import and Digitize >
  Import.
- **Background removal is edge-aware** (`emb_art::background`, owner
  report): colors are compared after an average over 5 × 5 pixels, so
  grain does not count; where the averaged color changes more than 0.35
  tolerance a pixel there is an edge and the removal stops; the reference
  color follows slow shading but stays within 2.5 tolerances of the first
  color, so it cannot creep from a light wall into dark hair; a band of a
  few pixels along the edge is then checked pixel by pixel. The edge color
  counts the top edge four times and the bottom edge once, as a subject
  usually stands on the bottom edge. New tools: an erase and a restore
  brush, shrink the edge, remove small bits and fill small holes, and
  noise smoothing before the color reduction.

## Consequences

- Photo stitches are not editable as shapes: change the settings in the
  wizard again, or use Recognize outlines for a plain fill.
- Photo stitches with a removed background make many trims (one at each
  gap longer than 2 mm). The app density check warns about the dark
  parts of a photo stitch, which are dense on purpose.
- The `smart-logo` golden snapshot changes by one stitch and a slightly
  different green: the edge-aware removal leaves other edge pixels.
