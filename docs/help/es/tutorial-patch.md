---
id: tutorial-patch
title: "Tutorial: un parche a partir de una imagen"
group: tutorials
keywords: tutorial, parche, logotipo, imagen, png, jpeg, asistente, imagen a diseño, fondo, colores, puntadas, imagen de fondo, digitalizar, exportar, digitalización automática, foto
---
# Tutorial: un parche a partir de una imagen

En este tutorial crea un parche de dos colores a partir de la imagen de un
logotipo con **Imagen a diseño**, y escribe el archivo para su máquina. El
asistente encuentra las áreas de cada color y les da puntadas.

No necesita ningún archivo: el diseño inicial tiene la imagen de un
logotipo, un círculo azul marino con una estrella amarilla. Con su propia
imagen, use una imagen sencilla (PNG, JPEG, BMP, TIFF o GIF) con pocos
colores.

## Abrir el diseño inicial

Haga clic en [Abrir el diseño inicial](sample:patch-picture). El logotipo
se ve como imagen de fondo, atenuado bajo el diseño.

![La imagen del logotipo](patch-start)

## Imagen a diseño

1. Haga clic en **Arte > Importar > Imagen a diseño**. El diseño tiene una
   imagen de fondo, así que el asistente la usa. (Sin imagen de fondo, el
   asistente pide un archivo de imagen.)
2. Haga clic en **Siguiente ›**. El paso **Fondo** quita las partes que no
   se bordan.
3. Haga clic en **Quitar el color del borde**. El blanco alrededor del
   logotipo desaparece: es el color de los bordes de la imagen. Para
   quitar otras partes, haga clic en **Haga clic en un color** y haga clic
   en ellas en la imagen, o use los pinceles.

![El paso Fondo del asistente](patch-background)

4. Haga clic en **Siguiente ›**. En el paso **Simplificar**, ponga
   **Colores** en 2: el logotipo tiene dos colores de hilo.
5. Haga clic en **Siguiente ›**. El paso **Colores** muestra cada color.
   Desmarque un color que no quiere bordar.
6. Haga clic en **Siguiente ›**. El paso **Puntadas** da a cada color un
   relleno, un satén o un contorno. **Auto** elige según la forma de cada
   área: las puntas de la estrella son estrechas, así que pueden pasar a
   ser satenes.
7. Haga clic en **Siguiente ›**. El paso **Terminar** muestra el número de
   objetos y las comprobaciones. Una comprobación puede tener un botón que
   la corrige.
8. Haga clic en **Terminar**. El diseño está en el lienzo, y los objetos
   están seleccionados. Pulse **Esc**.

El asistente crea las imágenes de sus pasos en segundo plano. Mientras
trabaja, la vista previa muestra **Trabajando…** y la última imagen.

## Ver las puntadas

Haga clic en **Ver > Mostrar > Aspecto de hilo** y marque **Aspecto de
hilo** en su menú. La línea de estado muestra **Colores: 2**.

![El parche con el aspecto de hilo](patch-stitches)

## Escribir el archivo de máquina

1. Haga clic en **Diseño > Producción > Exportar** y después en
   **Exportar…**. El formato de su máquina es el predeterminado.
2. Guarde también el proyecto (**Guardar**), para cambiar el diseño
   después.

Para un parche real, añada un borde de satén alrededor del logotipo (vea
[Tutorial: su primer diseño](tutorial-first-design)) y bórdelo sobre
sarga con una entretela firme.

## Si no se ve bien

- **El fondo se borda.** Vuelva al paso **Fondo** (haga clic en
  **‹ Atrás**) y quite el color de fondo.
- **Hay demasiadas áreas pequeñas.** En el paso **Simplificar**, aumente
  **Quitar motas menores que**, o defina menos **Colores**.
- **Falta una línea fina.** En el paso **Puntadas**, reduzca el valor de
  **Áreas pequeñas de menos de**.

## Siguiente

- [Tutorial: un logotipo a mano](tutorial-logo-by-hand): el mismo trabajo
  con las herramientas de dibujo
- [Imágenes e Imagen a diseño](pictures): todos los ajustes del
  asistente
- [Tutorial: un nombre en un arco](tutorial-lettering)
