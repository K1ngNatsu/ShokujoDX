# ShokujoDX: Open-Source Embroidery Digitizer — Project Spec

Oct 21, 2026 · @Zero

## Purpose and scope

The project builds a free, standalone, open-source embroidery digitizer. The feature set and professional workflow of a leading commercial embroidery suite (the reference product) are the reference (owner decision, 2026-10-21). Consumer embroidery products stay the reference for ease of use for new users. Claude Code uses this document as the project specification.

- Feature and UI inventory: `docs/design/feature-inventory.md`.
- Roadmap to parity: `docs/design/roadmap.md`.

**Problem.** Current free tools are not intuitive. Ink/Stitch has a good stitch engine, but it operates as an Inkscape extension, so the user must work in a general vector editor. Commercial embroidery suites are intuitive and complete, but they are expensive.

**Target user.** A home or small-shop user with a single-needle machine. The first test machine is a Husqvarna Viking Topaz 20.

**MVP includes:** manual digitizing of satin, fill and running stitch objects; automatic underlay and pull compensation; stitch order and color sequence control; a 2D thread preview; hoop presets and an open-field mode; export to VP3, PES and DST.

**MVP does not include:** auto-digitizing from images, 3D thread preview, lettering with keyboard fonts, multi-hoop splitting, or direct machine connection. These items are in later phases.

## Key decisions

| Decision | Choice | Reason |
| --- | --- | --- |
| Method | Behavioral reimplementation. Do not decompile, disassemble or copy code or assets from closed-source embroidery products. | The EULAs of closed-source embroidery products prohibit reverse engineering. The digitizing methods are public knowledge, so decompilation gives little value. |
| Reference for behavior | Public product documentation of the reference product (reference manual, quick reference, release notes, product comparison), public help of consumer embroidery products, the UIs as a user sees them, and stitch files that the project team digitizes. Describe behavior in our own words; do not copy text, icons or shortcut tables. | These sources are legal and sufficient. |
| Engine source | Port the stitch algorithms of Ink/Stitch to Rust. Use pyembroidery as the reference for file formats. | Ink/Stitch is mature and tested. The weakness is its UI, not its engine. |
| License | GPL-3.0-or-later. | Ink/Stitch is GPL, so a port must also be GPL. pyembroidery is MIT, which is compatible. |
| Language | Rust for the engine and the app. | Memory safety, speed for stitch generation and preview, and one language for all layers. |
| Platforms | Windows first, then macOS and Linux. | Most home embroidery users use Windows. |
| UI framework | egui (with `eframe` and `egui_dock`). See ADR 0001. | The canvas must draw 100k+ stitches at interactive speed with no IPC. One language for all layers. |
| Geometry library | `geo` with its `i_overlay` back end (pure Rust). Do not use Clipper2 bindings. See ADR 0002. | Clipper2 bindings call C++ through FFI, which needs `unsafe`. |
| Undo and redo | Snapshot history with structural sharing (`Arc` per object). See ADR 0004. | Every operation becomes undoable with no inverse code to write or test. |

Record the source of each ported algorithm in a comment at the top of the Rust file: the Ink/Stitch file path and the commit hash.

## Target hardware and hoops

The reference machine is a Husqvarna Viking Topaz 20. Its maximum embroidery field is 260 mm × 200 mm (DESIGNER Crown Hoop; the machine also ships with a 120 mm × 120 mm Splendid Square Hoop). Husqvarna Viking gives the front-to-back size first: the field is 200 mm from left to right and 260 mm from the front to the back, and the machine shows designs upright in it (owner test, ADR 0034). Its native file format is VP3 (confirmed: the P4 sew-outs sewed from VP3 files).

**Machine profiles.** The app keeps machine profiles in a data file (`data/machines.toml`). Each profile has these fields: `id`, `name`, `brand` (optional), `default_format`, `formats` (list), `hoops` (list of hoop preset ids that fit the machine), `max_stitch_length_mm`, `min_stitch_length_mm`, and `stitches_per_minute` (for the estimated sew time). The export dialog takes its default format from the selected profile. A hoop that is not in the profile list shows a warning. Besides the Topaz 20 and a generic profile, there is one profile for each of these multi-needle brands: Tajima, Happy, Toyota, Ricoma, BAI, Highland (ADR 0014) and Barudan (U01, ADR 0037).

