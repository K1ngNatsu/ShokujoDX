# ADR 0009: User settings in a JSON file, separate from designs

Date: 2026-10-21 · Status: Accepted

## Context

Roadmap P2 adds settings that belong to the user, not to a design: units,
grid spacing, snap, nudge steps, and the default parameters of new objects
(the property bar). They must survive a restart. Guides, in contrast, belong
to a design.

## Decision

- `emb_app::workspace::Settings` is saved as pretty JSON in
  `emb-digitizer/settings.json` in the user's configuration folder:
  `%APPDATA%` on Windows, `~/Library/Application Support` on macOS,
  `$XDG_CONFIG_HOME` or `~/.config` elsewhere. No extra crate is needed.
- All fields have defaults (`#[serde(default)]`), so an old or partial file
  loads, and a bad file gives the defaults.
- The app writes the file when a setting changes. Tests never write it
  (`AppState::save_settings` is false unless the app starts normally).
- Settings are not in the undo history. Guides are in `Design::guides`, so
  they are saved with the project and undo works on them.

## Consequences

- Two users of one computer have their own settings; a project looks the
  same for both, except for units and grid.
- Settings pages in P3 (thread charts, fabrics, styles) can add fields to
  the same file.
