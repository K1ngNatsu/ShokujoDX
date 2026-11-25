# ADR 0172: Check and fix machine files

Date: 2026-11-25 · Status: Accepted (P22 Milestone 5,
`docs/design/p22-plan.md`).

## Context

Recognize Objects (ADR 0134) changes a machine file into objects. Many
users only want to sew a file from a customer with no faults: a stitch
that the machine cannot sew, a color change to the same thread, a jump
that leaves a thread across the design, a start that comes loose. The
idea is from zigzag (an open-source program under Apache-2.0); the code
is our own. The writers of the program were checked only by its own
readers.

## Decision

- **The engine** is `emb_art::file_check` (in `emb-art`: the travel
  check uses `emb_art::parts`). It reads the records as a machine sews
  them: a **piece** is the needle points between two cuts (a trim, a
  color change, the end); a jump with no trim does not cut. `check` finds,
  with the stitch lengths and the trim distance of `PlanOptions`:
  stitches shorter or longer than the machine sews (with a margin of
  0.05 mm for the 0.1 mm grid of the files; a stitch in the same needle
  hole is not counted), color changes to the same thread (color, brand
  and code), jumps longer than the trim distance with no trim, pieces
  with fewer than three needle holes (too short for the trimmer), starts
  and ends with no lock stitches in their seven first or last needle
  points (a needle point back in the hole of the needle point two before
  it, or three stitches of 0.7 mm or less), and travel that shows. The
  travel of a file is a run part between two other parts of one chain
  that goes almost straight (at most 60 mm, and at most 1.6 times the
  distance between its ends plus 2 mm: the curved rows of a spiral fill
  are runs too); `emb_stitch::shown_travel` (ADR 0167) measures it.
- `fix` makes the fixes that are on, in this order: join the color
  blocks of the same thread (a trim stays where the move to the next
  block is longer than the trim distance), remove the short stitches
  (the next stitch starts at the needle point before), split the long
  stitches into equal stitches, put a trim before each long jump, and
  add lock stitches (forward and back twice, as the stitch plan) at the
  starts and ends with no lock. The other needle points do not move.
  Pieces too short for the trimmer and travel that shows have no fix.
- **Stitch objects as a file.** `emb_model::records_of_stitch_objects`
  is the reverse of the import of a machine file: the records of stitch
  objects, with their threads (and ids) and the object of each record.
  `objects_from_stitches` (split from `design_from_stitches`) makes the
  objects again. Each run of stitch objects that sew one after the other
  is one file.
- **The app.** Design info shows the faults of the stitch objects (one
  warning for each object and kind, with the number and the first place;
  short stitches are in the check of each object already). A start of an
  object with Tie-In and an end of an object with Tie-Off are not faults:
  the stitch plan locks them. Fix Stitches keeps Tie-In and Tie-Off of a
  run of objects that all have them, and adds no locks of its own there. Edit >
  Machine Functions > Fix Stitches… (`fix_stitches.rs`): a check box and
  the number for each fix, Apply Fixes (one undo step) and the faults
  that have no fix. The checks are kept for the state of the design
  (`AppState::stitch_checks`). The fixed objects take the place of the
  run in the sew order; blocks of one thread become one object.
- **The command line.** `emb-cli check <file>` prints the number and the
  first places of each fault; `emb-cli fix <in> <out> [--fixes ...]`
  writes the fixed file. Both take a machine file or the stitch objects
  of a design, and `--machine`.
- **A cross-check of the writers.** `tools/check-writers.py` writes the
  corpus in each format with `emb-cli`, reads the files with pyembroidery
  (MIT license, not a part of the program, installed only to run the
  tool), and compares the needle points (to 0.15 mm, after a move of the
  whole design and a turn of the y axis, which it reports) and the color
  changes with the stitch plan. A stop that a format writes as a color
  change is a reason, not a difference.

- **Faults of Recognize Objects that the check found** (the tutorial
  "a file from a customer"). The stitch object of the stitches that stay
  joined pieces that were not next to each other in the file with a jump
  and no trim, and the objects had Tie-In and Tie-Off off (the functions
  of an imported file, whose lock stitches are in the stitches), though
  their stitches no longer have the locks of the file. Now
  `part_objects::stitches_kind` puts a trim where such a jump is longer
  than the trim distance, and the objects that Recognize Objects makes
  get Tie-In and Tie-Off (`recognize_objects::replace_locked`; an object
  that stays as it was keeps its functions).

## Consequences

- The exit test in software is met: a file with each fault is fixed with
  the right count for each fix, and the needle points that no fix
  touches keep their place exactly (`emb_art::file_check` tests, and on
  a corpus file `tests/file_check.rs`); the files of the program have no
  long stitch, no color change to the same thread, no long jump, no piece
  too short and no travel that shows. The short stitches that the check
  finds in the files of the program come from short stitches of the
  designs, which Design info shows too; cutwork and appliqué placement
  lines have no lock stitches by design (ADR 0051).
- The cross-check needs pyembroidery: a download, with the owner's
  permission. Its report is in `out/check-writers/report.txt`.
- Removing a short stitch in a lock pattern can remove the lock; the
  lock fix after it adds a new lock. The numbers of a fix can then be
  larger or smaller than the numbers of the check; the window shows the
  numbers of the check, and the message after Apply Fixes the numbers of
  the fix.
