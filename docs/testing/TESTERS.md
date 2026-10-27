# ShokujoDX: test build

Thank you for testing. This is an early build of a free embroidery
digitizer. We want to know what is hard to find, what does not work, and
what you miss from the software that you use now. Your notes decide what
we build next.

## Start

The program has no signature yet, so your system can show a warning the
first time. The steps below tell you how to start it.

**Windows 10 or 11, 64-bit.** Use the installer (`…-windows-setup.exe`) or
the portable zip (`…-windows-portable.zip`).

1. Installer: start it and follow the steps. You need no administrator
   rights. It adds a Start menu entry, and project files (`.embproj`) open
   with the program. As a choice, machine files (DST, PES, VP3, JEF and
   more) get the program in their "Open with" menu.
2. Portable zip: unzip the folder to a place of your choice (for example
   your Desktop) and start `ShokujoDX.exe`. Nothing is
   installed.
3. Windows can show "Windows protected your PC". Click **More info**, then
   **Run anyway**.

To remove the program, use **Settings > Apps** (installer) or delete the
folder (zip).

**macOS 10.15 or later** (Apple silicon or Intel). Open the disk image
(`…-macos.dmg`) and drag **ShokujoDX** to **Applications**.
The first time, macOS can say that it cannot check the program. Open
**System Settings > Privacy & Security** and click **Open Anyway** (on
older macOS: Control-click the app and click **Open**). On macOS, a
double-click on a design does not open it yet: use **File > Open**, the
recent designs, or drag the file onto the window.

**Linux** (64-bit). On Debian, Ubuntu or Linux Mint, install the `.deb`
(double-click it, or `sudo apt install ./shokujodx_….deb`). On
other systems, make the AppImage executable (`chmod +x …AppImage`) and
start it.

On all systems you can drag a design, a machine file, an SVG or a picture
onto the window to open it. Your settings are in `%APPDATA%\ShokujoDX`
(Windows), `~/Library/Application Support/ShokujoDX` (macOS) or
`~/.config/shokujodx` (Linux). Removing the program does not remove
them; delete that folder too if you want.

## Before you start

- **Machine and hoop**: on the **Design** tab, select your machine and
  hoop. The machine sets the file format for export (for example VP3 for
  Husqvarna Viking, PES for Brother, DST for most others).
- **Save often** (Ctrl+S). The program also makes a backup every 2 minutes,
  and offers it again if the program stops.
- The start page shows your recent designs and the tutorials. **Help > Start
  Page** shows it again.
- Put the pointer on a button and push **F1**: the help opens at that
  command. Help > Help has all topics, and a box for a question.
- Undo is Ctrl+Z. The Space bar does the last ribbon command again.

## New in this build

- **Column** (Digitize tab): click along one side of the shape, then
  along the other side. The fill shows between the two sides as you draw.
- **Border position**: in **Border Trace** and **Borders and Offsets**
  (Artwork tab), put the border outside, centered on or inside the line.
- **Command windows dock** at a corner of the canvas. Drag one near
  another corner and it docks there.
- The help has more topics and tutorials: push **F1** on any window or
  button.
- The program has a new name, ShokujoDX, and a new icon.
- **A clean start**: a new install has no sample designs and no test
  objects. The start page shows your own recent designs only.
- **Needles** (Design tab), for multi-needle machines: set the number of
  needles of your machine, then see the needle of each color, the threads
  to put on before the start, and the needle sequence. A color that sews
  two times keeps its needle. TBF and U01 files have the needles, and the
  worksheet has a needle column. If you have a multi-needle machine,
  please sew a TBF or U01 file with more colors than needles and tell us
  if the machine stops at each thread change (*).
- **Quote** (Design tab, Orders): set your prices once, then see the
  price of an order (quantity prices, setup, digitizing, rush, tax), the
  run time on a machine with more heads, and save a quote sheet for the
  customer. Set the heads and hooping times in **Machine Settings**. If
  you run a shop, please compare the quote and the run time of a few
  real orders with your own numbers.
- **Order Details** (Design tab, Orders): the customer, order number,
  garment, sizes, placement and due date of a design. The worksheet and
  the approval sheet show them, and the worksheet barcode has the order
  number. Please print a sheet for a real order, and scan its barcode
  with your phone or shop scanner.
- **Thread Stock** (Design tab, Orders): add the threads that you have
  (from a chart, or a CSV file), then turn on **Only threads in stock**
  so that Picture to Design and Match Design to Chart use only them. **Use
  Threads in Stock** changes a design to your threads, and the window
  lists the threads to buy for the order. Tell us if the numbers of
  spools to buy look right.
- **Scripts** (Edit tab): small tools that change the design. Try the
  samples (Outline Every Fill, Same Density for All Satins, Name Threads
  by Code, Check Before Output). If you write your own (Help > Scripts),
  tell us which functions you miss.
- **Sketch** (Digitize tab, Shapes > Start Sketch): draw with exact sizes,
  as in a CAD program. Draw lines, arcs and circles, add constraints
  (horizontal, tangent, equal and more) and dimensions, and drag the
  shape: the constraints stay true. Blue parts can move, green parts are
  fixed, red is a conflict. Try to draw a shield 60 mm wide that is fully
  constrained (all green), and tell us how long it took.
- **Sketch edits**: while you draw, the program adds horizontal,
  vertical, tangent and on-curve constraints (hold Alt for none). Try
  Trim, Extend, Fillet, Chamfer, Offset, Mirror and the patterns on the
  Sketch tab. Round the corners of a rectangle with Fillet, then change
  its width: the corners must stay round and smooth.
