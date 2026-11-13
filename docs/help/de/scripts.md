---
id: scripts
title: Skripte
group: reference
keywords: skript, skripte, rhai, automatisierung, makro, werkzeug, stapel, plugin, kontur um jede füllung, satindichte, garne benennen, vor der ausgabe prüfen, emb-cli script
---
# Skripte

Ein Skript ist ein kleines Werkzeug, das das Design ändert, zum Beispiel
eine Kontur um jede Füllung oder dieselbe Dichte für alle Satins.
**Bearbeiten > Skripte** führt die Beispielskripte des Programms und Ihre
eigenen Skripte aus.

## Ein Skript ausführen

1. Wählen Sie die Objekte aus (oder wählen Sie nichts aus: Das Skript
   bekommt dann das ganze Design).
2. Klicken Sie auf **Bearbeiten > Skripte** und wählen Sie ein Skript. Das
   Fenster zeigt, was es tut, und seine Einstellungen.
3. Legen Sie die Einstellungen fest und klicken Sie auf **Skript
   ausführen**.

Die Meldungen des Skripts erscheinen unter **Meldungen**. **Rückgängig**
nimmt alle Änderungen des Skripts in einem Schritt zurück. Wenn das Skript
mit einem Fehler stoppt, ändert sich das Design nicht.

Ein Skript kann nur das Design ändern. Es kann keine Dateien öffnen oder
speichern, nicht das Netzwerk benutzen und keine Programme starten. Ein
Skript, das länger als 5 Sekunden läuft, stoppt.

## Ihre eigenen Skripte

Skripte sind Textdateien mit der Endung `.rhai`, in der Sprache Rhai
(siehe rhai.rs). Legen Sie sie in den Ordner `scripts` des
Einstellungsordners: **Skriptordner öffnen** öffnet ihn, und **In meine
Skripte kopieren** kopiert ein Beispiel dorthin, das Sie ändern können.
Klicken Sie auf den runden Pfeil, um den Ordner neu zu lesen, nachdem Sie
eine Datei geändert haben.

Die ersten Zeilen, die mit `//!` beginnen, beschreiben das Skript:

```rhai
//! Outline Every Fill
//! Adds a running outline round each fill.
//! input distance: 0.0 "Distance from the edge (mm)"
```

Die erste Zeile ist der Titel, die nächsten Zeilen sind die Beschreibung,
und jede Zeile `input` ist eine Einstellung: ein Name, der Standardwert
(eine Zahl, `true` oder `false`, oder ein Text in Anführungszeichen) und
die Beschriftung. Das Skript liest die Werte als `input.distance`.

## Funktionen

Objekte sind Zahlen (`id`), Garne sind Zahlen (`t`). Größen sind in mm.

Das Design:

- `objects()`: alle Objekte in Stickreihenfolge. `selection()`: die
  ausgewählten Objekte. `select([id, …])`: die Auswahl nach dem Skript.
- `design_name()`, `set_design_name(text)`, `notes()`, `set_notes(text)`.
- `info()`: die Zahlen der Stiche: `stitches`, `colors`,
  `color_changes`, `trims`, `jumps`, `width_mm`, `height_mm`, `warnings`.

Ein Objekt:

- `kind(id)`: `"fill"`, `"satin"`, `"running"`, `"applique"`,
  `"lettering"`, `"vector"`, `"stitches"`, `"cross_stitch"`, `"bling"` oder
  `"sketch"`.
- `name(id)`, `set_name(id, text)`; `thread(id)`, `set_thread(id, t)`.
- `size(id)`: `x`, `y`, `width`, `height`.
- `settings(id)`: die Stichparameter als Map, mit den Namen der
  Projektdatei (zum Beispiel `spacing_mm` einer Füllung oder eines
  Satins). `set_settings(id, map)` setzt sie:

  ```rhai
  let s = settings(id);
  s.spacing_mm = 0.4;
  set_settings(id, s);
  ```

- `sew_index(id)`, `move_to(id, index)`: die Stelle in der
  Stickreihenfolge (0 ist die erste).
- `remove(id)`.

Mehr Objekte:

- `move_objects([id, …], dx, dy)`, `rotate_objects([id, …], degrees)`,
  `scale_objects([id, …], factor)` (um die Mitte der Objekte).
- `add_outline([id, …], distance)`: eine Steppstichkontur (außen, wenn der
  Abstand größer als 0 ist, innen, wenn kleiner). `add_border([id, …],
  width)`: ein Satinrand außen. Beide geben die neuen Objekte zurück.
- `shape([id, …], "weld")`: Formen (`weld`, `intersect`, `exclude`,
  `front_minus_back`, `back_minus_front`, `flatten`, `divide`, `combine`).
  Es gibt die neuen Objekte zurück.

Garne:

- `threads()`: eine Liste von Maps mit `id`, `name`, `color`
  (`"#rrggbb"`), `brand` und `code`.
- `add_thread(name, "#rrggbb")`: ein neues Garn.
- `set_thread_name(t, text)`, `set_thread_color(t, "#rrggbb")`,
  `set_thread_code(t, brand, code)`.

Meldungen und Fehler:

- `print(text)`: eine Meldung im Fenster.
- `throw "text"`: mit einem Fehler stoppen; das Design ändert sich nicht.

## Aus der Befehlszeile

`emb-cli script my-tool.rhai design.embproj --out done` führt das Skript
für jede Projektdatei aus und speichert die geänderten Dateien im Ordner
`done`. `--set distance=1.5` setzt eine Eingabe. Ohne `--out` gibt es nur
die Meldungen aus, und nichts wird gespeichert.
