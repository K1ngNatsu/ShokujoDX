# ADR 0165: Puff ends at the joints of letters

Date: 2026-11-24 · Status: Accepted (found by the steps of the tutorial
"Puff on a cap", P19 Part 2 Milestone 4; the owner: "begin M4").

## Context

A puff column has end stitches over each of its ends (ADR 0050): the
needle cuts the foam there, and the end is closed. In a letter, two
columns often end at the same place: the stem and the bowl of "P" meet
at its top corner, the legs of "A" at its apex. Each column put its end
stitches there, so a joint had two layers of end stitches on two layers
of dense satin. The lettering "CAP" in Block Sans at 20 mm on a cap
frame gave two "Thread builds up" warnings in Design Info (7.8 and 6.9
layers).

ADR 0163 already gives a satin part of a letter the areas of the other
parts of the letter (for its underlay), but not which of them sewed
before it.

## Decision

In a lettering object, a puff column sews no end stitches at an end that
lies in a part of the same letter that sewed before it, or nearer than
`satin::JOINT_MARGIN_MM` to it: the part that sewed first closed the
foam there. The column that sews first at a joint keeps its end
stitches, and a free end keeps its end stitches. `LetterSewer` keeps the
parts that are sewn (`LetterSewer::parts`), and `satin::satin_on` gets
their areas (`before`); `satin::in_parts` is the test of ADR 0163 for a
point in other parts.

## Consequences

- The top corner of "P" has one layer of end stitches. The apex of "A"
  still has 7.6 layers on 6 mm²: there the two dense legs lie on each
  other and come to a point. The tutorial says so: a letter with sharp
  points has more thread at its points than a letter with blunt ends
  (`docs/design/open-work.md`).
- A puff column that is not in a letter, and a column with no part sewn
  before it at its end, have their end stitches as before.
- The golden snapshots `puff` and `view-puff` change (10 and 8 fewer
  stitches); the commit message says why.
