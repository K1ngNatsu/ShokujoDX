# ADR 0177: Lessons in the program

Date: 2026-11-25 · Status: Accepted (the owner: "Finish up P19 part 2,
M6", Milestone 6 of Part 2 of `docs/design/p19-plan.md`).

## Context

Part 2 of P19 made 31 tutorials, each with a step file that a test does
in the whole program (ADR 0162). The step files were made so that a
lesson can use them later: the plan says that a lesson panel runs the
step files in the program, marks the target, checks each step and goes
on. A step file has no text for the user, and its steps are smaller
than the instructions of the topic: one instruction ("Click **Digitize >
Select > Select**, then click the circle") can be two steps, and some
steps are only for the test (an Esc that closes a menu).

## Decision

1. **Stages.** A step with `item = ["<## heading>", n]` starts a stage
   of the lesson. The stage shows that numbered item of the topic in the
   language of the program and the items after it, up to the item of
   the next stage in the same section; `n = 0` shows the whole section
   (a section with paragraphs or bullets only, as "Check it"). The
   translations have the blocks of the English topic (ADR 0084), so the
   heading is found by its place among the `##` headings. A step with
   only `item` is a stage that the user only reads (work at the
   machine). Each numbered item of the sections before "If it does not
   look right" is in a stage. The markers of the 31 step files were
   written for this milestone (three agents, each with a set of files,
   checked by the tests below).
2. **Steps for the test only.** `test_only = true` marks a step that
   the test needs and the topic does not ask for (an Esc that closes the
   menu of Thread Look, a Cancel where the user saves a file, a second
   click that turns a view off). The lesson does not mark it and does
   not wait for it; the test does it.
3. **What the lesson sees.** `steps.rs` (program code now) has the step
   model, the checks (`Expect::misses`) and the search for widgets
   (`Screen`) that the tests used; the lesson and the tests share them,
   so the checks of a lesson are the tested checks. The lesson finds the
   widgets through the screen reader tree of egui: an egui plugin
   (`lesson::Eyes`) keeps the tree of each frame, and the clicks, keys
   and text of the user before the program takes them. The screen reader
   tree is on while a lesson shows (and stays on if a screen reader had
   it on before). The ribbon keeps the places of its buttons while a
   lesson shows (`ribbon::RECORD`, `ribbon::button_rect`). The widgets
   of the lesson window and the Help window do not count: their texts
   name the buttons of the steps. The runs of text in a label are not
   widgets (`Role::TextRun`).
4. **A step is done** when the user did it and the design is as its
   `expect` says. Did it: a click in the mark (a button, a field, the
   item of an open list), a command that ran (`AppState::run`), a start
   design that opened, a drag or clicks on the canvas, a key, typed text
   (in a field, or where no field shows it: the value of a sketch
   dimension), the value of a field or a list, a window that closed, the
   current thread. The checks of a lesson leave out the numbers that
   depend on the machine, the hoop and the hand of the user (stitches,
   trims, the size, the warnings); the tests check all of them.
5. **The marks.** An orange frame round the widget (or round the way to
   it: the tab of the command, a folded panel, the arrow of a drop-down
   button), numbered rings on the canvas for clicks (a dot: the left
   button; a ring: the right button), two rings and an arrow for a drag.
   The window says what to do in one line ("Click **Artwork > Borders
   and Offsets > Add Border**.").
6. **Going on.** A stage where the user does something goes on by itself
   a short time after it is done; a stage that the user only reads, or
   where the lesson only checks the design, waits for **Next ▶**. When
   the user did a step and the design is not as the step says, the
   window says what is not right (the objects, the selection, a window)
   and to click Undo or Skip. **◀ Back**, **Skip ▶**, **Tutorial** (the
   topic at the section of the stage) and **Stop the Lesson**. The place
   of a lesson that stops is `Settings::lessons`; the card of the
   tutorial then has **Go On at Step n** and **Start Again**.
7. **Ways to start.** **Start the Lesson** on the card of each tutorial
   in the Help window (a link `lesson:<id>`, also in the topic "All
   tutorials"), and "Your first design as a lesson" on the start page.
   The Help window closes when a lesson starts. The lesson window docks
   at the bottom right corner of the canvas (`dock::docked_at`).

## Tests

- `each_lesson_follows_its_steps` (the exit test): for each tutorial
  (and the first one in Japanese), the test starts the lesson and does
  the steps. Before each step of the user, the lesson must be at that
  step (the test clicks **Next ▶** for the stages that the user only
  reads), a click of the step must be in the mark of the lesson, and
  after the step the lesson must see it done; at the end the lesson is
  finished. A command that opens a window of the system is not clicked
  by the test: the test tells the lesson that it ran.
- `each_step_file_is_a_lesson`: the stages of each step file are items
  of the topic in each language.
- The tests that did the step files before still do (their runner uses
  `steps.rs`).

## Consequences

- A new step file needs its markers (and `test_only` where the topic
  does not ask for a step). A change of the numbered items of a tutorial
  can need a change of its markers; the tests say where.
- The agents that wrote the markers found places where the topic and
  its steps do not agree (for example, a step of the fonts tutorial
  types a search that the text does not ask for). They are in
  `docs/design/p19-plan.md`. The topic of the Japanese text now says to
  select the lettering before it sets the height.
- Lessons of the user and of shops (Part 1, Milestone 3, item 4) are not
  in this milestone.