**Hoop model.** Each hoop preset stores the sewable field, not the outer frame size. The app keeps all presets in a data file (`hoops.toml`), not in code. Users can add, edit and share presets without a rebuild.

Each preset has these fields: `id`, `name`, `brand` (optional), `width_mm`, `height_mm`, `shape` (rectangle, rounded rectangle, oval, circle), `corner_radius_mm` (optional), `max_stitch_length_mm` (optional machine limit), and `position` (optional list for a multi-position hoop: `x_mm`, `y_mm`, `width_mm`, `height_mm` of the field of each position; ADR 0032).

**Open-field mode.** The user can select "Open field". In this mode, the canvas has no boundary. The app shows the current design extents and a list of presets that the design fits. Export shows a warning if the design is larger than the selected machine limit. In open-field mode, the design origin is the canvas origin. When the user selects a hoop, the hoop center goes to the center of the design extents. This changes no object coordinates.

**Default presets.** The sizes below are common industry sizes. They are approximate. Each brand has small differences, so verify each size against the manufacturer data before release.

| Preset | Field (mm) | Common name |
| --- | --- | --- |
| Topaz 20 max | 200 × 260 (upright) | Reference machine (DESIGNER Crown Hoop "260 x 200") |
| Topaz 20 square | 120 × 120 | Reference machine (Splendid Square Hoop) |
| Small | 100 × 100 | 4 × 4 in |
| Medium | 130 × 180 | 5 × 7 in |
| Square large | 200 × 200 | 8 × 8 in |
| Long | 160 × 260 | 6 × 10 in |
| Large | 200 × 300 | 8 × 12 in |
| Extra large | 360 × 200 | 14 × 8 in class (does not fit the Topaz 20) |

**Rules for the hoop boundary:**

1. Show the sewable field as a solid outline on the canvas.
2. Show a safety margin (default 5 mm, configurable) as a dashed line inside the field.
3. Show a warning on each object that goes outside the safety margin.
4. Do not let export continue when a stitch is outside the field. Show the location of the first out-of-field stitch.
5. Let the user rotate the design 90° to fit the hoop. This is a design-level rotation (`Design::rotation`, 0/90/180/270). The engine applies it when it builds the stitch plan, so object geometry and stitch angles do not change. The hoop can also turn by any angle on the canvas (`HoopPlacement::angle_deg`); the plan turns the stitches back, so the hoop is straight in the machine file (ADR 0032).
6. Preset width is the size from left to right on the machine, height the size from the front to the back. The canvas marks the back edge of the hoop, so the user sees where the top of the design goes on the machine (ADR 0034).

## Architecture

The project is one Cargo workspace with separate crates. The engine crates must not depend on any UI crate. This lets other tools use the engine as a library.

```text
embroidery/
├── crates/
│   ├── emb-geom      # 2D geometry: Bezier paths, flattening, polygons, offsets, clipping (wraps geo / i_overlay)
│   ├── emb-model     # Design data model: objects, properties, threads, hoop, undo/redo history, native project format
│   ├── emb-stitch    # Stitch generation: running, satin, fill, underlay, compensation, routing
│   ├── emb-formats   # Machine formats (VP3, PES, DST, ...) and vector artwork import (SVG, PDF, AI, EMF, WMF)
│   ├── emb-render    # 2D thread preview: stitch list -> colored line segments (no UI dependency; later: 3D)
│   ├── emb-letter    # Lettering: fonts, glyph strokes, satin and fill letters, text layout (no UI dependency)
│   ├── emb-art       # Bitmap artwork: crop, adjust, color reduction, thread matching, background removal and subject cut-out, auto-trace, click-to-stitch, picture to design, photo stitch, recognize outlines (no UI dependency)
│   ├── emb-library   # Design library: index of design folders, thumbnails, search, sort, group (no UI dependency)
│   └── emb-app       # Desktop UI (egui)
├── data/
│   ├── hoops.toml    # Hoop presets
│   ├── machines.toml # Machine profiles
│   └── threads/      # Thread color charts (Madeira, Isacord, Robison-Anton, ...)
└── tests/
    └── corpus/       # Reference designs and expected stitch outputs
```

