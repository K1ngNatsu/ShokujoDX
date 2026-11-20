# ADR 0139: A design stays in the range of a project file

Date: 2026-11-20 · Status: Accepted (the audit of the whole project,
2026-11-20; adds to ADR 0004 and ADR 0005)

## Context

A project file does not open if a part of its design is more than 1 km
from the origin, or if a number is not a real number
(`project::MAX_COORD_MM`, the audit of 2026-10-25): such a value comes
from a damaged file, and the stitches to go there use all the memory.

The check was only in the loader. The program could make such a design
and write it:

- A script with `move_objects(objects(), 2000000.0, 0.0)` moved all
  objects 2 km. The program saved the design, and the file did not open
  again ("object 1 is out of range"). The save put the new file in the
  place of the good file.
- A script that divides 0.0 by 0.0 gave a number that is not a real
  number. The file had `null` at its place and did not open.
- A typed move, a scale or a typed length can do the same.
- A machine file can have a stitch that is km from its start (165,000
  jumps of a DST file are 2 km). The stitch object from it has the same
  problem.
- The writers of machine files split a long move into the largest moves
  of the format. A move of 10^9 mm is 80 million records of a DST file.

So a rule of the loader was not a rule of the model: the model permitted
a state that the loader refused.

## Decision

1. **One check.** `emb_model::project::in_range` says if a project file
   can keep a point, and `project::out_of_range` gives the first part of
   a design that is out of range (the hoop center, an object, the product
   or a place on it). The loader, the document and the readers of artwork
   (`svg::drop_far`) use them.
2. **The document does not make a change that puts its design out of
   range.** `Document::edit`, `edit_merged` and `try_edit` are the only
   ways to change a design (ADR 0004). After a change, the document
   checks the objects that the change touched (an object that did not
   change has the same `Arc`). If a part is out of range, the design of
   before comes back, the history gets no step (the redo steps stay), and
   `Document::take_refused` gives the reason one time. The app shows it
   in a notice ("The change was not made").
3. **A file that the program cannot open again is not written.**
   `project::save` reads the text of the file again before it writes it
   (`project::checked_string`). If the loader refuses it, the result is
   `ModelError::CannotKeep`, and the file that is there stays. The design
   that is sent by email uses the same check.
4. **A script that makes such a design gets an error** with the reason
   (`emb_script::run`), as for each other error of a script: the design
   does not change.
5. **A machine file with a stitch more than 1 km from the center of its
   design gives an error** (`emb_formats::read_machine`).

## Consequences

- A design in a document is always a design that the program can save
  and open again. A machine file from it has moves of at most 2 km: a DST
  file of 0.5 MB.
- A new part of a design that has coordinates (not an object, the hoop
  or the product) needs its check in `project::out_of_range`.
- A new rule of the loader (`project::validate`) is a rule of `save` with
  no more work. If it is about a value that a change in the program can
  set, it also needs its place in `out_of_range`, so that the user gets
  the message at the change and not at the save.
- A change that is not made returns the result of its closure: an id of
  an object that the closure added is not in the design. The app does
  not use `unwrap` on `Design::object`.
- The check after each change is small: one compare of a pointer for
  each object, and the control box of each changed object.
- A save reads its text again (about the time of one open). A backup
  does the same.
- Rhai has its own math functions (`sin`, `exp`), which use the math
  library of the system (not ADR 0003). The result of a script is in the
  design after the script, so the design is the same on each system
  after that.
