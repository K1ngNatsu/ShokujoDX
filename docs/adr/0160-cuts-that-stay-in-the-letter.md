# ADR 0160: Cuts that stay in the letter

Date: 2026-11-24 · Status: Accepted (P21 Milestone 14,
`docs/design/p21-plan.md`).

## Context

After Milestone 13 the owner asked to go on with the letter shapes. The
next items of the open work were the needle points in counters that are
narrower than a thread in very heavy fonts (Shrikhand "p", Poster Heavy
"n", Giant Round "m": 121 of the 1,114 needle points out of the outline in
the survey of 127 fonts) and the spikes at sharp tips (Funky Slab "N", "V"
and "W", Suez One "X" and "x", Racer Sans "W": 14 to 16 each).

ADR 0158 gave pull compensation as the cause of the first item. The
pictures and the rails showed another cause, the same for both items: the
split of the outline (ADR 0152).

1. The cut from a notch follows the line that halves it. In a heavy "n",
   the line from the notch on top of the stem goes down the whole stem,
   beside the narrow counter, to the baseline (970 font units, 2.2 times
   the typical width): a cut along a stroke, not across the place where
   two strokes meet.
2. The end of each piece at a cut goes into the piece on the other side by
   a tenth of the length of the cut, square to the cut, so the joint has
   no gap (`BRIDGE_OVERLAP`). Where a cut meets the outline at a slant,
   the moved end of the piece is out of the letter, and the rail of the
   column of the piece follows it: the stem of the heavy "n" went 97 font
   units (2 mm) into its counter, and the top of the diagonal of Funky
   Slab "N" went 1.6 mm out of the stem.

## Decision

1. **A cut along a stroke is optional.** A cut from a notch goes across
   the place where strokes meet: some point of it (at a quarter, the
   middle or three quarters) is at least `CUT_CLEAR` (0.15) of its length
   from the outline. Measured on 127 cuts in 80 letters of nine fonts:
   the cuts across strokes are 0.23 or more, the cuts along strokes 0.14
   or less. A cut along a stroke is an optional link (`stroke::Optional`):
   the glyph keeps it only if its columns sew no worse (ADR 0159). It was
   left out at first: Round Sans "d" lost 13.8 mm2 of cover. The cuts of
   elbows have no such check: the cut of an elbow along the hairline serif
   of Fat Face Serif makes the serif a piece of its own, and the cuts of
   elbows are optional already.
2. **The pieces stay in the letter.** An end of a piece at a cut that
   would go out of the letter stays where it is (`split::moved_ends`).
   Two other ways were measured: an end taken back along the moved cut to
   the outline (pieces that were not right where the outline is far:
   72 % fewer needle points out of the outline, but 2.6 % more area with
   two layers), and an end taken back only where the outline is near
   (between the two). The end that stays has the fewest needle points out
   of the outline and the least area with two layers; it has 12 more
   trims than the second way, and 5 fewer than Milestone 13.
3. Pull compensation for each side of a part near a narrow counter (ADR
   0158) is not needed for these letters.
4. **No travel stitch shorter than the shortest stitch of the machine**
   (`plan::sew_run`). With the new pieces, the travel run of the "A" of
   Lato Bold ended 0.23 mm from the first needle point of the column after
   it, and the plan made that stitch (it made each travel stitch of 0.1 mm
   or more, `SAME_PLACE_MM`); the program shows a stitch shorter than 0.3
   mm as a fault (it breaks the thread). A travel stitch shorter than the
   shortest stitch of the machine is not made now: the run starts where
   the needle is. In the golden designs this changes only stitches (Tartan
   Fill one fewer, Shaped Text 30 fewer).
5. The Mitre Corners setting (ADR 0121) joined one corner in the letters
   and digits of Pacifico (its "f"); with the new pieces it joins none. The
   test of the setting asks for joins only in the three fonts with
   corners.

## Consequences

- The survey (7,874 letters of 127 fonts as lettering at 25 mm), Milestone
  13 to now: needle points out of the outline 1,114 to 220 (80 % fewer;
  the cut along a stroke alone: 955); needle points out with no pull
  compensation 2,414 to 368; area out of the outline 990 to 596 mm2; area
  with two layers 50,609 to 44,685 mm2; open area 3,158 to 3,040 mm2; open
  corners 1,882 to 1,798; crossing pairs 750 to 657; fans 449 to 393 mm;
  columns on top 1,175 to 1,130; trims 780 to 775. Bulges 6,874 to 7,048
  mm (+2.5 %), sides that go back 1,036 to 1,048 mm.
- The letter fault bench (embroidery font / lettering): needle points out
  of the outline Block Sans 2 / 2 to 1 / 1, Anton 2 / 3 to 1 / 1,
  Pacifico 3 / 3 to 0 / 0; with no pull compensation Block Sans 14 to 1,
  Anton 11 to 3 / 4, Pacifico 7 / 9 to 1 / 0; rail points out Pacifico 32
  / 31 to 8 / 8, Varsity 18 / 19 to 7 / 7; fans 0.5 mm more in Block Sans,
  less in the others; columns on top Varsity 52 to 56; trims the same.
- The fonts: 33,937 to 33,935 trims in the letters (Friendly Serif 10
  fewer; Fat Face Serif 12, Varsity 7 and Suez One 6 more, in letters that
  are not A-Z, a-z or 0-9 for Varsity); the column fonts of the program
  94.5 to 94.4 on mean. Fat Face Serif 79 to 91, Zilla Slab (a package)
  89 to 84 (more columns too narrow at its smallest size and too long at
  its largest).
- The golden designs with lettering: stitches 0.1 % to 7.4 % fewer (less
  area with two layers), Lettering Bridge 1.1 % more; the same trims but
  Embroidery Fonts 3 (2 more), Embroidery Fonts 7 and Lettering Circle
  (one more each), from the letters, not from decision 4.
- The tests of the parts of P18 (the corpus as DST and back): the outlines
  92.6 % to 92.5 %; the measured columns with the width and the spacing
  of their source 93.5 % to 94.3 %; the runs with the stitch length and
  the bean setting of their source 77.6 % of 125 to 76.3 % of 135.
- Shrikhand "p": 45 needle points out of the outline to 0, open area 23.1
  to 0.1 mm2; Poster Heavy "n" 42 to 2; Giant Round "m" 34 to 0, open area
  17.7 to 0.5 mm2; Funky Slab "N", "V" and "W", Suez One "X" and "x",
  Racer Sans "W": 14 to 16 each to 0 or 1.
- Worse: Geo Slab "M" 15.6 mm2 more open area (the serifs at the top of
  its stems: the cut beside each stem meets the top edge at a small slant,
  its moved end goes out by a few font units and stays, so the edge of the
  piece turns); Fat Face Serif "n" 5.5 mm2; Slab Round "W", Black Ops One
  "L", Slab Heavy "X" and Geo Slab "x" about 3.3 mm2 each.
