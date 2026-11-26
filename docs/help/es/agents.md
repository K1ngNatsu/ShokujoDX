---
id: agents
title: Agentes (MCP)
group: reference
keywords: agente, ia, asistente, mcp, model context protocol, herramientas, servidor, claude, chat, automatización, emb-cli mcp, mostrar, archivo cambiado, abrir de nuevo
---
# Agentes (MCP)

Un agente de IA propio (un programa asistente que puede usar
herramientas) puede crear y cambiar diseños con ShokujoDX. El programa
no tiene IA y no usa la red: el agente inicia la herramienta de línea de
comandos `emb-cli` del programa, y `emb-cli mcp` da al agente las
herramientas del programa por el Model Context Protocol (MCP).

## Configurar

Añada un servidor en los ajustes MCP de su agente, con este comando (la
carpeta de ShokujoDX, y después `emb-cli mcp`):

```text
"C:\Program Files\ShokujoDX\emb-cli.exe" mcp
```

En un Mac la herramienta está dentro del programa:
`ShokujoDX.app/Contents/MacOS/emb-cli`. En Linux es `emb-cli` junto a
`shokujodx`. Muchos agentes guardan sus servidores en un archivo JSON:

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

## Lo que puede hacer el agente

El servidor guarda un diseño. Sus herramientas:

- empezar un diseño nuevo o abrir uno (un proyecto, un archivo de
  máquina o un archivo SVG), cambiar su máquina, su bastidor y su tela, y
  guardarlo;
- listar los objetos, añadir formas con texto de trazado SVG (líneas,
  rellenos, columnas de satén y formas vectoriales) y añadir letras en
  las fuentes de bordado del programa;
- cambiar los ajustes y el hilo de los objetos, moverlos, escalarlos,
  girarlos y borrarlos, y ejecutar un script (vea [Scripts](scripts));
- comprobar el diseño, hacer una imagen con el aspecto 3D del hilo y
  exportar archivos de máquina, hojas de trabajo y hojas de aprobación;
- hacer muestras de prueba, hacer el diseño más rápido de bordar, y
  comprobar y reparar archivos de máquina;
- deshacer y rehacer.

Cada cambio es un paso de deshacer. Las medidas son en mm desde el
centro del bastidor. Una herramienta que solo lee lo dice, y una
herramienta no acepta un argumento que no conoce.

## Ver el diseño en el programa

La herramienta **show** guarda el diseño y lo abre en ShokujoDX: en una
pestaña nueva del programa abierto (o en su pestaña, si ya está abierto),
o si no, en un programa nuevo. Un diseño sin archivo va a un archivo de
la carpeta de los archivos temporales y sigue sin guardar:
**save_design** lo guarda.

Cuando el agente guarda un archivo que está abierto en una pestaña, el
programa pregunta por él. **Abrir de nuevo** muestra la versión nueva
(los cambios no guardados de la pestaña se pierden). **Conservar esta
versión** deja la pestaña como está, y la pestaña queda sin guardar (al
cerrarla, el programa pregunta).

## Pruébelo

Pida a su agente un diseño en una frase, por ejemplo: «Haz una insignia
redonda de 60 mm de ancho con la palabra STAR en rojo y un borde de
satén azul, y muéstrala en ShokujoDX».
