---
id: colors-and-threads
title: Colores e hilos
group: production
keywords: color, hilo, carta, marca, madeira, isacord, sulky, combinación de colores, fondo, color de la tela, paleta, ajustar, cambiar color, rueda, complementario, rotar, separar hilo, quitar sin usar, importar carta, gpl, csv, consumo de hilo, bobina, coste, cono
---
# Colores e hilos

## El panel Hilos

**Color > Hilos** muestra el panel Hilos.

- **Hilos del diseño** lista los hilos del diseño. Haga clic en el color de
  un hilo para cambiarlo. Haga clic en su nombre para que sea el hilo de
  los objetos nuevos.
- Para dar otro hilo a un objeto, seleccione el objeto y haga clic en el
  hilo. O haga clic derecho en el objeto y use **Hilo**. Propiedades del
  objeto también tiene el hilo.
- **Añadir hilo** añade un hilo.
- **Quitar sin usar** quita los hilos que ningún objeto usa.
- **Rotar colores**: cada hilo usado toma el color del siguiente. Repítalo
  para probar más combinaciones.
- **Separar hilo**: da a la selección su propio hilo, para que su color
  pueda cambiar solo.

## Cartas de hilos

Una carta de hilos es la lista de colores de una marca de hilo. El
programa tiene las cartas de 75 marcas y tipos de hilo (Brother, Madeira,
Isacord, Sulky, Robison-Anton, Gunold, Marathon, Floriani y más).

- Seleccione una carta en **Color > Carta de hilos**, o en el panel Hilos.
- **Buscar** encuentra un hilo por su nombre o su código.
- Haga clic en un hilo de la carta para dar su color y su nombre al hilo
  actual. **Ctrl+clic** lo añade como hilo nuevo.
- **Ajustar a la carta** da a cada hilo usado el color más cercano de la
  carta.
- **Importar carta** lee una carta de una paleta de GIMP (`.gpl`) o de un
  archivo CSV (`code,name,#rrggbb`). La carta se conserva para el próximo
  inicio.

Imagen a diseño puede tomar sus colores de la carta: vea [Imágenes e
Imagen a diseño](pictures).

## Combinaciones de colores

Una combinación de colores es un conjunto de colores para el diseño. Cree
más combinaciones para bordar el mismo diseño en otros colores, por
ejemplo para telas de otros colores.

- **Combinación de colores nueva** crea una copia de los colores actuales,
  para cambiarla.
- **Actual** (pestaña Color) o un clic en el panel Hilos muestra una
  combinación.
- **Eliminar combinación de colores** borra la combinación actual. Las
  flechas del panel Hilos cambian el orden de las combinaciones.
- El color de la tela forma parte de la combinación.

Exportar y Producir diseño escriben la combinación actual.

## El color de la tela

**Color > Color de la tela** define el **Fondo**: el color de la tela
dentro del bastidor. El lienzo, el aspecto del hilo y las imágenes
muestran la tela en ese color. **Sin fondo** lo quita. El aspecto del hilo
en 3D muestra el tejido de la tela del diseño (Diseño > Tela) bajo el
color.

## La rueda de colores

Abra **Rueda de colores** en el panel Hilos. Muestra los colores del
diseño en una rueda. Haga clic en un punto para que sea el hilo actual.
Bajo la rueda hay colores que combinan con el hilo actual (el color
opuesto, los colores vecinos y otros), como hilos de la carta. Haga clic
en uno para usarlo en el hilo actual; Ctrl+clic lo añade como hilo nuevo.

## Cuánto hilo usa un diseño

La **Información del diseño** muestra el hilo superior de cada color y el
hilo de bobina del diseño. La hoja de trabajo también los muestra. Los
números son una estimación a partir de las puntadas y del grosor de la
tela del diseño (Diseño > Tela). La tensión, el hilo y la tela los
cambian.

Para ver el coste del hilo, defina en **Opciones** la longitud y el precio
de un cono y de una bobina, y la moneda. Un precio de 0 no muestra coste.

## Consejos

- Formatos como DST no tienen colores. La máquina muestra sus propios
  colores. **Producir diseño** puede escribir archivos de colores de hilo
  junto al archivo de máquina. Vea [Archivos y salida](files-and-output).
- Mantenga juntos los objetos de un color en el orden de bordado, para que
  la máquina cambie el hilo menos veces. Vea [Organizar y el orden de
  bordado](arranging).
