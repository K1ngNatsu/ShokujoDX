---
id: checking
title: Ein Design prüfen
group: production
keywords: prüfen, neigen, neigeansicht, drehen, lupe, vergleichen, vorher und nachher, durchscheinen, hohes garn, fadenrisse, lichtvorgaben, vorschau, stich-player, abspielen, stickablauf, simulieren, stichcursor, verbindungsstiche, stichliste, funktionen, designinfo, warnungen, dichtekarte, lagen, aufbau, garnansicht, 3d, licht, einstichpunkte, sprünge, fadenschnitte, produkt, t-shirt, mockup, wiederholungen, endlos, freigabe, stickzeit, stichzahl
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
**Stoffstruktur** (die Oberfläche des Stoffs des Designs in der
Stofffarbe: die Fäden seines Gewebes, die Schlingen eines Handtuchs oder
die Narbung von Leder, im Licht der Garnansicht). **Standardlicht** setzt sie zurück.

Mit **Feine Oberfläche** (beim Start ein) zeigt die 3D-Ansicht die
Oberfläche der Stiche: Garn auf Garn ist höher, jede Fläche ist heller
oder dunkler nach der Richtung ihrer Stiche zum Licht, und hohe Teile
werfen Schatten auf den Stoff. Jedes Nadelloch ist eine kleine Delle
(größer auf Leder). Auf einem Handtuch oder Fleece ohne Deckvlies sinken
dünne Stiche in den Flor (siehe [Stoffe, Stile und Vorlagen](fabrics-and-styles)).
Was kein Garn ist, hat seine eigene Oberfläche: Applikationsstoff mit
seiner Webart und einer Schnittkante, Strasssteine mit Facetten, die das
Licht fangen, die Löcher von Cutwork, kein Stoff unter Spitze, Polster
unter einer Trapunto-Füllung und auf einem Quilt das Vlies zwischen den
Stichlinien. Metallgarn hat bei großem Zoom kleine Glanzpunkte. **Licht
aus** zeigt einen dunklen Raum: Nur nachtleuchtendes Garn ist zu sehen,
in seinem eigenen Licht.
Das Programm macht die Oberfläche in
Teilen, während Sie arbeiten; ein Teil, der noch nicht fertig ist, zeigt
kurz die schnellere Ansicht. Schalten Sie **Feine Oberfläche** auf einem
langsamen Computer aus.

## Die Ansicht prüfen

Die Gruppe **Prüfen** der Registerkarte Ansicht hat Werkzeuge, um ein
Design zu prüfen, bevor Sie es sticken.

- **Beleuchtung**: die Vorgaben **Von oben links** (der Standard),
  **Flach von oben**, **Flach von der Seite** (sie zeigt jede Höhe und
  jede Lücke) und **Zwei Lichter**, und die Richtung und die Höhe des
  Lichts. Halten Sie **L** gedrückt und ziehen Sie auf dem Design, um das
  Licht zu bewegen: In der Mitte der Ansicht ist es genau oben, am Rand
  ist es flach.
- **Neigeansicht** zeigt die Stickerei auf der Arbeitsfläche schräg.
  Ziehen Sie nach oben und unten, um sie zu neigen, und quer, um sie zu
  drehen; ziehen Sie mit der rechten Taste, um sie zu verschieben, und
  drehen Sie das Rad, um zu zoomen. **Höhen** macht die Höhen größer (×1
  ist die echte Höhe). Das Licht bleibt an seiner Stelle, wenn sich das
  Design dreht, wie wenn Sie einen Rahmen unter einer Lampe drehen. Die
  Werkzeuge der Arbeitsfläche warten, bis Sie auf **Schließen** klicken.
  **Neigefenster** zeigt die gleiche Ansicht in einem Fenster, damit Sie
  das Design ändern und es schräg sehen können.
- **Lupe** zeigt das Garn in 20-facher echter Größe in einem runden
  Fenster am Zeiger.
- **3D vergleichen** zeigt die Stiche auf der Arbeitsfläche und die
  3D-Ansicht in der geteilten Ansicht, an der gleichen Stelle und mit dem
  gleichen Zoom (**Gleiche Stelle** in der geteilten Ansicht).
- **Vorher und nachher** behält die Stiche so, wie sie jetzt sind.
  Ändern Sie eine Einstellung und ziehen Sie dann die Linie auf der
  Arbeitsfläche: Die linke Seite zeigt die Stiche vorher, die rechte
  Seite die Stiche jetzt.
- **Stoff scheint durch** markiert die Stellen, an denen der Stoff durch
  eine Füllung oder einen Satin scheint, aus den gelegten Garnen (ihren
  Breiten, Höhen und Lücken). Füllungen, die absichtlich offen sind (ein
  Verlauf, eine Struktur, Motive), werden nicht geprüft. Machen Sie die
  Dichte höher oder ändern Sie den Winkel der Reihen, bis sich die Stelle
  schließt.
