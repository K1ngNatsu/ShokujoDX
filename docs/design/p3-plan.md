# Plan: roadmap phase P3 (production, color and quality)

Date: 2026-10-21 · Status: Accepted. Milestones 1–8 done in software; the exit test with users is next.

## Goal

What a shop needs to produce a design and hand it over (roadmap P3). The
exit test: five test users complete three set tasks with no help, and one
command makes the machine file, the project file and the worksheet.

The owner started P3 before the exit tests of P1 (sew-outs) and P2 (user
test) were done. Those tests stay open and run in parallel.

## Order

Each milestone ends with something a user can try in the app.

| # | Scope | Exit test |
| --- | --- | --- |
| 1 | **Threads and color.** Thread charts (built-in, imported GIMP palette and CSV files), Threads panel (edit, search a chart, match design colors to a chart, remove unused), colorways (several color versions of one design) with an editor, background (fabric) color, cycle used colors, split a thread. A Color tab on the ribbon. | Match a design to a chart, make a second colorway with other colors, and export both. |
| 2 | **Machine functions and stitch editing.** Stop, trim and tie-off per object; object functions in the plan and in the Sequence panel; split an object at the needle; stitch list panel; jumps and speed are a later step if the formats need them. | Put a stop between two objects, see it in the stitch list, export, and find it in the file. |
| 3 | **Machine files and formats.** Open DST, VP3 and PES as stitch objects (one per color block), recolor, scale with stitch processing; read and write EXP and JEF; read SEW, XXX, HUS, VIP and SHV (pyembroidery is the reference). | Open a machine file, recolor it, add a running line, export to another format. |
| 4 | **Views.** Stitch player (play, pause, speed), 2D thread preview (thread width and shading), overview window, design information panel (size, stitches, colors, trims, sew time), warnings list with click-to-select (spec UX rule 6). | Play a design, find a warning in the list, and fix it. |
| 5 | **Output.** Production worksheet and approval sheet (print preview, PDF), design picture (PNG), design properties CSV, one "Output design" command for the machine file, the project file and the worksheet; output profiles and batch output. | One command makes the three files for the corpus patch. |
| 6 | **Fabrics, styles and templates.** Fabric presets (density, underlay, compensation), styles (saved property sets), make properties current and apply current properties, new design from a template, save as template; plain-language parameter labels (spec UX rule 2). | Change a design from cotton to towel in one step, then apply a style to three objects. |
| 7 | **Appliqué.** Appliqué object (placement line, stop, tack-down, cover satin), with and without holes, partial appliqué, combine appliqué, fabric display. | Digitize an appliqué patch that sews in the right order with stops. |
| 8 | **Workspace.** Design tabs (several designs open), split view, automatic backups and recovery, screen calibration, sew-time setup per machine; items moved from P2: snap to objects, corner radius per corner in a dialog, previous view and zoom factors, holes offset from the outline. | Recover a design after the app is closed by force. |

Then the exit test with users (roadmap P3 exit).

## Progress

