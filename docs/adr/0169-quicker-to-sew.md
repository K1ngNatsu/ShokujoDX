# ADR 0169: Quicker to Sew and the trim distance of a design

Date: 2026-11-25 · Status: Accepted (P22 Milestone 2,
`docs/design/p22-plan.md`).

## Context

The trim distance was a setting of the machine only
(`MachineProfile::trim_gap_mm`, My Machines): a gap to the next part of
the same color up to it gets a travel stitch, a longer gap a trim. A user
who wanted fewer trims on one design changed it for all designs of the
machine, and saw the cost (the thread that shows) only in a sew-out.
The ways to make a design quicker to sew were in different places: Fewer
Color Changes (Arrange), Clean Up (Edit), the colors of the threads by
hand. The idea of one window with the numbers before and after is from
zigzag (an open-source program under Apache-2.0); the code is our own.

## Decision

- **The trim distance of a design.** `Design::trim_gap_mm` (`None`: the
  machine decides; saved in the project file, 0 to 50 mm, else the edit
  is refused, `project::out_of_range`). `build_plan_until` uses it in
  place of `PlanOptions::trim_mm`, so each place that makes a plan (the
  canvas, the machine files, `emb-cli`, the library) uses it.
- **`emb_model::quicker`** (an engine module, so `emb-cli` and the agents
  of Milestone 9 can use it): `QuickerToSew` with four parts, each on or
  off, and `apply`:
  - merge the threads whose colors differ by the value or less (CIE76,
    `Lab::distance`; 5 at the start): the thread that sews more (its
    stitches in the plan, given by the caller) stays, and the threads
    that no object uses go;
  - remove the fills, satin columns and lines whose box is smaller than
    the size on its longer side (1 mm at the start), but not those in a
    group (monograms, blends), in a record (border traces, linked shapes,
    sketches, buttonholes, appliqué, template marks) or locked;
  - fewer color changes (`Design::optimize_color_changes`);
  - the trim distance of the design (3 mm at the start; off: `None`).
- **The window** (`emb-app/src/quicker.rs`; Arrange > Sew Order > Quicker
  to Sew…) works as Clean Up: each change of a value applies the settings
  to the design as it was when the window opened, as one merged edit; OK
  keeps it as one undo step, Cancel undoes it. A table shows the threads,
  color changes, trims, stitches, sew time and the travel that shows (ADR
  0167) before and after; "after" shows when the plan of the new state is
  made (the plan thread), else "…". While the window is open, the canvas
  marks the travel that shows, as View > Check > Travel That Shows. The
  help topic is "arranging".

## Consequences

- The tests `ok_is_one_undo_step_and_the_numbers_are_the_ones_shown` and
  `cancel_goes_back_to_the_same_design` (the same project text). The
  numbers after a change show in less than 1 s in the test; the plans of
  the corpus take at most about 60 ms each in a release build
  (`embroidery-fonts-8`), so the numbers of each corpus design follow at
  once.
- A design with its own trim distance sews the same on each machine in
  that respect; the machine setting stays the default for designs with
  none. The golden snapshots do not change (no corpus design has one).
- Merging threads changes the threads of all colorways: the thread that
  goes is removed from each.
