---
id: colors-and-threads
title: Farben und Garne
group: production
keywords: farbe, garn, garnkarte, marke, madeira, isacord, sulky, farbvariante, hintergrund, stofffarbe, palette, anpassen, farbe ändern, farbkreis, komplementär, rotieren, garn trennen, unbenutzte entfernen, karte importieren, gpl, csv, garnverbrauch, unterfaden, kosten, spule
---
# Farben und Garne

## Der Bereich Garne

**Farbe > Garne** zeigt den Bereich Garne.

- **Designgarne** listet die Garne des Designs auf. Klicken Sie auf die
  Farbe eines Garns, um sie zu ändern. Klicken Sie auf seinen Namen, um es
  zum Garn neuer Objekte zu machen.
- Um einem Objekt ein anderes Garn zu geben, wählen Sie das Objekt aus und
  klicken Sie auf das Garn. Oder klicken Sie mit der rechten Maustaste auf
  das Objekt und benutzen Sie **Garn**. Die Objekteigenschaften haben das
  Garn auch.
- **Garn hinzufügen** fügt ein Garn hinzu.
- **Unbenutzte entfernen** entfernt die Garne, die kein Objekt benutzt.
- **Farben rotieren**: Jedes benutzte Garn nimmt die Farbe des nächsten an.
  Wiederholen Sie es, um weitere Kombinationen zu probieren.
- **Garn trennen**: gibt der Auswahl ein eigenes Garn, damit sich ihre
  Farbe allein ändern kann.

## Garnkarten

Eine Garnkarte ist die Liste der Farben einer Garnmarke. Das Programm hat
die Karten von 75 Marken und Garnarten (Brother, Madeira, Isacord, Sulky,
Robison-Anton, Gunold, Marathon, Floriani und mehr).

- Wählen Sie eine Karte unter **Farbe > Garnkarte** oder im Bereich Garne.
- **Suchen** findet ein Garn nach seinem Namen oder seiner Nummer.
- Klicken Sie auf ein Garn der Karte, um dem aktuellen Garn seine Farbe und
  seinen Namen zu geben. **Strg+Klick** fügt es als neues Garn hinzu.
- **An Karte anpassen** gibt jedem benutzten Garn die nächstliegende Farbe
  der Karte.
- **Karte importieren** liest eine Karte aus einer GIMP-Palette (`.gpl`)
  oder einer CSV-Datei (`code,name,#rrggbb`). Die Karte bleibt für den
  nächsten Start.

Bild zu Design kann seine Farben aus der Karte nehmen: siehe [Bilder und
Bild zu Design](pictures).

## Farbvarianten

Eine Farbvariante ist ein Satz von Farben für das Design. Erstellen Sie
weitere Farbvarianten, um dasselbe Design in anderen Farben zu sticken,
zum Beispiel für Stoffe anderer Farben.

- **Neue Farbvariante** erstellt eine Kopie der aktuellen Farben, zum
  Ändern.
- **Aktuell** (Tab Farbe) oder ein Klick im Bereich Garne zeigt eine
  Farbvariante.
- **Farbvariante löschen** löscht die aktuelle Farbvariante. Die Pfeile im
  Bereich Garne ändern die Reihenfolge der Farbvarianten.
- Die Stofffarbe ist Teil der Farbvariante.

Exportieren und Design ausgeben schreiben die aktuelle Farbvariante.

## Die Stofffarbe

**Farbe > Stofffarbe** legt den **Hintergrund** fest: die Farbe des Stoffs
im Rahmen. Die Arbeitsfläche, die Garnansicht und die Bilder zeigen den
Stoff in dieser Farbe. **Kein Hintergrund** entfernt sie. Die 3D-Garnansicht
zeigt das Gewebe des Stoffs des Designs (Design > Stoff) unter der Farbe.

## Der Farbkreis

Öffnen Sie **Farbkreis** im Bereich Garne. Er zeigt die Farben des Designs
auf einem Kreis. Klicken Sie auf einen Punkt, um ihn zum aktuellen Garn zu
machen. Unter dem Kreis sind Farben, die zum aktuellen Garn passen (die
Gegenfarbe, die Farben daneben und andere), als Garne der Karte. Klicken
Sie auf eine, um sie für das aktuelle Garn zu benutzen; Strg+Klick fügt
sie als neues Garn hinzu.

## Wie viel Garn ein Design braucht

Die **Designinfo** zeigt den Oberfaden jeder Farbe und den Unterfaden des
Designs. Das Arbeitsblatt zeigt sie auch. Die Zahlen sind eine Schätzung
aus den Stichen und der Dicke des Stoffs des Designs (Design > Stoff). Die
Spannung, das Garn und der Stoff ändern sie.

Um die Kosten des Garns zu sehen, legen Sie in den **Optionen** die Länge
und den Preis einer Garnrolle und einer Unterfadenspule und die Währung
fest. Ein Preis von 0 zeigt keine Kosten.

## Tipps

- Formate wie DST haben keine Farben. Die Maschine zeigt ihre eigenen
  Farben. **Design ausgeben** kann Garnfarbdateien neben die Maschinendatei
  schreiben. Siehe [Dateien und Ausgabe](files-and-output).
- Halten Sie die Objekte einer Farbe in der Stickreihenfolge zusammen,
  damit die Maschine das Garn seltener wechselt. Siehe [Anordnen und die
  Stickreihenfolge](arranging).
