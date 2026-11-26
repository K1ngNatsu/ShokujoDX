# ADR 0179: A faster route, part 2: one search for the links of an edge, routes on the disk

Date: 2026-11-26 · Status: Accepted (the owner: "a faster letter route
part 2", the proposal in `docs/design/open-work.md` after ADR 0161).
Changes how ADR 0161 finds a route, not the route that it finds.

## Context

After ADR 0161 the route thread of a font of the computer made the 62
letters and digits of Georgia in 6.7 s, Times New Roman in 4.4 s, Arial
in 0.6 s and Webdings in 20.4 s (measured again for this work). A first
use of a letter in a new session waits for its route, also when the
program made it in an earlier session (a kanji: 0.004 s to 0.56 s).

A measure of Georgia (the time of all the threads of the cuts): half the
time was in the searches for a travel run (984,000 links, 101,000
searches that found a way, 143 million cells from the heap: about 1,400
for each search), 20 % in the check of the groups (ADR 0161), and the
rest in the joins of the edges, the simplification of the runs and the
check of a short straight run.

ADR 0161 named four things for later: a search toward the goal (A*),
search buffers that are used again, work kept between the cuts of a
letter, and routes kept on the disk.

## Decision

1. **One search for the links of an edge** (`route::Search`). The search
   of all orders asks, for the end of one part and one set of parts that
   are not sewn, the link to the start of each next part. These links
   start from the same cells (the joins of the edge), so one search
   serves all of them: the search keeps the cells in the order that it
   takes them from the heap, and each link reads them from its first goal
   cell (before it nothing can change the result) until the rule of the
   search of one link stops it. When a link needs more cells, the search
   goes on. The cells come in the order of a search of each link, so each
   link is the same (`a_shared_search_gives_the_link_of_a_search_of_its_own`).
   The buffers of the search are made one time for each router (a stamp
   says which values are of the search of now), and the goal cells of a
   link have a mark, not a look in a map.
2. **The groups of the start cells of an edge are kept** for the next
   link from the same edge (`Passage::reached`).
3. **Each cell is checked one time.** The joins of an edge check a cell
   from the points of the edge from the nearest, to the first with a line
   under the parts (they checked each cell from each of 9 points); a short
   straight run checks the pairs of points from the shortest. The
   simplification of a run checks the number of cells before the line
   under the parts. A map of the parts that cover each cell
   (`Router::cover`) replaces a look at all the parts.
4. **Routes on the disk** (`RouteStore`, `Font::set_route_store`). The
   app keeps the routes of the fonts of the computer in the folder
   `routes` of the folder of the program: one folder for each font (a
   hash of the font file, the face and the values of its axes), one file
   for each glyph with the shapes of its columns and travel runs (the
   program gives them the settings of a route, so a damaged file cannot
   give others). A route in the store is read at once, also with the
   route thread (`RouteMode::Background`). The store is for one build: the
   version of the program and the size and the time of its program file;
   the store of another build is moved out of the way and deleted on a
   thread. A file that is damaged, or has a point far out of the letter,
   is not used: the route is made again. The tests do not open a store
   (they do not write in the folders of the user).
5. **Not done.** A search toward the goal (A*): tried with the distance
   to the box of the goal cells; it took 14 % fewer cells and gave other
   routes (two ways of the same length change places), so it is not
   used. Work kept between the cuts of a letter: the two halves of a cut
   column change the passable cells near the cut, so few links of a cut
   letter are the links of the letter before; with the search of now, the
   cuts are a small part of the time.

## Consequences

- The same routes: the hash of the route of each of the 62 letters and
  digits of Georgia, Times New Roman, Arial and Webdings is the same, and
  no snapshot changed. The fonts of the program and of the packages made
  again are the files that the code before this change makes. (They are
  not the files in the repository: since ADR 0163, which changed the
  stitches that the converter measures when it chooses a split, the fonts
  were not made again; see `docs/design/open-work.md`.)
- The route thread of a font of the computer, the 62 letters and digits
  (release, one after the other on the same computer):

  | Font | Before | Now |
  | --- | --- | --- |
  | Georgia | 6.61 s | 2.40 s |
  | Times New Roman | 4.38 s | 1.73 s |
  | Arial | 0.63 s | 0.47 s |
  | Webdings | 20.3 s | 11.8 s |

  Most of the time of Webdings is now the tuning of the ends of its
  columns (`tune::round_parts`, `tune::extend_ends`: 7.4 s), not the
  route. All the fonts of the program and of the packages: 2 min 13 s.
- With the store, the second session: the 62 letters and digits of Georgia
  in 0.005 s (2.4 s the first time); about 18 kB on the disk for each
  letter.
- Tests: `a_shared_search_gives_the_link_of_a_search_of_its_own`, the
  groups test with the kept groups, `routes_are_kept_on_the_disk` (a
  second font reads the route, a changed file is read, a damaged file and
  a far point are not), `a_store_of_another_build_is_not_used`.
