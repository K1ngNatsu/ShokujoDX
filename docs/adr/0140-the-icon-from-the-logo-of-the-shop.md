# ADR 0140: The icon from the logo of the shop

Date: 2026-11-20 · Status: Accepted (owner request; changes the icon of
ADR 0043 and ADR 0061)

## Context

The icon of ADR 0043 and ADR 0061 was drawn in code by
`tools/make-icon.py`: a wooden hoop on a night sky, a gold star in satin
stitches, a needle and a gold thread, and a simpler drawing for 32 pixels
and less. The owner asked for a simpler icon (2026-11-20). The new logo
was made by a friend of the owner for the other software of the
embroidery and print shop where ShokujoDX is used: a purple star in an
embroidery hoop with a needle and a thread, on a dark tile with round
corners. The owner gave it as an SVG file.

## Decision

1. **The logo is the source of the icon**: `data/icon/icon.svg`, as the
   owner gave it. `tools/make-icon.py` reads it and writes
   `data/icon/icon-<n>.png` (16 to 1024 pixels) and `data/icon/app.ico`,
   as before. The places that use these files do not change.
2. **No SVG library.** The logo is one mask of polygons with straight
   lines (`M`, `L`, `Z`; a white polygon adds, a black one removes), a
   rectangle with round corners and two colors. The tool reads only such
   a file with Pillow, and stops with an error for other SVG data. A
   picture of 1024 pixels from the tool and the owner's picture of the
   logo differ by 0.02 of 255 on average, and by at most 40 in any pixel.
3. **One drawing for all sizes.** The ring and the star stay clear at 16
   pixels, so there is no simpler drawing for the small sizes. The tool
   makes each size from a picture of 4096 pixels with premultiplied
   alpha, so the round corners get no light or dark edge.

## Consequences

- A change of the icon is a new `icon.svg` and a run of the tool; the
  pictures are in the repository, so a build does not need Python.
- An SVG file with curves, gradients or other elements needs another way
  to make the pictures (for example a pure-Rust renderer in a tool); the
  tool says so.
- Open (owner): the license of the logo. The earlier icon was made in
  the repository and is under its license (GPL-3.0-or-later). The new
  logo has another maker, so its maker and the owner decide if it is
  under the same license or under a note of its own (for example in
  `data/licenses`).
- The Weaver Star of ADR 0061 (the name) does not change; only the
  picture of the icon does.
