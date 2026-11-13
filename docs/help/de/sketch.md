---
id: sketch
title: Skizzen
group: design
keywords: skizze, parametrisch, bedingung, maß, cad, genaue größe, linie, bogen, kreis, rechteck, vieleck, tangential, parallel, rechtwinklig, waagerecht, senkrecht, gleich, symmetrisch, fixieren, konstruktion, vollständig bestimmt, freie bewegungen, konflikt, stutzen, verlängern, teilen, rundung, fase, versatz, spiegeln, muster, entspannen, einrasten
---
# Skizzen

Eine Skizze ist eine Zeichnung mit genauen Größen, wie in CAD-Programmen.
Sie zeichnen Linien, Bögen und Kreise und geben ihnen dann Bedingungen
(zum Beispiel waagerecht, tangential oder gleich) und Maße (Längen, Winkel
und Größen). Wenn Sie ein Maß ändern, ändert sich die Skizze, und die
Bedingungen bleiben erfüllt. Eine Skizze stickt selbst nichts: Sie
erstellen Stickerei aus ihren geschlossenen Bereichen und ihren Kurven
(siehe Stickerei erstellen).

## Beginnen und beenden

- **Digitalisieren > Skizze beginnen** erstellt eine neue Skizze oder
  öffnet die ausgewählte Skizze. Ein Doppelklick auf eine Skizze öffnet sie
  auch.
- Der Tab **Skizze** zeigt sich, während die Skizze offen ist. Die anderen
  Objekte sind abgeblendet und nehmen keine Klicks an.
- **Skizze beenden** schließt die Skizze. Eine Skizze ohne Inhalt
  verschwindet.

Jede Änderung ist ein Schritt zum Rückgängigmachen; ein Ziehen ist ein
Schritt.

## Tasten und die Maus

- **Esc** beendet den Befehl und geht zurück zu **Auswählen**. In Auswählen
  hebt Esc die Auswahl auf.
- **Entf** oder **Rücktaste** löscht die ausgewählten Teile mit ihren
  Bedingungen. Ein gelöschter Punkt nimmt seine Kurven mit.
- **Eingabe** oder ein Rechtsklick beendet eine Kette von Linien.
- **Strg+A** wählt alles aus. **F8** zeigt die Markierungen der
  Bedingungen, und **F9** blendet sie aus (auch **Bedingungen anzeigen**
  im Bereich Eigenschaften).
- Mit Auswählen verschiebt das Ziehen eines Teils der Auswahl die ganze
  Auswahl. Strg+Klick oder Umschalt+Klick fügt der Auswahl einen Teil
  hinzu; ein Ziehen an einer leeren Stelle wählt die Teile in einem
  Rahmen aus.
- Der Teil unter dem Zeiger leuchtet vor dem Klick auf, mit seinen
  Bedingungen (auch wenn die Markierungen aus sind). Eine Bedingung unter
  dem Zeiger lässt ihre Teile aufleuchten.
- Ein **Rechtsklick** auf einen Teil wählt ihn aus und zeigt die Befehle
  dafür: den Wert eines Maßes ändern, ein passendes Maß oder eine passende
  Bedingung hinzufügen, am Klick stutzen, teilen oder verlängern, eine Ecke
  runden oder fasen, Konstruktion, Versatz, Spiegeln, Muster, seine
  Bedingungen löschen oder ihn löschen.
- **Rückgängig** (Strg+Z) und **Wiederholen** wirken wie im übrigen
  Programm.

## Zeichnen

- **Linie**: Klicken Sie den Anfang, dann jeden nächsten Punkt. Klicken Sie
  den ersten Punkt, um die Form zu schließen; Eingabe, Esc oder ein
  Rechtsklick beendet die Kette. Ziehen Sie vom Ende einer Linie, um einen
  dazu tangentialen Bogen zu zeichnen; die Kette geht dann vom Ende des
  Bogens weiter.
- **Rechteck**: zwei Ecken. Die Seiten sind waagerecht und senkrecht.
- **Rechteck aus Mittelpunkt**: die Mitte, dann eine Ecke.
- **Rechteck aus drei Punkten**: zwei Ecken einer Seite (in jedem Winkel),
  dann ein Punkt für die Höhe.
- **Rechteck aus drei Punkten vom Mittelpunkt**: die Mitte, die Mitte
  einer Seite, dann eine Ecke.
