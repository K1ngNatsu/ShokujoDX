---
id: files-and-output
title: Dateien und Ausgabe
group: production
keywords: öffnen, speichern, projekt, embproj, sicherung, wiederherstellen, absturz, importieren, exportieren, maschinendatei, format, dst, pes, vp3, usb, stick, senden, e-mail, zip, design ausgeben, arbeitsblatt, freigabeblatt, pdf, drucken, bild, png, svg, csv, barcode, garnfarbdateien, inf, edr, col, stapel, warteschlange, exporte, vorschaubild, symbol, explorer, finder
---
# Dateien und Ausgabe

## Projekte und Maschinendateien

- Ein **Projekt** (`.embproj`) behält alle Objekte und Einstellungen.
  **Datei > Speichern** (Strg+S) speichert es; **Speichern unter**
  speichert es mit einem neuen Namen.
- Eine **Maschinendatei** (DST, PES, VP3 und andere) hat nur die Stiche,
  die Farben (in den meisten Formaten) und die Maschinenfunktionen.
  Behalten Sie das Projekt, um Änderungen zu machen.
- **Datei > Öffnen** öffnet ein Projekt, oder eine Maschinendatei als
  Stichobjekte. Das Programm liest etwa 40 Maschinenformate; siehe
  [Dateiformate](file-formats).
- **Datei > Zuletzt verwendete Designs** öffnet ein Design, das Sie schon
  benutzt haben.
- **Datei > Schließen** und **Alle schließen** schließen Designs. Das
  Programm fragt zuerst bei jedem Design mit Änderungen.

## Sicherungen

Das Programm speichert alle 2 Minuten eine Sicherung jedes Designs mit
Änderungen (**Optionen > Automatische Sicherung**). Wenn das Programm
stoppt, zeigt das Fenster **Designs wiederherstellen** die Sicherungen
beim nächsten Start. Ein Absturzbericht kommt in den Ordner `crashes` der
Einstellungen; **Hilfe > Info** zeigt, wo er ist.

## Eine Maschinendatei exportieren

**Design > Exportieren** (Strg+E, oder Datei > Maschinendatei
exportieren) schreibt die Stiche für die Maschine. Das Fenster zeigt das
**Format** (das Format der Maschine ist der Standard), den Rahmen, die
Größe, die Stiche, Farben, Fadenschnitte und Stopps, die Farbvariante und
die Stickzeit.

| Format | Maschinen |
|---|---|
| DST | Tajima, die meisten Mehrnadelmaschinen |
| VP3 | Husqvarna Viking, Pfaff |
| PES | Brother, Babylock |
| PEC | Brother (der Maschinenteil von PES) |
| EXP | Melco, Bernina |
| JEF | Janome |
| XXX | Singer |
| TBF | Tajima, mit Nadeln und Farben |
| TAP | Happy |
| U01 | Barudan, mit der Langsam-Funktion |

## Auf USB senden

- **Design > Auf USB senden** schreibt das Design im Format der Maschine
  auf den USB-Stick der Maschine und liest es zur Prüfung zurück. Wählen
  Sie beim ersten Mal den Stick und den Ordner.
- **Senden an…** wählt den Stick und den Ordner neu. Das Programm behält
  sie für jede Maschine.

## Per E-Mail senden

**Design > Per E-Mail senden** legt die Maschinendatei, das Projekt und
ein Bild in eine ZIP-Datei neben dem Design und öffnet das Mailprogramm.
Hängen Sie die ZIP-Datei an die Nachricht an.

## Design ausgeben

**Design > Design ausgeben** macht alle Dateien der Produktion in einem
Schritt, in einem Ordner:

- die **Maschinendateien** (ein oder mehr Formate) und die Projektdatei;
- das **Produktions-Arbeitsblatt** und das **Freigabeblatt** (PDF), mit
  einem **Barcode auf dem Arbeitsblatt**, wenn Sie einen wollen, und der
  **Papier**-Größe;
- das **Designbild** (PNG, mit der 3D-Garnansicht, wenn Sie wollen) und
  (SVG);
- die **Designeigenschaften** (CSV);
- **Garnfarbdateien** (INF, EDR, COL) für DST und die anderen Formate
  ohne Farben;
- die **Strassvorlage** (SVG, PDF), für ein Design mit Steinen.

Das Programm behält Ihre Wahl für das nächste Mal. Die Arbeit kommt in
die Warteschlange; die Gruppe **Exporte** zeigt sie, damit Sie
weiterarbeiten können. Siehe [Anleitung: Ausgabe für die
Produktion](tutorial-output).

## Das Arbeitsblatt und das Freigabeblatt

**Design > Druckvorschau** zeigt das **Produktions-Arbeitsblatt** und das
**Freigabeblatt**, Seite für Seite. **Notizen für die Arbeitsblätter**
fügt Text hinzu, zum Beispiel den Auftrag, den Kunden, den Stoff und die
Nadel. **PDF speichern** schreibt die Blätter; zum Drucken drucken Sie
das PDF aus einem PDF-Programm.

Das Arbeitsblatt hat die Größe, die Stiche, die Stickzeit, den Rahmen und
die Stelle des Designs, jede Farbe und jeden Stopp, den Stoff, das
Stickvlies und das Deckvlies, ein Bild und die Arbeitsreihenfolge für
Strasssteine.

## Stapelausgabe

**Design > Stapelausgabe** macht die Dateien von Design ausgeben für
viele Projekt- und Maschinendateien auf einmal. Die Designbibliothek kann
die ausgewählten Designs auch ausgeben.

## Andere Dateien

- **Design > SVG exportieren**: das Designbild als SVG, die Stiche als
  Linien in den Garnfarben, in echter Größe.
- **Grafik > Umrisse exportieren**: die Formen als SVG- oder EMF-Pfade.
- **Digitalisieren > Schneideplotter-Datei**: die Schnittlinien der
  Applikationsteile (SVG, DXF).
- **Spezial > Vorlage**: die Strassvorlage (SVG, DXF, PDF).
- **Spezial > Zählmuster**: das Kreuzstich-Zählmuster (PDF).
- **Design > Rahmenschablone**: das Rahmenfeld in voller Größe (PDF).

## Bilder im Dateimanager

Nach der Installation zeigt der Dateimanager (Explorer, Finder, der
Dateimanager von Linux) das Bild jeder Designdatei als ihr Symbol:
Projekte und Maschinendateien.

## Tipps

- Prüfen Sie das Format Ihrer Maschine (die Liste Maschine im Tab
  Design). Die meisten Heimmaschinen lesen nur ihr eigenes Format.
- Prüfen Sie den Ordner auf dem USB-Stick. Auf USB senden benutzt den
  Ordner Ihrer Maschine.
- Siehe [Fragen und Probleme](troubleshooting), wenn die Maschine die
  Datei nicht liest.
