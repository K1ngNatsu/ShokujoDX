# Plan: polish and lettering (phase P17)

Date: 2026-11-18 · Status: Accepted (owner, 2026-11-18). Milestones 1 (ADR 0109) and 2 (ADR 0110) done; the owner checks the exit test of Milestone 1 with the next release.

## Goal

The program has a wide set of features (phases P1 to P16). The owner
wants the features that it has to become better before it gets new ones
(2026-11-18). A review of the same day ("Review of 2026-11-18" in
`docs/design/roadmap.md`) found where the program is behind the reference
product in depth: lettering fonts and content, proof on an industrial
machine, and trust in a new release.

This phase has eight milestones. Each one is small, can be released
alone, and keeps the projects of earlier versions.

What stays:

- No new feature groups in this phase. A problem that a tester reports
  comes before a milestone of this plan.
- The fonts, motifs and patterns are our own work or come from open
  sources with a license that permits it (rule 3).
- The program stays small: content that few users need is an asset
  package (My assets), not a part of the program.

## The testers

- About five users tried the program before 2026-11-18. Two of them sent
  files to their machines, and the files sewed well.
- The owner has an industrial multi-needle machine (a Tajima with 12
  needles, all its hoops and a cap frame) for tests from 2026-11-18.
- Professional users will do their daily work in the program in place of
  a consumer embroidery product, and report where they cannot continue.

## Milestone 1: update check (S)

A user of an old version learns that a new version exists.

1. **The check.** At the start, and not more than one time each day, the
   program asks GitHub for the newest release of the program. If the
   release is newer, a notice shows its version, its notes and a link to
   the download page. The program does not download and does not install
   a file.
2. **The choice of the user.** The Windows installer has a tick box
   (automatic checks: yes or no). The first-start window asks the same
   question on the systems and packages that have no installer. Options
   has the setting, and **Help > Check for Updates** checks at once.
3. **Privacy.** The request sends no data of the user and no design data.
   The install text and the help say what the check does.
4. **What is new.** After the first start of a new version, the start
   page shows a short list of the changes.

The check is the first use of the network in the program. The engine
crates get no network code.

**Done (2026-11-18), ADR 0109.** `emb-app/src/update.rs`. The `curl`
program of the system makes the request, so the program gets no network
library and `Cargo.lock` does not change. The installer writes the
choice to `install.ini` next to the program; a choice of "no" always
applies, and "yes" does not turn on checks that the user turned off.
A user of an earlier version gets one question at the next start. The
list of changes is the help topic **What is new**, with a link on the
start page at the first start of a new version. The installer script
was not compiled on the development PC (no Inno Setup there): the
release build in CI compiles it.

**Exit test:** a build with an older version number shows the notice;
with automatic checks off, the program makes no request (a test checks
that the start makes none).

## Milestone 2: pictures of the windows (S, development only)

Each window of the program is checked in each language.

1. A test opens each window and panel of the program in English, German,
   Spanish and Japanese, at two window sizes, with no screen, and writes
   a picture of each one to the `target` folder.
2. The test fails when a window gives an error. It does not compare
   pictures.
3. A person (or the assistant) looks at the pictures before a release.

All of it is test code (`#[cfg(test)]`) with a development dependency.
No code and no data of it are in the program that users get, and the
pictures are not in the repository.

**Exit test:** the pictures of all windows in four languages are made in
less than five minutes; a text that does not fit its button is found and
corrected.

**Done (2026-11-18), ADR 0110.** `emb-app/src/pictures.rs` (test code
only). A small software renderer makes the pictures from the triangles
of egui, so the test has no new dependency and needs no graphics card.
The run makes 520 pictures of 378 scenes (264 screens, 64 windows in
four languages) in about 90 seconds:

```text
cargo test --release -p emb-app -- --ignored write_window_pictures --nocapture
```

`EMB_PICTURES_ONLY=word` and `EMB_PICTURES_LANGS=en,ja` make a part of
them. Open `target/window-pictures/index.html`, and read `report.txt`.

Found and corrected with the first pictures:

- The Options window was 994 points high and had no scroll bar: on a
  laptop screen (768 points) the last rows and **Reset to defaults**
  were out of reach, in each language. It now has the height of the
  screen and scrolls.
