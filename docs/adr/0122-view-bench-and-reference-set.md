# ADR 0122: The bench of the 3D view and its reference set

Date: 2026-11-19 · Status: Accepted (Milestone 1 of the second part of
P18, `docs/design/p18-view-plan.md`; the sew-out of the set is open)

## Context

The upgrade of the 3D stitch view (P18, second part) changes how each
picture of the embroidery is made. Milestone 1 gives the work its tools
before the view changes: a set of designs that show what a sew-out shows,
pictures and numbers of the view of today, a compare of two pictures, and
the speed of the canvas on this computer, on a small computer and with
the software renderer. Nothing in the look of the view changes.

## Decision

- **The reference set is twelve corpus designs** (`view-satin-angles`,
  `view-fill-border`, `view-underlay`, `view-lettering`,
  `view-fill-over-fill`, `view-puff`, `view-applique`, `view-motif-fill`,
  `view-run`, `view-blend`, `view-towel`, `view-dark`). Each one is small
  (at most 120 × 110 mm), has a fabric color, and shows one item of the
  list "What a sew-out shows and the view does not" of the plan.
  `emb_cli::view::REFERENCE` gives what each one shows and the place of
  its close picture. They are in `corpus::NAMES`, so they have golden
  snapshots of their stitches as all corpus designs.
- **`emb-cli view-bench`** (`emb_cli::view`) makes the stitch plan, the
  segments and the pictures of each design at three zooms (the whole
  design at 2 pixels for each mm, where a thread is less than a pixel; at
  8 pixels for each mm; and a 16 mm square at 32 pixels for each mm), in
  the flat and the 3D look. It prints the time of each step and the size
  of the data that it makes (the stitch records, the segments, the
  pixels). The data size is not the memory of the process: a counter of
  the allocations needs `unsafe` code, and the data size is what a cache
  of tiles must keep. `--out` writes the 3D pictures, a page with them,
  the hashes and the pictures that changed since the run before (as the
  pictures of the windows, ADR 0110). `--stress` measures synthetic
  designs of about 10,000, 100,000 and 1,000,000 stitches
  (`emb_render::stress`, moved from the program so that the two benches
  use the same designs). `--sew` writes the files to sew the set.
- **The test `tests/view_bench.rs` keeps a hash of each picture** in a
  snapshot. The pictures are the same on each system: the 3D look uses
  only `emb_geom::math` and exact arithmetic (ADR 0003), and the debug
  and the release build give the same hashes. A change of a hash is a
  change of the look: the test writes the changed pictures to
  `target/tmp/view-bench`, and a new snapshot needs a note in the commit
  message (rule 7). CI runs the test on each push, and on Linux it also
  runs `emb-cli view-bench` and keeps the pictures as an artifact.
- **`emb-cli view-compare`** (`emb_render::compare`) puts two pictures
  side by side with their difference and prints the difference of the
  brightness of each area of a grid. The brightness is the relative
  luminance in linear light (the weights of sRGB). A mean of linear light
  is what the eye gets from an area at a distance, so the numbers of two
  zooms of one design can be compared (the exit test of Milestone 4). A
  photo of a sew-out goes on the picture with a place, a scale and an
  angle that the developer gives; the photos stay out of the repository.
- **`ShokujoDX --view-bench`** measures the canvas of today in a window
  of its own (`view_bench.rs`): no settings, no backups, no design. The
  stress designs move and zoom for 3 s in each of the three meshes of the
  canvas, with no vertical sync, and the program writes the frames in a
  second, the times of the frames and of the meshes to `view-bench.txt`
  in the settings folder (or to the file of `--view-bench=<file>`).
  `--renderer=software` measures the software renderer. A user with a
  small computer can run it from the installed program.

## A fault that the bench found

With about 800,000 stitches the program stopped: the 3D mesh of the
canvas needed 165 MB of vertices, egui-wgpu makes its vertex buffer two
times larger when a frame needs more, and the graphics library allows at
most 256 MB for one buffer. A frame that needs more than 128 MB of
vertices can stop the program; that is about 340,000 stitches that show
at one time in the 3D thread look, or 670,000 in the flat thread look.
Now `canvas::fitted_mesh` counts the stitches that show and uses the
next lighter mesh (3D, flat thread look, stitches) when the mesh can have
more than `canvas::MAX_MESH_VERTICES` (6,000,000) vertices; with too
many stitches for the lightest mesh it draws each n-th stitch. The canvas
and the other views use it. A design that shows fewer stitches gets the
same mesh as before.

## Consequences

- The numbers of today are in `docs/design/p18-view-m1-report.md`.
  With the graphics card of the development computer the canvas draws
  100,000 stitches in the 3D look at about 190 frames in a second, and
  800,000 stitches at about 24 (less than the 60 of the budget). The
  software renderer draws 10,000 stitches at about 4 frames in a
  second, and more stitches at about 1. A picture of the 3D look of
  800,000 stitches at 8 pixels for each mm takes about 10 s on one
  processor.
- The owner sews the set on two fabrics and takes the photos (item 5 of
  the milestone); the compare of each picture with its photo goes in the
  report then. The numbers of a small computer come from the owner too.
- The corpus has 79 designs. A change of the 3D look changes the
  snapshot of `view_bench`, not the golden snapshots of the stitches.
