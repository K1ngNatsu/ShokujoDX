# P15, step 1: the browser trial

Date: 2026-11-21 · Branch: `trial/browser` · Status: Report, with the
first changes of step 2 (owner, 2026-11-21: a test page on GitHub Pages
for tests with friends, and the recommendations below). See "P15.
Browser version" in `docs/design/roadmap.md`.

## What the trial is

The program (`emb-app`) builds for the browser (`wasm32-unknown-unknown`)
and runs in a web page. The desktop program does not change: on the
desktop, each new part is the part of before.

- **Build:** `python tools/build-web.py` makes `target/web` (the program as
  WebAssembly, its JavaScript glue from `wasm-bindgen` and the page
  `web/index.html`); `--serve` starts a local web server. The tool
  `wasm-bindgen-cli` of the version of `Cargo.lock` (0.2.129) is in
  `target/tools`, not in the repository.
- **Measure:** `node tools/web-bench.mjs` opens the page in a headless
  browser (Edge on Windows) and reads the frame times, the memory and a
  screenshot of each case. The page takes the measures from its address:
  `?stress=<stitches>` (a synthetic design, `emb_render::stress`),
  `look=flat|fast|good|best`, `bench=still|pan|zoom`, `open=<file>`,
  `export=<format>`, `letter=<text>` and `gl=1` (WebGL2 only).
- **Files:** File > Open is an upload (a project, a machine file, artwork
  or a picture). Save gives the project as a download, and Export Machine
  File gives the machine file as a download. The other file dialogs (about 45
  places in the code) show a notice that the command needs the desktop
  program.
- **Code** (all in the branch):
  - `emb_geom::threads::scope`: `std::thread::scope` on the desktop; in the
    browser each job runs when it is spawned (six places in the engine,
    one in the app). The jobs share only their inputs and an atomic
    counter, so the result is the same.
  - `emb-app`: `background::spawn` for the nine background jobs (in the
    browser the job runs at once and sends its result as before),
    `clock` (the clock of the browser through `web_time`; 20 files),
    `dialog::FileDialog` (the dialog of the system on the desktop; in the
    browser it gives no file; 23 files), and `web.rs` (the start on the
    canvas of the page, the upload, the downloads, the notice, the
    measures).
  - The routes of the letters of TrueType fonts are made when a letter is
    laid out (no route thread), and the 3D tiles are made in the frame
    (`TileCache::sync`).
  - Dependencies for the browser only: the browser backend of `getrandom`
    (0.3 and 0.4, and a flag in `.cargo/config.toml`), `wasm-bindgen`,
    `wasm-bindgen-futures`, `js-sys`, `web-sys`, `web-time` and the
    feature `wasmbind` of `chrono`. All were in `Cargo.lock` already
    (eframe uses them); no new crate.

## What works

- The program starts in about 0.1 s after the page has the program (from
  the cache of the browser): the ribbon, the panels, the windows, the
  help, the first start question, the 3D view at all qualities.
- New designs, drawing, lettering with the bundled fonts, Open by upload,
  Save and Export by download.
- **The same stitches as the desktop program:** a DST file exported in the
  browser is the same, byte for byte, as the file of `emb-cli convert` on
  the desktop (ADR 0003).
- WebGPU and WebGL2 (`?gl=1`, as in a browser with no WebGPU) both draw.
- **The test page** (owner, 2026-11-21): the workflow
  `.github/workflows/web-trial.yml` builds the page from the branch
  `trial/browser` and publishes it on GitHub Pages. The page says what the
  trial is, the program says it once in its status line (four languages),
  and the settings stay in the storage of the browser (the machine, the
  hoop, the language, the quality of the 3D look), so the first start
  question comes one time. The page is https://k1ngnatsu.github.io/ShokujoDX/
  (published 2026-11-21): GitHub Pages sends the program with gzip
  (38.2 MB), and a new headless browser on the development computer starts
  it in 3.4 s with the download (`BASE=<address> node tools/web-bench.mjs`
  measures a published page).

## Measures