| # | State | Notes |
| --- | --- | --- |
| 1 | Done in software | `emb_model::{chart, colorway}`: thread charts (GPL, CSV, CIE76 match, search), colorways with background color, cycle used colors, split thread, remove unused. `emb-app/src/threads.rs`: Threads panel (dock tab next to Sequence), chart import to the user folder. Ribbon tab **Color**. The canvas fills the hoop field with the background color. Export uses the current colorway. |
| 2 | Done in software | `emb_model::Functions` on each object: tie-in, tie-off, trim after (automatic, always, never), stop after. The plan applies them (`emb_stitch::plan`); VP3 writes a stop as a color block with the same thread. `emb-app/src/functions.rs`: functions in Object Properties and on the ribbon (Edit > Machine functions), split at needle (Edit > Split and join), Stitch list panel (View). The Sequence panel and the export summary show the functions. |
| 3 | Done in software | `emb-formats`: EXP, JEF, XXX read and write; SEW, HUS (with the decompressor), SHV read; `read_machine`, `write_machine`. EXP, JEF and XXX are byte-identical to pyembroidery for the corpus and a design with stops; DST and PES too (the label aside). `emb_model::import`: a machine file becomes one stitch object (`ObjectKind::Stitches`) per color block. The app opens machine files (File > Open) and inserts them (Digitize > Import > Insert stitches); export writes all six formats. Test `emb-cli/tests/machine_files.rs`: every corpus design in every format opens and sews the same needle points. |
| 4 | Done in software | `emb-app/src/player.rs`: stitch player (bar under the canvas; play, pause, start, end, color steps, speed, position slider, stitch, color and time). `canvas::thread_mesh`: Thread look (shaded threads with a shadow). `info.rs`: Design info panel (size, counts, sew time, colors with stitches and thread length) and the warnings list (outside the hoop field or margin, density, short stitches, many trims, no stitches) with click-to-select; the status bar shows the sew time and the warning count. `overview.rs`: overview window. |
| 5 | Done in software | `emb_render::{page, pdf, sheet, raster}`: pages as drawing steps, a PDF writer, the production worksheet and approval sheet, CSV, the design picture (PNG). `emb-app/src/output.rs`: Output design (one step for machine files, project file, worksheet, approval sheet, picture, CSV; output profile in the user settings), print preview with Save PDF, batch output; notes for the worksheets in Design info (`Design::notes`). `emb-cli output`: batch output with `designs.csv`. The PDFs parse in strict mode in pypdf and render in MuPDF. |
| 6 | Done in software | `emb_model::{fabric, style, template}`: fabric presets (`data/fabrics.toml`, ten fabrics) that change satin and fill settings by the difference between two fabrics; the current properties (`Design::current`), make current and apply current; styles in the design; templates (`.embtemplate`). `Underlay::Light` and `Underlay::Heavy` in `emb-stitch`. `emb-app/src/styles.rs`: Fabric window, Styles panel (dock tab), New from template and Save as template; the property bar edits the current properties. `emb-app/src/words.rs`: plain-language words and help texts for each stitch setting (spec UX rule 2). The worksheet, approval sheet and CSV show the fabric, backing and topping. `emb-cli --fabric` and `emb-cli fabrics`. |
| 7 | Done in software | `emb_model::applique`: appliqué object (`ObjectKind::Applique`) with pieces (outline, holes, edges with no cover) and parameters (placement line, tack-down run or zigzag, stop to cut, cover satin, fabric color). `emb_stitch::applique`: placement line, stop, tack-down, stop, cover, for all pieces together; runs can ask for a stop (`Run::stop`), and the plan lists the stops with their reason (`StitchPlan::stops`, `stop_notes`). `emb-app/src/applique.rs`: Appliqué tool (outline and holes), To appliqué (from fills), Combine appliqué, properties with edges for partial appliqué, fabric display on the canvas (View > Appliqué fabric). The worksheet lists the stops; sheets and the PNG picture show the fabric. Corpus design `applique-patch`. |
| 8 | Done in software | `emb-app/src/tabs.rs`: design tabs (each open design keeps its document, view, selection and stitch cache; New and Open use a new tab; close and quit ask for each unsaved design) and the split view (a second canvas with its own zoom and thread look). `backup.rs`: backups of unsaved designs every 2 minutes in a session folder with a heartbeat; after a crash the next start offers Recover or Discard. `setup.rs`: sew time for each machine (speed, color change, trim, stop; `emb_model::SewTimes`), screen calibration. `corners.rs`: corner radius dialog (all or each corner), hole from outline (a hole at a distance inside the outline), previous view, zoom factor list. Snap to objects (`Settings::snap_objects`). Exit test done on Windows: the app was killed by force with an unsaved design, and the next start offered it for recovery. |

## Added after the phase (owner request, 2026-10-22)

- **Design picture as SVG.** `emb_render::svg::picture`: each run of
  stitches in one color is one line in the thread color, in mm, over the
  appliqué fabric. Design > Output > Export SVG, the Output design profile
  ("Design picture (SVG)"), `emb-cli output --svg` and `emb-cli convert
  design.svg`.

## Decisions for Milestone 8

See ADR 0013.

- **Tabs swap state.** The active design keeps its state in the app state
  fields, as before; the other designs wait in slots. A tab change swaps
  the per-design state (document, file, view, selection, stitch cache,
  plan, filters, stitch cursor). The tools, settings, catalogs and
  clipboard are for all tabs, so copy and paste work between designs.
- **New and Open use a new tab**, so they never ask to save. A blank
  active tab (no objects, no changes, no file) is used again. A file that
  is open already gets its tab. Quit asks for each unsaved design in turn.
