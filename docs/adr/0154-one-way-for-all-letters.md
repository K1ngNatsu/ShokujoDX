# ADR 0154: One way for all letters, and the fonts made again

Date: 2026-11-23 · Status: Accepted (P21 Milestone 6,
`docs/design/p21-plan.md`).

## Context

The embroidery fonts of the program were made on 2026-11-19, with the
stroke finder of before P21. Lettering in TrueType fonts and the
converter of the embroidery fonts did not have the same steps: the
converter tuned the ends of the columns (`--extend`), lettering did not.
The tuning kept a longer end if more of the outline was covered, with no
look at the needle points.

## Decision

1. Lettering in TrueType fonts gets the steps of the converter
   (`tune::round_parts`, `tune::extend_ends` in `Font::route`), not for
   kana and kanji; the converter does not extend kana and kanji either
   (ADR 0146), and the Japanese fonts get `--extend` for their Latin
   letters.
2. The tuning keeps a change only if the changed columns have no more
   needle points more than `quality::SPIKE_MM` out of the outline
   (`quality::spike_points`). A new piece of a rail stops where it meets
   the outline. A small round part with two or more columns gets one
   column along its long axis.
3. The converter routes with the outline of each letter of the source
   font (`EmbFont::route_from`, `route_until` with a source), and rounds the
   coordinates of the file when the font is done (`EmbFont::round_letters`,
   before the score), not before the tuning.
4. All bundled fonts and font packages are made again with the stroke
   finder of P21 (the split of ADR 0152, the route of ADR 0153).
5. The sew-out kit of the phase is `tools/make-sewout-kit-p21.py`.

## Consequences

- The scores with the score of today: the mean of the 55 fonts of the
  program from 78.9 to 85.5; 46 higher, 8 lower by 1
  or 2. No font is below the limit.
- The bench: the embroidery fonts have the numbers of lettering in their
  source fonts (spikes in Block Sans, Anton, Pacifico and Varsity: 2, 25, 27 and 0;
  before P21: 153, 223, 575 and 55).
- 19 fonts of the packages have `quality::BUNDLE` or more: the
  owner decides if they come into the program (ADR 0116).
- The sew-out of the kit is open; the engine view is the measure until
  then (the owner finds it true to the sew-outs).
