# ADR 0096: Endless pan

Date: 2026-11-16 · Status: Accepted

## Context

The owner asked for an endless drag in the canvas, as Blender has (v0.3
plan, Stage 3): a pan stopped when the pointer came to the edge of the
canvas or of the screen. The owner accepted that it is for the view drags
only, not for object moves, and not for pen or touch input.

## Decision

- During a pan (a drag with the middle or the right button), if the
  pointer comes within 3 points of an edge of the canvas,
  `canvas::pan_wrap` gives a point on the other side, a little inside, and
  the canvas sends `ViewportCommand::CursorPosition`. At a corner both
  coordinates change.
- The jump of the pointer comes as a normal pointer movement, in a later
  frame. `canvas::take_jump` takes it out of the pan movement in the frame
  where it comes (the movement is near the jump), so the view does not
  jump back. A jump that does not come in 10 frames is forgotten (for
  example if the system did not move the pointer).
- The zoom tool draws a box, so it does not need the wrap. The other drags
  of the canvas (select, move, draw) do not wrap.
- Touch input does not wrap. `Settings::endless_pan` (Options > Pan, on by
  default) turns it off, for example for a pen that sends mouse input.

## Consequences

- On a remote desktop or a system that does not let a program move the
  pointer, the pan stops at the edge as before.
