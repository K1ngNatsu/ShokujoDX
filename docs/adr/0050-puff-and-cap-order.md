# ADR 0050: Puff (3D foam) and cap sequencing

Date: 2026-10-25 · Status: Accepted (P11 Milestone 7)

## Context

P11 Milestone 7 asks for puff: settings of a satin column or lettering
for sewing over foam (dense satin, no underlay rows across the column,
closed ends, a stop before the object to put the foam on, a note on the
worksheet), and for cap sequencing: a sew order for caps, from the center
out and from the bottom up, as one command and a setting of the cap hoop.
The methods are common digitizing practice, in our own words (commercial
programs have such features; their code is closed, spec rule 3).

## Decision

1. **Puff is a setting of the satin settings** (`SatinParams::puff`), so
   it is on satin columns, lines with a column border, and lettering (its
   satin settings). Edit > Stitch types > Puff turns it on for the
   selection (or off, if all have it) in one undo step: satin type,
   spacing `PUFF_SPACING_MM` (0.25 mm). Object Properties has a box.
2. **The stitch engine** (`satin::satin_with`): with puff, the underlay
   keeps only the center and edge walks (rows across the column would
   press the foam flat); stitches along the column over each end, 0.3 mm
   apart across it, from 1.2 mm in to 0.4 mm past the end, cut the foam
   there and close the end (`puff_cap`). A ring (a closed column) has no
   ends.
3. **The stop is a plan decision** (`plan::build_plan`): a stop with the
   reason `PlaceFoam` before the first of the puff objects that sew one
   after the other; the next puff objects need none. Its text is in the
   stop notes of the worksheet. Right after a color change the machine
   stops anyway: the stop is that color change (no second stop record),
   with the reason, so the note says what to do there (`stop_here`).
4. **Cap order** (`order::cap_order`): in each color block, the objects
   from the center of the hoop out, in bands of 10 mm (`CAP_BAND_MM`),
   and in a band from the bottom up. As Shortest order, the colors stay
   and an object never moves before an object that it overlaps. Arrange >
   Sew order > Cap order also sets lettering to sew from the middle
   letter out (`LetterOrder::CenterOut`). One undo step.
5. **The cap hoop setting:** `HoopPreset::cap` (`cap = true` in
   `data/hoops.toml`, the wide cap frame). With a cap frame as the hoop,
   Design info warns when the order is not the cap order.

## Consequences

- Puff columns are best 3 mm wide or more; narrow columns cut through
  the foam.
- The cap order moves objects only inside a color block; a design with
  many colors on a cap can still need colors in a better order.
- The exit tests are open: puff letters sewn on flat fabric (Topaz 20,
  clean ends), and cap sequencing on a cap frame (waits for a tester with
  one).
