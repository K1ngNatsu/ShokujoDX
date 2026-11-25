---
id: tutorial-vector-logo
title: "Tutorial: un logotipo vectorial"
group: tutorials
keywords: tutorial, vector, vectores, logotipo, svg, pdf, eps, ai, emf, wmf, importar, arte, gráficos vectoriales, importar arte, importar como vectores, soldar, formas, cortar, intersecar, convertir, a relleno, a satén, a contorno, borde, añadir borde, calcar, calcar imagen, calco automático
---
# Tutorial: un logotipo vectorial

En este tutorial crea bordado a partir de un logotipo vectorial: suelda
sus formas en una sola forma, convierte las formas en rellenos y añade un
borde. Después calca una imagen en formas vectoriales.

## Gráficos vectoriales

Un archivo vectorial tiene formas, no píxeles: SVG, PDF, AI, EPS, EMF o
WMF. Hay tres maneras de traer uno:

- **Archivo > Abrir** abre un archivo SVG como un diseño nuevo.
- **Arte > Importar > Importar arte** crea objetos con puntadas al
  momento: rellenos, satenes y contornos.
- **Arte > Importar > Importar como vectores** añade las formas como
  gráficos vectoriales sin puntadas. Limpie primero las formas, y después
  conviértalas. Este tutorial hace esto.

## Abrir el diseño inicial

Haga clic en [Abrir el diseño inicial](sample:vector-logo). Tiene un
logotipo como gráficos vectoriales, como después de **Importar como
vectores**: una nube hecha de cuatro formas que se superponen (tres
círculos y una base) y un sol. Los gráficos vectoriales se ven en sus
colores, sin puntadas.

## Soldar la nube

Las formas que se superponen se bordan dos veces donde se superponen.
Suéldelas en una sola forma.

1. Arrastre un recuadro alrededor de la nube. Las cuatro formas de la
   nube quedan seleccionadas, no el sol.
2. Haga clic en **Arte > Formas > Soldar**. La nube es una sola forma.

![La nube soldada en una sola forma](vector-welded)

Los otros comandos de **Arte > Formas** cortan formas: **Frente menos
fondo** corta la forma de delante de la forma de detrás, e **Intersecar**
conserva la parte donde las formas se superponen.

## Convertir en puntadas

1. Haga clic en **Editar > Seleccionar > Seleccionar todo**.
2. Haga clic en **Arte > Convertir > A relleno**. La nube y el sol son
   rellenos, cada uno en el color de su forma. (**A satén** crea columnas
   de satén con las formas estrechas, y **A contorno** crea puntadas
   rectas alrededor de las formas.)
3. Arrastre un recuadro alrededor de la nube para seleccionarla.
4. Haga clic en **Arte > Bordes y desplazamientos > Añadir borde**, y
   después en **Añadir contornos**. La nube tiene un borde de satén.
5. Haga clic en **Ver > Mostrar > Aspecto de hilo** y marque **Aspecto de
   hilo** en su menú.

![El logotipo en puntadas](vector-stitches)

## Calcar una imagen

Un logotipo que es una imagen (PNG, JPEG) no tiene formas. **Calcar
imagen** crea una forma vectorial de cada área de color de la imagen.

1. Haga clic en [Abrir el logotipo de la hoja](sample:logo-backdrop). La
   imagen de una hoja es la imagen de fondo.
2. Haga clic en **Arte > Calcar > Calcar imagen**. Cada área de color de
   la imagen es una forma vectorial en su color: tres formas en tres
   colores.

![La hoja calcada en formas vectoriales](vector-traced)

3. Convierta las formas como arriba. Antes de convertirlas, mire las
   formas pequeñas: borre las formas que no quiere bordar.

Para una imagen con muchos colores, **Arte > Importar > Imagen a diseño**
hace todos los pasos (vea [Tutorial: un parche a partir de una
imagen](tutorial-patch)).

## Si no se ve bien

- **El sol también está seleccionado.** El recuadro selecciona solo las
  formas que están enteras dentro de él. Dibuje el recuadro solo
  alrededor de la nube.
- **Un relleno se borda dos veces donde las formas se superponen.**
  Suelde o corte las formas antes de convertirlas.
- **Calcar imagen crea muchas formas pequeñas.** La imagen tiene ruido o
  bordes suaves. Use una imagen limpia, o borre las formas pequeñas.

## Siguiente

- [Tutorial: un archivo de un cliente](tutorial-customer-file)
- [Gráficos vectoriales y formas](artwork): todos los comandos de
  importar, de formas y de convertir
