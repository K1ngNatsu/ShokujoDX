---
id: troubleshooting
title: Fragen und Probleme
group: start
keywords: problem, frage, faq, falten, lücke, fadenschnitt, sprung, bruch, steif, kleine schrift, maschine liest nicht, datei, langsam, absturz, fehlt, falsche farbe, zu viele stiche, überlappung, anhäufung, svg, dichte, dichtekarte, lagen, bild, foto, bild zu design, objekte, faden reißt, vogelnest, schlaufen, umriss passt nicht, passgenauigkeit, verschiebung, flor, handtuch, sinkt ein, nicht mittig, rahmen, zu groß, passt nicht, buchstaben laufen zu, schaltfläche grau, kann nicht klicken, bereich weg
---
# Fragen und Probleme

## Der Stoff wirft Falten

Die Stiche ziehen den Stoff zusammen. Benutzen Sie ein stärkeres
Stickvlies (ein Schneidvlies für Strickstoffe), und spannen Sie den Stoff
straff ein. Legen Sie den richtigen Stoff fest (**Design > Stoff**).
Machen Sie die Füllung weniger dicht, oder benutzen Sie eine leichtere
Unterlage.

## Zwischen einer Füllung und ihrem Rand sind Lücken

Die Stiche ziehen die Füllung zusammen. Machen Sie den **Zugausgleich**
größer, oder lassen Sie den Rand etwas über die Füllung gehen. **Grafik >
Randkontur** fügt einen Rand hinzu, der unter die Kante der Füllung geht;
setzen Sie seine **Position des Rands** auf **Mittig auf der Linie** oder
**Innerhalb der Linie**, um mehr von der Kante zu bedecken. Bild zu Design
hat aus demselben Grund eine Einstellung **Überlappung**.

## Die Umrisse passen nicht zu den Füllungen

Der Stoff hat sich bewegt, während die Maschine stickte. Benutzen Sie ein
stärkeres Stickvlies und spannen Sie straff ein. Sticken Sie jeden Umriss
gleich nach der Füllung darunter, nicht am Ende des Designs. Mehr
Unterlage hilft der Füllung, ihre Form zu behalten.

## Das Design hat zu viele Fadenschnitte

Jeder Fadenschnitt kostet Zeit. Halten Sie die Objekte einer Farbe in der
Stickreihenfolge zusammen (**Anordnen > Weniger Farbwechsel**). **Anordnen
> Kürzeste Reihenfolge** setzt die Objekte jeder Farbe in die Reihenfolge
der kürzesten Wege. **Nächste Verbindung** gibt kurzen Lücken
Verbindungsstiche statt Fadenschnitten. **Danach schneiden: Verdeckter
Verbindungsstich** (Maschinenfunktionen) stickt Verbindungsstiche unter
den Farben, die später gestickt werden, statt eines Fadenschnitts. Die
Designinfo warnt vor Objekten mit vielen Fadenschnitten: Versuchen Sie
einen anderen Anfangs- oder Endpunkt.

## Bild zu Design erstellt zu viele Objekte

Ein Foto hat viele kleine Flächen jeder Farbe, und jede Fläche, die von
den anderen getrennt ist, braucht einen Fadenschnitt. Der Assistent gibt
seinen Objekten **Verdeckter Verbindungsstich**: Wo die Farben, die später
gestickt werden, den Weg bedecken, geht die Nadel mit Verbindungsstichen
und ohne Fadenschnitt dorthin. Im Assistenten:

- **Hintergrund:** Benutzen Sie **Motiv freistellen**, damit nur das Motiv
  gestickt wird. **Die Randfarbe entfernen** funktioniert nur bei einem
  einfarbigen Hintergrund gut.
- **Vereinfachen:** Machen Sie die Flecken größer (**Flecken entfernen
  unter**), benutzen Sie weniger Detail (ein größeres **Detail (mm je
  Pixel)**), oder benutzen Sie mehr **Ausgleichen**.
- **Fertig:** Die Prüfungen zeigen die Fadenschnitte und die sehr kleinen
  Objekte, und eine Schaltfläche ändert die Einstellungen für Sie.

Ein Logo oder eine Zeichnung mit flachen Farben gibt das beste Ergebnis.
Für ein Foto einer Person ist **Fotostich** oft besser.

## Satin aus einer SVG häuft sich auf der Maschine an

Wandeln Sie die Vektoren mit **Zu Satin** noch einmal um, oder setzen Sie
die Stichart der Füllung auf **Satin**: Jede Form wird dann in Teilen
gestickt, ohne einen Stich außerhalb. Um die Richtung der Stiche zu
steuern, benutzen Sie **Bearbeiten > Richtungslinie** auf dem Objekt.

## Kleine Schrift stickt sich nicht gut

Stickschriften haben einen Größenbereich; die Schriftliste zeigt ihn.
Unter etwa 5 mm verlieren Buchstaben ihre Form. Benutzen Sie eine Schrift,
die für kleine Schrift gemacht ist (**Schriften**, dann **Für kleine
Schrift**: diese Schriften gehen bis 4 mm), oder machen Sie den Text
größer. Setzen Sie die Unterlage kleiner Schrift auf **Keine** oder
**Leicht**.

## Die Buchstaben laufen zu oder sehen schwer aus

