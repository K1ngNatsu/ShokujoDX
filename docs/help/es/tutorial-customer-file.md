---
id: tutorial-customer-file
title: "Tutorial: un archivo de un cliente"
group: tutorials
keywords: tutorial, cliente, archivo de máquina, archivo de puntadas, dst, pes, exp, jef, vp3, abrir, reconocer, reconocer objetos, puntadas, objetos, tamaño, cambiar tamaño, escala, más grande, más pequeño, colores de hilo, contornos, escribir, exportar
---
# Tutorial: un archivo de un cliente

En este tutorial abre un archivo de máquina de un cliente, crea objetos a
partir de sus puntadas con **Reconocer objetos**, cambia su tamaño y lo
escribe de nuevo.

Un archivo de máquina (DST, PES, EXP, JEF, VP3 y otros) solo tiene
puntadas: ni formas ni ajustes. Si lo hace más grande como puntadas, las
puntadas se alargan y la densidad baja. Los objetos tienen sus formas y
sus ajustes, así que el programa crea puntadas nuevas en el tamaño nuevo.

## Abrir el archivo

Haga clic en [Abrir el diseño inicial](sample:customer-dst). Es el
archivo **LEAF.DST**, como lo abre **Archivo > Abrir**: un objeto de
puntadas para cada color. Un archivo DST no tiene colores de hilo: el
programa da los colores de su lista estándar. Defina los colores de hilo
que quiere el cliente (la barra **Hilos**). El diseño inicial muestra las
puntadas sobre una camisa blanca (**Color > Color de la tela**), porque
el primer color del archivo es el negro.

![Las puntadas del archivo](customer-file)

## Reconocer objetos

1. Haga clic en **Editar > Seleccionar > Seleccionar todo**.
2. Haga clic en **Arte > Convertir > Reconocer objetos**. La ventana
   muestra el archivo y los objetos que el programa encontró en sus
   puntadas: el relleno de la hoja (con su agujero), el satén del tallo,
   la puntada recta de la nervadura y las partes de la base del relleno.

![La ventana Reconocer objetos](customer-recognize)

3. Mire la lista. Una parte que el programa no reconoce se queda como
   puntadas. Desmarque una parte para conservar sus puntadas tal como
   están.
4. Haga clic en **Aceptar**. Los objetos sustituyen a los objetos de
   puntadas, en un solo paso de deshacer. La línea de estado dice lo que
   creó el programa: 5 objetos, 4 con un tipo de puntada y 1 que se queda
   como puntadas.

Si el cliente también envió el arte (SVG, PDF, AI, EPS, EMF o WMF), haga
clic en **Contornos…** antes de **Aceptar**: los objetos reciben las
formas del arte, que son más exactas que las formas a partir de las
puntadas.

## Cambiar el tamaño

1. Haga clic en **Editar > Seleccionar > Seleccionar todo**.
2. En Propiedades del objeto, bajo **Posición y tamaño**, deje **Mantener
   proporciones** marcado, y ponga **W** (el ancho) en 80 mm. El programa
   crea las puntadas de nuevo en el tamaño nuevo, con la misma densidad.

![La hoja, más grande](customer-larger)

## Escribir el archivo de nuevo

Haga clic en **Diseño > Producción > Exportar** y después en
**Exportar…**, y seleccione el formato de la máquina del cliente. Guarde
también el proyecto (**Guardar**): conserva los objetos, así que puede
cambiar el diseño otra vez más tarde.

## Si no se ve bien

- **Reconocer objetos está atenuado.** Seleccione primero los objetos de
  puntadas.
- **Una parte se queda como puntadas.** El programa no encontró una forma
  en ella (por ejemplo un relleno con un patrón). Consérvela como
  puntadas, y no haga el diseño mucho más grande ni mucho más pequeño.
- **Los colores no son correctos.** Un archivo DST no tiene colores. Haga
  clic en un hilo de la barra **Hilos** y defina su color.

## Siguiente

- [Tutorial: un aplique](tutorial-applique)
- [Archivos y salida](files-and-output): los formatos de archivo y
  Reconocer objetos
