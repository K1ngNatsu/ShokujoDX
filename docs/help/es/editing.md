---
id: editing
title: Cambiar objetos
group: design
keywords: editar, copiar, cortar, pegar, pegado especial, pegar después, duplicar, eliminar, clipart, seleccionar, por color, por tipo, dividir, unir, separar, cuchilla, dividir en la aguja, agujero, quitar agujeros, rellenar agujeros, radio de esquina, agujero desde el contorno, marco, simplificar, nodos, continuar línea, redwork, ramificación, cerrar forma, invertir, intercambiar lados, funciones de máquina, parada, corte, remate inicial, remate final, puntada de remate
---
# Cambiar objetos

La pestaña **Editar** cambia los objetos que tiene. La mayoría de los
comandos actúan sobre la selección; un comando que no puede ejecutarse
ahora aparece atenuado.

## Portapapeles

- **Copiar** (Ctrl+C), **Cortar** (Ctrl+X) y **Pegar** (Ctrl+V).
  Opciones > Edición define dónde pone Pegar las copias.
- **Duplicar** (Ctrl+D) crea una copia un poco desplazada. Opciones define
  la distancia.
- **Pegar después de la selección** pone las copias justo después de los
  objetos seleccionados en el orden de bordado, no al final.
- **Pegado especial** pregunta dónde van las copias: **En el mismo
  lugar**, **Desplazado una distancia**, **Centrado en el bastidor**,
  **Centrado en el cursor de puntada** o **Primera puntada en el cursor de
  puntada**.
- **Eliminar** (Supr) borra la selección.

## Clipart

Los elementos de clipart son partes de diseños que usa de nuevo, por
ejemplo un marco o un logotipo.

- **Clipart** muestra la pestaña Clipart. Haga doble clic en un elemento,
  o arrástrelo sobre el diseño.
- **Guardar como clipart** conserva los objetos seleccionados como un
  elemento, con un nombre y palabras clave para la búsqueda.

Vea [La biblioteca de diseños y los cliparts](library).

## Seleccionar

- **Seleccionar todo** (Ctrl+A), **Siguiente** (Tab) y **Anterior**
  (Mayús+Tab).
- **En el cursor de puntada** selecciona el objeto en el que está el
  cursor de puntada.
- **Por color** selecciona todos los objetos de un hilo. **Por tipo**
  selecciona todas las líneas rectas, todas las columnas de satén o todos
  los rellenos.

## Dividir y unir

- **Cuchilla**: corte los objetos seleccionados a lo largo de una línea.
  Haga clic en dos puntos.
- **Separar**: un objeto para cada parte separada de un objeto.
- **Unir**: un objeto a partir de los objetos seleccionados de un tipo y
  un color: líneas y columnas cuyos extremos se tocan, rellenos que se
  tocan. Dos columnas que se encuentran en una esquina (una termina y la
  otra empieza sobre ella) se vuelven una columna que gira en una esquina
  en inglete. Vuelve a juntar lo que Separar o la cuchilla separaron.
- **Dividir en la aguja**: corte en dos el objeto bajo el cursor de
  puntada, donde está el cursor.
- **Cerrar huecos…**: une los extremos de las líneas seleccionadas donde
  están cerca, para que las líneas que dibujó una a una (Mano alzada,
  Línea) sean una línea o una forma cerrada. Defina el **Hueco**: los
  extremos más cercanos se unen, los más cercanos primero, en el punto
  medio entre ellos. La ventana empieza con un hueco que une cada extremo
  que tiene otro extremo a menos de 10 mm. Con **Cortar sobrantes**, dos líneas que se cruzan
  cerca de sus extremos se cortan donde se cruzan. Una línea se cierra
  cuando sus dos extremos están cerca. Las líneas solo se unen con líneas
  del mismo color. El lienzo muestra el resultado mientras cambia el
  hueco; **Aceptar** lo conserva. Para bordar una forma cerrada como
  relleno, use **Convertir**.
- **Recortar tramo**: haga clic en una parte de una línea entre dos
  cruces, o en un extremo tras el último cruce: esa parte se quita. Los
  cruces son con otras líneas, los contornos de otros objetos y la propia
  línea. Úselo para los sobrantes de líneas dibujadas a mano.

## Forma

- **Ángulo** y **Línea de dirección** definen la dirección de las
  puntadas. Vea [Dibujar objetos](digitizing).
- **Añadir agujero** dibuja un agujero en el relleno seleccionado.
  **Quitar agujeros** llena los agujeros del relleno (los agujeros se
  bordan encima). **Rellenar agujeros** crea un objeto de relleno nuevo en
  cada agujero.
