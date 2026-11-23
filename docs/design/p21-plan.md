# Plan: Letter shapes (phase P21)

Date: 2026-11-23 · Status: Accepted (owner, 2026-11-23: the six fixes of
the review of the font pipeline, in this order). Milestone 1 is made
(ADR 0147); its faults at the ends of columns go to Milestone 2.
Milestone 2 is made (ADR 0148); its exit test is met in part. Milestone
3 is made (ADR 0149); its exit test is met in part. Milestone 4 is made
(ADR 0150). Milestone 7 (a middle line from a Voronoi diagram, a trial)
was added by the owner on 2026-11-23; it is made, and the grid stays (ADR
0151). Milestone 8 (strokes from a split of the outline at its concave
corners, a trial) was added by the owner on 2026-11-23; the trial is
made, and the owner chose the split for Latin letters and for kana and
kanji (2026-11-23, ADR 0152). Milestones 5 and 6 are updated for it.
Milestone 5 is made (ADR 0153); its exit test is met in part. Milestone 6
is made (ADR 0154): all phase work is done in the software; the sew-out
of the kit (`tools/make-sewout-kit-p21.py`) is open.

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

**Made (2026-11-23, ADR 0150).** `emb-letter` uses `emb-stitch`. The
score of a letter has `spikes` (needle points more than 0.2 mm out of the
outline with no pull compensation) and `crossing` (pairs that cross), from
`quality::needle_faults`; so the font score and the choice of columns or
fill see them. The fault bench is the snapshot test `tests/letter_faults.rs`
(25 letters of the four fonts as lettering in their source fonts, 28 s in
a debug build). The coverage of Auto and the second pass of the stroke
finder did not change (the thin bands of ADR 0147 already keep a missing
part out of the slivers).

The exit test is met: the snapshot runs in CI, a test shows that a column
out of its outline has spikes and that spikes take from the score, and
the four bundled fonts with a source font in the program, measured again,
are at the limit or more (Block Sans 84, Anton 72, Pacifico 68, Varsity
96; before 88, 82, 84, 98: they were made with the stroke finder of
before P21). The other bundled fonts are measured again in Milestone 6.

## Milestone 5: the sew order at joints (M)

Updated on 2026-11-23 for the split of Milestone 8: the split knows the
joints of a letter before the route.

1. **Layers.** A column that ends in another column sews before it, so
   the other column covers its end (a cost in the route between the
   trims and the travel). The split gives the joints: at a bridge, the
   piece whose end is on the bridge sews before the piece that goes
   through; at a cut from a notch, the two pieces overlap each other, and
   either one can sew first. The columns keep their joints (in `Strokes`
   and in the parts of a letter), so the route does not find them again
   from the shapes.
2. **The trims of the split.** The split made more parts at the
   junctions of kanji: the eight words of the lettering bench of P20 have
   1, 1 and 2 more trims at 10, 15 and 25 mm (龍 and 東京). The pieces at a
   bridge overlap, so a travel run can cross the joint under the piece
   that sews later: the route uses the joints of step 1 as its paths.
3. **Travel at each size.** The route is made at 25 mm; the gaps that need
   no run (`NO_RUN`) and the distance of a run from the edge (`EDGE`) are
   checked at the largest size of the font, in mm.
4. **Seams.** A column that the route cuts at a junction is cut at one
   pair across it, not at the nearest point of each rail. (Done in
   Milestone 1: `quality::cut_column`, ADR 0147.)

**Exit test:** the bench: at most 5 % of the joints have the column that
ends on top, with no more trims than with the split today; no travel run
out of the outline; the eight words of the lettering bench of P20 have
no more trims than before the split (48, 49, 54, 56 and 56 at 10, 15,
25, 50 and 80 mm).

**Made (2026-11-23, ADR 0153).**

1. The route finds the joints from the shapes of the parts: the middle
   of an end of a column in another column (`route::joints`), not where
   each of two columns ends in the other (the two legs at a cut from a
   notch). So the same rule works for letters with no split and for the
   embroidery fonts of before. Its cost is the trims, then the columns
   that end on top of a column sewn before them (`Routed::on_top`), then
   the travel. `route_with_splits` keeps a cut that gives as many trims
   and fewer columns on top, and also tries a cut in the middle of a
   column whose two ends lie in other columns (a stem between two
   serifs).