- **Keys and values in sketches**: Esc goes back to Select, Delete deletes
  the selected parts, and a right click shows the commands for a part.
  Type values in other units (`2 in`, `1 1/2"`, `50 mm`) or calculate
  (`120/4`). All length fields of the program take such values.
- **More sketch tools**: center and three-point rectangles, slots, a
  polygon round its circle, collinear. The Dimension tool shows a ghost of
  the dimension at the pointer; click to place it. Watch for the M
  (middle), X (crossing), // and ⊥ marks while you draw lines. Tell us
  which sketch tools of CAD programs you miss most.
- **Sketch moves and more dimensions**: drag from the end of a line for a
  tangent arc; Move, Copy, Rotate and Scale (select first); put a
  dimension of a slanted line above it, beside it or along it; right-click
  a waiting dimension for its kind (arc length, too); Relax Mode.
- **Even more sketch tools**: Tangent Circle (click three lines), the
  Three-Point Center Rectangle, the slot kinds (Properties panel),
  chamfers with two distances or an angle, Stretch, Center Point, and
  Automatic Dimensions.
- **Embroidery from a sketch**: draw a badge (two circles and a shape in
  them), click Regions, click inside the regions, then Fill, Satin or
  Appliqué (Properties panel or right click). Sew it and tell us how the
  satin ring and the fill with a hole come out.
- **Objects that follow the sketch**: make the badge, change its
  settings and threads, then double-click it and change a diameter. Do the
  fill and the satin ring change with their settings? Sew it before and
  after.
- **Parameters and templates**: give the badge a parameter `size` and
  type `size` in its diameter, save it as a template with its objects,
  then make a new design from the template with another size.
- **Splines and drawings**: draw a spline through a few points and drag
  them. Import a logo as vectors (or open a DXF drawing) and use Vector to
  Sketch: are the lines, arcs and tangents right?

## What to try (about one hour)

Do the tasks with no help first. Write down where you got stuck, and how
long each task took. Then use the program as you like.

1. **A patch from a picture.** Load a simple logo picture (PNG or JPEG)
   with **Artwork > Picture to Design** and make a two-color patch. Or
   load it as a backdrop picture and draw the shapes on it with the tools
   of the **Digitize** tab. Export it for your machine
   (**File > Export Machine File**).
2. **Lettering.** Put a name 25 mm high on an arc (**Lettering**
   tab). Try a second font, and change the spacing of one letter.
3. **Edit.** Change the stitch type, the stitch angle and the density of a
   fill. Change the sew order in the Sequence panel. Change a shape with
   **Reshape**.
4. **Production.** Make the machine file and the worksheet in one step
   (**Design > Output Design**). Print or look at the worksheet.
5. **Your own files.** Open one of your machine files (DST, PES, VP3, EXP,
   JEF and others). Change the colors and save it again.
   Import an SVG as vectors (**Artwork**), select it and click **To
   Satin**. Draw a **Direction Line** (**Edit** tab, Shape panel) through
   a satin and see the stitches turn with it. Click **Arrange > Shortest
   Order** and look at the number of trims in the status line. If you can,
   sew it: tell us if the satin builds up anywhere.
6. **More (if you have time).** A design larger than your hoop
   (**Hooping** tab), the design on a T-shirt (**Design > Product**),
   monograms and team names (**Lettering** tab), appliqué, the design
   library (the **Library** tab next to the design), Send to USB, cross
   stitch and rhinestones (**Specialty** tab: paint crosses on a grid,
   a picture to cross stitch, a chart to print; stone runs and fills,
   and a template for a cutter).

After you install the program, look at a folder of designs in Explorer,
Finder or your Linux file manager (large icons): project files and PES,
PEC, VP3, DST, HUS and JEF files show their pictures (on macOS and Linux
also EXP, XXX and others). Tell us if they do not.

Try the new embroidery fonts (**Lettering > Fonts**), and the fonts for
small text at 4 to 6 mm (**For small text**): tell us which sew well.

Turn on **View > Density Map** on your own designs: red shows where
layers of thread build up. Design info shows how much top and bobbin
thread a design uses; if you can, weigh a spool before and after a
sew-out and tell us the difference.

Also try **Ctrl+K** (Find a command) and, in **Options**, the text size
and high contrast. If you use a screen reader, tell us what it reads.

If you can, sew some of your results and take photos of the sew-out.

## What we want to know

Use the feedback form (`FEEDBACK FORM.txt`). Most useful:

- Each place where you did not know what to do next.
- Features that you use every day in other software and did not find here.
- Stitches that sew badly: send the project file (`.embproj`), the machine
  file and a photo of the sew-out.
- Anything that is slow.
- Errors and crashes (see below).

Send only designs that you are free to share.

## If the program stops

The program writes a crash report to the `crashes` folder in the
settings folder (for example `%APPDATA%\ShokujoDX\crashes`),
and tells you where it is at the next start. Please send that file with
your feedback. The report has no design data.

Your work is in the automatic backups: at the next start the program
offers to open it again.

**File > About** shows the version and the build. **Copy build
information** puts them on the clipboard for your report.

## What this build does not do yet

- No direct connection to machines over a network (Send to USB writes the
  file to a USB stick).
- No sequins, beads, chenille or Schiffli.
- The program is in English, German, Spanish and Japanese
  (Options > Language, or `--lang de`, `--lang es`, `--lang ja` on the
  command line); the help topics are in English only.

These are planned. Tell us which ones matter to you most.
