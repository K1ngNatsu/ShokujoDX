# Roadmap: toward parity with the reference product

Date: 2026-10-21 · Status: Accepted. Replaces the phase table in `docs/SPEC.md`.

## Goal

Every feature and UI element in `docs/design/feature-inventory.md` (the
reference product) gets a place in one of the phases, except the items that
section 8 of that document marks as out of scope. Phase 11 adds features of other
programs (Ink/Stitch and other free and commercial embroidery programs) that
the owner selected (section 9 of the inventory). Phase 13 adds the shop
tools that no earlier phase plans (owner request, 2026-10-27; section 10
of the inventory). Phase 14 adds a parametric sketch mode (owner
request, 2026-10-27; section 10 of the inventory). The order follows three rules:

1. **Sewable first.** Each phase ends with designs that sew correctly on a real
   machine. Stitch quality comes before more tools.
2. **Home user first, then the professional.** Phases 1–4 give a home user with a
   single-needle machine (the Topaz 20) a complete tool: digitize, letter, export.
   Phases 5–10 add the professional and specialty features.
3. **Foundations before features that need them.** For example, the stitch engine
   before decorative stitch types, the vector layer before auto-digitizing.

Sizes are relative effort: S (days), M (one to two weeks), L (a month), XL (several months).

## Where we are

| Phase | State |
| --- | --- |
| 0. Foundation | Done: workspace, model with undo/redo, project files, egui app, Sequence panel, canvas editing, draw tools, SVG import |
| 1. Stitch engine and core formats | Done in software: running, tatami fill, satin with underlay, VP3, DST, PES, `emb-cli`, golden tests. The exit test (corpus sew-outs) is covered by the later kits (owner decision, 2026-10-23). CI for three systems is in `.github/workflows` (it runs when the repository is on GitHub). Open: the Ink/Stitch comparison. See `docs/design/export-plan.md`. |
| 2. Digitizing toolset | Done in software: all seven milestones (tracing, workspace, input methods, stitch angles and holes, split/join/select, arrange, viewing and travel). Open: the exit test with users. See `docs/design/p2-plan.md`. |
| 3. Production, color and quality | Done in software: Milestones 1 (thread charts, Threads panel, colorways), 2 (machine functions, split at needle, stitch list) 3 (machine files: open as stitch objects; EXP, JEF, XXX, SEW, HUS, SHV) 4 (stitch player, thread look, design info, warnings, overview), 5 (worksheets, PDF, picture, Output design, batch output), 6 (fabrics, styles, templates, plain-language labels), 7 (appliqué) and 8 (design tabs, split view, backups and recovery, calibration, sew time, P2 carry-overs) done in software; the exit test with users is open. See `docs/design/p3-plan.md`. |
| 4. Lettering | Done: Milestones 1 (TrueType lettering: satin letters from glyph strokes, fill letters, lines, spacing, kerning, alignment, bundled OFL fonts and the fonts of the computer) 2 (embroidery fonts: `.embfont` format, converter, four starter fonts, Fonts window with previews, size ranges and warnings) 3 (arc, circle and line baselines; kerning, move, scale, turn and reshape of single letters) 4 (envelopes and lettering art; stitch type and underlay for each letter; sew order with optimized connections) 5 (monograms: styles, borders, ornaments, templates), 6 (team names and keyboard design collections) and 7 (custom alphabets, bitmap fonts) done: all seven milestones, and the exit test (sew-outs on towel and cap, 2026-10-23). See `docs/design/p4-plan.md`. |
| 5. Decorative stitches and effects | Done in software: Milestones 1 (column types satin, zigzag, E stitch, raised satin, tatami, square, coil and contour; lines with a width as outline types; sculpture run) 2 (fill types contour, offset, spiral, island coil, radial, maze and Florentine; liquid satin) 3 (texture fills: satin, zigzag, E stitch, raised satin and coil across shapes, square, double square, string, cross stitch, stipple, laydown) 4 (motifs: motif run, column and fill, gradient, 3D warp, library, create from the selection) 5 (patterned stitching: program split, flexi split, user split, carving stamps) 6 (stitch effects: auto jump, fractional and auto spacing, accordion spacing, corners, jagged edge, trapunto, hand stitch, cording, double underlay, pull compensation for each side and as a percent) and 7 (color blends and multi blend, buttonholes, decorative borders) done in software: all seven milestones. Open: the exit test (sew-outs of the nine corpus designs of the phase, with photos). See `docs/design/p5-plan.md`. |
| 6. Artwork, vector tools and auto-digitizing | Done in software: Milestones 1 (vector objects) 2 (shaping) 3 (offsets and arrangements) 4 (border trace) 5 (picture tools; PDF, AI, EMF and WMF import) 6 (auto-trace, click to stitch) 7 (Picture to design wizard) and 8 (photo stitch, recognize outlines), with the owner requests (linked shapes, cut out subject). Open: the exit tests (logo, photo sew-out). See `docs/design/p6-plan.md`. |
| 7. Hooping, layouts and presentation | Done in software: Milestones 1 (hoop placement) 2 (multi-hooping engine) 3 (hooping mode, output for each hooping) 4 (multi-position hoops) 5 (product backdrops) and 6 (repeats); the upright Crown hoop (ADR 0034). Open: the exit test (kit in `out/p7-sewout`). See `docs/design/p7-plan.md`. |
| 8. Management and connectivity | Done in software: Milestones 1 (design library engine) 2 (Library tab, catalog) 3 (Send to USB) 4 (export queue) 5 (clipart) 6 (more machine formats, the slow function) and 7 (cutter files, barcodes). Exit test part 1 passed (1000-design search in about 1 ms); part 2 open (the Topaz 20 reads a stick from Send to USB). See `docs/design/p8-plan.md`. |
| 9. Specialty decoration | Done in software for the parts that the owner can test: Milestones 1 (cross-stitch objects, ADR 0038), 2 (picture to cross stitch, charts), 3 (rhinestone objects, ADR 0039), 4 (rhinestone templates) and 5 (mixed decoration: sewn template marks, template files from Output design, order of work); audit in "Audit of P9". Open: the sew-outs (with a cut template). Milestones 6 (sequins, beads) and 7 (chenille, Schiffli) wait for a tester with such a machine. See `docs/design/p9-plan.md`. |
| 10. Polish and assistance | Done in software (the exit tests are open; see "Audit of the whole project"): Milestones 1 (editing helpers: paste after selected, paste special, close shape, reverse direction, swap sides, omit last stitch, single stones, editing options) , 2 (view and workspace helpers: center at stitch, travel by function, zoom steps, panels that close and open, close all, favorite styles, display colors, color wheel, show or hide the rhinestones) , 3 (the 3D thread look on the canvas and in the pictures, ADR 0040) , 4 (start page and help: topics in `docs/help`, search, F1) , 5 (simple mode and the lettering kiosk), 6 (files, assets and content), 7 (translations and accessibility, ADR 0042: German, Spanish, Japanese) and 8 (installers and release builds, ADR 0043) done in software; Milestone 9 (assistant) is benched. The small features that the audits moved here are in the P10 plan. See `docs/design/p10-plan.md`. |
| 11. Features of other programs | Done in software: Milestones 1 (density map and thread use, ADR 0044), 2 (brand thread charts, thirty embroidery fonts, joins of satin letters, ADR 0045), 3 (pictures of designs in the file manager, ADR 0046), 4 (tartan, gradient, meander, circular, guided fills and ripple stitch, ADR 0047), 5 (free-standing lace, ADR 0048), 6 (quilting helpers, ADR 0049), 7 (puff and cap sequencing, ADR 0050) and 8 (cutwork needles, ADR 0051); audit in "Audit of P11 and P12". Open: the exit tests (sew-outs on the Topaz 20, thumbnails after an install, a tester with a cap frame, a tester with a cutwork machine). See `docs/design/p11-plan.md`. |
| 12. Picture to design: robust and better results | Done in software: Milestones 1 (photo orientation, previews in the background, true width for Auto, the picture bench; ADR 0052), 2 (details on top, fill angles, small areas, checks before Finish; ADR 0053) and 3 (line art, edge-preserving smoothing, colors from the thread chart; ADR 0054). Open: the exit tests (a phone photo, a logo and a drawing sewn on the Topaz 20); audit in "Audit of P11 and P12". See `docs/design/p12-plan.md`. |
| 13. Shop tools | Done in software (owner request, 2026-10-27): Milestones 1 (quotes and run time on machines with more heads, ADR 0064), 2 (order details on the sheets, ADR 0065), 3 (thread stock and threads to buy, ADR 0066) and 4 (scripts, ADR 0067). Open: the exit tests with a shop tester (real orders, a printed sheet and its barcode, the spools to buy). See `docs/design/p13-plan.md`. |
| 14. Sketch mode | In progress (owner request, 2026-10-27): Milestones 1 (sketch model and constraint solver, ADR 0068), 2 (sketch mode in the app, ADR 0069), 3 (drawing help and sketch edits, ADR 0070), 4 (regions to embroidery, ADR 0075) and 5 (objects that follow their sketch, ADR 0076) done in software; Milestones 5 (parametric links) and 6 (parameters and more curves) open. See `docs/design/p14-plan.md`. |

