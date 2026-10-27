# ADR 0014: Multi-needle machine profiles, and formats only from open references

Date: 2026-10-22 · Status: Accepted

## Context

Users asked for support of multi-needle (industrial) machines: Happy,
Tajima, Toshiba, Toyota, Ricoma, BAI and Highland. Before this change the
app had profiles for the Topaz 20 and a generic DST machine. The industrial
formats were planned as "P3 (common), P8 (rest)" in
`docs/design/feature-inventory.md`. The spec says: do not guess undocumented
header fields; copy the behavior of pyembroidery.

## Decision

- **One profile for each brand** in `data/machines.toml`: Tajima, Happy,
  Toyota, Ricoma, BAI and Highland. A brand makes many models; the formats
  that they read are the same in public format lists and dealer
  documentation. The profiles are marked UNVERIFIED
  until a user confirms them on a machine.
- **Formats only where pyembroidery has the code.**
  - TBF (Tajima): read and write, a port of `TbfWriter` and `TbfReader`.
  - TAP (Happy): read and write. pyembroidery reads TAP as DST records with
    no header; our writer is the DST writer without the header.
  - DSB, DSZ, 10O and 100: read only. pyembroidery has readers but no
    writers, and we do not guess a writer.
- **Default formats**: DST for Tajima, Toyota, Ricoma, BAI and Highland;
  TAP for Happy. Toyota machines use 10O and 100, which we cannot write.
- **TBF needles** follow pyembroidery: needles 1 to 5, then 1 again. The
  operator can set the needles again on the machine.
- **Toshiba** has no profile. No public reference gives a Toshiba
  embroidery format, and the spec does not let us guess one. A Toshiba
  user can use the generic DST profile.
- New hoop presets for multi-needle machines: round 120, 150 and 180,
  square 300 × 300, a full field of 360 × 500 and a wide cap frame of
  360 × 75 (approximate classes, like the other presets).

## Consequences

- The formats of a profile must be ones that `emb-formats` reads, and the
  default format must be one that it writes. A test checks this.
- TBF is the first written format that keeps stops as stops. The import of
  machine files now keeps a trim before a stop, so that the stitches of an
  opened appliqué file are the same when they are stitched again.
- A writer for DSB, DSZ or 10O needs an open reference first (P8).
- The final test for each format is a sew-out on a machine of that brand.
  No machine of these brands is available to the project at this time.