Microsoft Edge 158 (headless, no frame limit), on the development
computer: AMD Ryzen 9 9950X (32 threads; the browser uses one), NVIDIA
GeForce RTX 5070 Ti, 62 GB. A frame of an empty design takes 0.9 ms, so
the numbers below are the work of the program.

| Case | Frame, mean | Slowest frame | Memory of the program |
| --- | ---: | ---: | ---: |
| Empty design | 0.9 ms | 8 ms | 73 MB |
| A small project (two-color patch) | 1.1 ms | 21 ms | 73 MB |
| 200,000 stitches, flat look, still | 49 ms | 132 ms | 440 MB |
| 200,000 stitches, flat look, pan | 48 ms | 129 ms | 440 MB |
| 200,000 stitches, flat look, zoom | 52 ms | 126 ms | 440 MB |
| 200,000 stitches, 3D Fast, pan | 49 ms | 122 ms | 440 MB |
| 200,000 stitches, 3D Good, pan | 59 ms | **3.5 s** | 494 MB |
| 200,000 stitches, 3D Best, still | 93 ms | **6.2 s** | 493 MB |
| 200,000 stitches, 3D Best, pan | 104 ms | **6.3 s** | 493 MB |
| 200,000 stitches, 3D Best, zoom | 124 ms | **6.2 s** | 493 MB |
| 1,000,000 stitches, flat look, pan | 153 ms | 438 ms | 1,525 MB |
| 200,000 stitches, flat, WebGL2 | 54 ms | 206 ms | 617 MB |
| 200,000 stitches, 3D Best, pan, WebGL2 | 107 ms | 6.2 s | 656 MB |

- **The slowest frames** of the 3D look at Good and Best are the first
  view: the lay of the surface and the first tiles run in one frame, so
  the page stops for 3.5 s to 6.2 s. The desktop program makes them on
  all processors in about 0.4 s to 0.6 s for 100,000 stitches
  (`docs/design/p18-view-m5-report.md`).
- **Speed:** with 200,000 stitches the browser draws about 10 to 20 frames
  a second. The desktop program draws 182 frames a second with 106,576
  stitches and 23 with 787,408 in the 3D meshes, and 740 and 117 on tiles
  (the same report). So the browser is about 10 to 30 times slower here:
  one thread, and WebAssembly is slower than native code.
- **Lettering:** 15 Latin letters in Lato take 0.3 s with their routes. A
  kanji takes up to 0.56 s on the desktop (ADR 0144), so a new kanji in
  the browser stops the page for about that time or more.
- **Download:** the program is 67.7 MB (37 MB with the gzip compression of
  a web server). The compressed bundled fonts are 12.5 MB of it. On a line
  of 10 Mbit/s the first visit takes about 30 s; then the browser keeps
  it.
- **Memory:** WebAssembly memory does not get smaller while the page is
  open. 440 MB with 200,000 stitches and 1.5 GB with 1,000,000 stitches;
  WebGL2 uses about 40 % more. Safari on an iPad stops a page at about
  1 to 1.5 GB (not measured here).

## After the first measures (2026-11-21)

- **The tiles of the 3D look are made a little in each frame.** With no
  threads, the tile cache made all tiles of a view in one frame. Now a
  frame uses at most 30 ms for tiles (`TileCache::frame_budget`; always
  one tile, so the tiles come), and the next frames make the others. The
  desktop and the tests have no budget, as before. The longest frame of
  the first 3D view of 200,000 stitches: 6.2 s to 1.0 s (Best) and 3.5 s
  to 0.54 s (Good). The rest is the lay of the surface, which runs in one
  piece (about 0.5 s for 200,000 stitches, much less for a design of
  20,000).
- **The quality of the 3D look** comes from the short measure of the
  first start (ADR 0130), as on the desktop: on the development computer
  it chooses Good. With the budget above, Good and Best do not stop the
  page for seconds, so the browser does not start on Fast.
- **`wasm-opt`** (`tools/build-web.py --opt`, binaryen 0.133 from
  crates.io): the program 70.1 MB to 59.8 MB, but the download with gzip
  stays 37 MB (the optimized code compresses less), and the frames are not
  quicker. The page is built with no `--opt`.