- **Split view** is a second canvas tab right of the first. It draws the
  same design with its own zoom, pan and thread look (on by default); the
  user can edit in either view.
- **Backups.** Every `backup_minutes` (default 2, 0 is off) the app writes
  each design with unsaved changes as a project file to
  `backups/<session>/`, and a heartbeat file about every 30 seconds. A
  normal exit deletes the folder; a saved or closed design loses its
  backup. At start, a session whose heartbeat is older than 2 minutes has
  stopped; the Recover designs window offers its designs. A recovered
  design opens in a new tab with unsaved changes and its old file name.
  The heartbeat (not a process id) finds crashed sessions, so no
  platform code is needed.
- **Sew time** is stitches at the machine speed, plus a time for each color
  change (30 s), trim (5 s) and stop (30 s). The user sets the times for
  each machine (Design > Sew time); they are in the user settings.
- **Screen calibration** is a factor on the screen size of 1 mm. The user
  changes it until a 100 mm line on the screen measures 100 mm, or types
  the measured length.
- **Corner radius** reads an outline of straight sides, where a curve
  between two sides is a rounded corner, so rectangles from the Round rect
  tool can be changed again. A radius is made smaller where a side is too
  short. Other curves (an ellipse) are not corners.
- **Hole from outline** offsets the outline inward (round joins) and adds
  it as a hole: a frame in one step.
- **Previous view** keeps up to 50 views from zoom commands and the Zoom
  tool (not from the mouse wheel). The zoom list sets 25 % to 3200 %.
- **Snap to objects** snaps a point to the nearest node of an object that
  is not selected, within the snap distance. It is off by default.

## Decisions for Milestone 7

See ADR 0012.

- **One object, three steps.** An appliqué object sews the placement line
  of every piece, then a stop, the tack-down of every piece, a stop, and
  the cover of every piece. Thus Combine appliqué gives fewer stops: two
  for any number of pieces.
- **Stops inside an object.** A run can ask the plan for a stop before it
  (`Run::stop`). The plan writes the stop where the run starts, and the
  thread is not cut. A stop before the first stitch of the design is
  dropped: with no placement line, the fabric goes on before the start.
- **The plan says why it stops** (`StopReason`): the stop function, put the
  fabric on, cut the fabric. The worksheet lists each stop with its color,
  object and what to do.
- **Geometry.** Placement on the outline; the tack-down 0.5 mm inside it
  (a running line, or a 1.2 mm zigzag); the cover a 3 mm satin that goes
  1 mm outside the outline. For a hole, "inside" is the fabric around the
  hole. The three steps of a ring start at the same point, and a step
  change on one ring is a stitch, not a trim, because the cover sews over
  it. The cover satin has no underlay by default: the tack-down is its
  underlay. A fabric preset changes the cover like other satin.
- **Partial appliqué** clears edges of the outline: one edge for each line
  or curve of the path (`outline_edges`). The cover sews the other edges as
  open columns. Holes always get a cover.
- **Fabric display.** Each appliqué object has a fabric color (light gray if
  none). The canvas fills it under the stitches with screen scanlines, so
  any shape with holes fills without a new dependency. The PDF sheets fill
  it with the even-odd rule, and the PNG picture fills it too.
- **Not in this milestone:** remove overlaps, cutter output (P8), more than
  one fabric layer.

## Decisions for Milestone 6

See ADR 0011.

- **Fabric change by difference.** A fabric has a spacing factor, a fill
  stitch length factor, a pull compensation offset and an underlay weight,
  relative to the base settings (woven cotton). A change from fabric A to
  fabric B multiplies the spacing by B/A and adds the pull difference.
  Thus each object keeps its own tuning, and a change back gives the first
  values again. Automatic underlay (normal, light, heavy) takes the weight
  of the fabric; no underlay and custom layers stay.
- **One undo step** changes the fabric, the current properties and (if the
  user wants) all satin and fill objects. Running stitches and stitches
  from machine files do not change.
- **Light and heavy underlay** are new automatic modes. Light: a center
  walk for wide columns, an edge walk for fills. Heavy: one layer more and a
  denser zigzag for columns; edge walk, the normal tatami and a second
  tatami along the rows for fills.
- **The current properties are part of the design**, not of the user
  settings, so a template can carry them. The property bar edits them as
  an undo step. The user settings keep the shape settings only (column
  width, corner radius, star points); the old stitch settings in a
  settings file are ignored.
