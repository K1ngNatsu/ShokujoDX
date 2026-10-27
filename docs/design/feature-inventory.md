# Reference product: feature and UI inventory

Date: 2026-10-21 (statuses checked against the code 2026-10-24, after P8) · Status: Reference for scope. The roadmap is in `docs/design/roadmap.md`.

## Purpose and method

The project reference is a leading commercial embroidery suite (the
reference product). This document lists
every feature and UI element of the reference product that the public documentation describes, and gives
each one a status in this project and a target phase.

Sources (public, read 2026-10-21):

- The public quick reference and reference manual of the reference product
- Its public release notes
- Its public product comparison page

Clean-room rules (spec, "Key decisions"):

- Descriptions are short summaries in our own words. Do not copy text, icons, tool
  names that are trademarks, shortcut tables or file samples from the material of
  the reference product or any other closed-source product.
- We copy the **job** each feature does. Our design can differ, and should where we
  can do better (see the principles in `docs/design/toolsets.md`).
- Proprietary file formats are implemented only from public, documented or
  open-source references (pyembroidery and similar). We never inspect the binaries
  of closed-source embroidery products.

### Legend

| Mark | Meaning |
| --- | --- |
| ✅ | Done in this project |
| ◐ | Partly done |
| — | Not started |
| ⇄ | Replaced by our own approach (see note) |
| ✕ | Out of scope (see note) |

**Phase** refers to `docs/design/roadmap.md` (P1 to P14).

## 1. Product structure

The reference product is sold in several editions, from lettering only
to full digitizing, and some features are paid add-ons. The full edition
has about 15 outline stitch types, 22 fill stitch types, 19 stitch
effects and 19 manual digitizing tools.

The paid add-ons include kiosk lettering, advanced team names, a hand-stitch effect, photo
digitizing (three kinds), advanced offsets, mirror-merge arrangements, single
sequins, shading and open fills (color blending, accordion spacing, stipple, cross
stitch), curved fills (Florentine, liquid, contour, spiral), custom splits (custom
split effects, user-defined split, carving stamp), motifs (motif fills/runs/singles,
3D warp, create motif), advanced motifs (flexi split, guide lines for effects,
gradient motifs, motif column, string stitch, maze fill), alphabet creator, advanced
sequins, beading, chenille, Schiffli.

**Our approach:** one product with all features. No levels, no paid add-ons (GPL).
Simple mode (P10 Milestone 5) hides the advanced tools for new users: six tabs with large buttons for the patch and lettering tasks, and fewer panels.

## 2. Workspace and modes

| Element | What it does | Status | Phase |
| --- | --- | --- | --- |
| Embroidery workspace | Main mode: digitize and edit embroidery objects | ✅ | — |
| Graphics mode | Separate mode with a full vector editor for artwork | ⇄ ✅ Vector objects in the same canvas (no mode switch, ADR 0024): draw, import, reshape, show, dim, convert, shaping, offsets, arrangements | P6 |
| Home screen | Account, news, tutorials, community links | ✅ Start page (a tab at the start, Help > Start page): new design, new from template, open, Picture to design, the design library, recent designs with pictures (no sample designs: a clean start, ADR 0080), the tutorials, a question box (P10 Milestone 4). Account, news and community: out of scope | P10 |
| Design Explorer | Browse, search and catalogue designs on disks and network | ✓ Library tab (P8 Milestone 2): library folders, thumbnails or a list, search with filters, sort, group, open, new from selected, catalog (PDF), design list (CSV); `emb-cli library` (network folders work as folders; send: P8 Milestone 3) | P8 |
| Design tabs | Several designs open as tabs | ✅ Design tabs: New and Open use a new tab; close and quit ask for each unsaved design | P3 |
| Tab groups | Split the window into two tab groups (side by side or stacked) | ◐ View > Views > Other design: another open design next to the active one, with its own zoom; Edit this design makes it the active one (P10 Milestone 6). Two groups of editable tabs: not planned (the split view and the views cover it) | P10 |
| Split window | Several views of the same design | ◐ Split view: a second view of the design with its own zoom and the thread look , and up to three more views (View > Views > New view; P10 Milestone 6); the dock tiles them | P3, P10 |
| Docker system | Dockable, floatable panels | ✅ (egui_dock) | — |
| Floating and dockable toolbars | Toolbars can float, dock and be shown or hidden | ⇄ Ribbon with tabs (ADR 0010); can be hidden | P3 |
| Status bar | Cursor position, design size, stitch count, current tool state | ✅ | — |
| Prompt bar | Step-by-step instructions for the current tool | ✅ (draw tools) | P2 for all tools |
| Property bar | Numeric position and size of the selection; Enter applies | ✅ Property bar (selection, tool defaults) | P2 |
| Context menus | Right-click commands | ✅ | — |
| Operating units | mm or inches without changing system settings | ✅ mm or inches (View tab and Options); fields, rulers and the status bar follow | P2 |
| Screen calibration | Make 1:1 zoom show real size | ✅ Options > Screen calibration (scale, or the measured length of a 100 mm line) | P3 |
| Options dialog | App settings: view, grid, guides, editing, hoop, etc. | ✅ Appearance (dark or light; dark is the default; high contrast; text size; language: P10 Milestone 7, ADR 0042), units, grid, snap, rulers, nudge, margin, backups, screen calibration; hoop options (P7) and multi-hoop options (Hooping tab) (editing, paste and reshape options (P10 Milestone 1: the place of Paste, after the selected objects, the duplicate offset, the handles of all nodes in Reshape)) | P2–P3, P7, P10 |

## 3. Menus

The reference product has twelve menus. We will group the same commands in our own menus; this list
is for coverage.

