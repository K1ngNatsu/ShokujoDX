# ShokujoDX

**A free, open-source machine embroidery digitizer for Windows, macOS and
Linux.**

Draw shapes, type lettering or start from a picture. ShokujoDX makes the
stitches and writes the file for your embroidery machine.

The name: Shokujo (織女, "the weaving woman") is the Weaver Star (Vega) of
the Tanabata legend, who weaves the finest cloth of the sky. On Tanabata
people asked her for skill in weaving and needlework. DX is for deluxe.

The goal is the workflow of professional embroidery software, in a
standalone desktop program that is easy to learn.

> **Status:** first public version (0.1.0). The roadmap phases P1 to P14
> are done in software. Many sew-out tests on real machines and the tests
> with users are still open, so test each design on scrap fabric first.

## Features

**Digitizing**
- Running, satin and fill (tatami) stitches, drawn with corner and curve
  points, by hand, or as rectangles, ellipses, stars and rings.
- Satin columns by two sides, by pairs of points, or at a fixed width;
  turning fills, stitch angles and direction lines.
- Appliqué with placement, tack-down and cover steps, up to four layers,
  and cutter files for a fabric cutter.
- Reshape nodes, knife, holes, join and break apart, simplify, redwork and
  branching, and a full sew order toolset (shortest order, fewer color
  changes, cap order).

**Stitch types and effects**
- More than 25 fill types: contour, spiral, radial, Florentine, motif,
  program split, stipple, meander, circular, guided, tartan, gradient,
  lace mesh and more.
- Column types such as zigzag, E stitch, raised satin, coil and motif
  columns; motif runs and ripple lines.
- Effects: accordion spacing, jagged edge, hand stitch, trapunto, cording,
  corners, color blends, buttonholes and decorative borders.

**Lettering**
- Thirty bundled embroidery fonts, and TrueType or OpenType fonts made
  into satin and fill letters (or converted to embroidery fonts).
- Text on arcs, circles and lines, envelopes, and single letters moved,
  turned or reshaped by hand.
- Monograms, team names, keyboard designs, your own alphabets, and a
  lettering kiosk for shops.

**Pictures and artwork**
- Picture to design: a wizard that removes the background, finds the
  colors and makes the stitches, or makes a photo stitch or cross stitch.
- Trace a picture to vectors, or click a color area to stitch it.
- Import SVG, PDF, AI, EMF and WMF artwork; shaping (weld, intersect,
  flatten, divide), offsets and border traces.

**Specialty**
- Cross stitch with printed charts, rhinestones with cutter templates,
  free-standing lace, quilting (echo, stipple, quilt blocks), puff (3D
  foam), laydown for towels, and cutwork needles.

**Production**
- Machine profiles and hoops, auto hoop, turned hoops and multi-position
  hoops; designs larger than the hoop in hoopings, with registration marks.
- Thread charts of 75 brands, colorways, thread use and cost.
- A stitch player, a stitch cursor, a density map and design checks.
- Output in one step: machine files, worksheets and approval sheets (PDF),
  pictures with a 3D thread look, and Send to USB.
- A design library with search and a printed catalog, clipart, templates
  and styles.

**Shop tools**
- Quotes with quantity prices, setup and rush, and the run time on
  machines with more heads.
- Order details (customer, garment, sizes, due date) on the worksheets,
  with a barcode.
- A thread stock, with the threads to buy for an order.
- Scripts (Rhai) for your own tools, with samples.

**Sketch mode**
- Draw with exact sizes, as in a CAD program: lines, arcs, circles and
  splines with constraints and dimensions, and a solver that keeps them
  true when you drag.
- Trim, extend, fillet, chamfer, offset, mirror and patterns; parameters
  and expressions with units (`width = 60 mm`, `2 in`).
- The regions of a sketch become fills, satins and appliqué that change
  when the sketch changes. Vectors, SVG and DXF drawings become sketches.

**The program**
- A ribbon of commands by task, command windows that dock at the canvas
  corners, simple mode, dark and light mode, and automatic backups.
- English, German, Spanish and Japanese, with screen reader names, text
  size and high contrast.
- Help in the program: 38 topics, seven tutorials, and a search that
  answers questions (no network needed). Push F1 on any button.
