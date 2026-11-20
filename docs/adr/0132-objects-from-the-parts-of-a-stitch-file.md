# ADR 0132: Objects from the parts of a stitch file

Date: 2026-11-19 · Status: Accepted (P18 Milestone 2,
`docs/design/p18-plan.md`)

## Context

Milestone 1 (ADR 0131) cuts the stitches of a machine file into parts:
runs, columns, rows and other stitches, each with its underlay. Milestone
2 makes objects with stitch types of the runs and the columns: a running
object and a satin column, with the settings that the stitches show. The
fills are Milestone 3; the command that changes a stitch object into
objects is Milestone 4. The native formats of closed-source programs stay
out of scope (rule 3).

## Decision

- **The engine** is `emb_art::part_objects` (not a port).
  `objects(&StitchDesign, &[Part])` gives the objects in sew order
  (`Made`: the object kind, the thread, and the stitch records of the file
  that it sews). A part that does not become an object stays stitches: the
  stitches of one thread one after the other are one stitch object (a new
  part after a jump, the trim kept). No stitch is lost: the records of the
  objects together are the records of the parts.
- `emb-art` now uses `emb-stitch` (the satin column of the engine pairs
  the rails, and the underlay settings give their layers). The two crates
  are at the same level; `emb-stitch` does not use `emb-art`.
- **Lock stitches**: short stitches at an end of a part that go back and
  forth within 1 mm and come back to their first point are not a part of
  the line.
- **A run** becomes a running object. When 60 % of its stitches sew three
  times (there, back and there again), it is a bean stitch (repeats 3).
  The line is the needle points with fewer points where they are on a
  straight line (0.1 mm). The stitch length is the length that 90 % of the
  stitches do not pass, to 0.1 mm: the engine gives the stitches between
  two corners one length, at most the stitch length. A line that ends at
  its start is closed.
- **A column** becomes a satin column:
  1. **Passes**: the stitches across go back and forth (a stitch across is
     0.3 to 4 times the median stitch across; a longer one is travel). A
     pass ends where the stitches go back 0.5 mm along the column (the
     direction along is from the mean of two stitches across, because a
     zigzag slants them one way and the other). A needle point at an end
     of a pass that is far from the needle point before it on its side
     (4 times the median step and 1 mm) is travel. The densest pass with 4
     stitches across or more, and each pass up to 1.25 times its spacing,
     is a top pass; a lighter pass before it is its underlay.
  2. **Rails**: the needle points of a pass are on the two sides in turn.
     A needle point is on its rail if it is not a short stitch (8 % of the
     width in from the line of its side, at least 0.12 mm) and it goes
     forward from the needle point before it on its side (at a sharp turn
     the needle points on the inside pivot and go back a little). The end
     of a rail where no stitch starts is half a step out (at most half of
     the median step). The rails are made smooth over 0.6 mm and have no
     point nearer than 0.05 mm to the line through the points next to it.
  3. **Rungs**: each needle point and the middle of the two needle points
     next to it on the other side are a pair of the stitches. The place of
     a needle point on its rail comes from the order of the points, not
     from the nearest point (a rail can come near itself, round a small
     counter). A rung is put where the pairs of the rails are more than
     0.25 mm from the pairs of the stitches.
  4. **Settings**: the spacing is two times the length along the middle
     divided by the number of stitches across. The pull compensation is 0,
     because the needle points are on the sides already. Short stitches
     are on if short stitches were found or no needle points are crowded.
     The split length is the default if it gives each stitch across its
     number of stitches, else it comes from the stitches. A spacing of 1 mm
     or more is a zigzag (open satin).
  5. **Underlay**: a lighter pass is a zigzag layer (its inset and its
     spacing; a pass out and a pass back are one layer). An underlay run
     along the middle is a center walk, along the two sides an edge walk;
     another run under the column is travel and becomes a running object
     of its own, before the column. When the layers are the layers of
     Auto, Light, Heavy or Double at the width of the column (the inset
     within 0.3 mm, the zigzag spacing within 20 %), the column gets that
     setting; else the layers (Custom), or no underlay.
- **A change of the parts** (Milestone 1): a run or a lighter part between
  two parts of the underlay of one part (or the underlay and the part)
  goes with that part too (a piece of a zigzag that the cells of the part
  do not cover). The exit test of Milestone 1 stays at 97.1 %.
- **The view**: View > Stitch Parts also draws the lines of the objects
  that the parts become (the line of a running object, the two rails of a
  satin column), with a key item.
- **The exit test** is the second test of `crates/emb-cli/tests/parts.rs`
  (the corpus as DST and back, the lettering cut into its letter parts).
  For each recognized satin, its source object is the object of most of
  its stitches. Only satins and lines with a border of the types satin and
  zigzag are measured, with no effect that moves the needle points (auto
  spacing, accordion, jagged edge, hand stitch, puff, spacing at a side).
  - The width is the distance from each side to the other at each pair
    (not the length of a slanted stitch). The median of the differences
    with the source at the same places must be within 0.2 mm. The sides
    of the source are its rails moved out along the stitches by the pull
    compensation, as the engine sews them; a border has its width and its
    pull compensation.
  - The spacing is compared with the spacing that the source sews: two
    times the length of its middle divided by its stitches across. The
    engine puts a whole number of stitches in each section between two
    rungs, so a column with many short sections sews more open or denser
    than its setting (a letter with 19 rungs sews 0.44 mm at the setting
    0.40 mm). It must be within 10 %.
  - 90 % of the stitches of the measured satins must pass.
- **Pictures** for a check by eye: `write_part_pictures` (the rails, the
  rungs and the stitches sewn again of each satin over the stitches of the
  file) and `write_whole_picture` (a design as its file and as its
  objects sewn again), both ignored tests in the same file.

## Consequences

- 98.9 % of the 222,257 stitches of the measured satins pass. 95.6 % of
  the 232,100 stitches of the columns of the corpus are in a recognized
  satin. Most misses are columns of fewer than 50 stitches (an end of a
  letter part) and the bars of buttonholes (a part with stitches of
  several objects).
- Runs are not in the exit test. 74.2 % of the 128 plain runs (90 % of
  their stitches from one source run, no hand-stitch effect) get the
  stitch length (10 %) and the bean setting of their source. A quilting
  line with corners near each other has no stitch as long as its setting;
  its stitches between the corners are the same at the length found.
- The objects of 18,000 stitches take 4 ms after the parts (45 ms).
- The stitch angle comes from the rungs; it is checked on the pictures,
  not measured.
- Open: the fills (Milestone 3), Recognize Objects (Milestone 4), and a
  test with machine files of other programs.