2. The trims of the split are back: the order that sees the joints keeps
   other runs, and the eight words have 48, 49, 54, 54 and 54 trims (49,
   50, 56, 56 and 56 with the split, 48, 49, 54, 56 and 56 before it).
3. Travel runs stay in the outline of the letter when the route knows it
   (`route_in`, `route_with_splits_in`: lettering in TrueType fonts and
   the bench give it): a run goes only through cells in the outline, from
   the points of the edges in it. The run across the opening of Anton "e"
   (under the fan of a column) is gone. The constants `NO_RUN` and `EDGE`
   did not change: at 80 mm a gap with no run is about 1 mm (one stitch),
   and a run is farther from the edges at larger sizes. The 11.7 mm
   stitch in the Arabic word of the corpus design `shaped-text` came from
   the direction of the route: the letters of a script from right to
   left are routed as their mirror images now (`route_with_splits_dir`,
   `route::is_rtl`), so each letter ends on its left, where the next one
   starts (the longest stitch of the design: 6.2 mm).
4. The fault bench counts trims (gaps between the parts of more than
   0.012 of the capital height) and does not count a joint where each
   column ends in the other.

The bench (lettering in the source fonts at 25 mm), before and after:

| Font | Columns on top of a column sewn before them | Trims | Travel runs out of the outline |
| --- | --- | --- | --- |
| Block Sans | 16 to 5 of 82 (6 %) | 2 to 2 | 2.3 mm to 0.2 mm |
| Anton | 29 to 11 of 70 (16 %) | 3 to 2 | 1.7 mm to 0.1 mm |
| Pacifico | 19 to 10 of 104 (10 %) | 2 to 2 | 0.7 mm to 0.2 mm |
| Varsity | 85 to 51 of 196 (26 %) | 19 to 20 | 0.2 mm to 0.0 mm |

The exit test is met in part. The trims of the words are back, and the
runs out of the outline are 0.2 mm or less in each font (short ends at
the corners of columns). The columns on top are not at 5 %: most of the
ones that stay are where the needle goes from one column into the next
(half of a serif and then the stem on it); another order needs a trim
there. The other orders that the trial measured:

| Order | Block Sans on top / trims | Anton | Pacifico | Varsity | Trims of the words at 25 mm |
| --- | --- | --- | --- | --- | --- |
| trims first (made) | 5 / 2 | 11 / 2 | 10 / 2 | 51 / 20 | 54 |
| one trim as two columns on top | 6 / 2 | 12 / 3 | 6 / 4 | 47 / 24 | 52 |
| one trim as one column on top | 3 / 4 | 3 / 8 | 3 / 6 | 13 / 43 | 56 |
| columns on top first | 0 / 7 | 3 / 8 | 0 / 9 | 2 / 54 | 59 |

An end on top that went back to the edge of the column under it (a butt
joint) took the columns on top to 1, 4, 6 and 38, but the gaps at the
places where the needle goes on were trims (6, 3, 6 and 36): not made.
The Varsity "I" shows the limit: its stem ends in a serif at each end,
so one serif sews before the stem, or there is a trim.

## Milestone 6: one way for all letters, and new fonts (M)

Updated on 2026-11-23 for the split of Milestone 8.

1. Lettering in TrueType fonts gets the same steps as the converter. (Both
   get the split already: it is in `Font::shapes`.)
2. The bundled embroidery fonts and the font packages are made again
   (`tools/make-embroidery-fonts.py`), audited, and the proof pictures of
   the ten tuned fonts are looked at. (This is also the open work of P20:
   the fonts with the stroke finder of P20 Milestone 3.) Latin letters and
   digits get the split with bridges and cuts, and the kana and kanji of
   the package `japanese` the split with bridges.
3. **The corrections of the bundled fonts**
   (`data/fonts/embroidery/corrections/*.json`) name the parts of the
   converted letter by their numbers (`order`, `reverse`, `join`), and the
   split changes the parts of many letters. Each correction is checked
   against the new letter: one that the split made unnecessary goes, and
   one that no longer fits is made again from the proof pictures.
