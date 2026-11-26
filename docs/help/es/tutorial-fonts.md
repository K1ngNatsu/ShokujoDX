---
id: tutorial-fonts
title: "Tutorial: elegir una fuente y un tamaño"
group: tutorials
keywords: tutorial, fuente, fuentes, tamaño, rango de tamaños, altura, texto pequeño, texto grande, calidad, puntuación de calidad, puntuación, fuente de bordado, truetype, opentype, automático, satén, relleno, tipo de puntada, paquete de fuentes, más fuentes, buscar
---
# Tutorial: elegir una fuente y un tamaño

En este tutorial busca una fuente en la ventana **Fuentes**, lee su rango
de tamaños y su puntuación de calidad, y ve qué hacer cuando el texto es
demasiado pequeño o demasiado grande para una fuente.

## Dos tipos de fuentes

- Las **fuentes de bordado** tienen letras que ya son bordado: columnas
  de satén y rellenos en un buen orden de bordado. Con el programa
  vienen 68. Cada una tiene un rango de tamaños y una puntuación de
  calidad.
- Las **fuentes TrueType** (y las fuentes OpenType) son las fuentes de su
  ordenador, y algunas fuentes del programa. El programa crea las
  puntadas de cada letra a partir de su contorno. No tienen puntuación de
  calidad: mire las puntadas antes de bordar.

Use una fuente de bordado cuando pueda.

## Buscar una fuente

1. Haga clic en [Abrir el diseño inicial](sample:lettering-start).
2. Haga clic en **Letras > Letras > Fuentes**. Se abre la ventana
   **Fuentes**.
3. En **Ordenar por**, haga clic en **Calidad**. Las fuentes con la mejor
   puntuación de calidad van primero. La puntuación (de 0 a 100) dice lo
   bien que se bordan las letras de una fuente.
4. Escriba `Pacifico` en el campo de búsqueda, y haga clic en
   **Pacifico** en la lista. La ventana muestra una vista previa de las
   letras, el rango de tamaños (de 10 a 300 mm, la altura de las
   mayúsculas) y la puntuación de calidad.

![La ventana Fuentes](fonts-window)

5. Cierre la ventana **Fuentes**.

## Crear letras dentro del rango de tamaños

1. Haga clic en **Letras > Letras > Letras**, y haga clic en el punto de
   construcción.
2. Escriba `Sam`.
3. En la lista **Fuente**, seleccione **Pacifico**. La lista tiene un
   campo de búsqueda arriba.
4. Ponga la **Altura** en 20 mm, y haga clic en **Aceptar**.

Propiedades del objeto muestra el rango de tamaños de la fuente bajo
**Altura**.

## Satén o relleno

Las letras de Pacifico son columnas de satén. Una columna de satén de más
de unos 12 mm queda suelta, así que para texto grande la fuente también
tiene cada letra como relleno. El **Tipo de puntada** en Propiedades del
objeto elige:

- **Automático**: columnas de satén hasta la altura en la que son
  demasiado largas, rellenos por encima de ella.
- **Satén**: siempre columnas de satén.
- **Relleno**: siempre rellenos.

1. Ponga el **Tipo de puntada** en **Relleno**. Las letras se bordan como
   rellenos.
2. Haga clic en **Ver > Mostrar > Aspecto de hilo** y marque **Aspecto de
   hilo** en su menú.

![El nombre con letras de relleno](fonts-fill)

3. Pulse **Esc**, haga clic en **Editar > Seleccionar > Seleccionar
   todo** y ponga el **Tipo de puntada** en **Automático** otra vez.

## Texto demasiado pequeño

1. Ponga la **Altura** en Propiedades del objeto en 6 mm. El rango de
   tamaños se muestra en el color de una advertencia, y la Información
   del diseño tiene una advertencia: las letras son más pequeñas que el
   rango de tamaños. Las columnas de satén pequeñas son demasiado
   estrechas para bordarse bien.
2. Haga clic en **Letras > Letras > Fuentes**, y haga clic en **Para
   texto pequeño**. La lista muestra las fuentes para letras desde 4 mm
   de alto.
3. Haga clic en **Atkinson Hyperlegible** y después en **Usar para las
   letras seleccionadas**. La advertencia desaparece.

![Fuentes para texto pequeño](fonts-small)

**Para texto grande** muestra las fuentes de relleno: sus letras son
rellenos, para letras desde 15 mm de alto.

## Más fuentes

- **Más fuentes…** en la ventana **Fuentes** abre la página del programa
  en internet. Los paquetes de fuentes de allí tienen más fuentes de
  bordado. Añada un paquete con **Archivo > Mis recursos** (**Importar
  un paquete**).
- Una fuente TrueType de su ordenador puede pasar a ser una fuente de
  bordado con un rango de tamaños y una puntuación de calidad: vea
  [Tutorial: una fuente de bordado propia](tutorial-own-font).

## Si no se ve bien

- **La fuente no está en la lista.** Mire el campo de búsqueda de la
  lista: conserva su texto. Bórrelo.
- **La Información del diseño muestra «por debajo del rango de
  tamaños».** Haga el texto más grande, o use una fuente para texto
  pequeño.
- **Las letras de satén grandes se ven sueltas.** Ponga el **Tipo de
  puntada** en **Automático** o **Relleno**.

## Siguiente

- [Tutorial: nombres de equipo](tutorial-team-names)
- [Letras y monogramas](lettering): todos los ajustes de las letras