- **A profile of a still frame** with 200,000 stitches (49 ms in the flat
  look): about 25 % strokes the outline of each stitch object (its outline
  is all its stitches, 53,000 points), about 20 % uploads the vertex
  buffers to the graphics card, about 14 % grows memory, and 3 % makes the
  mesh of the stitches. The next optimizations for large designs: no
  outline of a stitch object while its stitches show (a change of the
  desktop view too, so the owner decides), and a mesh of the stitches
  that is kept between frames when the view does not change.

## Tablets and touch screens (step 3, owner, 2026-11-21)

The owner opened the test page in Safari on a phone: it works, a little
slower than on the computer. Then the owner asked for tablets and touch
screens (step 3 of the roadmap; "only if users ask").

- **Two fingers on the canvas** pan, and a pinch zooms about the middle
  between them (`canvas::show`, egui `multi_touch`). From the second
  finger until all fingers are up, the tools take no input
  (`AppState::touch_gesture`), so a pinch does not move an object, make a
  selection box or add a point. A drag that one finger started before
  the second one ends normally. Before, a pinch zoomed about the first
  finger, nothing panned (a pan needed a middle or right drag), and the
  first finger of a pinch moved objects.
- **A long press is a right click** in egui (`Response::secondary_clicked`),
  so the menu of an object on the canvas, of the Sequence panel and of the
  other lists opens with a long press. No change was needed.
- **The draw tools on a touch screen** finish a line with Enter, remove a
  point with Backspace and stop with Esc, and a tablet often has no
  keyboard. After the first touch, a bar at the bottom of the canvas has
  **Finish**, **Remove Point** and **Cancel** while a shape has points
  (`canvas::touch_bar`; it calls `AppState::draw_key`). A computer with a
  mouse does not show it.
- **The first visit on a touch screen** (no saved settings, a coarse
  pointer): a tablet gets the text size 125 % (larger controls); a phone
  (narrower than 700 CSS pixels) gets simple mode (six tabs with large
  buttons). The settings of a visit before stay as they are.
- **The page** takes all fingers on its canvas (no scroll, zoom, bounce,
  text selection or callout of the page), has a manifest with icons and a
  service worker (`web/sw.js`; a cache for each version of the program, so
  an update is a new cache), so "Add to Home Screen" makes an app that
  also works offline. The service worker runs only on https, so a local
  test build is not kept. A new version takes the place of the old one
  when the page opens, and the next reload shows it (before 2026-11-22,
  only after all pages of the old version were closed, which a reload in
  Safari does not do). It gets its files from the network, not from the
  cache of the browser.
- **Tests:** `canvas::tests::two_fingers_zoom_about_their_middle_pan_and_move_nothing`
  (the harness sends the touch events of eframe on the web,
  `Harness::pinch`), and `tools/web-touch.mjs` in a headless browser with
  touch emulation: a tablet (1180 × 820) and a phone (390 × 844) on a
  first visit, and a pinch of two fingers (zoom 2.000 times, the view
  follows the fingers, no undo step, the gesture ends). With an emulated
  device scale of 2, the headless browser gave the program the canvas at
  half its size and the touches missed it; the test uses the scale 1. A
  real phone does not do this (the taps of the owner's phone hit their
  buttons).
- **Not tested on a real tablet:** the pinch and the pan, the long press,
  the touch bar, the keyboard of the screen in a text field, "Add to Home
  Screen" and offline use (iPad and Android). Hover tooltips and F1 do not
  exist on a touch screen; keyboard shortcuts need a keyboard.

## Phones (the owner's report, 2026-11-22)

The owner used the test page on a phone and found seven problems. The
changes:

1. **Words on top of words (upright phone).** The Start page has one
   column when it is narrower than 560 points (`start.rs`). The status
   bar scrolls to the side. The update check is not in the browser (no
   question at the first start, no check at the start; Help > Check for
   Updates says that it is not in the browser), so its window does not
   open wider than the phone.
2. **The ribbon tabs did not scroll.** The File button, the quick buttons
   and the tabs are in a row that scrolls to the side; the arrow that folds
   the ribbon stays at the right. The panels of a tab also scroll. These
   rows and the status bar scroll with a drag of the finger also before
   egui knows that the screen is a touch screen (in egui, a drag scrolls
   only after the first touch).
