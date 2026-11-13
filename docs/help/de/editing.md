---
id: editing
title: Objekte ändern
group: design
keywords: bearbeiten, kopieren, ausschneiden, einfügen, inhalte einfügen, nach auswahl einfügen, duplizieren, löschen, clipart, auswählen, nach farbe, nach art, teilen, verbinden, aufteilen, messer, an nadel teilen, loch, löcher entfernen, löcher füllen, eckenradius, loch aus umriss, rahmen, vereinfachen, knoten, linie fortsetzen, redwork, verzweigung, form schließen, umkehren, seiten tauschen, maschinenfunktionen, stopp, fadenschnitt, anfangsvernähen, endvernähen, vernähstich
---
# Objekte ändern

Der Tab **Bearbeiten** ändert die Objekte, die Sie haben. Die meisten
Befehle wirken auf die Auswahl; ein Befehl, der jetzt nicht ausgeführt
werden kann, ist grau.

## Zwischenablage

- **Kopieren** (Strg+C), **Ausschneiden** (Strg+X) und **Einfügen**
  (Strg+V). Optionen > Bearbeiten legt fest, wohin Einfügen die Kopien
  setzt.
- **Duplizieren** (Strg+D) erstellt eine Kopie etwas daneben. Die Optionen
  legen den Abstand fest.
- **Nach Auswahl einfügen** setzt die Kopien in der Stickreihenfolge
  direkt nach die ausgewählten Objekte, nicht ans Ende.
- **Inhalte einfügen** fragt, wohin die Kopien kommen: **An derselben
  Stelle**, **Um einen Abstand versetzt**, **Auf dem Rahmen zentriert**,
  **Auf dem Stichcursor zentriert** oder **Erster Stich am Stichcursor**.
- **Löschen** (Entf) löscht die Auswahl.

## Clipart

Clipart-Elemente sind Teile von Designs, die Sie wieder benutzen, zum
Beispiel ein Rahmen oder ein Logo.

- **Clipart** zeigt den Tab Clipart. Doppelklicken Sie auf ein Element,
  oder ziehen Sie es auf das Design.
- **Als Clipart speichern** behält die ausgewählten Objekte als Element,
  mit einem Namen und Stichwörtern für die Suche.

Siehe [Die Designbibliothek und Clipart](library).

## Auswählen

- **Alles auswählen** (Strg+A), **Weiter** (Tab) und **Zurück**
  (Umschalt+Tab).
- **Am Stichcursor** wählt das Objekt aus, in dem der Stichcursor ist.
- **Nach Farbe** wählt alle Objekte eines Garns aus. **Nach Art** wählt
  alle Steppstichlinien, alle Satinsäulen oder alle Füllungen aus.

## Teilen und verbinden

- **Messer**: Schneiden Sie die ausgewählten Objekte entlang einer Linie.
  Klicken Sie zwei Punkte.
- **Aufteilen**: ein Objekt für jeden getrennten Teil eines Objekts.
- **Verbinden**: ein Objekt aus den ausgewählten Objekten einer Art und
  Farbe: Linien und Säulen, deren Enden sich berühren, Füllungen, die sich
  berühren. Zwei Säulen, die sich an einer Ecke treffen (eine hört auf und
  die andere fängt darüber an), werden eine Säule, die an einer
  Gehrungsecke umbiegt. Es setzt wieder zusammen, was Aufteilen oder das
  Messer getrennt hat.
- **An Nadel teilen**: Schneiden Sie das Objekt unter dem Stichcursor dort
  in zwei, wo der Cursor ist.
- **Lücken schließen…**: verbindet die Enden der ausgewählten Linien, wo
  sie nahe beieinander liegen, sodass Linien, die Sie einzeln gezeichnet
  haben (Freihand, Linie), eine Linie oder eine geschlossene Form werden.
  Legen Sie die **Lücke** fest: Enden, die näher sind, werden verbunden,
  die nächsten Enden zuerst, in der Mitte zwischen ihnen. Das Fenster
  beginnt mit einer Lücke, die jedes Ende verbindet, das ein anderes Ende
  innerhalb von 10 mm hat. Mit
  **Überstände abschneiden** werden zwei Linien, die sich nahe ihren
  Enden kreuzen, am Kreuzungspunkt abgeschnitten. Eine Linie schließt
  sich, wenn ihre beiden Enden nahe sind. Linien verbinden sich nur mit
  Linien derselben Farbe. Die Arbeitsfläche zeigt das Ergebnis, während
  Sie die Lücke ändern; **OK** behält es. Um eine geschlossene Form als
  Füllung zu sticken, benutzen Sie **Konvertieren**.
- **Abschnitt kürzen**: Klicken Sie auf einen Teil einer Linie zwischen
  zwei Kreuzungen oder auf ein Ende nach der letzten Kreuzung: Dieser
  Teil wird entfernt. Die Kreuzungen sind mit anderen Linien, den
  Umrissen anderer Objekte und der Linie selbst. Benutzen Sie es für die
  Überstände von Hand gezeichneter Linien.

## Form

- **Winkel** und **Richtungslinie** legen die Richtung der Stiche fest.
  Siehe [Objekte zeichnen](digitizing).
- **Loch hinzufügen** zeichnet ein Loch in die ausgewählte Füllung.
  **Löcher entfernen** füllt die Löcher der Füllung (die Löcher werden
  überstickt). **Löcher füllen** erstellt in jedem Loch ein neues
  Füllobjekt.
- **Eckenradius**: rundet die Ecken einer Füllung oder einer Applikation
  mit geraden Seiten ab. Legen Sie einen Radius für **Alle Ecken** fest
  oder einen Radius für jede Ecke (nummeriert vom ersten Punkt des
  Umrisses an).