## Owner requests of 2026-10-22

| Request | Where | State |
| --- | --- | --- |
| Path simplification: quick and tuned | P2 (Reshape: smooth curves) | Done: Edit > Shape > Quick simplify and Simplify… (tolerance, corners, curves or straight lines, result on the canvas before OK). `emb_geom::simplify`; it never adds nodes. |
| Export to SVG | P3 (output: design picture); P6 (vector export) | Design picture as SVG done: Design > Output > Export SVG, Output design and batch output, `emb-cli output --svg`, `emb-cli convert x.svg`. Object outlines as SVG paths: done (File > Export outlines, Artwork tab; P6 Milestone 1). |
| Import SVGs as paths | P0 (artwork import); P6 (vector layer) | Done: Import as lines (each shape outline and each stroke becomes a running-stitch line). Paths with no stitches: done (Import as vectors; P6 Milestone 1). |
| Light and dark mode, dark by default | P2 (workspace and Options) | Done: View > Workspace > Dark mode, and Options > Appearance. Kept in the settings file. |
| Border trace with a new color, thickness, or a gap (knockout) | P6 (offsets, add border, remove overlaps) | Done (P6 Milestone 4): Artwork > Border trace. A satin border in a thread you choose, or a gap with no stitches, cut out of the objects under the selection; changes of the width update the cut (ADR 0025). |
| Fill then border, or border then fill | P6 (with the border trace) | Done (P6 Milestone 4): a sew-order choice in the Border trace window. |
| Double-click lettering to edit its text | P4 | Done (Milestone 5 work). |
| Space repeats the last command (as in CAD programs) | P2 (workspace) | Done: Space runs the last ribbon command again (not undo, redo or view switches). |
| Lettering on line: an easier way in | P4 | Done: select the lettering (or the line), use the command, then click the line (or the lettering). |
| Hoop orientation on the production worksheet (2026-10-23) | P3 (worksheets) | Done: the worksheet says how to put the hoop on (landscape, portrait or square, with its size), where the top of the design points and where its center is, and shows a small diagram of the hoop with the design in it. |
| Ctrl+J joins sections that were separated (2026-10-23) | P2 (split and join) | Done: Edit > Split and join > Join, and the object menu. Lines and columns whose ends touch, fills that touch (their areas are joined) and stitch objects of the same kind and color become one object. |
| Esc closes the lettering text box (2026-10-23) | P4 | Done: Esc closes the Lettering window, also when the text box has the focus. Nothing is added. |
| Right-click > Edit for lettering and monograms (2026-10-23) | P4 | Done: the object menu (canvas and Sequence panel) has Edit text… for one lettering object and Edit monogram… for a monogram (any of its objects). |
| Crossing selection box, as in CAD programs (2026-10-23) | P2 (selection) | Done: a box dragged to the right selects the objects all inside it (blue); a box dragged to the left selects the objects that it touches (green, dashed). |
| Stitch player and stitch cursor made the canvas jump while an object moved (2026-10-23) | P3 (stitch player) | Fixed: a stitch cursor at the end of the sew-out stays at the end when the plan changes length, so its status line does not come and go. |
| Border trace with a border and a gap at the same time, and a choice of the border stitch (2026-10-23) | P6 | Done: the Border trace window has a border width and a gap width (0: none) and the stitch of the border (any column type, run, triple run). |
| Keep flattened and gapped objects linked when they move (2026-10-23) | P6 | Done (ADR 0026): Flatten, Divide, Remove overlaps (Keep linked, on by default) and border traces are cut again when their objects move; Artwork > Unlink ends it. |
| Flatten and To fill split joined objects, and a rejoin lost the link (2026-10-23) | P6 | Fixed: cuts and To fill keep each object one object (Divide still makes pieces); conversions keep the link; Join, Break apart, the knife and Split at needle say "unlink first" for linked objects. |
| A linked drag after To fill and Flatten was very slow (2026-10-23) | P6 | Fixed: the fill engine is about 15 times faster for fills with many holes (the same stitches), and a drag rebuilds a slow stitch plan only when it stops. |
| Picture to vector: a step-by-step wizard with background removal (2026-10-23) | P6 Milestone 7 | Done (ADR 0029): Artwork > Trace > Picture to design: background (click to remove a color, or the edge color), detail as pixels, colors (leave out, merge), stitches and sew order, a picture of the stitches, Finish. |
| A hidden backdrop still took clicks (2026-10-23) | P6 | Fixed: the picture tools, Trace picture and Click to stitch need a shown backdrop. |
| Picture to design opens from a picture at once (2026-10-23) | P6 Milestone 8 | Done: with no backdrop it asks for the picture, then opens the wizard; first command of Artwork > Import and Digitize > Import. |
| The wizard struggled with noise round a person (2026-10-23) | P6 Milestone 8 | Done (ADR 0030): edge-aware background removal, erase and restore brushes, shrink the edge, remove small bits, noise smoothing. |
| A raw erase for large parts, more polish, a better way to find the subject (2026-10-23) | P6 | Done (ADR 0031): Cut out subject (a box, then keep and remove marks; GrabCut, no new dependency), Box and Lasso erase, zoom and pan in the preview, removed parts shown faintly, Ctrl+Z in the Background step. A trained model is an option for later. |
| Alt+V hides objects on screen to see under them (2026-10-23) | P2 (view) | Done: Alt+V hides the selected objects (View > Hide selected); they still sew and save; Alt+V with nothing selected shows all. The Sequence panel marks hidden objects. |
| More font files: OpenType (`.otf`) | P4 | Works: OpenType fonts with CFF outlines load and convert (checked with Minion Pro); the computer's `.otf` fonts are in the font lists. |
| Bitmap fonts: BDF (`.bdf`) and Windows raster fonts (`.fon`) | P4 Milestone 7 (import alphabets) | Done: Alphabets > Import, and `emb-cli font`; satin or fill (pixel look). |
| Break apart the objects of Picture to design (test build, 2026-10-23) | P6 | Fixed: Break apart also takes the selected objects out of their group, so each one can be selected alone. |
| The ribbon needs a scroll bar on a narrow screen (2026-10-24) | P8 (UI) | Done: a Lettering tab (lettering and personalization) takes them off the Digitize tab, the Import panel is on the Artwork tab only, and the panels that do not fit fold into one button each (a click shows the panel), from the right. A test checks that each tab fits a window 1366 points wide (ADR 0010). |
| The hoop on the canvas was across, but the Topaz 20 shows designs upright (test build, 2026-10-23) | P7 (hoop placement) | Fixed (ADR 0034): an owner test with two VP3 files showed that the Crown hoop is 200 mm left to right and 260 mm front to back, and that the machine turns a design that is too wide. The hoop preset is upright now; the canvas shows "Back" at the back edge of the hoop; a turned hoop or hooping turns the way the machine does. |

