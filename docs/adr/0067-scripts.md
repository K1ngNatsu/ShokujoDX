# ADR 0067: Scripts

Date: 2026-10-27 · Status: Accepted (owner request, P13 Milestone 4)

## Context

A shop does the same small jobs again and again: an outline round each
fill, the same density for all satins, threads named in the shop's way, a
check before output. The owner asked for tools that users write
themselves, in the program and in `emb-cli`. A script from another person
must not harm the computer (P13 plan, "Not in this phase").

## Decision

1. **Language: Rhai** (the `rhai` crate, MIT or Apache-2.0). It is pure
   Rust (no C or C++ through FFI, rule 9), it is made to be embedded, and
   a script can do only what the program gives it: Rhai has no file,
   network or process functions. Its syntax is near Rust and JavaScript,
   and it has limits for operations, call depth, string, array and map
   sizes. WebAssembly through a pure-Rust interpreter would let users
   write in more languages, but each tool would need a compiler and a
   build step, which is too much for a shop user.
2. **A new engine crate `emb-script`** (depends on `emb-geom`, `emb-model`
   and `emb-stitch`, no UI). `run(script, design, selection, inputs)`
   runs a script on a copy of the design and returns the changed design,
   the messages and the selection, or an error with the line. The program
   puts the result in the document with one `Document::edit`, so one
   undo takes back a script. With an error the design does not change.
3. **What a script can do**: read the objects, their kind, name, thread,
   size and settings (the stitch settings of the kind as a map, through
   serde, so each setting of the program is there with no extra code);
   change names, threads, settings, the sew order; move, turn, scale and
   remove objects; add threads; run engine commands (offsets and borders,
   shaping); read the numbers of the stitch plan (stitches, colors,
   trims, size); print messages. A script cannot open files, use the
   network, start programs or see the settings of the user.
4. **Limits**: 5 seconds and 50 million operations, call depth 64,
   strings of 1 MB, arrays of 100,000 items, maps of 10,000 items, and at
   most 20,000 objects in the design after the script. A script over a
   limit stops with a message, and the design does not change.
5. **Inputs**: the first lines of a script that start with `//!` are its
   title, its description and its inputs
   (`//! input width: 2.0 "Border width (mm)"`; a number, `true` or
   `false`, or a text in quotes). The program shows a form for the inputs
   and gives the values to the script as the map `input`.
6. **Where scripts are**: the sample scripts are part of the program
   (`data/scripts`, built in); the user's scripts are `.rhai` files in the
   `scripts` folder of the settings folder. The Scripts window (Edit tab)
   lists both, shows the description and the inputs, runs the script on
   the selection, and shows the messages. `emb-cli script` runs a script
   on project files.

## Consequences

- A new dependency (`rhai` and its pure-Rust dependencies); the list of
  third-party licenses changes.
- Scripts see the model through the functions of `emb-script`; a change of
  the model can need a change of these functions. The settings maps follow
  the serde names of the project file, which already must stay stable.
- A script can make a design that sews badly, as a user can; Design info
  checks the result as for any design.
