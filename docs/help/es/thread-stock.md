---
id: thread-stock
title: Existencias de hilo
group: production
keywords: existencias de hilo, inventario, conos, hilos en existencias, comprar, pedir hilo, reponer, mínimo, lista de compra, ajuste de colores, hilo más cercano, delta e
---
# Existencias de hilo

**Diseño > Existencias de hilo** conserva los hilos que tiene. Así el
programa puede ajustar los colores solo con esos hilos, cambiar los hilos
de un diseño por los hilos en existencias más cercanos y listar los hilos
que hay que comprar para un pedido.

## Añadir sus hilos

- **Añadir desde una carta**: elija una carta de hilos, escriba un nombre
  o un código y haga clic en **Añadir** junto a un hilo. Cada clic añade un
  cono.
- **Importar CSV**: un archivo con las columnas `brand,code,name,color,spools,spool_m,min_spools`
  (el color como `#rrggbb`). Las tres últimas columnas pueden faltar: un
  cono, la longitud de cono de Opciones, sin número mínimo. **Exportar
  CSV** escribe las mismas columnas, por ejemplo para pasar las
  existencias a otro ordenador.

En la lista, defina para cada hilo:

- **Conos**: los conos que tiene. Cuente como uno un cono empezado.
- **Longitud del cono**: el hilo de un cono. 0 usa la longitud de los
  precios del hilo en Opciones.
- **Mínimo**: el número mínimo de conos que quiere conservar. Por debajo,
  el hilo va a la lista de hilos que comprar.

## Solo hilos en existencias

**Solo hilos en existencias** (en la ventana Existencias de hilo y en el
panel Hilos) hace de las existencias la carta de hilos: la lista de
cartas, **Ajustar a la carta**, las herramientas de imagen e **Imagen a
diseño** usan entonces solo los hilos que tiene. Desactívelo para volver a
la carta que se mostraba antes.

El panel Hilos marca un hilo en existencias con un punto verde (●).

## Hilos de este diseño

La ventana lista cada hilo distinto del diseño: en existencias (con los
conos), o sin existencias con el hilo en existencias más cercano y la
diferencia de color (ΔE). Por debajo de unos 2,3 una persona no ve la
diferencia; por encima de unos 10 los colores son claramente distintos.

**Usar hilos en existencias** da a cada hilo usado que no está en
existencias el hilo en existencias más cercano. Deshacer devuelve todos
los cambios en un paso.

## Hilos que comprar

Para la cantidad del pedido (**Diseño > Detalles del pedido**), el
programa compara el hilo superior que necesita el pedido (de la
información del diseño) con las existencias. Un hilo está en la lista si
no está en existencias, si el pedido necesita más del que tiene, o si
después del pedido quedan menos conos que el número mínimo. La lista da
los conos que comprar. El hilo de bobina no está en la lista.

La lista está en la ventana Existencias de hilo (**Guardar la lista de
hilos que comprar** la escribe como CSV), la información del diseño
muestra el número de hilos que comprar, y la hoja de trabajo de producción
los lista.
