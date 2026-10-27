# ADR 0057: Command windows dock at the canvas corners

Date: 2026-10-26 · Status: Accepted (owner request)

## Context

The command windows (Border trace, Offsets, Lettering and others) opened
where egui put them, often over the middle of the design, and stayed
where the user left them. The owner asked for docking as in the command
windows of CAD programs: the window sits at a corner of the
graphics area, and snaps to a corner when you move it there.

## Decision

1. `emb-app/src/dock.rs`: `docked(window, ctx, id, canvas)` is called on
   the builder of each command window before `show`. A window opens
   docked at the top left corner of the canvas (8 points in). When a drag
   of it ends, `snap` docks it at the nearest corner of the canvas that is
   60 points or nearer, else it is free and stays where it is.
2. A docked window gets its corner position in each frame (pivot at the
   corner), so it follows the canvas when panels open or the program
   window changes size. Windows docked at one corner stack in their draw
   order; a window that did not show in the last frame takes no place.
3. The docks are in the memory of egui (temporary data, by window id), so
   they last while the program runs, not after a restart.
4. Command windows move by a drag of the title or of an empty place
   (`WindowDrag::Anywhere`). In the title-bar mode, egui 0.36.2 puts back
   the position of the last frame after `Area::begin`, so a set position
   has no effect; the widgets in the window still take their own drags.
5. 28 windows dock. Large dialogs (Help, Options, Output design, Print
   preview, Picture to design, Preview hoopings, Send to USB, My assets,
   About, Recover designs, Find a command) and the Overview do not.

## Consequences

A new command window adds `crate::dock::docked` (CLAUDE.md). The dock
does not stay after a restart; that can come later, in the settings.
