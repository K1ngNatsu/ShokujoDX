# ADR 0019: Monograms

Date: 2026-10-22 · Status: Accepted

## Context

P4 Milestone 5 adds monograms: one to three initials in a style, with a
border and ornaments, made from a template. A monogram usually has more
than one color (the initials, the border), and one design object has one
thread.

## Decision

- **A monogram is a group of normal objects.** `emb_letter::monogram`
  makes a lettering object for the initials, fill or running objects for
  the ornaments and satin or running objects for the border. The app adds
  them as one group, in this sew order, each with the thread of its color.
  Each object but the last trims, so no travel stitch shows between an
  ornament and the border or between two rings.
- **The design keeps the spec.** `Design::monograms` has a record for each
  monogram group: the initials, the spec and the middle. With a monogram
  selected, the Monogram window shows its settings and replaces its
  objects in the same place in the sew order. Copies and ungrouped objects
  are plain objects.
- **Styles use the lettering tools.** Classic makes the middle initial
  larger with a letter edit and kerning; stacked makes two initials small
  and moves them with letter edits; diamond and oval use envelopes (a new
  oval envelope: taller in the middle, shorter at the ends).
- **Our own ornaments and borders.** The ornaments (dot, diamond, star,
  heart, leaf, scroll) are small shapes drawn in code. The borders
  (circle, oval, square, rounded square, diamond, double circle) fit round
  the initials and the ornaments with a space, as satin, run or triple run.
- **Templates are specs with a name**, in JSON: six built-in ones, and the
  user's `.monogram` files in the `monograms` folder next to the settings.

## Consequences

- A monogram can be changed with the window while its group and record
  stay; after Ungroup or Break apart, its objects are normal objects.
- The initials are a normal lettering object: double-click edits the text
  (the record keeps the old initials until the window changes it).
- The ornaments are simple; P5 (motifs) can add more.