## Owner report of 2026-10-24: satin from SVG shapes

Designs made from SVG files, sewn on the reference machine: satin made
heavy overlaps and thread build-up, and there were too many jumps, in an
order that went to and fro. A beta tester asked for a stitch direction
line. Done in software (ADR 0041):

- Fills with a column stitch type (satin, zigzag and the others) sew in
  sections, so no stitch goes outside the shape or over another section;
  the spacing is measured across the stitches.
- Convert to satin: a turning column along the spine of a closed shape, a
  ring column for a shape with one hole, else a fill with the satin
  stitch type (`emb_geom::spine`, `emb_model::vector`); nearest first.
- Edit > Direction line (and the object menu): on a satin, new stitch angle lines across the
  line; on a fill with a column stitch type, a direction line that the
  stitches follow (`FillParams::direction`).
- Arrange > Sew order > Shortest order (`emb_stitch::order`).

Golden snapshots `patterns`, `texture-fills` and `vectors` changed. Open: a
sew-out of the probe shapes on the reference machine.

## Owner requests of 2026-10-25: help and ribbon layout

ADR 0055. Done in software:

- **Help**: 34 topics (was 11), about 23,000 words, in seven groups, with
  tables, a glossary, file formats, options and four new tutorials
  (appliqué, towel, two hoopings, lace). F1 over a window opens its topic.
- **Ribbon**: each command on the tab of its work (Quilting, Puff and
  Laydown on Specialty; the copies and Closest join on Arrange; the
  library on Design; Stitch cursor and Filter apart on View); no two
  commands of a tab share an icon.
- **Bugs**: about 170 labels of dialogs, the File menu, the right-click
  menus and the Design info warnings were not translated (string lists,
  name functions, `format!`); the
  Lettering, Monogram and template windows changed their id with their
  title.

Open: the owner checks the new places in daily use; the help topics are
English only (as before); file dialog titles and undo names stay
English.

## Audit of P11 and P12 (2026-10-25)

After P11 Milestones 1 to 8 and P12 Milestones 1 to 3, a check of the code
and the documents of the two phases: a probe with wrong values in the
project files, a probe with odd pictures in the Picture to design steps,
the rules of the spec, the translations, the licenses and the help.

**Fixed.**

- **A wrong bean count stopped the program.** A probe set each number of
  the 57 corpus projects in turn (the first two items of each list) to 0,
  −5, 10⁻⁹ and 10⁹ and made the stitches in a thread with a time limit.
  A running line with a bean count (`repeats`) of 10⁹ asked for 4.5 TB of
  memory, and the program stopped. The probe of the whole-project audit
  did not find this. Now a bean stitch sews each stitch at most 9 times
  (`running::MAX_REPEATS`). After the change the probe (14 207 cases)
  found no crash, no hang and no plan of more than 2 million records.
- **A very tall picture made a design of hundreds of metres.** A picture
  of 1 × 5000 pixels at 100 mm wide was 500 m tall: 45 million stitches
  and 9.5 s. Now the long side of a design from a picture is at most 1 m
  (`smart::MAX_SIDE_MM`, in the wizard and the bench): 146 ms.
- **Clear specks made holes and loose objects.** The speck removal of
  Simplify did not count clear as a color. So a clear speck inside an
  area was a hole of the fill, and a speck of color in the clear was an
  object of its own (a picture with noise in its transparency: 19 128
  objects, 8.5 s). Now clear counts as a color: a small clear area inside
  the picture takes the color round it, a small area of color in the clear
  goes, and a clear area at the edge of the picture stays (1363 objects,
  0.5 s; on the owner's photos 1 to 10 fewer objects; one stitch more in
  the bench snapshot).
- The status texts of P11 in the spec and the roadmap said "in progress"
  and "planned". Help: Questions and problems has "Picture to design makes
  too many objects".

**Left as it is.**

- Random noise in 16 colors at the finest detail (0.1 mm a pixel), with
  the edge color removed, takes 33 s in the stitch plan: the fills have
  very long outlines. The background job keeps the window responsive, but
  a job cannot be stopped (P12 plan, "Not in this phase"). No real picture
  of the bench or of the owner comes near this.
- Remove the edge color (0.5 s on a large picture) and cut out the subject
  (0.2 s) still run on the window thread.
- The wizard keeps the thread chart colors from when "Colors from the
  thread chart" was ticked; after a change of the chart in the Threads
  panel, tick it again.
- A photo gives a trim for nearly each object (the owner's four photos: 61
  to 271 objects), because its areas of one color are apart. Travel
  between them under later colors is not done.
- The bench counts the band of overlap and pull compensation round each
  area as "outside" (1.5 to 15 %); it is on purpose.
- The limits of line art (2 mm wide, a run under 1 mm, a triple run from
  0.6 mm) and the start values (specks 2 mm², details on top up to 25 mm²,
  small areas 4 mm²) need the sew-outs of the exit tests.

**Checked, no change.**

- Rules 3 and 4: each Ink/Stitch palette file names its source file and
  the commit (9a5b983); the stitch types of P11 Milestone 4 are our own
  methods from the public Ink/Stitch documentation (ADR 0047). Rule 9:
  `unsafe` only in `emb-thumbnail` (ADR 0046); the other ten crates
  forbid it. Since P11, `Cargo.lock` changed only for `emb-thumbnail` and
  its `windows` crates; the license test of the packages passes.
