# Plan: roadmap phase P6 (artwork, vector tools and auto-digitizing)

Date: 2026-10-23 · Status: Accepted. Milestones 1 to 8 done in software; exit tests open. Audit: `docs/design/roadmap.md`, "Audit of P6".

## Goal

Vector artwork and bitmap pictures become embroidery in the same canvas,
with no separate graphics mode (roadmap P6). The user can keep artwork as
vectors with no stitches, shape it (weld, cut, offsets, arrangements),
and turn it into embroidery objects by hand, by a click on a shape, or for
the whole picture at once. The exit test: a two-color logo PNG becomes a
sewable design with less than 10 minutes of manual touch-up, and a photo
becomes a recognizable stitch portrait.

The owner requests of 2026-10-22 that wait for this phase:

- **Border trace**: a border round an object with a thickness and a
  thread color, which cuts the object and its border out of the objects
  under it; a gap option (no border stitches, only the cut, so the fabric
  shows as a line); a choice of sew order (fill then border, or border
  then fill). Changes of thickness or gap update the cut.
- **Vector export**: object outlines as SVG paths.

The owner started this phase on 2026-10-23 before the P5 sew-outs (kit in
`out/p5-sewout`): all earlier sew-outs passed, so the owner does not
expect the P5 output to fail, and will sew it later. The exit tests of P1
(sew-outs), P2 and P3 (users) are also open.

Sources for what each tool does: the public documentation of commercial
embroidery products and common practice, in our own words (clean-room
rule). The tracing, color reduction and photo-stitch methods come from
public algorithms (marching squares, curve fitting, k-means, error
diffusion), written by us.

## Order

Each milestone ends with something a user can try in the app.

| # | Scope | Exit test |
| --- | --- | --- |
| 1 | **Vector objects.** A new object kind for artwork with no stitches (ADR 0024): closed and open paths with a fill color and a stroke. Draw tools can make vectors. SVG import keeps paths as vectors (a third import mode). Convert vectors to embroidery (fill, satin column from a closed shape or line, outline run) and embroidery to vectors. Show, hide and dim artwork. Export object outlines as SVG paths (owner request). | Import an SVG logo as vectors, convert it to fills and outlines, sew-out preview; export the outlines as SVG and open them in a vector program. Golden test of the conversion. |
| 2 | **Shaping.** Weld, intersect, exclude, front minus back, back minus front, flatten (remove the hidden parts), divide (all pieces that do not overlap), combine (one object with several shapes); keep originals; an overlap amount so pieces that touch do not show a gap. For vectors and for embroidery objects with an outline. | Corpus design `shaping` with a golden snapshot. |
| 3 | **Offsets and arrangements.** Simple offset (in and out), open offset (round a line), outline offsets (several rings), add border; mirror-merge (horizontal, vertical, both), array (rows and columns with spacing), reflect, wreath (copies round a center), kaleidoscope; automatic knife (cut a fill along lines with an overlap); remove overlaps (cut the objects under a shape, also for appliqué). | Corpus design `arrangements`. |
| 4 | **Border trace (owner request).** One command on the selection: border thickness, thread color, gap (knockout), sew order (fill then border, or border then fill). A record links the object, its border and the cut, so a change of thickness or gap updates them (as blends and monograms, ADR 0019 and 0023). | Corpus design `border-trace` (border, gap, both sew orders); sew-out on the Topaz 20. |
| 5 | **Bitmaps and more vector formats.** Backdrop crop (rectangle, ellipse, polygon; finalize), adjust (brightness, contrast, sharpen), color reduction ("prepare colors": a number of colors, merge small specks, clean edges), match the colors to a thread chart. Vector import: PDF and PDF-based AI files, EMF and WMF (pure-Rust readers). | Reduce a test picture to three colors; import a PDF and an EMF test file as vectors. Unit tests on generated pictures. |
| 6 | **Auto-trace and click-to-digitize.** Trace a reduced picture to vectors (contours, smooth curves, corners kept). Click a color area: fill (with or without holes), satin column (narrow areas, from the center line), center-line run, outline run. | A generated two-color picture traces to the expected shapes; click-to-fill and click-to-satin give sewable objects. Golden tests. |
| 7 | **Smart digitizing.** The whole picture at once: instant (automatic) and guided (choose the colors to keep, merge colors, the stitch type for each color, the order, a preview before OK). Stitch type from the size and shape of each area, angles from the shape, sew order with few trims, underlay from the fabric. | The exit test logo: a two-color logo PNG becomes a sewable design in less than 10 minutes of touch-up. Corpus design `smart-logo` from a generated picture. |
| 8 | **Photo digitizing and stitch processing.** Line-row photo stitch (rows that change width with the brightness), multi-color photo stitch (thread colors from the picture, layered rows), textured photo stitch (random curved runs, denser where dark). Process stitches (scale a stitch object and keep its density); recognize outlines of stitch objects (trace the area that the stitches cover). | The exit test photo: a portrait becomes a recognizable stitch portrait. Corpus design `photo` from a generated picture. |

