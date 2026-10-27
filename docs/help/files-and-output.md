---
id: files-and-output
title: Files and output
group: production
keywords: open, save, project, embproj, backup, recover, crash, import, export, machine file, format, dst, pes, vp3, usb, stick, send, email, zip, output design, worksheet, approval sheet, pdf, print, picture, png, svg, csv, barcode, thread color files, inf, edr, col, batch, queue, exports, thumbnail, icon, explorer, finder
---
# Files and output

## Projects and machine files

- A **project** (`.embproj`) keeps all objects and settings. **File >
  Save** (Ctrl+S) saves it; **Save As** saves it with a new name.
- A **machine file** (DST, PES, VP3 and others) has only the stitches,
  the colors (in most formats) and the machine functions. Keep the
  project to make changes.
- **File > Open** opens a project, or a machine file as stitch objects.
  The program reads about 40 machine formats; see [File
  formats](file-formats).
- **File > Recent designs** opens a design that you used before.
- **File > Close** and **Close All** close designs. The program asks
  first for each design with changes.

## Backups

The program saves a backup of each design with changes every 2 minutes
(**Options > Automatic backup**). If the program stops, the **Recover Designs** window shows the backups at the next start. A crash report goes
to the `crashes` folder of the settings; **Help > About** shows where it
is.

## Export a machine file

**Design > Export** (Ctrl+E, or File > Export machine file) writes the
stitches for the machine. The window shows the **Format** (the format of
the machine is the default), the hoop, the size, the stitches, colors,
trims and stops, the colorway and the sew time.

| Format | Machines |
|---|---|
| DST | Tajima, most multi-needle machines |
| VP3 | Husqvarna Viking, Pfaff |
| PES | Brother, Babylock |
| PEC | Brother (the machine part of PES) |
| EXP | Melco, Bernina |
| JEF | Janome |
| XXX | Singer |
| TBF | Tajima, with needles and colors |
| TAP | Happy |
| U01 | Barudan, with the slow function |

## Send to USB

- **Design > Send to USB** writes the design in the format of the
  machine to the USB stick of the machine, and reads it back to check it.
  The first time, choose the stick and the folder.
- **Send To…** chooses the stick and the folder again. The program keeps
  them for each machine.

## Send by email

**Design > Send by Email** puts the machine file, the project and a
picture in one zip file next to the design, and opens the mail program.
Attach the zip file to the message.

## Output design

**Design > Output Design** makes all the files of production in one step,
in a folder:

- the **Machine files** (one or more formats) and the project file;
- the **Production worksheet** and the **Approval sheet** (PDF), with a
  **Barcode on the worksheet** if you want one, and the **Paper** size;
- the **Design picture** (PNG, with the 3D thread look if you want) and
  (SVG);
- the **Design properties** (CSV);
- **Thread color files** (INF, EDR, COL) for DST and the other formats
  with no colors;
- the **Rhinestone template** (SVG, PDF), for a design with stones.

The program keeps your choice for the next time. The work goes in the
queue; the **Exports** panel shows it, so you can go on working. See
[Tutorial: production output](tutorial-output).

## The worksheet and the approval sheet

**Design > Print Preview** shows the **Production worksheet** and the
**Approval sheet**, page by page. **Notes for the worksheets** adds text,
for example the job, the customer, the fabric and the needle. **Save
PDF** writes the sheets; to print, print the PDF from a PDF viewer.

The worksheet has the size, the stitches, the sew time, the hoop and the
place of the design, each color and stop, the fabric, backing and topping,
a picture, and the order of work for rhinestones.

## Batch output

**Design > Batch Output** makes the files of Output design for many
project and machine files at once. The design library can output the
selected designs too.

## Other files

- **Design > Export SVG**: the design picture as SVG, the stitches as
  lines in the thread colors, at real size.
- **Artwork > Export Outlines**: the shapes as SVG or EMF paths.
- **Digitize > Cutter File**: the cut lines of the appliqué pieces (SVG,
  DXF).
- **Specialty > Template**: the rhinestone template (SVG, DXF, PDF).
- **Specialty > Chart**: the cross-stitch chart (PDF).
- **Design > Hoop Template**: the hoop field at full size (PDF).

## Pictures in the file manager

After the install, the file manager (Explorer, Finder, the file manager
of Linux) shows the picture of each design file as its icon: projects and
machine files.

## Tips

- Check the format of your machine (the Machine list on the Design tab).
  Most home machines read only their own format.
- Check the folder on the USB stick. Send to USB uses the folder of your
  machine.
- See [Questions and problems](troubleshooting) if the machine does not
  read the file.
