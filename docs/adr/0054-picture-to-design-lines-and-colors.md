# ADR 0054: Picture to design: line art, smoothing, thread chart colors

Date: 2026-10-25 · Status: Accepted (P12 Milestone 3)

## Context

Picture to design traced the outlines of a drawing as thin color areas:
fills or satins in the place of their color in the sew order, so an
outline could sew under the areas that it borders, and an outline network
with many holes became a fill. JPEG blocks and the grain of a scan gave
small areas of near colors, and the colors came from the picture, so two
of them could go to one thread later. Plan: `docs/design/p12-plan.md`,
Milestone 3. The methods are common digitizing and picture practice, in
our own words.

## Decision

1. **Line art** (`SmartSpec::line_art`, on at the start). With Auto, an
   area at most 2 mm wide (`LINE_MAX_MM`, the mean width of ADR 0052), at
   least 8 times as long as wide (`LINE_RATIO`) and with a CIELAB
   lightness of 40 or less (`LINE_DARK_L`) is a line: a run along its
   middle if it is narrower than 1 mm (`LINE_RUN_MAX_MM`), else a narrow
   satin, with any number of holes. A satin this narrow sews wider than
   the drawn line (its pull compensation and the thread); a run is near
   it. A run of a line 0.6 mm wide or more (`BEAN_MIN_MM`) is a triple
   (bean) run. A long area of any color narrower than 1 mm is a run too,
   in its place in the sew order. The lines sew after all areas; for
   the layers (ADR 0053) they count as the last color, so an area sews
   over a line inside it. The fill test of the layers uses the pixel area
   of each hole (`Area::hole_px`), because the smoothing makes a thin hole
   larger. The inside point of an area is the deepest pixel nearest to
   the center of the deepest pixels, so it is in the middle of a thin line.
2. **Edge-preserving smoothing** (`emb_art::adjust::smooth`, a bilateral
   filter: 5 × 5 pixels, distance spread 1.5 pixels, color spread 20 RGB
   units; weights from tables made with `emb_geom::math::exp`). Simplify
   has "Even out" (passes, 1 at the start) on the simplified picture,
   after the median and before the color reduction.
3. **Specks:** Simplify removes specks under 2 mm² at the start (was
   1 mm²). On the owner's photos this gives 16 to 26 % fewer objects and
   trims at the same color difference, and a dot of 2 mm (3 mm²) stays.
4. **Colors from the thread chart** (`reduce::reduce_with`). After the
   k-means, each color found goes to the nearest thread of the chart
   (CIELAB); colors that go to one thread are one color, and each pixel
   takes the nearest of the threads. The Simplify step has "Colors from
   the thread chart" (the chart of the Threads panel; off at the start).

## Consequences

- Bench (ADR 0052), against Milestone 2: the line-art picture goes from
  color difference 6.6 to 2.1, 2835 to 2106 stitches and 4.2 % to 0 %
  outside (98 % covered: a run is a little narrower than the line); the
  busy portrait from 28 objects and 24 trims to 10 and 10; the large
  picture from 8 objects to 4; the badge color difference from 4.1 to
  3.5; the JPEG-like logo from 5.4 to 5.3. (With the first run limit of
  0.8 mm the smoothing made the mouth line of the line-art picture 0.85
  mm wide, a satin, and the color difference 7.2.)
- The owner's photos (2026-10-25: grain silos, a cross-stitch picture, a
  space eye, a painted knight; 100 mm wide, 6 colors, the edge color
  removed) run in under 0.6 s for each step. They give 64 to 272
  objects, with a trim for nearly each one: a photo has many areas of
  one color apart from each other. Picture to design with color areas
  suits logos and drawings best; for such photos the cut-out of the
  subject, more colors or Photo stitch do better. The stitches outside
  the picture (1.5 to 13 %) are a band round each area: the overlap and
  the pull compensation, on purpose.
- `emb-cli bench` has the Simplify settings as options (`--width`,
  `--detail`, `--colors`, `--specks`, `--smooth`, `--denoise`,
  `--background keep|edge`) and writes the stitch pictures with `--out`.
- The golden snapshots of the corpus did not change (`smart-logo` has no
  line art and does not use Simplify).
- The bench runs with the colors of the picture (no chart).
