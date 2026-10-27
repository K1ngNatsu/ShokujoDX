# ADR 0048: Free-standing lace

Date: 2026-10-25 · Status: Accepted (P11 Milestone 5)

## Context

P11 Milestone 5 asks for free-standing lace: lace with no fabric, sewn on
water-soluble stabilizer that washes away. The lace must hold together
when the stabilizer is gone. The milestone asks for a mesh fill, a lace
underlay for satin columns and outlines, a check that each part touches
the mesh, a water-soluble stabilizer fabric preset and a template for a
lace ornament. The methods are common lace digitizing practice, in our
own words (other programs have lace features; their code is closed,
spec rule 3).

## Decision

1. **Lace mesh** is a fill type (ADR 0022): `FillPattern::Mesh`, open
   rows (1.6 mm apart at the start) at the fill angle and a second layer
   turned by `angle_deg` (90° at the start). Each row goes there and back
   on the same needle points (`fill::RowSplit::double`), so it is two
   threads that lock where the rows of the two layers cross. It is an
   open type, so it has no underlay.
2. **Lace underlay** is an underlay mode: `Underlay::Lace`. Columns: the
   edge walk two times on the same needle points (the threads lock), then
   a dense zigzag; a narrow column gets the center walk two times. Fills:
   the edge walk two times and the tatami of the automatic underlay.
   Outlines that are runs already have Repeat and Backtrack (`RunPass`).
3. **The fabric decides** that a design is lace: the fabric preset
   `water_soluble` has the underlay weight `lace`
   (`UnderlayWeight::Lace`), so the objects with automatic underlay get
   the lace underlay, and `Fabric::is_lace` is true. A design with a lace
   mesh is lace too (`lace::is_lace`).
4. **The check** (`emb_stitch::lace::loose_parts`) works on the stitch
   plan: the stitches of each object from its first needle point (not
   the travel to it), cut at jumps, trims and color changes, are lines;
   two lines hold together where stitches cross or come nearer than
   0.3 mm. The group with the most thread is the lace; each other group
   is a loose part. Design info lists each loose part of a lace design
   as a warning (a click selects its object); `emb-cli stitch` prints
   them.
5. **Templates:** a lace ornament (a snowflake 80 mm across with a loop)
   and a lace edge (120 mm with scallops), our own designs in
   `emb_model::lace`, in the New from template window and in the corpus
   (`lace-ornament`, `lace-edge`). A test checks that they have no loose
   part.
6. **Travel at sharp inner corners.** The lace edge showed travel spikes
   where two scallops meet: the travel graph is on the shape moved in by
   0.3 mm, and a mitred inner corner of a sharp cusp goes far into the
   shape. The travel inset now cuts off corners sharper than 45°
   (`Join::ShortMiter`). This changed the travel of five lettering
   corpus designs by one stitch.

## Consequences

- A lace design needs the same thread in the needle and the bobbin; the
  help says so.
- The check finds parts that no thread joins; it does not know if a join
  is strong enough (one thin stitch holds less than a satin border).
- The exit test (a lace ornament and a lace edge on water-soluble
  stabilizer, washed, with no part that falls off) is open.