3. **Side panels and ribbon: size, and open and close.** A button at the
   right edge of the canvas hides the side panels and shows them again
   (`canvas::side_handle`, `AppState::side_hidden`; larger on a touch
   screen). The dock is kept while they are hidden (`EmbApp::saved_dock`),
   so they come back with their tabs and sizes. On a touch screen the line
   between two panels is wider for a finger (3 points, 14 points to each
   side to drag). The arrow at the right of the tabs folds the ribbon (as
   before).
4. **Back to Select after a command.** When new lettering is added, the
   tool is Select again (on each computer; the message says what was
   added). On a touch screen the draw tools also go back to Select when a
   shape is finished; with a mouse they stay on, so the next shape can be
   drawn, and Esc stops them (as before).
5. **The rotate handle was too small for a finger.** On a touch screen the
   handles are drawn 1.6 times larger, the rotate handle is 44 points above
   the box (24 with a mouse), so it is farther from the top handles, and a
   handle takes a touch from 22 points away (the nearest handle wins).
6. **Only the Start page at the start.** While the Start page is the tab
   that shows, the side panels are hidden; when a design shows, they come
   back. A command that shows a side panel shows them also.
7. **The ribbon on a phone held across.** A screen lower than 520 points
   shows only the tab row of the ribbon. A tap on a tab shows its panels
   over the canvas; a command, a second tap on the tab or a tap outside
   closes them (`AppState::ribbon_peek`).

Tests: `ribbon::tests::a_swipe_scrolls_the_tabs_of_a_phone` (a swipe of one
finger scrolls the tabs and is not a tap on a tab) and
`a_short_screen_shows_the_panels_of_a_tab_over_the_canvas`.
`tools/web-shots.mjs` makes pictures of a phone (upright and across, the
Start page and a design, a swipe on the tabs, the side panels hidden, the
panels of a tab over the canvas) and of a tablet in the headless browser.
Not tested on a real phone yet.

## Upgrades and faults (the owner's second report, 2026-11-22)

1. **The version.** File > Check for Updates… (also on the desktop, where
   it is the check of Help > Check for Updates). In the browser it compares
   the build of the page with `version.json` on the server
   (`tools/build-web.py` writes the version, the time of the build and the
   commit into the page and that file): the window shows the build of the
   page and, if a newer build is there, Reload Now (the service worker
   gets the new files first). The test page has one build only, so no
   earlier build can be chosen.
2. **One group of side panels.** `app::default_dock` puts all eight side
   panels in one group at the right (`app::SIDE_PANELS`, their order in
   the group); a closed panel opens again in that group.
3. **Reset the Panels** (Options, row Panels, and View > Panels) puts the
   dock back to the default and opens the closed panels
   (`AppState::reset_panels`). On a phone (simple mode) the View tab is not
   on the ribbon; the Options are in the File menu.
4. **A list in a dropdown did not open** (the Machine and Hoop lists in
   Design > Machine and Hoop when that panel folds on a narrow screen;
   also on the desktop).
   egui keeps one open popup in its memory, so the list closed the folded
   panel, and the list did not show. The folded panels and the dropdowns of
   the ribbon are now `widgets::holding_popup`: they keep their own open
   state, and a click in any popup (the panel or a list in it) keeps them
   open. The panels of a tab over the canvas (a phone held across) stay
   open in the same way.
5. **The ruler menu.** A right click on a ruler (a long press on a touch
   screen) that is not on a marker opens the menu of that place: Add Guide
   at (X or Y from the hoop center; the top ruler gives a vertical guide),
   Construction Line, Construction Point (the tools) and Remove Guides
   (`guides::ruler_menu`, `AppState::ruler_menu`).
6. **Next or Finish.** The touch bar shows Next when Enter goes on to the
   next part of the tool (side B of a column, the angle lines of a turning
   fill, the holes of a complex fill or an appliqué) and Finish on the
   last part (`Tool::enter_goes_on`).
