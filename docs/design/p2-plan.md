# Plan: roadmap phase P2 (digitizing toolset)

Date: 2026-10-21 · Status: Accepted. Milestones 1–7 done in software; the user exit test is open.

## Goal

The professional input and editing tools (roadmap P2). The exit test: a new
user digitizes a two-color patch from a bitmap in less than 15 minutes with no
manual, and an experienced digitizer can make the corpus with the traditional
input methods.

## Order

The exit test is a tracing task, so the tracing tools come first: a bitmap
under the design, and a way to fix the shapes after they are drawn. Each
milestone ends with something a user can try in the app.

| # | Scope | Exit test |
| --- | --- | --- |
| 1 | **Tracing.** Backdrop bitmap (import PNG, JPEG, BMP; position, size, dim, hide, lock; saved in the project file). Reshape tool: move nodes, Bézier handles, corner/curve nodes, add and delete nodes, for all paths of running, fill and satin objects. Start and end points shown and draggable. | Trace a logo from a PNG with Shape and Column, then fix the outlines with Reshape. |
| 2 | **Workspace.** Units (mm, inch), grid settings and snap to grid, rulers, guides with snap, measure tool, zoom tools (marquee, 1:1, to selection, to hoop), options dialog, property bar with the settings of the active tool. | Place a 40 mm circle 10 mm from a guide, exactly, with snap. |
| 3 | **Input methods.** Run variants (triple, backstitch, stemstitch), manual stitches, star, ring, rounded rectangle with radius per corner, freehand (open and closed) with smoothing, complex fill (outline plus holes in one tool), complex turning, Column B, continue digitizing, backtrack and repeat, redwork. | Each method makes its object in one undo step, and the corpus can be made with these tools. |
| 4 | **Stitch angles and holes.** Fill angle by a line on the canvas, several stitch angles (turning fill), add, remove and fill holes, edit satin rungs. | A fill with a hole and a turning satin reshaped without new objects. |
| 5 | **Split, join and select.** Knife, break apart, branching, closest join; lasso, polygon and polyline select; select at needle, by color, by stitch type; next and previous object. | Split a fill with the knife and sew the parts with one trim fewer than before. |
| 6 | **Transform and arrange.** Skew, rotate by angle, mirror about a line, transform by reference points, align, space evenly, make same size, group and ungroup, lock and unlock; sequence back and forward by one color. | Align and space five objects, group them, rotate the group by 30°. |
| 7 | **Viewing and travel.** Stitch cursor; travel by stitch (1, 10, 100, 1000), object, segment, color, function, trim; view by color; hide others; show needle points, connectors and function symbols. | Find the trim of the third color with the keyboard only. |

Then the exit test with users (roadmap P2 exit).

## Progress

| # | State | Notes |
| --- | --- | --- |
| 1 | Done in software | Backdrop (`emb_model::Backdrop`, `emb-app/src/backdrop.rs`, ADR 0008): File > Load backdrop picture, Backdrop panel under Properties. Reshape tool (`emb_geom::nodes`, `emb-app/src/reshape.rs`). The plan reports each object's start and end (`StitchPlan::spans`); satins and fills honor a set end point. Not yet tried by a user on a real tracing task. |
| 2 | Done in software | `emb-app/src/workspace.rs`: units (mm, inch) in the property bar, status bar and Options; grid with spacing and major lines from the settings; snap to grid and guides (draw tools, Reshape, Move, Measure); rulers; guides dragged from the rulers, saved in the design (`Design::guides`); Measure and Zoom tools; zoom in, out, 100 %, to hoop, to selection; Options window; property bar. Settings are saved in the user's configuration folder. |
| 3 | Done in software | `emb-app/src/draw.rs`, `emb_stitch::{running, network}`: Manual, Freehand line and shape, Complex fill, Rounded rectangle, Star, Ring, Column B, Turning (complex turning), Continue line; run styles backstitch and stem stitch; passes backtrack, repeat, redwork and branching (`RunStyle`, `RunPass`). |
| 4 | Done in software | Angle tool (fill angle; new satin rung), rung drag and delete in Reshape, Add hole tool, Remove holes, Fill holes, satin split length (`SatinParams::split_mm`, turning fills). |
| 5 | Done in software | `emb-app/src/actions.rs`: Knife (fills, lines, satins), Break apart, Branching and Redwork commands, Closest join; Lasso, Polygon and Line select; select by color, by stitch type, at the stitch cursor; Tab and Shift+Tab. |
| 6 | Done in software | Align (6), space evenly (2), make same size (3), skew and rotate by angle (Arrange menu), Mirror line tool (Ctrl: copy), Reference points tool, group and ungroup, lock and unlock all, back and forward one color. |
| 7 | Done in software | `emb-app/src/viewing.rs`: stitch cursor with travel by 1/10/100/1000 stitches, object, color, function, segment; status line; stitches after the cursor dimmed; view by color, hide others, show all; needle points and function symbols. The plan reports the records of each object (`ObjectSpan::records`). |

