---
id: machines-and-hoops
title: Machines and hoops
group: production
keywords: machine, topaz, husqvarna, brother, tajima, multi-needle, needles, needle, needle sequence, thread change, needle chart, format, speed, sew time, color change time, trim time, dst trims, jumps for a trim, hoop, frame, field, margin, my hoops, own hoop, auto hoop, turn hoop, rotate design, hoop angle, hoop options, placement, auto centering, start point, end point, hoop template, print, multi-position
---
# Machines and hoops

## The machine

The **Machine** list (Design > Machine and Hoop) sets the machine of the
design. The machine sets:

- the file format of Export and Send to USB (for example VP3 for the
  Husqvarna Viking Designer Topaz 20, PES for Brother, DST for most
  multi-needle machines);
- the longest stitch, and the hoops of the machine;
- the speed and the times for the sew time.

**Machine Settings** (Design > Machine and Hoop) has:

- **Speed** (stitches a minute), and the time of a **Color change**, a
  **Trim** and a **Stop**. Design info and the worksheet use them for the
  sew time.
- **Trims in DST and TAP files**: these formats have no trim code, so a
  few short jumps tell the machine to cut the thread. Change **Jumps for
  a trim** and **Length of each jump** only if your machine does not cut,
  or cuts too often.
- **Heads**, **Hoop one piece**, **Frame change** and **Hoop the next
  pieces while the machine sews**: for the run time of an order. See
  [Quotes and run times](quotes).

## Needles of a multi-needle machine

A multi-needle machine has a thread on each needle and goes to the
needle of the next color by itself. **Needles** (Design > Machine and
Hoop) shows:

- **Needles on the machine**: the number of needles. The multi-needle
  machines have 15; set the number of your machine. **Reset** uses the
  number of the machine profile. A machine with one needle has no needle
  plan.
- **Colors in sew order**: the needle of each color. **Auto** keeps a
  thread on the needle that has it (a color that sews two times uses one
  needle) and puts a new thread on a free needle. Select a needle number
  to give the thread its own needle; undo takes it back.
- **Needles at the start**: the thread that each needle must have before
  the start, and the thread that is on the machine now. **put on** shows
  the threads to change.
- **Needle sequence**: the needle of each color in sew order. Type it on
  the machine for DST, TAP and EXP files; TBF and U01 files have the
  needles.
- **Keep these threads on the machine**: after you sew the design, its
  threads are on the needles. The next design uses them where it can, so
  you change fewer threads. **Clear the machine threads** forgets them.

If the design has more threads than the machine has needles, a needle
gets a new thread during the design: a **thread change** (*). TBF and
U01 files stop the machine there, the worksheet says which thread goes on
which needle, and the sew time counts a stop. Design info shows a
warning.

The production worksheet has a **Needle** column, the **Needles at the
start**, the needle sequence and the thread changes. An opened TBF or U01
file keeps its needles.

## The hoop

The **Hoop** list sets the hoop. The canvas shows the hoop field (where
the needle can go) and the safety margin.

- **My Hoops** comes first in the list. A hoop that the machine cannot
  use shows a warning.
- **Open field** has no hoop.
- **Auto Hoop** selects the smallest hoop that the design fits, from My
  hoops (else the hoops of the machine). The design turns 90° if it fits
  a smaller hoop that way.
- **Show Hoop** shows or hides the hoop on the canvas.
- The **hoop safety margin** (Options) is a distance inside the field.
  Stitches outside the margin get a warning in Design info; stitches
  outside the field are an error.

The top of the canvas is the back of the machine, and the hoop width is
from left to right on the machine.

## Turn the design or the hoop

- **Rotate Design** turns all of the design 90° clockwise in the hoop.
- **Turn Hoop Left** and **Turn Hoop Right** turn the hoop 15°, to fit a
  design on a slant. The machine file turns the stitches back, so the
  design sews as you see it.
- **Hoop Options > Hoop angle** sets the angle as a number.

## My hoops

**Design > My Hoops** lists the hoops. Mark **I have it** for the hoops
that you have: Auto hoop and the hoop list use them first.

**New hoop** adds a hoop of your own: the **Name**, the **Shape**
(rectangle, rounded rectangle, oval or circle), the **Width** and
**Height** of the field, and the **Corner Radius**. The size is the
sewable field, not the frame. A multi-position hoop has **positions**: the
center of each position from the hoop center. **Edit** and **Delete**
change your hoops.

## Hoop options

**Design > Hoop Options** has:

- **Placement**: **Manual** (the hoop stays where you put it) or **Auto
  centering** (the hoop center follows the center of the design).
- **Show the hoop**, and the **Hoop angle**.
- **Auto start and end**: the needle goes to the start point before the
  first stitch, and to the end point after the last trim. Choose the
  hoop center, a place on the design box (for example **Top left**) or
  **Off**. No stitch sews there. VP3 files have no jumps, so there the
  start point sets only where the first object starts.

## The hoop template

**Design > Hoop Template** saves the hoop field at full size as PDF, with
the center lines and the design. Print it at 100 % and put it on the
fabric, to put the design in the correct place. A large hoop prints on
more sheets.

## Designs larger than the hoop

See [Designs larger than the hoop](hooping).
