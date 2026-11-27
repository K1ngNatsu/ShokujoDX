# Plan: help and tutorials (phase P19)

Date: 2026-11-18 · Status: Proposed; Part 2 made (2026-11-25) and in v0.6.0
(2026-11-26). The owner wants much better
tutorials and help (2026-11-18); the owner selects the milestones and
their order before the work starts. On 2026-11-24 the owner asked to
expand the tutorials thoroughly: Part 2 (at the end) is that work. The
owner accepted the draft of Part 2 ("Begin M1 as drafted", 2026-11-24);
Milestone 1 of Part 2 is made (ADR 0162).

## Goal

The reference product has a manual of more than a thousand pages, an
online academy and many videos. The program has 39 help topics (seven of
them tutorials) as text with no pictures, a search that answers from the
text, F1 on each button and window, and Find a Command (Ctrl+K).

More text of the same kind does not close this gap. The program can do
what a manual and a video cannot do: it knows the design of the user, it
can show the real button, and it can check that a step is done. This plan
makes the help a part of the work.

Nine milestones are proposed. Milestones 1 to 4 change the most for a new
user.

What stays:

- The help works with no network. No account, and no data of the user
  leaves the computer.
- The topics are Markdown files in Simplified Technical English, with
  translations of the same blocks (ADR 0084).
- No names of closed-source programs (the glossary gives the other common
  names of a tool, not the names of products).
- A new user starts with a clean slate (ADR 0080): samples open from the
  help and are not put in the folders of the user.

## Milestone 1: pictures in the help (M)

1. A help topic can show pictures (a new block of the help Markdown).
2. The pictures are made from the program itself by the test of P17
   Milestone 2: a small script for each picture opens a window or a
   design and cuts the part to show. So the pictures are made again when
   the program changes, and each language has its own pictures.
3. A size limit for the pictures of the program; large ones go in a help
   package.

**Exit test:** each topic of the start group and each tutorial has
pictures in four languages; a changed button changes its pictures with no
work by hand.

## Milestone 2: Show Me (S)

1. A command name in a help topic is a link. A click opens the ribbon tab
   of the command and marks its button for a few seconds.
2. An answer of the help search and a row of Find a Command have **Show
   Me** too.
3. When the command cannot run now, the help says why (the text that the
   dimmed button has).

**Exit test (a new user):** the user finds five commands from the help
with no search by hand on the ribbon.

## Milestone 3: guided lessons (L)

A tutorial that the user does in the program, step by step.

1. A lesson is a file: a start design, and steps. A step has a text, the
   button or the place on the canvas to mark, and a check of the design
   (for example: the design has a satin column). The check is a small
   script with no access to files (the script engine of ADR 0067).
2. A lesson panel shows the step, marks the target, and goes to the next
   step when the check passes. The user can go back, skip a step, or
   stop; the program keeps the place.
3. The seven tutorials become lessons. New lessons: the first ten
   minutes, a logo from a picture, a name on a cap, a file from a
   customer (P18), a multi-needle order.
4. Users and shops can write lessons of their own (a lesson is an asset).

**Exit test (new users):** three new users complete "the first ten
minutes" and the patch lesson with no other help.

## Milestone 4: the sew-out doctor (M)

Help for the problem that the user has on the machine.

1. **Help > Sew-out Problem** asks what the user sees: puckered fabric,
   a gap between a fill and its border, fabric that shows through, loops,
   thread breaks, small text that cannot be read, outlines that are not
   in their place.
2. The program checks the open design for the usual causes (the density
   map, the pull compensation, the underlay, the stitch lengths, the
   fabric setting, the sew order), shows the objects that have them, and
   offers a correction for each (one undo step), as the checks of the
   Picture to design wizard do.
3. It also gives the causes that are not in the design (stabilizer,
   needle, thread tension), from the help text.

**Exit test (owner and testers):** for three real sew-out problems, the
doctor finds the cause and the corrected design sews better.

## Milestone 5: samples and Try It (S)

1. **Help > Samples** opens small sample designs from the program (not
   from a folder of the user): one for each object kind and task.
