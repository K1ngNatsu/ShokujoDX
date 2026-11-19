# Plan: polish and lettering (phase P17)

Date: 2026-11-18 · Status: Accepted (owner, 2026-11-18). Milestones 1 (ADR 0109), 2 (ADR 0110), 4 (ADR 0112) and 5 (ADR 0113) done in software; Milestone 3 is set aside by the owner (2026-11-18). The owner checks the exit test of Milestone 1 with the next release and sews the test sheets of Milestones 4 and 5.

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
The run makes 540 pictures of 382 scenes (264 screens, 69 windows in
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

Corrected after that (owner, 2026-11-18; ADR 0111):

- **Japanese in the PDF pages.** The long lines of the Japanese
  worksheet had a larger cause: the PDF writer printed each Japanese
  character as `?` (worksheets, approval sheets, quotes, catalogs). A
  page with Japanese text now has a standard Japanese font, the lines
  break at the characters, and a wide character has its width. The euro
  sign and the typographic quotation marks print too. A Japanese
  worksheet was checked with the PDF engine of Windows.
- **Japanese characters in a line.** The characters of Yu Gothic (the
  Japanese font of Windows) were higher than the Latin letters and
  digits; they now go down by 0.18 of the text size. The fonts of older
  Windows, macOS and Linux have no value yet.
- The name "Untitled" on the sheets is in the language of the program.
- New scenes: the Lettering window, Picture to design with a picture,
  the Product window and the Turntable with a product, Product Photo
  (382 scenes, 540 pictures).

Left as they are:

- At 1366 points, the titles of the docked panels do not all fit (in
  English too); the dock shows a scroll bar for them. This is how the
  dock works; fewer or shorter titles would be a change of the layout.
- The Options window is 230 to 270 points wider in German and Spanish
  (it fits the screen).
- A reader of Japanese must still check the look of the Japanese windows
  and sheets.

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
   middle and largest size, with the size at each group of rows.
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

**Done in software (2026-11-18, ADR 0112).** The owner set Milestone 3
aside and started this one.

- **Font test sheet:** Lettering > Font Test Sheet (also Test Sheet in the
  Fonts window, and `emb-cli font-sheet`). One group for each size, with a
  label (the name of the font and the size) above it, so the rows have the
  full width. The window starts with the sizes whose sheet fits the hoop,
  because all characters at the largest size are some meters of
  lettering; it shows the area of each size. Character sets: letters,
  digits and marks; all characters; capitals; small letters; digits and
  marks; accents; the text of the user.
- **Two changes of the engine** made a correction have an effect. Before,
  the stitch engine found the order and the directions of the parts of
  each letter, and the lettering object gave its settings to all parts.
  Now a letter can be **fixed** (it sews in the order of the font, each
  column from its start to its end), and a part can have its own
  underlay, pull compensation and fill angle.
- **Correction files:** `data/fonts/embroidery/corrections/<name>.json`,
  applied by `tools/make-embroidery-fonts.py` through `emb-cli font
  --corrections`. A correction is a new shape, or steps on the converted
  letter (order, reverse, join). A correction that does not fit a new
  conversion stops the tool. A test fails if a bundled font does not
  have a correction of its file.
- **Alphabets window:** Place Letter puts a letter of an embroidery font
  in the design as objects; Save Correction writes the selected objects
  as its correction, to the fonts folder of the user. The font library
  applies it, also to a bundled font. The file can go into the
  repository as it is.

Checked: all 30 bundled fonts were made again with the new code and are
the same bytes. A trial correction of "H" of Block Sans (not kept) stayed
after two runs of the tool, the other letters did not change, and a
correction for another number of parts stopped the tool. The windows
were looked at in the four languages (ADR 0110).

Not done: no bundled font has a correction yet (Milestone 5 does this
work). The owner must sew a test sheet.

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

**Done in software (2026-11-18, ADR 0113).**

An audit (`emb-cli font-audit`) and proof pictures of each letter
(`tools/font-proof.py`) showed where the converted fonts are not as good
as fonts that a person digitized. The faults were the same in each font,
so the converter got four new steps, and the font tool uses them for the
ten fonts:

1. **Ends of columns reach the corners.** An end goes on into a part of
   the outline with no stitches. An end that turns to the stroke that it
   meets (the top of "N", the corners of "Z") goes on straight.
2. **Round dots** get a column to their edge.
3. **Each letter is routed:** the order and the directions of its parts
   with the fewest trims, with travel runs under the strokes that sew
   later. A stroke is cut at a junction or a crossing where that saves a
   trim (the stem of "H" at its bar, one stroke of "X").
4. **Script letters join:** a letter ends at the end of its last stroke
   and starts where the last stroke of the letter before it ends.

The ten fonts, before and after (the trims in the 81 basic letters, each
sewn alone at the middle size of the font; the trims in the sentence "The
Quick Brown Fox Jumps over the lazy dog 0123456789" at the middle size,
with the trims between its 45 letters):

| Font | Kind | Outline covered | Trims in 81 letters | Trims in the sentence |
|---|---|---|---|---|
| Block Sans | block sans | 97.1 % to 98.8 % | 93 to 11 | 92 to 45 |
| Friendly Serif | serif | 96.5 % to 97.8 % | 194 to 30 | 135 to 52 |
| Pacifico | script | 97.1 % to 98.1 % | 114 to 10 | 115 to 30 |
| Atkinson Hyperlegible | small text | 97.0 % to 98.2 % | 99 to 12 | 93 to 46 |
| Varsity | athletic | 98.5 % to 98.9 % | 194 to 32 | 159 to 56 |
| Bebas Neue | narrow | 97.9 % to 98.7 % | 102 to 9 | 102 to 44 |
| Soft Round | round | 97.8 % to 98.8 % | 78 to 12 | 84 to 46 |
| Geo Slab | slab | 96.0 % to 97.6 % | 160 to 15 | 119 to 45 |
| Brush Hand | brush | 96.0 % to 97.5 % | 123 to 18 | 115 to 52 |
| Vintage Script | monogram | 97.3 % to 98.3 % | 106 to 10 | 93 to 32 |
| All ten | | | 1263 to 159 | 1107 to 448 |

The trims that stay in the letters are between parts that do not touch
(the dot of "i", the two parts of ":" and of "="), and in letters with
many small serifs. In the sentence, the fonts with separate letters keep
one trim for each gap between two letters; the scripts keep the trims at
the spaces, between the digits and at some capitals.

Each of the 81 basic letters of each font was looked at on a proof
picture after the last change of the converter. What the pictures still
show is in "Left open" below.

The Fonts window marks the ten fonts as **tuned**, and the font lists
show them first (`bundled::TUNED`).

**Left open:**

- **No sew-out.** The proof is pictures and measures. The owner sews the
  test sheets (the exit test). The cuts of columns (a seam in the stem of
  "H") and the travel runs under narrow columns at the smallest size are
  the first things to look at.
- **No hand correction yet.** No font has a correction file: all changes
  are steps of the converter, so the other twenty fonts can get them too
  (Milestone 7). A letter that a sew-out shows as wrong gets a correction
  (Milestone 4).
- Letters with more than eight parts (serif letters with many small
  parts, letters with accents) get a simple route, not the search of all
  orders.
- The letters with accents and the other characters after the basic set
  have the same steps, but they were not all looked at.
- **What the proof pictures still show:** the "m" of Brush Hand is ten
  parts that do not touch (7 trims), and its "&" and "@" have parts with
  no stitches. The "x" and the "1" of Geo Slab have small parts with no
  stitches. The serif letters "H", "K", "X" and "m" of Friendly Serif and
  Varsity keep 1 to 5 trims.
- **Keyboard designs** take the parts of a letter as single objects, so
  the stitch engine joins them as objects, not as a routed letter. The
  sample "KDC" has 14 trims (12 before): the order of the font is not the
  best order for single objects.
- **Fat Face Serif** was the first monogram font. Its hairlines and ball
  ends do not convert well (many letters with less than 90 % of the
  outline), so it is not a tuned font; Vintage Script is the monogram
  font.

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
