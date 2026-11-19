# ADR 0118: Content families

Date: 2026-11-19 · Status: Accepted

## Context

Phase P17 Milestone 8 (`docs/design/p17-plan.md`) wants libraries of
motifs and patterns that are large enough for daily work: 60 motifs, 40
program-split patterns, 30 carving stamps, 40 decorative borders and 40
clipart items, all our own. The program had 15, 10, 8, 12 and 8. Each
item was written by hand in the code: a list of points or of rows.

Many items are the same idea with other numbers: a wave with one or two
waves, a star with five or six points, a tile of small or large diamonds.

## Decision

- **A family** (`emb_model::family`) is a rule that makes an item from
  some numbers: a name, parameters (each with a range, a default value,
  and a mark if it is a count) and a function. There are families of
  four kinds: motifs (25 families), program-split patterns (18), carving
  stamps (15) and clipart items (19).
- **The description of an item** is a text: the name of the family and
  the values of its parameters, for example `Waves: 2, 0.3`
  (`Family::description`, `family::from_description`). The same
  description gives the same item. `Motif::builtin` takes a description
  too, so a border or a test can name a motif of a family with its own
  numbers.
- **The built-in items** of a family are descriptions with a name (the
  presets of the family). `motif::builtin`, `split::split_patterns` and
  `split::stamps` give the items that were written by hand first (their
  names and their points do not change) and then the items of the
  families. The clipart items of the families are written to the clipart
  folder of the user in a folder of their own (`clipart::FAMILY_FOLDER`),
  so a user who has the first built-in items gets them too.
- **The item is kept in the object** that uses it (the points of a motif,
  the rows of a pattern, the lines of a stamp), as before. A design opens
  with no family, and a later change of a family does not change a
  design.
- **Split patterns from shapes.** A tile can have the outline of a closed
  shape (a circle, a polygon, a star, a heart, a flower): each row has
  needle points where it crosses the outline. A row that crosses nothing
  gets one point at a place that is different in each row. No stitch of
  a row is longer than 6 mm: a longer one gets points between.
- **Borders** (`emb_model::decor`). A border is a shape and a stitch, so
  it is a description with parameters already. There are three more
  shapes (diamond, hexagon, octagon), and `Frame::with` gives a border
  another shape and another size of its stitch. The Borders window has
  these two parameters. 38 more borders use the new shapes and the motifs
  of the families.
- **The Families window** (`emb-app/src/families.rs`): the kind, the
  family, its built-in items, a slider for each parameter (a box for a
  yes or no), a picture, and the description. It uses the item for the
  selection (a motif, a pattern), adds it to the selected fills (a stamp)
  or inserts it (a clipart item), and it keeps a motif or a clipart item
  in the library of the user. The Motifs window, the pattern and stamp
  rows of Object Properties and the Clipart panel open it.
- **No asset package.** The plan said that content that few users need is
  an asset package. The items of the families are made by code when the
  program needs them, so they add no data to the program, and a package
  would save nothing.

## Consequences

- The program has 71 motifs, 48 program-split patterns, 39 carving
  stamps, 50 decorative borders and 45 clipart items.
- The names of the families and of the items are data in English, as the
  names of the motifs, patterns, stamps and borders were. The names of
  the kinds, of the parameters and of the border shapes are translated.
- A motif or a pattern from a description has the description as its
  name, so the object shows where it comes from.
- The corpus design `content-families` sews motifs, patterns, stamps,
  borders and clipart of the families. A contact sheet of all items was
  looked at when the families were made; the script that draws it is not
  in the repository.
- A pattern or a stamp of the user has no library: the object keeps it,
  and the description makes it again.
- No sew-out confirms the new items (the exit test of the owner: five new
  items of each kind).