**Dependency direction:** `emb-geom` ← `emb-model` ← (`emb-stitch`, `emb-formats`, `emb-render`, `emb-letter`, `emb-art`) ← `emb-app`. `emb-formats` and `emb-render` read stitch lists from `emb-model` and do not depend on `emb-stitch`. `emb-art` also uses `emb-letter` (the stroke finder, ADR 0028). `emb-library` uses `emb-stitch`, `emb-formats` and `emb-render` (ADR 0035). A lower crate must never import a higher crate.

**Data model.** A design is an ordered list of objects. The order is the sew order. Each object has:

- A stable `ObjectId` and a `revision` number. The revision increases each time the object changes.
- Geometry in mm, origin at hoop center, Y axis down. Geometry is stored as cubic Bezier paths (move, line, cubic, close), so the node editor can edit curves. The engine flattens paths to polylines with a set tolerance (default 0.05 mm). The geometry kind follows the stitch type:
  - Running: one open or closed path.
  - Fill: one closed outer path plus zero or more hole paths.
  - Satin: two rails plus zero or more rungs (pairs of points that set the stitch direction). The "alternate left and right clicks" input used by consumer embroidery products maps directly to rails and rungs.
- An optional start point and an optional end point. If they are not set, the engine selects them. Routing, travel stitches and trims between objects use these points.
- A stitch type and its parameters (density, angle, stitch length, underlay type, pull compensation).
- A thread reference.

The stitch cache is not part of the object. It is a separate map with the key `(ObjectId, revision)`. The engine regenerates an entry when the geometry or a parameter changes. Because undo restores an old revision, the cache can return the old stitches with no regeneration.

**Stitch plan.** The engine builds the full stitch plan from the per-object stitch lists. This step adds travel stitches, trims, lock stitches, color changes and the design rotation. Each stitch record has a kind: `Stitch`, `Jump`, `Trim`, `ColorChange`, `Stop`, or `End`. The stitch types live in `emb-model`, so `emb-formats` and `emb-render` do not depend on `emb-stitch`.

**Undo and redo.** Undo and redo are part of `emb-model` from Phase 0, not an app feature added later:

- The design stores objects as `Arc<Object>`. A snapshot of the design is cheap: it copies pointers, not objects. An edit clones only the objects that it changes (copy on write).
- All changes to a design go through `Document::edit(label, |design| ...)`. There is no other public way to change a design in a document, so no operation can skip the history.
- Continuous edits (a slider drag, a node drag) use a merge key. Edits with the same merge key that follow each other become one undo step.
- The history has a configurable depth limit (default 200 steps).
- View state (zoom, pan, selection, visible view mode) is not in the history.

**Native project format.** Save designs as a versioned, human-readable JSON file (`.embproj`) that keeps the objects and parameters. The file has a `format` string and a `version` number. The loader rejects unknown versions with a clear message and migrates old versions. Stitch formats such as VP3 and DST keep only stitches, so the app cannot edit them as objects. Always save the native file next to each export. The native format lives in `emb-model`; `emb-formats` is only for machine formats.

**Determinism across platforms.** Golden-file tests run on Windows, macOS and Linux, so the engine must give the same output on each platform:

- Use the `libm` crate (through the `emb_geom::math` module) for `sin`, `cos`, `tan`, `atan2`, `hypot` and other transcendental functions in engine crates. Platform math libraries can differ in the last bit. `sqrt` and basic arithmetic are exact under IEEE 754, so `std` is safe for them.
- Do not iterate over a `HashMap` or `HashSet` where the order changes the output. Use `BTreeMap`, `IndexMap`, or sorted vectors.
- Golden snapshots round coordinates to 0.01 mm.

**Units.** Use millimeters as `f64` in the engine. Convert to the format unit (for example, 0.1 mm for DST) only in `emb-formats`.

## Stitch engine requirements

Each stitch type is a pure function: geometry and parameters in, a stitch list out. The function must be deterministic. The same input must always give the same output, so tests can compare results exactly.

