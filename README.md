<div align="center">

<img src="data/icon/icon-256.png" alt="ShokujoDX icon: a purple star in an embroidery hoop with a needle and a thread" width="160">

# ShokujoDX

**Free, open-source embroidery digitizing for Windows, macOS and Linux.**

Draw a shape, type a name or start from a picture.<br>
ShokujoDX makes the stitches and writes the file for your machine.

[![Latest release](https://img.shields.io/github/v/release/K1ngNatsu/ShokujoDX?label=download&color=2b6cb0)](https://github.com/K1ngNatsu/ShokujoDX/releases/latest)
[![CI](https://github.com/K1ngNatsu/ShokujoDX/actions/workflows/ci.yml/badge.svg)](https://github.com/K1ngNatsu/ShokujoDX/actions/workflows/ci.yml)
[![License: GPL v3](https://img.shields.io/badge/license-GPL--3.0-blue)](LICENSE)
![Windows | macOS | Linux](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)

[**Download**](https://github.com/K1ngNatsu/ShokujoDX/releases/latest) ·
[**Try it in your browser**](https://k1ngnatsu.github.io/ShokujoDX/) ·
[Features](#features) ·
[Get started](#get-started) ·
[Help](docs/help/) ·
[Contribute](CONTRIBUTING.md)

</div>

---

## Why ShokujoDX

- **Professional tools, no price tag.** The full workflow of a commercial
  digitizer: all the stitch types, lettering, Picture to design,
  multi-hoop layouts, production sheets and shop tools.
- **Easy to learn.** A ribbon of commands by task, a simple mode with large
  buttons, help on every button (push **F1**) and seven step-by-step
  tutorials, in English, German, Spanish and Japanese.
- **Tested on real fabric.** The test designs of the first 14 roadmap
  phases sew correctly on a Husqvarna Viking Topaz 20.
- **Your files stay yours.** No account and no subscription. The program
  uses the network for one thing only, and only if you permit it: it can
  ask GitHub if a new version is available (no data about you or your
  designs is sent, and nothing is downloaded or installed). The
  project file keeps every object editable, and the program writes the
  formats of all the large machine brands.

## Features

**Digitize**
- Running, satin and fill stitches, drawn point by point, by hand, or as
  rectangles, ellipses, stars and rings.
- Satin columns, turning fills, stitch angles, appliqué with cutter files,
  and a full set of reshape and sew-order tools.

**Stitch types and effects**
- More than 25 fill types (contour, spiral, radial, motif, stipple,
  meander, tartan, gradient, lace mesh and more) and many column types.
- Effects such as raised satin, trapunto, cording, color blends,
  buttonholes and decorative borders.

**Lettering**
- 65 embroidery fonts, each with a quality score, and more in font
  packages; plus the TrueType and OpenType fonts of your computer.
- Text shaping: ligatures, the joined letters of script fonts, Arabic and
  Devanagari.
- Japanese lettering: kana and kanji as satin columns that follow the
  strokes, with few trims; two Japanese embroidery fonts (the 2,136 jōyō
  kanji) in a font package.
- Bold, italic, underline and stacked text; text on arcs, circles and
  lines; monograms, team names and a lettering kiosk for shops.

**Pictures and artwork**
- **Picture to design:** a wizard that removes the background, finds the
  colors and makes the stitches; also photo stitch and cross stitch.
- Trace pictures, and import SVG, PDF, AI, EPS, EMF and WMF artwork.
- **Designs of other programs:** Recognize Objects makes the stitches of a
  machine file into runs, satin columns and fills again (also with the
  outlines of the design); read and write Ink/Stitch files, and convert its
  fonts.

**Parametric sketch**
- Draw with exact sizes, as in a CAD program: constraints, dimensions and
  parameters (`width = 60 mm`).
- The regions of a sketch become fills and satins that change with it.

**See it before you sew**
- A 3D stitch view: the thread with its height, its sheen and its shadows
  on the weave of the fabric; a view from an angle, a magnifier, and
  checks for fabric that shows through and for thread that is too high.
- The design on a product (shirts, caps, bags and towels, or a photo of
  your own product): front and back, two colors, a turntable, and a page
  on the approval sheet.

**Production and shop**
- A machine library of 41 profiles, your own machines, and the standard
  hoops; designs larger than the hoop in hoopings with registration marks.
- Thread charts of 75 brands, a stitch player, a density map and design
  checks.
- Output in one step: the machine file, worksheets and approval sheets
  (PDF), a 3D thread picture, and Send to USB.
- Quotes and run times, order details with a barcode, thread stock and
  scripts.

**Specialty**
- Free-standing lace, quilting, puff (3D foam), cross stitch, rhinestones,
  cutwork and towel designs.

## Machines and formats

ShokujoDX knows the embroidery area, the hoops and the file format of
home machines of **Brother, Baby Lock, Janome, Elna, Husqvarna Viking,
Pfaff, Singer, Bernina and bernette**, and of the multi-needle machines of
**Tajima, Happy, Toyota, Ricoma, BAI, Highland and Barudan**. A generic
profile for each file format covers the other machines, and you can add
your own machine and share it.

| | Formats |
| --- | --- |
| **Write** | DST, VP3, PES, PEC, EXP, JEF, XXX, TBF, TAP, U01 |
| **Read** | All of the above, plus SEW, HUS, SHV, DSB, DSZ, 10O, 100, DAT and 22 more |
| **Project** | `.embproj` (every object and setting, to change later) |
| **Artwork** | SVG, PDF, AI, EPS, EMF and WMF in; SVG, EMF, DXF, PDF, PNG and CSV out |
| **Ink/Stitch** | SVG files with Ink/Stitch settings in and out; its fonts in |

The full list is in [the file formats help](docs/help/file-formats.md).

## Get started

### Try it in your browser

No install: open [ShokujoDX in your browser](https://k1ngnatsu.github.io/ShokujoDX/) on a
computer, a tablet or a phone. It is a trial of the browser version:
**File > Open** uploads a design and **Save** and **Export** download a
file, your designs stay on your device, and some commands need the
desktop program. The first visit downloads about 35 MB, then the browser
keeps it. On a phone or a tablet, **Add to Home Screen** makes it an app
that also works offline.

### Install

Get the file for your system from the
[latest release](https://github.com/K1ngNatsu/ShokujoDX/releases/latest).

| System | File | Install |
| --- | --- | --- |
| Windows 10 or 11 (64-bit) | `ShokujoDX-…-windows-setup.exe` | Start it and follow the steps. No administrator rights needed. |
| Windows, no install | `ShokujoDX-…-windows-portable.zip` | Unzip the folder anywhere and start `ShokujoDX.exe`. |
| macOS 10.15 or later | `ShokujoDX-…-macos.dmg` | Open it and drag **ShokujoDX** to **Applications**. |
| Debian, Ubuntu, Linux Mint | `shokujodx_…_amd64.deb` | `sudo apt install ./shokujodx_…_amd64.deb` |
| Other Linux (64-bit) | `ShokujoDX-…-linux-x86_64.AppImage` | `chmod +x` the file, then start it. |

The program has no paid code signature yet, so the system can warn you
the first time. On Windows, click **More info**, then **Run anyway**. On
macOS, open **System Settings > Privacy & Security** and click **Open
Anyway**.

### Your first design

1. **Pick your machine.** The first start asks for your machine and your
   hoop, so the program writes the file that your machine reads.
2. **Make the design.** Draw on the **Digitize** tab, type on the
   **Lettering** tab, or start from a picture with **Artwork > Picture to
   Design**.
3. **Sew it.** Click **Design > Output Design** for the machine file and a
   worksheet, or **Send to USB**.
4. **Keep it.** Save the project (**Ctrl+S**) to change the design later.

Push **F1** on any button for help. Test each new design on scrap fabric
first.

<details>
<summary><b>Upgrade, uninstall and problems at the start</b></summary>

- **Upgrade:** install the new version over the old one. The Windows
  installer removes the older version first; your settings and designs
  stay.
- **New versions:** the installer (or the first start) asks if the
  program looks for a new version when it starts. **Help > Check for
  Updates** looks at once, and **Options** changes the choice. The
  program shows the new version and its download page; you install it
  yourself.
- **Uninstall:** Windows: **Settings > Apps** (or delete the folder of the
  zip). macOS: drag the app to the Trash. Linux: `sudo apt remove
  shokujodx`, or delete the AppImage. The settings stay in
  `%APPDATA%\ShokujoDX` (Windows), `~/Library/Application
  Support/ShokujoDX` (macOS) or `~/.config/shokujodx` (Linux).
- **The program does not start:** it tries the graphics card, then
  OpenGL, then the software renderer, and shows a message if none works.
  Start it with `--diagnose` to write a report (`diagnose.txt` in the
  settings folder) and send it with your feedback. The help topic
  [Questions and problems](docs/help/troubleshooting.md) has more.
- **Checksums:** `SHA256SUMS.txt` on the release page.

</details>

## Project status

Phases 1 to 14 of the roadmap are done, and their sew-out tests passed on
the reference machine, a Husqvarna Viking Topaz 20. Phases 16 to 18 and
20 (the product view, better lettering, designs of other programs, the 3D
stitch view and Japanese lettering) are done in software and are in
version 0.4.0; their tests on fabric and with users are open. Also open: tests with new users, and tests on cap frames,
cutwork machines and other machines that the owner does not have. Sew-out
results from **your** machine help a lot:
see [CONTRIBUTING.md](CONTRIBUTING.md).

The phases and the open tests are in the [roadmap](docs/design/roadmap.md),
and all the open work is in one list:
[open work](docs/design/open-work.md).

## For developers

<details>
<summary><b>Build from source</b></summary>

You need [Git](https://git-scm.com) and [Rust](https://rustup.rs) 1.95 or
later. On Linux, also install the libraries of the window system:

```bash
sudo apt-get install libxcb-render0-dev libxcb-shape0-dev \
  libxcb-xfixes0-dev libxkbcommon-dev libssl-dev
```

Then:

```bash
git clone https://github.com/K1ngNatsu/ShokujoDX.git
cd ShokujoDX
cargo run -p emb-app --release
```

The first build takes some minutes. The program reads `data/` (hoops,
machines, fabrics, thread charts, fonts and translations), so you can
change the presets without a new build.

Packages go to `dist/`:

```powershell
powershell -ExecutionPolicy Bypass -File tools\package-windows.ps1 -Installer
```

```bash
tools/package-macos.sh
tools/package-linux.sh
```

The Windows installer needs [Inno Setup 6](https://jrsoftware.org/isinfo.php).

</details>

<details>
<summary><b>Command-line tool</b></summary>

`emb-cli` does the work of the program with no window, for scripts and
batch work. Build it with `cargo build --release -p emb-cli`.

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

`emb-cli --help` lists all commands and options.

</details>

<details>
<summary><b>Project layout</b></summary>

| Crate | Purpose |
| --- | --- |
| `emb-geom` | 2D geometry: paths, polygons, booleans, offsets |
| `emb-model` | The design model, undo and redo, hoops, machines, the project format |
| `emb-stitch` | Stitch generation: all stitch types, underlay, routing, sew order |
| `emb-formats` | Machine file formats, artwork import, Ink/Stitch files |
| `emb-render` | Stitch pictures, the 3D stitch view, products, sheets and PDF output |
| `emb-letter` | Fonts, letters and text layout |
| `emb-art` | Pictures: tracing, background removal, Picture to design; the objects of a stitch file |
| `emb-library` | The design library and thumbnails |
| `emb-script` | Scripts in Rhai |
| `emb-sketch` | Parametric sketches: constraints, the solver, sketch edits |
| `emb-cli` | The command-line tool and the test corpus |
| `emb-app` | The desktop program (egui) |
| `emb-thumbnail` | The Windows thumbnail handler for Explorer |

The engine crates have no UI and no C or C++ dependency. The
[specification](docs/SPEC.md) has the rules of the project, and
[docs/adr](docs/adr/) has one short record for each decision.

</details>

## Contributing

Bug reports, sew-out photos, translations and code are welcome. Read
[CONTRIBUTING.md](CONTRIBUTING.md) first. The most important rule: do not
decompile, disassemble or copy code, assets or sample files of
closed-source embroidery software.

## About the name

Shokujo (織女, "the weaving woman") is the Weaver Star (Vega) of the
Tanabata legend, who weaves the finest cloth of the sky. On Tanabata
people asked her for skill in weaving and needlework. DX is for deluxe.

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

## License

ShokujoDX is free software under the
[GNU General Public License v3.0 or later](LICENSE). The licenses of the
bundled fonts and the third-party crates are in
[data/licenses](data/licenses) and next to each font in `data/fonts`.