Then the exit test (roadmap P6 exit): the logo and the photo, sewn on the
Topaz 20, with photos.

## Progress

| # | State | Notes |
| --- | --- | --- |
| 1 | Done in software | `ObjectKind::Vector` with `VectorStyle` (ADR 0024); `ObjectKind::sews`, `Design::sewn_color_blocks` (the plan skips vectors with no extra color change); `emb_model::vector`: `from_vector` (fill with holes, satin column from a line or along the long axis of a closed shape, outline), `to_vector`; `emb_geom`: `Path::subpaths`, `closed`, `joined`, `region::nest` (moved from the SVG reader). `emb_formats::svg::import_svg_vectors` (one vector for each SVG path, with fill, stroke and stroke width). `emb_render::svg::outlines` (object outlines as SVG paths, owner request). App: the canvas draws vectors in their colors under the stitches; an Artwork ribbon tab (after Digitize): Import (as vectors, picture, artwork), Draw (Draw artwork: the draw tools make vectors), Convert (To fill, To satin, To outline, To vector), Show (Vectors, Dim vectors, also on the View tab), Output (Export outlines); Import as vectors also in Digitize > Import and the File menu; Object Properties of a vector (fill, line, width, Convert); right-click Convert; break apart splits a vector into its shapes. Corpus design `vectors` with a golden snapshot. Not yet tried by hand in the app. |
| 2 | Done in software | `emb_model::shaping`: `Design::shape` with `ShapeOp` (weld, intersect, exclude, front minus back, back minus front, flatten, divide, combine) and `ShapeOptions` (keep originals, overlap amount); `area_of` (fills, vectors with closed shapes or a fill, appliqué, satin columns by their area); results keep the kind, thread, group and functions of their owner (a satin column becomes fills); `emb_geom::region::xor`. App: Artwork > Shaping (the eight commands, Keep originals, Overlap), right-click > Shaping; one undo step, and no step when a command cannot run. Corpus design `shaping` with a golden snapshot. Not yet tried by hand in the app. |
| 3 | Done in software | `emb_model::arrange`: `Design::add_offsets` with `OffsetSpec` (distance out or in, number of outlines, spacing, corners, make a line, a vector or a satin border outside the outline); the areas of the objects are joined first, lettering by the area of its letters, lines get an outline round them; `Design::arrange` with `Arrangement` (mirror-merge right, below or both with a gap; array; wreath; kaleidoscope). `Design::remove_overlaps` (cuts the selection out of the objects with an area that sew before it; an object all under it goes). `emb_geom::region::offset_lines`. App: Artwork > Offsets (Offsets window, Add border, Remove overlaps, Knife) and Artwork > Arrange (Arrangements window, Mirror right, below, both, Reflect); the knife cuts fills with the overlap amount of the Shaping panel (automatic knife). Corpus design `arrangements` with a golden snapshot. Fix found by its machine-file test: a tie-in toward a run point nearer than 0.2 mm made lock stitches that a machine file puts in one hole; the tie-in now locks toward the next point past it (`monogram-templates` changes at two tie-ins). Not yet tried by hand in the app. |
| 4 | Done in software | `emb_model::trace` (ADR 0025): `BorderTrace` records in `Design::border_traces`; `Design::add_border_trace`, `update_border_trace`, `remove_border_trace`, `trace_of`; `TraceSpec` (border or gap, width, thread, sew order). The border is a line with a satin width, 0.4 mm over the edge of the objects; the objects under them are cut, and go 0.4 mm under the border; a gap cuts exactly its width. A change puts the cut objects back (with the stitch settings of their first piece) and cuts them again. App: Artwork > Border trace (window: border or gap, width, thread, sew order; Add; while a traced object is selected the values change the trace at once, one undo step for a drag; Remove). Corpus design `border-trace` (border, gap, border then fill) with a golden snapshot. Also fixed (owner report): the stitch cursor at the end of the sew-out stays at the end when the plan changes length, so the canvas does not jump while an object moves with the player open. Not yet tried by hand in the app; sew-out open. |
| 4+ | Done in software | Owner requests after Milestone 4: a border trace has a border width and a gap width (both, or one of them) and a border stitch (any column type, run, triple run); the traced objects and the border are one group. Linked shapes (ADR 0026): `emb_model::link` with `ShapeLink` records in `Design::shape_links`; Flatten, Divide and Remove overlaps stay linked ("Keep linked" in the Shaping panel, on by default), and border traces always: a move of a linked object (also a drag on the canvas) makes the cut again from the stored originals; the pieces keep their ids; Artwork > Unlink ends the links of the selection. Corpus `border-trace`: the third trace has a border and a gap. Not yet tried by hand in the app. |
| 5 | Done in software | New engine crate `emb-art` (ADR 0027): `adjust` (brightness, contrast, sharpen), `crop` (rectangle, ellipse, polygon), `reduce` (color reduction: k-means in Lab on a histogram, deterministic; specks; clean edges; `match_threads`). `emb_formats`: `pdf` (PDF and PDF-based AI through `lopdf`: paths, painting, colors, transform, form XObjects), `emf` and `wmf` (our own readers of the public specifications), `artwork::import_artwork` (one entry for SVG, PDF, AI, EMF, WMF; all give fills and lines, lines, or vectors). App: Artwork > Picture (Picture tools, Crop box, Crop oval, Crop shape); the Backdrop panel has Adjust picture and Prepare colors (with the nearest threads of the chart of the Threads panel and Add these threads); Import artwork takes SVG, PDF, AI, EMF and WMF (also by drag and drop). Unit tests on generated pictures and on PDF, EMF and WMF files written in the tests. Not yet tried by hand in the app. |
| 5+ | Done in software | Owner report after Milestone 5 (ADR 0026): a cut that leaves several areas of an object keeps it one object (one fill with several outer shapes; Divide still makes pieces), and To fill makes one fill of a vector with several shapes; the fill engine keeps an outer shape inside a hole (an island); To fill, To satin, To outline and To vector keep the link of a linked object (`Design::convert_linked`); Join, Break apart, the knife and Split at needle refuse linked objects. Golden snapshot `shaping` changes (Exclude gives one object). Speed (owner report: a linked drag of a fill with many holes was very slow): the travel search of fills tests lines against a grid of the edges (each cell and edge once), skips rings by their box, keeps the tests between corners, and skips corners that cannot be on a shorter path; the same stitches, about 15 times faster for a fill with 36 holes; while a drag goes on, a stitch plan that takes more than 30 ms is made again only when the drag stops for 0.15 s or ends (the shapes and the cuts move at once). |
| 6 | Done in software | `emb_art::autotrace` (ADR 0028): `AreaMap` (areas of one color; two pixels that touch at a corner join if their color is the less common), `trace` and `trace_at` (edges between colors traced once as chains from node to node, corners kept, middle points averaged, Schneider curve fit through `emb_geom::simplify::fit_points` and `fit_loop`), `background`; `emb_art::reduce::exact` (the colors of a picture as they are, up to 32); `emb_art::click` (`ClickStitch`: fill, fill with no holes, satin, center line, outline; satin and center line from `emb_letter::stroke`, with the rails kept inside the area). App: Artwork > Trace (Trace picture; the Click to stitch tool and five buttons for what it makes); the Backdrop panel has Trace and click to stitch (smoothing, corners, smallest area, trace the background too). Corpus design `autotrace` (a ring, a bar on a slant and a box made in code: a traced vector, click fill, click satin, click center line, click outline) with a golden snapshot. Not yet tried by hand in the app. |
| 7 | Done in software | Owner report after Milestone 6: auto-trace was rough on photos, there was no background removal, the trace tools were hard to understand, and a hidden backdrop still took clicks. The owner proposed a wizard (ADR 0029): Artwork > Trace > Picture to design, six steps with a preview each (Picture; Background: click a color to remove it, only the area or everywhere, tolerance, remove the edge color, undo; Simplify: design width, detail as mm a pixel shown as sharp pixels, colors, specks; Colors: leave out or merge, nearest threads; Stitches: Auto, fill, satin, outline or artwork for each color, sew order, overlap; Finish: a picture of the stitches, then the objects in one undo step, grouped, over the backdrop, with chart threads, backdrop hidden). `emb_art::background` (`remove_at`, `remove_edges` that follow slow shading, `trim`), `emb_art::smart` (`resample`, `final_colors`, `merge_colors`, `digitize` with `SmartStitch` and `SmartSpec`). Instant smart design is the wizard with its defaults. Also: the picture tools, Trace picture and Click to stitch work on a shown backdrop only (a hidden backdrop takes no clicks); Alt+V hides the selected objects on screen (View > Hide selected; they still sew; Alt+V with nothing selected shows all; the Sequence panel marks hidden objects); a picture file on the command line becomes the backdrop. Corpus design `smart-logo` (a shaded wall with noise, a ring, a leaf and a bar: edge color and the wall in the ring removed, 3 colors, Auto stitches) with a golden snapshot. Checked by hand in the app with a test photo; the logo exit test (less than 10 minutes of touch-up) is open. |
| 8 | Done in software | Owner requests after Milestone 7 (ADR 0030): Picture to design asks for a picture when there is no backdrop and is first in Artwork > Import and Digitize > Import; the wizard copes with a person on a busy, noisy background: background removal is edge-aware (averaged colors, an edge stops it, the reference cannot drift more than 2.5 tolerances, a pixel band at the edge; the top edge counts most), erase and restore brushes, shrink the edge, remove small bits and fill small holes, noise smoothing (median). Photo stitch (`emb_art::photo`): Rows, Color rows and Scribble, as the photo path of the wizard (Picture, Background, Photo stitch, Finish) with design width, spacing, angle, contrast, brightness, thread or colors; stitch objects, a trim at each gap longer than 2 mm. Recognize outlines (`emb_art::recognize`, Artwork > Convert): a stitch object becomes a fill of the area its stitches cover, so it scales with its density (process stitches). Corpus design `photo` (a portrait made in code in the three styles) with a golden snapshot; `smart-logo` changes by one stitch (the new removal). Checked by hand in the app with a made-up portrait on a busy background; the exit tests (a logo, a real photo, sewn on the Topaz 20) are open. |
| 8+ | Done in software | Owner requests after Milestone 8 (ADR 0031): Cut out subject in the Background step (drag a box round the subject; Mark keep and Mark remove lines cut again): GrabCut with our own Boykov-Kolmogorov max-flow (`emb_art::cutout`, `emb_art::maxflow`), no new dependency; Box and Lasso erase (erase inside, keep only inside, restore inside; `background::erase_shape`); the preview zooms and pans, shows removed parts faintly, starts drags where the button went down; Ctrl+Z undoes picture changes in the Background step. Checked by hand in the app on the made-up portrait (one box gives the person with the hair strands). A trained segmentation model is an option for later (an owner decision). |