- **Loch aus Umriss**: fügt ein Loch mit der Form des Umrisses hinzu, mit
  einem Abstand innerhalb davon. Benutzen Sie es für einen Rahmen oder
  einen Rand.
- **Schnell vereinfachen**: weniger Knoten, mit den Standardeinstellungen.
  Die Form bleibt innerhalb von 0,2 mm.
- **Vereinfachen…**: weniger Knoten mit Ihren Einstellungen: die
  **Toleranz** (wie weit der neue Umriss vom alten entfernt sein kann),
  **Ecken** (eine Wendung, die schärfer ist als dieser Wert, bleibt eine
  Ecke) und **Kurven** oder **Gerade Linien** zwischen den Ecken. Das
  Fenster zeigt die Zahl der Knoten vorher und nachher. Vereinfachen fügt
  nie Knoten hinzu.
- **Bereinigen…**: entfernt kleine Fehler aus den ausgewählten Linien,
  Füllungen und Vektorobjekten, zum Beispiel aus einer SVG-Datei oder
  einer Nachzeichnung. Legen Sie die **Detailgröße** fest: **Kleine
  Schlaufen und Spitzen entfernen** (wo eine Linie sich selbst kreuzt oder
  hin und zurück läuft), **Zickzacks glätten** (Ecken bleiben scharf) und
  **Winzige Teile entfernen**. Die Teile eines Füllungsumrisses, die sich
  überlappen, werden eine Fläche. Die Arbeitsfläche zeigt das Ergebnis;
  **OK** behält es. Satinsäulen, Applikationen und Schriftzüge ändern
  sich nicht.
- **Hülle…**: biegt die ausgewählten Linien, Füllungen, Satinsäulen,
  Applikationen und Vektorobjekte in eine Form (Bogen, Wölbung, Wimpel,
  Perspektive und mehr), mit einer **Stärke**. Die Arbeitsfläche zeigt
  das Ergebnis; **OK** behält es. Schriftzüge haben eine eigene Hülle
  (Schrifteffekte).
- **Eigenschaften übertragen**: Wählen Sie die Objekte aus, klicken Sie
  auf Eigenschaften übertragen und dann auf das Objekt, von dem Sie
  übertragen. Die ausgewählten Objekte erhalten seine Farbe und, wo sie
  von derselben Art sind, seine Sticheinstellungen (Füllung zu Füllung,
  Linie zu Linie und so weiter). Ein Rückgängig-Schritt.
- **Wiederholen**: führt den letzten Menübandbefehl erneut aus (auch die
  **Leertaste** und das Rechtsklickmenü).

## Linien und Säulen

- **Linie fortsetzen**: Fügen Sie Punkte am Ende der ausgewählten Linie
  hinzu, oder an ihrem Anfang (klicken Sie zuerst nahe dem Anfang).
- **Redwork**: Verbinden Sie die Linien zu einem Objekt. Jede Linie wird
  zweimal gestickt, und das Objekt endet, wo es beginnt. Gut für Umrisse
  ohne Fadenschnitte.
- **Verzweigung**: Verbinden Sie die Linien zu einem Objekt, das mit den
  wenigsten zweimal gestickten Linien gestickt wird.
- **Form schließen** schließt offene Linien, Vektoren und Steinreihen mit
  einer geraden Linie. **Mit Kurve schließen** schließt sie mit einer
  Kurve, die die Enden fortsetzt.
- **Richtung umkehren**: Die Linien, Säulen, Vektoren und Steinreihen gehen
  in die andere Richtung. Anfang und Ende tauschen die Plätze.
- **Seiten tauschen**: Die zwei Seiten der ausgewählten Satinsäulen
  tauschen die Plätze, mit den Einstellungen jeder Seite.

## Sticharten

Der Bereich **Sticharten** legt die **Umrissart** von Linien, die
**Füllart** von Füllungen und die **Säulenart** von Säulen fest und hat
die besonderen Füllungen: Motive, Führungslinien, Teilungslinien, Tartans,
Verläufe und Wellen. Siehe [Sticharten](stitch-types).

## Maschinenfunktionen

Maschinenfunktionen sagen der Maschine, was sie nach einem Objekt tun
soll. Wählen Sie die Objekte aus, dann klicken Sie auf:

- **Stopp danach**: Die Maschine hält nach den ausgewählten Objekten an,
  zum Beispiel, um Applikationsstoff aufzulegen.
- **Immer schneiden**: immer nach den ausgewählten Objekten schneiden.
- **Nie schneiden**: nach den ausgewählten Objekten nicht schneiden. Eine
  lange Lücke wird zu einem Sprung.
- **Anfangsvernähen**: Vernähstiche, wo die ausgewählten Objekte nach
  einem Fadenschnitt beginnen.
- **Endvernähen**: Vernähstiche, wo die ausgewählten Objekte vor einem
  Fadenschnitt enden.

Objekteigenschaften > **Maschinenfunktionen** zeigt dieselben
Einstellungen für ein Objekt, mit **Danach schneiden: Automatisch**
(schneiden, wenn das nächste Objekt weit weg ist, sonst ein
Verbindungsstich), **Verdeckter Verbindungsstich** (wie Automatisch, aber
ein weit entferntes Objekt derselben Farbe bekommt Verbindungsstiche, wo
später gestickte Objekte den ganzen Weg bedecken) und **Langsam sticken**
(nur U01-Dateien behalten es). Die Stichliste zeigt jede Funktion; siehe
[Ein Design prüfen](checking).
