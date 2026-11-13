---
id: artwork
title: Vektorgrafik und Formen
group: design
keywords: position des rands, innen, außen, mittig, vektor, grafik, svg, pdf, ai, emf, wmf, importieren, grafik zeichnen, umwandeln, zu füllung, zu satin, zu umriss, zu vektor, umrisse erkennen, formen, verschmelzen, schnittmenge, ausschließen, kombinieren, vorne minus hinten, hinten minus vorne, abflachen, teilen, überlappungen entfernen, verknüpfung lösen, verknüpft, originale behalten, überlappung, randkontur, konturen, umrisse, rand hinzufügen, umrisse exportieren, schneideplotter
---
# Vektorgrafik und Formen

Vektorgrafik sind Formen ohne Stiche. Sie zeigt sich auf der
Arbeitsfläche, aber sie stickt nichts, und Maschinendateien lassen sie
weg. Sie benutzen sie als Vorlage, oder Sie wandeln sie in Stickerei um.

## Grafik importieren

Der Bereich **Grafik > Importieren** liest SVG-, PDF-, AI-, EMF- und
WMF-Dateien:

- **Als Vektoren importieren**: Die Pfade werden zu Vektorgrafik. Wandeln
  Sie sie später um.
- **Grafik importieren**: Die Pfade werden sofort zu Stickobjekten
  (Füllungen und Linien).
- **Als Linien importieren**: Der Umriss jeder Form und jeder Strich wird
  zu einer Steppstichlinie.
- **Stiche einfügen** fügt die Stiche einer Maschinendatei als
  Stichobjekte zum Design hinzu.

Sie können eine SVG- oder PDF-Datei auch auf das Fenster ziehen.

## Grafik zeichnen

Schalten Sie **Grafik > Grafik zeichnen** ein. Die Zeichenwerkzeuge (Form,
Linie, Freihand, Rechteck, Ellipse, Stern) erstellen dann Vektorgrafik
statt Stickobjekten. Schalten Sie es aus, um wieder Stickerei zu zeichnen.

**Vektoren** zeigt oder verbirgt die Grafik. **Vektoren abdunkeln** zeigt
sie blass, damit die Stiche hervortreten.

## Umwandeln

Wählen Sie Vektoren aus, dann:

- **Zu Füllung**: Jede geschlossene Form wird zu einer Füllung, mit ihren
  Löchern.
- **Zu Satin**: Jede geschlossene Form wird zu einem Satin. Eine lange Form
  bekommt eine drehende Säule entlang ihrer Mitte, der Ring eines O
  bekommt eine Ringsäule, und andere Formen bekommen eine Füllung mit der
  Stichart Satin. Kein Stich geht aus der Form hinaus oder über einen
  anderen Teil von ihr. Eine Linie wird zu einer Satinsäule entlang der
  Linie.
- **Zu Umriss**: Alle Pfade werden zu einer Steppstichlinie.

Andere Befehle des Bereichs:

- **Zu Vektor**: Die ausgewählten Stickobjekte werden zu Vektorgrafik
  (ihre Umrisse, keine Stiche).
- **Umrisse erkennen**: Stichobjekte (aus einer Maschinendatei oder einem
  Fotostich) werden zu Füllungen der Fläche, die ihre Stiche bedecken,
  damit Sie sie skalieren können und sie ihre Dichte behalten.

## Formen

Formoperationen erstellen neue Formen aus zwei oder mehr ausgewählten
Formen (Vektoren, Füllungen, Applikationen und anderen Objekten mit einer
Fläche). Die vordere Form ist die, die zuletzt gestickt wird.

| Befehl | Ergebnis |
|---|---|
| Verschmelzen | Eine Form aus allen ausgewählten Formen. |
| Schnittmenge | Nur die Fläche, die alle ausgewählten Formen bedecken. |
| Ausschließen | Die Flächen, die die Formen nicht gemeinsam haben. |
| Kombinieren | Ein Objekt aus den ausgewählten Vektoren oder aus den ausgewählten Applikationsobjekten. |
| Vorne minus hinten | Die vordere Form, aus der die anderen ausgeschnitten sind. |
| Hinten minus vorne | Die hintere Form, aus der die anderen ausgeschnitten sind. |
| Abflachen | Jede Form ohne die Teile, die die Formen davor verdecken, damit nichts doppelt gestickt wird. |
| Teilen | Die Formen werden dort in Stücke geschnitten, wo sie sich überlappen; jede Überlappung geht an die vordere Form. |
| Überlappungen entfernen | Die ausgewählten Formen werden aus den Objekten darunter ausgeschnitten. |

