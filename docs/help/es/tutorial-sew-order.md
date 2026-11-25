---
id: tutorial-sew-order
title: "Tutorial: orden de bordado, cortes y desplazamientos"
group: tutorials
keywords: tutorial, orden de bordado, secuencia, orden, cortes, corte, saltos, desplazamiento, cambios de color, menos cambios de color, agrupar por color, orden más corto, desplazamiento oculto, cortar después, panel secuencia, al inicio, al final
---
# Tutorial: orden de bordado, cortes y desplazamientos

La máquina borda los objetos en el orden del diseño. Cada cambio de color
detiene una máquina de una aguja, y cada corte corta el hilo y cuesta
tiempo. En este tutorial hace que un diseño se borde con menos cambios de
color y menos desplazamiento, y aprende de dónde vienen los cortes.

## Abrir el diseño inicial

1. Haga clic en [Abrir el diseño inicial](sample:sew-order). Tiene seis
   puntos: rojo, azul, rojo, azul, en ese orden de bordado.
2. La línea de estado muestra **Colores: 6** y **Cortes: 6**. La máquina
   cambia el hilo cinco veces.
3. Haga clic en **Ver > Mostrar > Saltos**. Las líneas entre los puntos
   muestran dónde se desplaza la máquina de un objeto al siguiente.

![Seis puntos en un mal orden de bordado, con los saltos](sew-order-before)

## Menos cambios de color

Haga clic en **Organizar > Orden de bordado > Menos cambios de color**.
Ahora los puntos rojos se bordan uno después del otro, y después los
puntos azules. La línea de estado muestra **Colores: 2**: la máquina
cambia el hilo una vez.

La línea de estado todavía muestra **Cortes: 6**. Los puntos están lejos
unos de otros, así que la máquina corta el hilo después de cada punto: un
hilo de salto largo se vería en la tela. Los objetos que se tocan, o un
desplazamiento oculto (más abajo), se bordan sin corte entre ellos.

**Menos cambios de color** mantiene un objeto encima del objeto que
cubre. **Agrupar por color** crea el mínimo de cambios de color, pero
puede poner un objeto debajo de un objeto que lo cubría.

## Un camino más corto

Haga clic en **Organizar > Orden de bordado > Orden más corto**. El
programa busca el orden y los puntos de inicio y de fin con el
desplazamiento más corto, y mantiene juntos los colores. El mensaje de la
línea de estado muestra los cortes y los mm de desplazamiento antes y
después.

![Los puntos en el orden más corto](sew-order-after)

## Más maneras de cambiar el orden

- **Organizar > Orden de bordado > Al inicio**, **Atrás uno**,
  **Adelante uno** y **Al final** mueven los objetos seleccionados en el
  orden de bordado.
- **Por orden de clic** define el orden: haga clic en los objetos en el
  orden en que se deben bordar.
- El panel **Secuencia** lista los objetos en orden de bordado, por color.
  Arrastre un objeto hacia arriba o hacia abajo. El menú **Paneles** de la
  pestaña **Ver** muestra el panel.

## Desplazamiento en lugar de un corte

Entre dos objetos del mismo color, la máquina puede cortar, o desplazarse
con una puntada recta bajo las puntadas de un objeto posterior. Un
desplazamiento oculto ahorra el tiempo de un corte, y no se ve.
Seleccione un objeto y, en Propiedades del objeto, en **Funciones de
máquina**, ponga **Cortar después** en **Desplazamiento oculto**. Úselo
solo donde un objeto posterior cubre el camino de este objeto al
siguiente.

## Comprobarlo

- **Ver > Cursor de puntada > Reproductor de puntadas** reproduce el
  bordado: fíjese en los saltos entre los objetos.
- La **Información del diseño** muestra los cortes y los cambios de
  color.

## Si no se ve bien

- **Un objeto que estaba encima ahora está debajo.** Haga clic en
  **Deshacer** y use **Menos cambios de color** en lugar de **Agrupar por
  color**.
- **Se ve un corte en la tela.** Acerque el punto de inicio del objeto
  siguiente al final del anterior, o use un desplazamiento oculto.

## Siguiente

- [Tutorial: un nombre en un arco](tutorial-lettering)
- [Organizar y el orden de bordado](arranging)