- English texts in the other languages: "(mixed)" in Object Properties,
  "1 object" and "2 objects" in the Threads panel, "lines" in Options,
  "Untitled" on the design tab and in the window title, a notice title,
  and the names and descriptions of the fabrics ("Not set", "Towel
  (terry)" and the others) in the Styles panel, Design info and the
  Fabric window.
- Rows that were cut at the edge of a narrow panel now go on in a new
  line: Mirror and Center in Hoop (Object Properties), the colorway row
  (Threads), the current properties (Styles).

Found and left open:

- At 1366 points, the titles of the docked panels do not all fit in
  Spanish, German and Japanese; the dock shows a scroll bar for them.
- Japanese: the characters of the font of the computer sit higher than
  the Latin letters and digits in the same line. A correction needs the
  eye of a native reader, for the fonts of each system.
- Japanese: two lines of the worksheet (Print Preview) are longer than
  their column.
- The Options window is 230 to 270 points wider in German and Spanish
  (it fits the screen).
- Windows with no scene yet: the Lettering window, Picture to design
  with a picture, the Turntable and Product Photo with a product.

## Milestone 3: tests on an industrial machine (M)

The stitches and the files are correct on a multi-needle machine.

1. **A test kit** (`out/p17-industrial`, DST and TBF, 12 needles): the
   stitch types on one sheet, steps of density and pull compensation,
   small lettering (4 to 8 mm), trims and jumps (the trim code of DST),
   long stitches and auto split, the needle plan with its stops, a design
   at the speed of the machine, and a cap design in cap order on the cap
   frame.
2. **A report form** for testers (`docs/testing`): what the tester made,
   on which machine, what was wrong, and where the tester could not
   continue. A report has no name of a closed-source program in the
   repository.
3. **Corrections.** Each problem of the kit or of a report gets a
   correction and a test. The default values of the industrial machine
   profiles change when the sew-outs show better ones.

**Exit test (owner):** the kit sews correctly on the Tajima; the cap
design sews correctly on the cap frame (this also closes the cap test of
P11); the reports of the professional testers have no open problem that
stops their work.

## Milestone 4: font test sheet and letter corrections (M)

A font can be checked on a machine and corrected for good.

1. **Font test sheet.** Lettering > Fonts > Test Sheet makes a design
   with all letters, digits and common marks of a font at its smallest,
   middle and largest size, with the size next to each row.
2. **Letter corrections.** A correction file for each bundled font keeps
   the changes of single letters: the order of the strokes, the entry and
   exit, the joins, the stitch angle, the underlay, the pull compensation,
   and a new shape for a letter (a user-refined letter, P4 Milestone 7).
   `tools/make-embroidery-fonts.py` applies the corrections after the
   conversion, so a new conversion keeps them.
3. The Alphabets window writes a correction from a change that the user
   makes to a letter.

**Exit test:** a corrected letter stays corrected after the fonts are
made again; the owner sews the test sheet of one font.

## Milestone 5: ten tuned fonts (L)

Ten fonts are as good as fonts that a person digitized.

1. The ten fonts: a block sans, a serif, a script, a font for small text,
   an athletic (varsity) font, a narrow font, a round font, a slab, a
   brush font and a monogram font.
2. Each letter of each font is checked on the screen and on the test
   sheet, and corrected (Milestone 4).
3. The Fonts window marks these fonts as **Tuned** and shows them first.

**Exit test (owner):** the test sheet of each of the ten fonts sews
correctly on the Topaz 20 and on the Tajima, at the three sizes.

## Milestone 6: text shaping (M)

Lettering is correct for the writing systems that need shaping.

1. `emb-letter` shapes the text with a pure-Rust shaping library before
   the layout: ligatures, the forms of a letter that depend on its place,
   marks, and text from right to left.
2. The stroke finder and the sew order work on the shaped glyphs. The sew
   order of right-to-left text follows the reading direction.
3. One bundled font each for Arabic and Devanagari (OFL), with the
   license text.

An ADR records the library (rule 9: no C library).

**Exit test:** a test text in Arabic, in Devanagari and in a Latin script
font with ligatures gives the glyphs of a reference shaping; the owner
sews one word of each.

## Milestone 7: more fonts, with a quality score (M)

The user sees how good each font is before use.

1. **Quality score.** For each font: the part of each letter that the
   stroke finder covers (ADR 0100), columns that are too narrow at the
   smallest size, and stitches that are too long at the largest size.
   The Fonts window shows the score and can sort by it.
2. **More fonts.** About 100 fonts from OFL sources. The fonts with the
   best score are in the program; the others are font packages (My
   assets). A font with a low score is not included.

**Exit test:** each bundled font has a score above the limit; the size of
the program grows by less than 10 MB.

## Milestone 8: content families (M)

The libraries of motifs and patterns are large enough for daily work.

1. **Families from parameters.** A motif, a split pattern, a stamp or a
   border is made from a small description with parameters (for example
   the number and the height of the waves), so one description gives many
   items.
2. **Targets:** 60 motifs, 40 split patterns, 30 carving stamps, 40
   decorative borders and 40 clipart items, all our own.
3. Content that few users need is an asset package.

**Exit test (owner):** a sew-out of five new items of each kind.

## Not in this phase

- More formats to write: all machines of the machine library have a
  format that the program writes. A format is added when a tester has a
  machine that needs it.
- Sequins, beads, chenille and Schiffli: they wait for testers (P9).
- Features for designs of other programs (phase P18) and the help and
  tutorials (phase P19).
