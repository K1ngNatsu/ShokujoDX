# Plan: designs from other programs (phase P18)

Date: 2026-11-18 · Status: Accepted (owner, 2026-11-18). Started by the
owner on 2026-11-19, after the second part of P18 (the 3D stitch view).
Milestones 1 to 3 are done (ADR 0131, ADR 0132, ADR 0133).

## Goal

A shop gets most of its designs from other programs. The program opens
their machine files as stitch objects: the user can recolor, move, scale
and split them, and change single stitches. But a stitch object has no
shape and no settings, and **Recognize Outlines** makes one plain fill of
it (ADR 0030). The reference product makes objects with stitch types from
a machine file. The review of 2026-11-18 found this to be the largest gap
for professional users.

The owner decided (2026-11-18) to do all the legal ways, and not to read
the native formats of closed-source programs (rule 3; no public
description).

This phase has eight milestones.

What stays:

- No inspection of the programs or the native files of closed-source
  products. The test files are files that the project team or the testers
  digitized, or the designs of our corpus.
- A part that the program does not recognize stays a stitch object. The
  program never loses stitches.

## Milestone 1: the parts of a stitch file (M)

A stitch object is cut into parts of one kind.

1. The engine cuts the stitches at trims, jumps and color changes, and
   then by their pattern: a **run** (stitches along a line), a **column**
   (stitches that go from side to side), **rows** (long parallel lines of
   stitches), or **other**.
2. Stitches that sew under a later part of the same place (underlay) are
   found and go with that part.
3. A view colors the stitches by the kind of their part.

**Exit test:** for each design of the corpus, exported as DST and opened
again, 90 % of the stitches are in a part of the kind of their source
object.

**Done (2026-11-19, ADR 0131).** `emb_art::parts` cuts the stitches at
trims, jumps and color changes, then into legs (stitches in one
direction), and finds back-and-forth sequences of legs: rows (a turn
stitch between most rows, or long rows at one angle) or a column; the
other legs are runs. A part that later columns or rows of the same color
cover goes with them (underlay and travel). View > Stitch Parts colors the
stitches by part. The exit test (`crates/emb-cli/tests/parts.rs`, the
lettering cut into its letter parts): 97.1 % of the stitches of runs,
columns and fills of rows are in a part of their kind (columns 98.7 %,
rows 97.6 %, runs 67.9 %: the travel runs in letters go with the column
over them, as underlay).

## Milestone 2: runs and satin columns (L)

1. A run becomes a running object: its line, its stitch length, and a
   bean stitch when each stitch sews three or more times.
2. A column becomes a satin column: its two sides from the needle points
   of each side, made smooth; its spacing; its underlay kinds from the
   underlay stitches. The pull compensation is 0, because the sides have
   it already.

**Exit test:** on the corpus round trip, the width of a recognized column
is within 0.2 mm and its spacing within 10 % of the source object.

**Done (2026-11-19, ADR 0132).** `emb_art::part_objects` makes a running
object of each run (its line, its stitch length, a bean stitch when the
stitches sew three times) and a satin column of each column: the passes
of the stitches across, the densest pass as the top and the lighter ones
before it as zigzag underlay; the rails from the needle points of each
side, made smooth; rungs where the stitch angle needs them; the spacing
from the stitches across; pull compensation 0; the underlay runs as a
center walk or an edge walk, and an underlay setting (Auto, Light, Heavy,
Double) when its layers are the same. The other parts stay stitches. View
> Stitch Parts draws the lines of the objects. The exit test (the second
test of `crates/emb-cli/tests/parts.rs`): 98.9 % of the stitches of the
measured satins have the width within 0.2 mm and the spacing within 10 %
of their source (the spacing that the source sews: the engine rounds the
stitches in each section between two rungs). 95.6 % of the column
stitches are in a recognized satin.

## Milestone 3: fills (L)

1. Rows become a fill: its area with its holes (the method of ADR 0030),
   its angle from the rows, its row spacing, its stitch length and the
   offset of the needle points from row to row; its underlay.
2. A fill with rows in more than one direction becomes more than one
   fill.