2. A help topic of a stitch type or an effect has **Try It**, which opens
   its sample with the object selected and its settings open.

**Exit test:** each stitch type topic has a sample that opens.

## Milestone 6: ways to start (M)

1. The start page asks one time what the user wants to do: a home
   machine, a shop, or "I know another embroidery program". Each way has
   its lessons and its first topics.
2. For users of other programs: the glossary and Find a Command know the
   other common names of each tool, so a search with a known word finds
   our command.
3. **What is new** after an update (P17 Milestone 1) links to the topics
   and lessons of the new features.

**Exit test (testers):** a professional tester finds ten tools by the
names that the tester knows.

## Milestone 7: the help on the web and as a manual (M)

1. The same Markdown files make a website (GitHub Pages) with the
   pictures and a search, in each language. A search engine finds the
   program by its features.
2. The same files make a manual (PDF) for print.
3. The program has a link to the web page of each topic, to share it.

**Exit test:** the website and the manual are made by one command in CI;
they have each topic of the program.

## Milestone 8: moving pictures (M)

1. The test of Milestone 1 can run the steps of a lesson and write the
   frames as a short animation (the GIF writer of ADR 0108), so a topic
   can show a tool in use.
2. The owner can record videos with a voice from the same lessons; the
   start page and the website link to them.

**Exit test:** each lesson has an animation in the help on the web.

## Milestone 9: better answers (S)

1. The help search gets the words that testers use in their questions
   (a list of other words for each topic).
2. A question with no answer is kept in a list on the computer; the user
   can send the list with a report (the user decides; the program sends
   nothing).

**Exit test:** twenty questions of testers each give a useful first
answer.

## Part 2: more tutorials (accepted, 2026-11-24)

The owner asked to expand the tutorials thoroughly (2026-11-24, after the
release of v0.5.0). This part is the draft of that work. It uses parts of
Milestones 1, 3 and 5 above (pictures, the step files of lessons,
samples), so the work of the tutorials is not done two times.

### Where the tutorials are today

- Seven tutorials (48 to 88 lines each, in four languages): a patch from
  a picture, a name on an arc, a monogram on a towel, an appliqué, a
  lace ornament, a design in two hoopings, production output. The start
  page shows three of them under **Learn**.
- They are text only. Each starts from nothing (no start design) and
  ends with **Next** links.
- No test checks them. A check of 2026-11-24: the 151 bold names in the
  tutorials are texts of the program (a few lines break in the middle of
  a name), but nothing checks that a step still works, that a command is
  still in the panel that the step names, or that the result is what the
  step says.
- Most of the program has no tutorial: drawing by hand, the settings
  that make a design sew well, the sew order, fonts and their sizes,
  team names, a font of your own, Japanese text, photos, vector
  artwork, files from a customer (Recognize Objects), cutwork,
  rhinestones, quilting, cross stitch, puff on a cap, buttonholes,
  borders and motifs, multi-needle machines, quotes and approval sheets,
  the product view, sketch mode, scripts.

### The decisions of the draft

1. **Learning paths.** The tutorials are in seven paths (the start page
   and the Help window show them in order): first steps, drawing by
   hand, lettering, pictures and artwork, decoration and specialty,
   production, and more. Each tutorial says which tutorial comes before
   it.
2. **One form for each tutorial.** A card at the top (what you make, the
   time, the level, what you need, the tutorial before it), a start
   design that opens from the tutorial, numbered steps with the full
   place of each command (**Digitize > Fills > Shape**), a check with
   numbers ("the status line shows 2 colors and about 6,000 stitches"),
   "if it does not look right" (the two or three usual faults), and
   **Next**.
3. **Start designs from the program.** Each tutorial opens a start design
   or a start picture that the program makes (as the corpus designs of
   `emb_cli::corpus` are made in code), not a file in the folders of the
   user (ADR 0080). This is Milestone 5 (samples) for the tutorials.
