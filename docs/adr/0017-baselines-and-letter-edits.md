# ADR 0017: Baselines and changes to single letters

Date: 2026-10-22 · Status: Accepted

## Context

P4 Milestone 3 puts lettering on arcs, circles and paths that the user
draws, and lets the user change one letter by hand: kerning, move, size,
turn, and the shape of the letter. The lettering object must still make
its letters again from the font when the text, the font or a setting
changes (ADR 0015), and these changes must stay.

## Decision

- **The baseline is part of the lettering object.** `Lettering::baseline`
  is straight, an arc, a circle or a path, in lettering space. Layout first
  puts the letters on straight lines, then moves each letter as one rigid
  shape: the middle of its advance goes to the point at the same distance
  along the baseline, and the letter turns with the baseline there. The
  letters do not bend.
- **Arc and circle are different places for the origin.** An arc goes
  through the origin (the text stays where it is when the radius
  changes); a circle has its center at the origin (texts over and under a
  circle share the center when the radius changes). Clockwise text stands
  on the outside of the circle; counter-clockwise text hangs on the
  inside. A lower line is on a concentric circle.
- **A path baseline is a line of the design.** "Lettering on line" makes
  the selected line the baseline and removes the line object. Alignment
  refers to the path: start, middle or end. Letters past the ends continue
  in a straight line.
- **Changes by hand are `LetterEdit`s, kept by the character index in the
  text.** An edit has kerning (extra space before the letter; the letters
  after it on the line move too), a move along and down from the
  baseline, a size factor and a turn about the middle of the letter box,
  and an optional shape. When the text changes, the characters before and
  after the changed part keep their edits; the changed characters lose
  them.
- **A changed shape is kept in the letter frame.** Each letter keeps the
  transform from its own frame (pen point on the baseline, mm) to
  lettering space. When the user changes the nodes of a letter, its parts
  go back through the placement, the baseline and the edit, and the result
  is the shape of the letter. A new layout uses that shape (scaled with
  the height) while the character at that index is the same.
- **Move, size and turn need no font.** These change the parts of one
  letter in place. Kerning and baseline changes need a new layout, so the
  font must be on the computer.

## Consequences

- A design with changed letters sews the same on a computer that does not
  have the font.
- Letters on a tight curve keep their straight shapes, so they can touch
  on the inside of a small circle; the user changes the radius or the
  letter spacing.
- Edits follow the text by position only: a change in the middle of the
  text removes the edits of the changed characters.
- Fill angles of letters stay the angle of the object (with the turn of
  the placement); they do not turn with each letter.
