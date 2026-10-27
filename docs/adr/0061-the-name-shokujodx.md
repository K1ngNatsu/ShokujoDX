# ADR 0061: The name ShokujoDX

Date: 2026-10-26 · Status: Accepted (owner decision). Supersedes ADR 0059.

## Context

Before the repository went public, a search found a second free
embroidery program with the name Ariadne ("Ariadne Embroidery Studio",
`Nam3less-One/ariadne-embroidery`, public since 2026-09-12). It uses the
same shown name, and its package name `ariadne-embroidery` is the same as
our technical id. A deeper search found more problems:

- Ariadne GmbH has a registered EU trademark ARIADNE (019318528) for
  software in general, with an international registration and a US
  application.
- Many other live ARIADNE marks for software exist (Germany, Benelux,
  Italy, Greece, Switzerland, the UK, Korea).
- "ariadne" is a popular Rust crate and Python library, so the program
  would be hard to find.

The owner wanted to keep a name from myth. The checks of the other names
(GitHub, PyPI, crates.io, npm, the .com domain, TMview, J-PlatPat and the
web) are in the conversation of 2026-10-26; in short:

- Ergane: a new Swiss trademark for software for numerically controlled
  machines, and other software filings.
- Arachne, Mokosh, Clotho, Daedalus, Orihime, Clew, Tsuru: live software
  trademarks or many projects of the same name.
- AriadnEX: one letter from the registered French software mark AriadNEXT,
  and it still contains ARIADNE.
- Shokujo: no trademark in software, sewing machines or embroidery. The
  marks near it are a towel brand (Imabari Shokujo) and one Japanese mark
  for hotel furniture and fabrics (織女, ショクジョ). China's register was
  not searched (its search needs a person for a CAPTCHA).

Shokujo (織女, "the weaving woman") is the Weaver Star (Vega) of the
Tanabata legend. On Tanabata, people asked her for skill in weaving and
needlework, so the name fits an embroidery program. The owner chose
**ShokujoDX** (DX: deluxe).

## Decision

1. The shown name is **ShokujoDX**: the window title, About, the start
   page, the crash report, the PDF producer, the installers, the file type
   names and the help.
2. The technical id is **`shokujodx`**: the Linux program, package, app id
   and files in `packaging/linux`, the share folder, and the macOS ids
   `org.shokujodx.app`, `.app.thumbnails`, `.project` and `.stitches`.
3. Windows: `ShokujoDX.exe` in `Programs\ShokujoDX`, the ProgIDs
   `ShokujoDX.Project` and `ShokujoDX.MachineFile`, the packages
   `ShokujoDX-<version>…`. The installer AppId and the CLSID of the
   thumbnail handler do not change.
4. The settings folder is `ShokujoDX` (Linux: `shokujodx`). A folder of an
   earlier name moves to it once (`workspace::program_folder`):
   `Ariadne` (Linux: `ariadne-embroidery`), then `emb-digitizer`. Other
   programs can use the name Ariadne, so an `Ariadne` folder moves only if
   its `settings.json` is ours.
5. The project file stays `.embproj`, and the crates stay `emb-*`.
6. The GitHub repository is renamed to `K1ngNatsu/ShokujoDX`.
7. The icon (`tools/make-icon.py`) shows the Weaver Star: a gold star in
   satin stitches at the middle of dark fabric in a wooden hoop, a needle
   that sews in the fabric across the star (it goes down a little before
   the star and comes up a little after it) with a gold thread from its
   eye to a point of the star, and a night sky with the Milky Way and a small
   Altair (the cowherd star of the legend) across it. The owner's sketch
   set the layout.

## Consequences

- Before the program is sold or licensed commercially, search China's
  register for 织女 and get a full clearance search from a trademark
  lawyer (docs/SPEC.md, the open questions).
- ADR 0059 and the plans keep the names of their time.
