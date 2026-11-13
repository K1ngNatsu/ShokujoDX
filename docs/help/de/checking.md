---
id: checking
title: Ein Design prüfen
group: production
keywords: prüfen, vorschau, stich-player, abspielen, stickablauf, simulieren, stichcursor, verbindungsstiche, stichliste, funktionen, designinfo, warnungen, dichtekarte, lagen, aufbau, garnansicht, 3d, licht, einstichpunkte, sprünge, fadenschnitte, produkt, t-shirt, mockup, wiederholungen, endlos, freigabe, stickzeit, stichzahl
---
# Ein Design prüfen

Prüfen Sie jedes Design, bevor Sie es sticken. Das Programm zeigt, wie die
Maschine es stickt, und warnt Sie vor häufigen Fehlern.

## Der Stich-Player

**Ansicht > Stich-Player** zeigt eine Leiste unter der Arbeitsfläche. Sie
spielt den Stickablauf ab: Die Stiche erscheinen so, wie die Maschine sie
stickt.

- **Stickablauf abspielen** und **Pause**, und das Tempo.
- **An den Anfang**, **Eine Farbe zurück**, **Eine Farbe vor**, **Ans
  Ende**.
- Ziehen Sie die Leiste, um sich durch den Stickablauf zu bewegen.

## Der Stichcursor

**Ansicht > Stichcursor** markiert einen Stich. Die Stiche danach sind
blass. Bewegen Sie ihn mit den Tasten:

- **,** und **.**: einen Stich zurück oder vor (**Shift**: 10, **Strg**:
  100, **Strg+Shift**: 1000).
- **Bild auf**, **Bild ab**: das vorige oder das nächste Objekt.
  **Strg**: die vorige oder die nächste Farbe. **Shift**: die vorige oder
  die nächste Maschinenfunktion. **Alt**: das vorige oder das nächste
  Segment.
- **Funktionen** (Ansicht > Stichcursor) legt die Art der Funktion fest,
  zu der Shift+Bild auf und Shift+Bild ab gehen: jede Funktion,
  Fadenschnitte, Farbwechsel, Stopps, Sprünge oder Tempowechsel.
- **Auf Stich zentrieren** verschiebt die Ansicht zum Cursor.

Wenn der Cursor in einem Objekt ist, wählt **Bearbeiten > Auswählen > Am
Stichcursor** es aus, **Bearbeiten > An Nadel teilen** schneidet es dort,
und **Inhalte einfügen** kann am Cursor einfügen.

## Die Stichliste

**Ansicht > Stichliste** zeigt die Stichliste: jeden Stich, Sprung,
Fadenschnitt, Stopp und Farbwechsel, mit seiner Stelle, seiner Länge und
seiner Farbe. Klicken Sie auf eine Zeile, um den Stichcursor dorthin zu
setzen. **Nur Funktionen** zeigt nur die Sprünge, Fadenschnitte,
Farbwechsel und Stopps.

## Designinfo

Die Gruppe **Designinfo** zeigt die Zahlen des Designs:

- die Größe, die Rahmen, in die es passt, die Zahl der Objekte, Stiche,
  Farben, Farbwechsel, Fadenschnitte, Sprünge und Stopps;
- die Stickzeit beim Tempo der Maschine;
- den Stoff, das Obergarn und das Unterfadengarn, und die Garnkosten;
- die Dichte (die meisten Garnlagen);
- die Farben in Stickreihenfolge, mit dem Garn jeder Farbe.

Unter den Zahlen sind die **Warnungen**. Klicken Sie auf eine Warnung, um
das Objekt auszuwählen. Das Programm warnt vor:

- Stichen außerhalb des Rahmenfelds oder außerhalb des Sicherheitsrands;
- Stichen, die kürzer als der kürzeste Stich sind (der Faden reißt);
- vielen Fadenschnitten in einem Objekt (versuchen Sie einen anderen
  Anfangs- oder Endpunkt);
- einer zu hohen Dichte;
- Garn, das sich an einer Stelle aufbaut (mehr Lagen als die Grenze);
- Schrift, die kleiner oder größer als der Größenbereich ihrer Schrift
  ist;
- auf einem Kapprahmen einer Stickreihenfolge, die nicht von der Mitte
  nach außen geht;
- in freistehender Spitze einem Teil, das kein anderes Teil berührt;
- Objekten, die keine Stiche machen.

## Die Dichtekarte

**Ansicht > Dichtekarte** färbt die Stiche nach den Garnlagen: rot, wo
übereinanderliegende Objekte mehr Lagen als die Grenze aufbauen
(**Optionen > Dichtegrenze**, am Anfang 5 Lagen). Die Designinfo listet
diese Stellen auf. Entfernen Sie verdeckte Teile mit **Grafik > Formen >
Abflachen**, oder machen Sie die Dichte offener.

## Die Garnansicht

**Ansicht > Garnansicht** zeigt die Stiche als Garne, wie sie auf dem
Stoff aussehen. Ihr Menü hat **Flach** oder **3D (Licht und Schatten)**,
die Richtung und die Höhe des Lichts, die Stärke des Schattens und die
**Stoffstruktur** (das Gewebe des Stoffs des Designs unter der
Stofffarbe). **Standardlicht** setzt sie zurück.

## Andere Ansichten der Stiche

**Ansicht > Zeigen** hat **Sprünge** (Sprünge und Verbindungslinien),
**Einstichpunkte** (ein Punkt an jedem Einstich) und **Fadenschnitte und
Farben** (ein Symbol an jedem Fadenschnitt und Farbwechsel).

## Das Produkt

**Design > Produkt** zeigt das Design in seiner echten Größe auf einem
T-Shirt, Polo, Sweatshirt, einer Kappe, Tasche, einem Handtuch oder
Kissen, oder auf einem Bild Ihres eigenen Produkts (**Mein Bild…**, in
seiner echten Breite). Legen Sie die **Farbe** fest und wohin das Design
kommt (**Das Design setzen auf**). **Produkt zeigen** zeigt oder verbirgt
es. **Bild auf Produkt** speichert ein PNG-Bild des Designs mit der
Garnansicht auf dem Produkt, für einen Kunden. Das Produkt wird nicht
gestickt.

## Wiederholungen

**Design > Wiederholungen** zeigt das Design noch einmal in **Säulen**
und **Reihen**, um die Verbindung einer Bordüre zu prüfen, die weitergeht
(ein Endlosdesign), oder von Designs, die sich überlappen.

- **Schritt nach rechts** und **Schritt nach unten** legen die Stelle der
  Kopien fest. Auch ein Schritt nach unten ergibt eine schräge Reihe; auch
  ein Schritt zur Seite ergibt einen halben Versatz. **Nebeneinander**
  setzt Schritte in der Größe des Designs.
- **Ende mit Anfang verbinden** setzt einen automatischen Anfang links und
  ein automatisches Ende rechts am Design, damit die Nadel von Kopie zu
  Kopie geht.
- **Wiederholungen zeigen** zeigt oder verbirgt sie. Die Kopien werden
  nicht gestickt, bis Sie auf **Wiederholungen erstellen** klicken: Dann
  sind sie Objekte, nach dem Design.

## Druckvorschau

**Design > Druckvorschau** zeigt das Produktions-Arbeitsblatt und das
Freigabeblatt. Siehe [Dateien und Ausgabe](files-and-output).
