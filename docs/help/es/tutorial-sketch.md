---
id: tutorial-sketch
title: "Tutorial: una insignia en el modo boceto"
group: tutorials
keywords: tutorial, boceto, paramétrico, cad, cota, diámetro, tamaño exacto, círculo, polígono, hexágono, regiones, insignia, restricción, cambiar el tamaño, vinculado
---
# Tutorial: una insignia en el modo boceto

En este tutorial dibuja una insignia con tamaños exactos en el modo
boceto: dos círculos y un hexágono. Hace bordado con las regiones del
boceto y después cambia un tamaño: las puntadas lo siguen.

Un boceto es un dibujo con cotas y restricciones, como en un programa
CAD. No borda nada por sí mismo. Vea [Bocetos](sketch).

## Abrir el diseño inicial

Haga clic en [Abrir el diseño inicial](sample:sketch-start). Está vacío y
tiene tres hilos: Navy, White y Gold.

## Empezar un boceto

Haga clic en **Digitalizar > Boceto > Boceto paramétrico**. Se abre la
pestaña **Boceto**. El origen del boceto es un anillo con una cruz en el
centro del bastidor: los puntos se ajustan a él.

## Dibujar los círculos

1. Haga clic en **Boceto > Crear > Círculo**.
2. Haga clic en el origen (el centro) y después en un punto 30 mm a su
   derecha.
3. Haga clic otra vez en el origen y después en un punto 25 mm a su
   derecha. Los dos círculos tienen el mismo centro: el origen.

## Dibujar un hexágono

1. Haga clic en **Boceto > Crear > Polígono** (haga clic en la flecha
   junto a **Rectángulo** y después en **Polígono**). Un polígono nuevo
   tiene seis lados.
2. Haga clic en el origen y después en un punto 12 mm a su derecha.

## Dar a los círculos su tamaño

1. Haga clic en **Boceto > Restringir > Cota**.
2. Haga clic en el círculo exterior y después en un lugar para la cota,
   arriba a la derecha. Una casilla muestra el diámetro actual.
3. Escriba `60` y pulse Intro. El círculo mide 60 mm de ancho.
4. Haga clic en el círculo interior y después en un lugar abajo a la
   derecha. Escriba `52` y pulse Intro.
5. Pulse Esc para volver a **Seleccionar**.

![El boceto con sus dos cotas](sketch-dimensions)

También puede escribir otras unidades (`2 in`) o un cálculo (`60 - 8`).

## Hacer bordado con las regiones

Las curvas forman tres regiones cerradas: el anillo entre los círculos,
el disco entre el círculo interior y el hexágono, y el hexágono.

1. Haga clic en **Boceto > Boceto > Regiones**.
2. Haga clic en el color **Navy** de la barra **Hilos**. Haga clic dentro
   del disco (entre el hexágono y el círculo interior) y después en
   **Relleno**, en **Crear bordado** del panel Propiedades.
3. Haga clic en **White**. Haga clic dentro del hexágono y después en
   **Relleno**.
4. Haga clic en **Gold**. Haga clic dentro del anillo y después en
   **Satén**.
5. Haga clic en **Boceto > Salir > Terminar boceto**.

La insignia tiene ahora un relleno azul marino, un relleno blanco y un
anillo de satén dorado. Se bordan en el orden en que los hizo.

## Cambiar un tamaño

Los objetos siguen vinculados al boceto.

1. Haga clic en el anillo dorado. En el panel Propiedades, haga clic en
   **Editar boceto**.
2. Haga clic con el botón derecho en la cota del círculo exterior (60) y
   haga clic en **Cambiar el valor**.
3. Escriba `66` y pulse Intro. El anillo es más ancho, y sus puntadas lo
   siguen.
4. Haga clic en **Boceto > Salir > Terminar boceto**.

![La insignia después del cambio, con el aspecto de hilo 3D](sketch-stitches)

Deshacer quita el cambio del boceto y de sus objetos en un paso.

## Si no se ve bien

- **Los círculos no tienen el mismo centro.** Haga clic justo en el
  origen: el puntero muestra una marca cuando se ajusta.
- **Una región no se sombrea.** La región no está cerrada: las curvas
  deben tocarse o cruzarse.
- **La cota nueva está entre paréntesis.** El boceto ya define ese
  tamaño: la cota es conducida y no cambia nada.
- **La insignia no cambia.** Cambie el boceto (Editar boceto), no los
  objetos: un objeto hecho no se escala solo.

## Siguiente

- [Tutorial: un script que hace el trabajo](tutorial-script)
- [Bocetos](sketch): restricciones, parámetros y más herramientas