4. The converter gives the route the outline of each letter of the
   source font (lettering in TrueType fonts has it since Milestone 5), so
   the travel runs of the embroidery fonts stay in their letters too.
5. The scores of all fonts are measured again. A font keeps its place
   (ADR 0116); a font in a package that now has `quality::BUNDLE` or more
   is a candidate for the program (the owner decides).
6. A sew-out kit: the test sheets of the ten tuned fonts at three sizes,
   the letters with the most faults of the table above, the Latin
   letters whose joints the split changed most ("N", "M", "W", "Z", "k",
   "x") and kanji with crossings and junctions (十, 木, 林, 東, 龍).
7. The characters that the split does not take (punctuation, Hangul,
   Arabic, Devanagari, Greek, Cyrillic) keep the stroke finder of today.
   Their measure with the split is open work for a later phase.

**Exit test:** the bench: the embroidery fonts are as good as lettering
in their source fonts or better; each font has a score of
`quality::LIMIT` or more; the owner sews the kit and sees no spikes,
bulges or open corners.

**Made (2026-11-23, ADR 0154).**

1. Lettering in TrueType fonts gets the steps of the converter: round
   dots and ends of columns on to the corners of the outline
   (`tune::round_parts`, `tune::extend_ends` in `Font::route`), not for
   kana and kanji. The converter does not extend kana and kanji either
   (`EmbFont::extend_ends`), and the Japanese fonts get `--extend` for
   their Latin letters. The converter routes with the outline of each
   letter of the source font (`EmbFont::route_from`), and rounds the
   coordinates of the file when the font is done (`EmbFont::round_letters`,
   before the score): rounded before the tuning, the ends went on less far
   than in lettering (Anton "S").
2. The tuning of the ends keeps a change only if the changed columns have
   no more needle points out of the outline (`quality::spike_points`): with
   no such check, it took the open corners from 27 to 7 in Block Sans but
   the spikes from 2 to 8 (Anton 23 to 35, Pacifico 25 to 35). A new piece
   of a rail stops where it meets the outline (`tune::clip_at_outline`).
   A small round part with two or more columns (the stroke finder cut the
   short middle line of a dot at a corner: the colon of Pacifico, 87.5 %
   covered) gets one column along its long axis.
3. The corrections of the bundled fonts: there are none in the
   repository, so nothing was to check.
4. All the bundled fonts and the font packages are made again. The scores,
   all measured with the score of today (the files of before measured
   again): 78.9 to 85.5 for the 55 fonts of the program;
   46 higher, 1 the same, 8 lower by 1 or 2 (Varsity 96 to 94, Plain Sans 93 to 91, Sturdy Sans 94 to 93, Chakra Petch 89 to 87, Soft Slab 87 to 85, Poppins 89 to 88, Indie Flower 86 to 84, Wide Sans 89 to 88).
   The fonts of before that are below the limit with the score of today
   (Round Heavy and Balsamiq Sans, 61) are at 80 and 83. No font is below
   the limit. The fonts in packages that now have `quality::BUNDLE` or
   more (Comic Narrow, Compact Heavy, Jazz Hand, Fat Face Serif, Neucha, Noto Sans JP, Angular Sans, Barlow Semi Condensed, Barlow, Cabin Condensed, Classic Sans, Game Sans, Kanit, Khand, Machine Sans, Rubik Mono One, Wide Heavy, Bookish Slab, Sturdy Serif) are candidates for the program: the owner decides
   (ADR 0116).
5. The sew-out kit is `tools/make-sewout-kit-p21.py` (`out/p21-sewout`, 36
   designs: the ten tuned fonts at three sizes, joints and the letters
   with the most faults in the four fonts of the bench, kanji in Noto Sans
   JP, the Arabic word).

The bench, the embroidery fonts made before P21 and now (25 mm):