Not done in P2 (moved, then done in P3 Milestone 8): snap to objects and
per-corner radius editing in a dialog; "previous view" and zoom factors;
trims as separate travel steps are part of "function" travel. The P2 exit
test with users is open.

## Added after the phase (owner requests, 2026-10-22)

- **Simplify** (Reshape: smooth curves). `emb_geom::simplify`: the path is
  cut at corners and each part gets the fewest cubic curves within the
  tolerance (the published least-squares method of Schneider, our own
  code). A result with as many nodes or more is not used. Quick simplify
  (0.2 mm, corners over 40 degrees) and Simplify… with the values and a
  live result. Lettering and stitch objects do not change.
- **Dark and light mode** (workspace). `Settings::dark_mode`, default dark;
  View > Workspace > Dark mode and Options > Appearance.
- **Import as lines** (artwork import). `SvgImport::as_lines`: the outline
  and holes of each filled shape become one running-stitch line object.

## Decisions for Milestone 1

- **Backdrop storage.** The image file bytes are in the project file
  (base64), so a project is one file that moves without broken links. The
  model keeps the bytes in an `Arc`, so undo snapshots do not copy them.
  The app decodes the image (crate `image`, pure Rust, PNG, JPEG, BMP) and
  keeps one texture. One backdrop per design; more come with the vector and
  bitmap tools in P6.
- **Backdrop placement.** On import the image is fitted to 80 % of the hoop
  field (or 100 mm if there is no hoop) and centered on the hoop center, and
  it is locked. The Backdrop panel sets position, width (the height keeps the
  aspect ratio), dimming, hide and lock. When it is not locked, a drag on it
  with the Select tool moves it.
- **Reshape is Bézier node editing on the stored paths.** The draw tools
  store cubic Béziers (`Path::through_points`), and SVG import stores the
  artwork curves, so one editor works for both. A node is the end point of a
  segment. A node is smooth if its two handles are on one line, else a corner.
  Moving a node moves its handles with it. Moving a handle of a smooth node
  turns the other handle to stay on the line. Corner to curve gives the node
  handles along the line between its neighbors (as the draw tools do);
  curve to corner sets the handles on the node. A new node splits the
  segment at the clicked point (de Casteljau), so the shape does not change.
  Deleting a node joins its two segments into one cubic with the outer handles.
- **Start and end points.** The stitch plan reports where each object starts
  and ends. Reshape shows these points; dragging one sets the object's start
  or end point. A fill ends on the row end nearest to the end point: if that
  row is inside a section, the section is split there and the part with that
  row is sewn last. A satin starts at the column end farther from the end
  point. A running line runs toward the end point (a closed line starts there).
- **Settings, not design.** Units, grid, snap, nudge steps and the defaults
  of new objects are user settings in `settings.json` in the configuration
  folder; guides are part of the design (undo, saved in the project).
- **Snap.** The grid starts at the hoop center. Guides win over the grid.
  A move snaps its offset (start and current pointer both snapped), so
  objects move in grid steps.
- **Run styles (own design).** Backstitch: needle points at half the
  stitch length, sewn two forward, one back. Stem stitch: the same with the
  forward stitches 0.4 mm to the left. Both are made after the direction is
  chosen, so they look the same either way.
- **Redwork and branching (own design).** Lines are cut at crossings and at
  touching ends; a depth-first walk from the node nearest the needle sews
  each line out and back (redwork) or leaves the longest branch for last,
  sewn once (branching). See `emb_stitch::network`.
- **Turning fill.** An outline plus stitch angle lines becomes a satin
  column whose rails are the two parts of the outline between the first and
  last angle lines; spacing 0.8 mm (rows 0.4 mm apart) and a split of 3.5 mm
  make it look like a turning tatami.
- **Knife** cuts along the infinite line through the two points (fills with
  half-plane booleans, lines and satins at the crossings).
- **Groups and locks** are fields on the objects. A click selects the whole
  group; locked objects cannot be picked and leave the selection.
- **Stitch cursor keys** are our own: `,` and `.` (Shift 10, Ctrl 100,
  Ctrl+Shift 1000), Page Up/Down for objects (Ctrl colors, Shift functions,
  Alt segments). Tab and Shift+Tab select the next and previous object.
- **Reshape input.** Double-click on a node changes corner and curve;
  double-click on the outline adds a node; Delete or Backspace removes the
  selected node; Alt while dragging a handle makes the node a corner. The
  right-click menu has the same commands. Esc clears the node selection, a
  second Esc goes back to Select.