- **Hohes Garn** markiert die Stellen, an denen das Garn höher als die
  Grenze über dem Stoff ist (Schaum und Polster zählen nicht): ein Risiko
  für Fadenrisse.

Legen Sie die Grenzen der beiden Prüfungen in den **Optionen** fest
(**Stoff scheint durch** und **Grenze der Garnhöhe**).

## Andere Ansichten der Stiche

**Ansicht > Zeigen** hat **Sprünge** (Sprünge und Verbindungslinien),
**Einstichpunkte** (ein Punkt an jedem Einstich) und **Fadenschnitte und
Farben** (ein Symbol an jedem Fadenschnitt und Farbwechsel).

## Das Produkt

**Design > Produkt** zeigt das Design in seiner echten Größe auf einem
T-Shirt (kurz- oder langarm), Polo, Sweatshirt, Kapuzenpullover, einer
Kappe, Mütze, Schürze, Tasche, einem Handtuch oder Kissen, oder auf einem Bild Ihres eigenen Produkts (**Mein Bild…**, in
seiner echten Breite). Legen Sie die **Farbe** fest und wohin das Design
kommt (**Das Design setzen auf**). **Produkt zeigen** zeigt oder verbirgt
es. **Bild auf Produkt** speichert ein PNG-Bild des Designs mit der
Garnansicht auf dem Produkt, für einen Kunden. Das Produkt wird nicht
gestickt. Ein gezeichnetes Produkt hat Licht und Schatten, Falten und
die Struktur seines Stoffs, im Licht der Garnansicht; mit der 3D-Ansicht
hat die Stickerei darauf ihre Höhen, ihren Glanz und ihre Schatten im
gleichen Licht; auf einer Kappe zeigt das Bild
das Design ein wenig um die Krone gebogen. **Ein Design hinzufügen…**
zeigt ein weiteres Design (ein Projekt oder eine Maschinendatei) auf
dem Produkt, zum Beispiel einen Namen auf einem Ärmel mit diesem Design
auf der linken Brust: Legen Sie seine Stelle in der Liste **Andere
Designs auf dem Produkt** fest. Es erscheint auf der Arbeitsfläche und
im Bild und wird nicht gestickt. Ein Kleidungsstück hat eine
Vorder- und eine Rückseite: **Seite** im Fenster Produkt (oder
**Produkt drehen**, Strg+Umschalt+T) zeigt die andere Seite, und das
Design kommt an dieselbe Stelle darauf. Jedes andere Design ist auf
einer Seite. Mit **Beide Seiten im Bild** zeigt das gespeicherte Bild
Vorder- und Rückseite nebeneinander. Die Farbfelder unter **Farbe**
sind die üblichen Farben von Rohlingen. Mit **Besatzfarbe** bekommen
die Besatzteile des Produkts (zum Beispiel der Kragen eines Polos, die
Vorderteile oder der Schirm einer Kappe, die Bündchen eines
Sweatshirts) eine zweite Farbe; entfernen Sie ein Teil in
**Besatzteile**, damit es die Produktfarbe hat.

Für ein Foto Ihres eigenen Produkts öffnet **Foto bearbeiten…** das
Fenster **Produktfoto**. **Farbe wählen** und ein Klick auf das Produkt
markieren die Fläche dieser Farbe (auch Nähte und Stiche darin;
**Toleranz** legt fest, wie nah eine Farbe sein muss); **Farbe
entfernen**, **Pinsel** und **Radierer** ändern die Maske. Der markierte
Teil bekommt die Produktfarbe mit dem Licht und dem Schatten des Fotos,
so zeigt ein Foto eines weißen Produkts das Produkt in jeder Farbe.
**Stelle markieren** und ein Klick markieren eine übliche Stelle für ein
Design; geben Sie ihr einen Namen und die Größe ihres größten Designs.
**In Meine Produkte speichern** behält das Foto mit seiner echten Breite,
seiner Maske und seinen Stellen; wählen Sie es wieder in **Meine
Produkte**. **Exportieren…** und **Importieren…** bringen ein Produkt auf
einen anderen Computer.

**Design > Drehteller** zeigt ein gezeichnetes Produkt mit seinen
Designs von mehr Seiten: Es dreht sich in Schritten von 15°. Verwenden
Sie die Pfeile oder den Schieberegler, ziehen Sie das Bild, oder klicken
Sie auf **Abspielen**. Das Programm erstellt die 24 Ansichten nach jeder
Änderung im Hintergrund, so ist eine Drehung auch auf einem langsamen
Computer schnell. Das Produkt hat eine weiche, runde Form aus seiner
Zeichnung; die Arbeitsfläche bleibt flach. **GIF speichern…** speichert
die Drehung als animiertes Bild, und **Vier Ansichten speichern…**
speichert Vorderseite, zwei Vierteldrehungen und Rückseite nebeneinander.
Ein Foto eines Produkts kann sich nicht drehen.

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
