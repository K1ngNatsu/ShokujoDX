# ADR 0059: The name Ariadne

Date: 2026-10-26 · Status: Superseded by ADR 0061 (the name ShokujoDX)

## Context

The program had the working name "Embroidery Digitizer", which is a
description, not a name. The owner chose **Ariadne**: in the old story,
Ariadne's thread leads the way through the labyrinth (and the story is
the one of Daedalus and Icarus, the name of the project folder).

## Decision

1. The shown name is **Ariadne**: the window title, About, the start
   page, the crash report, the PDF producer, the installers, the file type
   names and the help.
2. The technical id is **`ariadne-embroidery`**, because `ariadne` alone
   is the name of other software (for example a well-known Rust crate):
   the Linux program, package, app id and files in `packaging/linux`, the
   share folder, and the macOS ids `org.ariadne-embroidery.app`,
   `.app.thumbnails`, `.project` and `.stitches`.
3. Windows: `Ariadne.exe` in `Programs\Ariadne`, the ProgIDs
   `Ariadne.Project` and `Ariadne.MachineFile`, the packages
   `Ariadne-<version>…`. The installer AppId and the CLSID of the
   thumbnail handler do not change, so a new installer updates an old
   install.
4. The settings folder is `Ariadne` (Linux: `ariadne-embroidery`). The
   folder of the first versions, `emb-digitizer`, moves to it once
   (`workspace::program_folder`); if the move fails, the old folder stays
   in use.
5. The project file stays `.embproj` (owner decision), the MIME types stay
   `application/x-embroidery-*`, and the crates stay `emb-*`. The ADRs and
   plans before this one keep the old name.

## Consequences

An install of a test build before this change keeps its old ProgIDs in
the registry until it is removed; the new installer adds the new ones.
