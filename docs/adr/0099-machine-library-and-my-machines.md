# ADR 0099: The machine library and the user's own machines

Date: 2026-11-16 · Status: Accepted

## Context

The machine list had nine profiles: the Topaz 20 (the owner's machine), a
Generic machine and seven multi-needle brands. The owner asked for a
machine library of the brands of the users (Brother and Baby Lock,
Janome and Elna, Husqvarna Viking, Pfaff and Singer, Bernina and
commercial multi-needle machines), with generic profiles and an easy way
to add a machine (v0.3 plan, Stage 6). Rule 3 of the spec forbids data
from closed-source embroidery programs; the makers publish the size of
the embroidery field of each machine.

## Decision

- **Library data** (`data/machines.toml`, `data/hoops.toml`). About thirty
  machines of the brands above. For each: the largest embroidery field and
  the needles from the maker's product page or spec sheet (else a dealer),
  with the source in a comment and the date of the research
  (2026-11-16); the file formats; the field as a hoop preset (`field_…`)
  and the standard sizes that fit in it (the hoop list is sizes, not the
  hoops of the brand). Values that no maker page confirms are marked
  UNVERIFIED in the file (for example the Singer formats, the MB-7 field
  and most speeds). The side of a hoop on the machine is not in the
  makers' data: a sewing and embroidery machine has the long side from
  front to back (as the owner's Topaz 20), the MB-7 from left to right.
- **Generic profiles** for each format of home machines: PES, JEF, VP3 and
  EXP, beside the Generic DST machine. Their first hoop (the hoop of new
  designs) is 130 × 180 mm. Machine profiles can now list XXX (the program
  writes it); the Singer Legacy SE300 uses it.
- **My Machines** (`emb-app/src/my_machines.rs`, Design > Machine and Hoop
  > My Machines, and a button in the first-start step). The user sets the
  name, the brand, the formats and the format for new files, the needles,
  the speed, the longest stitch and the hoops. The machines are in
  `machines.toml` next to the settings file; the app adds them to the
  catalog at the start, before the start design takes the user's machine
  (the user's hoops load first too now). An id is `my_` and the name.
  **Export** writes one machine and the user's hoops that it uses
  (`MachineShare`); **Import** adds them, with new ids where an id is in
  use.

## Consequences

- The library is only as good as the public data. A user should check the
  field and the formats in the manual; the help says so.
- A test checks that each brand group and each generic format is in the
  library, that each machine's hoops exist, and that the first hoop of a
  library machine is its largest field.