| Font | Spikes | Crossing pairs | Bulges mm | Open area | Open corners |
| --- | --- | --- | --- | --- | --- |
| Block Sans | 153 to 2 | 5 to 15 | 0.6 to 0.0 | 0.96 % to 0.17 % | 35 to 14 |
| Anton | 223 to 25 | 232 to 23 | 15.3 to 2.9 | 2.71 % to 1.14 % | 113 to 26 |
| Pacifico | 575 to 27 | 153 to 15 | 16.4 to 14.4 | 1.67 % to 0.34 % | - |
| Varsity | 55 to 0 | 76 to 7 | 0.2 to 0.0 | 0.76 % to 0.06 % | 58 to 0 |

The embroidery fonts and lettering in their source fonts now have the
same letters (the same steps), and the bench gives them the same numbers
but for the rounding of the file. The exit test is met in the software:
the embroidery fonts are as good as lettering, and each font has
`quality::LIMIT` or more. The sew-out of the kit is the owner's part of
it.

## Milestone 7: a middle line from a Voronoi diagram (trial) (M)

Added by the owner on 2026-11-23, after a look at open-source tools. No
open-source program converts a font to satin letters automatically
(Ink/Stitch uses fonts that people digitize by hand); the programs that do
it are closed. But open-source libraries make the middle line of a shape
with no grid.

The stroke finder makes the middle line on a grid of cells (thinning,
`grid::thin`), and many fixes of Milestones 1 to 3 are for faults of the
grid: the middle line is up to one cell off the middle, its ends bend to
a corner, side branches need rules, and the feet on the outline must be
found again. The Voronoi diagram of the segments of the outline gives the
exact middle line: each of its edges has the two segments of the outline
that make it (the feet of Milestone 1) and its distance to them (the
half-width).

1. **The middle line from the outline**, in a new module of the stroke
   finder, beside the grid: `boostvoronoi` (a port of the Voronoi diagram
   of Boost.Polygon to pure Rust, licence BSL-1.0), or `centerline`
   (built on it, MIT or Apache-2.0; for shapes like letters: rings with
   holes). An ADR, the licence list (`tools/third-party-licenses.py`), and
   rule 9 (pure Rust: no exception).
2. **The graph of the strokes from it**: the junctions, the side
   branches, the links at junctions and the pieces. The rails, the ends
   and the mitres of Milestones 1 to 3 stay.
3. **Outlines that the library does not take** (rings that touch other
   rings): clean them first, or keep the grid for that glyph.
4. **A switch** between the two middle lines, and the letter fault bench
   and the lettering bench of P20 for both.

**Decision (exit test):** the new middle line stays if, in the four fonts
of the bench, it is better in spikes, crossing pairs, open area, open
corners and bulges, the lettering bench of P20 keeps its exit tests, and
a glyph takes at most two times as long. Else the grid stays, and the
plan records the numbers of the trial.

**Made (2026-11-23, ADR 0151): the grid stays.** The module `medial`
made the middle line from the diagram of `boostvoronoi` 0.12.1: the
edges inside the outline (nonzero winding), with the exact half-width at
their points, into the graph of the stroke finder (`Graph::from_medial`);
the prune, the links, the chains and all the steps after them did not
change. A shape with segments that cross or overlap got the grid (3
characters of the Japanese reference set, none of the bench letters).
The trial measured three ways at the corners of the outline: (1) no edge
of the diagram from a corner; (2) the edge from each corner where the
outline turns 40 degrees or more, as the grid has a branch there; (3)
the half of that edge that is nearer to the middle line. The letter
fault bench (lettering in the source fonts, 25 mm):

| Font | Way | Spikes | Crossing pairs | Open area | Open corners | Bulges mm |
| --- | --- | --- | --- | --- | --- | --- |
| Block Sans | grid | 17 | 27 | 0.57 % | 43 of 233 | 0.0 |
| Block Sans | 1: no corner edges | 63 | 14 | 0.61 % | 66 | 0.0 |
| Block Sans | 2: corner edges | 122 | 4 | 0.52 % | 44 | 0.0 |
| Block Sans | 3: half corner edges | 73 | 11 | 0.61 % | 55 | 0.0 |
| Anton | grid | 44 | 40 | 1.67 % | 54 of 330 | 4.3 |
| Anton | 1 | 28 | 52 | 1.84 % | 62 | 5.8 |
| Anton | 2 | 95 | 81 | 1.26 % | 30 | 8.9 |
| Anton | 3 | 53 | 49 | 1.69 % | 61 | 5.4 |
| Pacifico | grid | 37 | 40 | 0.56 % | - | 13.5 |
| Pacifico | 1, 2 and 3 | 12 | 33 | 0.35 % | - | 14.7 |
| Varsity | grid | 15 | 11 | 0.49 % | 33 of 531 | 0.0 |
| Varsity | 1 | 24 | 19 | 0.54 % | 67 | 0.0 |
| Varsity | 2 | 90 | 27 | 0.52 % | 52 | 0.0 |
| Varsity | 3 | 32 | 24 | 0.38 % | 56 | 0.0 |

