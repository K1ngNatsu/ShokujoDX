# ADR 0046: Pictures of design files in the file manager, and the `unsafe` code of the Windows handler

Date: 2026-10-25 · Status: Accepted (P11 Milestone 3)

## Context

P11 Milestone 3 asks for the design picture as the thumbnail of project and
machine files in Explorer, Finder and the Linux file managers. Each system
has its own way:

- Linux (freedesktop): a `.thumbnailer` file names a program that writes a
  PNG.
- macOS: a Quick Look thumbnail extension (an `.appex` in the app). It runs
  in the app sandbox.
- Windows: a COM DLL (a shell thumbnail handler) that Explorer loads. Rust
  cannot check the calls of COM, so the DLL needs `unsafe` code; rule 9 of
  the spec forbids `unsafe` code unless an ADR records the exception. The
  owner asked (2026-10-25) for the most robust way that keeps the
  thumbnails on Windows.

## Decision

1. **One picture function** (`emb_library::preview`): `sniff` finds the kind
   of a file from its first bytes (projects, PES, PEC, VP3, DST, HUS, JEF;
   the other formats have no mark), `stitches_of` reads a project (its
   stitch plan) or a machine file, and `picture` draws it with the 3D thread
   look, at most a given size, on a transparent background. All three
   handlers use it; the damaged-file and wrong-value checks of the readers
   and of the stitch engine (the audit of 2026-10-25) protect it.
2. **Linux:** `embroidery-digitizer --thumbnail <file> <out.png> [<size>]`
   (the program stops before it opens a window), and
   `packaging/linux/embroidery-digitizer.thumbnailer` for the two MIME types
   of the program, in the `.deb` (`usr/share/thumbnailers`). An AppImage
   installs nothing, so it has no thumbnails.
3. **macOS:** `Thumbnails.appex` in `Contents/PlugIns` (Swift,
   `packaging/macos/thumbnails`), for two content types that the app
   declares (`org.embroidery-digitizer.project`, exported, and
   `org.embroidery-digitizer.stitches`, imported, for 12 machine formats).
   The extension cannot start the program (it is not in the sandbox), so a
   small helper, `emb-thumbnail-helper` (Rust, no `unsafe`), reads the bytes
   on standard input and writes the PNG to standard output. It opens no
   file and has the sandbox entitlements with `inherit`. The script signs
   the helper, the extension and the app, ad hoc, from the inside out.
4. **Windows: the crate `emb-thumbnail`**, a DLL, is the one exception to
   rule 9 for code of this project. To keep it robust:
   - The `unsafe` code is in this crate only (the crate does not take the
     lints of the workspace; `unsafe_op_in_unsafe_fn` is an error), and it
     only reads the stream, makes the bitmap and answers COM. Each `unsafe`
     block has a `SAFETY` note.
   - The handler takes the file as a stream (`IInitializeWithStream`):
     Windows then runs it in an isolated process (not in Explorer), so an
     error cannot stop Explorer.
   - Each call from Windows catches a panic and gives an error code; a file
     larger than 64 MB gets no picture.
   - Tests drive the DLL through COM as Explorer does (class factory,
     stream, picture, bitmap), with no registration.
   The installer registers the class for the seven kinds that `sniff`
   knows (a choice that is on), under the current user or all users.
5. The engine crates stay with no `unsafe` code and no C library (rule 9);
   CI checks them as before.

## Consequences

- If another program has a thumbnail handler for PES, DST or another
  machine format, the installer of this program takes its place, and the
  uninstaller removes the key. The choice in the installer lets the user
  keep the other handler.
- Explorer keeps old thumbnails in its cache; a file that had no picture
  can need a new view (or Disk Cleanup > Thumbnails) after the install.
- The macOS extension and the Windows handler were not tried by hand: this
  computer has no Mac, and the Windows registration changes the registry of
  the user. CI installs the Windows package and checks the registration;
  on macOS it checks the signatures and asks Quick Look for a thumbnail
  (a check that does not stop the release).
