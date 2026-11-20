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
| 1. Stitch engine and core formats | Done in software: running, tatami fill, satin with underlay, VP3, DST, PES, `emb-cli`, golden tests. The exit test (corpus sew-outs) is covered by the later kits (owner decision, 2026-10-23). CI is in `.github/workflows`: Windows and Linux on each push, macOS for a release (ADR 0060). The Ink/Stitch comparison is closed (owner, 2026-11-18: testers who used Ink/Stitch before report better results from this program, and their files sewed well). See `docs/design/export-plan.md`. |
| 2. Digitizing toolset | Done in software: all seven milestones (tracing, workspace, input methods, stitch angles and holes, split/join/select, arrange, viewing and travel). Open: the exit test with users. See `docs/design/p2-plan.md`. |
| 3. Production, color and quality | Done in software: Milestones 1 (thread charts, Threads panel, colorways), 2 (machine functions, split at needle, stitch list) 3 (machine files: open as stitch objects; EXP, JEF, XXX, SEW, HUS, SHV) 4 (stitch player, thread look, design info, warnings, overview), 5 (worksheets, PDF, picture, Output design, batch output), 6 (fabrics, styles, templates, plain-language labels), 7 (appliqué) and 8 (design tabs, split view, backups and recovery, calibration, sew time, P2 carry-overs) done in software; the exit test with users is open. See `docs/design/p3-plan.md`. |
| 4. Lettering | Done: Milestones 1 (TrueType lettering: satin letters from glyph strokes, fill letters, lines, spacing, kerning, alignment, bundled OFL fonts and the fonts of the computer) 2 (embroidery fonts: `.embfont` format, converter, four starter fonts, Fonts window with previews, size ranges and warnings) 3 (arc, circle and line baselines; kerning, move, scale, turn and reshape of single letters) 4 (envelopes and lettering art; stitch type and underlay for each letter; sew order with optimized connections) 5 (monograms: styles, borders, ornaments, templates), 6 (team names and keyboard design collections) and 7 (custom alphabets, bitmap fonts) done: all seven milestones, and the exit test (sew-outs on towel and cap, 2026-10-23). See `docs/design/p4-plan.md`. |
| 5. Decorative stitches and effects | Done in software: Milestones 1 (column types satin, zigzag, E stitch, raised satin, tatami, square, coil and contour; lines with a width as outline types; sculpture run) 2 (fill types contour, offset, spiral, island coil, radial, maze and Florentine; liquid satin) 3 (texture fills: satin, zigzag, E stitch, raised satin and coil across shapes, square, double square, string, cross stitch, stipple, laydown) 4 (motifs: motif run, column and fill, gradient, 3D warp, library, create from the selection) 5 (patterned stitching: program split, flexi split, user split, carving stamps) 6 (stitch effects: auto jump, fractional and auto spacing, accordion spacing, corners, jagged edge, trapunto, hand stitch, cording, double underlay, pull compensation for each side and as a percent) and 7 (color blends and multi blend, buttonholes, decorative borders) done in software: all seven milestones. The exit test passed: the sew-outs on the Topaz 20 passed (owner, 2026-11-16). See `docs/design/p5-plan.md`. |
| 6. Artwork, vector tools and auto-digitizing | Done in software: Milestones 1 (vector objects) 2 (shaping) 3 (offsets and arrangements) 4 (border trace) 5 (picture tools; PDF, AI, EMF and WMF import) 6 (auto-trace, click to stitch) 7 (Picture to design wizard) and 8 (photo stitch, recognize outlines), with the owner requests (linked shapes, cut out subject). The exit tests passed: the logo and the photo, sew-outs on the Topaz 20 passed (owner, 2026-11-16). See `docs/design/p6-plan.md`. |
| 7. Hooping, layouts and presentation | Done in software: Milestones 1 (hoop placement) 2 (multi-hooping engine) 3 (hooping mode, output for each hooping) 4 (multi-position hoops) 5 (product backdrops) and 6 (repeats); the upright Crown hoop (ADR 0034). The exit test passed: the kit in `out/p7-sewout`, sew-outs on the Topaz 20 passed (owner, 2026-11-16). See `docs/design/p7-plan.md`. |
| 8. Management and connectivity | Done in software: Milestones 1 (design library engine) 2 (Library tab, catalog) 3 (Send to USB) 4 (export queue) 5 (clipart) 6 (more machine formats, the slow function) and 7 (cutter files, barcodes). Exit test part 1 passed (1000-design search in about 1 ms); part 2 passed (the Topaz 20 reads a stick from Send to USB; owner, 2026-11-16). See `docs/design/p8-plan.md`. |
| 9. Specialty decoration | Done in software for the parts that the owner can test: Milestones 1 (cross-stitch objects, ADR 0038), 2 (picture to cross stitch, charts), 3 (rhinestone objects, ADR 0039), 4 (rhinestone templates) and 5 (mixed decoration: sewn template marks, template files from Output design, order of work); audit in "Audit of P9". The sew-outs on the Topaz 20 passed (owner, 2026-11-16) (with a cut template). Milestones 6 (sequins, beads) and 7 (chenille, Schiffli) wait for a tester with such a machine. See `docs/design/p9-plan.md`. |
| 10. Polish and assistance | Done in software (the exit tests are open; see "Audit of the whole project"): Milestones 1 (editing helpers: paste after selected, paste special, close shape, reverse direction, swap sides, omit last stitch, single stones, editing options) , 2 (view and workspace helpers: center at stitch, travel by function, zoom steps, panels that close and open, close all, favorite styles, display colors, color wheel, show or hide the rhinestones) , 3 (the 3D thread look on the canvas and in the pictures, ADR 0040) , 4 (start page and help: topics in `docs/help`, search, F1) , 5 (simple mode and the lettering kiosk), 6 (files, assets and content), 7 (translations and accessibility, ADR 0042: German, Spanish, Japanese) and 8 (installers and release builds, ADR 0043) done in software; Milestone 9 (assistant) is benched. The small features that the audits moved here are in the P10 plan. See `docs/design/p10-plan.md`. |
| 11. Features of other programs | Done in software: Milestones 1 (density map and thread use, ADR 0044), 2 (brand thread charts, thirty embroidery fonts, joins of satin letters, ADR 0045), 3 (pictures of designs in the file manager, ADR 0046), 4 (tartan, gradient, meander, circular, guided fills and ripple stitch, ADR 0047), 5 (free-standing lace, ADR 0048), 6 (quilting helpers, ADR 0049), 7 (puff and cap sequencing, ADR 0050) and 8 (cutwork needles, ADR 0051); audit in "Audit of P11 and P12". The sew-outs on the Topaz 20 passed (owner, 2026-11-16). Open: the thread use against weighed spools, thumbnails after an install on macOS and Linux, a tester with a cap frame, a tester with a cutwork machine. See `docs/design/p11-plan.md`. |
| 12. Picture to design: robust and better results | Done in software: Milestones 1 (photo orientation, previews in the background, true width for Auto, the picture bench; ADR 0052), 2 (details on top, fill angles, small areas, checks before Finish; ADR 0053) and 3 (line art, edge-preserving smoothing, colors from the thread chart; ADR 0054). The exit tests passed (a phone photo, a logo and a drawing sewn on the Topaz 20; owner, 2026-11-16); audit in "Audit of P11 and P12". See `docs/design/p12-plan.md`. |
| 13. Shop tools | Done in software (owner request, 2026-10-27): Milestones 1 (quotes and run time on machines with more heads, ADR 0064), 2 (order details on the sheets, ADR 0065), 3 (thread stock and threads to buy, ADR 0066) and 4 (scripts, ADR 0067). Open: the exit tests with a shop tester (real orders, a printed sheet and its barcode, the spools to buy). See `docs/design/p13-plan.md`. |
| 14. Sketch mode | Done (owner request, 2026-10-27): Milestones 1 (sketch model and constraint solver, ADR 0068), 2 (sketch mode in the app, ADR 0069), 3 (drawing help and sketch edits, ADR 0070), 4 (regions to embroidery, ADR 0075), 5 (objects that follow their sketch, ADR 0076) and 6 (parameters, splines and drawings to sketches, ADR 0078 and 0079). The owner's test in the app and the badge sew-outs on the Topaz 20 passed (owner, 2026-11-16). See `docs/design/p14-plan.md`. |
| 16. Product view | Done in software (owner request, 2026-11-17): Milestones 1 (a realistic look, ADR 0101), 2 (other designs on the product, ADR 0102), 3 (front and back, ADR 0103), 4 (the product on the approval sheet, ADR 0104), 5 (better products, ADR 0105), 6 (two-tone products and swatches, ADR 0106), 7 (photos of the user's products, ADR 0107) and 8 (turntable view, ADR 0108); audit in "Audit of P16". Open: the exit tests of the eight milestones (owner, at the end of the phase). See `docs/design/product-plan.md`. |
| 17. Polish and lettering | Accepted (owner, 2026-11-18); Milestones 1 (update check, ADR 0109), 2 (pictures of the windows, ADR 0110), 4 (font test sheet and letter corrections, ADR 0112), 5 (ten tuned fonts, ADR 0113), 6 (text shaping, ADR 0114) and 7 (quality score of the fonts, 55 fonts in the program and 68 in font packages, ADR 0115; then pen strokes in the stroke finder and routes for all fonts, ADR 0116, and ten fill fonts for large text, ADR 0117) and 8 (content families: 71 motifs, 48 split patterns, 39 stamps, 50 borders and 45 clipart items, ADR 0118) done in software, Milestone 3 set aside by the owner: eight milestones (update check, pictures of the windows, tests on an industrial machine, font test sheet and letter corrections, ten tuned fonts, text shaping, more fonts with a quality score, content families). See `docs/design/p17-plan.md`. |
| 18. Designs from other programs, and the 3D stitch view | First part: accepted (owner, 2026-11-18), started 2026-11-19, Milestones 1 to 3 (the parts of a stitch file; runs and satin columns; fills) done (ADR 0131, ADR 0132, ADR 0133), Milestone 4 (Recognize Objects) done except the sew-out of the owner (ADR 0134), Milestone 5 (read Ink/Stitch files) done except the files of testers (ADR 0135), Milestone 6 (write Ink/Stitch files and read its fonts) done except the test in Ink/Stitch and the bundled fonts (ADR 0136), Milestone 7 (outlines with a stitch file) done except the test of the owner (ADR 0137), Milestone 8 (EPS files, and a message for CDR files; changed by the owner on 2026-11-20 from import helpers) done except the files of testers (ADR 0138): eight milestones (the parts of a stitch file, runs and satin columns, fills, Recognize Objects, read Ink/Stitch files, write them and read its fonts, outlines with a stitch file, EPS files). See `docs/design/p18-plan.md`. Second part (owner request, 2026-11-19): a thorough upgrade of the engine of the 3D stitch view, started first (owner, 2026-11-19); Milestone 1 (measures and a reference set) done except the sew-out (ADR 0122, `docs/design/p18-view-m1-report.md`); Milestone 2 (linear light, thread kinds and weights) done (ADR 0123); Milestone 3 (the surface with heights) done (ADR 0124); Milestone 4 (the light on the surface) done except the compare with photos (ADR 0125); Milestone 5 (the canvas on tiles) done, with a speed pass (ADR 0126); Milestone 6 (the fabric) done except the compare with photos (ADR 0127); Milestone 7 (what is not thread) done except the compare with sew-outs (ADR 0128); Milestone 8 (the viewer) done except the check of the owner (ADR 0129); Milestone 9 (all pictures, quality, audit) done except the test with users (ADR 0130): nine milestones (measures and a reference set, linear light and thread kinds, the surface with heights, the light on the surface, the canvas on tiles, the fabric, what is not thread, the viewer with tilt and light by hand, all pictures and quality settings). See `docs/design/p18-view-plan.md`. Audit of both parts in "Audit of P18" (2026-11-20). |
| 19. Help and tutorials | Proposed (2026-11-18): nine milestones (pictures in the help, Show Me, guided lessons, the sew-out doctor, samples, ways to start, the help on the web, moving pictures, better answers); the owner selects them. See `docs/design/p19-plan.md`. |
| 20. Japanese lettering | Accepted (owner, 2026-11-20, before v0.4.0); Milestone 1 (the Japanese reference set and the lettering bench, a test font from Noto Sans JP, and font outlines with the nonzero rule) done (ADR 0141); Milestone 2 (the stitch of each column in Auto: satin to 12 mm with split stitches, tatami columns above, the width of a column between its sides) done (ADR 0142). Milestone 3 (one stitch for the text, crossings cut, no bulges, ends cut back in other columns, stroke ends to their corners; two layers 6.3 % to 1.4 %, each character covered 95 % or more) made, the sew-out of the owner is open (ADR 0143, `tools/make-sewout-kit-p20.py`). Milestone 4 (routes for the letters of TrueType fonts, made in the background in the app; 刺繍東京 at 20 mm 48 to 23 trims) done (ADR 0144). Milestone 5 (the height of Japanese text is the height of its characters, thin strokes sew as runs, the smallest size of a text in the Lettering window) done (ADR 0145). Milestone 6 (two Japanese embroidery fonts with the 2,136 jōyō kanji in a font package, scores 89 and 88) made, its sew-out is open (ADR 0146). Six milestones (a Japanese reference set and its measure, the stitch of each column, crossings and junctions, routes for the fonts of the computer, sizes and thin strokes, a Japanese embroidery font); the owner accepts them. The measures of 2026-11-20 (the fonts of the owner's computer): a character becomes all tatami when one column is wider than 8 mm (128 of 149 characters in Yu Gothic Bold at 50 mm), crossings have two layers on 10 % to 16 % of a kanji, and the letters of a font of the computer have no route (twice the trims). See `docs/design/p20-plan.md`. |
| 15. Browser version | Later (owner decision, 2026-11-16): not started. Start after the first feedback of users on the desktop program (about v0.4). See "P15. Browser version" below. |

## Review of 2026-11-18 and the next phases

After the audit of P16, a review of the project against the reference
product (the documents, the code, and the numbers of the public
repository). The inventory shows nearly all features as done; the gaps
are in depth and in proof. The owner decided the same day to make the
features that the program has better before it gets new ones.

**What the owner reported.**

- About five users tried the program; two sent files to their machines,
  and the files sewed well. Testers who used Ink/Stitch before report
  better results and an easier program, so the comparison with Ink/Stitch
  (open since P1) is closed.
- The owner has an industrial multi-needle machine for tests (a Tajima
  with 12 needles, all its hoops and a cap frame).
- Professional users will do their daily work in the program in place of
  a consumer embroidery product, and report where they cannot continue.
- The translations are for the comfort of users that the owner knows;
  their review by native speakers stays open.

**The gaps, from most to least important, and what is planned.**

| Gap | Planned |
| --- | --- |
| The stitches are proven on one home machine only | P17 Milestone 3: a test kit for the industrial machine, reports of professional testers |
| A machine file of another program becomes one plain fill; the native formats of closed-source programs cannot be read | P18: recognize runs, satin columns and fills; read and write Ink/Stitch files; outlines with a stitch file. Not planned: the native formats of closed-source programs (rule 3) |
| Thirty fonts, all converted by the program; small libraries of motifs, split patterns, stamps, borders and clipart; no text shaping | P17 Milestones 4 to 8 |
| A user of an old version does not learn of a new one; the packages have no signature | P17 Milestone 1 (update check); code signing continues (ADR 0062) |
| No window is checked automatically; texts in German and Japanese are longer | P17 Milestone 2 (pictures of the windows, development only) |
| The help is text with no pictures; seven tutorials; no videos | P19 (proposed) |
| EPS and CDR artwork cannot be read | P18 Milestone 8 (import helpers; changed by the owner on 2026-11-20: the program reads EPS files, and a CDR file gives a message, ADR 0138) |

**Decided, no work.**

- More formats to write: no. All machines of the machine library have a
  format that the program writes, and DST is the format that users asked
  for most. A format is added when a tester has a machine that needs it.
- Sequins, beads, chenille, Schiffli and sending over a network wait for
  testers and public protocols, as before.
- The rule about the names of closed-source programs stays.

**Checked and good:** undo with shared snapshots (ADR 0004), the stitch
cache for each object, the engine crates with no UI, the same stitches on
all systems (ADR 0003).

## Sew-out tests of 2026-11-16

The owner sewed the test kits of P1 to P14 on the Topaz 20 (the weekend of 2026-11-14) and cleared all the exit tests on that machine. The tables of exit tests in this file, the phase table of the spec and the plans say so.

Still open, because they need other people, machines or systems:

- Tests with users: P2 (a new user digitizes a patch in 15 minutes), P3 (five test users), P10 (the patch test on Windows, macOS and Linux, F1 and the help search with a new user, a screen reader, a review of the German, Spanish and Japanese texts by native speakers).
- Other machines: cap order on a cap frame and cutwork on a cutwork machine (P11); sequins, beads, chenille and Schiffli (P9 Milestones 6 and 7).
- Other systems and tools: the thumbnails after an install on macOS and Linux (P11); a phone app reads the barcode of a worksheet, and a cutter program opens a cutter file at real size (P8 Milestone 7, P13).
- Measurements and shop tests: the thread use against weighed spools (P11); the quotes and run times of real orders with a shop tester (P13). The comparison with Ink/Stitch (P1) was closed on 2026-11-18 ("Review of 2026-11-18").

## Owner requests of 2026-11-20: the icon and the stitch player

- **A simpler icon.** The logo of the software of the owner's shop (a
  purple star in a hoop, with a needle and a thread) is the icon now
  (ADR 0140): `data/icon/icon.svg`, and `tools/make-icon.py` makes the
  pictures from it. Open: the license of the logo (its maker and the
  owner).
- **The stitch player with the 3D look.** The 3D look came seconds after
  the needle, the player was jumpy, and a rectangle of the flat look
  showed for a frame when the tiles changed. The tiles now follow the
  needle as fast as they can be made, and a tile keeps its picture until
  the new one is there (ADR 0126, the note of 2026-11-20). With the
  measure `stitch_tiles::tests::player_budget`: 0.02 s behind the needle
  at 200 stitches a second (2.1 s before), 0.14 s at 2,000, and no frame
  with a rectangle of the flat look (28 of 470 before). The owner checked
  it at 1,000 and 5,000 stitches a second.
- **Slow and flat after a drag of the player slider** (owner report, the
  same day). After the light by hand and a drag of the slider back and
  forth, the view was flat and slow for a minute after the close of the
  player: the tile threads gave thousands of results of steps that no view
  showed any more, and the frame took them 24 at a time. Now such results
  are not kept, and a tile that many changes reached is made again all,
  not in parts (ADR 0126, the second note of 2026-11-20). On the owner's
  design: all tiles 0.5 s after the close (21 s before).

## Owner list of 2026-11-16 (v0.2.1 and v0.3.0)

After v0.2.0 the owner sent a list of changes and a start bug (the program showed nothing on a friend's PC). The work is in six stages: the start bug and the installer (v0.2.1), first start and saved preferences, endless drag, sketch changes, bold, italic, underline and stacked lettering, and a machine library. See `docs/design/v0.3-plan.md`.

### Audit of the v0.3 work (2026-11-16)

The audit of Stages 1 to 6 and the IP review of the release (the rules of the review of 2026-10-26: no names of closed-source programs, the trademark notes, the third-party list, a scan for secrets, the install documentation):

- Fixed: Align and Space Evenly moved the objects of one sketch once for each selected object (a sketch and its objects move together, ADR 0097); the objects of one sketch are now one unit. The lettering kiosk turned on Thread Look in the saved View options; the kiosk options are no longer kept. A panic of a background thread before the first frame started the program again with another renderer; only the main thread counts now. In My Machines, the format for new files follows the selected formats.
- Clean: no names of closed-source programs (Blender, FreeCAD and SolveSpace are open source), no secrets or personal data in the new commits, `THIRD-PARTY.txt` is current, and the code builds with Rust 1.95 (the `rust-version`).
- Docs: the README has the v0.3 features, the upgrade and start notes and a repaired install table; the spec has the machine library and a roadmap row for this work.
- Open: the start test on the PC of the owner's friends. Passed (owner, 2026-11-16): endless pan with a real mouse, the My Machines window. The installer removes the older version before it asks to close a running ShokujoDX, so a Cancel at that question leaves no version installed (install again to fix).


## Audit of the whole project (2026-11-20)

After the audit of P18, a second check of all crates and documents (the
first was on 2026-10-25, after P10). The owner asked for three things:
code that the program does not need, documents that are out of date, and
oversights of the architecture.

**Fixed: architecture.**

- **The program could save a design that it could not open again.** The
  limit of a project file (all parts less than 1 km from the origin, and
  real numbers) was a rule of the loader only. A script that moved the
  objects 2 km (`move_objects(objects(), 2000000.0, 0.0)`), or that
  divided 0.0 by 0.0, gave a design that the program saved; the file did
  not open, and the save put it in the place of the good file. A typed
  move or scale, and a machine file with a stitch km away, gave the same.
  A writer of machine files splits a long move into the largest moves of
  its format, so a move of 10^9 mm was 80 million records. Now
  (ADR 0139):
  - a document does not make a change that puts its design out of range:
    the design of before comes back, the history gets no step, and a
    notice says why ("The change was not made");
  - `project::save` reads its text again before it writes the file: a
    design that the loader refuses is not written, and the file that is
    there stays;
  - a script that makes such a design gets an error, and a machine file
    with a stitch more than 1 km from the center of its design gives an
    error;
  - the loader, the document and the artwork readers use one check
    (`project::in_range`, `project::out_of_range`).
- **Two engine crates were not in the check for the same numbers on all
  systems (ADR 0003).** The test `emb-geom/tests/determinism.rs` had the
  eight engine crates of 2026-10-25; the sketch crate and the script
  crate came later. The power of a parameter expression of a sketch
  (`2 ^ 0.5`) used the math library of the system; it uses
  `emb_geom::math::pow` now. The test also stopped at the first
  `#[cfg(test)]` of a file, so the code after a test module of another
  file (`#[cfg(test)] mod probe;`) was not checked. It reads each line
  that is not a test now, and has a test of its own.
- **The worksheet of the command-line tool named a topping for a design
  with no topping.** `emb-cli output` copied the topping of the fabric;
  the app left it out if the design says that it has none
  (`Design::no_topping`). The rule was written three times (the app, the
  tool, the 3D view), and a function for it was in the model with no user
  but its test. All three use `Fabric::topping_for` now.

**Fixed: code that the program does not need.** Removed, because no code
used them, or only their own tests: `ButtonholeShape::has_eye`,
`buttonhole::local_size`, `family::preset_clipart`, `CheckMap::thread_at`,
`Weave::has_pile` (the 3D view takes the pile from the relief of the
fabric), `FabricCatalog::topping_of` (see above), `Design::remove_knockout`
and `Design::set_knockout_overlap` (`add_knockout` sets the overlap, and
the Lettering window changes the knockout of the lettering). The three
copies of the range check of a point are one function.

**Fixed: documents.**

- The spec. The list of the crates had 9 of the 13 crates (no
  `emb-sketch`, `emb-script`, `emb-cli`, `emb-thumbnail`), and
  `emb-render` was a "2D thread preview (later: 3D)". The order of the
  crates did not show that `emb-model` uses `emb-sketch`. The ribbon had
  5 of its 12 tabs. The fields of a machine profile and of a hoop preset
  had no trim settings, needles, heads and cap frames, and the stitch
  kinds had no speed functions. Phase 0 said that CI was deferred, and
  P10 that the first release build was open (three releases are made).
  The test table named two folders that do not exist
  (`tests/differential`, `tests/sewouts`); the P5 plan and the P5 exit
  text named the second one too (the photos of sew-outs stay with the
  owner). The transfer method was an open
  question (Send to USB passed on 2026-11-16).
- The README: thirty embroidery fonts (the program has 65, and the font
  packages 72 more); "all 14 roadmap phases" (P16 to P18 are done in
  software); no product view, no 3D stitch view, no text shaping. A new
  test compares its numbers (fonts, machines, thread charts, tutorials,
  formats) with the program, as a test does for the help since the audit
  of P17.
- `CONTRIBUTING.md` said that the help topics are in English only (they
  are in four languages, ADR 0084).
- The roadmap (the P1 row: CI "runs when the repository is on GitHub"),
  the export plan (Milestone 6 "Done, except CI and the Ink/Stitch
  comparison": both are closed) and the inventory (the rows of the
  viewing modes, of the asset management and of Recognize Objects did
  not name the work of P17 and P18).
- A help text for the new notice (*Questions and problems*, four
  languages).

**Checked, no change.**

- Dependencies: each dependency of each crate is named in its code, and
  none is used only by tests. No source file is out of the module tree.
  Each tracked file of `data/`, `tests/`, `packaging/` and `tools/` has a
  user; the fonts of the font packages are not in the repository a second
  time.
- Lints: 16 `too_many_arguments`, one `type_complexity`, one
  `vec_init_then_push` and one `linker_messages`; no `dead_code`, no
  `TODO` or `FIXME`, no `todo!`, nothing deprecated.
- The other 25 public functions that only tests use are test helpers of
  the app (`#[cfg(test)]`) or the measures of exit tests
  (`parts::kinds_of_records`, `view::lay_design`, `Embroidery::of_segments`,
  `Laid::max_height_in`, `write_machine`).
- Rules 5 and 9: no cycle of crates. CI, the packaging test, the scan of
  the translations and now the determinism test have the same ten engine
  crates. Twelve crates forbid `unsafe` code; `emb-thumbnail` is the
  exception (ADR 0046).
- The 13 loops over a hash map or a hash set in the engine crates sort
  their result or do not depend on the order.
- The program starts another program only to show a folder, to open a
  file or a mail, to ask for a new version (`curl`, ADR 0109) and to
  start itself again with another renderer. No other code uses the
  network.
- `CLAUDE.md`: each code name and file path in it is in the code (a
  script checked them, and those of the spec, the README,
  `CONTRIBUTING.md`, the help and the inventory), and its rules are the
  rules of the spec. The four language files have the same 4,313 texts.

**Left as it is.**

- `emb-app/src/sketch.rs` has 6,500 lines (sketch mode: tools, keys,
  drawing), `AppState` has 185 fields, and `ribbon::commands::meta` is one
  table of 1,560 lines. They work and have tests. A split of `sketch.rs`
  into a folder module (as `ribbon/`, 2026-10-29) is the next step when
  sketch mode gets more work. *Done on 2026-11-20 (owner request):
  `sketch/` is a folder module of twelve files and its tests. Each item
  moved with no change; the items that more than one file uses are
  `pub(super)`. The three longest functions (`draw_overlay`,
  `handle_input` and `make`, 340 to 500 lines) are as they were.*
- The ADRs name the code as it was at their date (a name in each of
  ADR 0042, 0124 and 0127 is gone or has another name now). An ADR is a
  record, so they stay.
- The math functions of Rhai (`sin`, `exp`) use the math library of the
  system. A script runs one time, and its result is in the design
  (ADR 0139).
- The check for public functions with no user compares names: a method
  with a common name (`new`, `width`) can pass it.
- The open items of the audit of P18: a stitch file that sews one fill
  many times at one place, and no EPS or AI file of another program is
  tested.

**Open: needs the owner or test users.** No change: the tables of exit
tests in the audits of P16, P17 and P18, and the list in "Sew-out tests of
2026-11-16".

## Audit of P18 (2026-11-20)

After the eight milestones of the first part (designs from other
programs) and the nine milestones of the second part (the 3D stitch
view), a check of the code and the documents: probes with wrong and heavy
input for each reader of the files of other programs, the rules of the
spec, the translations, the help, the pictures of the windows and the
documents. The second part had an audit of its own in Milestone 9 (a
design 1 km wide, ten open designs; `docs/design/p18-view-m9-report.md`).

**Fixed.**

- **An EPS file could stop the computer.** A probe
  (`emb_formats::eps::probe`) runs each of the 353 operators of the
  reader with 45 sets of wrong operands (15,885 cases, in a debug build,
  which finds the overflows of whole numbers), and 50 programs that ask
  for much time or memory, each in a process of its own with a watch on
  its memory. Before: a program that painted in a loop with no end
  (fills, circles, strokes, or a long path that `gsave` copies) used more
  than 4 GB in 4 s; the first run of the probe stopped the work session
  of the audit. A program that made subpaths with no end did not stop in
  2 minutes: each new point counted all the points of the path again.
  Now the path keeps its length, a painted path and a copy of the path
  count for the memory limit (256 MB), and the 100,000th painted path
  stops the program with a warning. After: each of the 50 programs stops
  in less than 1 s and 350 MB; the 15,885 cases have no crash and no hang,
  and none takes more than 20 ms.
- **Shapes far away could make a project that does not open.** A large
  transform in an EPS file, or a number such as 1e30, gave shapes many km
  from the design. The program refuses a project with such a shape when
  it opens (the limit is 1 km, `project::MAX_COORD_MM`). Now all artwork
  readers (SVG, Ink/Stitch, PDF, AI, EPS, EMF, WMF) leave out a shape with
  a point farther than 1 km or with a wrong number (`svg::drop_far`), and
  a warning gives their number.
- **An Ink/Stitch file with a large repeat count took all the memory.** A
  probe (`crates/emb-cli/tests/p18_probe.rs`) sets each of the 136
  settings that the reader reads to 14 wrong values on each shape of the
  test badge (1,904 cases), and sews the design. With
  `center_walk_underlay_repeats="1e9"` the reader made 500 million
  underlay layers (more than 3 GB in 1 s). Now the reader takes at most
  10 repeats of a center walk, 4 of a bean stitch and 100 mm of
  `expand_mm` (a larger expand made an object many km wide), each with a
  warning. After: no problem; the slowest case takes 1.9 s (an expand of
  100 mm).
- **An outline could give its shape to an object of each color.**
  Recognize Objects matches the outlines to the objects of each stitch
  object (one for each color) alone. An outline round the design (a
  background), with only one small object of a color in it, gave that
  object its shape. In the probe, 5,000 colors and one outline round all
  became 5,000 fills of the background (5.3 million stitches in place of
  90,000). Now an outline takes objects only if they cover a quarter of
  its box (`part_outlines::MIN_COVER`). The exit test of Milestone 7 gives
  92.5 % (92.8 % before).
- **F1 did not open the topic of two windows.** The Tilt View window (the
  second part, Milestone 8) and the Product Photo window (P16) had an id
  that was not their name in the list of the help (`help::WINDOW_HELP`).
  Now their ids are these names, F1 over the Tilt View opens the topic
  *The 3D view*, and a test checks that each name of the list is a window
  of the program.
- The first error of an EPS file can be a name of thousands of letters
  from damaged data; the warning shows its first 40 letters now.
- The heading of P18 in this roadmap said that the first part was not
  started and named a helper program for EPS and CDR files. The README
  did not name EPS files, Recognize Objects or Ink/Stitch files.

**Left as it is.**

- A stitch file that sews one fill at one place many times: each layer
  is the underlay of the last, and the time to recognize it grows with
  the square of the rows (200 layers: 4.5 s; 500 layers: 28 s). The
  Recognize Objects window does this work when it opens, so the window
  waits. The corpus designs take at most 0.26 s each (85 designs in
  3.2 s), and many separate fills take a time that grows with their number
  (2,000 fills in 1 s).
- No EPS or AI file of another program is tested. The AI operators come
  from the format as it is known (its published description could not be
  read). A large procedure set of a commercial program can use an
  operator that the reader does not have; the warning names the first.
- An EPS program stops at 100,000 painted paths, 40 million steps or
  256 MB of memory; the import keeps what it painted before.
- A CDR file gives a message only. No font of Ink/Stitch is bundled (the
  owner chose a test only, 2026-11-20).
- The limits of the second part are in
  `docs/design/p18-view-m9-report.md` (the picture on the approval sheet,
  the sew-out animation, the short quality measure).

**Checked, no change.**

- Rules 3 and 9: the changes of the phase name no closed-source program
  (the names are the header line of EPS files and the brands of thread
  charts); no `unsafe` code; the new dependency edges (`emb-art` on
  `emb-stitch`, `emb-letter` on `emb-formats`, `flate2` in `emb-formats`
  and `emb-render`) are in the spec, and `flate2` is pure Rust and was
  already in the list of licenses; the trigonometry of the new engine
  code goes through `emb_geom::math`.
- Other input of the probes with no problem: Ink/Stitch fonts (45 cases:
  wrong page sizes, guides, transforms, widths, kerning and scales; 26 are
  refused by `EmbFont::validate`, the others sew), stitch files (15
  kinds, with and without outlines: no stitches, 10,000 stitches at one
  point, a stitch of 1 km, a column at the edge of the range, rows
  0.001 mm apart, 100,000 random stitches, 50,000 parts, 5,000 colors),
  and the fits of outlines (sizes of 0, two points on each other, an
  outline far away, 10,000 small outlines).
- Translations: the 4,308 texts of the template are in German, Spanish
  and Japanese, none missing and none extra.
- Help: the topics `artwork.md`, `file-formats.md`,
  `objects-and-stitches.md`, `troubleshooting.md`, `3d-view.md`,
  `checking.md` and `whats-new.md` describe the phase in four languages;
  F1 opens a topic from the Recognize Objects and Tilt View windows.
- ADRs 0122 to 0138; the two plans, the reports of the second part, the
  inventory, the spec and `CLAUDE.md` name each part of the phase.
- The pictures of the windows (636): the texts that do not fit are the
  titles of dock tabs, as in the audit of P17. The program is 83.3 MB.

**Exit tests.** The automated parts passed; the tests of the owner, of
testers and of users are open.

| Part | Milestone | Exit test | State |
| --- | --- | --- | --- |
| 1 | 1 | 90 % of the stitches of the corpus as DST in a part of their kind | Passed (97.1 %) |
| 1 | 2 | The width of a column within 0.2 mm and its spacing within 10 % | Passed (98.9 % of the stitches) |
| 1 | 3 | The angle of a fill within 5° and its row spacing within 10 % | Passed (99.9 % of the stitches) |
| 1 | 4 | A logo with lettering from a machine file sews at 150 % with the density of the first size | The automated part passed (0.89 to 1.14 of the density); the sew-out is open (owner) |
| 1 | 5 | Three Ink/Stitch files of testers open with their objects, kinds and colors; one sews | The test badge passed; the files of testers are open |
| 1 | 6 | A corpus design written for Ink/Stitch opens there with the same objects; a converted font sews at its sizes | The round trip passed (85 designs, 2,768 objects; Cats at three sizes); the test in Ink/Stitch and the sew-out are open (owner) |
| 1 | 7 | A corpus design as outlines and as DST becomes objects of the kinds of the source | Passed (92.5 % of 1,819 objects; each fill within 5°); the test of the owner is open |
| 1 | 8 | EPS files of three kinds open with their colors and shapes; a CDR file gives the message | Passed; the EPS and AI files of testers are open |
| 2 | 1 | The view bench in CI; a report with the numbers and a picture of each fault next to its photo | The bench runs in CI; the sew-out of the reference set and the small computer are open |
| 2 | 2 | Two thread weights show two widths; a matte thread has no highlight; linear light | Passed |
| 2 | 3 | The heights of a border, a satin with underlay and puff; 100,000 stitches in less than 1 s | Passed (0.17 s) |
| 2 | 4 | Eight satin columns have eight brightness values; each area within 3 % at each zoom; the compare with photos | The tests passed; the compare with photos is open (owner) |
| 2 | 5 | The budgets of pan, zoom, the first view and a change; no missing tile | Passed on the development computer; the small computer is open |
| 2 | 6 | Thin lettering on a towel with no topping is partly lost; each fabric at three zooms | The pictures are in the bench; the compare with photos is open (owner) |
| 2 | 7 | A reference design for each kind that is not thread, at two zooms | The pictures are in the bench; the compare with sew-outs is open (owner) |
| 2 | 8 | The owner checks a design with the low light, the tilt view and a check view; the tilt view at 20 frames in a second | 45 frames in a second; the check of the owner is open |
| 2 | 9 | Five test users tell a photo of a sew-out from a picture of the view; the budgets hold; the program starts with the software renderer | Open (test users; a computer with no graphics driver) |

## Audit of P17 (2026-11-19)

After Milestones 1, 2 and 4 to 8 of the phase (Milestone 3 is set aside by
the owner), a check of the code and the documents: a probe with wrong
values in font files and in letter corrections, a probe with text that is
hard to shape, the rules of the spec, the translations, the licenses, the
help and the numbers in the documents.

**Fixed.**

- **A font file with a wrong value ended the program.** A probe
  (`p17_probe.rs`) set each number of three small font files (a routed
  font, a script font and a fill font) in turn to wrong values (decimals
  0, −5, 10⁻⁹ and 10⁹; whole numbers 0, 100 000 and 10⁹), read the file,
  laid out and sewed a text, measured the font and made its test sheet,
  in a thread with a time limit. A font file had no check of its numbers
  at all. With a coordinate of 10⁹ mm in a letter, the probe asked for
  32 GB of memory and the test program ended (the step was not found
  out: the file is refused now). A font file can come from a font
  package of another person. Now `EmbFont::validate` checks a file when
  it is read: the capital height (1 mm to 1 m), the line height, the
  space, the size range (to 10 m, the smallest size not above the
  largest), each letter (`embfont::check_letter`: its width and each
  coordinate within 1 m; its parts are columns, fills and runs only; its
  settings), the kerning and the quality. A corrected letter of a
  corrections file has the same check. After the change: 978 cases, 256
  refused as wrong files, no crash, no hang and none slower than 3 s;
  98 cases of a corrections file, none with a problem.
- **The default motif and the default pattern made all items of the
  families.** After Milestone 8, `Motif::default` made the 71 built-in
  motifs and `SplitPattern::default` the 48 patterns to take the first
  one. The stitch engine takes the default for each motif column and
  each program-split column with no item of its own. Now the defaults
  are the first items that are written by hand, with no family.
- **The numbers in the help can go wrong with no notice.** The help says
  how many fonts, motifs, patterns, stamps, borders and clipart items
  the program has, in four languages. A new test compares these numbers
  with the program (`the_numbers_in_the_help_are_the_numbers_of_the_program`).
- ADR 0113 said that the script of the proof pictures is not in the
  repository; it is `tools/font-proof.py`. The heading of P17 in this
  roadmap named five milestones as done.

**Left as it is.**

- The Mitre Corners setting does nothing for the embroidery fonts of the
  program and of the packages (a routed letter is fixed, ADR 0116).
  Lane 6 of the review changed this (ADR 0121).
- Twelve fonts of the packages have a score of 85 or more, and fifteen
  fonts of the program have less (the lowest: 71). The owner left the
  fonts where they are.
- The score of a fill font says little: the ten fill fonts have 99 and
  100. The size range is what the measure gives (ADR 0117).
- The score measures the 62 letters and digits. A check of all 199
  characters of six fonts found at most one character of a font with
  less than 90 % of its outline; of the 1045 marks of the 55 fonts with
  satin letters, 12 have less than 90 %. A letter with an accent has a
  trim for the accent.
- The names of the motifs, patterns, stamps, borders, clipart items and
  families are in English only. The lists of the patterns (48), stamps
  (39) and borders (50) are names with no pictures.
- The Families window has no ribbon command: the Motifs window, Object
  Properties and the Clipart panel open it.
- The pictures of the windows show the titles of some dock tabs cut
  ("Clipart", "Información del diseño"; 115 texts in the report, 31 of
  them only in a translation), and the View Options window is wider in
  German and in Spanish than in English.
- The Motifs window reads the motifs folder at each frame while it is
  open (as before the phase).
- A join under the strokes that sew later can be one stitch of up to
  9.8 mm (ADR 0116). Heavy serif fonts and the "m" of Brush Hand are as
  the plan says.
- The update check needs the `curl` program of the system (ADR 0109).

**Checked, no change.**

- Rules 3 and 9: no names of closed-source programs in the changes of
  the phase; no `unsafe` code (the one crate with `unsafe` is
  `emb-thumbnail`, as before); the new dependencies are `harfrust` and
  `unicode-bidi` (pure Rust, ADR 0114), and the list of licenses has
  them; the trig functions of the engine code go through
  `emb_geom::math` (the one other call is in a test).
- Text that is hard to shape (Milestone 6): 36 cases (nine texts in four
  bundled fonts: a line of 6 400 characters, 800 lines, direction
  marks, 3 000 marks on one letter, Arabic with marks, Devanagari
  conjuncts, joiners, characters that no font has, mixed scripts). No
  crash, no hang; all cases and the corrections in less than 6 s.
- The test sheet of a fill font (a range to 300 mm): the middle and the
  largest size are on at the start only if the sheet fits the hoop.
- Translations: the 4109 texts of the template are in German, Spanish
  and Japanese, none missing and none extra. The help topics
  (`options.md`, `lettering.md`, `tutorial-lettering.md`,
  `stitch-types.md`, `whats-new.md`) and their translations describe
  each milestone; F1 opens a topic from the Fonts, Alphabets, Font Test
  Sheet, Families, Borders and Check for Updates windows.
- ADRs 0109 to 0118; the plan, the inventory, the spec and `CLAUDE.md`
  name each part of the phase.
- Licenses: the 65 fonts of the program have 65 license texts next to
  them; each font of a package has its license text in the package (a
  test checks it).
- Files of this version in an older program: the new fields of a font
  file and of a project (the quality, the fill mark, fixed letters) have
  default values, and the types do not refuse fields that they do not
  know. No test with the program of version 0.3.0 was made.
- The program is 72.3 MB (63.4 MB before Milestone 7).

**Exit tests.** The software parts passed; the sew-outs are open.

| Milestone | Exit test | State |
| --- | --- | --- |
| 1 | A build with an older version number shows the notice; with automatic checks off, the program makes no request | Passed in tests; a check with a real release is open (owner) |
| 2 | The pictures of all windows in four languages in less than five minutes; a text that does not fit is found and corrected | Passed (588 pictures in 100 s) |
| 3 | The kit sews correctly on the Tajima; the cap design on the cap frame; no open problem in the reports of the testers | Open (set aside by the owner) |
| 4 | A corrected letter stays corrected after the fonts are made again; the owner sews the test sheet of one font | The test passes; the sew-out is open (owner) |
| 5 | The test sheet of each of the ten fonts sews correctly on the Topaz 20 and on the Tajima, at the three sizes | Open (owner). The fonts were made again after the milestone (ADR 0116), so the sheets must be made again too |
| 6 | A test text in Arabic, in Devanagari and in a script font gives the glyphs of a reference; the owner sews one word of each | The test passes; the sew-out is open (owner) |
| 7 | Each bundled font has a score above the limit; the program grows by less than 10 MB | Passed (the lowest score is 71; 8.9 MB more with the fill fonts and Milestone 8) |
| 8 | A sew-out of five new items of each kind | Open (owner) |

## Review of P17 (2026-11-19): shortcomings and lanes

A look at the phase as a whole, after its audit. These are proposals;
the owner decides which become work.

**Shortcomings.**

1. **Nothing of the phase is sewn.** The scores, the routes, the fill
   fonts and the new content are checked on pictures and with counts.
   The limits (65 and 85, the 2 mm of a fill, the 6 mm of a tile row)
   come from pictures. One sew-out can change them: travel runs under a
   light thread, the seam of a cut column, a hidden join of 9.8 mm, the
   edge of a fill letter.
2. **The score does not see what the eye sees.** It measures the part of
   the shape that has stitches, columns that are too narrow, and
   stitches that are too long. It does not measure trims, stitches
   outside the shape, places with two layers, the turn of the stitches
   at a bend, the marks or the letters with accents. A fill font always
   has a high score.
3. **A font is all columns or all fills.** A heavy font has some letters
   that are good as columns and some that are not. The converter does
   not choose for each letter.
4. **Routed letters lost a setting.** Mitre Corners does nothing for all
   fonts of the program.
5. **The new content is hard to choose.** 48 patterns, 39 stamps and 50
   borders are lists of English names with no pictures. The clipart
   meets its number with simple shapes.
6. **The converter of the user is weaker than the font tool.** A font
   that the user converts in the program is not extended and not routed
   (the tool takes 5 to 30 s for a font).
7. **Text shaping is for TrueType fonts only.** Arabic and Devanagari
   text has letters that the program makes when the text is typed; they
   have no route and no score.
8. **Development tools are not complete.** The contact sheet of the
   items of the families was drawn by a script that is not in the
   repository. The pictures of the windows (588) are looked at as a
   sample; nothing shows which pictures changed since the last run.
9. **Milestone 3** (an industrial machine, reports of professional
   testers) is the oldest open part, and the fonts are not tested on
   such a machine.

**The owner took the lanes in this order (2026-11-19):** 1 (the kit), 3
(pictures in the lists), 4 (the route in the converter), 2 (development
tools), then 5 to 8. Lane 5 and lane 6 were to wait for a sew-out; the
owner started lane 5 on pictures alone (2026-11-19). After lane 6 the
owner shelved lanes 7 and 8 and started the 3D stitch view (the second
part of P18, 2026-11-19).

**Lanes, the cheapest first.**

1. **A sew-out kit of the phase** (S): one folder with the test sheet of
   a tuned font at three sizes, a routed font that is not tuned, a fill
   font at 15, 40 and 100 mm, one word in Arabic and in Devanagari, and
   the design of the content families. The owner can then do the open
   exit tests in one session. **Done (2026-11-19):**
   `tools/make-sewout-kit.py` makes `out/p17-sewout` (40 designs: VP3 and
   DST files, a worksheet and a picture of each one, and a checklist with
   the questions of the review). Two corpus designs came with it:
   `shaped-text` (the three words of the exit test of Milestone 6) and
   `content-clipart` (five clipart items, each in a border), so the kit
   has five new items of each kind for Milestone 8.
2. **Development tools** (S): a contact sheet of the items of the
   families from the repository (`emb-cli` writes the items, a script in
   `tools` draws them); a list of the window pictures that changed
   since the last run. **Done (2026-11-19):** `emb-cli content` writes
   the built-in items as data and `tools/content-sheet.py` draws one
   sheet for each kind (motifs, patterns, stamps, borders, clipart).
   The test of the window pictures keeps a hash of each picture
   (`hashes.txt`) and writes `changed.txt`: the pictures that are new
   and the pictures that changed since the run before. Two runs of the
   same program give no changed picture.
3. **Pictures in the lists** (M): the patterns, the stamps and the
   borders as picture grids, as the motifs have, with a search. The
   Families window draws them already. **Done (2026-11-19):**
   `emb-app/src/item_pictures.rs` draws a pattern as its needle points,
   a stamp as its lines and a border as its stitches round a small box.
   The lists of the patterns and of the stamps in Object Properties and
   the Borders window show these pictures with the names; the Borders
   window has a search. The lists of Object Properties open as menus,
   so they have no search.
4. **The route in the converter of the program** (S to M): Convert runs
   the extension and the route as a background job with a progress
   bar, so a font of the user is as good as a font of the tool. **Done
   (2026-11-19):** Lettering > Fonts > Convert has the setting **Few
   trims** (on at the start, not for fill letters): the ends of the
   columns, the route and the quality score, as `emb-cli font --extend
   --route` makes them (`fonts::convert_font`). The window shows the
   step and, for the route, a bar; **Stop** ends the conversion with no
   font. A conversion with the route takes 3 to 16 s for the three
   bundled TrueType fonts on the development computer (32 threads); a
   computer with few threads needs longer.
5. **The best of columns and fill for each letter** (M): the converter
   makes both for each letter and keeps the one with the better letter
   score. The score gets the measures that the audit of the fonts has
   already (stitches outside the shape, two layers, trims). **Done
   (2026-11-19, ADR 0119), on pictures only.** The score has two new
   measures: the area of a letter outside its outline and the area with
   two layers (not the trims: only the stitch engine can count them).
   The mean score of the fonts is 81.6 (82.3 before); no font falls
   below 65, and no letter of a font of the program changed. The
   converter (the font tool, and Convert with the stitch type Automatic)
   keeps the fill letter of a character if the columns leave a part of
   the shape open and the font is for large letters (from 15 mm): 32
   letters of five package fonts are fills now. **What the lane did not
   reach:** the heavy fonts, which have most of the poor letters, have
   size ranges from 5 to 9 mm, where a fill does not sew well. See lane
   5b.
   - **5b. Fill letters from a size** (M to L): each font with
     satin letters keeps the fill letter of each character too, and the
     layout takes the fills from the size at which the columns are too
     wide. A word is then all satin or all fill, each font goes up to
     300 mm, and the ten fill fonts are needed only for fills at small
     sizes. It changes the font format and the layout. **Done
     (2026-11-19, ADR 0120), on pictures only:** 123 of the 137 fonts
     have fill letters (not the fill fonts and the fonts for small
     text). The change to fills is at the end of the size range that a
     font had (25 to 206 mm), so lettering in that range sews as before;
     the columns and the scores of all fonts are the same. Stitch type
     in the Lettering window selects Satin or Fill for the whole text of
     such a font. The font files of the program grew from 42 MB to
     65 MB. The sew-out kit has three designs for it (`large/`).
6. **Mitre corners in routed letters** (M): the route joins two columns
   that meet at a corner when the font is made, or the layout does it
   for two parts that sew one after the other. **Done (2026-11-19, ADR 0121), on
   pictures only:** the layout joins two columns of a routed letter that
   sew one after the other and meet at a corner (`mitre::join_in_order`),
   so the route stays and no letter gets a new trim. With the setting on,
   a routed font has about as many mitre corners as a font with no route
   (Varsity 24 of 25, Anton 9 of 9, Block Sans 5 of 9). The check box is
   there for each font again. The corners where the route sews something
   between the two strokes stay a stop and a start.
7. **Embroidery fonts for Arabic and Devanagari** (L): converted fonts
   with a letter for each glyph of the font, routed and with a score.
   **Shelved by the owner (2026-11-19).**
8. **Clipart sets** (L): pictures with more detail by theme, as asset
   packages. They need drawing by hand or a source with a free license.
   **Shelved by the owner (2026-11-19).**

## Audit of P16 (2026-11-18)

After Milestones 1 to 8 of the product view, a check of the code and the
documents of the phase: a probe with wrong values in the product of a
project file, a review of the code, the rules of the spec, the
translations, the licenses and the help.

**Fixed.**

- **Wrong values in the product of a project made pictures of
  gigabytes.** A probe (`product_probe.rs`) set each number of the
  product of two projects in turn to wrong values (decimals 0, −5, 10⁻⁹
  and 10⁹; whole numbers 0, 100 000 and 10⁹), opened the project and
  made the product pictures, the approval sheet pictures and the
  turntable body in a thread with a time limit. The project check did
  not look at the product at all. A photo width of 10⁹ mm made a
  picture of 268 million pixels (1 GB); a mask of 10⁹ rows took more
  than 20 s and a mask of 10⁹ columns 15.6 s (a grid of a billion
  cells). Now `project::validate` checks the product: its place, the
  real width of a photo (1 mm to 10 m), the mask (at most 1024 cells on
  a side), the places on the photo, and the place and the contents of
  each other design. After the change: 252 cases, 64 refused as wrong
  files, no crash, no hang and no picture of more than 60 million
  pixels.
- **A design far from the product made a huge picture.** The picture of
  the design on the product took in all the stitches, so another design
  placed far away (allowed up to 1 km) made the picture as large as the
  pixel limit (16 384 pixels a side, 1 GB). Now stitches more than 1 m
  from the product do not make the picture larger
  (`product::decoration_area`), and the sides of the turntable show the
  product only (`product::turn_area`).
- **The turntable made the stitch plan at each step of a drag.** With the
  Turntable window open, each change of the design (each frame of a drag)
  made the exact stitch plan on the window thread and started a new job.
  Now the views are made again when the design stays the same for
  0.3 s; the old views show until then.
- The back of the hoodie showed the hood flat on the back, over the place
  of a back print (owner, 2026-11-18); the hood now stands as at the
  front, and the back has the usual places.

**Left as it is.**

- At an angle, the cloth at the far edge of a part of the turntable body
  is stretched (the soft body has a steep side); a beanie shows it most
  (ADR 0108).
- The turntable shows in its own window; the canvas stays flat, as for
  the curve of a cap (ADR 0105, 0108).
- A photo of a product cannot turn, and a photo of a dark product does
  not take a light color well (ADR 0107).
- A color drag on a masked photo makes the canvas picture again (25 ms
  for a photo of 2048 × 1536 pixels, then the texture).
- **Save in My Products** replaces a template of the same name with no
  question.

**Checked, no change.**

- Rules 3 and 9: the products are drawn in code and the photos are the
  user's; no names of closed-source programs in the changes of the phase;
  no `unsafe` code, no new dependency (`Cargo.lock` did not change); the
  trig functions of the engine code go through `emb_geom::math`.
- Translations: the 3906 texts of the template are in German, Spanish
  and Japanese, none missing and none extra. The help topic
  `checking.md` and its three translations describe each milestone, and
  F1 opens it from the Product, Product Photo and Turntable windows.
- ADRs 0101 to 0108; the plan, the inventory, the spec and `CLAUDE.md`
  name each part of the phase.
- Two-tone products add at most 3 ms to the canvas picture of a product;
  the 24 views show one by one as the background job makes them.

**Exit tests that are open.** The owner checks them at the end of the
phase; the software parts passed (each milestone has its tests, and the
pictures of `write_products`, `write_photo_colors` and
`write_turntables` were checked).

| Milestone | Exit test | State |
| --- | --- | --- |
| 1 | The products look like cloth in light and dark colors; a color drag on the canvas stays smooth | Open (owner) |
| 2 | A polo with a left chest logo and a sleeve name, saved as one picture | Open (owner) |
| 3 | A T-shirt with a chest logo and a back print in one picture | Open (owner) |
| 4 | An approval PDF with the product page opens in a PDF reader and prints on A4 and Letter | Open (owner; the PDF structure test passes) |
| 5 | Each product looks right; a test design at each placement is on the product | Open (owner; the placement test passes for the 17 sides) |
| 6 | A navy cap with a white front and a gray polo with a black collar | Open (owner; the pictures were made) |
| 7 | A photo of a real shirt of the owner, saved as a template, shows a design at its left chest in three colors | Open (owner) |
| 8 | A T-shirt with a chest logo and a back print turns in steps on a PC with the software renderer and saves as a GIF | Open (owner) |

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
  of the bench or of the owner comes near this. *Done on 2026-10-29 (ADR
  0081): the jobs have stop points and the wizard has a Stop button.*
- Remove the edge color (0.5 s on a large picture) and cut out the subject
  (0.2 s) still run on the window thread. *Done on 2026-10-29 (ADR 0081):
  they run on a thread.*
- The wizard keeps the thread chart colors from when "Colors from the
  thread chart" was ticked; after a change of the chart in the Threads
  panel, tick it again. *Done on 2026-10-29: the wizard follows the chart
  (`Wizard::follow_chart`).*
- A photo gives a trim for nearly each object (the owner's four photos: 61
  to 271 objects), because its areas of one color are apart. Travel
  between them under later colors is not done. *Done on 2026-10-29 (ADR
  0082): hidden travel, 19 to 42 % fewer trims on these photos.*
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
| P11 | Thread use within 15 % of weighed spools; 4 mm and 6 mm small-text lettering; thumbnails after an install on three systems; a sew-out of each new stitch type; a lace ornament and edge washed; a 300 × 300 mm quilt block in four hoopings; puff letters | The sew-outs on the Topaz 20 passed (owner, 2026-11-16). Open: the thread use against weighed spools, and the thumbnails after an install on macOS and Linux |
| P11 | Cap order on a cap frame; cutwork files on a cutwork machine | Open: wait for a tester with such a machine |
| P12 | A phone photo upright and the window responsive at the finest detail; a logo with no gaps and a drawing with outlines sewn on the Topaz 20 | Passed (owner, 2026-11-16) |

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
  *Done on 2026-10-29: `ribbon/` is a folder module of six files.*
- The installer, the macOS and the Linux packages have not run: the
  repository has no GitHub remote yet (ADR 0043).

**Open: needs the owner or test users.** The table of exit tests in the
"Audit of 2026-10-23" (P2, P3, P5, P6, P7, P8 part 2), the sew-outs of
P9, and for P10: the 15-minute patch test with users of Windows, macOS
and Linux, a new user with F1 and the help search (Milestone 4), a name
on a towel in the kiosk (Milestone 5), a screen reader (Milestone 7), a
review of the German, Spanish and Japanese texts by a native speaker, and
the first release (push the repository to GitHub, then a tag `v0.1.0`).
Also open: the comparison with Ink/Stitch (P1). *Closed on 2026-11-18 (owner; "Review of 2026-11-18").*

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
| P5 | Sew-outs of the nine P5 corpus designs (kit and checklist in `out/p5-sewout`) | Passed (owner, Topaz 20, 2026-11-16) |
| P6 | A two-color logo PNG becomes a sewable design with less than 10 minutes of touch-up; a photo becomes a recognizable stitch portrait; sew-outs of the P6 corpus designs (kit and checklist in `out/p6-sewout`) | Passed (owner, Topaz 20, 2026-11-16) |
| P7 | A 360 × 260 mm design split for the 260 × 200 hoop sews with aligned joins (kit and checklist in `out/p7-sewout`, made again for the upright hoop) | Passed (owner, Topaz 20, 2026-11-16) |
| P8 | A 1000-design library is searchable in under a second; a design goes to a USB stick for the Topaz 20 in one action | Part 1 passed (about 1 ms, `crates/emb-library/tests/thousand.rs`). Part 2 passed (owner, 2026-11-16): File > Send to USB to a stick, then the Topaz 20 reads it. Open (Milestone 7): a phone app reads the barcode of a printed worksheet, and a cutter program opens a cutter file at real size. |

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

### P5. Decorative stitches and effects (XL) — done (sew-outs passed, 2026-11-16) (`docs/design/p5-plan.md`)

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
sew-out (the photos stay with the owner; they are not in the repository).

### P6. Artwork, vector tools and auto-digitizing (XL) — done (exit tests passed, 2026-11-16) (`docs/design/p6-plan.md`)

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

### P7. Hooping, layouts and presentation (L) — done (exit test passed, 2026-11-16) (`docs/design/p7-plan.md`)

- Hoop templates, hoop placement settings, auto hoop, multi-position hoops.
- Multi-hooping: auto layout, split lines, overlap, registration marks, preview and
  calculate hoopings, per-hooping output.
- Product backdrops and product visualizer; virtual decoration image.
- Design repeats (continuous and overlapping designs).

**Exit:** a 360 × 260 mm design split for the 260 × 200 hoop sews with aligned joins.

### P8. Management and connectivity (L) — done (exit test part 2 passed, 2026-11-16) (`docs/design/p8-plan.md`)

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

### P11. Features of other programs (XL) — done in software, sew-outs passed (2026-11-16), tests with other machines and systems open (`docs/design/p11-plan.md`)

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

### P12. Picture to design: robust and better results (L) — done (exit tests passed, 2026-11-16) (`docs/design/p12-plan.md`)

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

### P14. Sketch mode (XL) — done (`docs/design/p14-plan.md`)

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

### P15. Browser version (L) — later, not started

The program in a web browser: one link for computers, Chromebooks and
tablets, with no install and no warning of an unknown publisher (owner
question, 2026-11-16). The main reason is more testers and users: a link
is easier to try than an installer. A version for touch is a separate
decision, after users ask for it.

**When to start.** After the first feedback of users on the desktop
program is fixed (about v0.4), so that one version gets the fixes first.
Code signing (ADR 0062) does not wait for this phase.

**What we know (2026-11-16).**

- The engine builds for the browser (`wasm32-unknown-unknown`): 7 of the
  10 engine crates build with no change; `emb-formats`, `emb-library`
  and `emb-script` need the `wasm_js` setting of `getrandom` (through
  `lopdf` and `rand`).
- eframe 0.36 runs in the browser (WebGPU, else WebGL2), so the canvas,
  the ribbon and the windows stay the same code.
- `emb-app` uses the desktop: 37 file dialogs (`rfd`), about 300 calls of
  `std::fs`, 9 places that start threads, `sysinfo` (Send to USB),
  `std::process` (open with another program, email, the start fallbacks),
  and 26 MB of `data` (mostly fonts).

**Steps.**

1. **Trial build (S).** `emb-app` for the browser with the desktop parts
   off (`cfg(target_arch = "wasm32")`): open is an upload, save is a
   download, the data is in the page. A test page; the owner tries it in
   a browser and on a tablet. Decide on step 2 from the result (speed,
   memory, how it feels).
2. **Web version for computers (M).**
   - One storage layer for all file access: files of the user in and out
     through upload and download; settings, backups, My Hoops and My
     Machines in the storage of the browser.
   - Long jobs (stitch plan, Picture to design, font samples) off the
     screen thread: web workers, or short jobs that give the screen time.
     Threads in WebAssembly need cross-origin isolation, which GitHub
     Pages does not set (a service worker can do it).
   - Fonts load when a design needs them; the user adds a TrueType font
     by upload (no fonts of the system).
   - Off in the browser: Send to USB, thumbnails, open with another
     program, email, the installer and the renderer fallbacks, the
     library of folders (a library in the storage of the browser can come
     later).
   - CI builds the web version, and the release workflow publishes it to
     GitHub Pages. The README gets "Try it in your browser".
   - An ADR: the targets, the storage layer, what is off on the web.
3. **Tablets (L), only if users ask.** Touch: pinch zoom, two-finger pan,
   long press for menus, larger targets (from simple mode); an install
   for offline use ("Add to Home Screen"); tests on an iPad (memory of
   Safari) and an Android tablet. The tasks of a tablet first: lettering,
   placement, colors and output.

**Not in this phase.** Apps in the App Store or Google Play: the GPL and
the terms of the App Store do not agree (and the code ported from
Ink/Stitch cannot get a new license), and both stores (and Android
installs from outside the store, from 2026 to 2027) want the legal name
of the developer.

**Cost after the phase.** Each new feature works in both versions; code
that uses files or threads needs a path for the browser.

**Exit:** the web version opens, edits and outputs the corpus designs in
Chrome, Edge, Firefox and Safari on a computer; a design made in the
browser sews on the Topaz 20; the page works offline after the first
visit. With step 3: a name on a towel is made on a tablet from start to
machine file.

### P16. Product view (M) — done in software, exit tests open (audit: "Audit of P16") (`docs/design/product-plan.md`)

The product under the design (P7 Milestone 5) good enough to show to a
customer (owner, 2026-11-17), in eight milestones: a realistic look
(light, folds, fabric, the shadow of the embroidery), several placements,
front and back, the product on the approval sheet, better products (the
cap again, new garments), two-tone products and color swatches, masks
and templates for the user's own product photos, and a turntable view
(the product turns in steps; owner, 2026-11-18).

**Exit:** the exit tests of the eight milestones in the plan.

### P17. Polish and lettering (L) — done in software but Milestone 3 (set aside), exit tests open (audit: "Audit of P17") (`docs/design/p17-plan.md`)

The features that the program has become better before it gets new ones
(owner, 2026-11-18), in eight milestones: an update check that the user
can turn off at the install, pictures of all windows in each language
(development only), tests on an industrial machine with the reports of
professional testers, a font test sheet and letter corrections that stay,
ten tuned fonts, text shaping, more fonts with a quality score, and
larger libraries of motifs, patterns, stamps, borders and clipart.

**Exit:** the exit tests of the eight milestones in the plan.

### P18. Designs from other programs, and the 3D stitch view (XL) — done in software, exit tests open (audit: "Audit of P18") (`docs/design/p18-plan.md`, `docs/design/p18-view-plan.md`)

A machine file of another program becomes objects with stitch types
(runs, satin columns, fills) that scale with their density; the program
reads and writes Ink/Stitch files and reads its fonts; outlines with a
stitch file become objects; the program reads EPS files with its own
PostScript interpreter, and a CDR file gives a message (the owner changed
Milestone 8 on 2026-11-20: no helper program). The native formats of
closed-source programs stay out of scope. All eight milestones are done
in software (ADR 0131 to ADR 0138); the tests of the owner and the files
of testers are open.

**Exit:** the exit tests of the eight milestones in the plan.

**Second part: the 3D stitch view** (owner request, 2026-11-19; L to
XL). The 3D thread look of P10 (ADR 0040) lights each stitch alone, as a
cord, and the canvas makes a mesh of all stitches again for each frame.
It does not look like a sew-out: no height, one sheen for satin and fill,
one kind of thread, a flat fabric, thin lines at a small zoom. The
upgrade makes a surface of the embroidery in tiles (height, direction of
the thread, color, material) and lights it: sheen by the direction of
the thread, shadows from the heights, threads with a kind and a
thickness, fabrics with a surface, a canvas whose speed does not change
with the number of stitches, a view from an angle with a light that the
user moves, and the same engine for all pictures of the program. A
picture needs no graphics card, as before. The owner started this part
first (2026-11-19). Milestone 1 is done except the sew-out of the
reference set and the numbers of a small computer: twelve reference
designs, `emb-cli view-bench` in CI, a picture compare, and
`ShokujoDX --view-bench` for the canvas (ADR 0122). The bench found that
very large designs stopped the program in the 3D thread look; the canvas
now uses a lighter mesh for such a frame. Milestone 2 is done: colors mix
in linear light in the pictures, a thread has a kind and a weight (the
Threads panel, the charts), and the views show its width and its shine
(ADR 0123). Milestone 3 is done: the stitches are laid with their heights
and the surface is made in tiles on all processors; the bench shows its
first light next to the look of today (ADR 0124). Milestone 4 is done
except the compare with the photos: sheen by the direction of the thread,
shadows, gaps, plies and zoom levels that keep the light (ADR 0125).
Milestone 5 is done: the canvas and all views show the surface on tiles
that threads make in the background, and a change makes again only the
tiles that it reaches (ADR 0126). A speed pass after it shows drafts for a
first view and quick pictures for a change of the light, so the budgets of
the development computer are met (a change of one object at its budget).
Milestone 6 is done except the compare with the photos: the fabric is a
surface (eight weaves at their real size, in the light of the view), each
needle point is a dent, thin stitches sink into a towel or fleece with no
topping, a fabric can have two colors, and the product view shows the
same fabric (ADR 0127). Milestone 7 is done except the compare with the
sew-outs: appliqué fabric with its weave and a cut edge, rhinestones with
facets, the holes of cutwork, lace with no fabric, trapunto padding and
the batting of a quilt, cross stitch on Aida, round puff, glints of
metallic thread, and a look with the lights off for glow thread (ADR 0128).
Milestone 8 is done except the check of the owner: light presets and a
light that moves with L and a drag, a tilt view (on the canvas or in a
window) at 45 frames in a second for 100,000 stitches, a magnifier,
Compare 3D and Before and After, the check views Fabric Shows Through and
High Thread, and the product and the turntable with the surface of the
stitches in the light of the product (ADR 0129). Milestone 9 is done
except the test with users: all pictures of the program from the
surface, a design picture up to 600 dots for each inch with no
background and a sew-out animation, the quality Fast, Good or Best from a
measure at the first start, no graphics card for now, the help topic The
3D view, and an audit (a lay of a design 1 km wide, the memory of ten
open designs) (ADR 0130).

**Exit:** the exit tests of the nine milestones in
`docs/design/p18-view-plan.md`; the last one is a test with users: a
photo of a sew-out next to a picture of the view.

### P19. Help and tutorials (L) — proposed (`docs/design/p19-plan.md`)

The help becomes a part of the work: pictures made from the program,
Show Me on the ribbon, guided lessons that check each step, a doctor for
sew-out problems, samples, ways to start for different users, the help
on the web and as a manual, moving pictures, better answers.

**Exit:** the exit tests of the milestones that the owner selects.

### P20. Japanese lettering (L) — accepted, Milestones 1, 2, 4 and 5 done, Milestones 3 and 6 wait for their sew-out (`docs/design/p20-plan.md`)

Hiragana, katakana and kanji sew as a digitizer makes them: the stitch is
chosen for each column (satin, satin with split stitches, or a tatami
column along the stroke), not for the whole character; where strokes
cross, one column goes through and the other stops at its sides; the
letters of the fonts of the computer get routes with few trims; the
height of Japanese text is the height of its characters, and thin strokes
sew as runs; a Japanese embroidery font in a font package. A Japanese
reference set and its measure come first.

**Exit:** with a Japanese test font, no character of the reference set
is all tatami from 15 mm to 80 mm, two layers on at most 3 % of each
character, half the trims of today; a font package with the kana and the
2,136 jōyō kanji; test sheets sew well on the owner's machine.

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
P1–P14 + user feedback (about v0.4) ─► P15 browser version (later)
P7 ─► P16 product view
P4 + P11 + tester reports ─► P17 polish and lettering
P6 + P17 ─► P18 designs from other programs (recognize outlines, vector import)
P10 + P11 + P16 ─► P18 second part: the 3D stitch view (thread look, density map, product view)
P10 + P13 + P17 ─► P19 help and tutorials (help, scripts, pictures of the windows)
P4 + P17 ─► P20 Japanese lettering (stroke finder, routes, quality score, font packages)
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
