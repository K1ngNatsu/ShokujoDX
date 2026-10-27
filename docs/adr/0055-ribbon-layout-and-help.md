# ADR 0055: Ribbon layout, help topics and the translation sweep

Date: 2026-10-25 · Status: Accepted (polish after P12)

## Context

The ribbon grew phase by phase, so some commands were where their phase
put them, not where a user looks for them: Quilting and the copy
arrangements were on the Artwork tab, the knife was on two tabs, Puff was
a stitch type, Laydown was a fill tool, Closest join (a sew-order
command) was with Split and join, the design library was on the View tab,
and the View tab had one panel for the stitch cursor and the filters.
Four commands shared the icon of Simple mode, and about 25 pairs of
commands on one tab shared an icon. The help had 11 topics (about 6,500
words); many features had 30 to 60 words, and Picture to design had no
reference topic. F1 knew only the ribbon buttons and the tools. Many
labels in dialogs came from plain string lists, so they were never in
the translation files.

## Decision

1. **The ribbon** (`emb-app/src/ribbon.rs`), by the work that each tab is
   for:
   - Specialty: Cross stitch (with Picture to cross stitch and Chart),
     Rhinestones, Cutwork, **Quilting** (from Artwork) and **Puff and
     laydown** (Puff from Edit > Stitch types, Laydown from Digitize >
     Fills).
   - Artwork: Show joins Draw; Remove overlaps and Unlink join Shaping
     (they use its overlap and its links); Border and Offsets are one
     panel **Borders and offsets**; the knife is only on Edit.
   - Arrange: a **Copies** panel (Arrangements, the mirror copies,
     Reflect, from Artwork); **Closest join** goes to Sew order.
   - Edit: Lines is **Lines and columns** (it has Swap sides).
   - View: **Stitch cursor** and **Filter** are two panels; the library
     goes to Design.
   - Design: **Rotate design** goes to the Hoop panel; **Library and
     templates** has the design library, the templates and My assets.
   - Color: the panel of the background color is **Fabric color**, so it
     is not the same name as Design > Fabric.
   - Help: the Tutorials panel has all seven tutorials.
   The test `the_commands_of_a_tab_have_different_icons` keeps the icons
   of one tab different (the tutorials and the 2× zoom buttons, which
   show text, are the exceptions).
2. **The help** (`docs/help`, 34 topics, about 23,000 words): the guide
   is split by task, with new topics for editing, arranging, stitch
   types, lettering, pictures, artwork, fabrics and styles, machines and
   hoops, hooping, checking, files and output, the library, cross stitch,
   rhinestones, quilting, cutwork, options, file formats, a glossary and
   embroidery basics, and four new tutorials (appliqué, towel, two
   hoopings, lace). The contents has the groups Start, Tutorials,
   Designing, Production, Specialty, The ribbon and Reference. The
   Markdown has tables (`| a | b |`); each row is a passage of the
   search.
3. **F1 over a window** opens the topic of the window
   (`help::WINDOW_HELP`, the id of the window and the topic;
   `help::window_topic` finds the window under the pointer by its layer
   id). A ribbon button under the pointer comes first, the active tool
   last. A window that changes its title has a fixed id (Lettering,
   Monogram, the template window), so it also stays in its place.
4. **Translations**: labels in string lists, name functions, the
   right-click menus of the canvas and the warnings of Design info go
   through `tr` or `fill`. Names in constants
   (the box places of the start point, the kiosk colors, the panel names,
   the library columns) are shown with `tr(name)` and listed in
   `i18n::tests::engine_texts`. File dialog titles and undo names stay
   English.

## Consequences

- Users who knew the old places find the moved commands with Ctrl+K and
  in the ribbon topics; the help names the new places.
- A new window with a help topic gets a row in `WINDOW_HELP`; the test
  `f1_over_a_window_opens_its_topic` checks that each topic exists.
- A new label in a list needs `tr("…")` at the literal, or a line in
  `engine_texts` for a constant.
