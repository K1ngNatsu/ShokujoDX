---
id: machines-and-hoops
title: Maschinen und Rahmen
group: production
keywords: maschine, topaz, husqvarna, brother, tajima, mehrnadel, nadeln, nadel, nadelfolge, garnwechsel, nadelplan, format, tempo, stickzeit, zeit für farbwechsel, zeit für fadenschnitt, dst fadenschnitte, sprünge für einen fadenschnitt, rahmen, feld, rand, meine rahmen, eigener rahmen, automatischer rahmen, rahmen drehen, design drehen, rahmenwinkel, rahmenoptionen, platzierung, automatisch zentrieren, anfangspunkt, endpunkt, rahmenschablone, drucken, mehrposition
---
# Maschinen und Rahmen

## Die Maschine

Die Liste **Maschine** (Design > Maschine und Rahmen) legt die Maschine
des Designs fest. Die Maschine legt fest:

- das Dateiformat von Exportieren und Auf USB senden (zum Beispiel VP3 für
  die Husqvarna Viking Designer Topaz 20, PES für Brother, DST für die
  meisten Mehrnadelmaschinen);
- den längsten Stich und die Rahmen der Maschine;
- das Tempo und die Zeiten für die Stickzeit.

**Maschineneinstellungen** (Design > Maschine und Rahmen) hat:

- **Tempo** (Stiche pro Minute) und die Zeit eines **Farbwechsels**, eines
  **Fadenschnitts** und eines **Stopps**. Die Designinfo und das
  Arbeitsblatt benutzen sie für die Stickzeit.
- **Fadenschnitte in DST- und TAP-Dateien**: Diese Formate haben keinen
  Code für den Fadenschnitt, darum sagen einige kurze Sprünge der Maschine,
  dass sie den Faden schneiden soll. Ändern Sie **Sprünge für einen
  Fadenschnitt** und **Länge jedes Sprungs** nur, wenn Ihre Maschine nicht
  schneidet oder zu oft schneidet.
- **Köpfe**, **Ein Stück einspannen**, **Rahmenwechsel** und **Die nächsten
  Stücke einspannen, während die Maschine stickt**: für die Laufzeit eines
  Auftrags. Siehe [Aufträge, Angebote und Laufzeiten](quotes).

## Nadeln einer Mehrnadelmaschine

Eine Mehrnadelmaschine hat auf jeder Nadel ein Garn und geht von selbst zur
Nadel der nächsten Farbe. **Nadeln** (Design > Maschine und Rahmen) zeigt:

- **Nadeln der Maschine**: die Zahl der Nadeln. Die Mehrnadelmaschinen
  haben 15; legen Sie die Zahl Ihrer Maschine fest. **Zurücksetzen**
  benutzt die Zahl des Maschinenprofils. Eine Maschine mit einer Nadel hat
  keinen Nadelplan.
- **Farben in Stickreihenfolge**: die Nadel jeder Farbe. **Auto** lässt ein
  Garn auf der Nadel, die es hat (eine Farbe, die zweimal gestickt wird,
  benutzt eine Nadel), und setzt ein neues Garn auf eine freie Nadel.
  Wählen Sie eine Nadelnummer, um dem Garn eine eigene Nadel zu geben;
  Rückgängig nimmt es zurück.
- **Nadeln am Anfang**: das Garn, das jede Nadel vor dem Start haben muss,
  und das Garn, das jetzt auf der Maschine ist. **einfädeln** zeigt die
  Garne, die zu wechseln sind.
- **Nadelfolge**: die Nadel jeder Farbe in Stickreihenfolge. Tippen Sie sie
  für DST-, TAP- und EXP-Dateien an der Maschine ein; TBF- und U01-Dateien
  haben die Nadeln.
- **Diese Fäden auf der Maschine lassen**: Nachdem Sie das Design gestickt
  haben, sind seine Garne auf den Nadeln. Das nächste Design benutzt sie,
  wo es kann, damit Sie weniger Garne wechseln. **Maschinenfäden löschen**
  vergisst sie.

Wenn das Design mehr Garne hat, als die Maschine Nadeln hat, bekommt eine
Nadel während des Designs ein neues Garn: ein **Garnwechsel** (*). TBF-
und U01-Dateien halten die Maschine dort an, das Arbeitsblatt sagt, welches
Garn auf welche Nadel kommt, und die Stickzeit zählt einen Stopp. Die
Designinfo zeigt eine Warnung.