- **A style is a copy.** Apply style copies the settings for each object
  type that the style has. A later change to the style does not change
  objects; Update takes the settings of the selection into the style.
  Styles are saved in the design and in templates (no separate library
  yet).
- **Templates** are project files with the extension `.embtemplate` in the
  `templates` folder next to the settings file. A template keeps the
  machine, hoop, fabric, current properties, styles, threads, colorways,
  guides and notes, and optionally the objects; not the backdrop. The
  user can select a template for New. The built-in template is "Normal".
- **Plain-language words** (`emb-app/src/words.rs`): each mm value shows a
  word (for example "0.36 mm (normal)") and each setting has a help text.
- **Fabric values** are starting points from general digitizing practice,
  in our own words. A test sew-out on the real fabric is still necessary.

## Decisions for Milestone 5

- **One layout, two outputs.** A page is a list of drawing steps in mm
  (`emb_render::page`). The PDF writer and the print preview draw the same
  list, so the preview is what prints.
- **Our own small PDF writer** (PDF 1.4): the standard Helvetica fonts (not
  embedded), WinAnsi text, vector lines; no new dependency. The stitches
  are vector lines with the thread color, so the picture is sharp at every
  size. A check script (not in the repository) reads every corpus PDF with
  pypdf in strict mode and renders it with MuPDF.
- **No direct printing.** Printing from the app needs the print system of
  each platform. The user saves the PDF and prints it from a PDF viewer.
- **Output design** writes `name.<format>` for each machine format of the
  profile (default: the format of the machine), `name.embproj` (always;
  it becomes the file of the design), `name worksheet.pdf`,
  `name approval.pdf`, `name.png` and `name.csv` as the profile says. A
  stitch outside the hoop field stops it, as it stops export.
- **Batch output** reads project and machine files (machine files become
  stitch objects) and writes the same files for each. A file that fails
  is reported; the others are written.
- **Dates** are local time (crate `chrono`, only in the app and the CLI;
  the engine crates stay free of system bindings). JEF headers get the
  local time too, as in pyembroidery.
- **The picture box** has the fabric color of the colorway, else light
  gray, so white thread shows on paper.
- **Barcodes** (Code 39, Code 128) wait: without a scanner to test with,
  a wrong table would print barcodes that do not scan.

## Decisions for Milestone 4

- **The player moves the stitch cursor.** All views that use the cursor
  (dimmed stitches, the Stitch list, select at needle) follow the player.
  Speeds are in stitches per second; a slow frame moves at most 0.25 s.
- **Thread look is 2D.** Each stitch is a 0.45 mm thread with a light
  middle, dark edges and a small shadow. When a thread is narrower than
  two screen points, the normal stitch view shows. A 3D look is P10.
- **Checks** (spec UX rule 6) work on each object from the stitch plan:
  a stitch outside the hoop field (an error: export stops), outside the
  safety margin, rows or columns closer than 0.25 mm, stitches shorter
  than the machine minimum, more than 4 trims in one object, and objects
  with no stitches. Errors come first. A click selects the object and
  zooms to it.
- **Sew time** is the stitch count at the machine speed, as in the export
  summary. Trims and color changes add time; the machine runtime settings
  of Milestone 8 add them to the estimate.
- **Thread length** is the length of the stitches on the fabric, per color.
  The thread that a machine uses is more (it goes through the fabric and
  the bobbin thread is extra); this is not estimated yet.

## Decisions for Milestone 3

- **Stitch objects.** A machine file becomes one `Stitches` object per color
  block. Its geometry is a path of straight lines: each subpath is one
  sequence of needle points, and a jump goes to the next subpath. A list
  says which subpaths start after a trim. Because it is a normal path,
  move, scale, rotate, mirror, recolor (thread) and Reshape work with no
  special code. Reshape on a stitch object is single-stitch editing: move,
  add and delete needle points.
- **Sewn as they are.** The plan keeps every needle point of a stitch object
  (also stitches shorter than the minimum), joins its parts with a jump or
  a trim as in the file, and adds no lock stitches (`functions` tie-in and
  tie-off are off; the lock stitches are in the file). Between objects the
  normal rules apply.
- **Stops in a file** end the object; the object gets "stop after" and
  "never trim", so the machine stops there again.
