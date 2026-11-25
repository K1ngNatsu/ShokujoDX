---
id: tutorial-script
title: "Anleitung: ein Skript, das die Arbeit macht"
group: tutorials
keywords: anleitung, skript, skripte, rhai, automatisierung, makro, vor der ausgabe prüfen, satindichte, abstand, alle satins, ein schritt rückgängig, meine skripte, werkzeug
---
# Anleitung: ein Skript, das die Arbeit macht

In dieser Anleitung führen Sie zwei Beispielskripte auf dem Vereinswappen
aus: Eines prüft das Design vor der Ausgabe, und eines gibt allen seinen
Satinsäulen in einem Schritt dieselbe Dichte. Dann sehen Sie, wie Sie ein
eigenes Skript erstellen.

## Das Startdesign öffnen

Klicken Sie auf [Das Vereinswappen öffnen](sample:club-crest). Seine zwei
Satinsäulen, der goldene Ring und der rote Winkel, haben verschiedene
Abstände.

## Das Design vor der Ausgabe prüfen

1. Klicken Sie auf **Bearbeiten > Skripte > Skripte**. Das Fenster
   **Skripte** öffnet sich.
2. Wählen Sie in der Liste **Skript** den Eintrag **Check Before Output**.
   Das Fenster zeigt, was das Skript macht, und seine Einstellungen: die
   Grenzen Ihres Betriebs.
3. Stellen Sie **Most colors** auf 3.
4. Klicken Sie auf **Skript ausführen**. Unter **Meldungen** sagt das
   Skript, dass das Design 4 Farben hat: mehr als 3.

Dieses Skript ändert nichts. Führen Sie es aus, bevor Sie die
Maschinendatei erstellen.

## Dieselbe Dichte für alle Satins

1. Wählen Sie in der Liste **Skript** den Eintrag **Same Density for All
   Satins**.
2. Stellen Sie **Spacing (mm)** auf 0,4.
3. Klicken Sie auf **Skript ausführen**. Die Meldung sagt, dass 2
   Satinsäulen jetzt einen Abstand von 0,4 mm haben.

![Das Fenster Skripte nach dem Skript](script-window)

Wenn nichts ausgewählt ist, arbeitet ein Skript am ganzen Design. Wählen
Sie zuerst Objekte aus, um nur sie zu ändern.

## Ein Schritt zum Rückgängigmachen

Klicken Sie auf **Rückgängig**: Die zwei Satinsäulen bekommen ihre
Abstände von vorher. Alle Änderungen eines Skripts sind ein Schritt.
Klicken Sie auf **Wiederherstellen**, um sie wieder auszuführen.

Wenn ein Skript mit einem Fehler anhält, ändert sich das Design nicht.

## Ein eigenes Skript

Ein Skript ist eine Textdatei in der Sprache Rhai, mit der Endung `.rhai`.

1. Wählen Sie ein Beispielskript, und klicken Sie auf **In meine Skripte
   kopieren**. Das Programm legt eine Kopie in Ihren Skriptordner.
2. Klicken Sie auf **Skriptordner öffnen**, und ändern Sie die Kopie in
   einem Texteditor. Die ersten Zeilen, die mit `//!` beginnen, sind sein
   Titel, seine Beschreibung und seine Einstellungen.
3. Klicken Sie auf den runden Pfeil neben der Liste **Skript**, um den
   Ordner neu zu lesen.

Ein Skript kann nur das Design ändern: Es kann keine Dateien öffnen oder
speichern und nicht das Netzwerk benutzen. [Skripte](scripts) listet die
Funktionen.

## Wenn es nicht richtig aussieht

- **Das Skript hat nichts geändert.** Lesen Sie die Meldungen: Die
  Auswahl war vielleicht nicht, was das Skript braucht. Wählen Sie nichts
  aus, um am ganzen Design zu arbeiten.
- **Das Skript hat mit einem Fehler angehalten.** Der rote Text sagt,
  warum; das Design hat sich nicht geändert.
- **Mein Skript ist nicht in der Liste.** Klicken Sie auf den runden
  Pfeil, um den Ordner neu zu lesen. Die Datei muss auf `.rhai` enden.

## Weiter

- [Skripte](scripts): die Funktionen eines Skripts
- [Anleitung: ein Abzeichen im Skizzenmodus](tutorial-sketch)