The lettering bench of P20 (the test font, 25 mm):

| Way | Covered, lowest | Two layers, mean | Characters over 3 % |
| --- | --- | --- | --- |
| grid | 96.0 % (ぎ) | 1.2 % | 19 |
| 1 | 92.7 % (ァ) | 1.2 % | 12 |
| 2 | 93.8 % (ぬ) | 3.9 % | 154 |
| 3 | 94.4 % (ぬ) | 1.6 % | 32 |

No way is better in all four fonts, and no way keeps the coverage of the
P20 exit test (95 % of each character). The time is not a problem: the
fault bench took 23.1 s with the diagram and 21.9 s with the grid, the
lettering bench 7.6 s and 8.0 s.

What the numbers and the proof pictures show:

- The middle line from the diagram is better on round strokes. In
  Pacifico all three ways have fewer spikes (37 to 12), fewer crossing
  pairs (40 to 33) and less open area (0.56 % to 0.35 %); the open wedge
  at the top of the middle arch of "m" is gone. Way 1 also has fewer
  Japanese characters with two layers (19 to 12).
- It is worse at square ends and sharp corners. The steps after the
  middle line (the prune of side branches, the free ends, the mitres)
  were tuned on the middle line of the grid. Thinning takes the line far
  into a square end and stops it before a sharp corner; the diagram
  stops at the fork of a square end and goes into the point of a sharp
  corner. With no corner edges (way 1), the flat base of Anton "v" is
  open (the short line from the junction to the base goes as a side
  branch) and the ends of the diagonal of Block Sans "Z" go out of the
  letter. With the corner edges (ways 2 and 3), long edges to sharp
  corners are not side branches, and they become columns.

The code of way 1 is on the branch `trial/voronoi-middle-line` (not in
`main`): `emb_letter::medial`, `stroke::MiddleLine`, `strokes_with`, and
`--middle grid|voronoi` for the benches. A middle line from the diagram of
each stroke alone (after a split of the outline into strokes) has no
junctions and no side branches at its corners; a later trial can use
this code for it.

## Milestone 8: strokes from a split of the outline (trial) (L)

Added by the owner on 2026-11-23, after the trial of Milestone 7 and a
look at the research on the strokes of Chinese and Japanese characters.

The stroke finder finds the strokes from the middle line of the whole
letter: at each junction it links the edges that continue most straight,
cuts the shorter of two strokes that cross, and holds and cuts back the
ends that meet another stroke. Most faults at joints come from these
rules (Milestones 1 to 3), and the trial of Milestone 7 failed at them:
they are tuned on the middle line of the grid.

Research on Chinese characters (Make Me a Hanzi,
<https://github.com/skishore/makemeahanzi>; StrokeStyles, Berio and
others, ACM Transactions on Graphics 2022) splits the outline first. Where
a stroke meets or crosses another stroke, the outline has two concave
corners (corners that turn into the shape), one on each side of the
stroke that ends. A straight line between them (a bridge) continues the
edge of the stroke that goes through. The outline cut along all the
bridges gives pieces, and each piece is one stroke with no junction. The
middle line of each piece then needs no rules for junctions.

The code of Make Me a Hanzi has no license, so no code is copied: this
trial uses the method from its description, in our own code. Its stroke
data (Arphic Public License) is not used.

1. **Concave corners.** The points where the outline turns into the
   shape more than an angle over a short length (as `outline_corners`
   does for all corners), with the directions of the outline before and
   after each point.