## Decisions

- **Picture to design (Milestone 7, ADR 0029).** Guided smart design
  is a wizard with a preview in each step, as the owner asked: the work
  has a fixed order, and the user must see what each setting does.
  Background removal is a step of it (and its engine is in `emb-art`).
- **Auto-trace and click-to-stitch (Milestone 6, ADR 0028).** Each edge
  between two colors is traced once and shared by the two areas, so the
  traced areas have no gap and no overlap. The background (the most common
  color at the edge of the picture) is left out unless the user asks for
  it. `emb-art` uses the stroke finder of `emb-letter` for click satin and
  click center line (this replaces the plan to move the grid functions).
  Click to stitch is one tool with five buttons that choose what it makes.

- **Offsets and arrangements (Milestone 3).** An offset goes round all
  selected objects together (their areas joined), so touching shapes get
  one outline. A border made by Offsets sits outside its outline, so the
  satin does not cover the shapes. Arrangement copies come at the end of
  the sew order; with the originals they stay selected, so the next
  command can use them all. Remove overlaps cuts only objects that sew
  before the selection (they are under it); the overlap amount of the
  Shaping panel is also the overlap of Remove overlaps and of the knife.

- **Shaping (Milestone 2).** The sew order is the stack order: the object
  that sews later is in front. Weld, intersect, exclude and back minus
  front keep the back object; front minus back keeps the front object; a
  divided overlap goes to the front object (it is the one that shows).
  The overlap amount shrinks the cutter, so a cut piece goes under the
  shape that cut it. A fill result is one fill object for each separate
  area; a vector result is one vector with all areas. Curves become
  straight lines (0.05 mm tolerance). Lines and lettering do not take
  part (no area).

