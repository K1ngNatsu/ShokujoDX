# ADR 0058: Column by sides, the start tab, and title case

Date: 2026-10-26 · Status: Accepted (owner requests)

## Context

Four owner reports of 2026-10-26: the Column tool took points in pairs
(side A, side B, side A, side B), which is not intuitive; the program
opened on the ribbon tab of the last session; the name of a folded panel
(for example "Borders and offsets ▾") went over the panel title under it;
and the command names were in sentence case ("Dim vectors").

## Decision

1. **Column** draws one side, Enter, then the other side, Enter (the
   method of the old Column B). While the second side is drawn, a light
   fill shows the satin between the two sides. The method of pairs stays
   as **Column Pairs** (`Tool::ColumnB`); the icons change with the
   methods. The enum names do not change, so saved settings stay valid.
2. **The start tab**: `Settings::ribbon_tab` is not saved
   (`#[serde(skip)]`), so the program starts on the Digitize tab. A folded
   ribbon stays folded.
3. **Large buttons**: `ribbon::large_fit` makes a large button wider (up
   to 120 points) when its name would need more than two lines, also for
   a folded panel. A test checks all large names in the four languages.
4. **Title case** for the names of commands: the ribbon commands and
   tools, the panel titles, the drop-down buttons of the ribbon, the File
   menu, the right-click menus, the titles of command windows and the
   panel names. Each word starts with a capital, except a, an, and, as,
   at, but, by, for, from, in, into, nor, of, on, or, per, the, to, via
   and with inside a name (the last word always has a capital: Zoom In,
   Send To…). Menu paths in tooltips and in the help ("Edit > Stitch
   Types > Guide Line") and the names in bold in the help follow. Undo
   names that are command names follow too. The translations keep their
   text under the new English keys; the names of the engine (fill types,
   shaping names) keep sentence case, so their old keys stay.

## Consequences

A new command name is in title case. Sentences (tooltips, messages,
options in windows) stay in sentence case.
