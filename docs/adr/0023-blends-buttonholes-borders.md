# ADR 0023: Blends, buttonholes and decorative borders

Date: 2026-10-23 · Status: Accepted

## Context

P5 Milestone 7 adds color blends (two colors, and multi blends with more
layers), buttonholes (one, or a string along a line) and a library of
decorative borders. Each is made of stitches that the engine can already
make: fills and satin columns with accordion spacing (Milestone 6),
satin columns and runs, and lines with a column type or a motif.

## Decision

- **Decoration is made of plain objects.** A blend layer is a fill or a
  satin column with an accordion profile; a buttonhole is a running box,
  two satin legs and two bar tacks; a border is one or two lines with a
  column type, a run or a motif. The user can change each object as any
  other object. The generators are in `emb_model::decor` (no UI, no
  stitches).
- **A multi blend is a group with a record** (`Design::blends`,
  `BlendRecord`), as a monogram is (ADR 0019). The Blends window shows
  the layers of the group: color, density at the start and at the end,
  order, add and remove.
- **The density of a blend layer changes evenly.** The accordion factor
  is 100 / density, so a blend profile has 21 points; a straight line of
  spacing would make the middle of a two-color blend too open.
- **Accordion profiles go from the start of an object as it is drawn**,
  also when a column is sewn from its end, so the layers of a blend match.
- **Buttonholes leave the opening free**: the gap between the legs is
  for the cut, and the worksheet message says to cut it after sewing.
- The built-in borders are our own designs: twelve frames (satin, raised
  satin, double line, triple run, blanket, rope, scallops, hearts,
  leaves, stars) on four shapes.

## Consequences

- A blend, buttonhole or border opens in any version that reads the
  objects; only the Blends window needs the record.
- Changing the shape of a blend means changing each layer (a later
  milestone can link them).
