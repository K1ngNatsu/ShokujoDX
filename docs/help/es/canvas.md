---
id: canvas
title: El lienzo y la selección
group: design
keywords: acoplar, acoplada, ventana de comando, esquina, lienzo, zoom, desplazar, mover, vista, seleccionar, selección, cruce, cuadro, lazo, empujar, flecha, tiradores, escalar, ocultar, cuadrícula, guías, reglas, marcador de regla, línea de construcción, punto de construcción, discontinua, bloquear guía, color de guía, intersección, ajustar, medir, vistas, vista dividida, vista general, otro diseño, comparar, lado a lado, paneles, modo oscuro
---
# El lienzo y la selección

El lienzo muestra el bastidor y el diseño. La parte superior del lienzo es
la parte trasera de la máquina.

## Zoom y desplazamiento

- Gire la rueda del ratón para hacer zoom en el puntero.
- Arrastre con el botón central o el derecho del ratón para mover la
  vista.
- Haga doble clic con la rueda del ratón, o pulse **Inicio**, para ver el
  bastidor y todo el diseño (**Ver > Zoom a la extensión**).
- **+** y **-** acercan y alejan (**Ver > Acercar**, **Alejar**). Opciones
  define el paso del zoom.
- **Ver > Zoom** tiene también **Zoom 100 %** (el tamaño real; calibre la
  pantalla en Opciones), **Zoom al bastidor**, **Zoom a la selección**, la
  herramienta **Zoom** (arrastre un cuadro), una lista de factores de zoom,
  y ½× y 2×.
- **Vista anterior** vuelve a la vista de antes del último zoom.

## Más vistas

- **Ver > Vista general** muestra una ventana pequeña con todo el diseño.
  Arrastre en ella para mover la vista.
- **Ver > Vistas** tiene:
  - **Vista dividida**: una segunda vista junto a la primera, con su propio
    zoom y su aspecto de hilo. Úsela para ver el tamaño real y un detalle
    a la vez.
  - **Vista nueva**: hasta tres vistas más del diseño.
  - **Otro diseño**: otro diseño abierto junto a este. Haga clic en
    **Editar este diseño** para trabajar en él.

## Seleccionar objetos

- Haga clic en un objeto para seleccionarlo. **Ctrl+clic** añade o quita
  un objeto. **Mayús+clic** selecciona todos los objetos entre dos objetos
  del orden de bordado.
- Arrastre un cuadro hacia la **derecha**: selecciona los objetos que
  están del todo dentro del cuadro. Arrastre un cuadro hacia la
  **izquierda**: selecciona los objetos que toca.
- **Tab** y **Mayús+Tab** seleccionan el objeto siguiente y el anterior
  del orden de bordado.
- **Ctrl+A** selecciona todos los objetos.
- **Lazo**, **Selección poligonal** y **Selección por línea** (Digitalizar
  > Seleccionar) seleccionan con una línea que usted dibuja: los objetos
  dentro de una línea a mano alzada, dentro de un polígono, o los objetos
  que cruza una línea.
- **Editar > Seleccionar** selecciona **Por color** o **Por tipo**, y **En
  el cursor de puntada**.
- Un clic en un objeto de un grupo selecciona todo el grupo.

## Mover y cambiar la selección

- Arrastre la selección para moverla. Los tiradores de las esquinas la
  escalan; el tirador redondo la gira. Al escalar, el centro se queda en
  su sitio (la pequeña cruz); **Propiedades del objeto > Posición y
  tamaño > Escalar desde** define otro punto. **Alt** mientras arrastra un
  tirador: desde el lado opuesto.
- Las teclas de flecha mueven un poco la selección. **Mayús** y una tecla
  de flecha la mueven más. Opciones define las distancias.
- **Propiedades del objeto > Posición y tamaño** define el lugar, el ancho
  y el alto como números. **Mantener proporciones** conserva la forma.
  **Inclinar** inclina la selección: ↔ mueve el borde superior a la
  derecha, ↕ mueve el lado derecho hacia abajo (grados).
- **H** y **V** reflejan la selección; **Ctrl+R** la gira 90°. Vea
  [Organizar y el orden de bordado](arranging) para todas las formas de
  girar, alinear y copiar.
- Haga clic derecho en la selección para un menú con los comandos de
  editar y organizar.

