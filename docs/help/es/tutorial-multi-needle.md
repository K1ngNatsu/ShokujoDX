---
id: tutorial-multi-needle
title: "Tutorial: un pedido para una máquina multiaguja"
group: tutorials
keywords: tutorial, multiaguja, máquina multiaguja, agujas, secuencia de agujas, existencias de hilo, conos, hilos que comprar, pedido, detalles del pedido, cliente, número de pedido, cantidad, tiempo de producción, cabezales, hoja de trabajo, operario, producción
---
# Tutorial: un pedido para una máquina multiaguja

En este tutorial prepara un pedido de 120 polos con el escudo de un club
para una máquina multiaguja: las agujas, los hilos que tiene, los detalles
del pedido, el tiempo de producción y la hoja de trabajo para el operario.

## Abrir el diseño inicial

Haga clic en [Abrir el escudo del club](sample:club-crest). Tiene cuatro
colores: un disco azul marino, una estrella blanca, un galón rojo y un
anillo de satén dorado.

## La máquina y sus agujas

1. En la pestaña **Diseño**, seleccione **Tajima multi-needle** en la
   lista **Máquina**. Si el bastidor del diseño no es para esta máquina,
   el programa toma el bastidor más pequeño de la máquina en el que cabe
   el diseño. La línea de estado dice cuál.
2. Haga clic en **Diseño > Máquina y bastidor > Agujas**. La ventana
   **Agujas** muestra la aguja de cada color en el orden de bordado.
3. Ponga **Agujas de la máquina** en el número de agujas de su máquina,
   por ejemplo 6.

![La ventana Agujas](multi-needle-needles)

**Auto** pone un hilo nuevo en una aguja libre y deja un hilo en la aguja
que lo tiene. Para poner un hilo en la aguja que usted quiera, seleccione
la aguja en la lista de su color. La **Secuencia de agujas** es el orden
de las agujas: escríbala en la máquina para un archivo DST. Los archivos
TBF y U01 tienen las agujas.

Cuando el diseño tiene más colores que agujas la máquina, la máquina se
para en cada cambio de hilo (\*), y el operario pone el hilo siguiente en
la aguja.

4. Cierre la ventana **Agujas**.

Después de bordar el diseño, haga clic en **Dejar estos hilos en la
máquina** en la ventana **Agujas**. El diseño siguiente usa entonces los
hilos que están en las agujas donde puede.

## Sus hilos

1. Haga clic en **Diseño > Pedidos > Existencias de hilo**.
2. En **Añadir desde una carta**, seleccione **Madeira Polyneon** en la
   lista **Carta de hilos**.
3. Escriba `1967` en el campo **nombre o código** y haga clic en
   **Añadir** junto al hilo (Light Navy). Cada clic añade un cono.
4. Añada `1804`, `1637` y `1624` de la misma manera (Blanc de Blanc, Warm
   Red y Gold).
5. En **Hilos de este diseño**, cada hilo del diseño muestra el hilo más
   cercano en existencias. Haga clic en **Usar hilos en existencias**. El
   diseño tiene ahora sus hilos, con sus códigos.
6. Cierre la ventana **Existencias de hilo**.

## El pedido

1. Haga clic en **Diseño > Pedidos > Detalles del pedido**.
2. Escriba el **Número de pedido** `A-1042`, el **Cliente** `Riverside
   Football Club` y la **Prenda** `Polo shirt`.
3. Ponga la **Cantidad** en 120. O haga clic en **Añadir las tallas
   habituales** y escriba las piezas de cada talla.
4. Cierre la ventana **Detalles del pedido**.

El pedido se guarda con el diseño. La hoja de trabajo, la hoja de
aprobación y el presupuesto lo muestran.

## El tiempo de producción

1. Haga clic en **Diseño > Máquina y bastidor > Ajustes de la máquina**.
2. Ponga **Cabezales** en 4: la máquina borda cuatro polos a la vez.
   Ponga el tiempo para enmarcar una pieza en **Enmarcar una pieza**.
3. Cierre la ventana **Ajustes de la máquina**.

La información del diseño muestra el tiempo de producción del pedido: el
tiempo para bordar 120 polos en 4 cabezales, con el tiempo para
enmarcarlos.

## Los hilos que comprar

Haga clic otra vez en **Diseño > Pedidos > Existencias de hilo**. **Hilos
que comprar para 120 piezas** compara el hilo que necesita el pedido con
sus conos. Cada hilo de la lista muestra los conos que debe comprar.

![Los hilos que comprar](multi-needle-stock)

## La hoja de trabajo

Haga clic en **Diseño > Producción > Vista previa de impresión**. La hoja
de trabajo para el operario tiene el pedido, la aguja de cada color, los
hilos que van en las agujas al principio y los hilos que comprar.

![La hoja de trabajo con las agujas](multi-needle-worksheet)

## Si no se ve bien

- **La ventana Agujas pide una máquina.** Seleccione antes la máquina del
  diseño en la pestaña **Diseño**.
- **Usar hilos en existencias está atenuado.** Añada antes hilos a las
  existencias.
- **Un hilo cambió a un color demasiado distinto.** Deshaga, añada a las
  existencias un hilo de un color más cercano y haga clic otra vez en
  **Usar hilos en existencias**. Por debajo de ΔE 2,3 la diferencia no se
  ve.
- **El tiempo de producción no está en la información del diseño.** Ponga
  una **Cantidad** de más de una pieza en **Detalles del pedido**.

## Siguiente

- [Tutorial: un presupuesto con una imagen del producto](tutorial-quote)
- [Máquinas y bastidores](machines-and-hoops): las agujas de una máquina
  multiaguja
- [Existencias de hilo](thread-stock)
- [Pedidos, presupuestos y tiempos de producción](quotes)
