# ADR 0056: Border position

Date: 2026-10-26 · Status: Accepted (owner request)

## Context

A border trace put its border outside the edge of the objects (a column
0.4 mm over the edge, ADR 0025), and the border of Offsets sat outside
its outline. The owner asked to choose where the border sits on the
line: outside it, centered on it, or inside it.

## Decision

1. `emb_model::BorderPlace` (Outside, Center, Inside) is one setting for
   both: `TraceSpec::place` and `OffsetMake::Border::place`. Outside and
   inside are away from the middle of the shape and toward it, also for
   an outline of Offsets inside the objects. `BorderPlace::middle_mm`
   gives the distance of the middle of the border from the line.
2. Border trace: the outer edge of the border is the old one (width −
   0.4 mm for a column) outside, half the width out centered, and on the
   edge inside. The cut of the objects under it, and the gap, are
   measured from that outer edge as before, so the cut pieces still go
   0.4 mm under a column border.
3. Outside is the default, and a trace with Outside writes no `place`
   field, so old projects and the golden snapshots do not change.
4. The windows (Border trace, Offsets) have the row **Border position**;
   the app keeps the last choice for new borders and for Add border.

## Consequences

An Offsets border inside the objects with Outside now sits between its
outline and the edge of the objects; before this change it sat further
in (the old border went away from the edge for inside offsets). Choose
Inside for the old result.
