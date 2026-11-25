# ADR 0162: Tutorials that the program runs

Date: 2026-11-24 · Status: Accepted (the owner: "Begin M1 as drafted",
Milestone 1 of Part 2 of `docs/design/p19-plan.md`).

## Context

The program had seven tutorials in four languages. They were text only:
each started from nothing, and no test checked them. A tutorial went
wrong with no sign when a command got a new name or went to another
panel. The first check of the places in the tutorials found one such
place (**File > Export machine file**: the menu item is **Export Machine
File…**).

The owner wants many more tutorials (the draft has 31), each in four
languages, with pictures. A person cannot check 31 tutorials in four
languages by hand after each change of the program.

## Decision

1. **The card of a tutorial.** The head of a tutorial has its path
   (`path`, one of `help::PATHS`), its time in minutes (`time`), its
   level (`level`, one of `help::LEVELS`) and the tutorial to do before
   it (`before`). The translations take them from the English topic. The
   Help window shows them as a card under the title, the contents list
   the tutorials by path, and the topic `help::TUTORIALS_ID` ("All
   tutorials") is made from them.
2. **Pictures in the help.** A line `![text](name)` is a picture
   (`Block::Picture`). The file is `docs/help/pictures/<code>/<name>.png`
   (a picture with text: one for each language) or
   `docs/help/pictures/<name>.png` (a picture of the canvas: no text, one
   file). `build.rs` puts them in the program (`HELP_PICTURES`);
   `help::picture_bytes` takes the picture of the language, else the one
   with no text, else the English one. A numbered list after a picture
   goes on with its number (`Block::List` keeps the number of its first
   item), so a picture can show between two steps.
3. **Start designs from the program.** A link `[text](sample:<name>)`
   opens a start design that the program makes in code
   (`emb-app/src/samples.rs`), in a new tab, on the machine and in the
   hoop of the design of now. Nothing goes in the folders of the user
   (ADR 0080).
4. **A step file for each tutorial.** `docs/help/steps/<id>.toml` has
   the steps of the tutorial: open a start design, click a tab, click a
   command by its place (`Digitize > Shapes > Ellipse`), click a thread
   color, drag or click on the canvas (mm), push a key, click a button
   of a window by its English text, close a window, check the design
   (objects, kinds, threads, colors, stitches, size, selection, a window
   that shows, a command that is on), and make a picture (of the canvas
   round a part of the design, of the open windows, or of a ribbon
   panel). The runner (`emb-app/src/tutorial_steps.rs`, test code) does
   them in the whole program with pointer and key events, through the
   studio of the window pictures (ADR 0110). It finds each name in the
   language of the run with `tr`, so one step file is for all languages.
   A command that opens a window of the system (a file or a folder) is
   only checked (`check_only`).
   - `each_tutorial_runs_in_each_language` (a normal test, in CI) does
     each step file in English, German, Spanish and Japanese, and checks
     that the steps make the pictures that the topic shows. A changed
     name of a command makes it fail.
   - `write_tutorial_pictures` (ignored, release) writes the pictures.
   - The runner uses a screen of 3840 × 1600 points, so no panel of the
     ribbon folds (a folded panel stays open as a menu, and it showed in
     the pictures of windows). It can still open a folded panel.
5. **A quick check of all tutorials.** Each bold place with " > " in a
   tutorial, in each language, is `Tab > Panel`, `Tab > Command`, `Tab >
   Panel > Command` or `File > Item` of the program
   (`the_places_in_the_tutorials_are_in_the_program`).
6. **The first path.** The tutorials "Your first design" and "From the
   computer to the machine" are the path "First steps". The Learn part
   of the start page shows them and "All tutorials"; the Help tab has
   **All Tutorials**.
7. **The size of the pictures.** The pictures of Milestone 1 are 15
   files, 0.8 MB; with Milestone 2, 54 files, 3.9 MB. Each milestone
   measures them; if all pictures go over 10 MB, they go to a help
   package (open question 1 of the draft).

The thread colors of the Threads strip now have screen reader names (the
names of the threads): the runner clicks them by name, and a screen
reader says them.

Milestone 2 (path 2 and the seven tutorials of before) added:

- **More step kinds:** `clicks` (a list of left clicks and right clicks,
  the corners and curve points of the drawing tools), `type` (text into
  the field that has the keyboard), `set` (the number or text field
  after a label: a click, all the text, the value, Enter) and `choose`
  (an item of the list after a label, or before it for a list with its
  label after it). Number fields and lists have no screen reader name,
  so the runner finds them by the label on their row. More checks:
  `fabric`, `warnings` (Design Info), `holes`, `hoopings` and `trims`.
  The runner waits for the background work of the Picture to Design
  wizard.
- **Start pictures:** a start design can have a backdrop picture that
  `samples.rs` draws (a leaf logo to trace, a logo for Picture to
  Design). Its shapes come from the same points that the step file
  clicks, so the stitches of the steps go on the picture. The wizard uses
  the backdrop picture, so no file is needed.
- **Pictures at most 720 pixels wide** (the Help window shows them at
  that size or smaller).
- The step files found two weak places of the program that the
  tutorials now avoid (`docs/design/open-work.md`): four of the six
  monogram templates, and the Classic style, build up too much thread
  where letters and borders overlap; the lace ornament template shows
  ten "Thread builds up" warnings.

Milestone 3 (paths 3 and 4, seven tutorials) added:

- **More step kinds:** `field` (a click on the text field with this hint
  text, or after this label, and all its text selected: what `type`
  types next replaces it), `font_file` (a font file of the repository as
  a font of the computer: the Japanese tutorial uses the test font of
  Noto Sans JP, so the steps do not depend on the fonts of the computer
  that runs them) and the check `names` (the rows of the team names). A
  `choose` in a long list with a search field types the item in the
  field. A `button` with no widget of its name clicks a row of a list in
  a window whose name starts with it ("Graduate Regular  ·  TrueType"),
  and never a text field or the text in it (the name of a text field is
  its text). A picture of the canvas with no `around` shows the whole
  design. The runner waits for the search of the fonts of the computer.
  A step that fails says which widgets the windows have, and what the
  status line says.
- **Start designs:** a team shirt (a name and a number), a photo of an
  apple (light and dark, drawn in `samples.rs`), a cloud logo of vector
  shapes, and a DST file of the leaf logo (the leaf sewn, written as DST
  and read again, so it is what File > Open gives).
- **Three faults of the program that the steps found:** the list of
  fonts closed when you clicked its search field (an egui combo box
  closes on any click; the list now closes on a click outside it or on a
  font); a docked command window went under itself for one frame when
  egui ran the windows two times in one frame (`dock::docked` now starts
  its stack for each pass, not for each frame): a click in that frame hit
  the canvas; and **All in one design** of the team names used the whole
  width of the hoop, so Design Info warned of the outer names (the rows
  are now not wider than the hoop in its safety margin).

Milestone 4 (path 5, seven tutorials) added: `choose` scrolls a long
list with the wheel of the mouse when the item is not in sight and the
list has no search field (the list of machines, of hoops); `close` finds
the title with a close button on its row (in Japanese "Family" and
"Families" are both ファミリー); a failed check of the warnings says
where each warning is. The steps found two faults of the program: the
Fabric list did not show its last fabrics, and puff letters had two
layers of end stitches at their joints (ADR 0165).

Milestone 5 (paths 6 and 7, four tutorials) added: `choose` scrolls in
the list that opened (its area, also when it opens over the list button)
and types only into a search field that came with the list, and an item
can be the start of its name before a space ("Same Density for All
Satins (sample)"); a field after a label is the field on the row of a
label of that name (a window can have "Quantity" for a field and for a
column); a command under the arrow of a drop-down button of the ribbon
is clicked through its arrow (`ribbon::MORE_KEY`); a picture of the
canvas has the product that shows; new checks are the threads in stock,
the product and its place, the hoop, sketch mode, the regions of the
sketch and the spacing of the satin columns. The start design
`club-crest` is the design of three tutorials. The owner saw a fault in
a picture that no check had: the worksheet showed the Tajima machine in
a hoop of the Topaz; the multi-needle steps now check the hoop.

## Consequences

- A change of the name or the place of a command, of the text of a
  button, or of what a step does makes a test fail. Change the topic,
  its translations and the step file together, then make the pictures
  again (`cargo test --release -p emb-app -- --ignored
  write_tutorial_pictures`) and look at them.
- A new tutorial needs: the topic with its card (and `help::FILES`), its
  translations, its step file, its pictures, and a start design if it
  needs one. The lessons (Milestone 6 of Part 2) can do the same step
  files in the program.
- The pictures of the canvas show the thread look of the quality Fast,
  as the tests of the app have no surface (ADR 0126).
