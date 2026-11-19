# ADR 0123: Light as light, and threads with a kind and a weight

Date: 2026-11-19 · Status: Accepted (Milestone 2 of the second part of
P18, `docs/design/p18-view-plan.md`)

## Context

The pictures of the 3D view mixed colors as the numbers of the screen
(sRGB): the soft edge of a thread, its shadow and a thread thinner than a
pixel were a mix of numbers, which is darker than the mix of light that
the eye sees. All threads were the same: 0.4 mm wide in the pictures,
0.45 mm on the canvas, with the same highlight. A thread had a color, a
name, a brand and a code, but no kind and no thickness.

## Decision

- **Colors mix in linear light** in `emb-render` (`emb_render::light`):
  the picture of the stitches (`raster`), the design picture on the
  product, and a picture on a PDF page over white. A mix changes each
  8-bit sRGB value to linear light with a table, mixes, and goes back to
  the nearest sRGB value with the 255 midpoints between the values, so
  the result is exact and the same on each system. A pixel that a thread
  covers fully keeps its color, so the flat look keeps its colors (a
  test). The shadow of a thread in the 3D look is 0.24 in linear light
  (0.12 of the screen numbers before), so a shadow on a light fabric is
  as dark as before. The light model of the product view (ADR 0101) is
  not a mix of two colors and stays as it is; the canvas is drawn by the
  graphics library and stays as it is until it shows tiles (Milestone 5).
- **A thread has a kind and a weight** (`emb_model::Thread::kind`,
  `weight`; the same in `ThreadLook`, so a colorway has them too). The
  kinds (`ThreadKind`): polyester (the default), rayon, cotton (and other
  matte threads), metallic, wool (and acrylic), glow in the dark. The
  weight is the weight number of the thread, 40 by default, from 8 to
  150 (the project check refuses others). A file of an earlier version
  reads with the defaults, and the defaults are not written, so its file
  stays the same.
- **The width of a weight**: `thread::width_factor` is the square root
  of 40 divided by the weight (a weight number is a length for each mass,
  so the section of a thread goes as one over the weight), and
  `thread::width_mm` is 0.4 mm times it. A 60 weight thread is 0.82 times
  as wide as a 40 weight thread, a 12 weight thread 1.83 times.
- **The views use them.** Each segment (`emb_render::Segment`) has the
  kind and the weight of its thread. The pictures and the canvas (flat
  and 3D thread look) give each stitch the width of its weight. The 3D
  look gives each stitch the highlight of its kind
  (`thread3d::shine_of`): rayon 1.25 and metallic 1.8 times the highlight
  of polyester, glow 0.6, cotton and wool none. The stitches and the
  machine files do not change.
- **The thread charts** have a kind and a weight, each with its source
  (`ThreadChart::kind`, `weight`; `data/threads/kinds.toml`). The source
  of the charts of the program is the name of the line, as the maker
  names it: 38 charts say polyester or rayon (the words Polyester, Rayon,
  Viscose, Poly or Polyneon), Madeira Matt and Aurifil Mako are matte,
  Aurifil Lana and Madeira Burmilana are wool, and one Simthread chart
  glows in the dark: 43 of the 75 charts. No chart
  name gives the weight ("Brothread 40" is a set of 40 colors), so all
  charts have the default weight. A chart with no source has the
  defaults. A chart thread gives its kind and weight to a thread
  (`chart_look`, Match to Chart, threads from a picture).
- **The Threads panel** shows and changes the kind and the weight of the
  current thread. For a weight that is not 40 and a thread with objects,
  a note says that the density is for a 40 weight thread, and the spacing
  to use: the width factor in percent.
- **The reference set** gets a thirteenth design, `view-thread-kinds`:
  one column of each kind and of weights 30 and 60, in one color. The
  columns are steps from the top left and the design starts there, so
  they all sew from the top down.

## Consequences

- The pictures of the reference set change at the edges of the threads
  and in their shadows. At the close and the normal zoom the brightness
  of an area changes by at most 5.5 %; at the zoom of the whole design,
  where a thread is less than a pixel and each pixel is a mix, by up to
  21 %, always lighter (`docs/design/p18-view-m2-report.md`).
- The bench found a fault of the view of today that this milestone does
  not change: a satin column that sews upward is darker than one that
  sews downward (128 and 178 of 255 for the same thread), because each
  stitch is painted over the one before and in a column that sews up its
  dark lower edges show. The surface of Milestone 3 removes it.
- The canvas and the pictures of a design with threads of other weights
  look different from before; designs with the defaults look as before
  except at the edges of the threads in the pictures.
