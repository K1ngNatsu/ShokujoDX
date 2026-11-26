# ADR 0167: Travel that shows

Date: 2026-11-25 · Status: Accepted (P22 Milestone 1,
`docs/design/p22-plan.md`).

## Context

The plan hides travel where it can: under objects that sew later
(hidden travel, ADR 0082) and along the rows of a tatami fill (ADR
0166). But the program did not tell the user where travel lies on top
of stitches that sew before it, or on the fabric. The measure of ADR
0166 was a test for tatami fills only, and the stitch records do not say
which stitches are travel.

## Decision

- **The plan marks travel.** `Run::travel` gives the segments of a run
  that are travel: `Sewer::go_to` marks its stitches
  (`Sewer::finish_with_travel`; `step_to`, the step from row to row, is
  not travel), the fills keep the marks (`object::fill_runs`), and a run
  of a fixed letter is all travel. The plan keeps the marks through the
  choice of the start and the direction (`plan::route`) and adds the
  stitch to the next run of the same color when the gap is short, the
  stitch of `RunLink::Travel` and hidden travel. `StitchPlan::travel` is
  the ranges of the records that end a travel stitch. The stitches do not
  change.
- **The measure** is `emb_stitch::shown_travel` (not a port). It looks at
  each travel stitch at points 0.2 mm apart, but the first and the last
  0.5 mm of a travel run (they join the stitches before and after it):
  - a stitch that is not travel and sews after it, nearer than 0.3 mm,
    covers the point;
  - else the stitch sewn last before it at the point is the top. If the
    travel crosses it (20° or more), the travel shows on top of sewn
    stitches, but at the edge of an area: where the needle path of the
    stitches under it turns (the ends of rows, the sides of a satin)
    nearer than 1.2 mm. If the travel goes along the top stitch (nearer
    than 0.45 mm), it lies between the rows and does not show;
  - else, with no stitch near, the travel shows on the fabric.

  The first measure counted all stitches under the travel, at the needle
  points of a row. The pictures of the corpus showed that it marked the
  travel along the edge of a fill (it goes 0.3 mm inside the edge, over
  the ends of the rows) and along the lane between two sections (it lies
  between the top rows, over the underlay). The rule of the top stitch
  and the turns of the needle path took these away.
- **In the program.** Design info shows the travel that shows on
  stitches and on the fabric. An object gets a warning for more than
  2 mm on top of sewn stitches, or for a piece of 4 mm or more on the
  fabric. A short travel stitch to the next part is normal: about 3 mm
  between the parts of 5 mm letters in the tutorial of Japanese
  lettering. Lace gets no warning (lace is all thread). View > Check >
  Travel That Shows draws the pieces: red on sewn stitches, blue on the
  fabric.
- **The corpus snapshot** `crates/emb-cli/tests/travel_shows.rs` keeps the
  numbers of each corpus design; a change of the travel shows there. The
  golden snapshots do not change.

## The audit of P22

The maps of the measure had a cell of 1 mm for each place of the box of
all the travel of a plan: travel at two places 100 m apart needed 10^10
cells, more memory than a computer has, and design info makes the measure
for each design. The maps keep only the blocks of 16 × 16 cells with
travel now (`CellMap`): the results are the same (the corpus snapshot),
the measure of the corpus takes 38 ms in place of 31 ms, and travel
1.8 km apart takes 2 ms. Each piece has the record where it starts
(`ShownPiece::record`), and `ShownTravel::by_object` gives the length on
sewn stitches and the longest pieces of each object in one pass.

## Consequences

- In the corpus (86 designs), the measure finds travel that shows in
  places that are real faults:
  - guided fills (55 and 42 mm) and a circular fill (37 mm): travel
    straight across rows that are sewn; ADR 0166 changed only the
    travel of tatami fills;
  - in `shaping`, a fill whose travel goes across its own cut-out part
    on the fabric (19 mm in one piece);
  - in lettering of `embroidery-fonts-3`, `-4`, `-6`, `-7` and of the
    towel and cap sew-out designs, travel of 8 to 20 mm in one piece
    across the fabric (for example at the "T" of a script font);
  - 4 mm on top of sewn stitches in a letter of `embroidery-fonts-9`.

  ADR 0168 fixes them, but for a link of the route of a font
  (`docs/design/open-work.md`). Milestone 2 uses the measure for the
  live numbers, and Milestones 5 and 9 show it.
- Satin stitches turn at each needle point, so travel on top of a satin
  column narrower than about 2.4 mm counts as the edge. The travel of a
  satin walk (ADR 0163) is not marked yet: it lies under the top stitches
  of its column.
- The measure runs once for each plan. Over the corpus it takes about
  11 % of the time of the plans (32 ms against 300 ms, at most 5.5 ms
  for one design); the test `the_measure_of_the_corpus_is_quick`
  (release) keeps it under 15 %. A stitch far from all travel is left
  out with one look at a table of sums of the cells that the travel goes
  through.

Note of 2026-11-26 (ADR 0178): a turn hides travel at the edge of an area
only if it is a turn of the object of the stitch on top at that point. The
turns at the sides of letters hid travel between the letters on a fill.
