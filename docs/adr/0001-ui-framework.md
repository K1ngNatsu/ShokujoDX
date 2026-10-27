# ADR 0001: UI framework is egui

Date: 2026-10-21 · Status: Accepted

## Context

The spec asked for a prototype in egui and in Tauri, and for a decision in Phase 0.
The main UI risk is the canvas. It must draw 10,000 to 1,000,000 stitches and
update immediately when a parameter changes (UX rule 1). The spec also prefers
one language for all layers.

## Decision

Use egui (`eframe` 0.36 with `egui_dock` 0.21 for the docked panels).
We did not build the Tauri prototype. The comparison would not change the
decision, for the reasons below.

## Reasons

- **Canvas speed.** egui draws from Rust memory. Tauri must send stitch data to
  a web view through IPC, or the canvas must be written twice. Measured on the
  Phase 0 prototype (release build, Windows, CPU mesh build only):

  | Stitches | Mesh build per frame |
  | --- | --- |
  | 10,000 | 0.06 ms |
  | 100,000 | 1.9 ms |
  | 500,000 | 9.3 ms |
  | 1,000,000 | 13.6 ms |

  Typical home designs have 5,000 to 100,000 stitches. To measure again, run
  `cargo test --release -p emb-app -- --ignored --nocapture mesh_build_timing`.
- **One language.** No JavaScript toolchain, no IPC layer, no second build system.
- **Docked panels.** `egui_dock` gives the classic docked layout (canvas, Object
  Properties, Sequence) with resizable splits.

## Costs and risks

- egui styling is less flexible than CSS. The app will look like an egui app
  unless we invest in a custom theme.
- Accessibility (screen readers) is weaker than in a web view. egui has AccessKit
  support; we must test it before a public release.
- egui changes its API often (0.36 renamed `App::update` to `App::ui` and changed
  the panel API). Pin minor versions and upgrade on purpose.
- `eframe`, `winit` and `wgpu` use `unsafe` to talk to the operating system. This is
  the accepted exception to rule 9 for the UI stack. Engine crates do not depend on them;
  CI checks this.

## How to undo this decision

`emb-render` converts stitch lists to colored segments with no UI dependency.
Only `emb-app` uses egui. A change of framework needs a new `emb-app`, not engine changes.

Consider Tauri again if one of these becomes true:

- User tests in Phase 2 show that the egui UI is a main cause of confusion that a theme cannot fix.
- Accessibility needs cannot be met with AccessKit.
