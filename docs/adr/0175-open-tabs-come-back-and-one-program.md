# ADR 0175: The open tabs come back, and one program

Date: 2026-11-25 · Status: Accepted (P22 Milestone 8,
`docs/design/p22-plan.md`).

## Context

A quit asked about each design with changes that were not saved, and the
next start began with an empty design. A file that the user opened while
the program ran (a double-click in the file manager, Open with) started a
second program with its own window. Both ideas are from zigzag (an
open-source program under Apache-2.0); the code is our own. The program
has no network code and no sockets (ADR 0109).

## Decision

- **The tabs come back** (`backup.rs`). With `Settings::restore_tabs`
  (Options > Open tabs; on by default), a quit asks nothing: it keeps
  the session folder of the backups with a backup project of each tab
  that has changes that are not saved or no project file, and writes in
  `session.json` the list of the tabs (`Closed`: for each tab its name,
  its file, its backup, if it had changes, and its view; the tab in
  front; the time). Blank tabs are left out. At the next start
  (`AppState::restore_tabs`, not in the kiosk) the tabs open again: a
  tab with a backup from the backup (with its file, so Save writes
  there, and with its changes marked), else from its file; the views
  and the tab in front come back, and the session folder goes. The
  sessions of more quits (two programs) all come back, the oldest first.
  A closed session is not a crashed session: after a crash the program
  asks as before (`find_recoverable` leaves out closed sessions). If the
  tabs cannot be kept, the quit asks as before. With the option off,
  the quit asks as before.
- **One program** (`one_program.rs`). With `Settings::one_program`
  (Options > Files; on by default), a start with a file (not with
  `--new-window` or `--kiosk`) looks for a session of the backups with a
  heartbeat newer than `STALE_S` and no quit (`running_sessions`, the
  newest first), writes the absolute path of the file in the `inbox`
  folder of that session (a `.tmp` file, then a rename to `.path`, so no
  half path is read), and stops before it opens a window. The program
  that runs reads its inbox each second (`POLL`; the frame repaints at
  least so often, also when the window is minimized: eframe 0.36 paints a
  minimized window at the time asked, at most every 100 ms), opens each
  file in a tab (`open_any`), and comes to the front (`Minimized(false)`
  and `Focus`). A start with no file starts a program as before.
  `--new-window` starts a second program also with a file.

## Consequences

- The exit test is met: a test with two processes
  (`crates/emb-app/tests/one_program.rs`: the second process gives its
  file to a session with a new heartbeat and stops in 0.5 s); a quit
  with three tabs, one with changes, gives the three tabs, the changes,
  the views and the same tab in front at the next start
  (`backup::tests::a_quit_keeps_the_tabs_and_the_next_start_opens_them`);
  after a crash the recovery question comes as before (the tests of
  before, and a closed session is not offered).
- A program whose heartbeat is older than two minutes (it hangs) does
  not get files; the new process starts a program of its own.
- On macOS the system sends a file to the program that runs as an event,
  not as a new process; the program does not read such events (as
  before).
- The program repaints each second when it waits (before: each 30 s).
