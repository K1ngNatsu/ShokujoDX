# Plan: Letter shapes (phase P21)

Date: 2026-11-23 · Status: Accepted (owner, 2026-11-23: the six fixes of
the review of the font pipeline, in this order). Milestone 1 is made
(ADR 0147); its faults at the ends of columns go to Milestone 2.
Milestone 2 is made (ADR 0148); its exit test is met in part. Milestone
3 is made (ADR 0149); its exit test is met in part.

## Goal

The letters that the program makes from a font keep the shape of the
font. The owner sees five faults in the letters of the embroidery fonts
and of lettering in TrueType fonts:

- spikes outside the shape of the letter;
- bulges that look out of place;
- a sew order that gives letters a bad shape;
- areas with no stitches (most often at corners);
- faults at the corners of letters.

This phase removes the causes of these faults, not each fault alone. A
measure on the needle points comes first, so each milestone shows what it
changed.

## The cause

The stroke finder makes a column from the middle line of a stroke: lines
square to the middle line meet the outline, and their ends are the rails
(`stroke::rails`). The end of a column is one line square to the stroke.
Then steps after it try to make the ends right: the ends go on to the
edge of the shape (`stroke::extend`), go back out of other strokes
(`trim_start_out_of`, `trim_ends`), follow the outline to the corners of
a free end (`trace_side`), and go on into open corners
(`tune::extend_ends`, for the embroidery fonts only). The outline is never
the edge of a column, and no step looks at the needle points.

- A letter ends at a slant (the feet of "A", the top of "M", the arms of
  "k") or round (a script font), but the column ends square: one corner
  of the end is out of the letter (a spike), the other corner has no
  stitches.
- A line square to the middle line goes on into a junction until its
  limit (1.6 half-widths): the column is wider there (a bulge). Short arms
  (the bar of a "t") are removed as side branches, and the stem covers
  them with a bulge.
- On the inside of a tight curve, the rail goes back: stitches cross.
- The rails are straight lines between points 0.7 half-widths apart: they
  cut the corners of the outline and lie inside its outer curves.
- `trace_side` stops where the outline turns 70° from its first segment,
  so on an end at a slant one side goes on along the end edge, and at a
  junction into the next stroke ("r").
- The checks (the coverage of Auto, the score, the tuning, the route) see
  the area between the rails, not the needle points. A missing corner is
  less than 3 % of a letter, so the second pass of the stroke finder
  counts it as covered.
- The route puts a column that ends in another column on top of it in a
  third of the joints.
- The bundled embroidery fonts were made on 2026-11-19, before the stroke
  finder of P20 Milestone 3. Lettering in TrueType fonts does not get
  `tune::extend_ends`.

## What the measures of 2026-11-23 show

The letter fault bench (Milestone 1, step 1): A to Z, a to z and 0 to 9
of the four embroidery fonts with a source font in the program, at 25 mm,
with the default satin settings. "emb": the bundled embroidery font;
"ttf": lettering in its source font (the stroke finder of today).

| Font | Spikes: at ends / middle | Out with no pull compensation | Rail points out: ends / middle | Crossing pairs | Bulges (mm) | Open area (%) | Corners open | Ends on top / under |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Block Sans emb | 149 / 4 | 239 | 424 / 67 | 5 | 0.6 | 0.96 | 35 of 233 | 33 / 77 |
| Block Sans ttf | 59 / 2 | 98 | 198 / 36 | 10 | 0.4 | 1.47 | 48 of 233 | 24 / 71 |
| Anton emb | 212 / 11 | 348 | 637 / 134 | 232 | 15.3 | 2.71 | 113 of 330 | 41 / 67 |
| Anton ttf | 113 / 10 | 194 | 346 / 96 | 94 | 9.1 | 2.84 | 75 of 330 | 28 / 74 |
| Pacifico emb | 566 / 9 | 748 | 1074 / 121 | 153 | 16.4 | 1.67 | (none) | 58 / 113 |
| Pacifico ttf | 97 / 4 | 148 | 230 / 87 | 66 | 13.7 | 2.01 | (none) | 40 / 81 |
| Varsity emb | 54 / 1 | 89 | 252 / 81 | 77 | 0.2 | 0.76 | 58 of 531 | 146 / 158 |
| Varsity ttf | 17 / 1 | 27 | 121 / 63 | 23 | 0.2 | 0.93 | 59 of 531 | 131 / 148 |

