---
id: tutorial-customer-file
title: "Anleitung: eine Datei von einem Kunden"
group: tutorials
keywords: anleitung, kunde, kundin, maschinendatei, stichdatei, dst, pes, exp, jef, vp3, öffnen, erkennen, objekte erkennen, stiche, objekte, größe, größe ändern, skalieren, größer, kleiner, garnfarben, umrisse, schreiben, exportieren
---
# Anleitung: eine Datei von einem Kunden

In dieser Anleitung öffnen Sie eine Maschinendatei von einem Kunden,
erstellen mit **Objekte erkennen** Objekte aus ihren Stichen, ändern ihre
Größe und schreiben sie wieder.

Eine Maschinendatei (DST, PES, EXP, JEF, VP3 und andere) hat nur Stiche:
keine Formen und keine Einstellungen. Wenn Sie sie als Stiche größer
machen, werden die Stiche länger, und die Dichte wird geringer. Objekte
haben ihre Formen und Einstellungen, darum erstellt das Programm in der
neuen Größe neue Stiche.

## Die Datei öffnen

Klicken Sie auf [Das Startdesign öffnen](sample:customer-dst). Es ist die
Datei **LEAF.DST**, wie **Datei > Öffnen** sie öffnet: ein Stichobjekt für
jede Farbe. Eine DST-Datei hat keine Garnfarben: Das Programm gibt ihr die
Farben seiner Standardliste. Stellen Sie die Garnfarben ein, die der Kunde
wünscht (die Leiste Garne). Das Startdesign zeigt die Stiche auf einem
weißen Shirt (**Farbe > Stofffarbe**), weil die erste Farbe der Datei
Schwarz ist.

![Die Stiche der Datei](customer-file)

## Objekte erkennen

1. Klicken Sie auf **Bearbeiten > Auswählen > Alles auswählen**.
2. Klicken Sie auf **Grafik > Umwandeln > Objekte erkennen**. Das Fenster
   zeigt die Datei und die Objekte, die das Programm in ihren Stichen
   gefunden hat: die Füllung des Blatts (mit ihrem Loch), den Satin des
   Stiels, den Steppstich der Ader und die Teile der Unterlage der
   Füllung.

![Das Fenster Objekte erkennen](customer-recognize)

3. Sehen Sie sich die Liste an. Ein Teil, den das Programm nicht erkennt,
   bleibt Stiche. Entfernen Sie den Haken bei einem Teil, damit seine
   Stiche bleiben, wie sie sind.
4. Klicken Sie auf **Übernehmen**. Die Objekte ersetzen die Stichobjekte,
   in einem Rückgängig-Schritt. Die Statuszeile sagt, was das Programm
   erstellt hat: 5 Objekte, 4 mit einer Stichart und 1, das Stiche bleibt.

Wenn der Kunde auch die Grafik geschickt hat (SVG, PDF, AI, EPS, EMF oder
WMF), klicken Sie vor **Übernehmen** auf **Umrisse…**: Die Objekte
bekommen die Formen der Grafik, die genauer sind als die Formen aus den
Stichen.

## Die Größe ändern

1. Klicken Sie auf **Bearbeiten > Auswählen > Alles auswählen**.
2. Behalten Sie in den Objekteigenschaften unter **Position und Größe** die
   Einstellung **Proportionen beibehalten**, und setzen Sie **W** (die
   Breite) auf 80 mm. Das Programm erstellt die Stiche in der neuen Größe
   neu, mit derselben Dichte.

![Das Blatt, größer](customer-larger)

## Die Datei wieder schreiben

Klicken Sie auf **Design > Ausgabe > Exportieren**, dann auf
**Exportieren…**, und wählen Sie das Format der Maschine des Kunden.
Speichern Sie auch das Projekt (**Speichern**): Es behält die Objekte,
damit Sie das Design später wieder ändern können.

## Wenn es nicht richtig aussieht

- **Objekte erkennen ist grau.** Wählen Sie zuerst die Stichobjekte aus.
- **Ein Teil bleibt Stiche.** Das Programm hat darin keine Form gefunden
  (zum Beispiel eine Füllung mit einem Muster). Behalten Sie ihn als
  Stiche, und machen Sie das Design nicht viel größer oder kleiner.
- **Die Farben sind falsch.** Eine DST-Datei hat keine Farben. Klicken Sie
  in der Leiste Garne auf ein Garn, und stellen Sie seine Farbe ein.

## Weiter

- [Anleitung: eine Applikation](tutorial-applique)
- [Dateien und Ausgabe](files-and-output): die Dateiformate und Objekte
  erkennen