- **Radio de esquina**: redondea las esquinas de un relleno o de un
  aplique con lados rectos. Defina un radio para **Todas las esquinas**, o
  un radio para cada esquina (numeradas desde el primer punto del
  contorno).
- **Agujero desde el contorno**: añade un agujero con la forma del
  contorno, a una distancia dentro de él. Úselo para un marco o un borde.
- **Simplificación rápida**: menos nodos, con los ajustes estándar. La
  forma no se aleja más de 0,2 mm.
- **Simplificar…**: menos nodos con sus ajustes: la **Tolerancia** (cuánto
  puede alejarse el contorno nuevo del antiguo), **Esquinas** (un giro más
  agudo que este queda como esquina), y **Curvas** o **Líneas rectas**
  entre las esquinas. La ventana muestra el número de nodos antes y
  después. Simplificar nunca añade nodos.
- **Limpiar…**: quita fallos pequeños de las líneas, rellenos y objetos
  vectoriales seleccionados, por ejemplo de un archivo SVG o de un
  calco. Defina el **Tamaño de detalle**: **Quitar bucles y picos
  pequeños** (donde una línea se cruza a sí misma, o va y vuelve),
  **Suavizar zigzags** (las esquinas siguen nítidas) y **Quitar partes
  diminutas**. Las partes del contorno de un relleno que se solapan se
  vuelven un área. El lienzo muestra el resultado; **Aceptar** lo
  conserva. Las columnas de satén, los apliques y las letras no cambian.
- **Envolvente…**: curva las líneas, rellenos, columnas de satén, apliques
  y objetos vectoriales seleccionados en una forma (arco, abombado,
  banderín, perspectiva y más), con una **Cantidad**. El lienzo muestra el
  resultado; **Aceptar** lo conserva. Las letras tienen su propia
  envolvente (Arte de letras).
- **Copiar propiedades**: seleccione los objetos, haga clic en Copiar
  propiedades y después en el objeto del que copia. Los objetos
  seleccionados reciben su color, y sus ajustes de puntada donde son del
  mismo tipo (relleno a relleno, línea a línea, etc.). Un paso de deshacer.
- **Repetir**: repite el último comando de la cinta (también **Espacio** y
  el menú del clic derecho).

## Líneas y columnas

- **Continuar línea**: añada puntos al final de la línea seleccionada, o a
  su inicio (haga clic primero cerca del inicio).
- **Redwork**: una las líneas en un objeto. Cada línea se borda dos veces,
  y el objeto termina donde empieza. Bueno para contornos sin cortes.
- **Ramificación**: una las líneas en un objeto, que se borda con el
  mínimo de líneas bordadas dos veces.
- **Cerrar forma** cierra líneas abiertas, vectores y filas de piedras con
  una línea recta. **Cerrar con curva** los cierra con una curva que
  continúa los extremos.
- **Invertir dirección**: las líneas, columnas, vectores y filas de
  piedras van en sentido contrario. El inicio y el final cambian de lugar.
- **Intercambiar lados**: los dos lados de las columnas de satén
  seleccionadas cambian de lugar, con los ajustes de cada lado.

## Tipos de puntada

El panel **Tipos de puntada** define el **Tipo de contorno** de las
líneas, el **Tipo de relleno** de los rellenos y el **Tipo de columna** de
las columnas, y tiene los rellenos especiales: motivos, líneas guía,
líneas de división, tartanes, degradados y ondas. Vea [Tipos de
puntada](stitch-types).

## Funciones de máquina

Las funciones de máquina dicen a la máquina qué hacer después de un
objeto. Seleccione los objetos y haga clic en:

- **Parar después**: la máquina se detiene después de los objetos
  seleccionados, por ejemplo para poner la tela de aplique.
- **Cortar siempre**: cortar siempre después de los objetos
  seleccionados.
- **No cortar nunca**: no cortar después de los objetos seleccionados. Un
  hueco largo pasa a ser un salto.
- **Remate inicial**: puntadas de remate donde los objetos seleccionados
  empiezan después de un corte.
- **Remate final**: puntadas de remate donde los objetos seleccionados
  terminan antes de un corte.

Propiedades del objeto > **Funciones de máquina** muestra los mismos
ajustes para un objeto, con **Cortar después: Automático** (cortar cuando
el objeto siguiente está lejos, si no una puntada de desplazamiento),
**Desplazamiento oculto** (como Automático, pero un objeto lejano del
mismo color recibe puntadas de desplazamiento donde los objetos que se
bordan después cubren todo el camino) y **Bordar despacio** (solo los
archivos U01 lo conservan). La lista de puntadas muestra cada función;
vea [Comprobar un diseño](checking).