| Menu | Commands (summary) | Status | Phase |
| --- | --- | --- | --- |
| File | New (from fabric or template), open, recent, open backup, close, close all, save, save as, save as template / monogram template / team-names template, print, print preview (worksheet / approval sheet), import embroidery, import graphic, scan graphic, export machine file, output files (EMB + machine files + PDF worksheets by profile), export multi-decoration files, send to connection manager, send to device via WLAN, queue design, send to appliqué cutter, send to bling cutter, export as vector (EMF/WMF), capture design bitmap, capture virtual-decoration bitmap, send by email, export design properties (CSV), cross stitch app, exit | ◐ New (new tab), new from template, open (new tab), close, save, save as, save as template, recent designs, recover backups, import graphic and machine files, export machine file (ten formats), Output design, print preview (PDF), design properties CSV, Send to USB (P8), export queue (P8), appliqué cutter file (P8), design picture and virtual decoration picture (P3, P7), close all (P10 Milestone 2), send by email (a zip file of the machine file, the project and a picture, and the mail program) and the outlines as EMF (P10 Milestone 6) (WMF out: not planned, EMF and SVG cover it; multi-decoration files: the rhinestone template with Output design (P9 Milestone 5); bling cutter: the rhinestone template (SVG, DXF, PDF; P9 Milestone 4); cross stitch: the Specialty tab (P9 Milestones 1 and 2: cells, picture to cross stitch, chart PDF); scan graphic and WLAN send: out of scope, ADR 0036) | P1–P9, P10 |
| Edit | Undo, redo, cut, copy, paste, paste after selected, paste special (by property position, shift, center or start at current stitch), duplicate, duplicate with offset, delete, select/deselect all, select by color, select by stitch type, close shape (straight or curve), reverse curves / swap sides, smooth curves, transform (reference points + numbers), envelope | ✅ Undo/redo, cut, copy, paste, duplicate (with an offset), delete, select all, select by color and by stitch type, smooth curves (Simplify), transform by reference points, envelopes (lettering), paste after selected, paste special (the same place, moved by a distance, centered on the hoop, centered on the stitch cursor, first stitch at the stitch cursor), close shape (straight or curve), reverse direction and swap sides (P10 Milestone 1) | P2, P10 |
| View | The 3D thread look (thread simulation), show elements, flip sequins, hoop, hoop template, grid, rulers & guides, zoom (marquee, 1:1, factor, 1.25×, 2×, fit, selected, product, hoop), pan, previous view, center at current stitch, measure, stitch player, view by color, view all colors, view by machine function | ◐ Grid, rulers and guides, hoop and hoop template (P7), zoom (marquee, 1:1, factor list, fit, selected, hoop), previous view, split view, measure, stitch player, view by color, functions, center at the current stitch and view by machine function (travel to the next trim, color change, stop, jump or speed change; P10 Milestone 2) , the thread look flat or in 3D (light and shade, shadows, the fabric texture, the light direction and height; P10 Milestone 3, ADR 0040) (flip sequins: P9 Milestone 6, when a tester has a sequin machine) | P2–P3, P7, P9, P10 |
| Design | Design properties, machine format, machine format settings, auto fabric, auto hoop, background & display colors, auto start & end, remove small stitches, repeats | ✅ Machine/hoop, fabric (with change of existing objects), background color (colorways), auto start (entry from the needle position, ADR 0007), small stitches (machine profile), auto hoop, auto start and end of the design (ADR 0032), repeats (show, join end to start, make; P7 Milestone 6), design properties (Design info and notes), display colors of the grid, guides, hoop and selection (Options, P10 Milestone 2) | P3, P7, P10 |
| Object | Make properties current, apply current properties, favorite styles, apply style, Florentine/liquid guidelines, create motif, create program split, create letter, user-refined letter, update kerning, create sequin shape, create sequin fixing, set color | ◐ Make properties current, apply current properties, apply style, Florentine fill and liquid satin (P5), create motif (P5), user split (P5), user-refined letter (Reshape tool on a letter), create letter (Alphabets window), kerning of single letters (P4), set color, favorite styles (P10 Milestone 2) ( sequin shapes and fixings: P9 Milestone 6, when a tester has a sequin machine) | P3–P5, P9, P10 |
| Arrange | Group/ungroup, lock/unlock, branching, redwork, closest join, break apart, sequence, align, space evenly, make same size, shaping, remove overlaps | ✅ All; shaping and remove overlaps on the Artwork tab (shaping also on the object menu) | P2, P6 |
| Function | Penetrations on/off, borers, insert stop / tie-off / trim / empty stitch / empty jump / any function, clear function, edit function | ◐ Per object: stop, trim (automatic, always, never), tie-in, tie-off, sew slowly (speed: U01, ADR 0037; penetrations, borers, empty stitch and jump: in no format with a public description) | P3, P8 |
| Stitch | Generate stitches, stitch-edit selection, process stitches (scale with density), recognize objects/outlines from stitches, split object at needle | ✅ Split object at needle; stitch objects from machine files, edited with Reshape; recognize outlines (a stitch object becomes a fill of its area, which then scales with its density: process stitches; ADR 0030) | P3, P6 |
| Graphics | Import graphic, instant smart design, smart design, keep graphics, color matching method, crop/remove/finalize crop, prepare bitmap colors, edit in external app, click-to turning satin / tatami with holes / tatami / centerline run / outline run, auto trace to vectors, adjust bitmap, photo digitizing (three kinds), add bitmap colors | ◐ Import graphic (SVG, PDF, AI, EMF, WMF, pictures), crop (box, oval, shape), adjust bitmap, prepare bitmap colors, color matching to a thread chart, click-to-stitch (satin, fill with or without holes, center line, outline), auto trace to vectors, smart design (the Picture to design wizard, with background removal and subject cut-out), photo digitizing (rows, color rows, scribble; ADR 0030), add bitmap colors (Add these threads); keep graphics: vectors stay as objects. Finalize crop: not needed (undo takes a crop back). Edit in external app: the backdrop picture opens in the picture program of the computer and changes here when it is saved (P10 Milestone 6) | P6, P10 |
| Setup | Options, manage thread charts, auto fabrics, styles, my hoops, motifs, user-refined letters, sequin fixings and shapes, alphabets, convert TrueType font, connection manager, bling machine, screen calibration, scanner, machine runtime setup | ◐ Options (with backups and screen calibration), thread charts (import, P3), fabric presets (`data/fabrics.toml`), styles in the design and in templates, My hoops (P7), motifs, sew time for each machine, convert TrueType font (Fonts window), alphabets (Alphabets window), Send to USB targets (P8) (sequin fixings and shapes: P9 Milestone 6, when a tester has a sequin machine; bling (stone-setting) machine, scanner, machine runtime setup: out of scope, no public protocol) | P3–P9 |
| Window | New/move tab group, remove tiling, split window, dockers, toolbars, kiosk, more windows | ◐ Docker layout (dock tabs that move and float) ; the lettering kiosk is a full-window screen (P10 Milestone 5) (another design next to the active one and more views: P10 Milestone 6; toolbars: not planned, the ribbon and the quick access buttons cover them) | P3, P10 |
| Help | Help, tutorials, about | ✅ Help tab: Help (topics and a search that answers questions from them, offline), Start page, Keyboard shortcuts, Questions and problems, About, seven tutorials; 34 topics in groups, with tables and a glossary (ADR 0055); F1 opens the help of the button or the window under the pointer, or of the active tool; the topics of the ribbon tabs and of the keyboard are made from the ribbon (P10 Milestone 4) | P10 |

## 4. Toolbars and toolboxes