| Stitch type | Input geometry | Main parameters | Default (starting values, tune in tests) |
| --- | --- | --- | --- |
| Running | Open or closed path | Stitch length, repeats (1, 3 for bean) | 2.5 mm, 1 repeat |
| Satin | Two rails plus optional rungs, or one path plus width | Density, width, pull compensation, short stitches at curves | 0.4 mm spacing, 0.2 mm compensation each side |
| Tatami fill | Closed polygon with holes | Density, angle, stitch length, offset pattern | 0.4 mm row spacing, 45°, 3.5 mm stitch |
| Underlay | Generated under satin and fill | Type (center walk, edge walk, zigzag, tatami), inset | Edge walk + zigzag for satin wider than 2 mm |

**Engine functions in Phase 1:**

1. Pull compensation: widen satin and fill shapes perpendicular to the stitch direction.
2. Short-stitch handling: on the inside of a satin curve, shorten every second stitch to prevent thread buildup.
3. Fill routing: cover the polygon in connected rows with a minimum of jumps. Travel inside the shape where possible.
4. Object connection: connect objects of the same color with a travel stitch if the gap is less than the trim threshold (default 2 mm). Insert a trim if the gap is larger.
5. Stitch-length limits: split each stitch that is longer than the machine maximum. Remove stitches shorter than the minimum (default 0.3 mm).
6. Lock stitches: add tie-in and tie-off stitches at the start and end of each color block and after each trim.

**Later engine functions:** lettering from TTF and OTF fonts (roadmap P4, done), a satin from a center line (P1, done: Column C), and auto-digitize from raster images (color reduction, region segmentation, stitch type selection: P6, done in software).

**Performance target:** regenerate stitches for one changed object in less than 100 ms for an object with 10,000 stitches. Regenerate only the changed object, not the full design.

## File formats

VP3 is the first export target because the reference machine uses it. Use pyembroidery (MIT) as the reference for each format structure.

| Format | Brand | Phase | Read | Write | Notes |
| --- | --- | --- | --- | --- | --- |
| VP3 | Husqvarna Viking, Pfaff | 1 | Yes | Yes | Reference machine format. Keeps thread colors and names. |
| DST | Tajima (industry standard) | 1 | Yes | Yes | Simple. Unit 0.1 mm. Maximum move per record 12.1 mm, so split longer moves into jumps. No color data. |
| PES | Brother, Babylock | 1 | Yes | Yes | Most common home format. Contains a PEC block that the machine reads. Write PES v1 first. |
| EXP | Melco, Bernina | P3 | Yes | Yes | Simple. Done (byte-checked against pyembroidery). |
| JEF | Janome | P3 | Yes | Yes | Has hoop-size codes in the header. Done (byte-checked against pyembroidery). |
| XXX | Singer | P3 | Yes | Yes | Done (byte-checked against pyembroidery). |
| SEW, SHV | Janome, Husqvarna Viking | P3 | Yes | No | Read only. Done. |
| HUS, VIP | Older Husqvarna Viking | P3 | Yes | No | Compressed. Read only. HUS done; VIP waits for a public reference (pyembroidery has no VIP reader). |
| TBF | Tajima | P3 | Yes | Yes | DST-like header with needles and thread colors. Done (byte-checked against pyembroidery). |
| TAP | Happy | P3 | Yes | Yes | DST records with no header. Done (same bytes as the DST records of pyembroidery). |
| DSB, DSZ | Barudan (also BAI, Ricoma), ZSK | P3 | Yes | No | Read only: pyembroidery has no writer. Done. |
| 10O, 100 | Toyota | P3 | Yes | No | Read only: pyembroidery has no writer. Done. |
| PEC | Brother | P8 | Yes | Yes | The machine part of PES. Done (byte-checked against pyembroidery). |
| U01 | Barudan | P8 | Yes | Yes | Keeps the slow function (ADR 0037). Done (byte-checked against pyembroidery). |
| DAT, EXY, FXY, GT, STX, PHB, PHC, BRO, EMD, INB, JPX, KSM, MAX, MIT, NEW, PCD, PCQ, PCS, PCM, SPX, STC, ZHS, ZXY | Barudan, Sunstar, Eltac, Fortron, Gold Thread, Data Stitch, Brother, Bits & Volts, Elna, Inbro, Janome, Pfaff, Mitsubishi, Ameco, Gunold, Zeng Hsing | P8 | Yes | No | Read only: pyembroidery has no writer. Done (each reader gives the same stitches as pyembroidery). |
| INF, EDR, COL | Thread color files | P8 | Yes | Yes | Next to a file with no colors (DST); read when it opens. |
| SVG | Vector import | 1 | Yes | Yes | Import paths as objects. Export the stitch preview. |

