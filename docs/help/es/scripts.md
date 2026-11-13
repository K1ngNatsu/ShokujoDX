---
id: scripts
title: Scripts
group: reference
keywords: script, scripts, rhai, automatización, macro, herramienta, lotes, complemento, contorno en cada relleno, densidad de satén, nombrar hilos, comprobar antes de producir, emb-cli script
---
# Scripts

Un script es una pequeña herramienta que cambia el diseño, por ejemplo un
contorno alrededor de cada relleno, o la misma densidad para todos los
satenes. **Editar > Scripts** ejecuta los scripts de ejemplo del programa
y sus propios scripts.

## Ejecutar un script

1. Seleccione los objetos (o no seleccione nada: entonces el script recibe
   todo el diseño).
2. Haga clic en **Editar > Scripts** y elija un script. La ventana muestra
   lo que hace y sus ajustes.
3. Defina los ajustes y haga clic en **Ejecutar script**.

Los mensajes del script aparecen en **Mensajes**. **Deshacer** devuelve
todos los cambios del script en un paso. Si el script se detiene con un
error, el diseño no cambia.

Un script solo puede cambiar el diseño. No puede abrir ni guardar
archivos, usar la red ni iniciar programas. Un script que dura más de 5
segundos se detiene.

## Sus propios scripts

Los scripts son archivos de texto con la extensión `.rhai`, en el lenguaje
Rhai (vea rhai.rs). Póngalos en la carpeta `scripts` de la carpeta de
ajustes: **Abrir la carpeta de scripts** la abre, y **Copiar a mis
scripts** copia allí un ejemplo para que lo cambie. Haga clic en la flecha
redonda para leer otra vez la carpeta después de cambiar un archivo.

Las primeras líneas que empiezan con `//!` describen el script:

```rhai
//! Outline Every Fill
//! Adds a running outline round each fill.
//! input distance: 0.0 "Distance from the edge (mm)"
```

La primera línea es el título, las líneas siguientes son la descripción,
y cada línea `input` es un ajuste: un nombre, el valor predeterminado (un
número, `true` o `false`, o un texto entre comillas) y la etiqueta. El
script lee los valores como `input.distance`.

## Funciones

Los objetos son números (`id`), los hilos son números (`t`). Los tamaños
están en mm.

El diseño:

- `objects()`: todos los objetos en orden de bordado. `selection()`: los
  objetos seleccionados. `select([id, …])`: la selección después del
  script.
- `design_name()`, `set_design_name(text)`, `notes()`, `set_notes(text)`.
- `info()`: los números de las puntadas: `stitches`, `colors`,
  `color_changes`, `trims`, `jumps`, `width_mm`, `height_mm`, `warnings`.

Un objeto:

- `kind(id)`: `"fill"`, `"satin"`, `"running"`, `"applique"`,
  `"lettering"`, `"vector"`, `"stitches"`, `"cross_stitch"`, `"bling"` o
  `"sketch"`.
- `name(id)`, `set_name(id, text)`; `thread(id)`, `set_thread(id, t)`.
- `size(id)`: `x`, `y`, `width`, `height`.
- `settings(id)`: los ajustes de puntada como un mapa, con los nombres del
  archivo de proyecto (por ejemplo `spacing_mm` de un relleno o un satén).
  `set_settings(id, map)` los define:

  ```rhai
  let s = settings(id);
  s.spacing_mm = 0.4;
  set_settings(id, s);
  ```

- `sew_index(id)`, `move_to(id, index)`: el lugar en el orden de bordado
  (0 es el primero).
- `remove(id)`.

Más objetos:

- `move_objects([id, …], dx, dy)`, `rotate_objects([id, …], degrees)`,
  `scale_objects([id, …], factor)` (alrededor del centro de los objetos).
- `add_outline([id, …], distance)`: un contorno de puntada recta (hacia
  fuera si la distancia es mayor que 0, hacia dentro si es menor).
  `add_border([id, …], width)`: un borde de satén por fuera. Los dos
  devuelven los objetos nuevos.
- `shape([id, …], "weld")`: formas (`weld`, `intersect`, `exclude`,
  `front_minus_back`, `back_minus_front`, `flatten`, `divide`, `combine`).
  Devuelve los objetos nuevos.

Hilos:

- `threads()`: una lista de mapas con `id`, `name`, `color`
  (`"#rrggbb"`), `brand` y `code`.
- `add_thread(name, "#rrggbb")`: un hilo nuevo.
- `set_thread_name(t, text)`, `set_thread_color(t, "#rrggbb")`,
  `set_thread_code(t, brand, code)`.

Mensajes y errores:

- `print(text)`: un mensaje en la ventana.
- `throw "text"`: detenerse con un error; el diseño no cambia.

## Desde la línea de comandos

`emb-cli script my-tool.rhai design.embproj --out done` ejecuta el script
en cada archivo de proyecto y guarda los archivos cambiados en la carpeta
`done`. `--set distance=1.5` define una entrada. Sin `--out`, solo imprime
los mensajes, y no se guarda nada.
