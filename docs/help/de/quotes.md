---
id: quotes
title: Aufträge, Angebote und Laufzeiten
group: production
keywords: auftragsdetails, kunde, auftragsnummer, größen, platzierung, liefertermin, barcode, angebot, preis, preisliste, kosten, schätzung, auftrag, menge, stiche, 1000 stiche, einrichtung, digitalisierung, eilauftrag, steuer, kleidungsstück, köpfe, mehrkopf, einspannen, laufzeit, produktionszeit, stück pro stunde
---
# Aufträge, Angebote und Laufzeiten

**Design > Auftragsdetails** behält den Auftrag des Designs: den Kunden,
die Auftragsnummer, das Kleidungsstück, die Größen, die Platzierung und
den Liefertermin. **Design > Angebot** gibt den Preis des Auftrags aus
Ihren Preislisten und die Zeit, ihn auf Ihrer Maschine zu sticken. Es
macht auch ein Angebotsblatt für den Kunden.

## Auftragsdetails

- **Auftragsnummer**, **Kunde**, **Kleidungsstück** und **Farbe des
  Kleidungsstücks**.
- **Platzierung**: Tippen Sie sie ein, oder wählen Sie eine Stelle aus dem
  Menü (die Stellen der Produkte, Design > Produkt).
- **Liefertermin**: als `YYYY-MM-DD`. **Heute** setzt das heutige Datum.
- **Größen**: **Größe hinzufügen** für jede Größe und ihre Stücke, oder
  **Übliche Größen hinzufügen** (XS bis XXL). Ohne Größen legen Sie die
  **Menge** fest.
- **Notizen zu diesem Auftrag**: Sie kommen auf den Blättern nach den
  Designnotizen.

Der Auftrag wird mit dem Design gespeichert, und Rückgängig nimmt eine
Änderung zurück. Eine Vorlage behält keinen Auftrag: Ein neues Design aus
einer Vorlage beginnt ohne Auftrag. **Auftrag löschen** entfernt den
Auftrag des Designs.

Wo der Auftrag erscheint:

- Das Produktions-Arbeitsblatt beginnt seine Liste mit dem Auftrag. Sein
  Barcode hat die Auftragsnummer (sonst den Dateinamen).
- Das Freigabeblatt hat den Auftrag und, wenn die Preisliste Preise hat,
  den Preis.
- Das Angebotsblatt hat den Auftrag.
- Die CSV der Designeigenschaften hat die Spalten order, customer,
  garment, quantity, placement und due.
- Mit **Auftragsnummer vor den Dateinamen** (Einstellungen von Design
  ausgeben) heißen die Dateien zum Beispiel `A-1042 rose.dst`.
- Die Designinfo zeigt die Auftragsnummer und den Kunden.

## Preislisten

Öffnen Sie **Preise** im Fenster Angebot und legen Sie die Preise der
Liste fest. Ein Preis von 0 fügt nichts hinzu.

- **Je 1000 Stiche**: der Preis von 1000 Stichen eines Stücks.
- **Mindestpreis je Stück**: Ein kleines Design kostet für jedes Stück
  mindestens so viel.
- **Farben im Preis** und **Jede zusätzliche Farbe**: Der Stichpreis
  enthält einige Garne; jedes Garn über dieser Zahl kostet mehr, für jedes
  Stück. Ein Garn, das zweimal stickt, ist ein Garn.
- **Einrichtung**: einmal für jeden Auftrag.
- **Digitalisierung** und **Digitalisierung, je 1000 Stiche**: einmal, nur
  für ein neues Design.
- **Mengenpreise**: ab einer Menge ein Prozent weniger vom Stickpreis. Die
  größte Menge, die der Auftrag erreicht, gilt.
- **Eilauftrag**: ein Prozent, das zur Summe eines Eilauftrags hinzukommt.
- **Steuer**: ein Prozent der Summe mit dem Eilpreis.
- **Währung**: der Text vor den Zahlen, zum Beispiel `$` oder `€`. Die
  Garnkosten in den Optionen benutzen dieselbe Währung.

Klicken Sie auf **Neue Liste**, um eine Kopie der Liste zu machen, zum
Beispiel eine Liste für Hemden und eine für Kappen. **Liste entfernen**
entfernt die Liste, die angezeigt wird.

## Der Auftrag

Legen Sie die **Menge** fest (mit Größen die Summe der Größen der
Auftragsdetails), den **Preis des Kleidungsstücks** (wenn Sie die
Kleidungsstücke liefern; 0, wenn der Kunde sie liefert), **Neues Design**
(fügt den Digitalisierungspreis hinzu) und **Eilauftrag**. Sie sind Teil
des Auftrags des Designs, und die Preisliste, die Sie wählen, wird mit ihm
behalten. Das Fenster zeigt jede Zeile des Preises, die Summe und den
Preis jedes Stücks. Jede Zeile ist in Cent, damit die Zeilen die Summe
ergeben.

Die Garnkosten jedes Stücks (aus den Optionen) erscheinen unter dem
Preis. Sie sind für Sie: Sie stehen nicht auf dem Angebotsblatt.

## Laufzeit

Die Laufzeit ist die Zeit, den ganzen Auftrag zu sticken. Sie benutzt die
Zeiten von **Design > Maschineneinstellungen**:

- **Köpfe**: die Stücke, die die Maschine auf einmal stickt. Ein Zyklus
  stickt ein Stück auf jedem Kopf.
- **Ein Stück einspannen**: die Zeit, ein Stück einzuspannen und es nach
  dem Sticken aus dem Rahmen zu nehmen.
- **Rahmenwechsel**: die Zeit, die Rahmen von der Maschine zu nehmen und
  die nächsten Rahmen aufzusetzen.
- **Die nächsten Stücke einspannen, während die Maschine stickt**: Sie
  haben einen zweiten Satz Rahmen. Dann dauert ein Zyklus so lange wie das
  Längere von der Stickzeit und der Zeit, die Stücke aller Köpfe
  einzuspannen. Ohne diese Option addieren sich die Zeiten.

Die Stickzeit eines Stücks ist die Stickzeit der Designinfo, mit den
Farbwechseln, Fadenschnitten, Stopps und Garnwechseln. Das Fenster sagt,
ob die Maschine oder die Bedienperson das Tempo bestimmt. Wenn die
Bedienperson das Tempo bestimmt, macht eine zweite Person zum Einspannen
der Stücke den Auftrag schneller.

Die Designinfo zeigt den Preis jedes Stücks und die Laufzeit der Menge im
Fenster Angebot.

## Angebotsblatt

**Angebotsblatt speichern** schreibt eine PDF-Seite mit dem Designbild,
den Zeilen des Preises, der Summe und Linien zum Unterschreiben. Die
Papiergröße ist das Papier der Ausgabeeinstellungen.

## Aus der Befehlszeile

`emb-cli quote design.embproj --prices prices.toml --quantity 48 --machine tajima --heads 6`
gibt dieselben Zeilen und die Laufzeit aus. Ohne `--quantity`,
`--new-design`, `--rush` oder `--garment` gibt der Auftrag des Designs
sie an. Die Preislistendatei hat die Felder einer Preisliste, zum
Beispiel:

```toml
name = "Shirts"
per_1000 = 1.50
min_piece = 5.00
setup = 20.00

[[breaks]]
from = 24
percent_off = 10
```

`--sheet quote.pdf` schreibt das Angebotsblatt.
