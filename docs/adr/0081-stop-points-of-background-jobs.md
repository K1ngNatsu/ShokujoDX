# ADR 0081: Stop points of background jobs

Date: 2026-10-29 · Status: Accepted

## Context

The Picture to design wizard makes the previews after the Background step
in a background job (P12 Milestone 1). A job could not stop: the engine
functions had no stop point (P12 plan, "Not in this phase"). A change of a
setting while a job ran had to wait for the end of the old job, and a
preview that took long could not be cancelled. On a picture with 18 000
areas, the objects take 2.3 s and the stitch plan 0.4 s. Also, the cut-out
of the subject and the removal of the edge color (up to 0.5 s on a large
picture) ran on the window thread, so the window did not respond.

## Decision

- The long engine functions have a variant with a stop function:
  `emb_stitch::build_plan_until` (a stop point before each object) and
  `emb_art::smart::digitize_until` (after the trace and before each area).
  When the stop function gives `true`, the work stops and the result is
  `None`. `build_plan` and `digitize` call them with a stop function that
  never stops. A stop function is a plain closure: the engine crates get
  no thread or UI type.
- Each wizard job has a stop flag (`Work::stop`). The wizard sets it when
  a setting of the step of the job changes (the result is old) or when the
  user clicks **Stop**. A stopped job gives back the results that it made
  (each with its key), and no picture. After **Stop**, the wizard does not
  make the same preview again until a setting changes.
- The cut-out, the marks and **Remove the edge color** run on a thread
  (`Wizard::start_edit`). The other changes of the picture wait until the
  change is done; Finish waits for it.

## Consequences

- A stop takes effect in about 10 ms while the objects are made, and in
  about 0.3 s in the stitch plan part (the preview design is made before
  the first stop point).
- One area or one object is the smallest step: a single very large fill
  can still take its time before the job stops.
- Other jobs (the export queue) can use the same functions when they need
  to stop.
