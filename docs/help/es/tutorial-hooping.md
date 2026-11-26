---
id: tutorial-hooping
title: "Tutorial: un diseño en dos colocaciones"
group: tutorials
keywords: tutorial, grande, colocaciones, varias colocaciones, dividir, distribución automática, marcas de registro, pancarta, volver a colocar
---
# Tutorial: un diseño en dos colocaciones

En este tutorial borda en dos partes un diseño más grande que el bastidor.
Coloca la tela en el bastidor dos veces, y unas marcas de registro
muestran dónde va la segunda parte.

## Abrir el diseño inicial

1. Haga clic en [Abrir el diseño inicial](sample:hooping-banner). Es una
   fila de cinco puntos, de 380 mm de ancho.
2. En la pestaña **Diseño**, seleccione su máquina y su bastidor más
   grande. Los contornos naranjas muestran los objetos que salen del
   bastidor.

## Crear las colocaciones

1. Haga clic en **Bastidores > Bastidores > Modo de bastidores**. El
   programa crea el mínimo de colocaciones que cubren el diseño
   (**Distribución automática**), con una línea de división entre ellas.
   Cada colocación tiene un número y un color. En un bastidor de
   260 × 200 mm, la fila necesita dos colocaciones.
2. Si una línea de división atraviesa un objeto que no quiere cortar,
   arrastre la línea o sus nodos a otro lugar. Los objetos que no caben en
   una colocación se cortan en la línea de división.
3. En el panel **Bastidores > Opciones**, deje activadas las **Marcas de
   registro**: la máquina borda cruces donde se unen las dos colocaciones.

![Las dos colocaciones de la fila](hooping-layout)

## Comprobar las colocaciones

1. Haga clic en **Bastidores > Producción > Vista previa de
   colocaciones**. La ventana muestra cada colocación con sus puntadas,
   colores y advertencias. Corrija cada advertencia, por ejemplo una
   colocación cuyas puntadas salen del campo.
2. Haga clic otra vez en **Vista previa de colocaciones** para cerrarla.

![La ventana Vista previa de colocaciones](hooping-preview)

## Escribir los archivos

Haga clic en **Bastidores > Producción > Producir colocaciones** y
seleccione una carpeta. El programa escribe un archivo de máquina para
cada colocación (name_1, name_2), los archivos de Producir diseño y una
hoja de colocaciones (PDF) con el orden y los lugares.

## Bordar las colocaciones

1. Coloque la tela para la colocación 1 y borde `name_1`. La máquina borda
   las cruces de registro al final.
2. Mueva la tela en el bastidor para la colocación 2, como muestra la hoja
   de colocaciones. La colocación 2 debe cubrir las cruces de la
   colocación 1.
3. Abra `name_2` en la máquina. Mueva el diseño en la máquina hasta que sus
   primeras cruces estén sobre las cruces que bordó.
4. Borde `name_2`. Después saque las cruces.

## Si no se ve bien

- **Las dos partes no se unen.** La tela se movió cuando la volvió a
  colocar en el bastidor. Use una entretela firme más grande que las dos
  colocaciones, y alinee las cruces con cuidado.
- **Distribución automática no hace nada.** El diseño tiene el campo
  abierto como bastidor. Seleccione un bastidor en la pestaña **Diseño**.
- **Una colocación tiene una advertencia.** Abra **Vista previa de
  colocaciones** y haga clic en la colocación para verla.

## Siguiente

- [Diseños más grandes que el bastidor](hooping)
- [Máquinas y bastidores](machines-and-hoops)
- [Tutorial: salida para la producción](tutorial-output)