## Ocultar y mostrar

- **Alt+V** oculta en pantalla los objetos seleccionados, para ver lo que
  hay debajo. Se siguen bordando. Alt+V sin nada seleccionado muestra de
  nuevo todos los objetos.
- **Ver > Filtro** muestra solo algunos objetos: **Solo estos colores**,
  **Ocultar los demás** y **Mostrar colores** (una lista con una casilla
  para cada color). **Mostrar todo** muestra todo de nuevo.
- **Ver > Mostrar** activa y desactiva las partes de la imagen: los
  contornos de los objetos (**Arte**), las **Puntadas**, el **Aspecto de
  hilo**, los **Saltos**, las **Penetraciones de aguja**, los símbolos de
  **Cortes y colores**, la **Tela de aplique**, los **Vectores** y el
  **Mapa de densidad**.

## Cuadrícula, reglas y guías

- **Ver > Cuadrícula** muestra una cuadrícula. Opciones define el espaciado
  y una línea principal.
- **Ver > Reglas** muestra reglas arriba y a la izquierda. Arrastre desde
  una regla para crear una guía. Cada guía tiene un marcador en su regla:
  arrastre el marcador para mover la guía, y arrástrelo fuera del lienzo
  para eliminar la guía. Con la herramienta Seleccionar también puede
  arrastrar una línea guía y soltarla en una regla para eliminarla.
- Haga doble clic en un marcador (o en una línea guía con la herramienta
  Seleccionar) para escribir el lugar de la guía, desde el centro del
  bastidor.
- Haga clic derecho en un marcador (o en una línea guía o una marca de
  construcción con la herramienta Seleccionar) para ver su menú:
  **Posición…**, **Mover al centro del bastidor**, **Color**,
  **Discontinua** (si no, continua), **Bloqueada** (arrastrar no la
  mueve), **Duplicar** (una copia a un paso de la cuadrícula) y
  **Eliminar**.
- **Ver > Línea de construcción** dibuja una línea de ayuda discontinua
  por dos puntos, en cualquier ángulo (Mayús: pasos de 15°). **Ver >
  Punto de construcción** pone un punto de ayuda. No se bordan.
- **Quitar guías** quita todas las guías, líneas de construcción y puntos
  de construcción.
- **Ajustar a la cuadrícula**, **Ajustar a las guías** y **Ajustar a los
  objetos** hacen que los puntos y los movimientos vayan a la cuadrícula,
  a las guías o a otros objetos. Con **Ajustar a las guías**, los puntos
  también van a las líneas y los puntos de construcción, y a los lugares
  donde cruzan las guías y se cruzan entre sí. Con **Ajustar a los
  objetos**, los puntos van a los nodos, los puntos medios, los centros y
  los bordes de otros objetos.
- **Ver > Medir** mide una distancia y un ángulo. Arrastre.

## Ventanas de comandos

Las ventanas de los comandos (por ejemplo Trazado de borde,
Desplazamientos, Letras, Monograma y Tela) se acoplan en una esquina del
lienzo, como en los programas de CAD:

- Una ventana de comando se abre acoplada en la esquina superior izquierda
  del lienzo.
- Arrástrela por su título, o por un lugar vacío de ella. Suéltela cerca
  de una esquina del lienzo: se acopla en esa esquina. Suéltela en otro
  lugar: se queda allí.
- Una ventana acoplada se queda en su esquina cuando el lienzo cambia de
  tamaño, por ejemplo cuando se abre un panel.
- Dos ventanas en una esquina se ponen una debajo de la otra.

## El espacio de trabajo

- **Ver > Paneles** abre y cierra cada panel: Propiedades del objeto,
  Estilos, Información del diseño, Cliparts, Secuencia, Hilos, Lista de
  puntadas, Exportaciones y Biblioteca.
- Arrastre la pestaña de un panel para ponerlo en otro lugar, o junto a
  otro panel.
- **Ver > Modo oscuro** cambia entre paneles claros y oscuros.
- **Ver > Opciones** tiene los demás ajustes del espacio de trabajo. Vea
  [Opciones](options).

## El reproductor de puntadas y el cursor de puntadas

Vea [Comprobar un diseño](checking): el reproductor muestra cómo borda la
máquina el diseño, y el cursor de puntadas marca una puntada.
