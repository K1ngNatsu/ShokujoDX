# Open work

Date: 2026-11-20 (after the audit of P20 and the release of v0.4.0) ·
Status: Reference. One list
of all the work that is not finished: from the roadmap, the plans of the
phases, the audits and the inventory (`docs/design/feature-inventory.md`).
The owner selects the work. Each audit makes this list current.

The program is done in software for phases P1 to P14, P16, P17 (not
Milestone 3), P18 and P20, and all of it is in the release v0.4.0. What
is open is mostly proof: sew-outs, tests with users and tests on other
machines and systems.

## 1. Sew-outs and checks of the owner

| Phase | What to do | Where |
| --- | --- | --- |
| P20 Milestone 3 | Sew the crossings and junctions at 25 mm and 50 mm, and 刺繍 and 東京 at 40 mm. Then choose the stroke that goes through at a crossing (the longer stroke, or the stroke that sews first; the split of P21 Milestone 8 cuts the thinner stroke now, ADR 0152) | `out/p20-sewout` and its checklist (`tools/make-sewout-kit-p20.py`; made again after the audit of P20, so use the new files) |
| P20 Milestone 6 | Sew the sheets of the two Japanese fonts at 20 mm; say which font looks better | The same kit |
| P21 | Sew the kit of the phase: the ten tuned fonts at three sizes, joints, kanji and Arabic; look for spikes, bulges, open corners and column ends on top. Then decide which fonts of the packages with a score of 85 or more come into the program | `out/p21-sewout` and its checklist (`tools/make-sewout-kit-p21.py`) |
| P18 part 1 | Milestone 4: a logo with lettering from a machine file at 150 %. Milestone 6: a corpus design written for Ink/Stitch opens there; a converted Ink/Stitch font sews at its sizes. Milestone 7: outlines with a stitch file | "Audit of P18" in the roadmap |
| P18 part 2 | Milestone 1: the sew-out of the reference set of the 3D view. Milestones 4 and 6: compare the view with photos. Milestone 7: compare with sew-outs. Milestone 8: check a design with the low light, the tilt view and a check view | "Audit of P18"; `docs/design/p18-view-m*-report.md` |
| P17 | Milestone 1: the update notice with a real release. Milestone 4: a font test sheet. Milestone 5: the test sheets of the ten tuned fonts at three sizes (make the sheets again first; the fonts changed). Milestone 6: one word in Arabic, in Devanagari and in a script font. Milestone 8: five new items of each kind | `out/p17-sewout` (`tools/make-sewout-kit.py`); "Audit of P17" |
| P16 | The eight exit tests of the product view (cloth look, several placements, front and back, the approval PDF printed on A4 and Letter, each product, two-tone products, a photo of a shirt as a template, the turntable with the software renderer) | "Audit of P16" |
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
| Letter shapes: the outline is the edge of each column, one way to end a column, the middle line, checks on the needle points, the sew order at joints, the bundled fonts made again; the split of the outline into strokes (ADR 0152) for the scripts that it does not take yet (punctuation, Hangul, Arabic, Devanagari, Greek, Cyrillic) | Accepted (owner, 2026-11-23), started: six milestones | P21, `docs/design/p21-plan.md` |
| Help and tutorials: pictures in the help, Show Me, guided lessons, the sew-out doctor, samples and Try It, ways to start, the help on the web and as a manual, moving pictures, better answers | Proposed (2026-11-18): nine milestones | P19, `docs/design/p19-plan.md` |
| Browser version | A trial on the branch `trial/browser`, on GitHub Pages for testers (2026-11-21 and 22: touch, phones, the loader); step 2 (the web version for computers) not started | P15 in the roadmap, `docs/design/p15-trial-report.md` on the branch |
| Touch in the desktop program (Windows touch screens) | Planned (owner, 2026-11-22). From the trial: two fingers pan and zoom, handles and targets for a finger, the touch bars (the draw tools and the sketch Dimension tool), the wider line between panels, the side panels under the canvas on a tall narrow screen. In the program today: the touch bar of the draw tools (Next or Finish), the ruler menu with a long press | P15 step 3 in the roadmap |
| The assistant | Benched (owner) | P10 Milestone 9 |
| Sequins and beads: sequin objects and lettering, sequin shapes and fixings, flip sequins, the sequin mode of DST | Waits for a tester with such a machine | P9 Milestone 6 |
| Chenille and Schiffli | Waits for a tester with such a machine and a public description of the format | P9 Milestone 7 |
| Embroidery fonts for Arabic and Devanagari (converted, routed, with a score) | Shelved (owner, 2026-11-19) | Lane 7 of the review of P17 |
| Clipart sets with more detail, by theme, as asset packages | Shelved (owner, 2026-11-19) | Lane 8 of the review of P17 |
| A bundled Ink/Stitch font | Needs a font with a license that permits it (the owner chose a test only) | P18 Milestone 6 |

### Japanese lettering (after P20)

- The kanji for names that are not jōyō kanji (the jinmeiyō kanji, about
  860) in the Japanese fonts.
- The あ of Zen Maru Gothic: a gap in its inner stroke (92.6 % covered).
  A letter correction (ADR 0112) or a change of the stroke finder.
- Two layers on more than 3 % of 17 characters of the reference set:
  junctions of three or four strokes at a slant (本, 桜).
- Not in the phase: vertical text with the vertical forms of punctuation
  (the OpenType feature `vert`; stacked text exists), furigana, the
  stroke order of calligraphy, and tests with Chinese and Korean text.

### Small gaps that the audits left

- The names of the motifs, split patterns, stamps, borders, clipart items
  and families are in English only.
- The Families window has no ribbon command (the Motifs window, Object
  Properties and the Clipart panel open it).
- The titles of some dock tabs are cut in the translations, and the View
  Options window is wider in German and in Spanish than in English.
- Recognize Objects is slow for a stitch file that sews one fill many
  times at one place (500 layers: 28 s).
- The routes of the letters of a font of the computer are kept while the
  program runs, not on the disk: the first use of a kanji in a new session
  makes its route again (0.004 s to 0.56 s for each character, in a
  thread).

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
  a protocol), the scanner (the operating system scans), the runtime
  setup of a machine.
- **Workspace:** two groups of editable design tabs and toolbars (the
  split view, the views, the ribbon and the quick access buttons do the
  job); WMF export (EMF and SVG do the job).
