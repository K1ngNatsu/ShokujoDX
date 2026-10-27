# ADR 0035: Design library

Date: 2026-10-23 · Status: Accepted (P8 Milestone 1)

## Context

P8 adds a design library: the user's folders of designs, with
thumbnails, search, sort, group and filters, a catalog and a design list
(roadmap P8, `docs/design/p8-plan.md`). The exit test asks for a search
in a library of 1000 designs in less than a second.

To get the size and the number of stitches of a project file, the
program must make its stitch plan, which can take a long time for a large
design. So the numbers cannot come from the files at each search.

## Decision

- **A new engine crate, `emb-library`.** It finds the design files in
  the library folders, reads them (`emb_model::project`, `emb_formats`),
  makes the stitch plan of each project (`emb_stitch`), and makes a
  thumbnail (`emb_render::raster`). It has no UI dependency, so the app
  and `emb-cli` use the same code. It reads the file system (the folders,
  the file sizes and dates); the other engine crates take bytes.
- **An index with an entry for each design.** The entry keeps the
  numbers that the search, sort and group use: name, file, folder,
  format, file size and date, width, height, stitches, colors, trims,
  the threads, the machine and the notes, and the name of its thumbnail.
  The index is a JSON file; the thumbnails are PNG files in a folder next
  to it (in the settings folder in the app).
- **A scan reads only what changed.** An entry is kept if the file has
  the same size and date. New and changed files are read again; entries
  of files that are gone are removed with their thumbnails. A file that
  cannot be read gets an entry with the error, so the user sees it.
- **Search in memory.** The search runs on the index only: words (each
  word must be in the name, file, folder, notes, thread names, machine or
  format) and filters (`format:`, `folder:`, `name:`, ranges for colors,
  stitches, trims, width and height, and `fits:` a hoop). Sort and group
  are on the found entries. The search is a linear pass over the entries;
  1000 entries take a few milliseconds, so no search index is needed.
- **Paths are as the file system gives them.** Folders in the index use
  `/` between their parts, relative to the library folder, so the
  grouping and the CSV are the same on each system.

## Consequences

- The first scan of a large library takes time (one stitch plan for each
  project file); the app runs it in the background (Milestone 2), and
  later scans are fast.
- Thumbnails and numbers of a project come from its stitch plan with the
  machine of the project, as Output design makes them.
- A change of the stitch engine does not change the index until the file
  changes. The index has a version number; a new version makes the next
  scan read all files again.
