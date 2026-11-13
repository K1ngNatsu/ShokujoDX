# ADR 0090: Close Gaps, Clean Up and the lettering preview

Date: 2026-11-13 · Status: Accepted

## Context

The owner asked for three changes:

- Lines drawn one at a time (Freehand, Line) do not join easily. Join
  joins only ends that touch (0.05 mm).
- Lines and outlines from SVG files can have many small overlaps and
  zig-zags. Simplify keeps them: it only removes nodes within a tolerance.
- The Lettering window shows the lettering only after OK, and the font
  list shows the names in the font of the program.

## Decision

- **Close Gaps (`emb_geom::gaps`, app `gaps.rs`).** The open ends of the
  selected lines (running stitch, or vector lines) of one color join when
  they are nearer than the gap, the nearest ends first, each end one time.
  Two ends meet halfway between them; the handles move with the nodes, so
  the directions stay. Before that, two lines that cross near both of
  their ends (an overshoot, less than the gap from each end) are cut at
  the crossing. A chain whose two ends are near closes if it is longer
  than four gaps. Each changed part becomes one object in the place of its
  first line; lines with no change stay. An end near the middle of a line
  does not join it. The window starts with a gap that joins each end that
  has another end within 10 mm (`gaps::suggested_gap`), because the gaps of
  lines drawn by hand are often 3 to 7 mm.
- **Clean Up (`emb_geom::cleanup`, app `cleanup.rs`).** One detail size
  controls three steps on the flattened subpaths: small loops, overlaps
  and spikes go (the line crosses itself, or comes back to less than half
  the detail size, within four detail sizes); zig-zags are smoothed (three
  passes of a moving average on points a quarter of the detail size apart,
  cut at the corners: turns of more than 55° over two detail sizes); tiny
  parts go. Curves are then fitted with `simplify`, and straight curves
  become lines. A fill makes its areas again from the cleaned rings
  (union), so parts of its outline that overlap become one area. The curve
  fit does not let a handle be longer than the points it fits, and it
  measures the error between the points too, so no curve makes a spike. Satin
  columns, appliqué and lettering do not change.
- **Lettering preview.** While the Lettering window is open, the design
  has the lettering of the window: a new object, or the changed object
  (made again from the object as it was when the window opened). All the
  changes of one window merge into one undo step (`Document::is_merging`
  tells if it is still the last step). OK keeps it; Cancel, Esc and the
  close button take it back. A font under the pointer in the list shows
  on the canvas; when the pointer goes, the font of the window shows again.
- **Font names in their font.** The font list draws each name with the
  outlines of its font (`FontLibrary::sample`; the parts of embroidery
  letters, or the glyph contours of a TrueType font), filled with the
  scanline mesh of the appliqué preview. A font that is not in use is read
  and let go; a frame makes at most six new samples, and the list draws
  only the rows that show. A font with fewer than half of the characters
  of its name (a symbol font) shows the name as text.

## Consequences

- Close Gaps and Clean Up change the shape a little (ends move half the
  gap; Clean Up stays within about a quarter of the detail size, but a
  sharp point narrower than half the detail size gets shorter). The
  windows show the result before OK.
- Opening a Lettering window for new lettering and typing adds an object
  at once; other edits while the window is open end the merge, and then
  Cancel leaves the lettering.