- Translations: the 2110 texts of the template are in German, Spanish
  and Japanese, none missing and none extra.
- Each feature of P11 and P12 has help text; ADRs 0044 to 0054.
- The inputs of the P11 windows: the sett has at most 200 stripes and
  positive counts, echo quilting at most 100 rows, a quilt block at most
  10 000 tiles; the probe covered their values in project files.

**Exit tests that are open.** They need the owner, test users or a tester
with another machine; the software parts passed.

| Phase | Exit test | State |
| --- | --- | --- |
| P11 | Thread use within 15 % of weighed spools; 4 mm and 6 mm small-text lettering; thumbnails after an install on three systems; a sew-out of each new stitch type; a lace ornament and edge washed; a 300 × 300 mm quilt block in four hoopings; puff letters | Open (Topaz 20, the three systems) |
| P11 | Cap order on a cap frame; cutwork files on a cutwork machine | Open: wait for a tester with such a machine |
| P12 | A phone photo upright and the window responsive at the finest detail; a logo with no gaps and a drawing with outlines sewn on the Topaz 20 | Open |

## Audit of the whole project (2026-10-25)

After P10 Milestone 8, a check of all crates and documents: the rules of
the spec, safety with damaged and wrong files, licenses, translations, the
documents and the open exit tests.

**Fixed.**

- **Wrong values in a project file stopped the program.** A probe set
  each number of the 42 corpus projects in turn to 0, −5, 10⁻⁹ and 10⁹
  and made the stitches. With 10⁹, these made the stitch engine run with
  no end and use all the memory, so the program stopped: a coordinate, the
  hoop center, the pull compensation (in mm and in percent, also of the
  letters and of the cover of appliqué), the cover width, cover outside
  and tack inset of appliqué, the width of a line border, the size of a
  motif, the gradient of a motif fill, the hand effect and a point of a
  guide line in the fill settings; with 10⁻⁹, the width of a program-split
  tile. Now:
  - a project file with a coordinate or a hoop center more than 1 km from
    the origin does not open ("out of range", `project::MAX_COORD_MM`);
  - `emb_stitch::stitch_object` refuses an object that is larger than 3 m
    with its distance settings (each setting that ends in `_mm`, so new
    settings are in the check too), that has a percent above 1000, or a
    point in its settings more than 1 km from the origin
    (`StitchError::TooLarge`, a warning for the object);
  - the motif gradient stays in 0.05 to 20 (the panel gives 0.25 to 4),
    and a program-split tile is 0.5 mm wide or more.
  After the changes, the probe (about 297 000 cases) found no crash, no
  hang and no plan of more than 2 million stitches. The probe was a test
  for one session (each case in a thread with a time limit); the new unit
  tests keep the checks.
- **Windows: open with another program, and send by email.** The program
  started the other program through `cmd`. A file name with `&` and no
  space (for example `a&b.png`) could start a second command, and the
  `&body=` of the mail link was lost, so the email had no text. The target
  is now in quotes (`share::start_command_line`).
- **Zip files.** A file in a zip that is larger than 256 MB is refused: a
  damaged or wrong asset package could ask for 4 GB of memory.
- **Same stitches on all systems (ADR 0003).** The cut-out of the Picture
  to design wizard used `exp` and `ln` of the system, and the borders of
  monograms used `hypot`. They now use `emb_geom::math` (with the new
  `exp` and `ln`). A new test (`emb-geom/tests/determinism.rs`) reads the
  engine crates and fails on such a call. No golden snapshot changed.
- **Licenses of the crates.** The packages had only the GPL and the font
  licenses. The MIT, BSD and Apache licenses of the 506 crates of the
  program, and the licenses of the egui fonts (Ubuntu Font Licence, OFL),
  must go with each copy. `data/licenses/THIRD-PARTY.txt` (made by
  `tools/third-party-licenses.py`) is now in each package; a test fails
  if a crate of `Cargo.lock` is not in it.
- **Translations.** The titles of the notice windows were translated in
  the language files but showed in English.
- **CI** checked 5 of the 8 engine crates for UI dependencies; it now
  checks all 8, also for crates of higher layers and for C libraries.
- **Documents.** Stale headers (`emb-app`, `emb-formats`), the P1 row
  (CI exists; the exit test is covered), the export plan, ADR 0033 (the
  names of the Windows packages), and three inventory rows that are now
  done (Edit, Design, Options).

**Checked, no change needed.**

- Each crate has `#![forbid(unsafe_code)]`. The engine crates have no UI
  crate, no C library (`-sys` crate) and no crate of a higher layer.
- Each module ported from pyembroidery names its source and commit
  c16d1b4; the satin engine says that it is not a port.
- The readers of the 40 machine formats, the color files and the artwork
  formats (SVG, PDF, AI, EMF, WMF) gave an error, not a crash, for about
  3 000 damaged files (cut short, changed bytes, large numbers, other
  formats, random bytes); also 400 changed copies of each file once.
  New test: `emb-cli/tests/damaged_files.rs`.
- `unwrap`, `expect` and `panic!` outside tests in the engine crates are
  only on built-in data or on a checked condition.
- All 506 crates have licenses that the GPL-3.0 permits (MIT, Apache-2.0,
  BSD, Zlib, ISC, Unicode, BSL-1.0, Unlicense; one crate is Apache-2.0 or
  GPL-2.0-only, and is used under Apache-2.0).
- The asset packages keep only the file name of each file (no path out of
  the folder).
- No TODO or FIXME notes; five functions have `too_many_arguments`.
- ADRs 0001 to 0043 have a date and a status; the rules in `CLAUDE.md`
  are the same as in the spec.

**Left as it is.**

- The texts of the engine crates (warnings of the stitch plan, file
  errors) and the lines of notices that have values in them stay in
  English (ADR 0042).
- `ribbon.rs` has 4 400 lines: most of it is the table of the commands.
- The installer, the macOS and the Linux packages have not run: the
  repository has no GitHub remote yet (ADR 0043).

**Open: needs the owner or test users.** The table of exit tests in the
"Audit of 2026-10-23" (P2, P3, P5, P6, P7, P8 part 2), the sew-outs of
P9, and for P10: the 15-minute patch test with users of Windows, macOS
and Linux, a new user with F1 and the help search (Milestone 4), a name
on a towel in the kiosk (Milestone 5), a screen reader (Milestone 7), a
review of the German, Spanish and Japanese texts by a native speaker, and
the first release (push the repository to GitHub, then a tag `v0.1.0`).
Also open: the comparison with Ink/Stitch (P1).

## Audit of 2026-10-23

After P5, a check of the code and the documents against this roadmap and
the inventory.

**Fixed.**

- Sliders with a fixed number of decimals rounded the stored value each
  time they showed, and number fields with a range moved values outside
  the range into it. So the selection of an object could change it and
  make an undo step with no user action (found on team names, where the
  fit to a width gives a height such as 7.5695 mm). All sliders and number
  fields of the app now round and clamp only what the user changes.
- `Design::same_content` (the check that a saved and loaded design is the
  same) did not compare monograms, team names and blends.