- **Langloch**: Die Art ist im Bereich Eigenschaften. Mitte zu Mitte: die
  zwei Mitten, dann ein Punkt für die Breite. Gesamt: die zwei Enden, dann
  die Breite. Mittelpunkt: die Mitte, die Mitte eines Endes, dann die
  Breite. Dreipunktbogen: die zwei Mitten, ein Punkt auf dem Bogen, dann
  die Breite. Mittelpunktbogen: die Mitte des Bogens, die zwei Mitten, dann
  die Breite. Die Enden sind Halbkreise, tangential zu den Seiten, mit
  demselben Radius.
- **Tangentialer Kreis**: Klicken Sie drei Linien; der Kreis berührt alle
  drei (auf der Seite Ihrer Klicks).
- **Kreis**: die Mitte, dann ein Punkt auf dem Kreis.
- **Vieleck**: die Mitte, dann eine Ecke. Legen Sie die Zahl der Seiten im
  Bereich Eigenschaften fest. Die Ecken liegen auf einem
  Konstruktionskreis, und die Seiten sind gleich. Mit **Vieleck um seinen
  Kreis** berühren die Seiten den Kreis, und der Klick ist die Mitte einer
  Seite.
- **Dreipunktbogen**: der Anfang, das Ende, dann ein Punkt auf dem Bogen.
- **Mittelpunktbogen**: die Mitte, der Anfang, dann das Ende (gegen den
  Uhrzeigersinn).
- **Tangentialbogen**: Klicken Sie das Ende einer Linie, dann das Ende des
  Bogens. Der Bogen setzt die Linie fort, tangential zu ihr.
- **Punkt**: ein Punkt, zum Beispiel, um eine Bedingung darauf zu setzen.
- **Spline**: Klicken Sie die Punkte einer glatten Kurve. Eingabe, ein
  Rechtsklick oder ein Klick auf den letzten Punkt beendet sie; ein Klick
  auf den ersten Punkt schließt sie. Der Spline hat zwischen je zwei
  Punkten ein Stück, mit zwei Griffen (dünne Linien von den Punkten).
  Ziehen Sie die Punkte und die Griffe, um ihn zu formen; eine Tangente an
  jeder Verbindung hält ihn glatt. Ein Spline kann an einem gemeinsamen
  Ende tangential zu einer Linie oder einem Bogen sein, und ein Punkt kann
  auf ihm liegen. Maße gehen auf seine Punkte, nicht auf den Spline.

Ein Klick nahe einem Punkt der Skizze benutzt diesen Punkt, darum teilen
Linien, die sich treffen, ihre Ecke. **Konstruktion** macht die
ausgewählten Kurven zu Konstruktionskurven (orange gestrichelt): Sie
helfen beim Zeichnen und werden nicht zu Stickerei. Ohne Auswahl schaltet
es dies für neue Kurven ein oder aus.

## Hilfe beim Zeichnen

Während Sie zeichnen, fügt das Programm Bedingungen hinzu. Eine Markierung
neben dem Zeiger zeigt sie vor dem Klick, und die Punkte Ihrer Klicks
zeigen sich sofort:

- Ein Klick nahe einem Punkt benutzt diesen Punkt (deckungsgleich).
- Ein Klick nahe der Mitte einer Linie setzt den Punkt dorthin (**M**, eine
  Raute).
- Ein Klick nahe der Stelle, wo sich zwei Kurven kreuzen, setzt den Punkt
  auf beide (**X**).
- Ein Klick nahe einer Kurve setzt den neuen Punkt auf die Kurve (**O**).
- Eine gestrichelte Linie zeigt sich, wenn der Zeiger auf einer Linie mit
  einem Punkt ist (quer oder auf und ab): Der Klick übernimmt sein x oder
  sein y.
- Eine Linie, die fast waagerecht oder senkrecht ist, wird es (**H**,
  **V**).
- Eine Linie, die am Ende eines Bogens in der Richtung des Bogens beginnt
  oder endet, oder die einen Kreis in seiner Richtung berührt, wird
  tangential (**T**).
