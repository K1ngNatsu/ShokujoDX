# ADR 0161: A faster route: maps of the passable cells, cuts on threads

Date: 2026-11-24 · Status: Accepted (the owner's choice after the audit of
P21, "Audit of P21" in `docs/design/roadmap.md`). Changes how ADR 0144
and ADR 0153 find a route, not the route that they find.

## Context

The route of a letter (ADR 0113, 0144, 0153) finds the order and the
directions of its parts with the fewest trims, then the fewest columns
on top. For a letter of up to 8 parts it tries all orders, and it does
that again for each place where a column can be cut. For each step from
one part to the next it asks if a travel run can go there under the
parts that are not sewn: a search through a grid of cells.

In the audit of P21 the letters of serif fonts of the computer took 2 s
to 7 s each in the route thread of their font (Georgia: 32.8 s for the
62 letters and digits; Arial: 0.6 s), so lettering in such a font got
its routes some seconds after it showed. A measure of Georgia "H"
(3.6 s): 120,000 steps asked for a run, 31,000 searches of a grid of
11,000 cells, 93 % of the time. 72 % of the searches found no way, but
only after they went through all the cells that they could reach, and
each cell was checked over all the parts of the letter again (183
million checks).

## Decision

1. **A map of the passable cells for each set of parts that are not
   sewn** (`route::Passage`, `Router::passage`). A cell is checked one
   time for each set, when a search first asks for it, not one time for
   each search. The search of all orders asks for one set at a time, so
   each router keeps one map: a number for each set and a stamp in each
   cell say which cells are for the set of now, so a new set needs no
   new memory. (A new map for each set made the converter, which routes
   16 letters at the same time, two times slower.)
2. **Groups of the passable cells.** A flood fill gives each passable
   cell that a search can reach from a start the group of that start
   (`Router::group`). A link whose goals are in no group of its starts
   is a trim at once, with no search (`Router::reaches`). A start can be
   a cell that is not passable (a search goes from it to its passable
   neighbours), and a goal that is a start is reached: the rule is the
   rule of the search, so each link is the same.
3. **The cuts of a letter on threads** (`route::route_with_splits_on`,
   `route::threads`: the processors, at most 8). The routes of the cut
   letters do not depend on each other; they are made on threads, and the
   choice goes through them in the order of before, so the route is the
   same with each number of threads. A cut whose route stops with a
   fault is not used. The route thread of a font of the computer
   (`Font::route`) and the letter fault bench use the threads.
4. **The converter takes the letters from a queue** (`EmbFont::route_until`):
   each thread takes the next letter that no thread has taken, and the
   cuts of a letter stay on its thread. With a group of letters for each
   thread, the threads with slow letters ("M", "W" of a serif font) worked
   long after the others were done.

## Consequences

- The same routes: a hash of the route of each of the 62 letters and
  digits of Georgia, Times New Roman, Arial and Webdings is the same
  (`probe_p21_font_times` in `crates/emb-cli/tests/p21_probe.rs`), the
  fonts made again are the same files, and no snapshot changed.
- The route thread of a font of the computer, the 62 letters and digits:
  Georgia 32.8 s to 7.6 s, Times New Roman 18.1 s to 4.6 s, Webdings
  50.9 s to 19.8 s, Arial 0.61 s to 0.40 s. The converter: Friendly Serif
  35.4 s to 9.2 s, Geo Slab 37.4 s to 13.8 s; all the fonts of the program
  and of the packages are made again in 4.5 minutes.
- Memory: the six letters "HKWMNX" of Georgia at the most 39 MB (12 MB
  before): the threads of the cuts, each with its router and one map.
- Tests: the groups give the links of the search with no groups, in
  letters of a font with their outline; a start that is not passable
  reaches through its neighbours; the cuts on threads give the route of
  one thread.
- Not done: a search toward the goal (A*) and reused buffers for the
  searches that find a way (two ways of the same length can change
  places, so a run could move); keeping work between the cuts of a
  letter; keeping the routes of a font on the disk.