- A new smoke test (`emb-app/src/smoke.rs`) draws every panel and window
  for each corpus design, with each object selected in turn, and fails if
  drawing changes the design. It takes the place of the "not yet tried by
  hand" notes in the P4 and P5 plans for this kind of error.

**Added (small features of P2 and P3 that were not done).** File > Recent
designs; colorway order (up and down in the Threads panel); Continue line
from the start of a line (the first click near the start).

**Moved to a later phase.** Each item has a reason; the inventory shows
the new phase.

| Item | From | To | Reason |
| --- | --- | --- | --- |
| Tab groups (two groups of design tabs) | P3 | P10 | The split view covers the main use. |
| Barcodes on worksheets | P3 | P8 | Done (P8 Milestone 7): Code 39 or Code 128 of the file name. |
| Penetrations, borers, speed, empty stitch and jump functions | P3 | P8 | Speed done (P8 Milestone 6, U01). Penetrations, borers and empty stitches are in no format with a public description: not planned until one is. |
| VIP read, PEC write, CSD | P3 | P8 | PEC write done (P8 Milestone 6). VIP and CSD: no public description; not planned until one is. |
| Color wheel and display colors | P3 | P10 | The color picker of the Threads panel does the job today. |
| Favorite styles bar | P3 | P10 | Styles panel and ribbon list do the job today. |
| Paste after selected, paste special, close shape, reverse curves | P2 | P10 | Small editing helpers; no user asked yet. |
| Keep or omit last stitch (Reshape) | P2 | P10 | Small editing helper. |
| Auto start and end of the design, hoop rotation by an angle | P2–P3 | P7 | Part of hoop placement. Done (P7 Milestone 1). |
| Remove overlaps for appliqué, automatic knife | P3, P2 | P6 | Use the offsets and cuts of P6. |

**Exit tests that are open.** They need the owner or test users; the
software parts passed.

| Phase | Exit test | State |
| --- | --- | --- |
| P1 | Corpus sew-outs on the Topaz 20 from VP3 (`emb-cli corpus out/corpus`) | Covered (owner decision, 2026-10-23): the engine changed much since P1; VP3 sewed correctly in the P4 sew-outs, and the P5, P6 and P7 kits sew running, satin and fill stitches of the current engine. |
| P2 | A new user digitizes a two-color patch from a bitmap in less than 15 minutes | Open. Test package for test users: `tools/package-windows.ps1` (tasks in `docs/testing/TESTERS.md`, ADR 0033). |
| P3 | Five test users complete three set tasks with no help | Open. The same test package (tasks 1 to 4). |
| P5 | Sew-outs of the nine P5 corpus designs (kit and checklist in `out/p5-sewout`) | Open |
| P6 | A two-color logo PNG becomes a sewable design with less than 10 minutes of touch-up; a photo becomes a recognizable stitch portrait; sew-outs of the P6 corpus designs (kit and checklist in `out/p6-sewout`) | Open |
| P7 | A 360 × 260 mm design split for the 260 × 200 hoop sews with aligned joins (kit and checklist in `out/p7-sewout`, made again for the upright hoop) | Open |
| P8 | A 1000-design library is searchable in under a second; a design goes to a USB stick for the Topaz 20 in one action | Part 1 passed (about 1 ms, `crates/emb-library/tests/thousand.rs`). Part 2 open: File > Send to USB to a stick, then the Topaz 20 reads it. Also open (Milestone 7): a phone app reads the barcode of a printed worksheet, and a cutter program opens a cutter file at real size. |

## Audit of P9 (2026-10-24)

After P9 Milestones 1 to 5 (the parts that the owner can test), a check of
the code and the documents of the phase, and a review of the inventory.
Milestones 6 (sequins, beads) and 7 (chenille, Schiffli) wait for a
tester with such a machine (the rule of the roadmap).

**Fixed.**

- Output design of a design with rhinestones only stopped with "The
  design has no stitches to export", so it gave no template. It now makes
  the template, the sheets and the picture, and no machine file (Export
  of a machine file still needs stitches; `emb-cli output` too).
- The design picture (PNG and SVG, Output design and Export picture)
  showed no rhinestones, and a design of rhinestones only gave an empty
  picture. The picture now has each stone as a disk in its color, and its
  box has the stones and the appliqué fabric.

**Left as it is.**

- A cross-stitch object that does not fit one hooping of a multi-hooping
  layout is not cut: it goes whole to one hooping, with a warning (as
  satin columns and lettering, P7). The rhinestones and the template marks
  of a design in several hoopings go with the hooping that has most of
  them; one template is made for all the stones.
- The design library thumbnails and the catalog show the stitches only,
  not the rhinestones.
- The cell size of cross stitch and the gap and clearance of rhinestones
  are in mm also when the units are inches (the trade uses mm and Aida
  counts).
- Travel stitches between cross-stitch rows go along the cell sides; the
  sew-out of Milestone 1 shows if they must go under the legs.

**Feature review: items moved to a phase.**

| Item | Phase | Note |
| --- | --- | --- |
| A switch to hide the rhinestones on the canvas; edit single stones of a run or a fill; a palette of the user's own stones | P10 | The stones always show; a run or a fill can change to single stones by Break apart of its lines, or be redrawn. |
| Rhinestones in the library thumbnails and the catalog | P10 | The worksheet and the design picture show them. |
| Sequins, beads (and the sequin mode of DST), sequin shapes and fixings, flip sequins | P9 Milestone 6 | When a tester has such a machine. |
| Chenille, Schiffli | P9 Milestone 7 | When a tester has such a machine and a public format description is known. |
| Stone-setting machines (YNG and others), Dahao DHE and other multi-decoration machine formats | Out of scope | No public description. |

## Audit of P8 (2026-10-24)

After P8, a check of the code and the documents of the phase, and a review
of the inventory: each feature that is partly done names the phase of the
rest.

**Fixed.**

- The library listed files with an extension that other programs also
  use (`.dat`, `.new`, `.max`, …) as designs. A machine file with no
  stitches is now an entry with an error ("not an embroidery design").
- Group by smallest hoop did not change when the machine of the open
  design changed; it does now.
- The library index has version 2: the next scan reads all files again,
  so the new formats and the color files next to DST files show.
- Library > Output selected (the files of the output profile for the
  selected designs, through the export queue) was in the inventory but
  not in the app. Added.
- Checks against pyembroidery and python-barcode (not in the repository;
  scripts in the session notes): U01 and PEC bytes of the 38 corpus
  designs, the 24 new readers, U01 speed changes, all Code 128 symbols and
  the Code 39 and Code 128 patterns of sample texts.

**Left as it is.**

- Output hoopings writes at once (its files come from the hooping
  previews); the other outputs go through the export queue.
- The Clipart tab reads its folder when it first shows (in the frame);
  clipart folders are small.
- Send to USB has no Run again in the Exports tab: Send to USB again does
  the same.
- The readers of the formats with no writer are checked with sample files
  made from the reader code, not with files from the machines.

**Feature review: items moved to a phase.** The inventory rows show the
same phases.