**Rules for format code:**

- Each format is a module in `emb-formats` with two functions: `read(bytes) -> StitchDesign` and `write(&StitchDesign) -> bytes`.
- Use round-trip tests: write a design, read it back, and compare. Allow a position tolerance of one format unit.
- Do not guess undocumented header fields. Copy the behavior of pyembroidery, and record the source line in a comment.
- The final test for each writer is a sew-out on a real machine. A file that opens in another program but does not sew correctly is a failure.

## UI and UX requirements

The UI is the main reason for this project. A new user must be able to digitize and export a simple two-color patch in less than 15 minutes, with no manual.

**Layout (ribbon, as in CAD programs; ADR 0010):**

- Top: a ribbon with a File menu, quick-access buttons and the tabs Digitize, Edit, Arrange, View and Design. Each tab has named panels of commands. Below the ribbon: the property bar of the current tool or selection.
- Center: canvas with the hoop boundary, a grid in mm, and zoom from 10 % to 3200 %.
- Right, top: Object Properties panel. It shows only the parameters of the selected stitch type.
- Right, bottom: Sequence panel. It shows color blocks and their objects at the same time, in sew order, with thumbnails. Drag objects or blocks to reorder. It shows the color-change count and offers a layer-safe merge. See `docs/design/toolsets.md`.
- Bottom: the thread colors of the design, and a status bar with stitch count, color count, design size in mm, and estimated sew time.

**UX rules:**

1. Each parameter change updates the stitch preview immediately. Do not use an "Apply" button.
2. Show each parameter in mm and in a plain-language form, for example "Density: 0.40 mm (normal)".
3. Give each stitch type good defaults, so most users never change them.
4. Let the user toggle between three views: Artwork, Stitches, and Thread preview.
5. Support full undo and redo for all operations.
6. Show warnings in a list with click-to-select: stitch outside the hoop, density too high, stitch too short, too many trims.
7. Let the user import a raster image as a locked background for manual tracing.
8. Use one keyboard shortcut set, documented in the app. Do not copy the shortcut tables of closed-source embroidery products exactly; use common conventions (Ctrl+Z, Del, arrow-key nudge). Canvas navigation follows CAD conventions: wheel zooms at the cursor, middle or right drag pans, and a double-click on the middle button (or Home) zooms to extents. The full table is in `docs/design/toolsets.md`.
9. Sew order is stacking order: an object that sews later lies on top. Automatic sequencing must not change what is on top of what unless the user asks for it.
10. Objects can be changed directly on the canvas: drag to move, handles to scale and rotate, box select, arrow-key nudge. A right click opens a menu of the commands that apply to what is under the pointer. Each command is also in a main menu, so the right-click menu is a shortcut, not the only way.

**Toolsets.** `docs/design/toolsets.md` maps each toolset of the consumer reference product to the job it does and to our version and phase. Use it when you plan a phase.

**Export dialog:** select the format, select the hoop, show a summary (size, stitches, colors, trims), and show a final warning list. The default format comes from the selected machine profile.

## Roadmap

Each phase has an exit criterion. Do not start the next phase until the exit criterion is met. Engine work for a later phase (for example P5 stitch types) can run in parallel with app work, because the engine crates do not depend on the app.

The full roadmap, with the scope, dependencies and exit criterion of each phase, is in `docs/design/roadmap.md`. Summary:

