# Plan: roadmap phase P12 (Picture to design: robust and better results)

Date: 2026-10-25 · Status: Accepted (owner request, 2026-10-25). Milestones 1 to 3 done in software.

## Goal

The Picture to design wizard (P6 Milestones 7 and 8, ADR 0029 to 0031)
makes embroidery from a picture. The owner asked to make it more
effective and more robust. A review of the wizard and its engine
(`emb_art`) on 2026-10-25 found ten items. The owner put them in three
milestones:

1. Robustness: the wizard must not fail or stop the program on real
   pictures, and a bench of test pictures must measure the results.
2. Better areas: fewer trims and objects, fills that look sewn by hand,
   and checks before Finish.
3. Lines and colors: outlines as lines, cleaner colors from noisy
   pictures, and colors from the thread chart.

The methods are common digitizing practice, in our own words. Commercial
programs have auto-digitizing; their code is closed (spec, rule 3).

A measurement on 2026-10-25 (release build, a noisy picture of 1200 ×
900 pixels): the color reduction takes up to 0.2 s, the stitch plan up
to 0.3 s, and the stitches of the color areas up to 8 s at fine detail
(3710 areas). Of the 8 s, 6 s are five large areas with a ragged edge:
Auto measures their width as twice the area over the edge length, the
ragged edge makes them "narrow", and the satin finder then works on a
large grid. 1 s is 3629 areas smaller than 2 mm.

## Order

Milestone 1 comes first: the bench gives the numbers that show that the
later milestones make the results better.

| # | Scope | Exit test |
| --- | --- | --- |
| 1 | **Robustness.** (a) Photo orientation: a picture from a phone has an EXIF orientation; each picture import applies it (backdrop, product picture, a picture changed in another program, the wizard); an old project with a turned backdrop gets the right pixel size when it opens. (b) The wizard works in the background: the previews of the steps after Background (Simplify, Colors, Stitches, Photo, Cross stitch, Finish) are made on another thread, so the window stays responsive; the last picture stays on the screen with a "Working…" note; only one job runs, and when it ends a job for the newest settings starts; Finish waits for the job. (c) Auto measures the width of an area with a distance map of its pixels, so a large area with a ragged edge is a fill and not a slow satin attempt. (d) The picture bench: `emb-cli bench` runs the wizard steps on a set of test pictures that the program makes (a logo saved as JPEG, a small icon, a large picture, a PNG with soft clear edges, line art, a busy portrait-like picture) and on the pictures of a folder that the user gives; for each picture it gives the objects, colors, stitches, trims, how much of the picture the stitches cover, the stitches outside the picture, the mean color difference (ΔE) between the stitch picture and the picture, and the time of each step. A golden snapshot keeps the numbers (not the times). | Unit tests; the bench runs; a phone photo opens upright; the window stays responsive at the finest detail (the owner tries with own photos). |
| 2 | **Better areas.** (a) Layers: a small area inside a larger one sews on top of it and is not a hole in it (a size limit; on by default for areas under the limit), so fills do not break into parts and no gap opens when the stitches pull. (b) Fill angles: each fill sews along its long axis, and fills that touch get angles at least 30° apart. (c) Small areas in mm: an area smaller than a limit (2 mm) becomes a short satin or a run, or goes into the color round it; no tiny fills with underlay. (d) Checks on the Finish step: the Design info checks of the result (density, small objects, trims, long stitches) before Finish, each with a fix where one exists (for example, a larger speck size, or two near colors merged). | Unit tests; the bench gives fewer trims and objects at the same color difference; a logo sews on the Topaz 20 with no gaps. |
| 3 | **Lines and colors.** (a) Line art: Auto finds thin, long, dark areas (outlines of a drawing) and sews them as a run or a narrow satin along their middle line, after the areas under them. (b) Edge-preserving smoothing before the color reduction (a filter that makes flat areas even and keeps the edges), for JPEG noise and scans. (c) Colors from the thread chart: the color reduction can take its colors from the thread chart of the Threads panel; two colors that go to one thread are one color. | Unit tests; the bench gives a smaller color difference and fewer objects on the JPEG and line-art pictures; a drawing with outlines sews on the Topaz 20. |

