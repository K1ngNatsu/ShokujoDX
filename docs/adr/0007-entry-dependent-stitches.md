# ADR 0007: Fill and satin stitches start from the needle position

Date: 2026-10-21 · Status: Accepted

## Context

Running stitches can start at either end, so the stitch plan picks the end
nearer to the needle after the object is stitched (ADR 0004 cache key
`(ObjectId, revision)`). A fill or a satin cannot be turned around after it is
stitched: the underlay must sew first, and a tatami fill chooses the order of
its sections from where it starts. To start a fill or satin near the end of the
previous object ("auto start and end", roadmap P1), the stitches must depend on
where the needle is.

## Decision

- `stitch_object(obj, entry)` takes the needle position. Running objects
  ignore it. Fill and satin objects start at the corner or column end nearest
  to it. A start point that the user set wins over the needle position.
- The needle position is rounded to a 1 mm grid (`ENTRY_GRID_MM`) before use,
  and the cache key becomes `(ObjectId, revision, rounded entry)`. The object is
  stitched from the rounded position, so the result is a pure function of the
  key, and two plans with the same design are equal (spec, determinism).
- The plan passes the needle position in design coordinates (it undoes the
  design rotation and the hoop offset).
- Runs from fill and satin objects are marked `fixed`: the plan does not
  reverse them or start them at another point.

## Consequences

- An edit of one object can change the entry of the objects after it, so
  those fill and satin objects are stitched again. A move smaller than the
  grid does not. In release builds a 44 000-stitch design (the corpus
  `full-field`) is planned in about 40 ms, so this is acceptable. If large
  designs become slow during a drag, stitch the later objects at the end of
  the drag only.
- Undo restores the old entries, so it is still a cache hit.
- A fill sewn from a different entry has different travel, so golden tests
  compare whole plans, not objects alone.
