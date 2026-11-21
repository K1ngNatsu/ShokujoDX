# P15, step 1: the browser trial

Date: 2026-11-21 · Branch: `trial/browser` · Status: Report. The owner
decides on step 2 of P15 (the web version for computers) from it. See
"P15. Browser version" in `docs/design/roadmap.md`.

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
  File gives the machine file as a download. The other file dialogs (about
  37) show a notice that the command needs the desktop program.
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
  USB), the settings, My Hoops and My Machines (not kept: no storage
  layer), the backups, the design library, the fonts of the computer
  (so no Japanese lettering: the Japanese font package is added with a
  dialog), the update check and the thumbnails.
- Not tested: Firefox, Safari, Chrome on another computer, a slow
  computer, a tablet, a phone, touch, and a first visit over the network.

## What step 2 needs (an estimate after the trial)

The plan calls step 2 M. After the trial it is L to XL:

1. **Work off the screen thread.** Without it, the 3D look stops the page
   for seconds, and kanji stop typing. Threads of WebAssembly need a
   shared memory, which needs the page to be cross-origin isolated
   (GitHub Pages cannot set it; a service worker can), and a standard
   library built with atomics, which today needs a nightly compiler.
   The other way is to make the heavy jobs (the lay, the tiles, the
   routes) in steps across frames, or in a second copy of the program in
   a web worker that gets its inputs as messages.
2. **A storage layer** for about 300 file calls: the settings, My Hoops,
   My Machines, the backups, the fonts and the asset packages in the
   storage of the browser.
3. **The 37 file dialogs** as uploads and downloads (or the File System
   Access API, which only Chrome and Edge have).
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
