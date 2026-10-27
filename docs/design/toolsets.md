# Toolset study: a consumer embroidery product and our design

Date: 2026-10-21 · Status: Superseded for scope by `feature-inventory.md` and `roadmap.md` (the reference is now the commercial reference product). The principles, the Sequence panel design, the object manipulation design and the navigation table here stay valid.

## Purpose and method

A consumer embroidery product is the reference for workflow and ease of use (see `docs/SPEC.md`).
This document lists the toolsets of that product, says what each one does for the user, and
records how this project will do the same job, often in a different way.

Clean-room rules for this document:

- Sources are public only: the Quick Reference (Rev 3, April 2024) of the consumer
  product and its public online help site. No software was installed,
  decompiled or inspected.
- The descriptions are short summaries in our own words. Do not copy text, icons,
  shortcut tables or other assets from the documentation of closed-source products
  into this repository.
- We copy **jobs to be done**, not designs. Where the consumer product has a known pain
  point, we do something different.

## Our principles (where we "reinvent the wheel")

1. **One mode.** The consumer product has separate Graphics and Embroidery modes. We keep artwork as
   a locked layer in the same canvas, so the user never switches modes.
2. **Sew order is stack order, and it is always visible.** An object that sews later
   lies on top. The Sequence panel shows color blocks and objects at the same time
   (the consumer product toggles between a color view and an object view).
3. **Live results.** Stitches update while the user draws or drags a handle (UX rule 1).
4. **Show the cost.** Color changes, trims and warnings show next to the thing that
   causes them, with a one-click fix where one exists.
5. **Safe automation.** Automatic tools must not change what is on top of what unless
   the user asks for that. Example: "Merge colors" only moves objects past objects
   that they do not overlap.
6. **Context actions, not tool modes.** Commands act on the selection. Few tools stay
   active after one use.
7. **CAD navigation.** Wheel zooms at the cursor, middle or right drag pans, and a
   double-click on the wheel zooms to extents, as in CAD programs.

## Toolbox map

| Consumer product toolset | What it does for the user | Our version | Phase |
| --- | --- | --- | --- |
| Manage Designs | Browse, search and batch-convert design files in a library window. | Normal file dialogs and a recent-files list. A design library is out of scope until users ask for it. | Later |
| Customize Design | Thread charts, match design colors to threads, optimize color changes, change settings for a fabric, set automatic start and end points. | Threads panel with chart import. **Fabric profiles** (a preset that sets density, underlay and compensation for a fabric type). Color-change optimization is part of the Sequence panel. | 1c, 2 |
| Artwork | Import, scan, adjust, dim and lock backdrop images. | Background image as a locked layer with an opacity slider (UX rule 7). SVG import makes objects directly (Phase 1a). | 1a, 2 |
| Auto-Digitize | Make embroidery from a bitmap automatically, or click a region to fill, outline or follow a centerline. Photo-stitch effects. | Phase 3 as in the spec. First step: **click-to-fill on vector artwork**, which is easy once SVG import exists. | 3 |
| Lettering / Monogramming | Text from embroidery alphabets or TrueType fonts; monogram templates. | Lettering from TTF/OTF fonts with satin columns (P4 Milestone 1 done; monograms in Milestone 5). | P4 |
| Digitize | Draw rectangles, ovals, standard shapes, freehand shapes, open and closed shapes (left click = corner, right click = curve), satin columns as point pairs, holes, backtrack and repeat. | **Draw tools:** Line (open), Shape (closed), Column (satin, one click per side), Rectangle and Ellipse. Left click places a corner, right click a curve point (the path goes through it smoothly), Enter finishes, Backspace removes the last point, Esc cancels. Shape also closes on a click on its first point. Rectangle and Ellipse are drags (Ctrl: equal sides, Alt: from center). The tool stays active after each object. Curve handles to drag come with Edit nodes. Backtrack and repeat are parameters of the running stitch (repeats, reverse), not separate tools. | 1c |
| Edit Objects | Reshape nodes, stitch angles and entry/exit points; knife, weld, break apart, remove overlaps; branching and closest join; sequence by selection order; edit single stitches. | **Edit nodes** shows shape nodes, stitch-angle handles and start/end points at the same time. Knife, Weld and Remove overlaps use `emb-geom` booleans. Branching and closest join become the engine's automatic routing, with connectors shown on the canvas. We do not plan single-stitch editing: it breaks the link between objects and stitches. | 1c, 2 |
| Appliqué | Placement line, cutting line, tack-down and cover stitches as one object; partial appliqué; combine for efficient stitch-out. | Appliqué object type with the four layers and machine stops. Common in home embroidery, so it comes earlier than in the original roadmap. | 2 |
| Create Layouts | Mirror-copy, copy arrays, circle layouts, work areas, buttonholes. | Duplicate, Mirror and Rotate in 1c. Array and circle layouts later. | 1c, 3 |
| Multi-Hooping | Split a large design over several hoopings with registration marks. | As in the spec. | 3 |
| Output Design | Export machine file, print worksheet, save design image. | Export dialog (spec). Printable worksheet with color list and sew time. | 1c, 2 |
| Context toolbar effects | Hand-stitch, feather edge, gradient fill, radial fill, curved fill, embossed patterns; automatic corners; always trim; underlay toggle. | Object Properties. Underlay and trim settings come with the engine. Decorative effects later. | 1b, 3 |
| Travel tools and Stitch player | Step through the design by stitch, object or color; play the stitch-out. | **Timeline scrubber** under the canvas, with color-block marks, play and pause, and stitch-by-stitch keys. | 2 |
| Thread simulation (3D preview) | Realistic thread simulation. | The "Thread preview" view. Polish comes late; see below. | 2, 3 |
| Sequence docker | Show and change the sew order by color block or by object; sequence by color; sequence by selection order. | **Sequence panel**, below. A prototype is in the Phase 0 app. | 0 (prototype), 1c |

