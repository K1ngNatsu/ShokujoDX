# ADR 0010: A ribbon with tabs replaces the menu bar and the tool column

Date: 2026-10-21 · Status: Accepted

## Context

After roadmap P2 the app had 27 tools and about 60 commands. The tools were
in one long column at the left of the window, and the commands were in eight
menus. The owner found this hard to use and asked for a layout like CAD
programs or the reference product: clean tabs, with the commands sorted and easy
to find.

The prototype also showed a canvas speed test (synthetic stitches, 10k to 1M)
and frame times in the status bar. The speed test was a Phase 0 task. Real
designs now give the canvas its load, so these controls were only noise for
users.

## Decision

- A ribbon at the top of the window (`emb-app/src/ribbon.rs`), in the style
  of CAD programs. We describe and build it in our own words; we copy no
  assets.
  - A File button opens the application menu. New, Open, Save, Undo and Redo
    are next to it.
  - Five tabs: **Digitize** (select tools, lines, fills, columns, shapes,
    appliqué,
    backdrop), **Edit** (clipboard, select, split and join, holes and angles,
    line networks), **Arrange** (transform, align, space and size, group and
    lock, sew order), **View** (show, stitch cursor and filters, zoom,
    workspace) and **Design** (machine and hoop, fabric and styles,
    templates, output).
  - Each tab has panels with a name below them. Panels have large buttons
    (icon above the name) and columns of up to three small buttons.
  - Commands that need a selection are dimmed when there is none.
  - A double-click on a tab hides the panels. The tab and this state are user
    settings (ADR 0009).
- The ribbon is data: `Cmd` (one command), `Panel` and `Item`. One function
  runs a command (`AppState::run`) and one says if it can run
  (`AppState::can`). Tests check that every tool has a button, and run each
  command on a sample design and undo it.
- Icons: the Phosphor icon font through the `egui-phosphor` crate (MIT or
  Apache-2.0; font MIT). This is a pure-Rust crate with an embedded font, so
  rule 9 of the spec is kept. The tools that make stitches have our own
  vector drawings (`emb-app/src/icons.rs`): dashes for running stitches,
  zigzags for satin, hatch lines for fills.
- The thread colors are in a bar at the bottom of the window, above the
  status bar. The property bar stays below the ribbon.
- The right-click menus on the canvas do not change.
- The speed test and the frame times are removed from the app. The test
  `canvas::tests::mesh_build_timing` (ignored by default) keeps the speed
  check for developers.
- The window opens maximized, because the ribbon needs the width.

## Consequences

- The left tool column and the menu bar are gone. Each command has one
  place on the ribbon. The keyboard shortcuts do not change.
- A narrow window scrolls the ribbon panels left and right. Panels do not
  yet shrink to icons as in CAD programs; we can add this later.

## Update 2026-10-24 (owner request: no scroll bar)

- The panels that do not fit the window fold into one large button each,
  with the icon of the panel's first command and the panel title; a click
  shows the panel under it. The panels on the right fold first. The
  widths of the panels are measured as they draw (`AppState::ribbon_widths`).
  The scroll area stays only for a window narrower than the folded panels.
- A Lettering tab holds the Lettering and Personalize panels (as the
  lettering toolbox of consumer embroidery products); the Digitize tab no longer has the Import
  panel, which is on the Artwork tab (Insert stitches moved there too).
- A test checks that each tab fits a window 1366 points wide.
- P9 adds a Specialty tab (cross stitch, ADR 0038; later rhinestones).
- The SPEC layout section is changed from a left tool bar in the consumer style to the
  ribbon.
