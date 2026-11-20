# Plan: designs from other programs (phase P18)

Date: 2026-11-18 · Status: Accepted (owner, 2026-11-18). Started by the
owner on 2026-11-19, after the second part of P18 (the 3D stitch view).
Milestones 1 to 3 are done (ADR 0131, ADR 0132, ADR 0133); Milestone 4 is
done except the sew-out of the owner (ADR 0134); Milestone 5 is done except
the exit test of the owner (ADR 0135); Milestone 6 is done except the exit
test of the owner and the bundled fonts (ADR 0136); Milestone 7 is done
except the exit test of the owner (ADR 0137); Milestone 8 (changed by the
owner: EPS files, and a message for CDR files) is done except the exit
test of the owner (ADR 0138).

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

**Done except the sew-out (2026-11-20, ADR 0134).** Artwork > Convert >
Recognize Objects opens a window with the stitches of the file and the
objects sewn again next to each other, the stitch counts, the places that
differ (red), and a list of the objects with a box to keep each one as
stitches. Accept is one undo step. Options > Recognize objects when a
file opens does it at Open (off by default). The automated part of the
exit test (the fourth test of `crates/emb-cli/tests/parts.rs`): six
corpus designs with lettering and logos, as DST and back, changed into
objects and scaled to 150 %, have 0.89 to 1.14 of the density of the
first size (the stitch objects: 0.67 to 0.75). The sew-out is for the
owner.

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

**Done except the exit test of the owner (2026-11-20, ADR 0135).**
`emb_formats::inkstitch` reads the settings of each shape (by its id,
after usvg) and the commands (by their connectors): fills (tatami, square,
contour, guided, circular, meander, cross stitch, linear gradient; angle,
row spacing, stitch length, end spacing, expand, underlay layers), satin
columns (rails, rungs, spacing, pull compensation, short stitches, split,
center walk, contour and zigzag underlay; E stitch and zigzag), lines
(run, bean, passes, zigzag, ripple, manual, cut lines), trim, stop, start
and end points, target points and ignored shapes and layers (vector
objects); the files of earlier versions with the changes of
`lib/update.py`. The thread colors are the colors of the shapes, with the
names of the chart that the file names. A setting that the program does
not read gives a warning. File > Open opens the file as a new design;
Import Artwork adds its objects. The names and defaults are from the
Ink/Stitch source (commit 42e771e9); the rail rules are ported with a
header (rule 4). The automated part of the exit test
(`crates/emb-cli/tests/inkstitch.rs`): a badge written by hand in the
structure of Ink/Stitch files opens with its objects, kinds, settings and
colors, and sews. The files of testers are for the owner.

## Milestone 6: write Ink/Stitch files and read its fonts (M)

1. **File > Export** writes a design as an SVG file with Ink/Stitch
   settings, for the objects that Ink/Stitch has.
2. The lettering fonts of Ink/Stitch (digitized by persons) become
   embroidery fonts (`.embfont`). Each font has its own license: a font
   is bundled only when its license permits it, with its license text and
   its source (rule 4); the others the user can convert.

**Exit test:** a corpus design written and opened in Ink/Stitch gives the
same objects; one converted font sews correctly at its sizes.

**Done except the exit test of the owner and the bundled fonts
(2026-11-20, ADR 0136).** Design > Export for Ink/Stitch
(`emb_formats::inkstitch::write`) writes fills, satin columns (rails and
rungs that Ink/Stitch pairs as the program does), lines, stitch objects
(manual stitches), lettering (its parts), appliqué (placement, tack-down
and cover with stops) and vector objects, with their settings, the
underlay layers that they sew, and commands for start and end points;
notes list what Ink/Stitch does not have. Lettering > Fonts > Ink/Stitch
Font… (`emb_letter::inkstitch`) converts a font folder of Ink/Stitch: the
glyph layers become fixed letters with the settings of each part, with the
advances, the kerning, the line height, the space and the size range of
`font.json`, and the license in the notes. The automated part of the exit
test (`crates/emb-cli/tests/inkstitch.rs`): the 85 corpus designs, written
and read again, give the same 2,768 objects, kinds and colors; the font
Cats of Ink/Stitch (OFL) converts with 115 letters, and its test sheet
sews at three sizes. The owner chose a test only for the fonts of
Ink/Stitch (2026-11-20): no font is bundled yet.

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

**Done except the exit test of the owner (2026-11-20, ADR 0137).**
`emb_art::part_outlines` puts the outlines on the stitches (by their
sizes, or by two points) and gives each outline the kind and the settings
of the made objects inside it: an area of fill stitches is a fill of the
outline, an area of column stitches a satin column of the outline with the
stitch directions of the column, a line a run or a satin along it. The
Recognize Objects window has Outlines…, Fit by Size, Align by Two Points
and Remove Outlines, and draws the outlines on the stitches of the file.
The automated part of the exit test (`crates/emb-cli/tests/outlines.rs`):
the corpus designs, saved with Export Outlines and as DST, give 92.8 % of
the 1,819 measured objects an object of their kind, and every such fill
has the angle of its source within 5°.

## Milestone 8: EPS files, and a message for CDR files (M)

Changed by the owner on 2026-11-20 (it was "import helpers": another
program on the computer converts CDR and EPS files). The program does not
depend on a second program: a shop must not install a vector editor to
open the logo of a customer, and the program must not run another program
on the files of customers.

1. The program reads EPS and PostScript files itself: a PostScript
   interpreter in pure Rust (from the public PostScript language
   reference and the EPS file format specification) runs the drawing
   operators of the file: paths, painting, colors, the transform, the
   graphics state, procedures and dictionaries. Text and pictures give
   warnings, as in the PDF import.
2. AI files of the old kind (PostScript, not PDF inside) open with the
   operators of the AI script.
3. A CDR file gives a message: the format has no public description, so
   save the file as SVG, PDF or EPS in the program that made it.
4. No helper program. Another program can be added later as an option if
   testers get files that they cannot convert.

An ADR records the method (ADR 0027 said that EPS and CDR are not
planned).

**Exit test:** EPS files of three kinds (simple procedures, a large
printing procset, an old AI script) open as vector objects with their
colors and shapes; a CDR file gives the message. The test of the owner:
the EPS and AI files of testers.

**Done except the exit test of the owner (2026-11-20, ADR 0138).**
`emb_formats::eps` is a PostScript interpreter in pure Rust (about 300
operators, limits for steps, memory and errors) and runs the AI script of
old AI files with its own operators; `artwork::import_artwork` takes EPS,
PS and AI files of both kinds, and a CDR file gives the message. The
automated part of the exit test (`crates/emb-cli/tests/eps.rs`): three
files of the same badge, written in three ways, open as the same six
objects with their colors and sizes, and sew. The published description
of the AI format could not be read (its copies were offline), so the
files of testers must check the AI operators.

## Audit

The audit of both parts of the phase is in the roadmap: "Audit of P18
(2026-11-20)". It limits what an EPS program can paint and copy, leaves
out the shapes of artwork files that are farther than 1 km, limits the
repeats and the expand of an Ink/Stitch file, and makes an outline take
objects only if they cover a quarter of its box.

## Not in this phase

- The native formats of closed-source programs.
- Changes to the stitch engine for a better match with a recognized
  design (the recognized objects use the engine as it is).