- Spikes: needle points more than 0.4 mm out of the outline (the pull
  compensation is 0.2 mm). 96 % to 99 % are near the end of a column, and
  most are out with no pull compensation too: the shape of the column is
  wrong, not the compensation.
- The letters with most faults: spikes in Pacifico "w", "m", "4" and
  Anton "M"; crossing stitches in Anton "c", "S", "s", "3", "6", "9";
  bulges in Anton "M", "W", "w", "t" and Pacifico "m"; open corners in
  Varsity "X", "x", Anton "t", "G", "w"; stitches across a notch in Block
  Sans "r".
- Travel runs outside the outline: 0.6 mm to 13.6 mm in each font.

## Milestone 1: the outline is the edge of a column (L)

1. **The letter fault bench** (`emb_cli::letter_faults`,
   `emb-cli letter-faults [--proof <file>]`, pictures with
   `tools/letter-faults.py`): the measures of the table above. It is the
   measure of each milestone of this phase.
2. **Rails from the outline.** Each point of the middle line takes the
   nearest point of the outline on each side (its foot), not the point of
   a line square to the middle line. Between two feet on the same ring of
   the outline, the rail follows the outline, with its points and its
   corners.
3. **Openings.** Where a side passes the place where another stroke meets
   or crosses (the foot jumps far along the outline, or it is much
   farther than the half-width of the stroke), the rail goes straight
   across the opening, at the width of the stroke before and after it.
4. **One way.** The feet on a side go along the outline one way only: a
   foot that would go back stays at the foot before (the inside of a
   tight curve), so stitches do not cross.
5. The steps that make columns again (`stroke::trim_column_ends`) keep the
   points of the outline.

The ends of the columns do not change in this milestone (Milestone 2).

**Exit test:** the bench, lettering in the four source fonts (ttf), in
each font: no rail point more than 0.1 mm out of the outline in the
middle of a column; at most a quarter of the crossing pairs of today; at
most 0.5 mm of sides that go back; at most half the bulges of today in
Anton and Pacifico; less open area than today; not more spikes or open
corners than today. The tests and the golden snapshots pass (a change of
a snapshot has a note).

**Made (2026-11-23, ADR 0147).** The rails are the feet of the middle
line on the outline (`stroke::side_rail`: in 55° of the normal, on a
segment in 40° of the stroke, at most 1.6 half-widths away) and follow the
outline between them (`Rails::geometry`, `Rings::along`), also round a
short arm that is not a stroke of its own; openings go straight across;
the feet go one way (a foot stays at most 3.5 times its distance). The
steps of the ends that met the new rails changed a little: `trace_side` is
not used where a side goes on into another stroke, an end that lies in its
own column is cut back, a short column that goes into others only at its
far end stays, and only a thin band counts as a sliver. The stitch engine
places a rung point on a rail that passes it two times near the other
point (Pacifico "P"), and the route cuts a column at one pair (Milestone
5, step 3).