| Phase | Scope | State |
| --- | --- | --- |
| 0. Foundation | Workspace, model with undo/redo, project files, egui app, Sequence panel, canvas editing, draw tools, file handling, SVG import. | Done (three-platform CI deferred by the owner). |
| P1. Stitch engine and core formats | Running, tatami, satin, underlay, compensation, plan; DST, VP3, PES; CLI and golden tests. Milestones in `docs/design/export-plan.md`. | Done (VP3, DST, PES; running, satin, tatami). The sew-out exit test is covered (owner decision, 2026-10-23): the engine changed much since P1; VP3 sewed correctly in the P4 sew-outs, and the P5, P6 and P7 kits sew running, satin and fill stitches of the current engine. |
| P2. Digitizing toolset | All input methods, reshape, stitch angles, holes, knife, branching, selection, transforms, travel, grid/guides/snap, backdrop tracing. Milestones in `docs/design/p2-plan.md`. | Done in software. Exit test with users open. |
| P3. Production, color and quality | Thread charts, colorways, fabrics, styles, templates, machine functions, stitch edit, machine files, more formats, worksheets, stitch player, appliqué. Milestones in `docs/design/p3-plan.md`. | Done in software: Milestones 1 (threads and color), 2 (machine functions, split at needle, stitch list), 3 (machine files and formats), 4 (stitch player, thread look, design info, warnings, overview), 5 (worksheets, PDF, picture, Output design, batch output), 6 (fabrics, styles, templates, plain-language labels), 7 (appliqué) and 8 (design tabs, split view, backups and recovery, calibration, sew time, P2 carry-overs) done in software; the exit test with users is open. |
| P4. Lettering | Fonts, TrueType conversion, lettering objects and effects, monograms, team names, keyboard design collections. | Done: Milestones 1 (TrueType lettering), 2 (embroidery fonts), 3 (baselines and letter editing), 4 (effects and sew order), 5 (monograms), 6 (team names, keyboard designs) and 7 (custom alphabets, bitmap fonts), and the sew-out exit test (2026-10-23). See `docs/design/p4-plan.md`. |
| P5. Decorative stitches and effects | All remaining stitch types, effects, motifs, splits, stamps, blends. | Done in software: Milestones 1 (column and outline types), 2 (curved fills), 3 (texture fills), 4 (motifs), 5 (patterned stitching), 6 (stitch effects) and 7 (blends, buttonholes, borders); the sew-out exit test is open. See `docs/design/p5-plan.md`. |
| P6. Artwork and auto-digitizing | Vector layer, shaping, offsets, arrangements, bitmap tools, auto-trace, click-to-digitize, smart and photo digitizing. | Done in software: Milestones 1 (vector objects), 2 (shaping), 3 (offsets and arrangements), 4 (border trace), 5 (picture tools; PDF, AI, EMF and WMF import), 6 (auto-trace, click to stitch), 7 (Picture to design wizard) and 8 (photo stitch, recognize outlines), with the owner requests (linked shapes, cut out subject); the exit tests are open. See `docs/design/p6-plan.md`. |
| P7. Hooping and presentation | Hoop templates, multi-position and multi-hooping, product backdrops, repeats. | Done in software: Milestones 1 (hoop placement), 2 (multi-hooping engine), 3 (hooping mode, output for each hooping), 4 (multi-position hoops), 5 (product backdrops, picture on product) and 6 (repeats); the exit test (kit in `out/p7-sewout`) is open. See `docs/design/p7-plan.md`. |
| P8. Management and connectivity | Design library, clipart, export queue, send to USB, open machine protocols, remaining formats. | Done in software (exit test part 2 open): Milestones 1 (design library engine, ADR 0035), 2 (Library tab, catalog), 3 (Send to USB, ADR 0036), 4 (export queue), 5 (clipart), 6 (more machine formats, speed functions, ADR 0037) and 7 (cutter files, barcodes). See `docs/design/p8-plan.md`. |
| P9. Specialty decoration | Sequins, beads, bling, chenille, Schiffli, cross stitch. Milestones in `docs/design/p9-plan.md`. | Done in software for the parts that the owner can test: Milestones 1 (cross-stitch objects, ADR 0038), 2 (picture to cross stitch, charts), 3 (rhinestone objects, ADR 0039), 4 (rhinestone templates) and 5 (mixed decoration and output). The sew-outs are open; Milestones 6 (sequins, beads) and 7 (chenille, Schiffli) wait for a tester with such a machine. |
| P10. Polish and assistance | 3D thread preview, start page, simple mode, kiosk, translations, installers (the optional assistant is benched, owner decision). Milestones in `docs/design/p10-plan.md`. | Done in software (Milestone 9, the assistant, is benched): Milestones 1 (editing helpers), 2 (view and workspace helpers), 3 (3D thread look, ADR 0040), 4 (start page and help), 5 (simple mode and lettering kiosk), 6 (files, assets and content), 7 (translations and accessibility: German, Spanish, Japanese) and 8 (installers and release builds, ADR 0043; the first release build in CI is open). |
| P11. Features of other programs | Density map, thread use, thread charts and fonts, file manager thumbnails, Ink/Stitch stitch types, free-standing lace, quilting, puff, cap sequencing, cutwork. Milestones in `docs/design/p11-plan.md`. | Done in software (the sew-outs and the tests with a cap frame and a cutwork machine are open): Milestones 1 (density map and thread use, ADR 0044) 2 (brand thread charts, thirty embroidery fonts, joins of satin letters, ADR 0045) 3 (pictures of designs in the file manager, ADR 0046) 4 (tartan, gradient, meander, circular, guided fills and ripple stitch, ADR 0047) 5 (free-standing lace, ADR 0048) 6 (quilting helpers, ADR 0049) 7 (puff and cap sequencing, ADR 0050) and 8 (cutwork needles, ADR 0051; files not tried on a machine) done in software. |
| P12. Picture to design: robust and better results | Photo orientation, background previews, true width for Auto, a picture bench; layers, fill angles, small areas, checks before Finish; line art, edge-preserving smoothing, colors from the thread chart. Milestones in `docs/design/p12-plan.md`. | Done in software: Milestones 1 (robustness and the picture bench, ADR 0052), 2 (layers, fill angles, small areas, checks before Finish, ADR 0053) and 3 (line art, edge-preserving smoothing, colors from the thread chart, ADR 0054); the exit tests (sew-outs on the Topaz 20) are open. |
| P13. Shop tools | Quotes and run time on machines with more heads, order details on the sheets, thread stock and threads to buy, scripts. Milestones in `docs/design/p13-plan.md`. | Done in software (owner request, 2026-10-27): Milestones 1 (quotes and run times, ADR 0064), 2 (order details, ADR 0065), 3 (thread stock, ADR 0066) and 4 (scripts, ADR 0067); the exit tests with a shop tester are open. |
| P14. Sketch mode | A parametric sketch: lines, arcs and circles with constraints and dimensions, a solver, sketch edits, regions to embroidery that stay linked, parameters. Milestones in `docs/design/p14-plan.md`. | Done in software (owner request, 2026-10-27): Milestones 1 (sketch model and solver, ADR 0068), 2 (sketch mode in the app, ADR 0069), 3 (drawing help and sketch edits, ADR 0070), 4 (regions to embroidery, ADR 0075), 5 (objects that follow their sketch, ADR 0076) and 6 (parameters, splines and drawings to sketches, ADR 0078 and 0079). The owner's test in the app is open. |