Das Produktions-Arbeitsblatt hat eine Spalte **Nadel**, die **Nadeln am
Anfang**, die Nadelfolge und die Garnwechsel. Eine geöffnete TBF- oder
U01-Datei behält ihre Nadeln.

## Der Rahmen

Die Liste **Rahmen** legt den Rahmen fest. Die Arbeitsfläche zeigt das
Rahmenfeld (wohin die Nadel gehen kann) und den Sicherheitsrand.

- **Meine Rahmen** kommt zuerst in der Liste. Ein Rahmen, den die Maschine
  nicht benutzen kann, zeigt eine Warnung.
- **Freies Feld** hat keinen Rahmen.
- **Automatischer Rahmen** wählt den kleinsten Rahmen, in den das Design
  passt, aus Meine Rahmen (sonst aus den Rahmen der Maschine). Das Design
  dreht sich um 90°, wenn es so in einen kleineren Rahmen passt.
- **Rahmen zeigen** zeigt oder verbirgt den Rahmen auf der Arbeitsfläche.
- Der **Sicherheitsabstand im Rahmen** (Optionen) ist ein Abstand innerhalb
  des Felds. Stiche außerhalb des Rands bekommen eine Warnung in der
  Designinfo; Stiche außerhalb des Felds sind ein Fehler.

Oben auf der Arbeitsfläche ist die Rückseite der Maschine, und die
Rahmenbreite geht an der Maschine von links nach rechts.

## Das Design oder den Rahmen drehen

- **Design drehen** dreht das ganze Design im Rahmen um 90° im
  Uhrzeigersinn.
- **Rahmen nach links drehen** und **Rahmen nach rechts drehen** drehen den
  Rahmen um 15°, damit ein Design schräg hineinpasst. Die Maschinendatei
  dreht die Stiche zurück, damit das Design so gestickt wird, wie Sie es
  sehen.
- **Rahmenoptionen > Rahmenwinkel** legt den Winkel als Zahl fest.

## Meine Rahmen

**Design > Meine Rahmen** listet die Rahmen auf. Markieren Sie **Habe ich**
für die Rahmen, die Sie haben: Automatischer Rahmen und die Rahmenliste
benutzen sie zuerst.

**Neuer Rahmen…** fügt einen eigenen Rahmen hinzu: den **Namen**, die
**Form** (Rechteck, abgerundetes Rechteck, Oval oder Kreis), die **Breite**
und **Höhe** des Felds und den **Eckenradius**. Die Größe ist das
stickbare Feld, nicht der Rahmen selbst. Ein Mehrpositionsrahmen hat
**Positionen**: die Mitte jeder Position von der Rahmenmitte aus.
**Bearbeiten** und **Löschen** ändern Ihre Rahmen.

## Rahmenoptionen

**Design > Rahmenoptionen** hat:

- **Platzierung**: **Manuell** (der Rahmen bleibt, wo Sie ihn hinsetzen)
  oder **Automatisch zentrieren** (die Rahmenmitte folgt der Mitte des
  Designs).
- **Den Rahmen zeigen** und den **Rahmenwinkel**.
- **Automatischer Anfang und Ende**: Die Nadel geht vor dem ersten Stich
  zum Anfangspunkt und nach dem letzten Fadenschnitt zum Endpunkt. Wählen
  Sie die Rahmenmitte, eine Stelle am Rahmen des Designs (zum Beispiel
  **Oben links**) oder **Aus**. Dort wird kein Stich gestickt. VP3-Dateien
  haben keine Sprünge, darum legt der Anfangspunkt dort nur fest, wo das
  erste Objekt beginnt.

## Die Rahmenschablone

**Design > Rahmenschablone** speichert das Rahmenfeld in voller Größe als
PDF, mit den Mittellinien und dem Design. Drucken Sie es mit 100 % und
legen Sie es auf den Stoff, um das Design an die richtige Stelle zu
setzen. Ein großer Rahmen wird auf mehr Blätter gedruckt.

## Designs, die größer als der Rahmen sind

Siehe [Designs, die größer als der Rahmen sind](hooping).
