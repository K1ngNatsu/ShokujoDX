# Export plan: roadmap phase P1 (formats and stitch engine)

Date: 2026-10-21 · Status: Accepted. Milestones 1–6 done. The sew-outs of this plan are covered by the later sew-outs (owner decision, 2026-10-23; see the roadmap, "Exit tests").

## Goal

Export a design that sews correctly on the Topaz 20. Each milestone ends with a
file that you can export and sew, so the work is testable on the machine early.

## Pipeline

```text
Design (objects, sew order)                         emb-model
  │  per object, pure and deterministic             emb-stitch::{running, fill, satin}
  ▼
ObjectStitches (runs of needle points, mm)          cached by (ObjectId, revision)
  │  route, connect, trim, lock, color change,      emb-stitch::plan
  │  stitch-length limits, design rotation,
  │  origin at hoop center
  ▼
StitchDesign (records: Stitch, Jump, Trim,          emb-model::stitch
  │           ColorChange, Stop, End; mm)
  │  units, record limits, header                   emb-formats::{dst, vp3, pes}
  ▼
machine file bytes
```

The app draws the same `StitchDesign` in the Stitches view, so what you see is
what you export.

## StitchDesign contract

- Coordinates in mm, Y down, origin at the hoop center (the needle start).
- `threads[0]` is the first color; each `ColorChange` moves to the next thread.
- A `Jump` moves the frame with no penetration. Writers split long jumps.
- A `Trim` cuts the thread at the current position. Its position is the
  current position.
- `ColorChange`, `Stop` and `End` are at the current position.
- The plan starts with a `Jump` to the first stitch and ends with `Trim`, `End`.

## Milestones

| # | Scope | Exit test |
| --- | --- | --- |
| 1 | Running stitch (even spacing, corners kept, bean stitch). Stitch plan: routing for open and closed lines, travel stitch or trim by gap, tie-in and tie-off, color changes, max and min stitch length. DST write and read, round-trip tests. Stitch cache and a real Stitches view. Export dialog (DST) with summary, warnings and the out-of-field block. | A line-art design exports to DST, opens in another viewer with the same shape, and sews on the Topaz 20. |
| 2 | VP3 write and read (pyembroidery `Vp3Writer`/`Vp3Reader`). Export default from the machine profile. | A VP3 file loads and sews on the Topaz 20. Confirms the open question "native format". |
| 3 | Tatami fill: rows at the fill angle clipped to the shape, row offset pattern, connected routing with travel inside the shape (port of the Ink/Stitch auto-fill graph method). Fill underlay. Pull compensation for fills. | 50 mm circle and two-hole fill from the corpus sew flat with no gaps. |
| 4 | Satin: rails and rungs to stitch pairs, spacing, pull compensation, short stitches on curves (port of Ink/Stitch satin column). Underlay: center walk, edge walk, zigzag; `Auto` rules from the spec. | 3 × 80 mm column and S-curve from the corpus sew with even coverage. |
| 5 | PES v1 with PEC block (pyembroidery `PesWriter`/`PecWriter`). | Opens in a Brother-compatible viewer; a test user with a Brother machine sews it. |
| 6 | `emb-cli convert` and `emb-cli stitch`. Golden snapshot tests for the corpus (`insta`). Differential script against Ink/Stitch (counts, extents, density). | CI runs the golden tests on three platforms. |

Milestones 2 and 3 can swap if the Topaz test of Milestone 1 shows that DST
does not load on the machine.

## Progress

| # | State | Notes |
| --- | --- | --- |
| 1 | Software done; sew-out open | `emb-stitch` (running, plan, cache), `emb_formats::dst`, Stitches view, Export dialog. `tests/corpus/svg/line-art-sampler.svg` exports to DST; pyembroidery reads it back with the same stitches, 3 colors and 5 trims. To write the sample files: `EMB_SAMPLE_OUT=<dir> cargo test -p emb-app -- --ignored write_sampler`. |
| 2 | Software done; sew-out open | `emb_formats::vp3`. The VP3 of the sampler has the same bytes as pyembroidery writes, except the stitch-count field (pyembroidery counts the records of its own pattern). The Export dialog selects VP3 for the Topaz 20. |
| 3 | Software done; sew-out open | `emb_stitch::{fill, travel}`: tatami rows, sections, travel inside the shape, staggered needle points, pull compensation, tatami and edge-walk underlay. |
| 4 | Software done; sew-out open | `emb_stitch::satin`: rails and rungs, spacing, pull compensation, short stitches, auto split, center-walk, edge-walk and zigzag underlay. Column C (`SatinGeometry::from_centerline`) and the Column C draw tool. |
| 5 | Software done; Brother test open | `emb_formats::pes`: PES version 1 with the PEC part. For all corpus designs the bytes are the same as pyembroidery writes (with the same rounding). |
| 6 | Done, except CI and the Ink/Stitch comparison | `emb-cli` (`convert`, `stitch`, `corpus`), the corpus in code (`emb_cli::corpus`), golden tests in `crates/emb-cli/tests/golden.rs`. CI on three platforms was deferred by the owner; it is now in `.github/workflows/ci.yml` (and the release builds, ADR 0043). The comparison with Ink/Stitch needs an Ink/Stitch install; it is open. |

To write all corpus designs as project, VP3, DST and PES files for a sew-out:

```bash
cargo run -p emb-cli --release -- corpus out/corpus
```

## Decisions for Milestone 1

- **Running stitch.** Not a port: even spacing along the line is common
  knowledge. Real corners (turn > 30° after flattening) always get a needle
  point; between them, stitches have equal length not more than the set length.
  Bean stitch (repeats = 3) sews each stitch forward, back, forward.
- **Routing.** If the object has no start point, an open line starts at the end
  nearer to the needle, and a closed line starts at its vertex nearest to the
  needle. A set start point overrides this.
- **Connect or trim.** Same color and gap ≤ 2 mm: one travel stitch. Larger
  gap: tie-off, trim, jump, tie-in.
- **Lock stitches.** Tie-in: two short stitches forward and back along the first
  stitch (0.5 mm, or half the first stitch if it is shorter). Tie-off: the same
  along the last stitch. At the start and end of each color block and around
  each trim.
- **Limits.** Stitches longer than the machine maximum are split into equal
  parts. Stitches shorter than the machine minimum (default 0.3 mm) are dropped,
  except lock stitches and the first stitch after a jump.
- **Fill and satin objects** before Milestones 3 and 4: the plan skips them and
  the export dialog lists them as warnings.
- **DST.** Copy pyembroidery behavior (commit `c16d1b4`): 0.1 mm units, Y up in
  the file, records of at most ±121 units per axis, trims as three jumps that net
  zero, color change and end records with no movement, 512-byte header with
  `LA`, `ST`, `CO`, `±X`, `±Y`, `AX`, `AY`, `MX`, `MY`, `PD`. One deviation: the
  label is cut to 16 ASCII characters, so the fixed header layout holds.
- **Export.** Export is blocked if a stitch is outside the hoop field; the
  dialog shows where. The native project file is saved next to each export
  (spec, "Native project format").

## Decisions for Milestones 2–6

- **VP3.** Port of pyembroidery `Vp3Writer`/`Vp3Reader`. VP3 has no jump
  record, so a jump is part of the move of the next stitch (as in all VP3
  files from pyembroidery and Ink/Stitch). The end adds a trim, because the
  machine does not trim by itself. The reader makes the first stitch of a
  color block, and the first stitch after a trim, into a jump plus a stitch.
  The design center is an integer, so the color block start positions agree
  with it (deviation, see `vp3.rs`).
- **Fill: not a port.** Ink/Stitch builds a graph of all row segments and
  edges and searches for one path through it. We use a scan-line fill with a
  boustrophedon decomposition: rows on a global grid, sections of rows that
  connect one to one, each section sewn back and forth, the next section is
  the one with the corner nearest to the needle. It is simpler, and fast
  (the 220 × 160 mm corpus panel takes a few ms). See `fill.rs`.
- **Travel.** Inside the shape, on the corners of the shape moved 0.3 mm
  inward (visibility graph, A*). Between sections, a step across the inside
  costs three times a step along the edge, so travel goes along the edge,
  where later rows and the edge cover it better. From row to row, the travel
  is the shortest path (a straight stitch where the edge does not bend inward).
  If two parts of a shape do not touch, the fill is two needle paths and the
  plan connects them with a travel stitch or a trim.
- **Tatami pattern.** Needle points are on a grid along each row, moved by a
  quarter of the stitch length from row to row (4 staggers). Grid points nearer
  than a quarter of a stitch to a row end are not used, so the last stitch of a
  row can be up to 1.25 times the set length.
- **Fill underlay (`Auto`).** Tatami at 90° to the top rows, 2 mm spacing,
  0.5 mm inside the edge, for shapes larger than 20 mm² that still exist after
  the inset. Custom underlay angles are relative to the fill angle.
- **Satin.** Rungs (and the rail ends) cut the rails into sections; in a
  section, equal fractions of the two rail parts pair up. Spacing is the
  distance between needle points on one side, measured along the middle of the
  column. Short stitches: on a side where needle points are nearer than
  0.25 mm, every second stitch stops 15 % of the width before the rail. Auto
  split: stitches longer than 7 mm get needle points that move by half a part
  from stitch to stitch.
- **Satin underlay (`Auto`).** Wider than 2 mm: edge walk and zigzag (2 mm
  spacing), 0.4 mm inside the rails. 1 to 2 mm: center walk. Narrower: none.
  Each underlay layer goes to the far end and back, so the top stitching
  always goes from the start to the end of the column.
- **Start of fill and satin.** They start at the corner or column end
  nearest to the needle. See ADR 0007 (the cache key has the needle position).
- **Column C.** A centerline with a fixed width (3 mm for the draw tool)
  becomes rails with mitered corners, and a rung at least every 2 mm, so the
  rails stay paired on curves.
- **PES.** Port of pyembroidery `PesWriter` (version 1) and `PecWriter`. The
  machine sews the PEC part; its colors are the nearest entries of the
  64-color Brother palette, each thread color to a different entry. A trim
  with no jump after it is not in the file (the PEC format has trim only as a
  flag on a jump).
- **Machine profiles.** `trim_gap_mm` (optional) sets the gap above which the
  plan trims, per machine. Max and min stitch length were already there.
- **Golden tests.** A text summary per corpus design (counts, extents,
  longest stitch, colors, and a hash of all records at 0.001 mm). A changed
  snapshot needs a note in the commit message (CLAUDE.md, rule 7).
