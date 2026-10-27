---
id: file-formats
title: File formats
group: reference
keywords: format, file, extension, read, write, open, import, export, dst, vp3, pes, pec, exp, jef, xxx, sew, hus, shv, tbf, tap, u01, dsb, dsz, 10o, dat, embproj, svg, pdf, ai, emf, wmf, png, jpeg, bmp, tiff, gif, gpl, csv, dxf, zip
---
# File formats

## Machine files the program writes

| Format | Machines | Colors |
|---|---|---|
| DST | Tajima, most multi-needle machines | no |
| VP3 | Husqvarna Viking, Pfaff | yes |
| PES | Brother, Babylock | yes |
| PEC | Brother (the machine part of PES) | yes |
| EXP | Melco, Bernina | no |
| JEF | Janome | yes |
| XXX | Singer | yes |
| TBF | Tajima, with needles and colors | yes |
| TAP | Happy | no |
| U01 | Barudan, with the slow function | no |

For a format with no colors, **Output Design** can write thread color
files (INF, EDR, COL) next to the machine file. TBF and U01 files have the
needle of each color (see **Design > Needles**); for DST, TAP and EXP the
worksheet gives the needle sequence to type on the machine. DST and TAP have no trim
code: see **Design > Machine Settings**.

## Machine files the program reads

**File > Open** and **Insert Stitches** read these formats, as stitch
objects:

DST, VP3, PES, PEC, EXP, JEF, XXX, SEW, HUS, SHV, TBF, TAP, DSB, DSZ,
10O, 100, U01, DAT, EXY, FXY, GT, STX, PHB, PHC, BRO, EMD, INB, JPX, KSM,
MAX, MIT, NEW, PCD, PCQ, PCS, PCM, SPX, STC, ZHS and ZXY.

A machine file has needle points, not shapes. To scale it and keep the
density, use **Artwork > Convert > Recognize Outlines**.

## Other files

| File | Read | Write | Use |
|---|---|---|---|
| EMBPROJ | yes | yes | The project of this program: all objects and settings. |
| SVG | yes | yes | Vector artwork in; outlines, the design picture and cutter files out. |
| PDF | yes | yes | Vector artwork in; worksheets, charts and templates out. |
| AI, EMF, WMF | yes | EMF | Vector artwork. |
| PNG, JPEG, BMP, TIFF, GIF | yes | PNG | Backdrop pictures in; design pictures out. |
| DXF | no | yes | Appliqué cutter files and rhinestone templates. |
| CSV | yes | yes | Thread charts and team names in; design properties and the design list out. |
| GPL | yes | no | Thread charts (GIMP palettes). |
| EMBFONT, BDF, FON | yes | EMBFONT | Embroidery fonts and alphabets; bitmap fonts. |
| TTF, OTF | yes | no | TrueType and OpenType fonts of the computer. |
| ZIP | yes | yes | Asset packages; the files of Send by email. |

## Names of machines and formats

The names of machines, file formats and thread brands in this program are
the trademarks of their owners. They only tell which files the program
reads and writes. The makers did not test or approve the program. Test
each new design on scrap fabric first.
