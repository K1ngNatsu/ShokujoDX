# ADR 0110: Pictures of the windows, made by a test

Date: 2026-11-18 · Status: Accepted

## Context

No test showed how a window of the program looks. Each phase ended with
"the windows were not looked at", and nobody looks at the windows in
German, Spanish and Japanese, where the texts are longer (the review of
2026-11-18). The owner wants a check for development only: no code of it
in the program that users get (Phase P17 Milestone 2,
`docs/design/p17-plan.md`).

## Decision

- **A test draws the whole program with no screen**
  (`emb-app/src/pictures.rs`, test code only). `EmbApp::draw` is one frame
  of the program (all of `EmbApp::ui` but the steps that need the window
  of the system), so a test draws exactly what the user sees: the ribbon,
  the docked panels and all windows.
- **A small software renderer** (`pictures::Painter`) makes the picture
  from the triangles of egui: the color of a corner times the color of
  the texture, blended over the picture, with the fill rule that paints
  each pixel of a shared edge one time. So the test needs no graphics
  card and no new dependency (`Cargo.lock` does not change), and it runs
  the same on each system.
- **Scenes.** A scene is a name and a function that puts the program in a
  state. The scenes are: each ribbon tab (also in simple mode and sketch
  mode), each docked panel, Object Properties for each object of the
  sample design, the start page, the lettering kiosk, some windows by
  hand (the first start, the update windows), and each command of the
  ribbon that does not open a file dialog: if the command opens a window,
  the window gets a picture. A new window with a command is in the
  pictures with no more work; a window that needs a special state needs
  a scene.
- **The run** (`write_window_pictures`, started by hand in a release
  build) draws each scene in English, German, Spanish and Japanese: the
  screens at 1366 × 768 and 1920 × 1080, the windows at 1366 × 768. It
  writes the pictures, `index.html` (each scene in the four languages
  next to each other) and `report.txt` to `target/window-pictures`. The
  pictures are not in the repository.
- **What the run finds with no person:** a scene that gives an error (the
  test fails), a window that does not fit the small screen, a window
  that is much wider than in English, and texts that do not fit (a text
  that the program made shorter, or a text whose right end is outside
  its area), with those that only a translation has in a list of their
  own.
- **The test does not compare pictures.** A person (or the assistant)
  looks at them before a release. Stored pictures would change with each
  change of a text and with each font of a system.

## Consequences

- Small tests that always run keep the renderer and the scenes correct
  (colors, the shared edge, one screen and one window).
- The Japanese pictures use the Japanese font of the computer, as the
  program does; a computer with no such font shows boxes.
- A window that only a click on the canvas, a file or a product opens
  has a scene by hand (the Lettering window, Picture to design, the
  Turntable, Product Photo).
