# ADR 0036: Send to USB and machine connections

Date: 2026-10-23 · Status: Accepted (P8 Milestone 3)

## Context

The P8 exit test asks that a design goes to a USB stick for the Topaz 20
in one action. The roadmap also lists machine connections, with
publicly documented protocols only.

To find the USB sticks, the program must ask the operating system which
drives are removable. The standard library cannot do this. Rule 9 of the
spec permits `unsafe` code only in the UI stack (ADR 0001) and asks for
an ADR for each other exception.

## Decision

- **`sysinfo` finds the removable drives.** The app crate uses `sysinfo`
  (MIT, pure Rust, maintained), with only its `disk` part. It calls the
  operating system (on Windows the volume functions) with `unsafe` code
  inside the crate. Our code stays `#![forbid(unsafe_code)]`, and no engine
  crate depends on it. The list has the removable drives that are not
  read-only, with their names and free space.
- **One action after the first send.** File > Send to USB (also Design >
  Output and the Library tab) writes the design in the default format of
  the machine profile (VP3 for the Topaz 20). The first time, a window
  shows the removable drives and a folder on the stick; the program keeps
  the stick (by its name, else by its drive letter) and the folder for
  the machine (`Settings::send_targets`). After that, Send to USB writes
  at once when that stick is in; else the window opens. Send to USB stick…
  always opens the window.
- **The same checks as Export.** A design with a stitch outside the hoop
  field is not sent. A design with hoopings sends one file for each
  hooping (`name_1.vp3`, …). Library designs (project and machine files)
  are converted to the format of the machine.
- **Each file is read back.** The file is written, flushed to the stick
  (`sync_all`), read again and compared with what was sent. A stick with
  too little space gets no file. A file of the same name is replaced, and
  the message says so. The program does not eject the stick; the message
  tells the user to eject it.
- **No machine connection protocol.** We found no public description of a
  network or cable protocol for the Topaz 20 (Husqvarna Viking connects
  its machines through its own software). Other brands also use their own
  protocols. So P8 adds no machine connection. A protocol with a public
  description can come later as a `send` target of its own.

## Consequences

- The app gets the `sysinfo` dependency (and the `windows` crate on
  Windows); the release program is a little larger.
- Tests never use the real drives of the computer: they give folders that
  act as sticks (`AppState::test_drives`). One test only reads the real
  list.
- The P8 exit test part 2 (the Topaz 20 reads the stick) needs the owner.
