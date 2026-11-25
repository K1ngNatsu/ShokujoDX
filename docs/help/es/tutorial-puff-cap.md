---
id: tutorial-puff-cap
title: "Tutorial: relieve en una gorra"
group: tutorials
keywords: tutorial, relieve, puff, 3d, espuma, espuma 3d, gorra, gorras, sombrero, marco de gorras, orden para gorras, desde el centro hacia fuera, gorra estructurada, parada, multiaguja, en relieve
---
# Tutorial: relieve en una gorra

En este tutorial hace que las letras del frente de una gorra se borden
sobre espuma (relieve 3D), pone el diseño en un marco de gorras y lo
borda en el orden para gorras.

Un marco de gorras es para máquinas multiaguja con un accionador de
gorras. La gorra gira bajo la aguja, así que un diseño en una gorra se
borda desde el centro hacia fuera y de abajo arriba.

## Abrir el diseño inicial

Haga clic en [Abrir el diseño inicial](sample:cap-front). Tiene un marco
azul marino (una línea de puntada recta) y las letras rojas **CAP** (Block
Sans, 20 mm).

## La máquina, el marco de gorras y la tela

1. Haga clic en la pestaña **Diseño**. En **Máquina**, seleccione **Tajima
   multi-needle** (o su máquina).
2. En **Bastidor**, seleccione **Wide cap frame 360 x 75 (270 degree
   class)**.
3. Haga clic en **Diseño > Tela y estilos > Tela**. En la lista **Tela**,
   seleccione **Gorra estructurada** y haga clic en **Aplicar**. Una gorra
   es rígida y curva: el ajuste de tela añade base y compensación de tiro.

## Letras en relieve

1. Arrastre un cuadro alrededor de las letras para seleccionarlas (no el
   marco).
2. Haga clic en **Especial > Relieve y capa base > Relieve**.

Ahora las letras se bordan sobre espuma:

- El satén es denso, y la base no tiene filas a través de las columnas:
  aplastarían la espuma.
- Hay puntadas sobre cada extremo de una columna, para que la aguja corte
  la espuma y el extremo quede cerrado.
- La máquina se detiene antes del primer objeto en relieve, para que
  ponga la espuma. La hoja de trabajo lo dice. Arranque la espuma después
  de los objetos en relieve.

Para una columna de satén, la casilla **Relieve (espuma 3D)** de
Propiedades del objeto hace lo mismo. Use columnas de 3 mm de ancho o más,
y espuma de un color parecido al del hilo.

La Información del diseño muestra ahora "El hilo se acumula" en la punta
de la **A**: las dos patas de la letra están una sobre otra allí, y el
satén en relieve es denso. Una letra con puntas agudas tiene más hilo en
sus puntas que una letra con extremos romos. Para mucho texto en relieve,
use una fuente de trazo grueso con extremos romos, y mire las puntas en
**Ver > Mostrar > Mapa de densidad**.

## El orden para gorras

1. Haga clic en **Organizar > Orden de bordado > Orden para gorras**. Cada
   color se borda desde el centro hacia fuera y de abajo arriba, y las
   letras se bordan desde la letra central hacia fuera. Los colores y lo
   que está encima no cambian.
2. Haga clic en **Ver > Mostrar > Aspecto de hilo** y marque **Aspecto de
   hilo** en su menú.

![Letras en relieve en el frente de una gorra](puff-cap-stitches)

La Información del diseño comprueba el orden: en un marco de gorras, avisa
cuando el orden de bordado no es desde el centro hacia fuera.

## Si no se ve bien

- **El marco de gorras no está en la lista.** La máquina no tiene
  accionador de gorras. Seleccione una máquina multiaguja.
- **Relieve está atenuado.** Seleccione antes columnas de satén o letras.
- **La espuma se ve en los extremos de las columnas.** Use columnas más
  anchas, y un hilo más grueso o más densidad.

## Siguiente

- [Tutorial: ojales](tutorial-buttonholes)
- [Decoración especial](specialty): relieve, capa base y encaje
- [Organizar y el orden de bordado](arranging): el orden para gorras
