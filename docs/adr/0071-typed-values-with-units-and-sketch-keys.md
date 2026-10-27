# ADR 0071: Typed values with units, and the keys of sketch mode

Date: 2026-10-27 · Status: Accepted (P14, after the owner tried Milestone 3)

## Context

The owner tried sketch mode and found that Esc did not end a command,
Delete did not delete a line or a dimension, the value box of a
dimension could not be closed, and a value could not be typed in other
units. The cause of the first three: the keys of the program
(`AppState::handle_shortcuts`) run before the canvas. They took Esc (clear
the selection of objects) and Delete (delete the selected object, which in
sketch mode is the sketch). The value box asked for the focus again in
each frame, so it kept the keys and the clicks.

## Decision

1. **Sketch mode takes its keys first.** `sketch::handle_keys` runs in
   `handle_shortcuts` before the other keys: Esc ends the command and goes
   back to Select (in Select it clears the selection), Delete and
   Backspace delete the selected parts (a point with its curves), Enter
   ends a chain of lines, Ctrl+A selects all. While the value box is open,
   it takes all keys; Esc closes it. Undo, redo and the other keys of the
   program still work.
2. **The value box** takes the focus once and selects its text, so typing
   replaces the old value. Enter or OK applies the value, Esc cancels, a
   click somewhere else applies it.
3. **Typed values with units** (`workspace::parse_length`,
   `parse_number`). A length field accepts a number in the units of the
   Options, or a number with its own unit (`mm`, `cm`, `m`, `in`, `"`,
   `ft`, `'`, `pt`), fractions (`3/4 in`, `1 1/2"`, `1-1/2"`), and
   `+ - * /` (also `x`) with parentheses. A number with no unit in a sum
   with a length takes the units of the Options. All length fields of the
   program (`workspace::length_value`) use it, and the value box of a
   dimension.
4. **A right-click menu** in sketch mode: it selects the part under the
   pointer and shows the commands that fit the selection: Change Value,
   Dimension, the constraints that the parts can take (not the ones that
   they have), Trim, Split, Extend at the click, Fillet and Chamfer at a
   corner, Construction, Offset, Mirror, the patterns, Delete Constraints,
   Delete, Select All and Finish Sketch. A right click while a draw tool
   has clicks ends them, as before.
5. **More help to see and select**: the part under the pointer lights up;
   a drag on an empty place selects the parts in a box; lengths and
   distances go on the side away from the middle of the sketch, with
   extension lines; a radius label goes out of its arc. The Properties
   panel has the units (mm, in), and the values of the edit tools are
   round numbers in those units.

## Consequences

- The units of a new user are mm (the default of the Options); the
  program does not read the units of the operating system.
- A value with a unit that the program does not know is refused with a
  message.
