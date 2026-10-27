# ADR 0004: Undo and redo with design snapshots

Date: 2026-10-21 · Status: Accepted

## Context

Full undo and redo is a requirement (UX rule 5). It must be part of the model
from the start, because it is very hard to add to an existing app.

There are two common methods:

- **Command pattern:** each operation has a do and an undo function. Each new
  operation needs correct inverse code, and a bug in an inverse corrupts the design.
- **Snapshots:** keep a copy of the design before each operation. Simple and always
  correct, but copies can use much memory.

## Decision

Use snapshots with structural sharing:

- `Design` stores objects as `Vec<Arc<DesignObject>>`. A clone of `Design` copies
  only pointers.
- `Design::object_mut` uses `Arc::make_mut`. It clones an object only if a
  snapshot shares it (copy on write), and gives the object a new revision.
- `Document` owns the design and the history. `Document::edit(label, |design| ...)`
  is the only way to change the design. `try_edit` discards a change that fails.
- `Document::edit_merged` joins continuous edits with the same `MergeKey` (a slider
  drag, a node drag) into one undo step. `end_merge` stops the merge.
- Default limit: 200 steps. View state (zoom, pan, selection) is not in the history.
- `Document::is_modified` compares the current state id with the saved state id,
  so undo back to the saved state clears the "modified" mark.

## Stitch cache

The stitch cache is not in the history. Its key is `(ObjectId, revision)`.
Revisions come from one process-wide counter, so they are unique across all
undo branches. After an undo, the old revision comes back and the cache returns
the old stitches with no regeneration. Revisions are not saved in project files.

## Consequences

- Memory per undo step is the size of the changed objects, not the full design.
- Each new operation is undoable with no extra code.
- Code must not keep `&mut Design` outside `Document::edit`. The `objects` field is
  private, so code cannot change an object without a new revision.