Der Satin ist für die Größe zu dicht, oder das Garn ist zu dick. Machen
Sie die Dichte der Schrift offener (Objekteigenschaften > Satin > Dichte),
benutzen Sie eine leichtere Unterlage, oder machen Sie den Text größer.

## Die Stiche sinken in ein Handtuch oder Fleece ein

Der Flor kommt durch die Stiche nach oben. Fügen Sie **Spezial >
Grundlage** unter dem Design hinzu, benutzen Sie ein wasserlösliches
Deckvlies, und legen Sie den Stoff **Towel (terry)** oder **Fleece and
sweatshirt** fest. Siehe [Anleitung: ein Monogramm auf einem
Handtuch](tutorial-towel).

## Das Design ist steif und die Nadel bricht

Die Stiche sind zu dicht, oder viele Lagen liegen übereinander. Um die
Stellen zu finden, schalten Sie **Ansicht > Dichtekarte** ein: Sie färbt
das Design nach den Garnlagen, und Stellen über der Grenze sind rot. Die
Designinfo listet jede Stelle auf; klicken Sie auf eine, um dorthin zu
gehen. Die Grenze ist in den **Optionen** (am Anfang 5 Lagen).

Machen Sie die Dichte größer (offener). Entfernen Sie die verdeckten Teile
der unteren Objekte mit **Grafik > Formen > Abflachen**.

## Der Faden reißt oder macht Schlaufen unter dem Stoff

- Sehr kurze Stiche lassen den Faden reißen: Die Designinfo warnt vor
  Stichen, die kürzer als der kürzeste Stich sind.
- Zu viele Lagen an einer Stelle (siehe die Dichtekarte).
- An der Maschine: Prüfen Sie das Einfädeln, die Spannung, die Nadel (eine
  neue Sticknadel der richtigen Stärke) und die Spule.

## Die Maschine liest die Datei nicht

- Prüfen Sie das Format Ihrer Maschine (die Liste **Maschine** auf dem Tab
  Design). Die meisten Heimmaschinen lesen nur ihr eigenes Format.
- Prüfen Sie den Ordner auf dem USB-Stick. **Auf USB senden** benutzt den
  Ordner Ihrer Maschine.
- Prüfen Sie, ob das Design in den Rahmen der Maschine passt: Die
  Designinfo sagt, ob Stiche außerhalb des Rahmenfelds liegen.

## Das Design ist an der Maschine nicht in der Mitte

Die Maschine setzt das Design dorthin, wo seine Datei es sagt, von der
Rahmenmitte aus. **Anordnen > Im Rahmen zentrieren** verschiebt die
Auswahl in die Rahmenmitte. Benutzen Sie **Design > Rahmenschablone**, um
die Stelle auf dem Stoff zu markieren. **Rahmenoptionen > Automatischer
Anfang und Ende** legt fest, wohin die Nadel zuerst geht.

## Das Design ist größer als mein Rahmen

**Automatischer Rahmen** wählt einen größeren Rahmen, wenn die Maschine
einen hat. **Design > Design drehen** dreht das Design um 90°, und
**Rahmen drehen** dreht den Rahmen. Oder sticken Sie das Design in
Teilen: siehe [Designs, die größer als der Rahmen sind](hooping).

## Die Farben an der Maschine sind falsch

Formate wie DST haben keine Farben. Die Maschine zeigt ihre eigenen
Farben. **Design ausgeben** kann Garnfarbdateien neben die Maschinendatei
schreiben. Prüfen Sie auch, welche Farbvariante aktuell ist (Tab Farbe):
Exportieren schreibt die aktuelle Farbvariante.

## Eine Maschinendatei wird zu dicht, wenn ich sie skaliere

Eine Maschinendatei hat Einstichpunkte, keine Formen. Benutzen Sie zuerst
**Grafik > Umwandeln > Umrisse erkennen**: Die Stiche werden zu
Füllungen, die ihre Dichte behalten, wenn Sie sie skalieren.

## Eine Schaltfläche ist grau

Der Befehl braucht zuerst etwas, zum Beispiel eine Auswahl, ein
Hintergrundbild oder zwei Objekte. Halten Sie den Zeiger auf die
Schaltfläche: Der Tooltip sagt, was sie tut, und die Hilfe (F1) sagt, was
sie braucht.

## Ich finde einen Befehl nicht

Drücken Sie **Strg+K** und tippen Sie einen Teil seines Namens. Das
Hilfethema jedes Tabs des Menübands listet alle seine Befehle auf. Im
einfachen Modus sind manche Befehle nur auf dem vollen Menüband: Schalten
Sie den einfachen Modus aus (Ansicht > Einfacher Modus).

## Ein Bereich ist weg

Das × eines Bereichs schließt ihn. **Ansicht > Bereiche** öffnet ihn
wieder.

## Warum startet das Programm mit einer Seite?

Das ist die Startseite. Entfernen Sie darauf das Häkchen bei **Beim Start
zeigen**, wenn Sie beim Start das leere Design wollen.

## Das Programm bricht ab oder ist langsam

Das Programm speichert Sicherungen und bietet sie beim nächsten Start an.
Ein Absturzbericht ist im Ordner `crashes` der Einstellungen (**Hilfe >
Info** zeigt, wo). Senden Sie ihn mit Ihrer Rückmeldung. Ein Design mit
sehr vielen Stichen oder einem großen Hintergrundbild ist langsamer;
blenden Sie die Garnansicht aus, während Sie arbeiten.