## Sequence panel

What the Sequence docker of the consumer product does: lists color blocks or objects (a switch toggles the
view), shows details such as stitch type and stitch count, resequences selected color
blocks by color, and sequences objects in the order that the user Ctrl+clicks them. A
separate command reduces color changes and keeps color layers.

What we do instead:

- **Both levels at the same time.** Each color block is a card: a color bar, the thread
  name, the block number and an object count. The card holds one row per object with a
  thumbnail of its shape, its sew number and its stitch settings. Cards can collapse.
- **Direct manipulation.** Drag object rows or whole blocks. A line shows where the drop
  goes. A drag of a selected row moves the full selection and keeps its order.
- **Selection is shared with the canvas.** Click, Ctrl+click (add/remove) and Shift+click
  (range in sew order) work the same in both places. A row under the pointer highlights
  its object on the canvas.
- **The cost is visible.** The panel shows the number of color changes. A block that has
  the same thread as another block shows a ↻ mark. When a merge is possible, a
  "Merge colors (N)" button appears.
- **Merge colors is layer-safe.** An object moves back to join an earlier block of its
  thread only if it does not overlap any object that it would pass. The test uses
  bounding boxes, so it never puts an object below one it covered. "Group all by color"
  (in the Sequence menu) ignores layers, for users who know that nothing overlaps.
- **Keyboard:** Ctrl+[ and Ctrl+] move the selection back and forward one place;
  Ctrl+Shift+[ and Ctrl+Shift+] move it to the start and the end. These are the usual
  "send backward / bring forward" keys, which match because sew order is stack order.
- **Click order.** Ctrl+click objects, then "Click order" sews them in that order at the
  position of the earliest one.
- **Every change is one undo step,** and a command that changes nothing adds no step.

Planned for Phase 1c: stitch counts and trims per block (needs the engine), thread
change from the block header, and a "sew this block only" filter for the canvas.

## Navigation and shortcuts

Our own set (spec UX rule 8). The consumer product uses Home to travel to the start of the design; we
use it for zoom extents, as CAD programs do.

| Action | Input |
| --- | --- |
| Zoom at cursor | Mouse wheel |
| Pan | Middle drag or right drag |
| Zoom extents (hoop, design and visible stitches) | Double-click the middle button, or Home |
| Command menu for what is under the pointer | Right click (canvas or Sequence panel) |
| Select / add or remove / range | Click / Ctrl+click / Shift+click |
| Box select (objects fully inside) | Left drag on empty canvas; Ctrl or Shift adds |
| Move | Left drag an object; Shift keeps it horizontal or vertical |
| Nudge | Arrow keys 0.5 mm; Shift+arrow 5 mm (repeated nudges are one undo step) |
| Scale | Drag a corner handle (in proportion; Shift for free) or a side handle; Alt scales about the center |
| Rotate | Drag the round handle above the box; Shift snaps to 15° |
| Rotate 90° clockwise / counterclockwise | Ctrl+R / Ctrl+Shift+R |
| Mirror left-right / up-down | H / V |
| Cut / copy / paste (in place) / duplicate (+5 mm) | Ctrl+X / Ctrl+C / Ctrl+V / Ctrl+D |
| New / open / save / save as | Ctrl+N / Ctrl+O / Ctrl+S / Ctrl+Shift+S |
| Import artwork (SVG) | Ctrl+I |
| Export machine file | Ctrl+E |
| Draw tools: finish / remove last point / cancel (twice: Select tool) | Enter / Backspace / Esc |
| Select all / none | Ctrl+A / Esc |
| Delete | Del |
| Undo / redo | Ctrl+Z / Ctrl+Y or Ctrl+Shift+Z |
| Sew order: back one / forward one | Ctrl+[ / Ctrl+] |
| Sew order: to start / to end | Ctrl+Shift+[ / Ctrl+Shift+] |

The clipboard holds objects and their threads as JSON, so a paste works between two
windows of the app. Threads that the target design does not have are added.

## Object manipulation

What the consumer product does: a Select tool, a context toolbar with rotate, mirror, scale and
"center all" buttons for the selection, and a Reshape tool for nodes.

What we do:

- **On the canvas.** The selection has a box with eight scale handles and one rotate
  handle. Drag an object to move it; an unselected object is selected by the same drag.
  Live values (size, offset, angle) show under the box. Each drag is one undo step.
- **Hit test by shape,** not by bounding box: a click inside a satin ring's hole or
  between the lines of a running stitch does not select it.
- **Direction follows the shape.** Rotate and mirror also turn the fill angle (and the
  tatami underlay angle), so the stitches turn with the shape. Spacing and stitch length
  do not scale.
- **In Object Properties,** a "Position and size" section shows X and Y (center from the
  hoop center), W and H (with "Keep proportions"), rotate buttons (±15°, ±90°), mirror
  buttons and "Center in hoop". Field drags are one undo step each.
- **Right-click menu** on objects (canvas and Sequence panel): undo and redo with the
  step name, cut, copy, paste, duplicate, delete, rotate and mirror, center in hoop,
  sew order, thread, zoom to selection, zoom extents and Properties. On empty canvas:
  undo, redo, paste, select all, center design in hoop, rotate design in hoop, zoom.
  The same commands are in the Object menu.

## Preview polish

The 2D stitch view is enough to digitize and check sew order. A realistic thread
preview (thread width and sheen, fabric texture) is polish. It moves to Phase 2 (2D
thread look) and Phase 3 (3D), after the engine output is correct.
