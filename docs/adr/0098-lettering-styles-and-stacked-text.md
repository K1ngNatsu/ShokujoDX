# ADR 0098: Bold, italic, underline and stacked lettering

Date: 2026-11-16 · Status: Accepted

## Context

The owner asked for bold, italic and underline options for fonts, and for
a text direction with stacked letters (each letter upright, from top to
bottom or bottom to top; v0.3 plan, Stage 5). Many fonts have no bold or
italic face, and the bundled embroidery fonts have one face each. The
owner accepted made styles for those, not disabled buttons.

## Decision

- **Model.** `Lettering::bold` and `italic` are made styles;
  `Lettering::underline` (`Underline`: gap from the baseline and width of
  the bar, in mm) and `Lettering::direction` (`TextDirection`: Horizontal,
  Down, Up). All have serde defaults, so older files open as before.
- **Faces first.** The Style buttons of Object Properties use the faces
  of the family (`lettering::styled_face`): the face with the most of the
  asked styles (for example Bold when the family has no Bold Italic), then
  a made style for the rest. A face name is bold if it has "bold", italic
  if it has "italic" or "oblique"; plain names ("Bold", "Bold Italic")
  come before others ("ExtraBold").
- **Made styles** (`emb_letter::style`). Bold: each satin rail goes out
  from the other rail and the column grows at its ends, by
  `bold_width` (4 % of the capital height, 0.15 to 0.8 mm); the stitch
  direction lines keep their ends on the rails; a fill grows by an offset
  of its outline. The advance of each letter gets both sides. Italic: a
  slant of 12° about the baseline. Lines (running parts) do not change.
- **Underline.** A satin bar under each line of horizontal text, from the
  start of the first letter to the end of the last, along the baseline
  (also an arc or a path; the envelope bends it too). It is a `Letter`
  with no text at the index of the line break, so it sews with the
  letters, takes part in the knockout and the border trace, and has no
  handle in letter editing.
- **Stacked text.** `layout::stacked` puts each letter upright, centered
  on the vertical line of its column, a step of 1.3 capital heights plus
  the letter spacing apart (a space is 0.6 of a step). Each text line is a
  column; columns go from left to right, the widest letter plus 0.3 of the
  height apart (times the line spacing). Justify puts the top, the middle
  or the bottom of each column at the start point. Stacked text has no
  baseline curve, envelope or underline.

## Consequences

- A made bold is an approximation: where columns of a letter meet (the
  middle of a "B"), the wider columns can overlap more than in a real
  bold face. A font with a bold face looks better.
- Stacked text of a script font does not join its letters.