4. **A step file for each tutorial.** The steps are also a small file
   that the app test harness can run (the harness that makes the
   pictures of the windows, `emb-app/src/pictures.rs`): click a command
   by its place, click points on the canvas, type a value, check the
   design (object kinds, colors, a range of stitches). One file gives
   three things:
   - **a test**: CI runs each tutorial, so a tutorial cannot go wrong
     when the program changes;
   - **the pictures** of the tutorial in four languages (Milestone 1),
     made again with no work by hand;
   - **the lesson** of the tutorial later (Milestone 3): the lesson panel
     uses the same steps and checks.
5. **A quick check for all tutorials** (a normal test): each bold place
   with ">" is a command, a panel or a window of the program; each link
   goes to a topic; each start design opens; each tutorial is in a path;
   the translations have the same steps.
6. **Four languages for each tutorial**, as all help topics (ADR 0084),
   in Simplified Technical English and its translations.

### The tutorials (draft list: 7 now, 24 new)

The time is for a new user. (R): the tutorial exists and is revised to
the new form.

| Path | Tutorial | What the user learns | Time |
| --- | --- | --- | --- |
| 1. First steps | The first ten minutes | Open the start design, add a fill and a satin border, change a thread, play the stitch player, save, write a machine file | 10 min |
| 1. First steps | From the computer to the machine | The machine and the hoop, the file format, Send to USB, the first sew-out (stabilizer, needle, test fabric) | 15 min |
| 2. Drawing by hand | A logo by hand on a backdrop | A backdrop picture, fills with holes, satin columns, runs, colors | 25 min |
| 2. Drawing by hand | Make it sew well | Fabric presets, underlay, density, pull compensation, the density map and the checks of the Check panel | 20 min |
| 2. Drawing by hand | Columns that turn | Column B and C, rungs, turning fills, Convert to satin, satin and split satin | 20 min |
| 2. Drawing by hand | Sew order, trims and travel | The Sequence panel, Arrange > Sew order, Shortest order, hidden travel, the trims in the status line | 15 min |
| 3. Lettering | A name on an arc (R) | Lettering, an arc baseline, kerning | 15 min |
| 3. Lettering | Choose a font and a size | Embroidery fonts and fonts of the computer, the size range, the quality score, Auto, Satin and Fill, small text, font packages | 15 min |
| 3. Lettering | Team names | One design with a name list, the output of each name | 15 min |
| 3. Lettering | A monogram on a towel (R) | Monograms, a towel with no topping | 15 min |
| 3. Lettering | An embroidery font of your own | Convert a font, the test sheet, corrections of single letters in Alphabets | 25 min |
| 3. Lettering | Japanese text | Kana and kanji in a font of the computer, the height of the characters, the smallest size | 10 min |
| 4. Pictures and artwork | A patch from a picture (R) | Picture to Design (the manual way moves to path 2) | 15 min |
| 4. Pictures and artwork | A photo in stitches | Picture to Design with a photo | 15 min |
| 4. Pictures and artwork | A vector logo | Import SVG, EPS or PDF, shaping (weld, cut), Convert to stitches, Auto-trace | 20 min |
| 4. Pictures and artwork | A file from a customer | Open a DST file, Recognize Objects, outlines, change the size, write it again | 20 min |
| 5. Decoration and specialty | An appliqué (R) | Appliqué objects, placement and tack-down | 20 min |
| 5. Decoration and specialty | A lace ornament (R) | Free-standing lace, the loose-part check | 20 min |
| 5. Decoration and specialty | Borders, motifs and families | Borders, motif runs and fills, a clipart item from a family | 15 min |
| 5. Decoration and specialty | Puff on a cap | Cap frames, the cap order, puff satin and the foam stop | 20 min |
| 5. Decoration and specialty | Buttonholes | A buttonhole set, standard sizes, the placement | 10 min |
| 5. Decoration and specialty | Cutwork | Cut lines, the cutting needles, a cut-out with a satin edge | 20 min |
| 5. Decoration and specialty | Rhinestones | A stone outline, the template and its marks | 15 min |
| 5. Decoration and specialty | A quilt block | Echo quilting, a background, block lines | 15 min |
| 5. Decoration and specialty | Cross stitch | Picture to Cross Stitch, the grid and the stitch kinds, Aida fabric, the chart | 15 min |
| 6. Production | Production output (R) | Worksheets, approval sheets, batch output | 15 min |
| 6. Production | A design in two hoopings (R) | Multi-hooping, the joins | 20 min |
| 6. Production | A multi-needle order | Needles, thread stock, the run time, order details, the worksheet | 20 min |
| 6. Production | A quote with a product picture | The product view, a quote, the approval sheet with the product page | 20 min |
| 7. More | A badge in sketch mode | A parametric sketch, dimensions, a badge from its regions, a change that changes the stitches | 30 min |
| 7. More | A script that does the work | The Scripts window, a sample script (check before output, satin density), a change of all objects of a kind in one undo step | 20 min |