| Item | Phase | Note |
| --- | --- | --- |
| More than two views of a design, tiled windows, tab groups, toolbars, kiosk window | P10 | The split view covers the main use. |
| Editing, paste and reshape options in Options | P10 | No user asked yet. |
| File: close all, send by email, export as EMF/WMF | P10 | SVG does the job of EMF/WMF for vector programs. |
| View: center at current stitch, view by machine function, 1.25× and 2× zoom steps, display colors | P10 | Small view helpers. |
| Favorite styles, toggles to close and open each docker | P10 | Styles panel and dock tabs do the job today. |
| More encoding settings for each machine profile (trim code, jump length before a trim) | P10 | With the users of the industrial profiles. |
| More built-in thread charts (where the maker lets us ship the colors), a larger built-in font library (from OFL fonts), one window to manage all assets | P10 | Content work; users import charts and convert fonts now. |
| More than one fabric layer in one appliqué object | P10 | Combine appliqué gives the same result with more objects. |
| Multi-decoration files, bling cutter, sequin shapes and fixings, cross-stitch app | P9 | Done in P9 Milestones 1 to 5, but sequin shapes and fixings (Milestone 6, when a tester has a sequin machine). |
| Scan graphic, WLAN send, machine runtime setup | Out of scope | The operating system scans; no maker publishes a network protocol (ADR 0036). |

**Ribbon (owner request).** The panels that do not fit the window fold
into one button each (ADR 0010); a new Lettering tab; the Digitize tab no
longer repeats the Import panel of the Artwork tab.

## Audit of P7 (2026-10-23)

After P7, a check of the code and the documents of the phase.

**Fixed.**

- The worksheet said "Upright" for a design in a turned hoop, and its
  arrow to the top of the design did not turn. It now says how far the
  design is on a slant in the hoop, and the arrow follows.
- Zoom to hoop showed the box of a straight hoop; it now shows all of a
  turned hoop.
- A product picture compared its image by the memory it is in, so a
  design with one read back from its file was never the same as before
  (the backdrop compares the bytes too; the product picture does now).
- Auto hoop could choose a multi-position hoop, but one machine file
  sews in one position only. Auto hoop leaves them out.
- Export of a design with hoopings, or on a multi-position hoop, wrote
  one file with no word about the hoopings. The export report now says to
  use Hooping > Output hoopings.
- Hooping mode stayed on (with a selection of the old design) when the
  last tab closed. It ends with the other per-design states now.
- Preview hoopings kept the old hoopings when a hoop of My hoops changed
  size. The hoopings are made again when the hoop changes.
- A start or end point of Hoop options showed its place in design
  coordinates; with the design turned that is not what the canvas shows.
  It shows and takes the place as the canvas shows it now.
- Tests: a project file keeps all P7 settings (hoop angle, placement,
  start and end, hoopings, product with a picture, repeats), and a file of
  P6 reads with the defaults; Auto hoop and multi-position hoops; export
  and hoopings; the turned hoop and Zoom to hoop; the worksheet slant.
- Speed check: the hoopings of each of the 38 corpus designs on the
  100 × 100 hoop take 0.28 s in all (release build), so no work was
  needed there.

**Left as it is.**

- `Document::set_limit` is used only by a test of the undo limit; it
  stays as part of the history API.
- Satin columns, lettering and stitch objects are not cut at split lines
  (a warning says so); cutting them is P10 work if users ask for it.
- Products and hoopings are in view coordinates (after the design
  rotation), as the guides are; a change of the design rotation moves
  them on the design.

**Check of the whole project (after the P7 audit).**

- Public items that only tests use, unused dependencies, `TODO` and
  debug prints, `allow` attributes (five `too_many_arguments` on drawing
  functions, kept): nothing else to remove.
- Open-field mode now lists the hoops that the design fits (Design info,
  "Hoops it fits"; the spec asks for it since P2; multi-position hoops are
  left out).
- The documentation builds with no warnings (`cargo doc --workspace
  --no-deps`): links in the port headers, two links to private constants
  and one ambiguous link fixed.
- README: the feature list has the P7 features; CLAUDE.md has the rules of
  the spec.

## Audit of P6 (2026-10-23)

After P6, a check of the code and the documents of the phase.

**Fixed.**

- A new or opened design in a blank tab kept the objects that Alt+V or
  Hide others had hidden (by id), so objects of the new design could stay
  hidden; and the Picture to design wizard of the old picture stayed
  open. Both are reset now; a change of design tab closes the wizard.
- Convert to appliqué took fills only, and changed a linked fill without
  its link record, so the next move made it a fill again. It takes closed
  vectors too now, and a linked object stays linked
  (`Design::convert_linked`).
- The Background step of the wizard kept every undo step (a copy of the
  picture each); it keeps the last 40 now.
- Dead code removed: `emb_formats::date_stamp` (the app uses `chrono`),
  `emb_letter::bundled::DEFAULT`, `emb_stitch::LETTER_TRAVEL_MM`,
  `Document::redo_len`, `Design::object_at` (the canvas picks with its own
  filter of locked and hidden objects; the tests keep it as a helper).
- A test for Remove overlaps on appliqué (from the last audit, moved to
  P6).

**Moved to a later phase.**

| Item | From | To | Reason |
| --- | --- | --- | --- |
| EMF and WMF vector output | P6 | P10 | Object outlines as SVG do the job for vector programs and cutters. |
| TIFF and GIF pictures | P2, P6 | P10 | PNG, JPEG and BMP cover most artwork. |
| Edit the picture in an external program | P6 | P10 | Save the picture, edit it, use Replace in the Backdrop panel. |
| A trained model to find the subject of a photo | P6 | Owner decision | Cut out subject (GrabCut) works with no extra weight (ADR 0031). |

## Phases

### P1. Stitch engine and core formats (L) — done (sew-out test covered by later kits)

The objects that you can draw today become sewable.

- `docs/design/export-plan.md` milestones 2–6: VP3; tatami fill with underlay and pull
  compensation; satin columns (Column A and B behavior: rails and rungs) with
  underlay, pull compensation, shortening and auto split of long satins; PES; CLI
  and golden tests.
- Column C behavior: a centerline with a fixed width becomes a satin.
- Machine format settings per machine profile (trims, jumps, max stitch length).
- Remove small stitches; auto start and end (closest join between objects).

**Exit:** the spec corpus (circle fill, satin column, S-curve, fill with holes,
two-color patch, full-field design) sews flat on the Topaz 20 from VP3.

### P2. Digitizing toolset (XL) — done in software, user test open (`docs/design/p2-plan.md`)

The professional input and editing tools. This is the old Phase 1c plus the
additions of the reference product.

- **Input methods:** Column A/B/C, complex fill, complex turning, run, triple run,
  backstitch, stemstitch, backtrack, repeat, manual and triple-manual stitches,
  star, ring, basic shapes, rounded rectangles (per-corner radius), freehand open
  and closed with smoothing, continue digitizing, redwork.
- **Reshape:** nodes (corner/curve, add, delete, convert), Bézier handles, stitch
  angle lines, entry and exit points, smooth curves, keep or omit last stitch.
- **Stitch angles and holes:** add/remove stitch angles; add, remove and fill holes.
- **Splitting and joining:** knife, break apart, branching, closest join.
- **Selection:** freehand, polygon, polyline, select at needle, select by color and
  by stitch type, next/previous object.
