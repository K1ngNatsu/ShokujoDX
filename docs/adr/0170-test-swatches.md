# ADR 0170: Test swatches

Date: 2026-11-25 · Status: Accepted (P22 Milestone 3,
`docs/design/p22-plan.md`).

## Context

The user finds the settings for a fabric by sewing them out: the same
object with one setting in steps. Before, the user made the copies by
hand, changed each one, and wrote the values on the fabric. The sew-out
kits of the project are made by Python tools. The idea is from zigzag
(an open-source program under Apache-2.0); the code is our own.

## Decision

- **`emb_letter::swatch`** (in `emb-letter`, not in `emb-model` as the
  plan said: the labels are lettering, laid out with `layout::relayout`).
  `SwatchSetting` is spacing, stitch length, pull compensation, width,
  angle or underlay (ADR 0171 added lean and band overlap, and a copy of
  a group is a new group); `value_in` and `set` read and change it for the kinds
  that have it (fills; satin columns; lines, and lines with a border for
  the column settings; lettering for spacing, pull compensation and
  underlay, with `Lettering::apply_params`). The width of a satin column
  changes round its middle (`widened`: each point of a rail moves from the
  middle between it and the other rail; the mean width is `satin_width`).
  For underlay, there is one copy for each of none, light, auto, heavy and
  double.
- `swatches` makes a new design with the threads, machine, hoop and
  fabric of the design. For each value it has a copy of the selected
  objects (the objects that do not have the setting stay as they are).
  The copies are in a grid round the center of the hoop, with as many
  columns as fit the field of the hoop. Under each copy is its value, as
  lettering in the bundled font for small text (`sheet::LABEL_FONT`,
  Soft Round, at `sheet::label_height`: 5 mm or its smallest size, 6.5
  mm; it was Atkinson Hyperlegible until the agent test of P22 Milestone
  9, where the bold digits closed their holes at 5 mm). The copies do not
  overlap, so their colors sew together
  (`Design::optimize_color_changes`).
- **The window** Design > Output > Test Swatches… (`test_swatches.rs`)
  lists only the settings that the selection has, with From, To and
  Copies (start values round the value of the design), and shows the
  labels and the size of the grid. Make Swatches opens the result as a new
  design tab, so the design of the user does not change. A grid larger
  than the hoop gets the hoopings of `HoopingLayout::auto` and a message.
  Lengths in the labels use the units of the program, with one decimal
  more (`Units::fmt_fine`), so that 0.30 and 0.35 mm do not look the same.
- **`emb-cli swatch`** makes the same swatches from a project file, for the
  tools that make sew-out kits.

## The audit of P22

The values of the copies stay in the range of the setting
(`SwatchSetting::range`; a value that is not a number is the start of the
range): a pull compensation that was not a number gave stitches that were
not numbers, and a width of 1e9 mm a design out of the range of a
project.

## Consequences

- The exit test of the milestone in software is met: five copies of a
  satin with spacing from 0.30 to 0.50 mm have these spacings and labels,
  and no label touches a copy (`five_copies_of_a_satin_with_their_spacings_and_labels`);
  each setting changes the kinds that have it
  (`each_setting_changes_the_kinds_that_have_it`); the design of the user
  does not change (`the_swatches_are_a_new_design_and_the_design_does_not_change`).
  The sew-out of a sheet of swatches on the Topaz 20 is the owner's.
- The labels sew in the thread of the first selected object.
- Milestone 4 uses swatches for the sew-out of the satin lean and the
  bands of threads.