Then the exit test of the phase: the exit tests of each milestone pass.

## Not in this phase

| Item | Reason |
| --- | --- |
| Stopping a job that runs | The engine functions have no stop point. With the faster Auto (Milestone 1) and fewer small areas (Milestone 2), one job is short. Revisit if the bench shows long jobs. |
| A learned model (neural network) for the cut-out or the colors | Large model files and a new kind of dependency. The graph cut of ADR 0031 does the job. |

## Progress

| # | State | Notes |
| --- | --- | --- |
| 1 | Done in software | ADR 0052. Pictures upright (`emb_art::decode` applies the EXIF orientation, `emb_art::picture_size`): backdrop, product picture, a picture changed in another program, the wizard; an old project with a turned backdrop gets its upright size when it opens. The wizard is a window state and a work state (`Work`); the previews after Background are made by one background job at a time (the last picture with "Working…"; the newest settings next; Finish waits). Auto measures the mean width with a distance map (`AreaMap::width`) and tries a satin only on areas with two holes or fewer: Auto at fine detail 8 s → 0.5 s. `emb-cli bench [<folder>]` (`emb_cli::bench`): six test pictures that the program makes and the pictures of a folder; objects, colors, stitches, trims, covered, outside, ΔE, times; snapshot `bench__picture_bench.snap` (no times). Shared steps in `emb_art::smart` (`work_picture`, `detail_px`, `simplify`). The `smart-logo` corpus bar is 4 mm now (6 mm is over the Auto satin limit); its golden snapshot changed, the others did not. Not done: the exit test (a phone photo upright, the window responsive at the finest detail, with the owner's photos). |
| 2 | Done in software | ADR 0053. `emb_art::smart::digitize`: details on top (a hole up to `layer_max_mm2`, 25 mm², that later colors fill is sewn over; `Area::inside`, a grid of 5 mm cells); fill angles (the long axis from the second moments, 1.3 times as long as wide; fills that touch 30° apart, `AreaMap::touching`); small areas (under `small_mm2`, 4 mm²: Auto makes a satin, or a run under 0.8 mm wide; small fills have no underlay); the Stitches step has the three settings. Finish step: checks made by the background job (build-up, more than 30 trims, objects under 1 mm, stitches over 7 mm, colors nearer than ΔE 8), each with a fix button where one exists. Bench: new picture `badge`; stitches down by up to 12 % at the same or a smaller color difference; trims the same (details of one color apart). The `smart-logo` golden snapshot changed (fill angle of the leaf). Not done: the exit test (a logo on the Topaz 20 with no gaps). |
| 3 | Done in software | ADR 0054. Line art (`SmartSpec::line_art`): with Auto, an area at most 2 mm wide, 8 times as long as wide and of lightness 40 or less is a run (under 1 mm; a triple run from 0.6 mm) or a narrow satin, with any holes, sewn after all areas (the last color for the layers); a long area of any color under 1 mm is a run in its place; the layers use the pixel area of each hole (`Area::hole_px`). Specks under 2 mm² go at the start (was 1 mm²). Edge-preserving smoothing `emb_art::adjust::smooth` (bilateral, 5 × 5): Simplify > Even out (1 at the start). Colors from the thread chart (`reduce::reduce_with`): Simplify > Colors from the thread chart. Bench against Milestone 2: line art color difference 6.6 → 2.1, stitches 2835 → 2106, outside 4.2 → 0 %; busy portrait 28 → 10 objects, 24 → 10 trims; large picture 8 → 4 objects; badge color difference 4.1 → 3.5; JPEG-like logo 5.4 → 5.3. The owner's four photos (ADR 0054) run in under 0.6 s a step and give many objects with a trim each (photos suit the cut-out, more colors or Photo stitch better). `emb-cli bench` has the Simplify settings as options and `--out` for the stitch pictures. Golden snapshots of the corpus: no change. Not done: the exit test (a drawing with outlines sewn on the Topaz 20). |
