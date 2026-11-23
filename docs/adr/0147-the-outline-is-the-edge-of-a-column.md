# ADR 0147: The outline is the edge of a column

Date: 2026-11-23 · Status: Accepted (P21 Milestone 1,
`docs/design/p21-plan.md`). Replaces decision 3 of ADR 0143.

## Context

The stroke finder made the rails of a column with lines square to the
middle line of a stroke: the points where these lines met the outline,
0.7 half-widths apart. The review of the font pipeline (2026-11-23) and
the letter fault bench (`emb_cli::letter_faults`) showed what this gives:

- Straight rails between points that are far apart cut the corners of the
  outline and lie inside its outer curves; across a concave part of the
  outline they go out of it (36 to 96 rail points more than 0.1 mm out of
  the outline in the middle of the columns of each font, at 25 mm).
- A line square to the middle line goes on into a junction to its limit:
  the column is wider there (bulges).
- On the inside of a tight curve, the lines square to the middle line
  cross: the rail goes back and the stitches cross (Anton "3", "s").

## Decision

1. **Feet.** Each point of the middle line takes the nearest point of the
   outline on each side (`stroke::side_rail`, `Rings::nearest`): in 55°
   of the normal (`FOOT_CONE_DEG`), on a segment of the outline in 40° of
   the direction of the stroke (`FOOT_ALONG_DEG`; where another stroke
   meets this one, its outline goes across), at most 1.6 half-widths away
   (`FOOT_REACH`, the reach of the lines before).
2. **The rail follows the outline** (`Rails::geometry` with the rings,
   `Rings::along`): between two feet on one ring, the rail takes the
   points of the outline between them, if the outline there is at most
   two times as long as the line between the feet and stays near the
   column (`FOLLOW_ARC`, `FOLLOW_NEAR`); points of an almost straight
   outline go (`FOLLOW_THIN`, Douglas and Peucker). It also follows the
   outline round a short arm that the graph of the middle lines took as a
   side branch (`STUB_ARC`, `STUB_NEAR`: the top of the stroke of ぁ above
   its bar), so that the arm has stitches, as before; Milestone 3 makes
   such arms strokes. `trim_column_ends` takes the rings too, so a column
   that changes follows the outline again.
3. **Openings.** Where a side has no foot, or where the outline between
   two feet goes away into another stroke (a jump), the rail goes
   straight across, from the foot before to the foot after; a foot that
   stays at the corner of a jump is in the opening too. The `INTO_OTHER`
   rule of ADR 0143 (a side that ends near the middle line of another
   piece) goes: openings do its work.
4. **One way.** The feet of a side go along the outline one way. A foot
   that would go back stays at the foot before it (the inside of a tight
   curve), for at most `STAY_MAX` (3.5) times its distance from the
   middle line; a longer stay is an opening.
5. The steps after the rails that met the new rails:
   - `trace_side` (the corners of a free end) is not used if a side goes
     on along the stroke to its limit with no corner, or farther than
     `END_NEAR_TIP` (1.5) widths from the end of the middle line: the
     side goes into another stroke (the stems of "n" and "r" and the bowl
     of a script "P" had fans of stitches across a notch).
   - `trim_ends` also cuts back an end that lies in its own column, more
     than `SELF_GAP` (3) widths along it (a stroke that goes round and
     ends on itself), and a short column that goes into the others only at
     its far end stays (the short strokes of 草 that a crossing cut: they
     were taken as lying all in the others).
   - The second pass of the stroke finder counts an open part as a sliver
     (covered with no stitches) only if it is a thin band
     (`SLIVER_THICK`, a mean thickness of at most 0.16 of the widest
     stroke): the tail of the comma of Lato became thin enough to count as
     covered.
6. **Rungs on a rail that passes a point two times.** The rails of such a
   stroke go over the same points of the outline two times. The place of
   a rung point on a rail is the place nearest to the part of the column
   of the other point (`satin::Rail::rung` in the stitch engine,
   `quality::Rail::rung`); before, the first place was taken, and the
   stitches of Pacifico "P" went across its bowl.
7. **The route cuts a column at one pair** (`quality::cut_column`, P21
   Milestone 5 step 3, done now because the new rails showed it): both
   rails are cut at the pair of stitches nearest the junction, not each
   rail at its own nearest point.

## Consequences

The bench, lettering in the four source fonts at 25 mm (before → after):

| Font | Spikes (ends / middle) | Out with no pull compensation | Rail points out (ends / middle) | Open area (%) | Corners open | Notches | Crossing pairs | Bulges (mm) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Block Sans | 59 / 2 → 33 / 0 | 98 → 55 | 198 / 36 → 133 / 0 | 1.47 → 1.34 | 48 → 44 of 233 | 24 → 0 | 10 → 25 | 0.4 → 2.2 |
| Anton | 113 / 10 → 90 / 0 | 194 → 128 | 346 / 96 → 184 / 1 | 2.84 → 2.49 | 75 → 50 of 330 | 11 → 2 | 94 → 106 | 9.1 → 6.1 |
| Pacifico | 97 / 4 → 73 / 0 | 148 → 132 | 230 / 87 → 198 / 1 | 2.01 → 1.54 | (none) | 0 → 5 | 66 → 73 | 13.7 → 17.1 |
| Varsity | 17 / 1 → 15 / 0 | 27 → 16 | 121 / 63 → 58 / 0 | 0.93 → 0.89 | 59 → 35 of 531 | 0 → 0 | 23 → 47 | 0.2 → 0.0 |

- The middle of the columns is on the outline: no spike in the middle of
  a column, and the curves are covered to their edge (the open area of
  Anton "G", "t", "9", "6", "e", "f", "0", "C" went down by 70 % to
  98 %).
- The faults that are left are at the ends of columns and where strokes
  meet: the ends are still made by the steps of before (extension, trace,
  cut back), and they show more now that the middle is right (the tops of
  "M" and "n", the bar of "e", the top end of "S", the joints of "V" and
  "w"). The crossing pairs of Block Sans and Varsity are there. The
  bulges of Pacifico and Block Sans are the short arms that a column
  covers (decision 2). Milestones 2 and 3 make the ends and the arms.
- The lettering bench of P20 (the snapshot changed): at each size the
  highest part outside the shape went from 1.3 % to 0.9 %, the mean part
  with two layers from 1.4 % to 1.2 %, and the mean coverage up; more thin
  strokes are runs at 10 mm and 15 mm (the columns no longer bulge at
  junctions, so their middle widths are smaller).
- The rails have more points (the points of the curves of the outline);
  a converted font file is larger.
- Lettering in TrueType fonts changes; the bundled embroidery fonts do not
  change until they are made again (Milestone 6), but for the place of a
  rung on a rail that passes it two times (decision 6).
