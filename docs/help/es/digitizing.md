---
id: digitizing
title: Dibujar objetos
group: design
keywords: dibujar, digitalizar, línea, manual, a mano alzada, forma, relleno, relleno complejo, forma libre, columna, satén, columna por pares, columna c, rectángulo, rectángulo redondeado, elipse, estrella, anillo, giratorio, curva, esquina, clic derecho, barra de propiedades, remodelar, nodo, tirador, punto de inicio, punto final, ángulo, longitud escrita, mayúsculas, 15 grados, ajustar, punto medio, centro, borde, alinear, arrastrar tirador, línea de dirección, dirección de puntada, agujero, cuchilla, bordes, ojales, mezclas
---
# Dibujar objetos

Las herramientas de la pestaña **Digitalizar** dibujan objetos nuevos.
Cada herramienta sigue activa después de un objeto, para que pueda dibujar
más. Pulse **Esc** para parar.

## Clics y teclas

- Un **clic izquierdo** crea un punto de esquina.
- Un **clic derecho** crea un punto de curva: la línea pasa por él de
  forma suave.
- **Intro** termina el objeto. Un clic en el primer punto cierra una
  forma.
- **Retroceso** quita el último punto. **Esc** cancela.
- La línea de estado dice qué hacer a continuación.
- La barra de propiedades define los ajustes de los objetos nuevos, por
  ejemplo la longitud de puntada o el ancho de columna. Estos ajustes son
  las propiedades actuales (vea [Telas, estilos y
  plantillas](fabrics-and-styles)).
- El ajuste a la cuadrícula, a las guías o a otros objetos (pestaña Ver)
  funciona mientras dibuja.

## Ayuda al dibujar

- **Arrastre** para hacer un punto de curva con un tirador: pulse el botón
  del ratón donde va el punto y mueva hacia donde va la curva. Un arrastre
  corto hace una esquina.
- Mantenga **Mayús** para que la línea desde el último punto quede en
  pasos de 15°.
- La longitud y el ángulo desde el último punto se muestran junto al
  puntero. El ángulo va en sentido antihorario desde la derecha: 0° es a
  la derecha, 90° es arriba.
- Escriba una longitud y pulse **Intro**: el siguiente punto va a esa
  distancia, hacia el puntero. Escriba una longitud y un ángulo, por
  ejemplo `25<45`, para un punto en ese ángulo. Puede escribir unidades y
  cálculos, por ejemplo `1 in` o `12+3`. **Retroceso** y **Esc** cambian
  primero el texto.
- Con **Ajustar objetos** activado, los puntos se ajustan a los nodos,
  los puntos medios, los centros y los bordes de otros objetos (se muestra
  una marca con su nombre). También se alinean con los puntos que usted
  colocó (se muestra una línea discontinua).

## Líneas

- **Línea**: una línea de puntada recta. Clic izquierdo: esquina; clic
  derecho: punto de curva; Intro: terminar.
- **Manual**: cada clic es un punto de aguja. Úselo para unas pocas
  puntadas exactas, por ejemplo una fijación.
- **A mano alzada**: arrastre para dibujar. El programa suaviza la línea.
- **Dibujo inteligente**: arrastre para dibujar. Un trazo cercano a una
  recta, un rectángulo, un triángulo, un círculo o una elipse se vuelve
  esa forma limpia (una línea o un rectángulo horizontal si está casi
  horizontal). Los demás trazos quedan como se dibujaron. **Ctrl+Z**
  devuelve el trazo.

La barra de propiedades de una línea tiene el tipo de recta (**Recta**,
**Pespunte**, **Punto de tallo**, **Recta escultura**, **Manual**),
**Triple** (cada puntada tres veces, una puntada de frijol), la longitud de
puntada y la pasada: **Una vez**, **Ida y vuelta** (adelante y atrás),
**Repetir** (dos veces), **Redwork** o **Ramificación** (todas las líneas
del objeto como una red). Vea [Tipos de puntada](stitch-types).

## Rellenos

- **Forma**: un área rellena (tatami). Haga clic alrededor del área; Intro
  o un clic en el primer punto la termina.
- **Relleno complejo**: un área con agujeros. Haga clic en el contorno,
  Intro; después en cada agujero, Intro; después Intro otra vez para
  terminar.
- **Forma libre**: arrastre alrededor del área. El programa suaviza el
  contorno.
- **Giratorio**: un relleno cuyas puntadas giran. Haga clic en el
  contorno, Intro; después haga clic en líneas de ángulo de puntada a
  través de la forma (dos clics cada una); Intro.
