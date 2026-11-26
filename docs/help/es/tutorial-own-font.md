---
id: tutorial-own-font
title: "Tutorial: una fuente de bordado propia"
group: tutorials
keywords: tutorial, fuente, fuente propia, convertir, conversión, truetype, opentype, fuente de bordado, hoja de prueba, hoja de prueba de fuente, corrección, corrección de letras, corregir una letra, alfabetos, pocos cortes, licencia, rango de tamaños, puntuación de calidad
---
# Tutorial: una fuente de bordado propia

En este tutorial crea una fuente de bordado a partir de una fuente
TrueType, crea una hoja de prueba de sus letras para bordarla y corrige
una letra.

Una fuente de bordado se borda mejor que una fuente TrueType: sus letras
se crean una sola vez, con las columnas, el orden de bordado y los
extremos revisados, y tiene un rango de tamaños y una puntuación de
calidad.

Compruebe que la licencia de la fuente le permite cambiarla. Una fuente
OFL (SIL Open Font License) con un nombre de fuente reservado (Reserved
Font Name) necesita un nombre nuevo.

## Convertir la fuente

1. Haga clic en **Letras > Letras > Fuentes**. Se abre la ventana
   **Fuentes**.
2. Haga clic en **TrueType**: la lista muestra solo las fuentes TrueType.
3. Escriba `Graduate` en el campo de búsqueda, y haga clic en
   **Graduate** en la lista. Graduate es una fuente TrueType del
   programa; use la fuente de su logotipo de la misma manera.
4. Haga clic en **Convertir en fuente de bordado…**.

![Los ajustes de la conversión](own-font-convert)

5. Escriba un **Nombre** para la fuente de bordado, por ejemplo
   `Graduate Embroidery`.
6. En **Tipo de puntada**, deje **Automático**: columnas de satén, y
   rellenos para los trazos demasiado anchos para el satén. **Relleno**
   crea una fuente de relleno para texto grande.
7. Deje **Pocos cortes**: el programa busca el orden de bordado de cada
   letra con el menor número de cortes. La conversión tarda más: unos
   segundos en un ordenador rápido, un minuto o más en uno lento.
8. Haga clic en **Convertir**. Una barra de progreso muestra los pasos.
   Después la ventana dice «La fuente de bordado Graduate Embroidery está
   lista», con su puntuación de calidad.

La fuente nueva está en las listas de fuentes, en la carpeta de sus
fuentes. Una versión nueva del programa la conserva.

## Crear una hoja de prueba

Una hoja de prueba tiene todas las letras de la fuente en su tamaño más
pequeño, en el medio y en el más grande. Bórdela sobre la tela de su
trabajo antes de bordar mucho texto con la fuente.

1. En la ventana **Fuentes**, con la fuente seleccionada, haga clic en
   **Hoja de prueba…**. Se abre la ventana **Hoja de prueba de fuente**.
2. En **Caracteres**, seleccione **Mayúsculas**. (**Letras, cifras y
   signos** crea una hoja más grande.)
3. Haga clic en **Crear hoja de prueba**. La hoja se abre como un diseño
   nuevo.

![Una hoja de prueba de mayúsculas](own-font-sheet)

Mire las letras con el aspecto de hilo, y borde la hoja. Busque huecos
entre los trazos, columnas demasiado estrechas en el tamaño más pequeño y
satén suelto en el tamaño más grande.

## Corregir una letra

Si una letra no se borda bien, corríjala. La corrección se conserva
cuando vuelve a crear la fuente.

1. Haga clic en **Nuevo**: la letra va a un diseño nuevo.
2. Haga clic en **Letras > Letras > Alfabetos**. Se abre la ventana
   **Alfabetos**. **Correcciones de letras** está en su parte inferior.
3. Seleccione la **Fuente**, y escriba la letra en **Carácter**, por
   ejemplo `A`.
4. Haga clic en **Colocar letra**. Los objetos de la letra van al
   diseño, entre guías, en su orden de bordado.

![Una letra colocada para una corrección](own-font-letter)

5. Cambie los objetos: su orden (el panel Secuencia), la dirección de una
   columna (**Editar > Líneas y columnas > Invertir dirección**), su forma
   (**Digitalizar > Seleccionar > Remodelar**) o sus ajustes (Propiedades
   del objeto).
6. Seleccione todos los objetos de la letra, y haga clic en **Guardar
   corrección**.

**Quitar corrección** devuelve la letra de la fuente. Las correcciones
son un archivo en la carpeta de sus fuentes (**Mostrar archivo**).

## Si no se ve bien

- **Convertir está atenuado.** Escriba primero un nombre.
- **La puntuación de calidad es baja.** Los trazos demasiado finos o
  demasiado anchos para columnas de satén pierden puntos. Pruebe
  **Relleno** para una fuente gruesa, o use la fuente solo dentro de su
  rango de tamaños.
- **La hoja de prueba es más grande que el bastidor.** Seleccione menos
  tamaños o caracteres, o defina un **Ancho de fila** menor.

## Siguiente

- [Tutorial: texto japonés](tutorial-japanese)
- [Letras y monogramas](lettering): fuentes, alfabetos y correcciones de
  letras
