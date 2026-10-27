# ADR 0043: Installers and release builds for Windows, macOS and Linux

Date: 2026-10-24 · Status: Accepted (P10 Milestone 8)

## Context

P10 Milestone 8 asks for CI builds of each tagged version: a Windows
installer (with file types and a Start menu entry), a macOS app in a disk
image, a Linux AppImage and a `.deb`, and the test package of ADR 0033 as
the Windows portable zip. Signing is a choice of the owner, because it
costs money.

The program has all its data inside (fonts, hoops, machines, fabrics,
thread charts, languages), so each package is the program, an icon, the
licenses, the guide and the sample designs. The program had no icon.

## Decision

1. **One icon.** `tools/make-icon.py` (Pillow) draws a wooden hoop with
   fabric, a satin column and a needle, and writes `data/icon/icon-<n>.png`
   (16 to 1024 pixels; 32 pixels and less have a thicker hoop and no
   needle) and `data/icon/app.ico`. The pictures are in the repository, so
   a build does not need Python. The window gets the 256-pixel picture
   (`icons::window_icon`). The Windows program gets the `.ico` and its
   version data (name, version, license) from `build.rs` with the crate
   `winresource`, which runs the resource compiler of the Windows SDK; a
   computer with no SDK makes a program with no icon and a warning.
2. **Windows: Inno Setup 6** (`packaging/windows/installer.iss`). It is
   free, makes one `.exe`, and has the languages of the program (English,
   German, Spanish, Japanese). The installer installs for the current
   user with no administrator rights, or for all users if the user
   selects it. It adds a Start menu entry, a desktop icon as a choice,
   the `.embproj` file type (the program opens it), and, as a choice
   that is on, "Open with" for 12 machine formats (DST, PES, VP3, JEF and
   more): the program that opens these files now stays. The AppId is the
   same for each version, so a new version updates the old one. The
   uninstaller keeps the settings and the user's files.
   `tools/package-windows.ps1 -Installer` makes the portable zip and the
   installer from the same folder.
3. **macOS: an app in a disk image** (`tools/package-macos.sh`). One
   program for Apple silicon and Intel Macs (`lipo`), macOS 10.15 or
   later (11 on Apple silicon). The bundle id is
   `org.embroidery-digitizer.app` (it can change when the project has its
   own domain; the settings do not use it). The app has an ad hoc
   signature only, because Apple silicon does not start a program with
   no signature. **The app declares no document types**: winit 0.30 does
   not give the program the files that Finder opens (that needs an
   application delegate in Objective-C, which is `unsafe` code, rule 9),
   so a double-click on a design would start the program with no design.
   On macOS a design opens with File > Open, the recent designs, or a
   drop on the window.
4. **Linux: a `.deb` and an AppImage** (`tools/package-linux.sh`), from one
   file tree: `usr/bin/embroidery-digitizer`, the `.desktop` file, the MIME
   types (`application/x-embroidery-project` for `*.embproj`, and
   `application/x-embroidery-stitches` for 10 machine formats with
   extensions that other programs do not use), the icons, AppStream data,
   the samples in `usr/share/embroidery-digitizer/samples` and the
   licenses. The window has the app id `embroidery-digitizer`, so the
   desktop shows its icon. The build runs on Ubuntu 22.04, so the program
   runs on systems with glibc 2.35 or later. The `.deb` names the window
   system libraries as dependencies (the program loads them when the
   window opens). The AppImage uses `appimagetool`.
5. **The program opens files from outside.** `AppState::open_any` opens a
   file from the command line, from the file manager and from a drop on
   the window: artwork goes into the design, a picture becomes the
   backdrop, a design or a machine file opens in a new tab. `--version`
   and `--help` write a line and stop, so the package scripts and CI can
   see that the program starts. The start page finds the samples next to
   the program, in `Contents/Resources` of the macOS app, or in
   `share/embroidery-digitizer` of a Linux package.
6. **CI** (`.github/workflows/release.yml`). A tag `v<version>` (it must be
   the version of `Cargo.toml`) runs the tests and the package scripts on
   Windows, macOS and Linux, installs each package and starts the program
   with `--version`, and makes a **draft** release with the packages and
   their SHA-256 sums. The owner checks the draft and publishes it. "Run
   workflow" makes test packages with no release. A package of a commit
   with no version tag has the commit in its name (ADR 0033).
7. **No signature yet** (owner decision). The guide says how to start an
   unsigned program: Windows SmartScreen (More info, Run anyway) and macOS
   Gatekeeper (Open in the menu of the app, or Privacy & Security, Open
   Anyway). A Windows code signing certificate and an Apple developer ID
   can come later; the scripts then get one signing step each.

## Consequences

- The packages are made only by CI on GitHub. The repository has no
  remote yet: the owner must push it to GitHub (and push a tag) before the
  first release.
- The Windows package script was tried on the computer of the developer
  (the zip). The installer, the macOS and the Linux scripts were not run:
  the computer has no Inno Setup, macOS or Linux. The first CI run tests
  them.
- macOS users cannot open a design with a double-click until winit (or a
  safe crate) gives the program the files of Finder.
- The settings stay in the folder of each system (ADR 0013): an
  uninstall does not remove them.
