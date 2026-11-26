# Open work

Date: 2026-11-24 (after the audit of P21 and the release of v0.5.0;
ideas from zigzag added on 2026-11-25, and phase P22 made of them; made
current by the audit of the work after P22, 2026-11-26) ·
Status: Reference. One list of all the work that is not finished: from
the roadmap, the plans of the phases, the audits and the inventory
(`docs/design/feature-inventory.md`).
The owner selects the work. Each audit makes this list current.

The program is done in software for phases P1 to P14, P16, P17 (not
Milestone 3), P18, P20 and P21, and all of it is in the release v0.5.0
(2026-11-24). What is open is mostly proof: sew-outs, tests with users
and tests on other machines and systems.

## 1. Sew-outs and checks of the owner

| Phase | What to do | Where |
| --- | --- | --- |
| P20 Milestone 3 | Sew the crossings and junctions at 25 mm and 50 mm, and 刺繍 and 東京 at 40 mm. Then choose the stroke that goes through at a crossing (the longer stroke, or the stroke that sews first; the split of P21 Milestone 8 cuts the thinner stroke now, ADR 0152) | `out/p20-sewout` and its checklist (`tools/make-sewout-kit-p20.py`; made again after the audit of P20, so use the new files) |
| P20 Milestone 6 | Sew the sheets of the two Japanese fonts at 20 mm; say which font looks better | The same kit |
| P21 | Sew the kit of the phase (made again on 2026-11-24 with the fonts of Milestone 14): the ten tuned fonts at three sizes, joints (with the serif joints of Friendly Serif, Milestone 9), kanji and Arabic, and the group `late/` of Milestones 11 to 14 (elbows, balls, heavy fonts); look for spikes, bulges, open corners and column ends on top. Then decide which fonts of the packages come into the program: since Milestone 10 the pointed ends of columns are not in the score, so most scores are higher and a score of 85 is easier to get (ADR 0156); look at the proof pictures. The fonts of earlier versions with a score over 85 are back in the program (the owner's choice after the audit of P21: Slab Heavy, Fat Face Serif, Shrikhand; Great Vibes, 75, stays in its package); Shrikhand is in the group `late/` of the kit, Slab Heavy and Fat Face Serif are in no kit yet | `out/p21-sewout` and its checklist (`tools/make-sewout-kit-p21.py`) |
| P18 part 1 | Milestone 4: a logo with lettering from a machine file at 150 %. Milestone 6: a corpus design written for Ink/Stitch opens there; a converted Ink/Stitch font sews at its sizes. Milestone 7: outlines with a stitch file | "Audit of P18" in the roadmap |
| P18 part 2 | Milestone 1: the sew-out of the reference set of the 3D view. Milestones 4 and 6: compare the view with photos. Milestone 7: compare with sew-outs. Milestone 8: check a design with the low light, the tilt view and a check view | "Audit of P18"; `docs/design/p18-view-m*-report.md` |
| P17 | Milestone 1: the update notice with a real release. Milestone 4: a font test sheet. Milestone 5: the test sheets of the ten tuned fonts at three sizes (make the sheets again first; the fonts changed). Milestone 6: one word in Arabic, in Devanagari and in a script font. Milestone 8: five new items of each kind | `out/p17-sewout` (`tools/make-sewout-kit.py`); "Audit of P17" |
| P16 | The eight exit tests of the product view (cloth look, several placements, front and back, the approval PDF printed on A4 and Letter, each product, two-tone products, a photo of a shirt as a template, the turntable with the software renderer) | "Audit of P16" |
| P22 Milestone 3 | Sew a sheet of test swatches (Design > Output > Test Swatches, or `emb-cli swatch`) on the Topaz 20: the copies, the steps and the labels | ADR 0170 |
| P22 Milestone 4 | Sew swatches of the lean (0° to 15°) and of the overlap of bands (0 to 0.6 mm) from the corpus design `satin-lean-bands` (`emb-cli corpus`, then Test Swatches or `emb-cli swatch --setting lean` and `--setting band-overlap`) on a knit and on a woven fabric | ADR 0171 |
| Icon | The license of the logo (its maker and the owner) | ADR 0140 |
| Release | Code signing of the Windows packages through the SignPath Foundation | ADR 0062 |

## 2. Tests that need other people, machines or systems

| Kind | Test | Phase |
| --- | --- | --- |
| Users | A new user digitizes a two-color patch from a picture in less than 15 minutes, on Windows, macOS and Linux | P2, P10 |
| Users | Five test users do three set tasks with no help | P3 |
| Users | A new user finds three commands with F1 and the help search; a name on a towel in the kiosk in 5 minutes | P10 |
| Users | A screen reader; a review of the German, Spanish and Japanese texts by native speakers | P10 |
| Users | Five test users tell a photo of a sew-out from a picture of the 3D view | P18 part 2 |
| Testers' files | Three Ink/Stitch files of testers open with their objects, kinds and colors, and one sews | P18 Milestone 5 |
| Testers' files | EPS and AI files of other programs | P18 Milestone 8 |
| Machines | The test kit on the industrial machine (the owner's Tajima with 12 needles, its hoops and its cap frame) and the reports of professional testers. Set aside by the owner | P17 Milestone 3 |
| Machines | Cap order on a cap frame (the Tajima above can do it); cutwork files on a cutwork machine | P11 |
| Machines | Sequins and beads; chenille and Schiffli (see section 3) | P9 Milestones 6 and 7 |
| Systems | The thumbnails of the file manager after an install on macOS and on Linux | P11 |
| Systems | The budgets of the 3D view on a small computer; the program on a computer with no graphics driver (software renderer) | P18 part 2 |
| Systems | The start test on the PC of the owner's friends | v0.3 Stage 1 |
| Tools | A phone app reads the barcode of a printed worksheet; a cutter program opens a cutter file at its real size | P8 Milestone 7, P13 |
| Shop | The thread use against weighed spools | P11 |
| Shop | Quotes and run times of real orders, a printed sheet with its barcode, the spools to buy, with a shop tester | P13 |

## 3. Features that are not made

### Planned or proposed (the owner decides)

| Feature | State | Where |
| --- | --- | --- |
| Letter shapes: the split of the outline into strokes (ADR 0152) for the scripts that it does not take yet (punctuation, Hangul, Arabic, Devanagari, Greek, Cyrillic) | Not made: each script needs a measure with the split first. The other work of the phase is made (fourteen milestones) | P21, `docs/design/p21-plan.md` |
| Help and tutorials: pictures in the help, Show Me, guided lessons, the sew-out doctor, samples and Try It, ways to start, the help on the web and as a manual, moving pictures, better answers | Proposed (2026-11-18): nine milestones | P19, `docs/design/p19-plan.md` |
| More tutorials (P19 Part 2): seven paths, about 31 tutorials with start designs, pictures and step files (ADR 0162), and lessons in the program (ADR 0177) | Accepted (2026-11-24); done (2026-11-25): 31 tutorials, each also a lesson. Open: the test of the lessons with new users (P19 Part 1, Milestone 3), lessons of users and shops, and a few topics that can say their steps better (the list in the plan) | P19 Part 2, `docs/design/p19-plan.md` |
| Checks, swatches and agents (P22): travel that shows, live numbers and Quicker to Sew, test swatches, satin lean and bands of threads, machine files (check, fix, a cross-check with pyembroidery), letter measures against the eye, overlapped strokes (a trial), the open tabs come back and one program, agents through MCP | Accepted (owner, 2026-11-25); Milestones 1 to 6 made (ADR 0167 to 0173); Milestone 7 measured and not adopted (ADR 0174); Milestone 8 made (ADR 0175); audited ("Audit of P22" in the roadmap); Milestone 9 made in the software (ADR 0176), the owner's test with an agent open. The findings of the agent test are fixed (the labels of test swatches: Soft Round; the travel between two letters on a fill: ADR 0178) | `docs/design/p22-plan.md` |
| Monograms that build up thread at large sizes. Measured on 2026-11-26: the worst places were wide column ends whose cap the stitches meet at a small angle, in letters larger than the columns of the font go (ADR 0178, 0180). ADR 0180 and 0181: the pairs that the stitches do not need go, short stitches where stitches meet a side at a small angle, and a letter chooses columns or fills by its size in the design (the edit of the Classic middle initial, the envelope of Oval and Diamond). The probe (64 cases): no build-up in 63 (38 places before ADR 0180). | Open: the joint of the arm and the stem of "K" (JMK, Classic at 60 mm, towel: 6.1 layers on 4 mm², two satin columns on each other); a travel of 4 mm on the sewn rows of the fill letter "M" (JMK, Oval at 60 mm, cotton) | `crates/emb-app/src/monogram.rs` (`probe_monogram_warnings`), ADR 0180, ADR 0181 |
| The pictures of the tutorials were 16.1 MB (154 files) after Milestone 5 of P19 Part 2, more than the 10 MB limit of ADR 0162 for a help package; the program file is about 100 MB | The pictures are 9.2 MB since 2026-11-26 (written again with no change of a pixel: no alpha channel, the best compression; `pictures::write_png`). Open (later phase): a help package or link-time optimization for the size of the program; pictures with a palette of their colors would be 0.7 MB smaller (a direct use of the `png` crate) | `docs/help/pictures`, ADR 0162 |
| Browser version | A trial on the branch `trial/browser`, on GitHub Pages for testers (2026-11-21 and 22: touch, phones, the loader; on 2026-11-24 it has the work of v0.5.0); step 2 (the web version for computers) not started | P15 in the roadmap, `docs/design/p15-trial-report.md` on the branch |
| Touch in the desktop program (Windows touch screens) | Planned (owner, 2026-11-22). From the trial: two fingers pan and zoom, handles and targets for a finger, the touch bars (the draw tools and the sketch Dimension tool), the wider line between panels, the side panels under the canvas on a tall narrow screen. In the program today: the touch bar of the draw tools (Next or Finish), the ruler menu with a long press | P15 step 3 in the roadmap |
| The assistant | Benched (owner) | P10 Milestone 9 |
| Sequins and beads: sequin objects and lettering, sequin shapes and fixings, flip sequins, the sequin mode of DST | Waits for a tester with such a machine | P9 Milestone 6 |
| Chenille and Schiffli | Waits for a tester with such a machine and a public description of the format | P9 Milestone 7 |
| Embroidery fonts for Arabic and Devanagari (converted, routed, with a score) | Shelved (owner, 2026-11-19) | Lane 7 of the review of P17 |
| Clipart sets with more detail, by theme, as asset packages | Shelved (owner, 2026-11-19) | Lane 8 of the review of P17 |
| The embroidery fonts made again with the stitch engine of ADR 0163 | Measured and kept (owner, 2026-11-26: "if they are better than they were then accept"). The fonts in the repository are from P21 Milestone 14 (2026-11-24); ADR 0163 changed the stitches that the converter measures when it chooses a split, so the fonts made now differ in some letters of 91 of 139 fonts. Not better: the same sum of scores (12,923) and of trims (33,935; nine fonts 1 to 3 more, six fonts 1 to 4 fewer), the same letters clean to the eye (480 of 496), the same trims in the corpus; the bench mixed (Block Sans: fewer bulges, more needle points out; Anton: fewer needle points out, more open corners). So the fonts stay as they are; the next remake (after a change of the stroke finder, the tuning or the route) brings these changes with it, and those of ADR 0180 | `tools/make-embroidery-fonts.py`, ADR 0163, ADR 0179, ADR 0180 |
| A bundled Ink/Stitch font | Needs a font with a license that permits it (the owner chose a test only) | P18 Milestone 6 |

### Letter shapes (after P21 Milestone 14)

- The foot of a stem past a junction (Anton "a"): the middle line prunes
  it as a side branch; a second pass for lost strokes in all letters
  added more faults than it removed (ADR 0158).
- The serifs at the top of the stems of Geo Slab "M" (15.6 mm2 more open
  in P21 Milestone 14, ADR 0160): the cut beside each stem meets the top
  edge at a small slant, and its moved end stays.
- Elbows that the cut of ADR 0159 does not find: an outer corner cut by a
  short edge that is not a side of a stroke (Fat Face Serif "7"), and an
  elbow that turns less than 80 degrees (the flag of Candy Sans "F").
- The parts of a stitch file: the width of a dense column with short
  stitches is often not found well (the measured columns of the corpus
  95.7 % to 93.5 % in P21 Milestone 13, ADR 0159; the limit is 90 %). The
  rails could follow the needle points that are not short.
- The size range of a converted font and the narrow measure of the score
  do not agree: the range allows 10 % of the length of the columns too
  narrow at its smallest size, and the score takes points for each part
  (Fat Face Serif 90 to 79 in Milestone 13, ADR 0159; 91 again in
  Milestone 14, with a size range from 42.5 mm).

- A dot with the shape of a "D" (the colon of Jazz Hand): the middle line
  goes to a corner, and the column skips a corner of the dot. The colon
  has a correction (ADR 0157).

### Japanese lettering (after P20)

- The kanji for names that are not jōyō kanji (the jinmeiyō kanji, about
  860) in the Japanese fonts.
- The あ of Zen Maru Gothic: a gap in its inner stroke (92.6 % covered).
  A letter correction (ADR 0112) or a change of the stroke finder.
- Two layers on more than 3 % of 9 characters of the reference set at
  10 mm and of 6 at the larger sizes (17 before P21): junctions of three
  or four strokes at a slant.
- Not in the phase: vertical text with the vertical forms of punctuation
  (the OpenType feature `vert`; stacked text exists), furigana, the
  stroke order of calligraphy, and tests with Chinese and Korean text.

### Small gaps that the audits left

Done on 2026-11-26: the names of the built-in motifs, split patterns,
stamps, borders, clipart items and families show in the language of the
program (they stay English in the files; the Clipart and border searches
find both names); the Families window has a ribbon command (Edit >
Stitch Types > Families); the Options window is no wider than in English
(Simple Mode and the paste choice have their explanation in a tooltip,
and the last row wraps), and the words after the sliders of Object
Properties and the values of Design Info go on a second line where the
panel is narrow; Recognize Objects takes 0.4 s for a stitch file that
sews one fill 500 times (28 s: the area of its rows made a piece for each
pair of the same rows on a line).

- Done (2026-11-26, the owner's choice): the side panels are two groups,
  one over the other (`app::SIDE_GROUPS`), and only the open tab of a
  group has its ×, so all the titles of each group show on a screen of
  1366 points in each language (a test measures them). A group that the
  user closes comes back as its group (the audit of 2026-11-26).

### Left by the audit of the work after P22 (2026-11-26)

The details are in the roadmap ("Audit of the work after P22").

- Fixed on 2026-11-26 (the owner's choice): a column slanted more than
  65° sews at its spacing (ADR 0181: the pair nearer to one step, with
  a carry); TrueType lettering in Auto chooses its columns after the
  envelope; `show` of `emb-cli mcp` starts the program through
  PowerShell on Windows, so the client gets the end of the output
  (ADR 0176); `check_machine_file` puts the middle of the stitches at the
  middle of the hoop.
- The Japanese lesson asks for the font Noto Sans JP (the font of its
  test); a user with another Japanese font clicks **Skip** at that step.
- The search of the Clipart panel finds a translated name only as one
  part of the search text (words in English: each word).

### Ideas from zigzag (2026-11-25)

zigzag (`github.com/simpsoka/zigzag`, Apache-2.0, Rust and egui) is an
open-source embroidery program. The owner asked for a comparison
(2026-11-25). It read the README, the changelog and the code of zigzag;
it did not run the program. The owner made a phase of the ideas that are
worth adding: P22, checks, swatches and agents (`docs/design/p22-plan.md`,
nine milestones: travel that shows, live numbers and Quicker to Sew,
test swatches, satin lean and bands of threads, machine files, letter
measures against the eye, overlapped strokes, the open tabs come back,
agents through MCP). One idea is not in the phase:

- **Send to the Brother Skitch PP1 over Bluetooth.** Its protocol is in
  open-source code (Respira, Apache-2.0), so for this machine a protocol
  is public (see section 4, Connections). Only when a tester has a
  Skitch; Bluetooth on Linux also needs a C library (D-Bus).

## 4. Not planned

These have no public description, or the owner decided against them.
The inventory has a note for each.

- **Formats:** VIP and CSD (home); CND, PCH, ART, JAN, GNC and INP
  (condensed formats); EBD, VEP, DSN, PMU, PUM, SAS, MST and ARX
  (industrial); YNG and the other formats of stone-setting machines; DHE
  and the other multi-decoration formats; CDR artwork (a message says to
  save as SVG, PDF or EPS); the native format of the reference product.
  More formats to write only when a tester has a machine that needs one.
- **Machine functions:** penetrations, borers, empty stitches and empty
  jumps: no format with a public description has them.
- **Connections:** sending to machines over a network (no maker publishes
  a protocol; for the Brother Skitch, see "Ideas from zigzag"), the
  scanner (the operating system scans), the runtime setup of a machine.
- **Workspace:** two groups of editable design tabs and toolbars (the
  split view, the views, the ribbon and the quick access buttons do the
  job); WMF export (EMF and SVG do the job).
