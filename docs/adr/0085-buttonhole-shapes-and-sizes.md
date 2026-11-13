# ADR 0085: Buttonhole shapes, standard sizes and changes

Date: 2026-10-29 · Status: Accepted (extends ADR 0023)

## Context

The owner asked for buttonholes that a user can sew on the embroidery
machine with the design, so that the user does not change to a sewing
machine: all the usual buttonhole shapes, a size that scales, and presets
for standard button sizes in inches and in mm. The Buttonholes window of
P5 had one shape (straight legs and bar tacks), each size typed in mm,
and no way to change buttonholes after they were placed.

## Decision

- **The buttonholes are in the engine module `emb_model::buttonhole`**
  (they were in `decor`). A buttonhole stays plain objects (ADR 0023): a
  running box first, then satin parts. The shapes are `ButtonholeShape`:
  square ends, round end, round ends, keyhole, tapered keyhole, stretch
  (for knits) and eyelet. A round end, an eye and an eyelet are satin
  columns along an arc, with short stitches; the taper is a satin with two
  rails that come together; the stretch legs are a satin with open spacing
  (a zigzag). The objects of the square shape are the same as before, so
  the corpus design `buttonholes` does not change.
- **Corded and turned.** `ButtonholeSpec::corded` puts a run along each
  satin part before it (a cord: stronger and raised). `flip` turns a shape
  with two different ends end for end.
- **Standard sizes.** `METRIC_BUTTONS` and `INCH_BUTTONS` are button
  diameters, with the size in lignes (1/40 in) in the label;
  `METRIC_EYELETS` and `INCH_EYELETS` are holes. The opening is the
  diameter of the button plus its thickness (2 mm for a flat button).
  `ButtonholeSpec::scaled` sets the legs, the gap, the bar tacks and the
  eye from the opening (an 18 mm opening keeps the settings of P5), and
  the satin of an eyelet from its hole. The window shows lengths in the
  units of the program and takes typed lengths such as `5/8 in`.
- **A record for changes.** `Design::buttonholes` keeps a
  `ButtonholeRecord` for each click: the group, the spec, and the box of
  each buttonhole. The box is the first object, and its first segment goes
  along the buttonhole, so `placement_of_box` finds the middle and the
  angle of each buttonhole again after any move, turn or mirror (the
  shapes are symmetric across their length, so a mirror is a turn). The
  window loads the spec of the selected buttonholes and makes them again
  in their places, as one undo step, at their place in the sew order.
  There is no hook in `transform_objects`: the box is the record of the
  place.
- **Strings.** A string along a line is spaced by the number (as before)
  or by a distance from the start of the line (`string_every`). One
  buttonhole with no line is upright or level.
- The window is `emb-app/src/buttonholes.rs`; the help topic is
  `docs/help/buttonholes.md`.

## Consequences

- A user can sew shirt, coat and knit buttonholes and eyelets in the hoop.
  The shapes are not tried on a machine yet: the help says to test on a
  scrap of the same fabric first.
- Projects with buttonholes of P5 have no record, so the window can only
  place new ones there. The new fields of the spec have defaults, so old
  files read.
- A buttonhole whose box the user deleted is not made again by a change.
- The corpus design `buttonhole-shapes` (all shapes, and five standard
  sizes) has a new golden snapshot.