2. **Bridges.** A bridge joins two concave corners. It is a candidate if
   it is in the shape (it touches the outline only at its ends), it is at
   most a factor of the width of the stroke at its ends long, and at each
   end it continues one of the two directions of the outline (the edge of
   the stroke that goes through). The cost of a candidate is its length
   (a factor of the width) and its angles to these directions. The
   bridges are the candidates with the least sum of costs, with each
   corner in one bridge at most (a corner can have no bridge: the inner
   corner of an "L" or of a stroke that turns). Where the bridges of two
   strokes cross (a "+", 十), the bridges that keep the longer stroke
   whole stay, as in `stroke::cut_crossings` today.
3. **Pieces.** The outline cut along each bridge. Each side of a piece
   is an edge of the outline or a bridge.
4. **Columns of a piece.** The stroke finder of today on each piece, with
   the middle line of the grid and with the middle line of the Voronoi
   diagram (the code of Milestone 7 on the branch
   `trial/voronoi-middle-line`). An end of a piece at a bridge is not a
   free end: the column ends on the bridge and goes `END_OVERLAP` into the
   piece on the other side. So the rules for ends at junctions (the held
   width, `trim_start_out_of`, `trim_ends`) are not used for these ends.
5. **A way back.** A letter whose bridges cross the outline or cross
   each other in a way that step 2 does not take, or a piece that still
   has a junction, gets the stroke finder of today for that letter or
   that piece.
6. **A switch** between the split and the stroke finder of today, for the
   letter fault bench and the lettering bench of P20, and proof pictures
   of the reference set of P20 with the bridges drawn.