- **Vectors are objects in the design, with no stitches** (ADR 0024).
  They are in the object list, so they stack, group, lock, transform and
  undo as all other objects. The stitch plan, the machine files and the
  sew time skip them. The reference product has "vector outline" and "vector fill" stitch
  types; here, Convert does that job.
- **One canvas.** No graphics mode: the vector tools are in the ribbon
  next to the embroidery tools. The View tab shows, hides or dims artwork.
- **Engine work stays in engine crates.** Shaping, offsets and arrangements
  go in `emb-geom` (the boolean and offset functions are there) and
  `emb-model` (the commands on objects). Bitmap work, tracing and
  auto-digitizing go in a new engine crate (`emb-art`, ADR 0027), with no
  UI and no FFI. The click-to-satin center line and the satin letters
  share one implementation: `emb-art` uses the stroke finder of
  `emb-letter` (ADR 0028).
- **Readers are pure Rust.** PDF through a pure-Rust PDF reader (to choose
  at Milestone 5; the path operators are simple); EMF and WMF through our
  own reader of the path records. EPS needs a PostScript interpreter: not
  planned (save as PDF or SVG first). The CDR format (a graphics suite)
  has no public specification: not planned.
- **Deterministic.** Tracing, color reduction (k-means with a fixed
  start) and the photo stitches use no random numbers that change from run
  to run (ADR 0003): random-looking effects use the seeded hash of
  `emb_stitch::effects`.
- **Test pictures are made in code**, as the corpus designs are, so the
  repository has no picture files that we did not make, and the golden
  tests do not depend on a picture decoder.

## Not in this phase

Hooping and product backdrops (P7); the design library, clipart and
barcodes (P8); sequins and bling (P9); the leftovers of earlier phases
that moved to P10 (see the roadmap, "Audit of 2026-10-23").
