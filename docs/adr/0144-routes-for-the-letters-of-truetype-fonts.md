# ADR 0144: Routes for the letters of TrueType and OpenType fonts

Date: 2026-11-20 · Status: Accepted (P20 Milestone 4,
`docs/design/p20-plan.md`)

## Context

The embroidery fonts of the program have the route of each letter (ADR
0113): the order and the directions of its parts with the fewest trims,
and travel runs under the parts that sew later. The letters that lettering
makes from a TrueType or OpenType font had no route: the stitch engine put
their columns in order, and it can hide only a straight travel. In the
test font of the lettering bench, 刺繍東京 at 20 mm had 48 trims. A kanji
has many columns, more so since the crossings are cut (ADR 0143).

## Decision

1. **The route of a glyph** (`Font::route`, `font::Shared::make_route`):
   the satin columns of the glyph, scaled to the capital height of the
   embroidery fonts (25 mm), go through `route::route_with_splits`, and the
   routed parts go back to font units. `LetterSource::parts` of a `Font`
   gives them when the letter sews as columns (the stitch types Satin and
   Auto), and `fixed` is `true` for a letter with its route. The fills of a
   letter have no route. A letter of more glyphs (a ligature, a cluster of
   a script) does not sew in the order of the route of each glyph: the
   layout leaves out its travel runs.
2. **When routes are made** (`RouteMode`). `Wait` (a font is read with it):
   the route of a glyph is made when the glyph is first laid out, so the
   tools and the tests give the same letters each time. `Background`: a
   thread of the font makes the routes of the glyphs that wait
   (`Font::routing`), and a glyph has no route until it is ready;
   `Font::routes_made` counts the routes that are ready. `Off`: no routes.
   The routes and the shapes of the glyphs are kept for the font (one
   `Shared` part that the thread uses too).
3. **The app** reads the fonts of the computer with `Background`
   (`FontLibrary::background_routes`; not in its tests), so typing stays
   quick. Each frame, `AppState::poll_routes` asks the fonts for new routes
   (`FontLibrary::routes_changed`) and lays out again the lettering of the
   design in such a font. The change is a part of the last edit
   (`Document::amend`, new): no new undo step, and an undo goes back to the
   design before that edit, as the edit that typed the letters.
4. **A search for letters with many parts** (`route::Router::beam`). A
   letter with more than 8 parts had a simple route: each time the nearest
   part with no trim sewed next. It used up the parts that would hide the
   runs later. A beam search keeps the 12 best routes of each length (the
   fewest trims, then the shortest travel), one for each set of sewn parts
   and last part. A letter keeps its order only above 64 parts (24 before).
   Letters with up to 8 parts keep the search of all orders, so the letters
   of the embroidery fonts do not change until they are made again.
5. **The bench** measures the areas of the parts before the route (the
   route changes their order, not their areas), and the trims of its words
   with the routes.

## Consequences

- The exit test: 刺繍東京 at 20 mm has 23 trims (48 with no routes): at
  most half. The words of the bench at 15 mm: 刺繍 28 to 17, 東京 19 to 9,
  ありがとう 14 to 6, 龍 16 to 6, 山田太郎 25 to 6.
- A route takes 0.004 s to 0.56 s for one kanji of the test font (鶴, 34
  parts after the route); the app makes it in the background. The tools
  wait for it.
- A new letter in the app shows with no route for a moment, then the
  design changes to the routed letter. The stitch plan is made again then.
- The golden designs with TrueType lettering have routed letters (fewer
  trims); see the commit.
- The bundled embroidery fonts get the beam search when they are made again
  (letters with more than 8 parts, for example in a font with many
  accented capitals).