| Bench, ttf (before → after) | Block Sans | Anton | Pacifico | Varsity |
| --- | --- | --- | --- | --- |
| Rail points out in the middle | 36 → 0 | 96 → 1 | 87 → 1 | 63 → 0 |
| Spikes (ends / middle) | 59 / 2 → 33 / 0 | 113 / 10 → 90 / 0 | 97 / 4 → 73 / 0 | 17 / 1 → 15 / 0 |
| Out with no pull compensation | 98 → 55 | 194 → 128 | 148 → 132 | 27 → 16 |
| Open area (%) | 1.47 → 1.34 | 2.84 → 2.49 | 2.01 → 1.54 | 0.93 → 0.89 |
| Corners open | 48 → 44 | 75 → 50 | (none) | 59 → 35 |
| Notches | 24 → 0 | 11 → 2 | 0 → 5 | 0 → 0 |
| Crossing pairs | 10 → 25 | 94 → 106 | 66 → 73 | 23 → 47 |
| Sides that go back (mm) | 0.5 → 0.9 | 7.4 → 6.9 | 5.3 → 5.5 | 2.0 → 3.2 |
| Bulges (mm) | 0.4 → 2.2 | 9.1 → 6.1 | 13.7 → 17.1 | 0.2 → 0.0 |

The middle of each column is on the outline, and curves are covered to
their edge (the open area of Anton "G", "t", "9", "6", "e", "f", "0" and
"C" went down by 70 % to 98 %). The exit test is met for the rail points
(one point is left in Anton and in Pacifico), the open area, the spikes
and the open corners. It is not met for the crossing pairs, the sides
that go back and the bulges: the pictures show them at the ends of
columns and where strokes meet (the tops of "M" and "n", the bar of "e",
the top end of "S", the joints of "V" and "w"), which the steps of before
still make, and the bulges of Pacifico and Block Sans are short arms that
a column covers. These targets go to Milestones 2 and 3. The lettering
bench of P20 keeps its exit tests (its snapshot changed: less outside the
shape and fewer two layers).

## Milestone 2: one way to end a column (L)

Each end of a column is a line, and the rails end at its two points:

1. **A free end** (the end of a pen stroke): the end edge of the outline
   from the corner of one side to the corner of the other. A corner is
   where the outline leaves the direction of the stroke (not the
   direction of its first segment). The stitches turn to the angle of the
   end over about one width of the column. A round end gets a short last
   part whose rails close along the outline.
2. **A corner** (a cut of a stroke that turns): a mitre line from the
   inner corner to the outer corner of the outline. The two columns meet
   on this line (as `emb_model::mitre` joins columns).
3. **A junction or a crossing**: the edge of the other column, with a
   small overlap into it.

These replace `stroke::extend`, the cut of the start of a piece at a
corner, `trace_side`, the cut back of ends in `trim_ends`, and
`tune::extend_ends`. Short pieces and the pieces of a corner get the same
ends as long pieces.

**Exit test:** the bench, ttf, in each font: at most a tenth of the
spikes and of the needle points out with no pull compensation of today;
at most 5 % of the convex corners open; no stitch across a notch; open
area at corners at most a quarter of today. And the targets of
Milestone 1 that the ends make: at most a quarter of the crossing pairs
of today, at most 0.5 mm of sides that go back, at most half the bulges
of today in Anton and Pacifico.

**Made (2026-11-23, ADR 0148).** A free end follows the outline round it
from a pair back from the end (`stroke::free_end`): the pairs are the
points of its two sides at the same places along the stroke, so the
stitches stay across the stroke and end on the end edge, on the slant of a
slanted end, or round a round end. A corner of a stroke ends both pieces
on its mitre (`stroke::mitre_at`, `close_mitre`) where the mitre is clear
(a turn of at most 110°, at most 1.6 widths of the thinner piece, across
both pieces, in the letter); other corners and the junctions are made as
before. The tails of the route must reach the end of the place of the
letter (`route::TOUCH` 0), so the feet of an "A" are not a tail.
`trace_side` stays for ends with no outline round them, and
`tune::extend_ends` stays until the fonts are made again (Milestone 6).

