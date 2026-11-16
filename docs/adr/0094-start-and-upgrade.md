# ADR 0094: The start of the program and the upgrade of the installer

Date: 2026-11-16 · Status: Accepted

## Context

After v0.2.0, friends of the owner installed the program on a PC with
Windows 11 24H2 and Intel UHD 630 graphics. The program showed nothing:
no window and no message (v0.3 plan, Stage 1). Three causes were
possible, and the program could not tell which:

- The program has no console on Windows. An error of the window or of
  the renderer at the start went nowhere (only a panic wrote a crash
  report).
- The program needed the Visual C++ runtime (`VCRUNTIME140.dll`), and
  the installer did not install it. The CI runners have it, so the tests
  did not see this.
- A PC with no graphics driver of its maker, or with an old one, can
  have no adapter that the renderer accepts.

The owner also asked that the installer remove older versions, and that
it check for the runtimes and drivers.

## Decision

- **Static C runtime.** `.cargo/config.toml` builds the Windows target
  with `+crt-static`. The program and the thumbnail DLL need no Visual C++
  runtime. `tools/package-windows.ps1` stops if a file of the package
  imports it.
- **Renderer fallbacks** (`emb-app/src/startup.rs`). The program tries
  `Renderer::ORDER`: the graphics card (`auto`: DirectX 12, Vulkan or
  OpenGL, as wgpu selects), then OpenGL only (`gl`), then a software
  adapter (`software`, WARP on Windows). If `eframe::run_native` returns
  an error, or a panic occurs before the second frame, the program starts
  a new process with the next renderer (`--renderer=…`,
  `--failed-renderer=…` and the reason in `SHOKUJODX_START_ERROR`) and
  stops. A process cannot make a second winit event loop, so the
  fallback is a new process. `startup::args` gives the app its arguments
  without these.
- **Kept renderer.** A fallback renderer that drew a frame is kept in
  `renderer.txt` (user folder) with the list of the graphics adapters.
  The next start uses it at once. If the list of adapters changes (a new
  driver), the program tries the graphics card again.
- **Messages.** If no renderer works, a message box (rfd) tells the
  reason, what to do and where the log is. A start on OpenGL or on a
  software adapter (also when wgpu selects a software adapter itself)
  shows a notice at the start with the advice to install the graphics
  driver.
- **Start log and diagnosis.** Each start writes `start.log` (the
  renderer, the adapter in use, a failure and the fallback).
  `--diagnose` writes `diagnose.txt`: all adapters, a test of each
  renderer with no window, the kept renderer and the last start log.
- **The runtime and driver check is in the program, not in the
  installer.** With the static runtime the only outside need is a graphics
  driver, and an installer cannot test a driver as well as the program
  can (the program makes the device that it uses).
- **Upgrade.** The `[Code]` of `installer.iss` finds the installs of
  the program for this user (HKCU) and for all users (HKLM), shows them on
  the Ready page, and removes them with their uninstaller
  (`/VERYSILENT`) before the new files go in. It waits until the uninstall
  entry goes (the uninstaller continues in a copy of itself). The settings
  (`%APPDATA%\ShokujoDX`) and the designs are not in the program folder,
  so they stay. The release workflow installs the last published version
  for all users, then the new one for this user, and checks that the old
  one is gone and that a settings file stays.

## Consequences

- A start on a PC with a bad driver takes longer (each failed renderer is
  one start of a process), but only once: the next start uses the kept
  renderer.
- The software renderer is slow on large designs. The notice tells the
  user to install the driver.
- `start.log` and `diagnose.txt` have no design data.
- The upgrade test needs a published release with a Windows installer.