## Test strategy

The final judge of correctness is a sew-out on a real machine. Software tests make sure that a known-good result does not change by accident.

| Test type | What it checks | Where |
| --- | --- | --- |
| Unit | Geometry functions, parameter limits, unit conversion. | Each crate, `#[cfg(test)]` |
| Property | Invariants for random input: no stitch outside the shape plus compensation, no stitch longer than the limit, no NaN coordinates. Use `proptest`. | `emb-stitch`, `emb-geom` |
| Golden file | Stitch output for each design in `tests/corpus/` matches a stored snapshot. Use `insta`. A change needs a review and a note. | `tests/` |
| Differential | Compare stitch output with Ink/Stitch for the same SVG and parameters. Compare stitch count, extents and density, not exact positions. | `tests/differential/` (script) |
| Round-trip | Write and read each format with a tolerance of one format unit. | `emb-formats` |
| Sew-out | Sew each test design on the Topaz 20. Record a photo and a pass or fail result in `tests/sewouts/`. | Manual, each phase exit |

**Standard test corpus (minimum):** a 50 mm circle in tatami fill; a 3 mm × 80 mm satin column; an S-curve satin with varied width; a fill with two holes; a two-color patch with a satin border; a design at the full 260 mm × 200 mm field.

## Instructions for Claude Code