| Toolbar | Tools (summary) | Status | Phase |
| --- | --- | --- | --- |
| Standard | New, open, recent, save, export, print, preview, cut/copy/paste, undo/redo, import embroidery, import graphic, output files, send tools | ✓ File menu, quick-access buttons, Edit tab, print preview, Output design, recent designs, Send to USB | P3, P8 |
| View | Options, the 3D thread look, show stitches / outlines / needle points / connectors / functions / repeats / bitmaps / vectors, dim artwork, appliqué fabric, product, bling, flip sequins, grid, rulers & guides, snap (grid, guides, objects), units, auto start & end, stitch player | ◐ Artwork, stitches, jumps (connectors), needle points, functions, backdrop (bitmaps), vectors, dim artwork, grid, rulers, guides, snap (grid, guides, objects), units, appliqué fabric, dark mode, repeats, product, auto start and end (P7), stitch player a switch to show or hide the rhinestones (Specialty > Rhinestones, P10 Milestone 2) , the 3D thread look (P10 Milestone 3) (flip sequins: P9 Milestone 6) | P2–P3, P7, P9, P10 |
| Zoom | Zoom marquee, 1:1, 1.25×, factor, fit, selected, product, hoop | ◐ Marquee, 1:1, in/out, factor list, fit, selected, hoop, product (fit takes the product in, P7), the zoom step of Zoom in and out (1.25×, 1.5× or 2×, Options) and Zoom in and out 2× (P10 Milestone 2) | P2–P3, P7, P10 |
| Hoop | Show hoop, my hoops, hoop options, auto hoop, rotate hoop ±15° or by angle | ✅ Hoop select, show hoop, My hoops (the hoops you have, your own hoops), Hoop options, auto hoop, turn hoop ±15° or by an angle (ADR 0032) | P2, P7 |
| Hooping mode | Toggle hooping mode, select hoops/split lines, reshape, options, auto layout hoops, splitting line, preview and calculate hoopings, add/delete hoop | ✅ Hooping tab (ADR 0032): hooping mode, select and drag hoopings and split lines, move nodes, draw split lines, options, auto layout, preview hoopings (calculated when the design changes), add, turn and delete hoopings | P7 |
| Mode | Home, embroidery workspace, graphics mode, Design Explorer, convert embroidery ⇄ graphics, tag as appliqué / turning satin / photo, keep graphics, match palette, show embroidery, virtual decoration quality | ⇄ ◐ No modes; conversion commands on the selection: vectors to fill, satin (a turning column along the spine of a closed shape, a ring column round one hole, else a fill with the satin stitch type; ADR 0041) or outline, embroidery to vectors, fills and vectors to appliqué (tag as appliqué); tag as photo is Picture to design > Photo stitch. Home and Design Explorer: P8 (design library); virtual decoration: the Product window (P7) | P6–P8 |
| Dockers | Toggles for every docker (section 5) | ◐ Each panel is a dock tab that can move and float; commands show the panel (Properties, Styles, Threads, Stitch list, Design info, Library, Clipart, Exports) , and each panel closes with its × and opens again from View > Panels (P10 Milestone 2) | P3, P8, P10 |
| Color | Colorway switch, colorway editor, background & display colors, product visualizer, current color, pick color, apply color, add/remove color, hide/remove unused, split palette color, cycle used colors, color wheel, threads docker | ◐ Colorway switch, background color, current color, add color, remove unused, split color, cycle used colors, Threads panel, color wheel (the design colors on a wheel, and complementary, analogous, triad and split complementary colors from the chart, P10 Milestone 2), display colors (Options, P10 Milestone 2) | P3, P10 |
| Colorway editor | Create, rename, delete, reorder colorways; edit color; details; hide unused | ✅ Create, rename, delete, reorder, edit color (table in the Threads panel) | P3 |
| Toolbox | Select, reshape, edit, lettering, team names, monogramming, buttonhole, laydown, offsets (simple, outlines & offsets, open), add border, multi blend, auto-digitizing, envelope, arrange, shaping | ✅ Select, Reshape, draw tools, lettering (Lettering tab), team names, monogramming, buttonholes, laydown, offsets (simple, outlines, open), add border, multi blend, envelope, arrange, shaping, border trace with a border and a gap (knockout), auto-digitizing (Picture to design, click to stitch, trace) | P2–P8 |
| Selection | Select (Ctrl/Shift, marquee), freehand select, select current (at needle), polygon select, polyline select | ✅ Also a crossing box (drag to the left: the objects that the box touches; owner request) | P2 |
| Reshape | Reshape, show nodes, show Bézier handles, show stitch angles, show entry/exit, stitch edit, keep/omit last stitch | ✅ Nodes, Bézier handles (of the selected node, or of all nodes: Options), stitch angle lines, entry/exit, single stitches of stitch objects, single stones, keep or omit the last stitch of a satin column (Object Properties, P10 Milestone 1) | P2, P10 |
| Edit | Break apart, knife, automatic knife (split with overlaps) | ✅ Break apart, knife (with the overlap amount: automatic knife), and Join (Ctrl+J, owner request) to put parts back together | P2, P6 |
| Envelope | Bridge, pennant, perspective, diamond envelopes; delete envelope | ✅ 12 envelope shapes with an amount for lettering (Lettering art gallery); "none" removes the envelope | P4 |
| Arrange | Group, ungroup, lock, unlock all, branching, closest join, break apart, align (6), align centers, space evenly (2), make same width/height/both | ✅ | P2 |
| Traditional digitizing | Column A (turning, varying width), Column B (different sides), Column C (fixed width), complex fill, complex turning, star, ring, manual, motif, branching, redwork, outlines | ✅ Column A, B, C, complex fill, complex turning, star, ring, manual, branching, redwork, outlines, motif (as an outline type of lines) | P2, P5 |
| Outline input | Run, triple run, motif run, backstitch, stemstitch, backtrack, repeat | ✅ All (motif run: P5 Milestone 4) | P2, P5 |
| Manual stitch | Manual (single stitches), triple manual | ✅ | P2 |
| Graphics digitizing | Open shape, closed shape, column, rectangle, ellipse, basic shapes, freehand open/closed | ✅ Also rounded rectangle, star, ring, freehand open and closed | P2 |
| Stitch angles | Add stitch angles, remove stitch angles, continue digitizing | ✅ ; a direction line through a satin or a fill with a column stitch type: the stitches go across it at a set angle (P10, ADR 0041) | P2, P10 |
| Rectangle corners | Corner radius together or per corner, relative scaling | ✅ Corner radius dialog: all corners or each corner (also on rounded rectangles); per corner with Reshape | P2–P3 |
| Holes | Add holes, remove holes, fill holes (exact or offset) | ✅ Add, remove, fill (exact or with an offset), hole from an outline | P2–P3 |
| Appliqué | Appliqué with/without holes (guide, cut, tack, cover), combine appliqué, partial appliqué, remove overlaps | ✅ Appliqué with and without holes (placement, stop, tack-down run or zigzag, stop, cover), combine appliqué, partial appliqué (edges with no cover), remove overlaps (Artwork > Shaping) | P3, P6 |
| Auto-digitizing | Instant smart design, smart design, keep graphics, color matching, crop tools, prepare bitmap colors, click-to turning satin / tatami (with or without holes) / centerline run / outline run, auto trace to vectors, adjust bitmap, photo flash, color photostitch, reef photostitch | ✅ Crop tools, adjust bitmap, prepare bitmap colors, color matching, click-to-stitch (satin, fill with or without holes, center line, outline), auto trace to vectors (ADR 0028), smart design guided and instant (Picture to design wizard, ADR 0029; photos upright, previews in the background and a picture bench, ADR 0052; details on top, fill angles, small areas and checks before Finish, ADR 0053; line art, edge-preserving smoothing and colors from the thread chart, ADR 0054); photo stitch: rows, color rows, scribble (ADR 0030) | P6, P12 |
| Outline stitch types | Run, sculpture run, triple run, backstitch, stemstitch, motif run, string, satin, raised satin, zigzag, E stitch, tatami border, program split border, square, coil, contour, vector outline | ✅ Run, triple (bean), backstitch, stem stitch, sculpture run; a line with a width as satin, raised satin, zigzag, E stitch, tatami border, square, coil, contour, string, motif column or program split; motif run; vector outline as Convert (vector objects, ADR 0024) | P1, P5, P6 |
| Fill stitch types | Satin, raised satin, zigzag, E stitch, tatami, coil, square, double square, island coil, string, program split, motif fill, motif column, cross stitch, contour, spiral, maze, offset, stipple (run / backstitch / stemstitch), laydown, vector fill | ✅ Tatami, satin, raised satin, zigzag, E stitch, coil, square, double square, island coil, string, cross stitch, contour, spiral, maze, offset, stipple (run, backstitch, stem stitch), laydown, motif fill, motif column, program split, flexi split, user split; vector fill as Convert (vector objects, ADR 0024) | P1, P5, P6 |
| Stitch effects | Auto underlay, pull compensation, auto jump (keep long stitches), shortening, fractional spacing, mitre/cap/lap corners, jagged edge, trapunto, user-defined split, accordion spacing, flexi split, radial fill, Florentine, liquid, 3D warp, hand-stitch effect, cording | ✅ Auto underlay, pull compensation, shortening, auto jump, fractional spacing, mitre/cap/lap corners, jagged edge, trapunto, user-defined split, accordion spacing (preset and custom), flexi split, radial fill, Florentine, liquid, 3D warp, hand-stitch effect, cording (as couching: a line, a stop to lay the cord, a zigzag over it) | P1, P5 |
| Mirror-merge | Mirror copy horizontal / vertical / both, array (rows × columns with spacing), reflect, wreath, kaleidoscope | ✅ Arrange > Copies: mirror right, below, both (with a gap), Reflect (a mirror copy about a line you click), Arrangements window (array, wreath, kaleidoscope) | P6 |
| Shaping | Weld, intersect, exclude, front-back, back-front, flatten, divide, combine, keep originals, overlap amount | ✅ All, on fills, vectors, appliqué and satin columns (Artwork > Shaping, right-click > Shaping); combine for vectors and appliqué; Flatten and Divide can stay linked (cut again when the objects move; ADR 0026) | P6 |
| Sequence | Back/forward one object, back/forward one color, to start, to end, sequence by selects, sequence by color | ✅ ; shortest order (Arrange > Sew order): nearest first in each color, what is on top stays (P10, ADR 0041) | P2, P10 |
| Transform | Mirror horizontal/vertical, mirror by reference line, rotate ±15° or by angle, skew, transform by reference line (numeric or free) | ✅ | P2 |
| Styles | Make properties current, apply current properties, favorite styles (up to 10), apply style | ◐ Make properties current, apply current properties, styles (Styles panel and ribbon list), up to 10 favorite styles for all designs (the star in the Styles panel, Design > Favorites; P10 Milestone 2) | P3, P10 |
| Stitch editing | Generate stitches, stitch edit, selects on/off while traveling, process stitches, split object at needle | ✅ Split object at needle; single stitches of stitch objects with Reshape; process stitches as Recognize outlines (ADR 0030) | P3, P6 |
| Functions | Penetrations, borers, insert stop / tie-off / trim / empty stitch / empty jump / function, clear, edit, slow, fast | ◐ Per object: stop, trim (automatic, always, never), tie-in, tie-off, sew slowly (speed: U01, ADR 0037; penetrations, borers, empty stitch and jump: in no format with a public description) | P3, P8 |
| Legacy | Embroidery disks, serial/hardware machine setup | ✕ Obsolete hardware | — |
| Travel | Start/end, by object, segment, color, function, trim, 1000/100/10/1 stitches | ✅ (trims via function) | P2 |
| Sequins | Palette editor, manual sequin, left/right (twin), sequin run auto/manual, sequin fill, sequin edit, sequin split edit, vectors to sequins, drop sequin | — Waits for a tester with a sequin machine (P9 Milestone 6) | P9 |
| Beading | Bead palette editor, manual bead, bead run auto/manual, bead edit | — Waits for a tester with a bead machine and a public format description (P9 Milestone 6) | P9 |
| Bling | Bling palette editor, manual bling, bling run, bling fill, bling lettering, overlap removal on/off, highlight overlaps, bling edit, show machine work area | ◐ Specialty > Rhinestones: Stone, Stone run, Stone fill (rows, rings), outline, center line (lettering) and fill to stones, Remove overlaps for each object, overlaps in red, Reshape of runs and fills; sizes SS6 to SS34 and 12 colors or any color (ADR 0039) (editing single stones of a run or a fill: To single stones, then Reshape (P10 Milestone 1); a palette of the user's own stones: My stones in the stone picker, also in asset packages (P10 Milestone 6); the work area of a setting machine: out of scope, no public format; templates: Specialty > Template, P9 Milestone 4) | P9, P10 |
| Chenille | Chain/moss modes, needle height, compound chenille, view by chain/moss | — Waits for a tester with a chenille machine and a public format description (P9 Milestone 7) | P9 |
| Schiffli | Blatt/Stepp, jumps, speed, borer in/out and depth, RPM, Fadenleiter, thread brake, drop sequin, Schiffli arrangement | — Waits for a tester with a Schiffli machine and a public format description (P9 Milestone 7) | P9 |
| Manage designs / Design Explorer | New from selected (template), open selected, output files, send, print selected / to catalog, export design list, library folders, layout panes, sort/group/filter/search | ✅ Library tab: library folders, sort, group, filter, search, open selected, new from selected, send selected to USB, output files of the selected designs (batch output), print to catalog (PDF), export design list (CSV), thumbnails or list | P8 |

## 5. Dockers (panels)

| Docker | What it does | Status | Phase |
| --- | --- | --- | --- |
| Object properties | Settings of the selected objects, or the defaults for new ones | ✅ (subset) | P1–P5 grow with features |
| Color-object list | All objects grouped by color, in sew order; drag to resequence | ✅ (Sequence panel) | — |
| Design information | Size, stitches, colors, job details, comments for worksheets | ✅ Design info panel: size, counts, sew time, colors with stitches and thread length, warnings, the hoops it fits; job details and comments (design notes, on the worksheets) | P3 |
| Overview window | Thumbnail of the design for zoom and pan | ✅ View > Overview | P3 |
| Threads | Search threads in charts, assign to design colors | ✅ Threads panel | P3 |
| Colorway editor | Several color versions of one design | ✅ In the Threads panel | P3 |
| Stitch list | Every stitch and function, filter and select | ✅ Stitch list panel (functions filter, click moves the stitch cursor) | P3 |
| Team names | Name lists for a team-name design | ✅ Team names window: columns (name, number) with a width limit, rows with quantities, paste or CSV import, show a name, save each name (one step), all names in one design color by color (name drop) | P4 |
| Monogramming | Monogram styles, borders, ornaments | ✅ Monogram window: six styles (single, block, classic, stacked, diamond, oval), six borders (satin, run or triple run), six ornaments, colors, six built-in templates and the user's own; change a selected monogram | P4 |
| Keyboard design collection | Map designs to keys and type them like letters | ✅ Keyboard designs window: our `.embkdc` collections, a built-in shadow collection, type text, map the selection to a key, new collections | P4 |
| Lettering | Text, font, size, baseline, spacing | ✅ Text, font, height, letter and line spacing, alignment, baseline (straight, arc, circle, line) and radius, values of the selected letter | P4 |
| Carving stamp | Patterns made from stamp shapes pressed into fills | ✅ In Object Properties of tatami and program-split fills: add, size, turn, move, remove | P5 |
| Embroidery clipart | Save and reuse design pieces | ✓ Clipart tab (P8 Milestone 5): save the selection as an item with a name and keywords, search, double-click or drag onto the design; 8 built-in items | P8 |
| Exports | Background export queue with history | ✓ Exports tab (P8 Milestone 4): Output design, batch output, Send to USB and the library catalog run as jobs one after another; waiting, running and ended jobs with their files, details and errors; Cancel, Show folder, Run again, Remove; the history (100 jobs) is kept between sessions | P8 |
| Multi blend | Layers of a multi-color blend: add, edit, reorder | ✅ Blends window: color, density at the start and the end, up, down, add, remove | P5 |
| Preview hoopings | Thumbnails of each hooping in a multi-hoop design | ✅ Preview hoopings window (stitches, colors, warnings of each hooping) | P7 |
| Answers | Ask an AI assistant about embroidery | — Benched (owner decision, 2026-10-24): not necessary for the architecture. The help search (P10 Milestone 4) answers from the help text | P10 (benched) |

## 6. Features by area (Reference Manual)

The manual has eleven parts. Each row is a feature group; the toolbars above give
the individual tools.

### 6.1 Object-based embroidery (Part I: chapters 1–5)

| Feature | What it does | Status | Phase |
| --- | --- | --- | --- |
| Object-based design | Designs are objects with outlines and settings; stitches are generated | ✅ | — |
| Templates and fabrics on New | New design starts from a template or a fabric preset | ✅ New from template; a template for New (Settings); fabric in the template; Design > Fabric changes it later | P3 |
| Open backup | Automatic backups and recovery | ✅ Automatic backups (every 2 min by default) and recovery after a crash | P3 |
| Grid, rulers, guides | Major/minor grid, 10 mm default, rulers, draggable guides, remove guides | ✅ | P2 |
| Snap | To grid, guides, objects; snap radius | ✅ Grid, guides, the nodes of other objects, radius | P2–P3 |
| Measure | Distance and angle on screen | ✅ | P2 |
| Viewing modes | Stitch view, outlines, the 3D thread look, needle points, connectors, function symbols with colors | ◐ Stitches, artwork, thread look (flat or 3D with light, shadows and the fabric texture; P10 Milestone 3), needle points, jumps, function symbols | P2–P3, P10 |
| Travel through design | Stitch cursor by stitch, object, color, segment, function, trim | ✅ | P2 |
| Stitch player | Animated stitch-out with speed control | ✅ Player bar: play, pause, color steps, speed, position, sew time | P3 |
| View by color, hide others | Show only some colors or objects | ✅ Only these colors, hide others, show all (View tab) | P2 |
| Thread charts | Commercial charts, custom charts, match colors, search | ✅ Brother PEC and 74 brand charts built in (the GPL palettes of Ink/Stitch: Madeira, Isacord, Sulky, Robison-Anton, Gunold, Marathon, Floriani and more; owner decision, P11 Milestone 2, ADR 0045); import GPL and CSV charts; match; search | P3, P10, P11 |
| Design info and worksheets | Production worksheet, approval sheet, print preview, barcodes (Code 39, Code 128) | ✅ Worksheet and approval sheet (PDF), print preview, a barcode of the file name on the worksheet (Code 39 or Code 128, P8 Milestone 7) | P3, P8 |

### 6.2 Setup (Part II: chapters 6–8)

| Feature | What it does | Status | Phase |
| --- | --- | --- | --- |
| Options | General, view, grid, editing, paste, reshape, hoop, multi-hoop settings | ◐ Appearance (dark or light), units, grid, snap, nudge, margin, backups, calibration; hoop and multi-hoop options (P7) (editing, paste and reshape options: P10 Milestone 1) | P2–P3, P7, P10 |
| Machine formats | Per-machine encoding settings (trims, jumps, max stitch, functions) | ◐ Machine profiles: max/min stitch, trim gap, default format, hoops; profiles for Tajima, Happy, Toyota, Ricoma, BAI, Highland and Barudan (U01 with the slow function, P8) , the trim code of DST and TAP (the number of jumps and their length) for each profile (`machines.toml`) and for each user (Design > Machine settings; P10 Milestone 6) | P1, P3, P8, P10 |
| Machine runtime setup | Named machines for sew-time and cost estimates | ✅ Sew time for each machine: speed, color change, trim and stop times | P3 |
| Needle sequence | The needle of each color on a multi-needle machine | ✅ Design > Needles: the number of needles, Auto (a thread keeps its needle, the threads on the machine stay) or a needle for each thread, thread changes when the design has more threads than needles, the needles and stops in TBF and U01 files, the needle chart and sequence on the worksheet (ADR 0063; added 2026-10-26) | P3 |
| Scanner, digitizing tablet | Hardware input | ✕ Scanner (use OS tools); tablet works as a mouse | — |
| Connection manager | Send to machines through vendor software or removable media | ◐ Send to USB: the stick and folder of each machine, one action, read back (ADR 0036) (machine software and network links: out of scope while no maker publishes a protocol) | P8 |

### 6.3 Embroidery digitizing (Part III: chapters 9–12)

| Feature | What it does | Status | Phase |
| --- | --- | --- | --- |
| Stitch types | Run, satin, tatami and the decorative types in section 4 | ✅ Run, satin, tatami and the decorative types of P5 (sections 4 and 6.8) | P1, P5 |
| Graphical digitizing | Draw shapes, then apply an outline or fill stitch type | ✅ | P2 |
| Traditional digitizing | Column A/B/C, complex fill, complex turning, star, ring | ✅ | P2 |
| Manual stitches | Place single stitches with no outline | ✅ | P2 |
| Freehand embroidery | Freehand drawing with smoothing | ✅ | P2 |
| Object properties | Per type: density, angle, length, underlay, compensation, effects | ✅ For each object type, with the stitch types and effects of P5 | P1–P5 |
| Styles and templates | Saved property sets; NORMAL template | ✅ Styles (named property sets) and templates; the built-in Normal template | P3 |
| Colorways | Several color versions; background and fabric swatches | ✅ Colorways with background color | P3 |

### 6.4 Digitizing with artwork (Part IV: chapters 13–16)

| Feature | What it does | Status | Phase |
| --- | --- | --- | --- |
| Bitmap backdrops | Import, crop, adjust, dim, lock for tracing | ✅ Import, dim, lock, hide, move, size; crop to a box, an oval or a shape; brightness, contrast, sharpen (no finalize step: undo takes a crop back) | P2 (backdrop) / P6 (tools) |
| Vector artwork | Import vectors, convert to embroidery objects | ✅ SVG, as objects (fills and lines), as lines only, or as vector objects to convert later PDF, AI, EMF and WMF the same way | P6 |
| Auto-digitize shapes | Click a shape in the artwork to make a fill, satin, centerline or outline | ✅ Artwork > Trace > Click to stitch: a click on a color area of the backdrop makes a fill (with or without holes), satin columns, a center line or an outline (ADR 0028) | P6 |
| Auto-digitize designs | Whole design from artwork (instant or guided) | ✅ Artwork > Import > Picture to design: a wizard (background, detail, colors, stitch types, sew order, a picture of the stitches); instant = the wizard with its defaults (ADR 0029) | P6 |
| Auto-trace | Bitmap to vectors | ✅ Artwork > Trace picture: each color area becomes a vector shape (corners kept, smooth curves, no gaps between areas; the background left out) (ADR 0028) | P6 |
| Photo digitizing | Photo to embroidery: line rows, multi-color, coral-like texture | ✅ Picture to design > Photo stitch: rows (zigzag width and stitch length from the darkness), color rows (a layer of rows for each thread color), scribble (ADR 0030) | P6 |

### 6.5 Modifying designs (Part V: chapters 17–20)

| Feature | What it does | Status | Phase |
| --- | --- | --- | --- |
| Combining objects | Sequence, branching, closest join, redwork | ✅ | P2 |
| Transforming | Move, scale, rotate, skew, mirror, reference-line transforms, arrays | ✅ All, arrays in the Arrangements window | P2, P6 |
| Reshaping | Nodes, Bézier handles, stitch angles, entry/exit points, smooth curves | ✅ Nodes, handles, corner/curve, stitch angle lines, entry/exit, simplify (quick, and with tolerance, corners and curves) | P2 |
| Splitting | Knife, automatic knife, break apart, split at needle | ✅ Knife (with an overlap: automatic knife), break apart, join, split at needle | P2–P6 |
| Productivity | Styles, favorites, templates, clipart, paste special, repeat last | ◐ Copy/paste, styles, templates, clipart, repeat last command (Space) paste special (P10 Milestone 1), favorite styles (P10 Milestone 2) | P3, P8, P10 |

### 6.6 Lettering (Part VI: chapters 21–28)

| Feature | What it does | Status | Phase |
| --- | --- | --- | --- |
| Embroidery fonts | Large library of digitized fonts with recommended size ranges, previews, filter | ◐ Our embroidery font format (`.embfont`) with size ranges; four bundled fonts made from OFL fonts; Fonts window with search, kind filter and preview; size warnings; the user converts more fonts (Fonts window, `emb-cli font`); thirty bundled embroidery fonts made from OFL fonts (six in P10 Milestone 6, twenty in P11 Milestone 2: four for small text from 4 mm, slabs, serifs, scripts and display fonts; ADR 0045), with joins of satin letters that do not build up | P4, P10, P11 |
| TrueType/OpenType conversion | Use system fonts as embroidery | ✅ TrueType and OpenType fonts of the computer and bundled OFL fonts as satin (strokes) or fill letters | P4 |
| Lettering objects | Baselines (line, arc, circle, custom), spacing, kerning, justification, per-letter edit | ✅ Baselines (straight, arc over and under, circle clockwise and counter-clockwise, a line of the design), letter and line spacing, kerning from the font and by hand, justification, move, scale, turn and reshape one letter (Reshape tool and Object Properties) | P4 |
| Lettering effects | Envelopes, lettering art, stitch types and underlay for letters | ✅ Envelopes (12 shapes with an amount) in a Lettering art gallery, stitch type (automatic, satin, fill) and underlay for the object and for each letter, satin and fill settings, sew order (text, center out, closest join) with optimized connections and a trim length | P4 |
| Monogramming | Styles, borders, ornaments, templates | ✅ Monogram window: six styles (single, block, classic, stacked, diamond, oval), six borders (satin, run or triple run), six ornaments, colors, six built-in templates and the user's own; change a selected monogram | P4 |
| Keyboard design collections | Designs mapped to keys | ✅ Keyboard designs window: our `.embkdc` collections, a built-in shadow collection, type text, map the selection to a key, new collections | P4 |
| Team names | Name lists, one design with many names, name drop, export per name or combined | ✅ Team names window: columns (name, number) with a width limit, rows with quantities, paste or CSV import, show a name, save each name (one step), all names in one design color by color (name drop) | P4 |
| Custom lettering | Create letters, user-refined letters, alphabet creator, manage alphabets | ✅ Alphabets window: create a letter from selected objects between guides (alphabet creator), new, rename, delete, import (also BDF and Windows `.fon` bitmap fonts), export, delete a letter; user-refined letters (Milestone 3) | P4 |
| Lettering kiosk | Simple personalization screen for retail | ✅ Lettering > Lettering kiosk: one full-window screen: product (kind, color, place; it sets the fabric), text (size, straight or arc), embroidery font, thread color (matched to the chart), a preview on the product with the 3D look and a size warning, then save, output, send to USB or save a picture; a template can be the start (P10 Milestone 5) | P10 |

### 6.7 Stitch quality (Part VII: chapters 29–32)

| Feature | What it does | Status | Phase |
| --- | --- | --- | --- |
| Underlay | Center run, edge run, zigzag, tatami, double; auto by width | ✅ | P1, P5 |
| Pull compensation | Absolute or percent, per side (twin-sided, 2026) | ✅ Absolute, percent of the width, and for each side | P1, P5 |
| Auto fabric | Fabric presets that change density, underlay, compensation | ✅ Ten fabric presets that change density, stitch length, pull compensation and underlay weight | P3 |
| Optimizing | Auto split for long satins, satin auto-spacing, shortening, fractional spacing, remove small stitches | ✅ | P1 (basic), P5 |
| Connectors | Travel runs, jumps, tie-in/tie-off, trims, auto start & end, closest join, branching | ✅ Travel, jumps, ties, trims, auto start, closest join, branching, auto start and end of the design (ADR 0032; VP3 has no jumps, so there the start point only sets the entry) | P1–P2, P7 |
| Machine functions | Insert and edit stop, trim, tie-off, jumps, speed, color change | ◐ Per object: stop, trim (automatic, always, never), tie-in, tie-off, sew slowly (speed: U01, ADR 0037; penetrations, borers, empty stitch and jump: in no format with a public description) | P3, P8 |
| Stitch editing | Edit individual stitches; process stitches; recognize objects from stitch files | ✅ Stitch objects: move, add, delete needle points; recognize outlines (process stitches; ADR 0030) | P3, P6 |

### 6.8 Decorative embroidery (Part VIII: chapters 33–36)

| Feature | What it does | Status | Phase |
| --- | --- | --- | --- |
| Patterned stitching | Program splits, user-defined splits, flexi split, carving stamps, split satin | ✅ Program split (10 patterns of our own) on fills and columns, flexi split (across a shape and along columns), user split (split lines), carving stamps (8 stamps of our own; size, turn, place) | P5 |
| Curved stitching | Contour, spiral, radial, Florentine, liquid, maze | ✅ Contour (columns, lines and fills), spiral, offset, radial (with a center), Florentine (a guide line), liquid (satin), maze | P5 |
| Motif stitching | Motif runs, motif fills, motif column, 3D warp, gradient motifs, create motifs, libraries | ✅ Motif run, motif column, motif fill (row offset, gradient, 3D warp), 15 built-in motifs of our own, create a motif from the selection, rename and delete (Motifs window) | P5 |
| Specialty stitching | Raised satin, trapunto, jagged edge, hand stitch, cording, stipple, string, cross stitch, laydown, color blends, multi blend, accordion profiles (custom, 2026) | ✅ Raised satin (columns, lines, fills), trapunto, jagged edge, hand stitch, cording, stipple, string, cross stitch, laydown (a fill type, and Laydown under the selection or the whole design), accordion profiles (preset and custom), color blends, multi blend (Blends window) | P5 |

### 6.9 Applied decoration (Part IX: chapters 37–42)

| Feature | What it does | Status | Phase |
| --- | --- | --- | --- |
| Appliqué | Appliqué objects with up to four layers, fabrics, partial appliqué, cutter output | ◐ Appliqué objects (placement, tack-down, cover), fabric color display, partial and combined appliqué, stops on the worksheet, cutter file of the cut lines (SVG, DXF; P8 Milestone 7) , up to four fabric layers in one appliqué object, each piece in its own color (P10 Milestone 6) | P3 (core), P8 (cutter), P10 |
| Mixed decoration | Embroidery with sequins, beads, bling in one design | ◐ Embroidery and rhinestones in one design, with sewn template marks and the order of work on the worksheet (P9 Milestone 5) (sequins and beads: P9 Milestone 6, when a tester has such a machine) | P9 |
| Sequins | Single and twin sequins, runs, fills, palettes, shapes, fixings, flip sequins | — Waits for a tester with a sequin machine (P9 Milestone 6; DST has a public sequin mode) | P9 |
| Beads | Bead runs and fills, palettes | — Waits for a tester with a bead machine (P9 Milestone 6) | P9 |
| Bling (rhinestones) | Palettes, runs, fills, lettering, overlap removal, cutting output, CAMS machine | ◐ Stone sizes and colors, single stones, runs, fills, lettering (center lines or outlines), overlap removal (P9 Milestone 3) template output as SVG, DXF and PDF with a clearance and registration marks, stones on the worksheet (P9 Milestone 4) (stone-setting machines: no public file format) | P9 |

### 6.10 Production (Part X: chapters 43–50)

| Feature | What it does | Status | Phase |
| --- | --- | --- | --- |
| Design hooping | Hoop list, my hoops, auto hoop, hoop templates, rotate hoop, center all, hoop placement | ✅ Presets, my hoops, auto hoop, hoop template (PDF at full size, on more sheets if needed), turn hoop, center in hoop, hoop placement (manual, auto centering) | P2, P7 |
| Multi-position hoops | Hoops with several fixed positions | ✅ Positions in hoop presets and My hoops; a hooping for each position, one file for each position (ADR 0032). No real multi-position hoop in the data yet (sizes from the maker) | P7 |
| Multi-hooping | Split large designs over several hoopings, split lines, registration | ✅ Auto layout, split lines, zones, cut with a join overlap, registration marks, a machine file for each hooping and a hooping sheet (ADR 0032). Satin columns and lettering are not cut (a warning) | P7 |
| Design reports | Worksheets, approval sheets, barcodes | ✅ Worksheets, approval sheets, design properties CSV; the worksheet shows the hoop orientation and a barcode of the file name | P3, P8 |
| Machine files | Open machine files, recolor, recognize objects | ✅ Open and insert as stitch objects, recolor, transform, split; recognize outlines (a stitch object becomes a fill, ADR 0030) | P3, P6 |
| Embroidery output | Export, output-files profiles, batch output (2026), send to USB | ✓ Export of eight formats, Output design with an output profile, batch output (app and `emb-cli output`), Send to USB (ADR 0036) | P1, P3, P8 |
| Multi-decoration output | Separate files for each decoration process | ✓ Output design writes the machine file (embroidery and marks) and the rhinestone template (SVG, PDF) (P9 Milestone 5) | P9 |
| Machine networking | Machine manager, WLAN send, design queue, Brother wireless (2026) | ✗ No public protocol found for the Topaz 20 or the other brands (ADR 0036); Send to USB instead | P8 (open protocols only) |
| Buttonholes (2026) | Place buttonhole strings | ✅ Buttonholes window: one, or a string along a line (across or along it); opening, legs, gap, bar tacks, density | P5 |
| Redwork (2026) | Combine outlines into one object with the same entry and exit | ✅ | P2 |
| Continue digitizing (2026) | Continue an object from its end or start | ✅ Continue line: new points join the end of a line, or its start when the first click is near the start | P2 |

### 6.11 File management (Part XI: chapters 51–56)

| Feature | What it does | Status | Phase |
| --- | --- | --- | --- |
| Design management | Library folders, search, sort, group, thumbnails, catalog print, CSV list | ✓ Library tab: library folders, search, sort, group, thumbnails (with the rhinestones and the appliqué fabric of projects, P10 Milestone 6), catalog print (PDF), CSV list | P8 |
| Asset management | Templates, styles, fabrics, fonts, motifs, splits, stamps, hoops, borders, thread charts, machine formats | ◐ Hoops, machines and fabrics in TOML; templates and styles; motifs (`.embmotif`); split patterns and stamps (built in); borders (12 built in); embroidery fonts and alphabets; clipart (P8); the design library (P8) , My assets: one window to import, export and share templates, fonts, motifs, thread charts, clipart and hoops, and packages with all of them and the favorites (P10 Milestone 6) | P3–P5, P8, P10 |
| Product backdrops | Show the design on a garment image | ✅ Eight products drawn in code at real size (T-shirt front and back, polo, sweatshirt, cap, tote bag, bath towel, cushion) in any color, with their usual places for a design; a picture of the user's own product at its real width (P7 Milestone 5) | P7 |
| Font libraries | Standard and custom fonts | ✅ Bundled embroidery fonts (Block Sans, Anton, Pacifico, Varsity), bundled OFL TrueType fonts, the user's embroidery fonts and the fonts of the computer | P4 |

## 7. File formats

The reference product reads and writes its own native format and more than forty machine formats.

| Group | Formats | Status | Phase |
| --- | --- | --- | --- |
| Native | The native format of the reference product | ⇄ Our `.embproj` (JSON). That format is undocumented: not planned. | — |
| Condensed (outlines) | CND (Melco), PCH (Gunold), ART/ART42 (Bernina), JAN, GNC, INP | ✕ Undocumented. Revisit if open specs appear. | — |
| Machine, home | VP3, VIP, HUS, SHV (Husqvarna/Pfaff), PES, PEC (Brother), JEF, SEW (Janome), EXP (Melco/Bernina), XXX (Singer), CSD | ◐ VP3, PES, PEC, EXP, JEF, XXX read and write; SEW, HUS, SHV, PHB, PHC, JPX, PCD, PCQ, PCS, PCM, MAX, KSM read (VIP and CSD: no public description; pyembroidery has no VIP reader) | P1, P3, P8 |
| Machine, industrial | DST, DSB, DSZ, TBF (Tajima), U?? (Barudan), TAP (Happy), EBD (SWF), KSM, PCD/PCM/PCQ/PCS (Pfaff), DAT, VEP (Hiraoka), Z?? (ZSK), 10O (Toyota), DSN, STX, PMU, PUM, SAS, MST, INB, BRO, EMD, JPX, ARX | ◐ DST, TBF, TAP, U01 (with speeds) read and write; DSB, DSZ, 10O, 100, DAT (Barudan, Sunstar), EXY, FXY, GT, STX, INB, BRO, EMD, MIT, NEW, SPX, STC, ZHS, ZXY read (pyembroidery has no writer for them; EBD, VEP, DSN, PMU, PUM, SAS, MST, ARX: no public description); thread color files INF, EDR, COL (ADR 0037) | P3 (common), P8 (rest) |
| Templates | EMT, AMT, JMT | ⇄ Our own templates (`.embtemplate`, project JSON) | P3 |
| Graphics in | Vector (SVG, PDF, AI, EPS, CDR, EMF, WMF), bitmap (PNG, JPG, BMP, TIFF, GIF) | ◐ SVG, PDF, AI (PDF inside), EMF, WMF (as objects, as lines or as vectors); PNG, JPEG, BMP as backdrop. TIFF and GIF (P10 Milestone 6); EPS (needs a PostScript interpreter) and CDR (no public specification): not planned, save as PDF or SVG first (ADR 0027) | P2 (bitmaps), P6 (vectors), P10 |
| Graphics out | EMF/WMF vectors, design bitmap, virtual decoration bitmap, PDF worksheets | ◐ Design picture (PNG and SVG), PDF worksheets, object outlines as SVG paths, the virtual decoration picture (the design on the product, PNG) , object outlines as EMF (P10 Milestone 6) (WMF: not planned) | P3, P7, P10 |
| Bling | YNG and cutter formats | ◐ Cutter formats: the template as SVG and DXF (P9 Milestone 4) (YNG and other formats of stone-setting machines: no public description) | P9 |
| Multi-decoration | Dahao DHE and others | ✕ No public description of these formats; the machine file and the template files do the job (P9 Milestone 5) | — |

## 8. Out of scope

| Item | Reason |
| --- | --- |
| Licensing, accounts, trials, subscriptions | Free software (GPL). No account needed. |
| Bundled graphics suite of the reference product | Proprietary. We build vector tools into the app (P6). |
| Native formats of closed-source programs, and other undocumented condensed formats | No public specification; reverse engineering is not allowed. |
| Legacy embroidery disks and serial connections | Obsolete hardware. |
| Vendor-proprietary connection software | Only where a protocol is publicly documented. |
| The academy, community and blog of the reference product | Services of the maker of the reference product. Our equivalent is the help topics and tutorials in the program (`docs/help`, P10 Milestone 4). |
| The font files of other embroidery programs | No public description of the formats; reverse engineering is not allowed. |
| WiFi and cloud send to machines (the apps of machine makers) | No maker publishes a protocol (ADR 0036); Send to USB does the job. |

## 9. Features of other programs

Features that the reference product does not have, or that this inventory did
not list, from the public features of Ink/Stitch and of other free and
commercial embroidery programs (owner request, 2026-10-25).
Phase P11 (`docs/design/p11-plan.md`) plans them.

| Feature | In | What it does | Status | Phase |
| --- | --- | --- | --- | --- |
| Density map | Ink/Stitch, others | Colors the design by the needle points for each mm², over all objects, to show build-up | ✅ View > Density map: layers of thread over all objects; places over the limit in Design info (ADR 0044) | P11 (Milestone 1) |
| Thread use and cost | Commercial programs | Top and bobbin thread for each color; cost of the thread of a design | ✅ Design info, worksheet, `emb-cli stitch`; cost from the spool and bobbin prices in Options (ADR 0044) | P11 (1) |
| More brand thread charts | Ink/Stitch (GPL palettes), all | Madeira, Isacord, Sulky, Robison-Anton and more | ✅ 74 brand charts of Ink/Stitch built in (ADR 0045) | P11 (2) |
| More embroidery fonts, fonts for small text | All | Hundreds of fonts; fonts for letters 4 to 6 mm high | ✅ Thirty bundled fonts, four for small text (Fonts > For small text; ADR 0045) | P11 (2) |
| Thumbnails in the file manager | Commercial programs | Design pictures for project and machine files in Explorer, Finder and Linux file managers | ✅ Windows handler, Quick Look extension, freedesktop thumbnailer (ADR 0046) | P11 (3) |
| Tartan fill | Ink/Stitch | Bands of color in two directions, from a sett | ✅ Edit > Stitch types > Tartan: a group of fills, one for each color of the sett (ADR 0047) | P11 (4) |
| Ripple stitch | Ink/Stitch | Lines that repeat a shape from its outline to its center or along a guide | ✅ Outline type Ripple, with a point and a guide (ADR 0047) | P11 (4) |
| Meander fill | Ink/Stitch | A line with no crossings that fills an area, from a tile pattern | ✅ Fill type Meander: squares, triangles, hexagons (ADR 0047) | P11 (4) |
| Circular fill | Ink/Stitch | Rings round a center point | ✅ Fill type Circular (ADR 0047) | P11 (4) |
| Guided fill | Ink/Stitch | Rows that follow guide lines | ✅ Fill type Guided, one or more guide lines (ADR 0047) | P11 (4) |
| Linear gradient fill | Ink/Stitch | Two or more colors that change across a shape | ✅ Edit > Stitch types > Gradient: a group of fills, one for each color (ADR 0047) | P11 (4) |
| Free-standing lace | Commercial programs | Mesh fill, lace underlay, a check that each part touches the mesh | ✅ Lace mesh fill, Lace underlay, water-soluble fabric, loose parts in Design info, ornament and edge templates (ADR 0048) | P11 (5) |
| Quilting helpers | Commercial programs | Echo quilting, stipple round shapes, edge-to-edge blocks | ✅ Specialty > Quilting: echo, background, quilt block in hoopings (ADR 0049) | P11 (6) |
| Puff (3D foam) | Commercial programs | Satin settings for sewing over foam | ✅ Puff on satin columns and lettering: dense satin, no underlay across, closed ends, a stop for the foam (ADR 0050) | P11 (7) |
| Cap sequencing | Commercial programs | Sew order from the center out and from the bottom up for caps | ✅ Arrange > Sew order > Cap order; cap frame hint in Design info (ADR 0050) | P11 (7) |
| Cutwork needles | The programs of machine makers, others | Lines that cutting needles cut, with the needle angle | ◐ Specialty > Cutwork: cut lines by needle angle, a color for each needle (ADR 0051); files not tried on a machine | P11 (8) |

## 10. Shop tools

Tools that a shop uses every day to sell, plan and supply its work, from
a review of the gaps on 2026-10-27 (owner request). Phase P13
(`docs/design/p13-plan.md`) plans them, and phase P14
(`docs/design/p14-plan.md`) the sketch mode.

| Feature | What it does | Status | Phase |
| --- | --- | --- | --- |
| Run time of an order | The time to sew a quantity on machines with more heads, with the time to hoop and to change frames | ✅ Design > Quote and Design info; heads and hooping times in Machine Settings; `emb-cli quote` (ADR 0064) | P13 (1) |
| Quotes | A price from price lists (stitches, setup, digitizing, colors, quantity breaks, rush, tax), a quote sheet for the customer | ✅ Design > Orders > Quote: price lists, the lines of the price, the quote sheet (PDF); the price of each piece in Design info; `emb-cli quote` (ADR 0064) | P13 (1) |
| Order details | Customer, order number, garment, sizes, placement and due date on the worksheet and the approval sheet; a line to sign | ✅ Design > Orders > Order Details (`Design::order`, ADR 0065): on the worksheet (the barcode has the order number), the approval sheet (with the price), the quote sheet and the CSV; the order number in file names | P13 (2) |
| Thread stock | The threads of the shop; color matching with only them; the threads to buy for an order | ✅ Design > Orders > Thread Stock (ADR 0066): the stock with CSV, Only threads in stock (the stock is a chart), Use Threads in Stock, threads to buy in the window, Design info and the worksheet; marks in the Threads panel | P13 (3) |
| Scripts | Tools that a user writes, with no access to files or the network, in the program and in `emb-cli` | ✅ Edit > Scripts and `emb-cli script`: Rhai scripts in the engine crate `emb-script` (ADR 0067), four samples, inputs from `//!` lines, limits, one undo step | P13 (4) |
| Sketch mode | A parametric sketch as in CAD programs: constraints, dimensions, a solver; its regions become embroidery objects that change with it (owner request, 2026-10-27) | ◐ The sketch model and the constraint solver (engine crate `emb-sketch`, ADR 0068; P14 Milestone 1); sketch mode in the app: Digitize > Start Sketch, the Sketch tab with draw tools, constraints, dimensions, colors of the state and drag (ADR 0069; Milestone 2); drawing help (snaps, horizontal, vertical, tangent), trim, extend, split, fillet, chamfer, offset, mirror, patterns and relax (ADR 0070; Milestone 3); the closed regions of a sketch to fills, satins, appliqué and vectors, curves to runs and satins (ADR 0075; Milestone 4); the objects follow the changes and moves of the sketch (ADR 0076; Milestone 5); named parameters and expressions, templates that ask for them (ADR 0078), splines, Vector to Sketch and DXF drawings to sketches (ADR 0079; Milestone 6) | P14 |
