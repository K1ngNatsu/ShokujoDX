# ADR 0163: Less thread at the joints of letters

Date: 2026-11-24 · Status: Accepted (the owner: "Fix monogram issues",
after the tutorial steps of P19 Part 2 found them).

## Context

The steps of the towel tutorial (ADR 0162) found that four of the six
monogram templates, and the Classic style, gave "Thread builds up"
warnings in Design Info (up to 11 layers of thread, limit 5), and some
gave short stitches. A probe of each template and style (initials JMK,
ABC, SRW, EAN; woven cotton and towel) and of single letters of Block
Sans at 30 mm found four causes. None of them was in the layout of the
monogram:

1. **The underlay crowded where rungs are close.** A column puts at least
   one pair across it between each two rungs. That is correct for the top
   stitches (a needle point at each rung), but the underlay used the same
   pairs. Where a column pivots, its rungs are very close, so its zigzag
   underlay (2 mm apart) got a pair at each rung, and all its lines went
   through the inside of the turn. One column of "K" had 8.0 layers by
   itself (2.8 with no underlay).
2. **Two underlays at each joint.** Where two columns of a letter meet,
   each column sewed its underlay under the other: at the corners of "M"
   and the junction of "K", 7 to 9 layers.
3. **Lock stitches shorter than the machine stitch.** A lock stitch could
   be 0.1 mm long, but Design Info warns of stitches shorter than the
   shortest stitch of the machine (0.3 mm). A column or a small fill that
   starts or ends in a point got four such stitches.
4. **New lettering and monograms did not suit the fabric.** They start
   from the base settings of lettering; only the objects that the tools
   draw (with `Design::current`) got the settings of the fabric of the
   design. A monogram made after **Towel (terry)** had the underlay and
   the density of woven cotton.

## Decision

1. **The underlay goes at even steps along the column**
   (`Column::even_pairs`): the center walk, the edge walk and the zigzag.
   The top stitches keep a needle point at each rung (`Column::pairs`).
2. **No underlay under another part of the letter.** When a lettering
   object sews a letter, a satin part leaves out its underlay where
   another satin or fill part of the same letter lies, and
   `satin::JOINT_MARGIN_MM` (1 mm) round it: the top stitches of that part
   go past its outline (pull compensation), and a joint is not a sharp
   line. The underlay crosses such a place with a walk, under the top
   stitches of the column (`satin::satin_on`, `LetterSewer::parts`). At a
   joint the fabric has two layers of satin, which hold it.
3. **A lock stitch is at least the shortest stitch of the machine**
   (`Planner::min_lock`). A tie-in locks toward the first needle point
   that is far enough; a tie-off toward a needle point before it that is
   far enough (at most 12 back), else it has no lock.
4. **New lettering and monograms suit the fabric:** they get
   `Design::fabric_change`, the change from the base settings to the
   fabric of the design, as `Design::set_fabric` gives to the objects that
   are there.

The limit of the density map stays 5 layers.

## Consequences

- The built-in monogram templates sew with no warning for the initials
  of the test (`the_monogram_templates_sew_with_no_warning`), also on
  towel fabric, with known places that are still over the limit
  (`docs/design/open-work.md`): the Oval style squeezes the outer letters
  into narrow columns, so "Oval in a double circle" has small places over
  the limit with some initials and on towel fabric, and on towel fabric
  "Block in a square" has one place of 4 mm² (5.8 layers). A large style
  with a circle border can be larger than the hoop: Design Info says so,
  and that is correct.
- An underlay with its ends left out starts at the start of the column
  and walks to its first point, and walks from its last point to the
  start of the top stitches (`satin::walk_to`): no long stitch.
- The stitches of all satin columns with rungs change (the underlay), and
  the joints of all satin lettering have less underlay. The golden
  snapshots, the bench pictures and the sew-out kits change with this
  decision; the commit message says why.
- The column of "K" that pivots went from 8.0 layers to 4.6, and the
  joints of "JMK" in Block Sans at 30 mm have no place over the limit.
- `emb_render::density::DensityMap::layers_at` is public (the probes of
  the monograms use it).
- ADR 0164 fixed the Oval style (the ends of the oval envelope, and the
  height of the middle initial of the oval and diamond styles).