### Milestones of Part 2 (draft)

1. **Tools and the first path (M).** The tutorial form and the card; the
   picture block in the help Markdown; start designs from the program
   (a `sample:` link); the step file and its runner in the harness; the
   quick check of all tutorials; the Learn part of the start page and the
   Help window show the paths. The two tutorials of path 1, with
   pictures, steps and tests, in four languages.
   **Exit test:** both tutorials run in CI and make their pictures in
   four languages; a changed command name makes the test fail.
2. **Drawing by hand and the old tutorials (M).** Path 2 (four
   tutorials), and the seven tutorials of today in the new form (start
   designs, steps, pictures). The manual way of the patch tutorial moves
   to path 2.
3. **Lettering, pictures and artwork (L).** The new tutorials of paths 3
   and 4 (seven).
4. **Decoration and specialty (L).** The new tutorials of path 5
   (seven).
5. **Production and more (M).** The new tutorials of paths 6 and 7
   (four).
6. **Lessons (L, later; Milestone 3 above).** The lesson panel runs the
   step files in the program: it marks the target, checks each step, and
   goes on.

Each milestone: the topics in English, German, Spanish and Japanese; the
pictures of the windows looked at; the help tests; What Is New of the
next release.

### State of Part 2

- **Milestone 1: made (2026-11-24, ADR 0162).** The card of a tutorial
  (path, time, level, the tutorial before it), pictures in the help
  (`![text](name)`, `docs/help/pictures`), start designs
  (`emb-app/src/samples.rs`, links `sample:<name>`), the step files
  (`docs/help/steps`) and their runner (`emb-app/src/tutorial_steps.rs`),
  the quick check of all tutorials, the list of all tutorials by path
  (the start page and the Help tab), and the two tutorials of the first
  path ("Your first design", "From the computer to the machine") in four
  languages with 15 pictures (0.8 MB). The exit test passes: both
  tutorials run in CI in four languages and make their pictures, and a
  changed command name makes the test fail
  (`a_step_with_a_wrong_place_fails`). The quick check found one wrong
  place in the old tutorials (**File > Export machine file**), now
  correct. The seven tutorials of before have their cards; they get
  start designs, steps and pictures in Milestone 2. What Is New gets
  the tutorials at the next release.
