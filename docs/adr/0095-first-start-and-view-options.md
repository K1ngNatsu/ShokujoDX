# ADR 0095: The first start and the View options that stay

Date: 2026-11-16 · Status: Accepted

## Context

The owner had to switch on Thread Look at each start: the View options
(Stitches, Thread Look, Jumps and the others) were fields of the app
state, not settings. Also each start put the empty design on the Topaz 20
(the reference machine of the owner), not on a machine that suits a new
user (v0.3 plan, Stage 2). The owner chose that the first start asks for
the machine, with the Generic machine selected until the user picks one.

## Decision

- **View options.** `Settings::view` (`viewtools::ViewOptions`) has the
  on/off options of View > Show and the view windows (Overview, Split
  View). The app applies them at the start and calls
  `AppState::keep_view_options` once a frame, which saves the settings
  only when an option changed. The light and the fabric of the 3D look
  were settings before (`Settings::look`). While the stitch player shows,
  Stitches keeps its value from before the player.
- **The user's machine and hoop.** `Settings::machine` and
  `Settings::hoop` (catalog ids). At the start, `AppState::start_on_machine`
  puts the empty design on them (no undo step); with no machine, on the
  Generic machine (`app::GENERIC_MACHINE`) in its first hoop. The Generic
  machine's first hoop is now 130 × 180 mm (5 × 7 in), which most home
  machines have; it was 100 × 100 mm, in which the sample designs did not
  fit. A machine or a hoop that the user picks in Design > Machine and Hoop
  is kept as the user's.
- **First start** (`first_start.rs`). If `Settings::machine` is `None`
  (a new install, or settings of an older version), a window asks "Which
  machine do you have?" with the machine and the hoop, Generic selected.
  **Use This Machine** keeps the choice; **Skip** keeps the Generic
  machine. Both close the step for good. The empty start design changes
  to the choice; a design that the user opened or started does not. The
  lettering kiosk does not ask.
- `AppState::new` (and so each test) starts on the Generic machine. Tests
  about the reference machine call `start_on_machine("husqvarna_topaz20",
  None)`; the test harness does.

## Consequences

- A user who updates from v0.2.0 sees the step once.
- The step lists the machines of the catalog. Stage 6 (the machine
  library) makes the list longer and adds Add My Machine.