- Pictures of designs in the file manager (Windows Explorer, macOS Finder,
  Linux file managers).

## File formats

| | Formats |
| --- | --- |
| **Write** | DST, VP3, PES, PEC, EXP, JEF, XXX, TBF, TAP, U01 |
| **Read** | All of the above, plus SEW, HUS, SHV, DSB, DSZ, 10O, 100, DAT and 22 more |
| **Project** | `.embproj` (all objects and settings, to change later) |
| **Other** | SVG, PDF, AI, EMF and WMF artwork in; SVG, EMF, DXF, PDF, PNG and CSV out; INF, EDR and COL thread color files |

See [docs/help/file-formats.md](docs/help/file-formats.md) for the full
list.

## Install

Download the file for your system from the
[Releases page](https://github.com/K1ngNatsu/ShokujoDX/releases).

| System | File | What to do |
| --- | --- | --- |
If the Releases page has no files for your system yet,
[build the program from source](#build-from-source).

| Windows 10 or 11 (64-bit) | `ShokujoDX-…-windows-setup.exe` | Start it and follow the steps. No administrator rights needed. |
| Windows, no install | `ShokujoDX-…-windows-portable.zip` | Unzip the folder anywhere and start `ShokujoDX.exe`. |
| macOS 10.15 or later (Apple silicon or Intel) | `ShokujoDX-…-macos.dmg` | Open it and drag **ShokujoDX** to **Applications**. |
| Debian, Ubuntu, Linux Mint (64-bit) | `shokujodx_…_amd64.deb` | `sudo apt install ./shokujodx_…_amd64.deb` |
| Other Linux (64-bit) | `ShokujoDX-…-linux-x86_64.AppImage` | `chmod +x ShokujoDX-…-linux-x86_64.AppImage`, then start it. |

`SHA256SUMS.txt` on the release page has the checksum of each file.

**The first start.** The program has no paid code signature yet, so the
system can warn you the first time:

- Windows: "Windows protected your PC". Click **More info**, then
  **Run anyway**.
- macOS: "cannot be opened" or "cannot check". Open **System Settings >
  Privacy & Security** and click **Open Anyway** (on older macOS:
  Control-click the app and click **Open**).

**Uninstall.** Windows: **Settings > Apps** (installer) or delete the
folder (zip). macOS: drag the app to the Trash. Linux:
`sudo apt remove shokujodx`, or delete the AppImage. Your
settings stay in `%APPDATA%\ShokujoDX` (Windows),
`~/Library/Application Support/ShokujoDX` (macOS) or
`~/.config/shokujodx` (Linux); delete that folder too if you want.

## First design

1. On the **Design** tab, select your machine and your hoop. The machine
   sets the file format.
2. Draw a shape on the **Digitize** tab, type text on the **Lettering**
   tab, or start from a picture with **Artwork > Picture to Design**.
3. Click **Design > Output Design** to write the machine file (and a
   worksheet), or **Send to USB**.
4. Save the project (**Ctrl+S**, `.embproj`) to change the design later.
   The machine file cannot be changed well.

Push **F1** in the program for help on any button or window. The help has
seven step-by-step tutorials. Test each new design on scrap fabric first.

## Build from source

You need [Git](https://git-scm.com) and [Rust](https://rustup.rs) 1.95 or
later. On Linux, also install the libraries of the window system:

```bash
sudo apt-get install libxcb-render0-dev libxcb-shape0-dev \
  libxcb-xfixes0-dev libxkbcommon-dev libssl-dev
```

Get the source:

```bash
git clone https://github.com/K1ngNatsu/ShokujoDX.git
cd ShokujoDX
```

Run the program (the first build takes some minutes):

```bash
cargo run -p emb-app --release
```

Open a design or a machine file at start:

```bash
cargo run -p emb-app --release -- path/to/design.embproj
```

The program reads `data/` (hoops, machines, fabrics, thread charts, fonts
and translations) next to the program or in the working folder, so you can
change the presets without a new build.

### Packages

```powershell
powershell -ExecutionPolicy Bypass -File tools\package-windows.ps1 -Installer
```

```bash
tools/package-macos.sh
tools/package-linux.sh
```

The packages go to `dist/`. The Windows installer needs
[Inno Setup 6](https://jrsoftware.org/isinfo.php). Without `-Installer`,
the Windows script makes a test zip with the tester guide and a feedback form (see
[docs/testing/](docs/testing/)).

## Command-line tool

`emb-cli` does the work of the program with no window, for scripts and
batch work. The packages do not include it: build it from source with
`cargo build --release -p emb-cli` (the program is then in
`target/release/`).

| Command | What it does |
| --- | --- |
| `convert <in> <out>` | Stitch a design or read a machine file, and write a machine file, JSON or SVG |
| `stitch <in>` | Print the stitch summary, thread use and density warnings |
| `output <in>... --dir <dir>` | Batch output: machine files, worksheets, pictures |
| `library <folder>...` | Search design folders; write a design list or a catalog |
| `cutter`, `bling`, `chart` | Appliqué cutter files, rhinestone templates, cross-stitch charts |
| `font <font.ttf> <out.embfont>` | Make an embroidery font from a TrueType font |
| `quote <in>` | The price and the run time of an order |
| `script <script.rhai> <in>...` | Run a script on designs |
| `fabrics` | List the fabric presets |
| `bench`, `corpus` | The picture bench and the test corpus |

```bash
cargo run -p emb-cli --release -- convert design.embproj design.vp3
cargo run -p emb-cli --release -- --help    # all commands and options
```

## Project layout

| Crate | Purpose |
| --- | --- |
| `emb-geom` | 2D geometry: paths, polygons, booleans, offsets |
| `emb-model` | The design model, undo and redo, hoops, machines, fabrics, the project format |
| `emb-stitch` | Stitch generation: all stitch types, underlay, routing, sew order |
| `emb-formats` | Machine file formats and artwork import |
| `emb-render` | Stitch pictures, the 3D thread look, PDF output |
| `emb-letter` | Fonts, letters and text layout |
| `emb-art` | Pictures: tracing, background removal, Picture to design |
| `emb-library` | The design library and thumbnails |
| `emb-script` | Scripts in Rhai |
| `emb-sketch` | Parametric sketches: constraints, the solver, sketch edits |
| `emb-cli` | The command-line tool and the test corpus |
| `emb-app` | The desktop program (egui) |
| `emb-thumbnail` | The Windows thumbnail handler for Explorer |

The engine crates have no UI and no C or C++ dependency.

## Documentation

- [Help topics](docs/help/): the user guide that the program shows.
- [Specification](docs/SPEC.md): the scope, the rules of the project and
  the phases.
- [Roadmap](docs/design/roadmap.md): the phases, their status and the open
  tests.
- [Architecture decisions](docs/adr/): one short record for each decision.
- [Tester guide](docs/testing/TESTERS.md).

## Trademarks

ShokujoDX is not affiliated with, endorsed by, or connected to any commercial
embroidery software company or machine maker. Other product and company names in this
repository are trademarks or registered trademarks of their respective
owners. They are named only to describe facts (a file format, a machine, a
thread brand, or a public source of information), not to suggest approval
or partnership. A machine or format name tells which files the program reads
and writes; it does not mean that the maker tested or approved the program.
ShokujoDX is an independent clean-room project: no code,
assets or file samples come from closed-source embroidery software, and the
documentation does not name the commercial programs that it compares features with.

## Contributors

- **[@K1ngNatsu](https://github.com/K1ngNatsu)**: the owner. The
  idea, the scope and the decisions, and the sew-out tests on a real
  machine.
- **Evan Rush** ([@rushyrush](https://github.com/rushyrush)): the review of
  the names and trademarks of other products, the trademark notes, and the
  list of the bundled fonts and thread charts.
- **Claude** (Anthropic, through [Claude Code](https://claude.com/claude-code)):
  most of the code, the tests and the documentation, made at the request
  and under the direction of the owner.

## Contributing

Bug reports, sew-out results, translations and code are welcome. Read
[CONTRIBUTING.md](CONTRIBUTING.md) first. The most important rule: do not
decompile, disassemble or copy code, assets or sample files of
closed-source embroidery software.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE). The licenses of the bundled
fonts and the third-party crates are in [data/licenses](data/licenses) and
next to each font in `data/fonts`.
