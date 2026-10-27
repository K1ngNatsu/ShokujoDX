# ADR 0012: Runs can ask for a stop, and the plan records why it stops

Date: 2026-10-22 · Status: Accepted

## Context

Before P3 Milestone 7, the machine stopped only after an object (the stop
function, Milestone 2). An appliqué object needs stops inside it: after the
placement line, the operator puts the fabric on; after the tack-down, the
operator cuts the fabric. Several pieces must share these stops (combined
appliqué). The worksheet must also tell the operator what to do at each
stop.

## Decision

- An object's stitches are runs (`emb_stitch::Run`). A run has an optional
  `stop: Option<StopReason>`. The plan writes a stop before the run, after
  the trim if there is one. The thread is not cut for the stop itself.
- `RunLink::Travel` joins a run to the one before it with a stitch, also
  over a gap longer than the trim distance. Appliqué uses it at a step
  change on one ring, where the cover sews over the travel stitch.
- `StitchPlan::stops` lists each stop with its record, its object and its
  reason (`Function`, `PlaceFabric`, `TrimFabric`).
  `StitchPlan::stop_notes` gives one line of text for each stop, for the
  worksheet.
- A stop before the first stitch of the design is dropped: the operator
  does that work before starting the machine.

## Consequences

- The machine formats need no change: they already write a stop record
  (VP3 as a color block with the same thread, DST and others as a color
  change).
- The output of existing designs does not change (the golden snapshots of
  the six earlier corpus designs are the same).
- Other features that need stops inside an object (for example sequins or
  a second appliqué fabric layer) can use the same field.
