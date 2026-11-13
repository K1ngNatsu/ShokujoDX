---
id: thread-stock
title: Fadenlager
group: production
keywords: fadenlager, garnlager, bestand, inventar, spulen, fäden auf lager, kaufen, garn bestellen, nachbestellen, mindestens, minimum, einkaufsliste, farbabgleich, nächstes garn, delta e
---
# Fadenlager

**Design > Fadenlager** behält die Garne, die Sie haben. Das Programm
kann dann Farben nur mit diesen Garnen abgleichen, die Garne eines Designs
in die nächsten Garne auf Lager ändern und die Garne auflisten, die Sie
für einen Auftrag kaufen müssen.

## Ihre Garne hinzufügen

- **Aus einer Farbkarte hinzufügen**: Wählen Sie eine Garnkarte, tippen
  Sie einen Namen oder Code ein und klicken Sie neben einem Garn auf
  **Hinzufügen**. Jeder Klick fügt eine Spule hinzu.
- **CSV importieren**: eine Datei mit den Spalten `brand,code,name,color,spools,spool_m,min_spools`
  (die Farbe als `#rrggbb`). Die letzten drei Spalten können fehlen: eine
  Spule, die Spulenlänge der Optionen, keine Mindestzahl. **CSV
  exportieren** schreibt dieselben Spalten, zum Beispiel um das Lager auf
  einen anderen Computer zu bringen.

Legen Sie in der Liste für jedes Garn fest:

- **Spulen**: die Spulen, die Sie haben. Zählen Sie eine angefangene Spule
  als eine.
- **Spulenlänge**: das Garn auf einer Spule. 0 benutzt die Länge der
  Garnpreise in den Optionen.
- **Mindestens**: die kleinste Zahl von Spulen, die Sie behalten wollen.
  Darunter kommt das Garn auf die Liste der zu kaufenden Fäden.

## Nur Fäden auf Lager

**Nur Fäden auf Lager** (im Fenster Fadenlager und in der Gruppe Garne)
macht das Lager zur Garnkarte: Die Kartenliste, **An Karte anpassen**,
die Bildwerkzeuge und **Bild zu Design** benutzen dann nur die Garne, die
Sie haben. Schalten Sie es aus, um zur Karte zurückzugehen, die vorher
angezeigt wurde.

Die Gruppe Garne markiert ein Garn auf Lager mit einem grünen Punkt (●).

## Garne dieses Designs

Das Fenster listet jedes verschiedene Garn des Designs auf: auf Lager (mit
den Spulen), oder nicht auf Lager mit dem nächsten Garn auf Lager und dem
Farbunterschied (ΔE). Unter etwa 2,3 sieht ein Mensch den Unterschied
nicht; über etwa 10 sind die Farben deutlich verschieden.

**Fäden auf Lager verwenden** gibt jedem benutzten Garn, das nicht auf
Lager ist, das nächste Garn auf Lager. Rückgängig nimmt alle Änderungen in
einem Schritt zurück.

## Zu kaufende Fäden

Für die Menge des Auftrags (**Design > Auftragsdetails**) vergleicht das
Programm das Obergarn, das der Auftrag braucht (aus der Designinfo), mit
dem Lager. Ein Garn ist auf der Liste, wenn es nicht auf Lager ist, wenn
der Auftrag mehr braucht, als Sie haben, oder wenn nach dem Auftrag
weniger Spulen als die Mindestzahl übrig sind. Die Liste gibt die Spulen
an, die Sie kaufen müssen. Das Unterfadengarn ist nicht auf der Liste.

Die Liste ist im Fenster Fadenlager (**Einkaufsliste der Fäden
speichern** schreibt sie als CSV), die Designinfo zeigt die Zahl der zu
kaufenden Fäden, und das Produktions-Arbeitsblatt listet sie auf.