- **Transform and arrange:** skew, rotate by angle, mirror by reference line,
  transform by reference points, align, space evenly, make same size, group,
  ungroup, lock, unlock.
- **Sequence:** back/forward one color (the rest is done).
- **Viewing and travel:** stitch cursor; travel by stitch (1/10/100/1000), object,
  segment, color, function, trim; view by color; hide others; show needle points,
  connectors and function symbols.
- **Workspace:** property bar, units (mm/inch), options dialog, major/minor grid,
  rulers, guides, snap (grid, guides, objects, radius), measure, zoom tools
  (marquee, 1:1, factor, to selected, to hoop), hoop rotation, prompt bar for all tools.
- **Backdrop:** bitmap import as a locked, dimmable tracing layer (crop and adjust
  come in P6).

**Exit:** a new user digitizes a two-color patch from a bitmap in less than 15
minutes, with no manual (old Phase 1c criterion), and an experienced digitizer can
reproduce the corpus with the traditional input methods.

### P3. Production, color and quality (L) — done in software, user test open (`docs/design/p3-plan.md`)

What a shop needs to produce and hand over a design.

- **Threads and color:** commercial and custom thread charts, Threads docker
  (search, match), colorways and colorway editor, background and fabric display
  colors, color wheel, cycle used colors, split palette color.
- **Fabrics and styles:** auto fabric presets; styles, favorites, make properties
  current / apply current properties; design templates (new from template,
  save as template).
- **Machine functions and stitch editing:** insert, edit and clear stop, trim,
  tie-off, jumps, speed; stitch edit; split object at needle; process stitches
  (scale with density); generate stitches.
- **Machine files:** open DST, VP3, PES and others as stitches; recolor;
  recognize objects and outlines from stitches.
- **More formats:** EXP, JEF, SEW, XXX, HUS/VIP/SHV read (pyembroidery references).
  Done later in P3: TBF and TAP read and write, DSB, DSZ, 10O and 100 read, and
  profiles for multi-needle machines (Tajima, Happy, Toyota, Ricoma, BAI,
  Highland; ADR 0014). Added on 2026-10-26: the needles of multi-needle
  machines (the needle of each color, the threads on the machine, thread
  changes, needles in TBF and U01, the needle chart on the worksheet; ADR
  0063). Open: a test on a Tajima and a Barudan machine.
- **Output:** production worksheet, approval sheet, print preview, PDF, barcodes,
  design properties CSV, output-file profiles and batch output, design bitmap.
- **Views:** stitch player, overview window, stitch list docker, design information
  docker, 2D thread-look preview (the first step toward a realistic thread look).
- **Appliqué:** appliqué objects with placement, cut, tack and cover; with and
  without holes; combine appliqué; partial appliqué; appliqué fabric display.
- **Workspace:** design tabs, tab groups, split window, floating toolbars,
  automatic backups and recovery, screen calibration, machine runtime estimates.

**Exit:** five test users complete three set tasks with no help (old Phase 2
criterion); one command makes the machine file, the project file and the worksheet.

### P4. Lettering (XL) — done (`docs/design/p4-plan.md`)

The main reason many home users buy software.

- **Font engine:** our own embroidery font format (letters as embroidery objects
  with baselines and recommended size range).
- **Fonts:** convert TrueType/OpenType outlines to satin or fill letters; a starter
  library made from OFL-licensed fonts and project-digitized alphabets.
- **Lettering objects:** baselines (straight, arc, circle, custom path), letter and
  line spacing, kerning (automatic and manual), justification, per-letter
  reshaping, user-refined letters, minimum/maximum size warnings.
- **Lettering effects:** envelopes (bridge, pennant, perspective, diamond), lettering
  art, stitch types and underlay for letters, lettering sequence optimization.
- **Personalization:** monogramming (styles, borders, ornaments, templates), team
  names (lists, name drop, combined or separate output), keyboard design collections.
- **Custom lettering:** create letter, alphabet creator, manage alphabets.

**Exit:** names at 10 mm and 25 mm sew legibly on towel and cap; a 20-name team
list exports in one step.

### P5. Decorative stitches and effects (XL) — done in software, sew-outs open (`docs/design/p5-plan.md`)

The Designing-level stitch catalog. Each item is a pure engine function with
golden tests (spec, "Stitch engine requirements").

- **Outline types:** sculpture run, motif run, string, raised satin, zigzag, E stitch,
  tatami border, program-split border, square, coil, contour.
- **Fill types:** raised satin, zigzag, E stitch, coil, square, double square, island
  coil, string, program split, motif fill, motif column, cross stitch, contour,
  spiral, maze, offset, stipple (run, backstitch, stemstitch), laydown.
- **Effects:** auto jump, fractional spacing, mitre/cap/lap corners, jagged edge,
  trapunto, user-defined split, flexi split, accordion spacing (preset and custom
  profiles), radial fill, Florentine, liquid, 3D warp, hand-stitch effect, cording,
  color blends, multi blend.
- **Libraries and creators:** motif library and create motif, program-split library
  and create pattern, carving stamps, gradient motifs, buttonholes, borders.

**Exit:** every stitch type and effect has a corpus design, a golden test and a
sew-out photo in `tests/sewouts/`.

### P6. Artwork, vector tools and auto-digitizing (XL) — done in software, exit tests open (`docs/design/p6-plan.md`)

Replaces the separate graphics mode with vector tools in the same canvas.

- **Vector layer:** vector objects without stitches; draw, reshape, fill/stroke;
  import PDF, AI (PDF-based), EPS, EMF/WMF; export vectors. Imported SVG paths
  can then stay paths with no stitches (today they become embroidery objects:
  fills and lines, or lines only).
- **Shaping:** weld, intersect, exclude, front-back, back-front, flatten, divide,
  combine; keep originals; overlap amount (booleans already exist in `emb-geom`).
- **Offsets and arrangements:** simple, open and outline offsets, add border;
  mirror-merge, array, reflect, wreath, kaleidoscope; automatic knife; remove overlaps.
- **Border trace (owner request, 2026-10-22).** One command on a selected
  object: make a border round it at a set thickness, in a thread color that
  the user chooses, and cut the object and its border out of the objects
  that sew before it (so no area sews twice). A choice of sew order for the
  object and its border: fill then border, or border then fill. A gap
  option: no border stitches, only the cut, so the fabric shows as a line
  of negative space between the object and the objects under it. Changes
  of thickness or gap update the cut. It uses the offsets and remove
  overlaps of this phase.
- **Vector export (owner request, 2026-10-22).** Object outlines as SVG
  paths (for vector programs and cutters). The design picture as SVG is
  done (P3 output).
- **Conversion:** vectors to embroidery and back; tag as appliqué; tag as turning satin.
- **Bitmaps:** crop (with finalize), adjust, color reduction ("prepare colors"),
  color matching to thread charts.
- **Auto-digitizing:** auto-trace to vectors; click-to-fill (with and without
  holes), click-to-satin, click-to-centerline, click-to-outline; whole-design
  smart digitizing (instant and guided).