Die Optionen des Bereichs:

- **Originale behalten**: Die ausgewählten Objekte bleiben; das Ergebnis
  kommt danach.
- **Verknüpft lassen**: Abflachen, Teilen und Überlappungen entfernen
  schneiden neu, wenn sich ihre Objekte bewegen. **Verknüpfung lösen**
  beendet die Verknüpfungen der Auswahl; die Objekte bleiben, wie sie
  sind.
- **Überlappung**: wie weit ein geschnittenes Stück unter die Form geht,
  die es geschnitten hat, damit zwischen den Stücken kein Stoff zu sehen
  ist. Überlappungen entfernen und das Messer benutzen sie auch.

Schriftzüge bleiben in Abflachen und Überlappungen entfernen
Schriftzüge. Ein Schriftzug davor schneidet die Objekte darunter mit
seinen Buchstaben aus. Ein Schriftzug unter anderen Objekten erhält eine
Aussparung: Seine Stiche lassen die Fläche der Objekte davor aus, und
Text, Schriftart und Buchstaben bleiben änderbar. Die anderen Befehle
von Formen benutzen keine Schriftzüge.

## Randkontur

**Grafik > Randkontur** fügt einen Rand um die ausgewählten Objekte hinzu,
eine Lücke um sie herum oder beides, ausgeschnitten aus den Objekten
darunter. In der Lücke ist der Stoff zu sehen.

- **Randbreite**, der **Stich** (eine Säulenart, ein Steppstich oder ein
  dreifacher Steppstich) und das **Garn**.
- **Position des Rands**: wo der Rand auf der Kante der Objekte sitzt.
  **Außerhalb der Linie**: Der Rand geht um die Objekte herum und bedeckt
  nur ein wenig ihrer Kante (eine Säule geht 0,4 mm darüber, damit kein
  Stoff zu sehen ist). **Mittig auf der Linie**: Die Mitte des Rands ist
  auf der Kante. **Innerhalb der Linie**: Der Rand bedeckt die Kante der
  Objekte, und das Design wird nicht größer.
- **Stickreihenfolge**: **Objekte, dann Rand** oder **Rand, dann
  Objekte**.
- **Lückenbreite**: der Raum um den Rand (oder um die Objekte, ohne Rand),
  in dem der Stoff zu sehen ist.

Die Kontur bleibt verknüpft: Sie wird neu geschnitten, wenn sich ihre
Objekte bewegen. Ändern Sie sie im Fenster, während ihre Objekte
ausgewählt sind. **Randkontur entfernen** bringt die geschnittenen Objekte
zurück.

Die Objekte und ihr Rand sind eine Gruppe, darum wählt ein Klick alle aus.
Um die Stiche eines von ihnen zu ändern (zum Beispiel die Füllart),
doppelklicken Sie darauf, oder klicken Sie auf seine Schaltfläche unter
**Objekte der Gruppe** im Bereich Eigenschaften. Aufteilen und Verbinden
wirken nicht auf verknüpfte Objekte: Benutzen Sie zuerst **Grafik >
Verknüpfung lösen**.

## Konturen

**Grafik > Konturen** fügt Umrisse mit einem Abstand um die ausgewählten
Objekte herum (**Außen**) oder in ihnen (**Innen**) hinzu:

- **Entfernung** des ersten Umrisses, die Zahl der **Umrisse** und der
  **Abstand** zwischen ihnen.
- **Ecken**: **Rund**, **Spitz** oder **Ausschneiden**.
- **Erstellen**: eine **Linie**, einen **Vektor** oder einen Satin-**Rand**
  einer Breite.
- **Position des Rands** (für einen Rand): **Außerhalb der Linie** (weg
  von der Mitte der Objekte, damit ein Rand im Abstand 0 sie nicht
  bedeckt), **Mittig auf der Linie** oder **Innerhalb der Linie** (zur
  Mitte der Objekte hin). Außen und innen bleiben für Umrisse in den
  Objekten gleich.

**Rand hinzufügen** öffnet dasselbe Fenster, eingestellt auf einen
Satinrand, mit der Position des Rands, die Sie zuletzt benutzt haben.

## Umrisse exportieren

**Grafik > Umrisse exportieren** schreibt die Formen der Objekte als SVG-
oder EMF-Pfade, für Vektorprogramme und Schneideplotter. Für
Applikationsteile benutzen Sie **Digitalisieren > Schneideplotter-Datei**.