- **Milestone 2: made (2026-11-24, ADR 0162).** Path 2 has four
  tutorials: "A logo by hand" (a fill with a hole, a satin column and a
  line on a backdrop picture; the manual way of the patch tutorial moved
  here), "Make a design sew well" (the fabric, the density, the pull
  compensation, the density map), "Columns that turn" (Column Pairs,
  Column C, Turning, To Satin) and "Sew order, trims and travel" (Fewer
  Color Changes, Shortest Order, hidden travel). The seven tutorials of
  before have start designs (or the lace template), steps and pictures:
  the patch tutorial uses Picture to Design on a start picture, the
  appliqué starts from a heart fill, the hoopings from a row of five
  dots. All 13 tutorials run in CI in four languages; 54 pictures, 3.9
  MB. The steps found two weak places of the program, now in
  `docs/design/open-work.md`: four of the six monogram templates build
  up too much thread (the towel tutorial uses one that sews clean; ADR
  0163 and ADR 0164 fixed them, except one small place of "Block in a
  square" on towel fabric), and the lace ornament template shows ten
  "Thread builds up" warnings.
- **Milestone 3: made (2026-11-24, ADR 0162).** Path 3 has four new
  tutorials: "Choose a font and a size" (the Fonts window, the quality
  score, the size range, satin or fill, fonts for small text), "Team
  names" (columns, a pasted list, Show, All in one design), "An
  embroidery font of your own" (the convert settings, a test sheet, a
  letter correction) and "Japanese text" (a font with Japanese
  characters, the height of the characters, text that is too small).
  Path 4 has three: "A photo in stitches" (Photo stitch on a start photo
  of an apple), "A vector logo" (Weld, To Fill, Add Border, Trace
  Picture) and "A file from a customer" (a DST file, Recognize Objects,
  a new size). 20 tutorials run in CI in four languages. The steps found
  three faults of the program, now fixed: a click in the search field of
  the font list closed the list; a docked window went under itself for
  one frame when egui ran a frame two times (a font in the list that
  showed its preview), so a click in that frame hit the canvas; and **All
  in one design** of the team names put the outer names in the safety
  margin of the hoop. The
  steps of the own font do not convert a font or save a correction
  (they write files in the folder of the user's fonts), and they make
  the test sheet of an embroidery font of the program (the letters of a
  TrueType font take too long in a test).
- **Milestone 4: made (2026-11-24, ADR 0162).** Path 5 has seven new
  tutorials: "Borders, motifs and families" (a border of hearts, a motif
  run and a motif fill, a clipart star from a family), "Puff on a cap"
  (a multi-needle machine and a cap frame, puff lettering, the cap
  order), "Buttonholes" (a string of four on a placket, a standard size,
  mitre corners), "Cutwork" (a running line, the cut lines of a Bernina
  kit, a satin edge), "Rhinestones" (Outline to Stones, a stone run, the
  template marks), "A quilt block" (echo lines, a stipple, a block from a
  wave line) and "Cross stitch" (Picture to Cross Stitch on Aida, crosses
  and a backstitch line by hand, the chart). 27 tutorials run in CI in
  four languages. The steps found two faults of the program, now fixed:
  the Fabric list was too short to show its last fabrics (Aida, Quilt),
  and puff letters put end stitches two times at their joints (ADR
  0165). The puff apex of "A" still builds up thread (open-work list).
  The steps do not use the Clipart tab (it writes the built-in clipart
  into the folder of the user the first time it shows), and they only
  check the commands that save a file (Template, Chart). The pictures of
  the tutorials are now 127 files, 13 MB: more than the 10 MB of answer
  1, so they would go to a help package. The owner put the size of the
  program off to a later phase (2026-11-24: "we're not bloated, I'd
  rather not deal with optimization yet"), so they stay in the program
  for now (`docs/design/open-work.md`).
- **Milestone 5: made (2026-11-25, ADR 0162).** Path 6 has two new
  tutorials: "A multi-needle order" (the needles of a 15-needle machine,
  threads of a chart in the stock and Use Threads in Stock, the order
  details, the heads and the run time, the threads to buy, the
  worksheet) and "A quote with a product picture" (a polo shirt and the
  left chest, a price list with a quantity price, the quote, the
  approval sheet with its product page). Path 7 has two: "A badge in
  sketch mode" (two circles and a hexagon, two diameters, fills and a
  satin from the regions, a changed diameter that the stitches follow)
  and "A script that does the work" (Check Before Output, Same Density
  for All Satins, one undo step, a script of your own). 31 tutorials run
  in CI in four languages; the pictures are 154 files, 16 MB (9.2 MB
  since 2026-11-26, the same pixels, ADR 0162). The steps
  found faults of the program, now fixed: the "Add from a chart" part of
  the Thread Stock window went out of sight when the stock had threads
  (it is now first), and the windows Thread Stock, Quote and Order
  Details showed only 420 points of their lists (egui gives a window
  that is not resizable its default height); in German, Redo and Repeat
  had the same name (Redo is now "Wiederherstellen"); one head and one
  spool had plural texts; the worksheet cut the code of a thread
  ("Madeira Polyneon 1" for 1967; now a long brand gets shorter and the
  code stays whole), and a long label of a translation ran into its
  value (the values now start after the longest label); a new machine
  kept a hoop that does not go on it (the worksheet showed the Tajima
  in the Crown hoop of the Topaz; the owner saw it): the hoop now
  changes to the smallest hoop of the machine that holds the design
  (`AppState::hoop_for_machine`). The pictures showed one more fault:
  the travel between the sections of a fill with a hole crossed the
  rows (on top of sewn rows, or as a ridge under the next rows); it now
  goes along the rows (ADR 0166). The steps do not save a
  quote sheet, a PDF or a copy of a script (they write files).
- **Milestone 6: made (2026-11-25, ADR 0177).** The lessons: each of the
  31 tutorials is a lesson in the program. A step with `item` starts a
  stage that shows that item of the topic (and the items after it) in
  the language of the program; `test_only` marks the steps that only the
  test needs. The **Lesson** window shows the stage and what to do now,
  an orange mark is round the button (or its tab, its folded panel, the
  arrow of its drop-down button), on the points of the canvas, or on the
  item of an open list, and the lesson goes on when the user did the
  step and the design is as the step says (the checks that do not
  depend on the machine and the hand of the user). **◀ Back**, **Skip
  ▶**, **Tutorial**, **Stop the Lesson**; the place of a lesson that
  stops is kept, and the card of the tutorial can go on from it. The
  lesson starts from the card of each tutorial (**Start the Lesson**),
  from "All tutorials" and from the start page (the first tutorial).
  The exit test passes: `each_lesson_follows_its_steps` starts the lesson
  of each tutorial (the first one also in Japanese) and does its steps;
  the lesson marks what each step clicks, sees each step done and comes
  to its end. Three agents wrote the markers of the step files (one set
  of files each). They found places where a topic and its steps do not
  agree; the lesson shows the step in its own line, so a user can do
  them, but the topics can say them better:
  - "Japanese text": the topic did not say to select the lettering
    before it sets **Height** (Esc takes the selection away; the step
    clicks **Edit > Select > Select All**). Fixed: the topic and its
    translations now say it.
  - "Choose a font and a size": a step types `Atkinson` in the search
    field of the font list, which the text does not ask for (the field
    keeps `Pacifico` from before).
  - "Cross stitch" and "A badge in sketch mode": the steps turn on the
    thread look for a picture, which no item asks for (test_only now).
  - "Your own embroidery font": the steps use a font of the program in
    place of the converted font (a test does not convert), so those
    steps are test_only and the items are stages that the user reads.
  - Numbered items with no step (for example "Select your machine and a
    hoop") show with the item before them, or as a stage to read.
  Lessons that users and shops write (Milestone 3 of Part 1, item 4) are
  not in this milestone.

### Answers to the open questions

"Begin M1 as drafted" (2026-11-24) took the recommendations of the
draft:

1. The pictures are in the program: a picture of the canvas is one file
   for all languages, a picture of a window one for each language. Each
   milestone measures them; over 10 MB they go to a help package.
2. All tutorials, in the order of the milestones.
3. The lessons come later (Milestone 6 of this part); the step files are
   ready for them.
4. The start page shows the first path and a link to all tutorials.

### Open questions for the owner (draft, answered above)

1. **The size of the pictures.** About 4 pictures for each tutorial, 31
   tutorials and 4 languages is about 500 pictures. Crops of the program
   at 20 to 40 KB each are 10 to 20 MB in the program (the program is
   about 95 MB now). Other ways: pictures of the canvas only in one
   language (they have no text); or the pictures in a help package; or
   the English pictures in the program and all languages on the web
   (Milestone 7).
2. **All 24 new tutorials, or a first set?** The draft order gives the
   most to a new user first (paths 1 and 2), then lettering.
3. **Lessons now or later?** The step files make the lessons cheaper, but
   the lesson panel is a new part of the program (Milestone 6 of this
   part).
4. **The start page.** All paths in the Learn part, or the first path
   and a link to the others.