Put this document in the repository as `docs/SPEC.md`. Copy this section into `CLAUDE.md` at the repository root.

**Rules:**

1. Read `docs/SPEC.md` before you start a task. If a task conflicts with the spec, stop and ask.
2. Work in one phase at a time. Do not add features from a later phase.
3. Do not decompile, disassemble or copy code, assets or file samples from closed-source embroidery products.
4. When you port an algorithm from Ink/Stitch or pyembroidery, add a header comment with the source file path and the commit hash.
5. Keep engine crates free of UI dependencies. Do not add a dependency from a lower crate to a higher crate.
6. Use mm as `f64` everywhere outside `emb-formats`.
7. Add or update tests with each change. Do not update a golden snapshot without a note in the commit message that says why the output changed.
8. Run `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test --workspace` before each commit.
9. Do not use `unsafe` code. Each crate has `#![forbid(unsafe_code)]`. Engine crates (`emb-geom`, `emb-model`, `emb-stitch`, `emb-formats`, `emb-render`, `emb-letter`, `emb-art`, `emb-library`, `emb-script`, `emb-sketch`) must not depend on C or C++ libraries through FFI. Normal pure-Rust dependencies are permitted. The UI stack (`eframe`, `winit`, `wgpu`) uses `unsafe` internally to talk to the operating system; ADR 0001 records this. The app also uses `sysinfo` to find USB sticks (ADR 0036). Record any other exception in `docs/adr/`.
10. Record each architecture decision as a short ADR in `docs/adr/NNNN-title.md`.
11. Write documentation and code comments in ASD-STE100 Simplified Technical English.

**First task:** Phase 0. Create the workspace and the crate skeletons. Set up CI for three platforms. Load and validate `data/hoops.toml` and `data/machines.toml`. Build the undo/redo history and the native project format in `emb-model`. Build an egui prototype that shows a hoop outline and a pan-and-zoom canvas, and that draws a large synthetic stitch list to test speed. Write ADR 0001 with the UI framework decision.

## Open questions

- [x] VP3 is the native format of the Topaz 20: the P4 sew-outs sewed from VP3 files (2026-10-23).
- [ ] Confirm the transfer method (USB stick or PC cable).
- [ ] Confirm the maximum stitch length and the minimum stitch length of the Topaz 20.
- [ ] List the hoops that you own, and measure the sewable field of each.
- [x] UI framework: egui. See ADR 0001.
- [x] Maximum field of the Topaz 20: 260 mm × 200 mm (DESIGNER Crown Hoop), confirmed from Husqvarna Viking data.
- [x] Project name: ShokujoDX (ADR 0061; Ariadne, ADR 0059, was dropped). Checks of 2026-10-26: no trademark for Shokujo or ShokujoDX in software, sewing machines or embroidery in TMview (the US, EU, UK, Japan and most other offices) or in Japan's J-PlatPat (織女 and ショクジョ: one mark, hotel furniture and fabrics); the only Latin-letter marks are a towel brand (Imabari Shokujo). No project of that name on GitHub, PyPI, crates.io or npm. China's register (织女) was not searched: CNIPA needs a person for its CAPTCHA. Do that, and a full clearance search with a trademark lawyer, before the program is sold or licensed commercially.
- [x] Thread charts: the program ships the Brother PEC chart of pyembroidery (MIT) and the 74 brand charts of Ink/Stitch (GPL), each file with its source line (owner decision, 2026-10-25, ADR 0045). The brand names are the trademarks of their owners and only name the threads.
- [x] Ink/Stitch is GPL 3.0 or later: the LICENSE file is the plain GPL 3.0 text, and the header of `inkstitch.py` says "Licensed under the GNU GPL version 3.0 or later" (checked 2026-10-27). So ported code can be in this GPL-3.0-or-later project.
- [ ] Ask the Ink/Stitch maintainers if they want to coordinate. A shared engine could serve both projects.
