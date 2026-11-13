---
id: file-formats
title: Dateiformate
group: reference
keywords: format, datei, endung, lesen, schreiben, öffnen, importieren, exportieren, dst, vp3, pes, pec, exp, jef, xxx, sew, hus, shv, tbf, tap, u01, dsb, dsz, 10o, dat, embproj, svg, pdf, ai, emf, wmf, png, jpeg, bmp, tiff, gif, gpl, csv, dxf, zip
---
# Dateiformate

## Maschinendateien, die das Programm schreibt

| Format | Maschinen | Farben |
|---|---|---|
| DST | Tajima, die meisten Mehrnadelmaschinen | nein |
| VP3 | Husqvarna Viking, Pfaff | ja |
| PES | Brother, Babylock | ja |
| PEC | Brother (der Maschinenteil von PES) | ja |
| EXP | Melco, Bernina | nein |
| JEF | Janome | ja |
| XXX | Singer | ja |
| TBF | Tajima, mit Nadeln und Farben | ja |
| TAP | Happy | nein |
| U01 | Barudan, mit der Langsam-Funktion | nein |

Für ein Format ohne Farben kann **Design ausgeben** Garnfarbdateien (INF,
EDR, COL) neben die Maschinendatei schreiben. TBF- und U01-Dateien haben
die Nadel jeder Farbe (siehe **Design > Nadeln**); für DST, TAP und EXP
gibt das Arbeitsblatt die Nadelfolge an, die Sie an der Maschine
eintippen. DST und TAP haben keinen Code für den Fadenschnitt: siehe
**Design > Maschineneinstellungen**.

## Maschinendateien, die das Programm liest

**Datei > Öffnen** und **Stiche einfügen** lesen diese Formate, als
Stichobjekte:

DST, VP3, PES, PEC, EXP, JEF, XXX, SEW, HUS, SHV, TBF, TAP, DSB, DSZ,
10O, 100, U01, DAT, EXY, FXY, GT, STX, PHB, PHC, BRO, EMD, INB, JPX, KSM,
MAX, MIT, NEW, PCD, PCQ, PCS, PCM, SPX, STC, ZHS und ZXY.

Eine Maschinendatei hat Einstichpunkte, keine Formen. Um sie zu skalieren
und die Dichte zu behalten, benutzen Sie **Grafik > Umwandeln > Umrisse
erkennen**.

## Andere Dateien

| Datei | Lesen | Schreiben | Verwendung |
|---|---|---|---|
| EMBPROJ | ja | ja | Das Projekt dieses Programms: alle Objekte und Einstellungen. |
| SVG | ja | ja | Vektorgrafik hinein; Umrisse, das Designbild und Schneideplotter-Dateien hinaus. |
| PDF | ja | ja | Vektorgrafik hinein; Arbeitsblätter, Zählmuster und Vorlagen hinaus. |
| AI, EMF, WMF | ja | EMF | Vektorgrafik. |
| PNG, JPEG, BMP, TIFF, GIF | ja | PNG | Hintergrundbilder hinein; Designbilder hinaus. |
| DXF | nein | ja | Schneideplotter-Dateien für Applikationen und Strassvorlagen. |
| CSV | ja | ja | Garnkarten und Teamnamen hinein; Designeigenschaften und die Designliste hinaus. |
| GPL | ja | nein | Garnkarten (GIMP-Paletten). |
| EMBFONT, BDF, FON | ja | EMBFONT | Stickschriften und Alphabete; Bitmap-Schriften. |
| TTF, OTF | ja | nein | TrueType- und OpenType-Schriften des Computers. |
| ZIP | ja | ja | Ressourcenpakete; die Dateien von Per E-Mail senden. |

## Namen von Maschinen und Formaten

Die Namen von Maschinen, Dateiformaten und Garnmarken in diesem Programm
sind Marken ihrer Inhaber. Sie sagen nur, welche Dateien das Programm
liest und schreibt. Die Hersteller haben das Programm nicht getestet oder
freigegeben. Testen Sie jedes neue Design zuerst auf Reststoff.