7. **Sketch dimensions on a touch screen.** A finger picks a point or a
   curve from 18 points (a mouse from 7: `sketch::pick_px`). The sketch
   touch bar (`sketch::touch_bar`) has Length when a line is picked (its
   length dimension at once, with no second tap), Add Dimension when a
   dimension waits for the place of its label (it goes to its default
   place), Cancel, and Done and Cancel while a chain of lines or a spline
   has points. Then the value box opens as before.

Tests: `ribbon::tests::a_list_in_a_folded_panel_opens_and_picks` (it
fails with the old popup), `guides::tests::a_right_click_on_a_ruler_adds_a_guide_there_and_starts_the_mark_tools`,
`draw::tests::the_touch_bar_says_next_until_the_last_part` and
`sketch::tests::on_a_touch_screen_the_bar_makes_a_dimension_and_picks_are_wider`;
they find the buttons in the tree of the screen reader
(`Harness::reader_nodes`, `Harness::widget`).

## The loader (the owner, 2026-11-22)

While the program downloads, the page shows the logo (`logo.svg`: the
logo of `data/icon/icon.svg` with no tile, made by `tools/build-web.py`)
in the middle: a dim logo, and the bright logo over it in a sweep from
12 o'clock round the middle of the hoop, as far as the download has come
(`?loader=up`: a line from the bottom to the top). The page reads the
program through a stream that counts its bytes against the size of the
unpacked program (the build writes it into the page; the Content-Length
of a packed file is smaller), and gives the stream to the browser as a
response, so the browser still compiles during the download. A browser
that does not compile such a response gets the file again from its
cache. Then "Starting…" (the logo breathes), and the loader fades out
when the program runs (`web::start`). Checked in a headless browser with
a download of 6 MB a second; not on a phone yet.

## Faults that the trial found (fixed in the branch)

In a browser, these calls stop the program: `std::time::Instant::now`
and `SystemTime::now` (the clock), `std::thread::spawn` and
`std::thread::scope` (threads), and `std::process::id` (the backups). The
dialogs of `rfd` that wait for the user do not exist in the browser.
`thread::Builder::spawn` gives an error, and the places that use it went
on with no thread already (the stitch plan, the tiles, the checks, the
wizard).

## Not in the trial, and not tested

- Not in the trial: the file dialogs other than Open, Save and Export
  (artwork import, backdrop, fonts and alphabets, assets and packages,
  thread charts, Output Design, the pictures and the PDF sheets, Send to
  USB), My Hoops and My Machines (not kept: the storage layer has only the
  settings), the backups, the design library, the fonts of the computer
  (so no Japanese lettering: the Japanese font package is added with a
  dialog), the update check and the thumbnails.
- Not tested: Firefox, Safari, Chrome on another computer, a slow
  computer, a tablet, a phone, touch, and a first visit over the network.

## What step 2 needs (an estimate after the trial)

The plan calls step 2 M. After the trial it is L to XL:

1. **Work off the screen thread.** The tiles are made in steps now (see
   above); the lay of the surface and the routes of new kanji still stop
   the page for up to about a second. Threads of WebAssembly need a
   shared memory, which needs the page to be cross-origin isolated
   (GitHub Pages cannot set it; a service worker can), and a standard
   library built with atomics, which today needs a nightly compiler.
   The other way is to make the heavy jobs (the lay, the tiles, the
   routes) in steps across frames, or in a second copy of the program in
   a web worker that gets its inputs as messages.
2. **A storage layer** for about 300 file calls: My Hoops, My Machines,
   the backups, the fonts and the asset packages in the storage of the
   browser (the settings are there already).
3. **The other file dialogs** (about 45 places in the code) as uploads
   and downloads (or the File System Access API, which only Chrome and
   Edge have).
4. **A smaller first visit:** the fonts load when a design needs them, and
   `wasm-opt` makes the code smaller.
5. **Memory:** a limit for large designs, so a tablet does not stop the
   page.

## Recommendation

Keep the trial on its branch and do not merge it into `main` yet. It
shows that the program runs in a browser with the same stitches, and that
small designs are quick. Large designs and the 3D look need work off the
screen thread first. Start step 2 after the feedback of users on v0.4, as
the roadmap says; the first part of it is the choice of item 1.
