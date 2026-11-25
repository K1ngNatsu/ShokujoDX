# ADR 0173: Letter measures against the eye

Date: 2026-11-25 · Status: Accepted (P22 Milestone 6,
`docs/design/p22-plan.md`).

## Context

The letter fault bench (P21, `emb_cli::letter_faults`) counts faults
from the needle points of the letters: needle points out of the outline,
crossing pairs, fans, bulges, open area, open corners, columns on top.
Each measure has a limit, but no test showed that a letter that the
bench calls faulty looks faulty, or that a letter that looks faulty has
a measure that finds it. The idea is from zigzag (an open-source
program under Apache-2.0); the code is our own.

## Decision

- **The judged set** is `crates/emb-cli/tests/data/judged-letters.json`:
  the 496 letters of the bench (A-Z, a-z, 0-9 of Block Sans, Anton,
  Pacifico and Varsity, as the embroidery font and as lettering in the
  source font, 25 mm), each `clean` or with the fault that the eye sees
  (`open-area`, `open-corner`, `spike`, `bulge`, `crossing`), and the
  rule of the judgment. Claude judged them on 2026-11-25 from pictures
  with no marks of the measures (`tools/letter-faults.py --plain`: the
  area of the outline in gray, the stitches as thread 0.3 mm wide, the
  edge of the outline in red), and looked at the doubtful letters at a
  larger size next to each other. 16 letters have a fault (open area:
  Block Sans 6, Anton N, Y, a, Pacifico p; open corner: Anton r, x,
  Varsity a; each in both ways). The owner looks at 50 of them (all the
  faulty letters, the letters near a limit, the letters that the bench
  measures call faulty, and some at random; `out/p22-judged`) and
  corrects the set.
- The survey of 127 fonts (ADR 0158) is not in the set: its source fonts
  are not in the repository, and the test must run on each computer.
- **A new measure.** The open area of a letter (any open cell) called
  435 clean letters faulty: the slivers between the stitches of a curve
  are open area too, and they close when the thread spreads.
  `Faults::open_patch` is the largest connected patch of open cells
  after an erosion of two cells (0.1 mm; `largest_patch`, `PATCH_ERODE`).
  `Faults::out_max_mm` is the farthest needle point out of the outline.
- **The limits of the eye** (`EYE_*`, `eye_faults`): an open patch of
  0.38 mm² or more (the faulty letters have 0.41 mm² or more, the clean
  ones 0.35 mm² or less), open area at the convex corners of 0.5 mm² or
  more (0.54 or more against 0.45 or less), a needle point 0.6 mm out, a
  bulge over 3 mm, five crossing pairs. The last three have no faulty
  letter in the set, so the eye does not test them yet.
- `emb-cli letter-faults --judged <file>` (`compare`) gives the
  agreement, the letters where the measures and the eye do not agree,
  the measures of the bench with their own limits on the faulty and the
  clean letters, and the letters near a limit (within a quarter).

## Consequences

- The measures agree with the eye on all 496 letters (the exit test asks
  for 90 %). The limits come from the same set, so the letters near a
  limit (Pacifico p at 0.41 mm², Pacifico M at 0.35 mm², Anton x and
  Varsity a at 0.54 to 0.61 mm² of corner) are where the owner's review
  can move them.
- The measures of the bench keep their limits: they find faults to fix
  in the stroke finder, smaller than the eye sees at 25 mm (needle
  points 0.4 mm out, crossing pairs, columns on top). The report shows
  how many clean letters each of them calls faulty.
- A column on top of another does not show in a flat picture; the set
  does not judge it.
- The test `the_measures_agree_with_the_eye` uses the letters of
  `SNAPSHOT_CHARS` (fast); `the_measures_agree_with_the_eye_on_all_letters`
  (ignored, release) uses all. The snapshot of the bench did not change
  (the new measures are not in its table).
- Milestone 7 (the overlapped strokes trial) is measured with this set
  and these limits.
