# ADR 0037: Speed functions and more machine formats

Date: 2026-10-24 · Status: Accepted (P8 Milestone 6)

## Context

P8 Milestone 6 adds the machine formats that pyembroidery (MIT, commit
`c16d1b4`) reads or writes and the program did not, and the machine
functions of the industrial formats where a format has them. Of these
functions, only the speed (slow and fast) is in a format that the
program writes: Barudan U01 (and the Barudan DAT that it reads).
Penetrations, borers and empty stitches are in no format with a public
description.

## Decision

- **Speed changes are stitch records.** `StitchKind` has `Slow` and
  `Fast`: no move and no needle, at the needle position. An object can
  have the function "Sew slowly" (`Functions::slow`); the stitch plan
  puts `Slow` before the first stitch of such an object (after the jump
  to it) and `Fast` before the first stitch of the next object that is
  not slow. The lock stitches of an object have its speed.
- **Only U01 writes them.** The shared encoder of `emb-formats` leaves the
  speed records out unless a writer asks for them (`encode_all`), so the
  bytes of the other formats do not change. The renderer, the design info
  and the worksheet do not count them as stitches. Opening a U01 or DAT
  file makes an object for each run between speed changes, with "Sew
  slowly" on the slow ones.
- **New formats.** Read and write: PEC (the machine part of PES) and U01.
  Read: DAT (Barudan or Sunstar), EXY, FXY, GT, STX (the DST, DSZ and EXP
  records after a header), PHB and PHC (the PEC records inside), BRO,
  EMD, INB, JPX, KSM, MAX, MIT, NEW, PCD, PCQ, PCS, PCM, SPX, STC, ZHS and
  ZXY. Thread color files COL, EDR and INF are read and written
  (`emb_formats::colors`).
- **Color files next to a machine file.** A machine file with no colors
  (DST, EXP, …) gets the colors of a color file of the same name next to
  it when it opens, in the library and in batch output
  (`emb_library::read_machine_file`). Output design can write the color
  files (an option of the output profile).
- **Checked against pyembroidery.** The U01 and PEC files of the 38
  corpus designs have the same bytes as pyembroidery writes; each new
  reader gives the same stitches, commands and colors as pyembroidery for
  a sample file; a U01 file with speed changes that pyembroidery writes
  reads and writes again with the same bytes.
- **Left out.** PMV (a stitch pattern for Brother sewing machines, not an
  embroidery design), ART (pyembroidery has no reader), VIP and CSD (no
  public description found).

## Consequences

- No golden snapshot changed: no corpus design has a slow object.
- A Barudan machine profile (default U01) is in `data/machines.toml`; its
  values are not checked on a machine.
- Each new `StitchKind` must be handled in every match on it; the
  compiler finds the places.
