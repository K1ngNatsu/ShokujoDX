---
id: tutorial-vector-logo
title: "Anleitung: ein Vektorlogo"
group: tutorials
keywords: anleitung, vektor, vektoren, vektorgrafik, logo, svg, pdf, eps, ai, emf, wmf, importieren, grafik, grafik importieren, als vektoren importieren, verschmelzen, formen, schneiden, schnittmenge, umwandeln, zu füllung, zu satin, zu umriss, rand, rand hinzufügen, nachzeichnen, bild nachzeichnen, automatisch nachzeichnen
---
# Anleitung: ein Vektorlogo

In dieser Anleitung erstellen Sie Stickerei aus einem Vektorlogo: Sie
verschmelzen seine Formen zu einer Form, wandeln die Formen in Füllungen um
und fügen einen Rand hinzu. Dann zeichnen Sie ein Bild in Vektorformen
nach.

## Vektorgrafik

Eine Vektordatei hat Formen, keine Pixel: SVG, PDF, AI, EPS, EMF oder WMF.
Es gibt drei Wege, eine Vektordatei in das Programm zu bringen:

- **Datei > Öffnen** öffnet eine SVG-Datei als neues Design.
- **Grafik > Importieren > Grafik importieren** erstellt sofort Objekte mit
  Stichen: Füllungen, Satins und Umrisse.
- **Grafik > Importieren > Als Vektoren importieren** fügt die Formen als
  Vektorgrafik ohne Stiche hinzu. Bereinigen Sie zuerst die Formen, und
  wandeln Sie sie dann um. Diese Anleitung macht es so.

## Das Startdesign öffnen

Klicken Sie auf [Das Startdesign öffnen](sample:vector-logo). Es hat ein
Logo als Vektorgrafik, wie nach **Als Vektoren importieren**: eine Wolke
aus vier Formen, die sich überlappen (drei Kreise und ein Sockel), und
eine Sonne. Vektorgrafik zeigt sich in ihren Farben, ohne Stiche.

## Die Wolke verschmelzen

Formen, die sich überlappen, werden dort, wo sie sich überlappen, zweimal
gestickt. Verschmelzen Sie sie zu einer Form.

1. Ziehen Sie einen Rahmen um die Wolke. Die vier Formen der Wolke sind
   ausgewählt, nicht die Sonne.
2. Klicken Sie auf **Grafik > Formen > Verschmelzen**. Die Wolke ist eine
   Form.

![Die Wolke, zu einer Form verschmolzen](vector-welded)

Die anderen Befehle von **Grafik > Formen** schneiden Formen: **Vorne
minus hinten** schneidet die vordere Form aus der Form dahinter, und
**Schnittmenge** behält, wo sich die Formen überlappen.

## In Stiche umwandeln

1. Klicken Sie auf **Bearbeiten > Auswählen > Alles auswählen**.
2. Klicken Sie auf **Grafik > Umwandeln > Zu Füllung**. Die Wolke und die
   Sonne sind Füllungen, jede in der Farbe ihrer Form. (**Zu Satin**
   erstellt Satinsäulen aus schmalen Formen, und **Zu Umriss** erstellt
   Steppstiche um die Formen.)
3. Ziehen Sie einen Rahmen um die Wolke, um sie auszuwählen.
4. Klicken Sie auf **Grafik > Ränder und Konturen > Rand hinzufügen**, dann
   auf **Umrisse hinzufügen**. Die Wolke hat einen Satinrand.
5. Klicken Sie auf **Ansicht > Zeigen > Garnansicht**, und wählen Sie
   **Garnansicht** in ihrem Menü.

![Das Logo in Stichen](vector-stitches)

## Ein Bild nachzeichnen

Ein Logo, das ein Bild ist (PNG, JPEG), hat keine Formen. **Bild
nachzeichnen** erstellt eine Vektorform aus jeder Farbfläche des Bilds.

1. Klicken Sie auf [Das Blattlogo öffnen](sample:logo-backdrop). Das Bild
   eines Blatts ist das Hintergrundbild.
2. Klicken Sie auf **Grafik > Nachzeichnen > Bild nachzeichnen**. Jede
   Farbfläche des Bilds ist eine Vektorform in ihrer Farbe: drei Formen in
   drei Farben.

![Das Blatt, in Vektorformen nachgezeichnet](vector-traced)

3. Wandeln Sie die Formen wie oben um. Sehen Sie sich vor dem Umwandeln
   kleine Formen an: Löschen Sie die Formen, die Sie nicht sticken wollen.

Bei einem Bild mit vielen Farben macht **Grafik > Importieren > Bild zu
Design** alle Schritte (siehe [Anleitung: ein Aufnäher aus einem
Bild](tutorial-patch)).

## Wenn es nicht richtig aussieht

- **Die Sonne ist auch ausgewählt.** Der Rahmen wählt nur die Formen aus,
  die ganz in ihm sind. Ziehen Sie den Rahmen nur um die Wolke.
- **Eine Füllung wird zweimal gestickt, wo sich Formen überlappen.**
  Verschmelzen oder schneiden Sie die Formen, bevor Sie sie umwandeln.
- **Bild nachzeichnen erstellt viele kleine Formen.** Das Bild hat
  Rauschen oder weiche Kanten. Benutzen Sie ein sauberes Bild, oder löschen
  Sie die kleinen Formen.

## Weiter

- [Anleitung: eine Datei von einem Kunden](tutorial-customer-file)
- [Vektorgrafik und Formen](artwork): alle Befehle zum Importieren, Formen
  und Umwandeln
