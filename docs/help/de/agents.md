---
id: agents
title: Agenten (MCP)
group: reference
keywords: agent, ki, assistent, mcp, model context protocol, werkzeuge, server, claude, chat, automatisierung, emb-cli mcp, anzeigen, datei geändert, erneut öffnen
---
# Agenten (MCP)

Ein eigener KI-Agent (ein Assistenzprogramm, das Werkzeuge verwenden
kann) kann mit ShokujoDX Designs erstellen und ändern. Das Programm
enthält keine KI und verwendet kein Netzwerk: Der Agent startet das
Befehlszeilenwerkzeug `emb-cli` des Programms, und `emb-cli mcp` gibt dem
Agenten die Werkzeuge des Programms über das Model Context Protocol
(MCP).

## Einrichten

Fügen Sie in den MCP-Einstellungen Ihres Agenten einen Server mit diesem
Befehl hinzu (der Ordner von ShokujoDX, dann `emb-cli mcp`):

```text
"C:\Program Files\ShokujoDX\emb-cli.exe" mcp
```

Auf einem Mac ist das Werkzeug im Programm:
`ShokujoDX.app/Contents/MacOS/emb-cli`. Unter Linux ist es `emb-cli`
neben `shokujodx`. Viele Agenten speichern ihre Server in einer
JSON-Datei:

```json
{
  "mcpServers": {
    "shokujodx": {
      "command": "C:\\Program Files\\ShokujoDX\\emb-cli.exe",
      "args": ["mcp"]
    }
  }
}
```

## Was der Agent tun kann

Der Server hält ein Design. Seine Werkzeuge:

- ein neues Design beginnen oder eines öffnen (ein Projekt, eine
  Maschinendatei oder eine SVG-Datei), seine Maschine, seinen Rahmen und
  seinen Stoff ändern und es speichern;
- die Objekte auflisten, Formen aus SVG-Pfadtext hinzufügen (Linien,
  Füllungen, Satinsäulen und Vektorformen) und Schriftzüge in den
  Stickschriften des Programms hinzufügen;
- die Einstellungen und das Garn von Objekten ändern, sie verschieben,
  skalieren, drehen und löschen und ein Skript ausführen (siehe
  [Skripte](scripts));
- das Design prüfen, ein Bild davon mit dem 3D-Fadenaussehen erstellen
  und Maschinendateien, Arbeitsblätter und Freigabeblätter exportieren;
- Testmuster erstellen, das Design schneller stickbar machen und
  Maschinendateien prüfen und reparieren;
- rückgängig machen und wiederholen.

Jede Änderung ist ein Rückgängig-Schritt. Maße sind in mm ab der Mitte
des Rahmens. Ein Werkzeug, das nur liest, sagt das, und ein Werkzeug
nimmt kein Argument an, das es nicht kennt.

## Das Design im Programm sehen

Das Werkzeug **show** speichert das Design und öffnet es in ShokujoDX:
in einem neuen Tab des laufenden Programms, sonst in einem neuen
Programm.

Wenn der Agent eine Datei speichert, die in einem Tab geöffnet ist,
fragt das Programm danach. **Erneut öffnen** zeigt die neue Version (die
nicht gespeicherten Änderungen des Tabs gehen verloren). **Diese Version
behalten** lässt den Tab, wie er ist.

## Ausprobieren

Bitten Sie Ihren Agenten in einem Satz um ein Design, zum Beispiel:
„Erstelle ein rundes Abzeichen mit 60 mm Breite, mit dem Wort STAR in Rot
und einem blauen Satinrand, und zeige es in ShokujoDX."