Not in this trial: stroke data for each character (KanjiVG,
<https://kanjivg.tagaini.net>, CC BY-SA 3.0) to choose which pieces are
one stroke. It needs an ADR on the license of the data and a converted
data file; it is a later step if the trial wins and its joins are the
weak point.

**Decision (exit test):** each script is decided alone.

- Latin letters get the split if, in the four fonts of the letter fault
  bench, it is better in spikes, crossing pairs, open area, open corners
  and bulges (the rule of Milestone 7).
- Kana and kanji get the split if, in the lettering bench of P20, fewer
  characters have more than 3 % with two layers, each character is at
  least 95 % covered, the eight words have no more trims, and the proof
  pictures of the reference set show no new faults.
- A letter takes at most two times as long.

Else the stroke finder of today stays for that script, and the plan
records the numbers of the trial. The code is on the branch
`trial/stroke-split` (from `trial/voronoi-middle-line`) until a script
gets the split.

**Made (2026-11-23): the trial.** The module `split` (on the branch
`trial/stroke-split`) finds the concave corners, the bridges and the
pieces, and the stroke finder of today makes the columns of each piece
(`stroke::Split`, `emb-cli ... --split` or `--split-bridges`). Changes to
the steps above, from the measures:

- A bridge that does not cut the shape (it joins the outline to a hole,
  as at the bowl of an Anton "P" or "B") makes a slit, and the stroke
  finder made fans across it: such a bridge goes.
- A sharp concave corner with no bridge (a notch: the outline turns 110
  degrees or more there, as in the "V" of a "w" or the inner corners of
  an "N", "M" or "Z") gets a cut along the line that halves it, at most
  2 widths long. Without these cuts, the pieces with a sharp "V" had
  spikes (Varsity: spikes 15 to 20).
- A bridge is at most 1.3 widths long (not 2): a longer bridge went
  across the place where three strokes meet under the bar of 木 in 林,
  and 林 was 94.8 % covered at 15 mm.
- Where two strokes cross, the bridges that keep the longer stroke whole
  (step 2 of the plan) made the Japanese bench worse (7 to 8 characters
  over 3 % with two layers, 57 trims at 25 mm). The trial keeps the
  bridges with the least cost: the thinner stroke is cut.
- The end of a piece at a bridge or a cut goes 0.1 of the length of the
  bridge into the next piece. Less at the cuts (0.03 or 0) gave more
  crossing pairs in Varsity or less coverage in Japanese.

The letter fault bench (lettering in the source fonts at 25 mm), the
stroke finder of today and the split with bridges and cuts:

| Font | Spikes | Crossing pairs | Open area | Open corners | Bulges mm |
| --- | --- | --- | --- | --- | --- |
| Block Sans | 17 to 2 | 27 to 15 | 0.57 % to 0.22 % | 43 to 27 | 0.0 to 0.0 |
| Anton | 44 to 23 | 40 to 24 | 1.67 % to 1.33 % | 54 to 45 | 4.3 to 2.5 |
| Pacifico | 37 to 25 | 40 to 22 | 0.56 % to 0.45 % | - | 13.5 to 13.4 |
| Varsity | 15 to 0 | 11 to 6 | 0.49 % to 0.04 % | 33 to 1 | 0.0 to 0.0 |

The lettering bench of P20 (the test font; 15, 25, 50 and 80 mm):

| Way | Characters under 95 % covered | Characters over 3 % with two layers | Two layers, mean at 25 mm | Trims of the eight words at 10, 15, 25, 50, 80 mm |
| --- | --- | --- | --- | --- |
| today | 0 | 19, 19, 18, 18 | 1.22 % | 48, 49, 54, 56, 56 |
| bridges | 0 | 6, 4, 6, 6 | 1.16 % | 49, 50, 56, 56, 56 |
| bridges and cuts | 0 | 24, 21, 24, 24 | 1.46 % | 45, 50, 54, 56, 56 |

With the middle line of the Voronoi diagram (Milestone 7) in the pieces,
both scripts were worse: Block Sans 46 open corners, Pacifico bulges
15.7 mm, ア and ァ under 95 % covered. The fault bench took 29.9 s with the
split and 21.9 s without; the lettering bench 7.1 s and 8.0 s.

By the decision rule:

- Latin letters: the split with bridges and cuts is better in spikes,
  crossing pairs, open area and open corners in all four fonts, and in
  bulges where the stroke finder of today has any (Block Sans and Varsity
  have none either way). Latin letters get the split.
- Kana and kanji: the split with bridges has far fewer characters with
  two layers (19 to 4) and keeps each character 95 % covered, but the
  eight words have 1, 1 and 2 more trims at 10, 15 and 25 mm (龍 +2, 東京
  +1; 山田太郎 -1), the same at 50 and 80 mm. It misses one condition;
  the owner decides.

**In the program (2026-11-23, ADR 0152).** The owner chose the split
for both scripts: the trims of the kanji go to Milestone 5. The stroke
finder splits the glyphs of a TrueType or OpenType font by their
character (`stroke::Split::for_char`: Latin letters and digits with
bridges and cuts, kana and kanji with bridges, other characters not;
`Font` keeps the character of each glyph). The Voronoi middle line stays
on its branch. In the program the numbers of the fault bench are the
numbers of the trial; the lettering bench (the snapshot) has 5 to 7
characters over 3 % with two layers (18 to 19 before) and each
character 96.9 % covered or more at 15 mm and more (96.0 % before), with 1, 1 and 2 more
trims at 10, 15 and 25 mm. Travel runs out of the outline: Block Sans
0.4 mm to 2.3 mm (Milestone 5 removes them). The golden designs with
lettering in TrueType fonts have more stitches (lettering-names 2481 to
2597, mitre-corners 7623 to 7897, shaped-text 6899 to 6927) and the
same trims.

## Exit test of the phase

The exit tests of the six milestones, and the sew-out of Milestone 6.

## Order

The owner's order: Milestones 1 to 6. Milestone 7 (the trial) comes
before Milestone 6, so the fonts are made again with the middle line that
wins (the grid, ADR 0151). Milestone 8 (the trial of the split) comes
before Milestone 6, for the same reason; the owner chose to make it
before Milestone 5 (2026-11-23). The bench (Milestone 1, step 1)
comes first, as the measure of each milestone. The bundled fonts are made
again only in Milestone 6, so the sew-outs of the older milestones use
lettering in the TrueType fonts.

## Not in this phase

- New fonts, and letter corrections by hand.
- The fill letters, and the stitch engine of fills.
- Japanese characters as a target: the lettering bench of P20 shows the
  changes there, and they are checked with it. Milestone 8 is the one
  exception (the owner, 2026-11-23): its split is for the junctions of
  kana and kanji as much as for Latin letters.
