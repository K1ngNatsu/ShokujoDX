# ADR 0092: Drawing aids from drawing programs

Date: 2026-11-13 · Status: Accepted

## Context

The owner compared ShokujoDX with a general drawing program and asked for
six features that help with lines drawn by hand and with shapes: Smart
Fill, trim between crossings, envelope and slant for any object, copy
properties, shape recognition, and repeat last command with break at node.

## Decision

- **Smart Fill** (tool, Digitize > Fills; `emb_geom::linework::enclosed_area`).
  The outlines of the shown lines, fills, satins, appliqué and vectors
  become bands 0.3 mm wide; the free area round the click (if it does not
  reach the frame round all lines) grows back to the middle of the bands
  and becomes a fill with the current fill settings. So lines can cross,
  gaps up to 0.3 mm close, and shapes inside are holes.
- **Trim Segment** (tool, Edit > Split and Join; `emb_geom::linework::trim_piece`).
  The crossings of the clicked line with all other outlines and with
  itself are found on short pieces of its curves (exact on lines). The
  part between the crossings on each side of the click goes; a closed
  line opens. Lines and vector lines only, so a fill never gets an open
  outline. Crossings at the ends of an open line do not count.
- **Envelope…** (Edit > Shape): `Envelope::warp_kind` (the lettering art
  envelopes) on the selected lines, fills, satins, appliqué and vectors,
  with the box of the selection, then Simplify at 0.03 mm. The window
  shows the result (one merged undo step). **Slant** (Object Properties >
  Position and size): horizontal and vertical degrees in screen
  directions, about the Scale from point (ADR 0091); the fields show the
  change in progress and go back to 0.
- **Copy Properties** (Edit > Shape): waits for a click (`Pick::PropertiesFrom`).
  The selection gets the thread of the clicked object, and its stitch
  settings where the kind is the same (for lettering: the font and the
  look of the letters, not the text, baseline, envelope or place).
- **Smart Drawing** (tool, Digitize > Lines; `emb_geom::recognize`): a line,
  rectangle (level within 6°), triangle, quadrilateral, circle or ellipse;
  else the smoothed stroke. The result is a running line.
- **Repeat** (Edit > Shape and the right-click menus): the command that
  Space already ran (`AppState::last_cmd`) is now on the ribbon and in the
  menus with its name. **Break at Node** (Reshape: B or the right-click
  menu; `NodePath::break_at`): an open subpath becomes two, a closed one
  opens; lines and vector lines only.

## Consequences

- Smart Fill can be slow on designs with very many outline points (one
  buffer of all lines for each click).
- Envelope changes curves to lines and fits curves again, so node counts
  change; it does not bend lettering (use Lettering art).