| Bench, ttf (before P21 → after M1 → after M2) | Block Sans | Anton | Pacifico | Varsity |
| --- | --- | --- | --- | --- |
| Spikes | 61 → 33 → 20 | 123 → 90 → 50 | 101 → 73 → 33 | 18 → 15 → 15 |
| Out with no pull compensation | 98 → 55 → 33 | 194 → 128 → 68 | 148 → 132 → 57 | 27 → 16 → 16 |
| Crossing pairs | 10 → 25 → 20 | 94 → 106 → 48 | 66 → 73 → 41 | 23 → 47 → 14 |
| Sides that go back (mm) | 0.5 → 0.9 → 2.0 | 7.4 → 6.9 → 2.6 | 5.3 → 5.5 → 12.7 | 2.0 → 3.2 → 0.7 |
| Bulges (mm) | 0.4 → 2.2 → 2.3 | 9.1 → 6.1 → 4.3 | 13.7 → 17.1 → 17.8 | 0.2 → 0.0 → 0.0 |
| Notches | 24 → 0 → 0 | 11 → 2 → 2 | 0 → 5 → 5 | 0 → 0 → 0 |
| Open area (%) | 1.47 → 1.34 → 0.98 | 2.84 → 2.49 → 1.84 | 2.01 → 1.54 → 0.92 | 0.93 → 0.89 → 0.80 |
| Open area at corners (mm²) | 19.4 → 18.6 → 13.0 | 36.5 → 26.0 → 21.4 | (none) | 25.6 → 18.3 → 15.3 |
| Corners open | 48 → 44 → 46 | 75 → 50 → 56 | (none) | 59 → 35 → 40 |

The exit test is met for Anton's bulges (half of before P21) and nearly
for the notches (2 and 5 are left). It is not met for the rest: the
spikes are a third to a fifth of before P21 (the target is a tenth), the
open corners are 8 % to 17 % (the target is 5 %), and the crossing pairs,
the sides that go back and the bulges of Pacifico are not down enough.
The pictures show what is left: the tips of very sharp ends, the joints
where pieces meet at a sharp point (the V of "M", "W", "w", the crossing
of "x"), which have no mitre and keep the ends of before, the short arms
that a column covers (Milestone 3), and the round tips of Pacifico (the
column turns at the tip). These go to Milestones 3 and 5; the targets stay
the exit test of the phase.

## Milestone 3: the middle line (M)

The faults of the middle line that Milestones 1 and 2 do not remove:

1. A square end of a stroke: the middle line forks into its two corners;
   the stroke finder keeps one branch, so the line bends to one corner.
   Both branches go, and the line ends at the fork, in the middle.
2. A short arm (the bar of "t", the arms of "E" and "F", serifs) is a
   stroke of its own if it is longer than its distance into the junction,
   not a side branch.
3. At an L corner, the middle line bends into the outer corner. The bend
   is cut, so each piece is straight up to the corner.
4. The ends of smoothing: the first and the last points are smoothed too
   (with the points of the line only).
5. The short arms that a column covers today (Milestone 1, `STUB_ARC`
   and `STUB_NEAR`: the rail goes round an arm that the graph took as a
   side branch) are strokes, and the rails no longer go round them.

**Exit test:** the bench, ttf: no open corner in Anton "t", Block Sans
"E", "F", "k" and Varsity "X", "x"; at most 1 mm of bulges and at most
0.5 % open area in each font.

**Made (2026-11-23, ADR 0149).** Side branches are measured with the
half-width of the strokes that they hang on (`Graph::side_radius`), not of
their junction, and go together; so the short arms of a crossing (the bar
of "t") are strokes. An end at a junction holds its width for 0.5
half-widths (1.5 before), and its rails follow the outline beyond that.
The rails go round a short arm that is still a side branch only up to 2.0
half-widths. Steps 1, 3 and 4 needed no change: the thinning of these
fonts leaves no fork at a square end, and the free ends and mitres of
Milestone 2 do not use the bent end of the middle line.

