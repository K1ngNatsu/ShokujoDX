# ADR 0044: Density map in layers of thread, and an estimate of thread use

Date: 2026-10-25 · Status: Accepted (P11 Milestone 1)

## Context

P11 Milestone 1 asks for a density map over all objects (the owner report
of 2026-10-24: satin from SVG shapes built up on the machine) and for the
thread use and cost of a design. Design info checked the density of one
object at a time (rows closer than 0.25 mm), so it could not see objects
on top of each other. Design info showed the length of the stitches on
the fabric only.

## Decision

1. **The density map counts layers of thread** (`emb_render::density`),
   not needle points. The thread of each stitch goes to the cells of
   0.5 mm that it crosses, by length; the layers of a cell are that length
   times the thread width (0.4 mm, 40 wt), divided by the cell area; each
   cell takes the mean of the 3 × 3 cells round it. A layer is a unit that
   a user can see in the fabric, and it adds up the same way for each
   stitch type. The map works on the stitch plan (machine coordinates),
   so it sees what the machine sews: travel, underlay and lock stitches
   too.
2. **The limit is 5 layers** (Options > Density limit, 2 to 12). On the
   corpus designs a fill with its underlay has about 2 layers and a satin
   column with its underlay 3 to 4, because satin stitches go both ways.
   Over 5 are a satin or a second fill on a fill with no knockout, the
   joins of small satin letters, letters on their shadow letters and the
   center of radial fills. Places smaller than 4 mm² are left out: a satin
   that ends in a point has small peaks there. Up to 20 places go in the
   warnings of Design info (a click zooms to the place).
3. **View > Density map** draws the map as one texture over the stitches
   (blue, green, yellow up to the limit, red over it), with a line that
   gives the most layers.
4. **Thread use is a model with three parts** (`emb_model::thread_use`):
   the top thread is `L × 1.6`, the bobbin thread `L × 0.4` (on the back,
   the bobbin thread shows in the middle 40 % of a stitch and the top
   thread in the rest), each needle point adds the fabric thickness to each
   thread, and each start and trim adds a tail of 15 mm of top thread.
   `L` is the length of the stitches on the fabric. Each fabric preset has
   a thickness (`thickness_mm` in `data/fabrics.toml`, 0.6 mm if a design
   has none). The corpus designs give 5 to 6 m of top thread for each
   1 000 stitches; the trade rule is 4 to 5 m. The exit test (spools
   weighed before and after two sew-outs) sets the factors.
5. **Cost** from the length and price of a spool and of a bobbin
   (Options), with a currency text. A price of 0 shows no cost.
6. Design info, the worksheet (the column "Top thread" and a line with
   the totals) and `emb-cli stitch` show the thread use; `emb-cli stitch`
   also lists the places over the limit. `emb_cli::summary` does not
   change, because the golden snapshots are made from it.

## Consequences

- The model does not know the tension of the machine or the thread
  weight; a thicker thread (30 wt) shows fewer layers than it has.
- A design of 300 × 200 mm has 240 000 cells; a larger design gets larger
  cells (at most 4 million), so the map stays fast.
- The test of satin from SVG shapes now also checks that the map shows no
  place over the limit, and that it shows the old fault (a second column on
  top of a ring) all round the ring.
- Design info labels are translated now (they were English in all
  languages).
