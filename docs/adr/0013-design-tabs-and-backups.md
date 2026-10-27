# ADR 0013: Design tabs swap per-design state; backups find crashed sessions by a heartbeat

Date: 2026-10-22 · Status: Accepted

## Context

P3 Milestone 8 opens several designs at once (design tabs) and recovers
designs after the app stops by force. Before it, `AppState` held one
document and the state that goes with it (view, selection, stitch cache,
stitch plan, file path) in plain fields, and about a hundred functions use
those fields.

## Decision

- **Tabs swap state.** The active design keeps its state in the `AppState`
  fields. Each other open design waits in a `DocSlot`
  (`emb-app/src/tabs.rs`). A tab change moves the fields of the active
  design into its slot and the fields of the new design out of its slot.
  The code that uses the fields does not change. Tools, settings, catalogs,
  the clipboard and the dialogs of the app are for all tabs.
- **Backups in a session folder with a heartbeat.** Each running app has
  `backups/<process id>-<start time>/` next to the settings file. It writes
  each design with unsaved changes there every few minutes, and
  `session.json` (the open backups and the time) about every 30 seconds. A
  normal exit deletes the folder. At start, a session with a heartbeat
  older than 2 minutes has stopped, and its designs can be recovered.
- A heartbeat needs no platform code (no check of process ids), and two
  apps that run at the same time do not take each other's backups.

## Consequences

- A new per-design field must also go into `DocSlot`, `take_slot` and
  `put_slot`, or it stays with the app instead of the design.
- A computer that sleeps for more than 2 minutes makes a running session
  look stopped to an app that starts in that time. That app then offers
  the backups of the sleeping one. Recover makes a copy in a new tab, and
  the designs in the sleeping app are not changed; it writes a new backup
  of a design at the next change.
- Backups are project files, so a user can also open one by hand.
