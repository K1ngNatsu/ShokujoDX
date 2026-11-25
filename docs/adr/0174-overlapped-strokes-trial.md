# ADR 0174: Overlapped strokes in letters: a trial, not adopted

Date: 2026-11-25 · Status: Accepted (P22 Milestone 7,
`docs/design/p22-plan.md`; the owner's choice: keep the way of today).

## Context

Today a stroke that ends on a stroke that goes through ends at its edge,
and goes 10 % of its bridge into it (`split::BRIDGE_OVERLAP`, ADR 0152,
0153, 0159). A hand digitizer often sews such a joint otherwise: the
stroke that ends goes on under the other stroke to the middle of the
joint and sews first, and the other stroke sews from end to end. zigzag
(an open-source program under Apache-2.0) measured more clean letters
with it (81 % against 74 %, with its own measures) and about one sixth
more trims.

## Decision

The trial was made and measured, and it is not adopted. Its code is on
the branch `trial/overlapped-strokes` (commit 8c4b4e8): the split keeps
the ends of its pieces at bridges (`split::Joint`, with the distance to
the middle of the stroke on the other side), `glyph::overlap_joints`
takes each such end of a column on to that middle after the tuning of
the ends, and the route sews a column that ends in another first.
`EMB_OVERLAP_TRIAL=1` turned it on for Latin letters; `=2` also took the
ends at cuts from notches.

The measure (the letter fault bench in TrueType lettering with the
limits of the eye of ADR 0173; the numbers are in the plan): 238 letters
of 248 clean to the eye in both ways of the trial, against 240 today;
the same open area (the joints of this kind are covered today already;
the open areas that the eye sees are at other joints); the same trims;
51 % and 64 % more area with two layers of thread (thicker, stiffer
joints); bulges of 3.0 to 3.5 mm in three letters. In the flat proof
pictures the ways look almost the same. The owner chose the way of
today.

The main branch keeps the measures of the trial: `Faults::two_layers`
and `letter_faults::eye_summary` (`emb-cli letter-faults --judged`).

## Consequences

- No change of the stroke finder, the route, the fonts or the snapshots.
- A later idea for joints is measured in the same way: the clean letters
  to the eye, the open area, the two layers and the trims
  (`eye_summary`), against the numbers of today in the plan.
- The open areas that remain (Anton N, Y, a, Pacifico p, Block Sans 6)
  are at joints of other kinds (a diagonal or a curve that meets a stem
  at a corner); they are a task of their own if the owner wants it.
