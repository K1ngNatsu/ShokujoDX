# ADR 0077: Resource limits for low-end computers

Date: 2026-10-27 · Status: Accepted

## Context

A tester stopped the program on a computer with 16 GB of memory and a
Ryzen 5 3600 while the tester used the Picture to design wizard. There is
no crash report yet. The owner wants the program to run on almost any
computer.

We measured the heap peak of each step of the wizard and of the picture
tools of the backdrop, in a release build, with a counting allocator
(synthetic JPEG photos: 4032 × 3024 with EXIF orientation 6, 6000 × 4000,
12000 × 8000, and a noisy 12000 × 8000). The wizard works on a picture of
at most 1200 pixels (`smart::WORK_PX`, ADR 0029), so the steps after the
decode used less than 250 MB. The large parts were:

| Step (12000 × 8000 photo)              | Before   | After    |
|----------------------------------------|----------|----------|
| Backdrop texture (decode to 4096 px)   | 835 MB   | 335 MB   |
| Wizard open (decode to the work size)  | 910 MB   | 452 MB   |
| Wizard open, noisy photo               | 955 MB   | 496 MB   |
| Prepare colors (picture tools)         | 1482 MB  | 195 MB   |
| Trace (picture tools, 82 629 areas)    | 2203 MB  | 281 MB   |

Background, cut-out, Simplify, Stitches, Finish, photo stitch and cross
stitch stayed between 30 and 220 MB before and after. The Finish preview
of a design 1000 mm wide at the most detail (718 581 stitches) took 13 to
30 seconds, in a background job.

We also found these causes of a stop that are not the amount of memory:

- **Texture size.** `egui-wgpu` asks the graphics card for textures of
  8192 pixels on a side. A larger texture is a validation error of
  `wgpu`, and the program stops. The stitch picture of the Finish step of
  a tall, narrow picture (and the density map of a long design) can be
  larger.
- **Stack.** The curve fit of auto-trace (`emb_geom::simplify`) called
  itself once for each cut of a rough edge. A long edge can need
  thousands of cuts, and a stack overflow stops the program with no crash
  report. The main thread of Windows has a stack of 1 MB, other threads
  2 MB.
- **Panics.** A panic in the wizard window (on the main thread) stopped
  the program.
- **Waits.** Finish waited on the main thread for the job of the preview
  (up to 30 seconds). Windows then shows the program as not responding.
  After Finish, the canvas made the stitch plan of the new design on the
  main thread: 16.6 seconds for a design of 718 581 stitches (a photo
  1000 mm wide at the most detail), 0.06 seconds at 300 mm and normal
  detail.

## Decision

1. **Decode to the size that is used** (`emb_art::decode_fit`,
   `smart::decode_work`). The picture is made smaller in the color type of
   the file, before the conversion to RGBA and before the EXIF turn, so
   only one full-size copy is in memory (3 bytes a pixel for a JPEG).
   `decode_work` uses the triangle filter of `smart::resample`: the work
   picture has the same pixels as before (a picture turned by EXIF can
   differ by rounding). `decode_fit` (backdrop texture, product photo,
   picture tools) uses a box filter, which needs no large work buffer.
2. **A memory budget for each decode.** The decoder may use half of the
   free memory (`sysinfo`, its `system` part; at least 256 MB, at most
   2 GB; `resources::decode_budget`). A larger picture is refused with a
   message (`emb_art::TOO_LARGE`) before the pixels are allocated.
3. **The picture tools work at the size of the canvas.** Adjust, Prepare
   colors, Trace and Click to stitch use the backdrop at most 4096 pixels
   on its long side (`picture::TOOL_PX`). Adjust and Prepare colors put a
   picture of that size in the backdrop. Crop keeps the full size.
4. **Textures fit the graphics card.** Each texture that can be large goes
   through `resources::load_texture` or is decoded to
   `resources::max_texture_side` (the limit that the card gave, 2048 if it
   gave none).
5. **No recursion on picture data.** The curve fit keeps its parts on a
   list. The wizard job thread has a stack of 64 MB
   (`resources::JOB_STACK`; reserved, not used).
6. **An error in the wizard closes the wizard, not the program.** The
   window runs in `catch_unwind`; the crash report is written as before,
   and the message asks for less detail or a smaller picture. An engine
   error in a job was already a note.
7. **Finish does not wait on the main thread.** It waits for the job in
   the background, and the window stays usable.
8. **The plan that the canvas shows is made in a thread.**
   `AppState::shown_plan` (the canvas, the overlay, the status bar, the
   density map, the overview, the player, the repeats, the reshape
   markers, the stitch list and the kiosk) starts a thread for the plan of
   a new design state and waits for it at most 25 ms (`FRAME_WAIT`). A
   slower plan shows when it is done; until then the last plan shows with
   the note "Working…", and the thread asks egui to paint again when it
   ends. One thread at a time: it has the stitch cache, which comes back
   with the plan. `AppState::plan_view` stays exact: export, output, Send
   to USB, the sheets and the commands wait for the plan of the design as
   it is. A plan thread that stops with an error keeps the last plan; the
   state is not tried again in the background.

The background threads were already one at a time for each kind of work
(the wizard preview, the export queue, the library scan, the font scan,
the start page pictures). The program uses no thread pool, so the number
of threads does not grow with the cores of the computer.

## Consequences

- A 12000 × 8000 photo goes through the wizard with a heap peak under
  0.5 GB. The `emb-cli bench` results do not change (the bench pictures
  are not decoded from files).
- Adjust and Prepare colors make a very large backdrop smaller (4096
  pixels). The canvas showed it at that size already.
- `sysinfo` now also has its `system` part (one more crate, `ntapi`, on
  Windows). The exception of ADR 0036 for its `unsafe` code covers it.
- A frame waits at most 25 ms for a new plan. A small design (most
  edits) still shows its new plan in the same frame.
- The Needles window and the commands still wait for the exact plan, so
  they can take some seconds on a very large design.
