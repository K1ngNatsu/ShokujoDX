---
id: tutorial-sketch
title: "Anleitung: ein Abzeichen im Skizzenmodus"
group: tutorials
keywords: anleitung, skizze, parametrisch, cad, maß, durchmesser, genaue größe, kreis, vieleck, sechseck, bereiche, abzeichen, bedingung, größe ändern, verknüpft
---
# Anleitung: ein Abzeichen im Skizzenmodus

In dieser Anleitung zeichnen Sie ein Abzeichen mit genauen Größen im
Skizzenmodus: zwei Kreise und ein Sechseck. Sie erstellen Stickerei aus
den Bereichen der Skizze und ändern dann eine Größe: Die Stiche folgen.

Eine Skizze ist eine Zeichnung mit Maßen und Bedingungen, wie in einem
CAD-Programm. Sie selbst stickt nichts. Siehe [Skizzen](sketch).

## Das Startdesign öffnen

Klicken Sie auf [Das Startdesign öffnen](sample:sketch-start). Es ist leer
und hat drei Garne: Navy, White und Gold.

## Eine Skizze beginnen

Klicken Sie auf **Digitalisieren > Skizze > Parametrische Skizze**. Der
Tab **Skizze** öffnet sich. Der Ursprung der Skizze ist ein Ring mit einem
Kreuz in der Mitte des Rahmens: Punkte rasten an ihm ein.

## Die Kreise zeichnen

1. Klicken Sie auf **Skizze > Erstellen > Kreis**.
2. Klicken Sie auf den Ursprung (die Mitte), dann auf einen Punkt 30 mm
   rechts davon.
3. Klicken Sie wieder auf den Ursprung, dann auf einen Punkt 25 mm rechts
   davon. Die zwei Kreise haben dieselbe Mitte: den Ursprung.

## Ein Sechseck zeichnen

1. Klicken Sie auf **Skizze > Erstellen > Vieleck** (klicken Sie auf den
   Pfeil neben **Rechteck**, dann auf **Vieleck**). Ein neues Vieleck hat
   sechs Seiten.
2. Klicken Sie auf den Ursprung, dann auf einen Punkt 12 mm rechts davon.

## Den Kreisen ihre Größe geben

1. Klicken Sie auf **Skizze > Bedingungen > Maß**.
2. Klicken Sie auf den äußeren Kreis, dann auf eine Stelle für das Maß,
   oben rechts. Ein Feld zeigt den jetzigen Durchmesser.
3. Geben Sie `60` ein, und drücken Sie die Eingabetaste. Der Kreis ist
   60 mm breit.
4. Klicken Sie auf den inneren Kreis, dann auf eine Stelle unten rechts.
   Geben Sie `52` ein, und drücken Sie die Eingabetaste.
5. Drücken Sie Esc, um zu **Auswählen** zurückzugehen.

![Die Skizze mit ihren zwei Maßen](sketch-dimensions)

Sie können auch andere Einheiten (`2 in`) oder eine Rechnung (`60 - 8`)
eingeben.

## Stickerei aus den Bereichen erstellen

Die Kurven bilden drei geschlossene Bereiche: den Ring zwischen den
Kreisen, die Scheibe zwischen dem inneren Kreis und dem Sechseck, und das
Sechseck.

1. Klicken Sie auf **Skizze > Skizze > Bereiche**.
2. Klicken Sie in der Leiste **Garne** auf die Farbe **Navy**. Klicken Sie
   in die Scheibe (zwischen dem Sechseck und dem inneren Kreis), dann im
   Bereich Eigenschaften unter **Stickerei erstellen** auf **Füllung**.
3. Klicken Sie auf **White**. Klicken Sie in das Sechseck, dann auf
   **Füllung**.
4. Klicken Sie auf **Gold**. Klicken Sie in den Ring, dann auf **Satin**.
5. Klicken Sie auf **Skizze > Beenden > Skizze beenden**.

Das Abzeichen hat jetzt eine marineblaue Füllung, eine weiße Füllung und
einen goldenen Satinring. Sie werden in der Reihenfolge gestickt, in der
Sie sie erstellt haben.

## Eine Größe ändern

Die Objekte bleiben mit der Skizze verknüpft.

1. Klicken Sie auf den goldenen Ring. Klicken Sie im Bereich Eigenschaften
   auf **Skizze bearbeiten**.
2. Klicken Sie mit der rechten Maustaste auf das Maß des äußeren Kreises
   (60), und klicken Sie auf **Wert ändern**.
3. Geben Sie `66` ein, und drücken Sie die Eingabetaste. Der Ring ist
   breiter, und seine Stiche folgen.
4. Klicken Sie auf **Skizze > Beenden > Skizze beenden**.

![Das Abzeichen nach der Änderung, mit der 3D-Garnansicht](sketch-stitches)

Rückgängig nimmt die Änderung der Skizze und ihrer Objekte in einem
Schritt zurück.

## Wenn es nicht richtig aussieht

- **Die Kreise haben nicht dieselbe Mitte.** Klicken Sie genau auf den
  Ursprung: Der Zeiger zeigt eine Markierung, wenn er einrastet.
- **Ein Bereich wird nicht schattiert.** Der Bereich ist nicht
  geschlossen: Die Kurven müssen sich treffen oder kreuzen.
- **Das neue Maß steht in Klammern.** Die Skizze legt diese Größe schon
  fest: Das Maß ist abhängig und ändert nichts.
- **Das Abzeichen ändert sich nicht.** Ändern Sie die Skizze (Skizze
  bearbeiten), nicht die Objekte: Ein erstelltes Objekt skaliert nicht
  allein.

## Weiter

- [Anleitung: ein Skript, das die Arbeit macht](tutorial-script)
- [Skizzen](sketch): Bedingungen, Parameter und weitere Werkzeuge
