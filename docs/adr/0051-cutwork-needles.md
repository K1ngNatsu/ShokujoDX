# ADR 0051: Cutwork needles

Date: 2026-10-25 · Status: Accepted (P11 Milestone 8, files not yet tried on a machine)

## Context

P11 Milestone 8 asks for lines that the cutting needles of Bernina,
Brother or Janome machines cut, with the needle angle of each line, in the
machine files that these machines read. The plan said that it waits for a
tester with such a machine and a public description of the files. The
owner asked to begin it (2026-10-25).

What is public: the Ink/Stitch documentation ("Cutwork", updated
2022-12-30; Ink/Stitch is GPL-3.0, spec rule 4) says that a kit has four
needles, each cutting in one range of directions, and gives the needle
order and the thread colors of Bernina, Brother and Baby Lock, Husqvarna
Viking and Pfaff, and Janome; for Bernina, an `.inf` file with the same
name goes next to the `.exp` file. The Bernina software help says that each
cut needle is a color block ("Cut Needle 1" and so on). How a machine finds
a cutting needle in its file is not public, and the files of the makers
must not be taken apart (spec, rule 3).

## Decision

1. **A cut line is a running line with `RunningParams::cut`.** Its needle
   points are the cuts (1 mm apart at the start); it has no lock
   stitches, and a **trim** goes between its parts, so the needle lifts in
   each format (VP3 has no jump of its own, and a stitch there would cut a
   line across the fabric).
2. **Specialty > Cutwork** makes cut lines of the selected lines
   (`Design::make_cutwork`, `emb_model::cutwork`): each line is cut into
   pieces by direction (as the screen shows it, in [0°, 180°)), each piece
   goes to the needle whose blade is nearest (45° of directions each), and
   the pieces of one needle are one object with a thread named "Cutwork
   needle N (sign)" in the brand color. The needles are in needle order,
   at the place of the first line, so each needle is a color block and the
   machine stops to change the needle at each color change. The kit comes
   from the brand of the machine of the design (else Bernina), or the
   user sets it.
3. **The files are the normal machine files** (EXP with its INF, PES,
   JEF, VP3 and the others): the needles are color changes with the
   thread names and colors above. Nothing in a file marks a cutting
   needle, because no public description says how.

## Consequences

- A tester with a cutwork machine must check that the machine shows the
  right needle at each color change and that the cuts are clean (the exit
  test). Until then the window says that the files are not tried.
- If a maker publishes how its files mark cutting needles, the writer of
  that format can add it with no change to the design model.
