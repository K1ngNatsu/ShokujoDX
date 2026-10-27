# ADR 0020: Team names and keyboard design collections

Date: 2026-10-22 · Status: Accepted

## Context

P4 Milestone 6 adds team names (one design, many names, for example on a
set of shirts) and keyboard design collections (a design for each key,
typed like letters). The exit test: a list of 20 names exports in one
step.

## Decision

- **Team names live in the design.** `Design::team_names` has columns
  (lettering objects of the design, for example the name and the number,
  each with an optional width limit) and rows (a value for each column and
  a quantity). The list is part of the undo history and of the project
  file.
- **The design of a row** is the design with the values of the row in the
  column objects (a new layout of each; an empty value removes its
  object). A longer text gets a smaller height to fit the width limit.
- **Two outputs.** Save each name writes one machine file for each row (in
  the formats of the output profile), named after the design and the
  first value of the row, in one step. All in one design puts every row
  (as often as its quantity) in a grid centered on the hoop, in a new tab,
  and sews all objects of one color before the next color, so the machine
  changes color only as often as the design has colors.
- **Import.** Pasted text or a CSV file: one row for each line; the values
  are separated by tabs, commas or semicolons, and can be in quotes.
- **Keyboard design collections are our own JSON format** (`.embkdc`):
  for each character, a key design with an advance and objects with their
  colors, at a design height. Typing places the key designs side by side
  and sews them color by color. A key design comes from selected objects
  (their bottom is the baseline). One collection is built in: the letters
  of Block Sans over a shadow in a second color. The user's collections
  are in the `keyboard` folder next to the settings.

## Consequences

- The all-in-one design can be larger than the hoop; the app says so.
  Splitting into hoopings is P7.
- Key designs are fixed objects: they scale, but do not get the kerning
  or the baselines of lettering.