- **Scaling** a stitch object scales the stitches: the stitch length and
  density change. Process stitches (scale with the density kept) and
  recognize objects from stitches need the shape tools of P6.
- **Byte checks.** The new writers port pyembroidery's encoder rules that
  change bytes: the jump steps add up as floats and the writer rounds each
  move half to even; after a trim, stop or color change, a format with full
  jumps gets a jump to the next stitch. The DST and PES writers now use the
  same jump steps. A script (not in the repository) writes each corpus
  design with pyembroidery and with our writers and compares the bytes.
- **Multi-needle machines (after Milestone 8; ADR 0014).** TBF (Tajima)
  and TAP (Happy) read and write; DSB, DSZ (also read by BAI and Ricoma
  machines) and Toyota 10O and 100 read only. TBF and TAP are
  byte-identical to pyembroidery for the corpus (TAP against the records of
  pyembroidery's DST); the four readers give the same records as
  pyembroidery for 160 random files. A machine-file import now keeps a trim
  before a stop (TBF is the first written format that keeps stops).
- **No VIP.** pyembroidery has no VIP reader, and the spec says not to
  guess undocumented formats. **SEW, HUS and SHV** have no public sample
  files; their tests use files made from the reader rules. pyembroidery's
  stored-form HUS compressor writes the 16-bit count little-endian, which
  its own decompressor reads big-endian; our test helper writes it
  big-endian.
- **JEF dates**: the app and the CLI give local time (Milestone 5);
  `emb_formats::date_stamp` is a UTC helper. JEF has no trims: the machine cuts at a long jump, and the reader
  makes a jump longer than 3 mm a trim.

## Decisions for Milestone 2

- **Functions belong to objects, not to stitch positions.** The stitches
  are made again after each change, so a function at a stitch number would
  move or get lost. An object says what happens after it (trim, stop) and
  whether it has lock stitches. This covers the common cases: a stop for
  appliqué fabric, a trim between near objects, no trim between far ones.
  Functions at single stitches come with stitch objects (Milestone 3).
- **Trim after.** *Automatic* trims when the next object of the same color
  is farther than the trim distance of the machine. *Always* trims. *Never*
  keeps the thread: a long gap becomes a jump. A color change always trims.
- **Stop after.** No stop is written after the last object of a color
  block or of the design: the machine stops there anyway.
- **Stops in VP3.** VP3 has no stop record, and a machine stops at each
  color block, so a stop becomes a new block with the same thread. DST
  writes its stop code; PES writes a color change with the same color.
- **Split at needle** cuts the object under the stitch cursor: a line at
  the nearest point; a fill along its rows through the needle; a satin
  across the column, from the nearest point of one side to the nearest
  point of the other. A closed line or a satin ring cannot be split at one
  point; use the knife.
- **Stitch editing** of single stitches waits for stitch objects
  (Milestone 3): it breaks the link between an object and its stitches.

## Decisions for Milestone 1

- **Thread charts are data, not code.** A chart is a GIMP palette (`.gpl`)
  or a CSV file. The app ships one chart: the 64 colors of the Brother PEC
  palette (from pyembroidery, MIT), in `data/threads/brother-pec.gpl`. The
  spec has an open question about the license of commercial chart data, so
  users import other charts (for example the GPL palettes of Ink/Stitch).
  The app copies an imported chart to the `threads` folder in the user's
  configuration folder (ADR 0009), so it stays after a restart.
- **GPL lines** are `R G B name`. If the last word of the name has a digit,
  it is the thread code. CSV lines are `code,name,#rrggbb` (a header line is
  allowed).
- **Color match** uses the CIE76 color difference in CIELAB, which is near to
  what a person sees and simple to explain.
- **Colorways.** `Design::threads` always holds the colors of the current
  colorway, so the stitch plan, export and all views need no change.
  `Design::colorways` stores the look (name, color, brand, code) of each
  thread in each colorway, and the background color. A switch saves the
  current looks into the current colorway and loads the looks of the new
  one. A thread that a colorway does not know keeps its current look.
- **Background color** belongs to the colorway: the fabric changes with the
  thread colors. The canvas fills the hoop field with it.
- **Cycle used colors** moves the looks of the used threads one step round
  (thread 1 gets the look of thread 2, and so on), so the user can try
  color combinations quickly. It is one undo step.
- **Split thread** gives the selected objects a new thread with the same
  look, so their color can change alone.
