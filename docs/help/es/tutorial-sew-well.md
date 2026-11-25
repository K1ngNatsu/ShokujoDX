---
id: tutorial-sew-well
title: "Tutorial: hacer que un diseño borde bien"
group: tutorials
keywords: tutorial, bordar bien, calidad, tela, ajuste predefinido, punto, camiseta, densidad, espaciado, base, compensación de tiro, tiro, empuje, fruncidos, huecos, mapa de densidad, advertencias, información del diseño, comprobar
---
# Tutorial: hacer que un diseño borde bien

En este tutorial prepara el logotipo de la hoja para una camiseta. Las
puntadas son las mismas que en el tutorial del logotipo, pero los ajustes
todavía no son buenos: el relleno es demasiado denso. Aprende los ajustes
que hacen que un diseño borde bien en su tela: el ajuste predefinido de
la tela, la densidad, la base y la compensación de tiro.

## Abrir el diseño inicial

Haga clic en [Abrir el diseño inicial](sample:sew-well). La línea de
estado muestra una advertencia. Haga clic en la advertencia: la
**Información del diseño** la muestra. Las filas de la hoja están a
0,20 mm unas de otras. Un relleno así es rígido, y la aguja se puede
romper.

## Indicar la tela al programa

1. Haga clic en **Diseño > Tela y estilos > Tela**.
2. En la lista **Tela**, seleccione **Punto de camiseta**. La ventana
   muestra lo que cambia la tela y la entretela que debe usar.
3. Deje activado **Cambiar los objetos de satén y relleno del diseño** y
   haga clic en **Aplicar**.

![La ventana Tela con el punto de camiseta](sew-well-fabric)

Un tejido de punto se estira, así que el programa hace las filas un poco
más abiertas, pone una base fuerte y añade compensación de tiro. Las
filas de la hoja siguen demasiado juntas (0,22 mm), y ahora la línea de
estado muestra dos advertencias: la densidad y las puntadas cortas que
crean las filas densas.

## Definir la densidad

1. Haga clic en **Digitalizar > Seleccionar > Seleccionar** y después haga
   clic en la hoja (no en la nervadura).
2. En Propiedades del objeto, ponga **Densidad** en 0,4. Es la distancia
   entre las filas en mm: un valor mayor crea menos puntadas.
3. Las dos advertencias desaparecen.

Valores habituales: 0,4 mm para rellenos, de 0,35 a 0,45 mm para
satenes. Una tela fina o un hilo grueso necesitan filas más abiertas.

## Definir la compensación de tiro

Las puntadas tiran de la tela hacia dentro, así que una columna de satén
se borda más estrecha que su forma. La **Compensación de tiro** la hace
más ancha en esta cantidad.

1. Haga clic en el tallo.
2. En Propiedades del objeto, ponga **Compensación de tiro** en 0,3.
3. Pulse **Esc**.

Un tejido de punto necesita más (de 0,3 a 0,4 mm) que una tela tejida
(0,2 mm). Mire también **Base** en Propiedades del objeto: la base se
borda bajo las puntadas de encima y sujeta la tela. **Fuerte** va bien
para un tejido de punto; **Ligero** va bien para texto pequeño.

## Ver el mapa de densidad

1. Haga clic en **Ver > Mostrar > Mapa de densidad**. Los colores
   muestran cuántas capas de hilo hay en cada lugar: azul es una capa,
   rojo son demasiadas.
2. Los lugares donde la nervadura y el tallo pasan sobre la hoja tienen
   más capas. Es normal en una parte pequeña.
3. Haga clic en **Ver > Mostrar > Mapa de densidad** otra vez para
   desactivarlo.

![El mapa de densidad de la hoja](sew-well-density)

## Comprobarlo

- La línea de estado no muestra ninguna advertencia.
- La **Información del diseño** muestra los cortes, la densidad y las
  advertencias del diseño.
- Borde una prueba en la misma tela, con la misma entretela, antes de
  bordar la prenda real.

## Si no se ve bien

- **Ondas alrededor del diseño.** Use una entretela más firme y coloque
  la tela más tensa en el bastidor. Un valor de densidad mayor también
  ayuda. Vea [Preguntas y problemas](troubleshooting).
- **Huecos entre un satén y un relleno.** Añada compensación de tiro, o
  haga que los dos objetos se superpongan 1 mm.
- **La tela se ve a través de un relleno.** Las filas están demasiado
  separadas: defina un valor de densidad menor, o una base más fuerte.

## Siguiente

- [Tutorial: columnas que giran](tutorial-columns)
- [Telas, estilos y plantillas](fabrics-and-styles)
- [Comprobar un diseño](checking)
