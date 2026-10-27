# ADR 0006: Native file dialogs (rfd) and SVG import (usvg)

Date: 2026-10-21 · Status: Accepted

## Context

The app needs Open, Save As and Import dialogs, and it needs to read SVG
artwork. The spec puts SVG import in Phase 1a; the owner asked for it earlier,
together with file handling and draw tools, so the app can be tested end to end.

## Decision

- **File dialogs:** `rfd` 0.17 in `emb-app` only. It shows the system dialogs
  (Windows common dialogs, macOS panels, XDG portal on Linux). The dialogs are
  modal and block the UI thread while open, which is normal for these dialogs.
- **SVG:** `usvg` 0.48 in `emb-formats`, with default features off (no text, no
  system fonts, no compressed SVGZ). It is pure Rust and resolves CSS, transforms,
  `use` elements and units, so `emb_formats::svg` only maps paths to objects.

## Notes on rule 9 (no `unsafe`, no FFI in engine crates)

- `rfd` calls operating-system APIs, so it uses `unsafe` internally. It is a UI
  dependency like `eframe` (see ADR 0001), and only `emb-app` uses it.
- `usvg` is pure Rust. `emb-formats` stays free of C and C++ dependencies.

## Consequences

- SVG text is not imported (a warning says so). Users convert text to paths in
  their vector editor first. Lettering is a Phase 3 feature.
- On Linux, file dialogs need an XDG desktop portal (normal on current desktops).
  Check this when Linux CI and packaging start.