**Exit test:** on the corpus round trip, the angle is within 5° and the
row spacing within 10 % of the source object.

**Done (2026-11-20, ADR 0133).** `emb_art::part_fills` cuts the rows of a
part into layers by their direction (a denser layer after a layer makes
it underlay) and makes a fill of each layer of rows: the angle, the row
spacing, the stitch length and the offset of the needle points (tatami
or square) from the rows; the area from the ends of the rows (traced as
in ADR 0030), with its holes; the underlay as tatami layers and edge
walks, or the setting that has them. Sections of one fill are one fill; a
fill with rows in more than one direction becomes more than one fill.
View > Stitch Parts draws the outlines. The exit test (the third test of
`crates/emb-cli/tests/parts.rs`): 99.9 % of the stitches of the measured
fills have the angle within 5° and the row spacing within 10 % of their
source; 98.7 % of the stitches of fills of rows are in a recognized fill.

## Milestone 4: Recognize Objects (M)

The user changes a stitch file into objects in one step.

1. **Artwork > Convert > Recognize Objects** shows the stitch object and
   the recognized objects next to each other, with the stitch counts and
   the places where the two differ.
2. The user accepts all parts, or keeps single parts as stitches.
3. The change is one undo step. **Open** can do it at once (an option,
   off by default).

**Exit test (owner):** a machine file of a logo with lettering becomes
objects; at 150 % of its size it sews correctly, with the density of the
first size.

## Milestone 5: read Ink/Stitch files (M)

Ink/Stitch keeps its objects and their settings in SVG files, in an open
format. The program reads them as objects.

1. The SVG reader reads the Ink/Stitch settings of each shape: fills
   (angle, row spacing, stitch length, underlay), satin columns (the two
   sides and the lines across them, pull compensation, underlay), lines
   (run, bean, zigzag), and the commands (trim, stop, start and end).
2. The thread colors come from the colors of the shapes.
3. A setting that the program does not know goes in a list of warnings.

Ink/Stitch is GPL-3.0. The reader follows its public documentation; a
part that is ported from its code gets the header of rule 4.

**Exit test:** three Ink/Stitch files of testers open with the same
objects, kinds and colors; one sews correctly.

## Milestone 6: write Ink/Stitch files and read its fonts (M)

1. **File > Export** writes a design as an SVG file with Ink/Stitch
   settings, for the objects that Ink/Stitch has.
2. The lettering fonts of Ink/Stitch (digitized by persons) become
   embroidery fonts (`.embfont`). Each font has its own license: a font
   is bundled only when its license permits it, with its license text and
   its source (rule 4); the others the user can convert.

**Exit test:** a corpus design written and opened in Ink/Stitch gives the
same objects; one converted font sews correctly at its sizes.

## Milestone 7: outlines with a stitch file (M)

Many programs can save the outlines of a design as a vector file. With
the machine file of the same design, the outlines become objects.

1. The user opens a vector file (SVG, PDF, EMF) and a machine file of the
   same design. The program aligns them (by their sizes, or by two points
   that the user clicks).
2. Each outline takes the kind, the angle, the spacing and the color of
   the stitches inside it (the parts of Milestones 1 to 3).

**Exit test:** a corpus design, saved as outlines and as DST, becomes
objects of the same kinds as the source.

## Milestone 8: import helpers (S)

The program opens artwork formats that it cannot read, with the help of a
program that the user has.

1. A table gives a command for a file type. The command runs as its own
   process with a time limit, and makes an SVG or PDF file that the
   program reads.
2. Built in: Inkscape, when it is on the computer, for CDR, EPS and the
   other formats that it reads. Options shows the helpers that the
   program found, and the user can add one.
3. No helper is a part of the program or of its packages.

An ADR records the method (ADR 0027 said that EPS and CDR are not
planned).

**Exit test:** with Inkscape on the computer, a CDR file and an EPS file
open as vector objects; with no helper, the program says what to install.

## Not in this phase

- The native formats of closed-source programs.
- Changes to the stitch engine for a better match with a recognized
  design (the recognized objects use the engine as it is).