| Bench, ttf (after M2 → after M3) | Block Sans | Anton | Pacifico | Varsity |
| --- | --- | --- | --- | --- |
| Spikes | 20 → 17 | 50 → 44 | 33 → 37 | 15 → 15 |
| Rail points out (ends) | 100 → 38 | 105 → 84 | 111 → 76 | 53 → 30 |
| Crossing pairs | 20 → 27 | 48 → 40 | 41 → 40 | 14 → 11 |
| Bulges (mm) | 2.3 → 0.0 | 4.3 → 4.3 | 17.8 → 13.5 | 0.0 → 0.0 |
| Open area (%) | 0.98 → 0.57 | 1.84 → 1.66 | 0.92 → 0.56 | 0.80 → 0.47 |
| Corners open | 46 → 43 | 56 → 54 | (none) | 40 → 33 |

The exit test is met for Anton "t" and Block Sans "E", "F", "k" (no open
corner), for the bulges of Block Sans and Varsity, and for the open area
of Varsity. It is not met for Varsity "X" (3 open corners) and "x" (2:
where the diagonals widen into the serifs), for the bulges of Anton
(4.3 mm) and Pacifico (13.5 mm), and for the open area of Block Sans
(0.57 %), Anton (1.66 %) and Pacifico (0.56 %). The lettering bench of
P20 has more characters with more than 3 % of two layers (8 to 18; the
mean stays 1.2 %).

## Milestone 4: the needle points decide (M)

1. `emb-letter` uses `emb-stitch` (a new dependency between two engine
   crates; an ADR and the dependency direction in the spec). The quality
   score of a letter adds needle points out of the outline, crossing
   stitches and open corners (`quality::LetterQuality`).
2. The choices of the converter use them: columns or fill for each letter
   (`EmbFont::choose_fills`), the ends, the coverage of Auto
   (`glyph::AUTO_MIN_COVERAGE`), and the second pass of the stroke finder
   (a missing corner is not a sliver).
3. The bench becomes a snapshot test (`tests/letter_faults.rs`), so a
   change that makes the letters worse shows in CI.

**Exit test:** the snapshot runs in CI; a letter with spikes gets a lower
score than the same letter with none; no font of the program goes below
the score limit.

## Milestone 5: the sew order at joints (M)

1. **Layers.** A column that ends in another column sews before it, so
   the other column covers its end (a cost in the route between the
   trims and the travel).
2. **Travel at each size.** The route is made at 25 mm; the gaps that need
   no run (`NO_RUN`) and the distance of a run from the edge (`EDGE`) are
   checked at the largest size of the font, in mm.
3. **Seams.** A column that the route cuts at a junction is cut at one
   pair across it, not at the nearest point of each rail. (Done in
   Milestone 1: `quality::cut_column`, ADR 0147.)

**Exit test:** the bench: at most 5 % of the joints have the column that
ends on top, with no more trims than today; no travel run out of the
outline.

## Milestone 6: one way for all letters, and new fonts (M)

1. Lettering in TrueType fonts gets the same steps as the converter.
2. The bundled embroidery fonts and the font packages are made again
   (`tools/make-embroidery-fonts.py`), audited, and the proof pictures of
   the ten tuned fonts are looked at. (This is also the open work of P20:
   the fonts with the stroke finder of P20 Milestone 3.)
3. A sew-out kit: the test sheets of the ten tuned fonts at three sizes,
   and the letters with the most faults of the table above.

**Exit test:** the bench: the embroidery fonts are as good as lettering
in their source fonts or better; each font has a score of
`quality::LIMIT` or more; the owner sews the kit and sees no spikes,
bulges or open corners.

## Exit test of the phase

The exit tests of the six milestones, and the sew-out of Milestone 6.

## Order

The owner's order: Milestones 1 to 6. The bench (Milestone 1, step 1)
comes first, as the measure of each milestone. The bundled fonts are made
again only in Milestone 6, so the sew-outs of the older milestones use
lettering in the TrueType fonts.

## Not in this phase

- New fonts, and letter corrections by hand.
- The fill letters, and the stitch engine of fills.
- Japanese characters as a target: the lettering bench of P20 shows the
  changes there, and they are checked with it.