- **Relleno inteligente**: haga clic dentro de un área que las líneas o
  los contornos de otros objetos cierran por completo. El área se vuelve
  un relleno, con agujeros para las formas dentro de ella. Las líneas
  pueden cruzarse; los huecos de hasta 0,3 mm se cierran.

## Columnas

- **Columna**: una columna de satén. Haga clic en los puntos de un lado y
  pulse **Intro**; después haga clic en los puntos del otro lado y pulse
  **Intro**. Mientras dibuja el segundo lado, el satén llena el espacio
  entre los dos lados. Clic derecho para puntos de curva.
- **Columna por pares**: haga clic en un punto de un lado, después en el
  punto correspondiente del otro lado, par a par. Cada par define allí la
  dirección de las puntadas. Intro: terminar.
- **Columna C**: una columna de ancho fijo a lo largo de una línea. La
  barra de propiedades define el ancho.
- **Anillo**: un anillo de satén. Arrastre de esquina a esquina. La barra
  de propiedades define el ancho.

Las columnas se bordan mejor entre unos 1,5 mm y 7 mm de ancho. Las
columnas más anchas reciben puntos de aguja extra (Propiedades del objeto
> **Dividir puntadas de más de**), o use un relleno.

## Formas

**Rectángulo**, **Rect. redondeado**, **Elipse** y **Estrella** dibujan
formas rellenas. Arrastre de esquina a esquina. **Ctrl** hace lados
iguales (un cuadrado o un círculo); **Alt** dibuja desde el centro. La
barra de propiedades define el radio de las esquinas de un rectángulo
redondeado y el número de puntas de una estrella.

## Decorar

- **Bordes**: bordes decorativos (satén, rectas, festón, cordón, motivos)
  alrededor de la selección, o alrededor de todo el diseño si no hay nada
  seleccionado.
- **Ojales**: un ojal, o una fila de ojales a lo largo de la línea
  seleccionada. Defina la abertura, el ancho de los cordones, el espacio
  para el corte, la presilla y la densidad. Corte cada abertura entre los
  cordones después de bordar.
- **Mezclas**: dos o más colores en un relleno o una columna de satén.
  Cada capa va de densa a abierta, así los colores se mezclan.

Vea [Tipos de puntada](stitch-types) para los ajustes.

## Aplique

**Aplique** dibuja el contorno de una pieza de tela. Haga clic en el
contorno, Intro; después en los agujeros, si los hay; después Intro otra
vez. El programa borda una línea de colocación, se detiene, borda una
fijación, se detiene para que pueda cortar la tela, y borda un satén de
cubierta. Vea [Tutorial: un aplique](tutorial-applique).

## Cambiar la forma: Remodelar

1. Seleccione el objeto y haga clic en **Remodelar** (Digitalizar >
   Seleccionar).
2. Arrastre un nodo para moverlo. Los cuadrados son esquinas; los
   círculos son nodos de curva.
3. Arrastre un tirador para cambiar una curva. **Alt** mientras arrastra
   crea una esquina. Haga doble clic en un nodo para cambiarlo entre
   esquina y curva.
4. Haga doble clic en el contorno para añadir un nodo. **Eliminar** quita
   el nodo seleccionado. **B** (o clic derecho > **Partir en el nodo**)
   corta una línea en dos en el nodo seleccionado; una línea cerrada se
   abre allí.
5. El punto verde es donde el objeto empieza a bordarse; el punto rojo es
   donde termina. Arrástrelos para moverlos.

Con un objeto de letras, Remodelar selecciona una letra: muévala, gírela
o escálela. Con un objeto de puntadas (de un archivo de máquina),
Remodelar mueve, añade y borra puntos de aguja sueltos.

## Ángulos de puntada, agujeros y cortes

Estas herramientas están en el panel **Editar > Forma**:

- **Ángulo**: haga clic en dos puntos a través de un relleno o un satén
  para definir la dirección de las puntadas.
- **Línea de dirección**: haga clic en los puntos de una línea a través
  del satén o del relleno seleccionado, y después pulse **Intro**. Las
  puntadas cruzan la línea, así giran donde gira la línea. La barra de
  propiedades define el ángulo de las puntadas con la línea (90°:
  perpendicular a ella). Un relleno con el tipo de puntada satén conserva
  la línea: Propiedades del objeto cambia su ángulo o la quita. En otro
  relleno, la línea define el ángulo del relleno.
- **Añadir agujero**: dibuje un agujero en el relleno seleccionado.
- **Cuchilla** (Editar > Dividir y unir): corte los objetos seleccionados a
  lo largo de una línea. Haga clic en dos puntos.

Vea [Cambiar objetos](editing) para todos los comandos de edición.