- Eine Linie, die fast im 90°-Winkel zur Linie davor steht, wird
  rechtwinklig (**⊥**). Bewegen Sie den Zeiger zuerst über eine andere
  Linie, und eine neue Linie, die dazu fast parallel ist oder fast im
  90°-Winkel steht, wird es (**//**, **⊥**).

Die Werkzeuge zeigen die Form, die der nächste Klick erstellt.

Halten Sie **Alt** beim Klicken, um keine davon hinzuzufügen (ein Klick auf
einen Punkt benutzt den Punkt trotzdem). Das Kästchen **Beim Zeichnen
Bedingungen hinzufügen** im Bereich Eigenschaften schaltet sie aus.

## Bedingungen

Die Befehle der Bedingungen sind die Symbole des Bereichs Bedingungen (der
Tooltip gibt den Namen). Wählen Sie zuerst die Teile aus (Klick;
Strg+Klick für mehr), dann klicken Sie auf eine Bedingung. Oder klicken
Sie zuerst auf die Bedingung, dann auf die Teile. Die Statuszeile sagt,
was zu klicken ist.

| Bedingung | Teile |
| --- | --- |
| Deckungsgleich | zwei Punkte |
| Waagerecht, Senkrecht | eine Linie oder zwei Punkte |
| Parallel, Rechtwinklig, Kollinear | zwei Linien |
| Tangential | eine Linie und ein Kreis oder ein Bogen, oder zwei Kreise oder Bögen |
| Gleich | zwei Linien, oder zwei Kreise oder Bögen |
| Konzentrisch | zwei Kreise oder Bögen |
| Mittelpunkt | ein Punkt und eine Linie |
| Auf Kurve | ein Punkt und eine Linie, ein Kreis oder ein Bogen |
| Symmetrisch | zwei Punkte und die Linie zwischen ihnen |
| Fixieren | ein Punkt |

Eine Markierung neben dem Teil zeigt jede Bedingung (H, V, //, ⊥, T, =, ◎,
M, O, S, F). Wählen Sie eine Markierung aus und drücken Sie Entf, um die
Bedingung zu entfernen.

## Maße

Klicken Sie auf **Maß**. Das Maß, das Ihr nächster Klick erstellt, zeigt
sich als Schatten am Zeiger, damit Sie sehen, was Sie bemaßen. Klicken
Sie:

- einen Kreis: seinen Durchmesser; einen Bogen: seinen Radius;
- zwei Punkte: den Abstand zwischen ihnen;
- eine Linie, dann eine leere Stelle: ihre Länge;
- eine Linie, dann eine zweite Linie: den Winkel zwischen ihnen;
- einen Punkt und eine Linie: den Abstand vom Punkt zur Linie.

Wenn die Teile gewählt sind, folgt der Schatten dem Zeiger: Klicken Sie
dort, wo das Maß hinkommt. Bei einer schrägen Linie oder zwei Punkten
wählt die Stelle die Art: darüber oder darunter ein waagerechter Abstand,
links oder rechts ein senkrechter Abstand, sonst der Abstand entlang
ihnen. Klicken Sie mit der rechten Maustaste, während das Maß wartet, um
die Art selbst zu wählen: Ausgerichtet, Waagerecht oder Senkrecht; bei
einem Bogen Radius, Durchmesser oder Bogenlänge.

Eine **Mittellinie** (Bereich Zeichnen; wählen Sie zuerst Linien aus) ist
eine Konstruktionslinie mit langen und kurzen Strichen. Ein Maß von einem
Punkt zu einer Mittellinie ist ein Durchmesser (⌀), wie bei einem Teil,
das sich um die Linie dreht. Ziehen Sie mit Auswählen ein Maß, um es zu
verschieben; Beschriftung zurücksetzen (Rechtsklick) setzt es zurück.

Wenn die Skizze die Größe schon festlegt, ist das neue Maß **abgeleitet**:
Es zeigt die Größe in Klammern und ändert die Skizze nicht. Klicken Sie
mit der rechten Maustaste auf ein Maß für **Abgeleitet machen** oder
**Steuernd machen**.

Tippen Sie den Wert und drücken Sie Eingabe (Esc bricht ab; ein Klick
woanders behält den Wert). Sie können einen Wert in anderen Einheiten als
denen des Programms tippen: `2 in`, `1 1/2"`, `3/4 in`, `50 mm`, `3 cm`.
Sie können auch rechnen: `2 in + 3 mm`, `120/4`, `(10 + 5) * 2`. Eine Zahl
ohne Einheit ist in den Einheiten des Programms, die Sie in den Optionen
oder im Bereich Eigenschaften der Skizze festlegen (mm oder in). Ein
Doppelklick auf ein Maß ändert es wieder. Der Bereich Eigenschaften listet
alle Bedingungen und Maße auf, mit **Ändern** für einen Wert und ✕ zum
Entfernen.

## Parameter

Ein Parameter ist ein benannter Wert, zum Beispiel `width = 80 mm`, oder
ein Ausdruck aus anderen Parametern, zum Beispiel `border = width / 20`.

1. Klicken Sie auf **Parameter** (Tab Skizze, Bereich Maß).
2. Tippen Sie einen Namen und einen Wert oder einen Ausdruck, dann klicken
   Sie auf **Hinzufügen**. Ändern Sie einen Ausdruck oder einen Namen in
   der Liste; ✕ entfernt einen Parameter, den kein Wert benutzt.
3. Tippen Sie im Wertfeld eines Maßes den Namen eines Parameters oder einen
   Ausdruck, zum Beispiel `width`, `width - 2 * border` oder
   `max(border, 3 mm)`. Die Beschriftung zeigt `width = 80.00`, oder `ƒx`
   für einen anderen Ausdruck.

Wenn Sie einen Parameter ändern, ändern sich die Maße, die ihn benutzen,
die Skizze wird gelöst, und die daraus erstellten Objekte folgen.
Ausdrücke können Einheiten, `+ - * / ^`, Klammern, `pi` und die Funktionen
`sqrt`, `abs`, `min`, `max`, `sin`, `cos` und `tan` (Grad) benutzen. Eine
Zahl ohne Einheit in einer Summe mit einer Länge ist in den Einheiten, die
das Fenster zeigt (mm oder in). Ein Parameter, der sich selbst benutzt,
direkt oder über andere Parameter, ist eine Schleife: Das Programm lehnt
ihn ab.

**Vorlagen**: Speichern Sie ein Design mit einer Skizze, die Parameter hat,
als Vorlage (**Als Vorlage speichern**, mit ihren Objekten). Ein neues
Design aus dieser Vorlage fragt nach den Werten der Parameter und erstellt
dann die Skizze und ihre Objekte in dieser Größe.

## Skizzen aus Grafik

- **Vektor zu Skizze** (Tab Grafik, Umwandeln; oder Rechtsklick >
  Umwandeln): Die ausgewählten Vektorobjekte werden zu einer Skizze. Gerade
  Teile werden zu Linien, runde Teile zu Bögen und Kreisen, andere Kurven
  zu Splines. Das Programm fügt die Bedingungen hinzu, die die Zeichnung
  zeigt: waagerechte und senkrechte Linien, Tangenten, wo Kurven glatt
  zusammenkommen, und gleiche Längen und Radien. Dann können Sie Maße
  hinzufügen und die Form ändern.
- Um eine SVG-, PDF- oder EMF-Datei zu benutzen, importieren Sie sie mit
  **Als Vektoren importieren**, dann benutzen Sie Vektor zu Skizze.
- Eine **DXF**-Zeichnung (Grafik importieren, oder die Datei auf das
  Fenster ziehen) öffnet sich als neue Skizze in der Mitte des Rahmens, in
  den Einheiten der Zeichnung.

## Farben und freie Bewegungen

- **Blau**: Der Teil kann sich noch bewegen.
- **Grün**: Der Teil ist fest: Nichts kann ihn bewegen.
- **Rot**: Eine Bedingung steht im Konflikt mit den anderen. Entfernen Sie
  sie, oder ändern Sie einen Wert.
- **Graue** Markierungen: Bedingungen, die andere wiederholen.

Die Statuszeile gibt die Zahl der freien Bewegungen an. Eine Skizze ist
vollständig bestimmt, wenn sie 0 hat: Sie braucht dann einen fixierten
Punkt (Fixieren) oder Maße von einem aus, sonst kann sie sich noch als
Ganzes bewegen.

## Ändern

Die Werte dieser Werkzeuge (der Radius, die Abstände) sind im Bereich
Eigenschaften, während das Werkzeug aktiv ist. Jede Bearbeitung ist ein
Schritt zum Rückgängigmachen; eine Bearbeitung, die nicht möglich ist,
ändert nichts und gibt eine Meldung.

- **Stutzen**: Klicken Sie das Stück einer Kurve, das weg soll. Das Stück
  geht von einer Kurve, die es schneidet, bis zur nächsten. Eine Kurve,
  die nichts schneidet, verschwindet; ein Kreis mit zwei Schnitten wird zu
  einem Bogen. Die neuen Enden bleiben auf den Kurven, die sie schneiden.
- **Verlängern**: Klicken Sie nahe dem Ende einer Linie oder eines Bogens.
  Das Ende geht bis zur nächsten Kurve, die die Verlängerung trifft, und
  bleibt darauf.
- **Teilen**: Klicken Sie eine Linie oder einen Bogen, um sie dort in zwei
  zu schneiden. Die zwei Stücke bleiben in einer Linie (oder auf einem
  Kreis).
- **Rundung**: Klicken Sie die Ecke zweier Linien. Ein Bogen des Radius
  rundet die Ecke, tangential zu beiden Linien; tippen Sie den Radius in
  das Feld. Die Ecke bleibt als Punkt auf beiden Linien, darum bleibt ein
  Maß zur Ecke erhalten. Eine Länge einer Linie wird zum Abstand zur Ecke.
- **Fase**: Klicken Sie die Ecke zweier Linien. Eine Linie schneidet die
  Ecke: derselbe Abstand auf jeder Linie, zwei Abstände (der erste auf der
  älteren Linie) oder ein Abstand und ein Winkel (legen Sie die Art im
  Bereich Eigenschaften fest).
- **Dehnen**: Ziehen Sie einen Rahmen um die Punkte, die sich bewegen
  sollen, dann klicken Sie einen Basispunkt und wohin er geht. Die Kurven,
  die an den Punkten enden, dehnen sich.
- **Versatz**: Wählen Sie Kurven aus und klicken Sie auf Versatz, oder
  klicken Sie auf Versatz und dann auf eine Kurve (sie und die Kurven, die
  mit ihr verbunden sind). Die Kopie liegt im Abstand: positiv außerhalb
  einer geschlossenen Form (sonst links), negativ auf der anderen Seite.
  Eine versetzte Linie bleibt in ihrem Abstand parallel; ein versetzter
  Bogen behält seine Mitte.

## Verschieben, kopieren, drehen und skalieren

Wählen Sie zuerst die Teile aus.

- **Verschieben**: Klicken Sie einen Basispunkt, dann wohin er geht. Oder
  tippen Sie X und Y im Bereich Eigenschaften und klicken Sie auf **Um die
  Werte verschieben**.
- **Kopieren**: wie Verschieben, mit einer Kopie; die Teile bleiben.
- **Drehen**: Legen Sie den Winkel im Bereich Eigenschaften fest (positiv
  ist gegen den Uhrzeigersinn), dann klicken Sie die Mitte.
- **Skalieren**: Legen Sie den Faktor fest, dann klicken Sie den
  Basispunkt. Die Maße zwischen den Teilen skalieren mit.

Eine gestrichelte Vorschau zeigt, wohin die Teile gehen. Drehen und
Skalieren können kopieren (**Kopieren** im Bereich Eigenschaften). Die
Teile bewegen sich so weit, wie ihre anderen Bedingungen es erlauben.

## Kopieren

Wählen Sie zuerst die Kurven aus.

- **Spiegeln**: Klicken Sie auf Spiegeln, dann auf die Linie, an der
  gespiegelt wird. Jeder Punkt der Kopie ist symmetrisch zu seinem Punkt
  (ein Punkt auf der Linie wird geteilt), darum folgt die Kopie, wenn Sie
  die Kurven ändern.
- **Rechteckmuster**: Legen Sie die Spalten, die Zeilen und den Abstand im
  Bereich Eigenschaften fest, dann klicken Sie auf **Muster erstellen**.
- **Kreismuster**: Legen Sie die Kopien und den Winkel fest (360° ringsum),
  dann klicken Sie die Mitte.

Die Kopien eines Musters haben die Bedingungen der Kurven zwischen ihnen,
aber ihre Stellen sind frei.

## Entspannen

Wenn Bedingungen im Konflikt stehen (rot), entfernt **Entspannen** sie,
Maße zuerst, bis kein Konflikt übrig ist. Die Meldung und der Bereich
Eigenschaften listen auf, was es entfernt hat. Rückgängig bringt sie
zurück.

## Automatische Maße

**Automatische Maße** (Bereich Maß) fügt hinzu, was die ausgewählten
Kurven (oder alle Kurven) brauchen: waagerecht und senkrecht für Linien,
die es fast sind, dann Längen, Durchmesser, Radien und Winkel, mit den
jetzigen Größen. Es fügt nichts hinzu, was sich wiederholt oder im
Konflikt steht. Die Stelle der Skizze bleibt frei: Fügen Sie einem Punkt
ein **Fixieren** hinzu, um sie festzulegen.

## Mittelpunkte

Wählen Sie Punkte aus und klicken Sie auf **Mittelpunkt** (Bereich
Zeichnen), um sie als Kreuz zu zeigen, zum Beispiel die Stelle eines
Lochs.

## Entspannungsmodus

**Entspannungsmodus** (Bereich Maß) ist ein oder aus. Während er ein ist,
ändert ein Ziehen die Maße, die die Teile halten: Sie nehmen die neuen
Größen an. Die anderen Bedingungen (waagerecht, tangential und so weiter)
bleiben.

## Stickerei erstellen

Die Kurven (nicht die Konstruktionskurven) bilden geschlossene Bereiche,
wo sie sich kreuzen: Ein Kreis in einem Kreis ergibt einen Ring und eine
Scheibe; eine Form in einem Bereich ist eine Insel, und der Bereich
bekommt dort ein Loch.

1. Klicken Sie auf **Bereiche** (Tab Skizze), dann klicken Sie in die
   Bereiche, die Sie wollen (sie sind schattiert; klicken Sie noch einmal,
   um die Wahl aufzuheben). Ohne gewählten Bereich nehmen die Befehle alle
   Bereiche.
2. Klicken Sie in **Stickerei erstellen** (Bereich Eigenschaften) oder mit
   einem Rechtsklick auf **Füllung**, **Satin**, **Applikation** oder
   **Vektor**.

Für Kurven (auch Konstruktionskurven): Wählen Sie sie aus, dann
**Steppstich**, **Satinlinie** (die Breite steht daneben) oder, für zwei
Kurven, **Satin dazwischen** (eine Satinsäule mit den zwei Kurven als
Seiten).

Die neuen Objekte bekommen die aktuellen Eigenschaften und das aktuelle
Garn und kommen ans Ende der Stickreihenfolge. Wenn die Skizze geschlossen
ist, erstellt ihr Bereich Eigenschaften Stickerei aus allen ihren
Bereichen oder Steppstiche entlang aller ihrer Kurven.

## Objekte, die der Skizze folgen

Die Objekte, die Sie aus einer Skizze erstellen, bleiben mit ihr
verknüpft. Wenn Sie ein Maß oder eine Bedingung ändern, erstellt das
Programm sie neu. Sie behalten ihre Sticheinstellungen, ihr Garn und ihren
Platz in der Stickreihenfolge. Rückgängig nimmt die Änderung und die neuen
Objekte in einem Schritt zurück.

- Um die Skizze wieder zu ändern, doppelklicken Sie auf eines ihrer
  Objekte, oder klicken Sie auf **Skizze bearbeiten** (Rechtsklickmenü oder
  Bereich Eigenschaften).
- Wenn Sie die Skizze verschieben, drehen oder skalieren, bewegen sich ihre
  Objekte mit. Wenn Sie nur die Objekte verschieben, bleiben sie in diesem
  Abstand zur Skizze.
- Wenn eine Änderung einen Bereich öffnet oder eine Kurve löscht, bleiben
  seine Objekte, wie sie sind, und das Programm zeigt eine Warnung.
  Schließen Sie den Bereich wieder, oder klicken Sie auf **Verknüpfung
  lösen**, um daraus einfache Objekte zu machen.
- Formbefehle nehmen keine verknüpften Objekte an. Lösen Sie zuerst die
  Verknüpfung. Eine Änderung der Form eines verknüpften Objekts von Hand
  bleibt nach der nächsten Änderung der Skizze nicht erhalten.

## Ziehen

Ziehen Sie mit **Auswählen** einen Punkt oder eine Linie: Die Skizze
bewegt sich mit, und die Bedingungen bleiben erfüllt. Ziehen Sie den Rand
eines Kreises, um seine Größe zu ändern.