- **Photo digitizing:** line-row photo stitch, multi-color photo stitch, textured
  photo stitch.

**Exit:** a two-color logo PNG becomes a sewable design with less than 10 minutes
of manual touch-up; a photo becomes a recognizable stitch portrait.

### P7. Hooping, layouts and presentation (L) — done in software, exit test open (`docs/design/p7-plan.md`)

- Hoop templates, hoop placement settings, auto hoop, multi-position hoops.
- Multi-hooping: auto layout, split lines, overlap, registration marks, preview and
  calculate hoopings, per-hooping output.
- Product backdrops and product visualizer; virtual decoration image.
- Design repeats (continuous and overlapping designs).

**Exit:** a 360 × 260 mm design split for the 260 × 200 hoop sews with aligned joins.

### P8. Management and connectivity (L) — done in software, exit test part 2 open (`docs/design/p8-plan.md`)

- Design library: folders, search, sort, group, filter, thumbnails, catalog print,
  CSV list, new from selected.
- Embroidery clipart docker; export queue docker with history.
- Send to USB or removable media; machine connections with publicly documented
  protocols only.
- Remaining machine formats from open references; appliqué cutter output (SVG/DXF).

**Exit:** a 1000-design library is searchable in under a second; a design goes to a
USB stick for the Topaz in one action.

### P9. Specialty decoration (XL, low priority) — Milestones 1 to 5 done in software; 6 and 7 wait for a tester (`docs/design/p9-plan.md`)

For machines with extra devices. Each part starts only when an owner of such a
machine can test it.

- Sequins: single and twin, runs, fills, palettes, shapes, fixings, flip sequins.
- Beads: runs, fills, palettes. Bling (rhinestones): palettes, runs, fills,
  lettering, overlap removal, cutting output.
- Chenille (chain, moss), Schiffli functions, cross-stitch editor.
- Multi-decoration output files.

**Exit:** per device, a design sews or cuts correctly on a real machine.

### P10. Polish and assistance (L) — done in software, exit tests open (`docs/design/p10-plan.md`)

- Full 3D thread preview, with lighting and fabric texture.
- Start page with recent files, templates and tutorials; context help (F1).
- Simple mode that hides advanced tools; lettering kiosk mode.
- Optional assistant panel that answers embroidery questions: benched (owner
  decision, 2026-10-24). It is not necessary for the architecture; the help
  search of P10 Milestone 4 answers from the help text.
- Accessibility, translations, installers for Windows, macOS and Linux.
- The small features that earlier phases moved here: see the tables "Moved
  to a later phase" (audit of 2026-10-23) and "Feature review" (audits of
  P8 and P9).

**Exit:** the 15-minute patch test passes for users of Windows, macOS and Linux.

### P11. Features of other programs (XL) — done in software, exit tests open (`docs/design/p11-plan.md`)

Features that the reference product does not have, or that the inventory did not
list, from a review of other programs (owner request, 2026-10-25):

- Density map over all objects, and the thread use and cost of a design.
- More thread charts (the GPL palettes of Ink/Stitch; owner decision,
  2026-10-25) and more bundled embroidery fonts, also for small text.
- Design pictures as thumbnails in Explorer, Finder and the Linux file
  managers (the Windows handler needs an exception to rule 9).
- New stitch types from Ink/Stitch: tartan, ripple, meander, circular,
  guided and linear gradient fills.
- Free-standing lace, quilting helpers, puff (3D foam) and cap sequencing.
- Cutwork needles, when a tester has such a machine.

Not planned: the font files of other embroidery programs (no public description of the formats),
WiFi and cloud send (no published protocol).

**Exit:** the sew-outs and user tests of each milestone pass (see the plan).

### P12. Picture to design: robust and better results (L) — done in software, exit tests open (`docs/design/p12-plan.md`)

The owner asked (2026-10-25) to make the Picture to design wizard more
effective and more robust:

- Robustness: photos upright, previews in the background, a true width
  for Auto, and a bench of test pictures with numbers.
- Better areas: small areas on top (not holes), fill angles along each
  area, small areas in mm, the checks of Design info before Finish.
- Lines and colors: line art as lines, edge-preserving smoothing, colors
  from the thread chart.

**Exit:** the bench numbers get better, and a logo and a drawing sew on
the Topaz 20 with no gaps (see the plan).

### P13. Shop tools (L) — done in software, exit tests open (`docs/design/p13-plan.md`)

The tools that a shop uses every day to sell, plan and supply its work
(owner request, 2026-10-27):

- Quotes: a price list (stitches, setup, digitizing, colors, quantity
  breaks, rush, tax), a quote sheet, and the run time of an order on
  machines with more heads.
- Order details (customer, order number, garment, sizes, placement, due
  date) on the worksheet and the approval sheet, with a line to sign.
- Thread stock: color matching with only the threads in stock, and a
  list of the threads to buy for an order.
- Scripts: tools that a user writes, with no access to files or the
  network, in the program and in `emb-cli`.

Not planned: a customer database, invoices and accounts (a shop
management system), and on-line thread orders.

**Exit:** the quotes and run times of real orders agree with a shop
tester's numbers, and the sample scripts pass (see the plan).

### P14. Sketch mode (XL) — Milestones 1 to 6 done in software (`docs/design/p14-plan.md`)

A parametric sketch, as in CAD programs (owner request, 2026-10-27):

- Lines, arcs and circles with geometric constraints and dimensions, and
  a solver in pure Rust (new engine crate `emb-sketch`).
- Sketch mode in the app: draw, constrain, dimension and drag; the color
  shows a fully constrained sketch or a conflict.
- Drawing help (constraints added while drawing) and edits (trim, fillet,
  offset, mirror, patterns).
- The closed regions of the sketch become fills, satins, runs or
  appliqué, and stay linked: a change of a dimension makes them again.
- Named parameters and expressions, splines, vectors to sketches.

**Exit:** a badge made from a sketch sews on the Topaz 20, and sews again
after a change of its width (see the plan).

## Dependencies

```text
P1 engine ─┬─► P2 digitizing ─► P3 production ─► P4 lettering
           │                         │
           └─► P5 stitch types ◄─────┘ (needs P3 style/property system)
P2 + P5 ─► P6 artwork and auto-digitizing ─► P7 hooping ─► P8 management
P5 + P6 ─► P9 specialty        P1–P8 ─► P10 polish
P1–P10 ─► P11 features of other programs (P5 stitch types, P7 hoopings, P10 installers)
P6 + P11 ─► P12 Picture to design: robust and better results
P3 + P8 + P11 ─► P13 shop tools (sew time, sheets, thread use)
P6 ─► P14 sketch mode (vector objects, linked shapes)
```

P5 engine work can run in parallel with P2–P4 app work, because the engine crates
do not depend on the app (spec, "Architecture").

## Mapping from the old phases

| Old (spec) | New |
| --- | --- |
| 1a Formats, 1b Engine | P1 |
| 1c App MVP | P2 |
| 2 Usability | P3 (and appliqué from P3) |
| 3 Advanced | P4 (lettering), P6 (auto-digitizing), P7 (multi-hooping), P10 (3D preview) |
