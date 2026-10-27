# ADR 0052: Picture to design: robustness and the picture bench

Date: 2026-10-25 · Status: Accepted (P12 Milestone 1)

## Context

The owner asked to make the Picture to design wizard (ADR 0029 to 0031)
more effective and more robust. The plan is `docs/design/p12-plan.md`. A
measurement (release build, a noisy picture of 1200 × 900 pixels) found
that each step ran on the thread of the window: at fine detail Auto took
8 s, so the window stopped. Of the 8 s, 6 s were five large areas with a
ragged edge and hundreds of holes (specks of other colors). Auto took
them as narrow, because it measured the width as twice the area over the
edge length, and the satin finder then worked on a large grid. Also, the
program did not apply the EXIF orientation of a photo, so a photo from a
phone could come in on its side. There was no way to measure whether a
change made the results better.

## Decision

1. **Pictures come in upright.** `emb_art::decode` applies the EXIF
   orientation, and `emb_art::picture_size` gives the upright size without
   a full decode. The backdrop, the product picture, a picture changed in
   another program and the wizard use them. An old project with a turned
   backdrop gets its upright pixel size when it opens (`backdrop::upright`
   in `start_design`, not an undo step); the width and the center stay.
2. **The wizard works in the background.** The wizard is the state of the
   window and a work state (`Work`: the picture, the settings and the
   results with a key of their inputs). The keys are quick (they hash the
   inputs). The steps up to Background are quick and are made on the
   window thread; for the later steps a job (a thread) gets a copy of the
   work state and makes the preview. The last picture stays with a
   "Working…" note. One job runs at a time: when it ends, the window takes
   its results (the settings stay as they are now; new colors reset the
   color choices, as before) and, if the settings changed, starts a job for
   the newest settings. The controls show only results for the settings as
   they are. Finish waits for the job. A job that fails is not tried again
   until a setting changes.
3. **Auto measures the true width.** A distance map of the area pixels
   gives the mean width (4 × the mean distance to the edge, less 2), so a
   ragged edge does not make an area narrow. Auto tries a satin only on an
   area with two holes or fewer. On the measured picture Auto takes 0.5 s.
4. **The picture bench** (`emb_cli::bench`, `emb-cli bench [<folder>]`):
   the wizard steps with their defaults on six test pictures that the
   program makes (no license question, the same on each computer; a
   JPEG-like picture comes from our own 8 × 8 DCT quantization, because a
   JPEG decoder can differ in the last bit between processors), and on the
   pictures of a folder. For each picture: objects, satins, colors,
   stitches, trims, the part of the picture that the stitches cover, the
   stitches outside it, the mean ΔE between a picture of the stitches and
   the picture, and the time of each step. The golden snapshot
   `bench__picture_bench.snap` keeps the numbers without the times. The
   steps that the bench and the wizard share are in `emb_art::smart`
   (`work_picture`, `detail_px`, `simplify`).

## Consequences

- The `smart-logo` corpus design has a 4 mm bar now: its 6 mm bar is
  wider than the satin limit of Auto (5 mm), which the old measure did not
  see.
- A job cannot be stopped. The stitch plan of a fill with hundreds of
  holes still takes seconds at fine detail; Milestone 2 (small areas on
  top, not holes) makes this smaller. The window stays responsive.
- The Background tools (remove the edge color: 0.5 s on a large picture,
  cut out the subject: 0.2 s) still run on the window thread.
