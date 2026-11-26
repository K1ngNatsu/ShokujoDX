# ADR 0181: The size of a letter in the design, and the cap of a wide column end

Date: 2026-11-26 · Status: Accepted (the owner: "finish the issues with
the monogram application that are still open", after ADR 0180). Changes
how an embroidery font chooses columns or fills (ADR 0120) and the rule of
ADR 0180 where stitches run along a side of a column.

## Context

After ADR 0180 the monogram probe (64 cases) had build-up in 23 cases,
all at 60 mm in the styles Classic, Diamond and Oval, most on towel. A
probe of each place gave two kinds:

- **The ends of wide columns** (the foot of "A", the top corners of "W",
  the leg of "K"): about 4 layers of top stitches and 3 of underlay. The
  stitches meet the cap of the end (a side of the column, after the
  tuning took the end to a corner) at about 16°: the pairs are half a step
  apart across the stitches, so the top stitches are two times as dense
  there, and the rule of ADR 0180 (pairs less than half a step apart) did
  not start.
- **The joints of two columns** (the arms of "K", "M", "R"): two satins on
  each other, 5.6 to 6.5 layers.

The columns were wide because the letters were larger than the columns of
the font go. Block Sans sews columns up to 58 mm and fill letters above
(ADR 0120). The Block style at 60 mm sews fills. The Classic style lays
the text out at 65 % of the height and makes the middle initial larger
with its edit; Oval and Diamond lay out smaller text and the envelope
makes its middle taller (ADR 0164). The font chose by the height of the
layout, so a middle initial of 60 mm sewed as columns.

## Decision

1. **The size of a letter in the design chooses columns or fills.** The
   layout gives the source the scale of the letter times the scale of its
   edit and the most that the envelope makes the text taller
   (`Envelope::max_stretch`: 1 + amount for the oval and the diamond).
   The scale chooses only (`LetterSource::parts`, `fixed`, `tuning`); the
   parts are scaled as before. The stretch of the envelope is for the
   whole text, so the letters of an envelope are the same kind.
2. **Stitches along a side of a column** (`satin::thin_across`): where a
   stitch is 25° or less from a side, the pairs that stay can be 1.5
   steps apart across the stitches (`ALONG_SIDE_GAP`), so pairs half a
   step apart there stay one step apart, as in the middle of the column.
   A column whose stitches meet both sides at more than 25° (also one
   slanted all along) keeps its pairs: a factor of 1.5 for all pairs would
   sew such a column lighter than its spacing.
3. **Not done.** The underlay that stops short of the ends of a column
   (tried with 0.6 to 1.5 mm) took the worst place only from 8.4 to 7.9.
   Travel in a tatami fill that goes round the sewn rows (as in circular
   and guided fills) made the travel longer in 11 corpus designs and the
   travel on the fabric of `embroidery-fonts-8` longer, with no travel on
   stitches less; with a new needle path where there is no way round, it
   added trims. Neither is used.

## Consequences

- The monogram probe: no build-up in 63 of 64 cases (23 cases before,
  38 before ADR 0180). The one place left is the joint of the arm and the
  stem of "K" (JMK, Classic at 60 mm, towel: 6.1 layers on 4 mm², the
  smallest place that the check shows): two satin columns of a 39 mm
  letter on each other, as at each joint of satin letters.
- New: in the Oval style at 60 mm on cotton (JMK, also with a circle),
  the fill letter of "M" has a travel of 4 mm on its sewn rows (Travel
  that shows): the travel goes from one side of the leg to the other at
  the end of a section, and no way round is open.
- The letter fault bench and the judged letters: the same to the eye.
- Tests: `stitches_along_a_side_are_one_step_apart`,
  `the_most_stretch_of_an_envelope`, the letters by size in
  `a_font_has_fill_letters_for_large_lettering` (an edit and an envelope),
  `the_large_styles_build_up_no_thread` (Classic, Diamond and Oval at
  60 mm, "ABC" and "SRW", on cotton and towel).
