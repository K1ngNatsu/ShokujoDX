---
id: sketch
title: Bocetos
group: design
keywords: boceto, paramétrico, restricción, cota, cad, tamaño exacto, línea, arco, círculo, rectángulo, polígono, tangente, paralelo, perpendicular, horizontal, vertical, igual, simétrico, fijar, construcción, totalmente restringido, movimientos libres, conflicto, recortar, extender, dividir, redondeo, chaflán, desplazamiento, espejo, patrón, relajar, ajustar
---
# Bocetos

Un boceto es un dibujo con tamaños exactos, como en los programas de CAD.
Usted dibuja líneas, arcos y círculos, y después les da restricciones (por
ejemplo horizontal, tangente o igual) y cotas (longitudes, ángulos y
tamaños). Cuando cambia una cota, el boceto cambia y las restricciones se
siguen cumpliendo. Un boceto no borda nada por sí mismo: usted crea
bordado a partir de sus regiones cerradas y de sus curvas (vea Crear
bordado).

## Empezar y terminar

- **Digitalizar > Iniciar boceto** crea un boceto nuevo, o abre el boceto
  seleccionado. Un doble clic en un boceto también lo abre.
- La pestaña **Boceto** aparece mientras el boceto está abierto. Los demás
  objetos están atenuados y no reciben clics.
- **Terminar boceto** cierra el boceto. Un boceto sin nada desaparece.

Cada cambio es un paso de deshacer; un arrastre es un paso.

## Teclas y ratón

- **Esc** termina el comando y vuelve a **Seleccionar**. En Seleccionar,
  Esc borra la selección.
- **Supr** o **Retroceso** borra las partes seleccionadas con sus
  restricciones. Un punto borrado se lleva sus curvas.
- **Intro** o un clic derecho termina una cadena de líneas.
- **Ctrl+A** selecciona todo. **F8** muestra las marcas de las
  restricciones y **F9** las oculta (también **Mostrar restricciones** en
  el panel Propiedades).
- Con Seleccionar, arrastrar una parte de la selección la mueve entera.
  Ctrl+clic o Mayús+clic añade una parte a la selección; un arrastre en un
  lugar vacío selecciona las partes dentro de un cuadro.
- La parte bajo el puntero se ilumina antes del clic, con sus
  restricciones (también cuando las marcas están desactivadas). Una
  restricción bajo el puntero ilumina sus partes.
- Un **clic derecho** en una parte la selecciona y muestra los comandos
  para ella: cambiar el valor de una cota, añadir una cota o una
  restricción adecuada, recortar, dividir o extender en el clic, redondear
  o achaflanar una esquina, construcción, desplazamiento, espejo, patrones,
  borrar sus restricciones, o borrarla.
- **Deshacer** (Ctrl+Z) y **Rehacer** funcionan como en el resto del
  programa.

## Dibujar

- **Línea**: haga clic en el inicio, y después en cada punto siguiente.
  Haga clic en el primer punto para cerrar la forma; Intro, Esc o un clic
  derecho termina la cadena. Arrastre desde el final de una línea para
  dibujar un arco tangente a ella; la cadena sigue después desde el final
  del arco.
- **Rectángulo**: dos esquinas. Los lados son horizontales y verticales.
- **Rectángulo desde el centro**: el centro, y después una esquina.
- **Rectángulo de tres puntos**: dos esquinas de un lado (en cualquier
  ángulo), y después un punto para la altura.
- **Rectángulo de tres puntos desde el centro**: el centro, el centro de
  un lado, y después una esquina.
- **Ranura**: el tipo está en el panel Propiedades. De centro a centro:
  los dos centros, y después un punto para el ancho. Total: los dos
  extremos, y después el ancho. Punto central: el centro, el centro de un
  extremo, y después el ancho. Arco de tres puntos: los dos centros, un
  punto del arco, y después el ancho. Arco por centro: el centro del arco,
  los dos centros, y después el ancho. Los extremos son semicírculos,
  tangentes a los lados, con el mismo radio.
- **Círculo tangente**: haga clic en tres líneas; el círculo toca las tres
  (del lado de sus clics).
- **Círculo**: el centro, y después un punto del círculo.
- **Polígono**: el centro, y después una esquina. Defina el número de
  lados en el panel Propiedades. Las esquinas están en un círculo de
  construcción y los lados son iguales. Con **Polígono alrededor de su
  círculo**, los lados tocan el círculo y el clic es el centro de un lado.
- **Arco por tres puntos**: el inicio, el final, y después un punto del
  arco.
- **Arco por centro**: el centro, el inicio, y después el final (en
  sentido antihorario).
- **Arco tangente**: haga clic en el final de una línea, y después en el
  final del arco. El arco continúa la línea, tangente a ella.
- **Punto**: un punto, por ejemplo para ponerle una restricción.
- **Spline**: haga clic en los puntos de una curva suave. Intro, un clic
  derecho o un clic en el último punto la termina; un clic en el primer
  punto la cierra. El spline tiene una pieza entre cada dos puntos, con
  dos tiradores (líneas finas desde los puntos). Arrastre los puntos y los
  tiradores para darle forma; una tangente en cada unión lo mantiene
  suave. Un spline puede ser tangente a una línea o a un arco en un
  extremo común, y un punto puede estar sobre él. Las cotas van en sus
  puntos, no en el spline.

Un clic cerca de un punto del boceto usa ese punto, así las líneas que se
encuentran comparten su esquina. **Construcción** convierte las curvas
seleccionadas en curvas de construcción (naranja discontinuo): ayudan a
dibujar y no pasan a ser bordado. Sin nada seleccionado, se activa o
desactiva para las curvas nuevas.

## Ayuda al dibujar

Mientras dibuja, el programa añade restricciones. Una marca junto al
puntero las muestra antes del clic, y los puntos de sus clics aparecen al
momento:

- Un clic cerca de un punto usa ese punto (coincidente).
- Un clic cerca del centro de una línea pone el punto allí (**M**, un
  rombo).
- Un clic cerca del lugar donde se cruzan dos curvas pone el punto en
  ambas (**X**).
- Un clic cerca de una curva pone el punto nuevo sobre la curva (**O**).
- Aparece una línea discontinua cuando el puntero está alineado con un
  punto (a lo ancho o a lo alto): el clic toma su x o su y.
- Una línea casi horizontal o vertical pasa a serlo (**H**, **V**).
- Una línea que empieza o termina en el final de un arco, en la dirección
  del arco, o que toca un círculo en su dirección, pasa a ser tangente
  (**T**).
- Una línea casi a 90° de la línea anterior pasa a ser perpendicular
  (**⊥**). Pase primero el puntero sobre otra línea, y una línea nueva casi
  paralela o a 90° de ella pasa a serlo (**//**, **⊥**).

Las herramientas muestran la forma que crea el clic siguiente.

Mantenga **Alt** al hacer clic para no añadir ninguna (un clic en un punto
sigue usando el punto). La casilla **Añadir restricciones al dibujar** del
panel Propiedades las desactiva.

## Restricciones

Los comandos de restricción son los iconos del panel Restringir (la
descripción da el nombre). Seleccione primero las partes (clic; Ctrl+clic
para más), y después haga clic en una restricción. O haga clic primero en
la restricción, y después en las partes. La línea de estado dice dónde
hacer clic.

| Restricción | Partes |
| --- | --- |
| Coincidente | dos puntos |
| Horizontal, Vertical | una línea, o dos puntos |
| Paralela, Perpendicular, Colineal | dos líneas |
| Tangente | una línea y un círculo o un arco, o dos círculos o arcos |
| Igual | dos líneas, o dos círculos o arcos |
| Concéntrica | dos círculos o arcos |
| Punto medio | un punto y una línea |
| Sobre curva | un punto y una línea, un círculo o un arco |
| Simétrica | dos puntos y la línea entre ellos |
| Fijar | un punto |

Una marca junto a la parte muestra cada restricción (H, V, //, ⊥, T, =, ◎,
M, O, S, F). Seleccione una marca y pulse Supr para quitar la restricción.

## Cotas

Haga clic en **Cota**. La cota que crea su clic siguiente aparece como una
sombra en el puntero, para que vea qué acota. Haga clic en:

- un círculo: su diámetro; un arco: su radio;
- dos puntos: la distancia entre ellos;
- una línea, y después un lugar vacío: su longitud;
- una línea, y después una segunda línea: el ángulo entre ellas;
- un punto y una línea: la distancia del punto a la línea.

Cuando las partes están elegidas, la sombra sigue al puntero: haga clic
donde va la cota. Para una línea inclinada o dos puntos, el lugar elige el
tipo: encima o debajo, una distancia horizontal; a la izquierda o a la
derecha, una distancia vertical; si no, la distancia a lo largo de ellos.
Haga clic derecho mientras la cota espera para elegir el tipo usted mismo:
Alineada, Horizontal o Vertical; para un arco Radio, Diámetro o Longitud
de arco.

Una **Línea de centro** (panel Dibujar; seleccione primero líneas) es una
línea de construcción con trazos largos y cortos. Una cota de un punto a
una línea de centro es un diámetro (⌀), como para una pieza que gira
alrededor de la línea. Con Seleccionar, arrastre una cota para moverla;
Restablecer lugar de la etiqueta (clic derecho) la devuelve.

Si el boceto ya define el tamaño, la cota nueva es **conducida**: muestra
el tamaño entre paréntesis y no cambia el boceto. Haga clic derecho en una
cota para **Hacer conducida** o **Hacer conductora**.

Escriba el valor y pulse Intro (Esc cancela; un clic en otro lugar
conserva el valor). Puede escribir un valor en otras unidades que las del
programa: `2 in`, `1 1/2"`, `3/4 in`, `50 mm`, `3 cm`. También puede
calcular: `2 in + 3 mm`, `120/4`, `(10 + 5) * 2`. Un número sin unidad
está en las unidades del programa, que define en Opciones o en el panel
Propiedades del boceto (mm o in). Un doble clic en una cota la cambia de
nuevo. El panel Propiedades lista todas las restricciones y cotas, con
**Cambiar** para un valor y ✕ para quitar.

## Parámetros

Un parámetro es un valor con nombre, por ejemplo `width = 80 mm`, o una
expresión de otros parámetros, por ejemplo `border = width / 20`.

1. Haga clic en **Parámetros** (pestaña Boceto, panel Cota).
2. Escriba un nombre y un valor o una expresión, y después haga clic en
   **Añadir**. Cambie una expresión o un nombre en la lista; ✕ quita un
   parámetro que ningún valor usa.
3. En el cuadro de valor de una cota, escriba el nombre de un parámetro o
   una expresión, por ejemplo `width`, `width - 2 * border` o
   `max(border, 3 mm)`. La etiqueta muestra `width = 80.00`, o `ƒx` para
   otra expresión.

Cuando cambia un parámetro, las cotas que lo usan cambian, el boceto se
resuelve, y los objetos creados a partir de él lo siguen. Las expresiones
pueden usar unidades, `+ - * / ^`, paréntesis, `pi` y las funciones
`sqrt`, `abs`, `min`, `max`, `sin`, `cos` y `tan` (grados). Un número sin
unidad en una suma con una longitud está en las unidades que muestra la
ventana (mm o in). Un parámetro que se usa a sí mismo, directamente o a
través de otros parámetros, es un bucle: el programa lo rechaza.

**Plantillas**: guarde un diseño con un boceto que tiene parámetros como
plantilla (**Guardar como plantilla**, con sus objetos). Un diseño nuevo a
partir de esa plantilla pide los valores de los parámetros, y después crea
el boceto y sus objetos a ese tamaño.

## Bocetos a partir de gráficos

- **Vector a boceto** (pestaña Arte, Convertir; o clic derecho >
  Convertir): los objetos vectoriales seleccionados pasan a ser un boceto.
  Las partes rectas pasan a ser líneas, las redondas arcos y círculos, y
  las demás curvas splines. El programa añade las restricciones que
  muestra el dibujo: líneas horizontales y verticales, tangentes donde las
  curvas se unen con suavidad, y longitudes y radios iguales. Después
  puede añadir cotas y cambiar la forma.
- Para usar un archivo SVG, PDF o EMF, impórtelo con **Importar como
  vectores**, y después use Vector a boceto.
- Un dibujo **DXF** (Importar arte, o suelte el archivo en la ventana) se
  abre como un boceto nuevo en el centro del bastidor, en las unidades del
  dibujo.

## Colores y movimientos libres

- **Azul**: la parte todavía se puede mover.
- **Verde**: la parte está fija: nada puede moverla.
- **Rojo**: una restricción está en conflicto con las demás. Quítela, o
  cambie un valor.
- Marcas **grises**: restricciones que repiten otras.

La línea de estado da el número de movimientos libres. Un boceto está
totalmente restringido cuando tiene 0: entonces necesita un punto fijo
(Fijar) o cotas desde uno, si no aún puede moverse entero.

## Modificar

Los valores de estas herramientas (el radio, las distancias) están en el
panel Propiedades mientras la herramienta está activa. Cada edición es un
paso de deshacer; una edición que no se puede hacer no cambia nada y da
un mensaje.

- **Recortar**: haga clic en el trozo de una curva que quiere quitar. El
  trozo va de una curva que la corta a la siguiente. Una curva que nada
  corta desaparece; un círculo con dos cortes pasa a ser un arco. Los
  extremos nuevos se quedan en las curvas que los cortan.
- **Extender**: haga clic cerca del final de una línea o de un arco. El
  final llega a la siguiente curva que encuentra la extensión, y se queda
  sobre ella.
- **Dividir**: haga clic en una línea o un arco para cortarlo en dos allí.
  Las dos piezas siguen alineadas (o en un círculo).
- **Redondeo**: haga clic en la esquina de dos líneas. Un arco del radio
  redondea la esquina, tangente a las dos líneas; escriba el radio en el
  cuadro. La esquina queda como un punto en las dos líneas, así una cota a
  la esquina se mantiene. Una longitud de una línea pasa a ser la
  distancia a la esquina.
- **Chaflán**: haga clic en la esquina de dos líneas. Una línea corta la
  esquina: la misma distancia en cada línea, dos distancias (la primera en
  la línea más antigua), o una distancia y un ángulo (defina el tipo en el
  panel Propiedades).
- **Estirar**: arrastre un cuadro alrededor de los puntos que quiere
  mover, y después haga clic en un punto base y en el lugar adonde va. Las
  curvas que terminan en los puntos se estiran.
- **Desplazamiento**: seleccione curvas y haga clic en Desplazamiento, o
  haga clic en Desplazamiento y después en una curva (ella y las curvas
  unidas a ella). La copia está a la distancia: positiva por fuera de una
  forma cerrada (si no, a la izquierda), negativa por el otro lado. Una
  línea desplazada sigue paralela a su distancia; un arco desplazado
  conserva su centro.

## Mover, copiar, girar y escalar

Seleccione primero las partes.

- **Mover**: haga clic en un punto base, y después en el lugar adonde va.
  O escriba X e Y en el panel Propiedades y haga clic en **Mover con los
  valores**.
- **Copiar**: como Mover, con una copia; las partes se quedan.
- **Girar**: defina el ángulo en el panel Propiedades (positivo es en
  sentido antihorario), y después haga clic en el centro.
- **Escala**: defina el factor, y después haga clic en el punto base. Las
  cotas entre las partes también se escalan.

Una vista previa discontinua muestra adónde van las partes. Girar y Escala
pueden copiar (**Copiar** en el panel Propiedades). Las partes se mueven
tan cerca como lo permiten sus demás restricciones.

## Copiar

Seleccione primero las curvas.

- **Espejo**: haga clic en Espejo, y después en la línea respecto a la que
  se refleja. Cada punto de la copia es simétrico a su punto (un punto
  sobre la línea se comparte), así la copia sigue cuando cambia las
  curvas.
- **Patrón rectangular**: defina las columnas, las filas y el espaciado en
  el panel Propiedades, y después haga clic en **Crear patrón**.
- **Patrón circular**: defina las copias y el ángulo (360° la vuelta
  entera), y después haga clic en el centro.

Las copias de un patrón tienen las restricciones de las curvas entre
ellas, pero sus lugares son libres.

## Relajar

Cuando hay restricciones en conflicto (rojo), **Relajar** las quita,
primero las cotas, hasta que no queda conflicto. El mensaje y el panel
Propiedades listan lo que quitó. Deshacer las devuelve.

## Cotas automáticas

**Cotas automáticas** (panel Cota) añade lo que necesitan las curvas
seleccionadas (o todas las curvas): horizontal y vertical para las líneas
que casi lo son, y después longitudes, diámetros, radios y ángulos, con
los tamaños actuales. No añade nada que repita o entre en conflicto. El
lugar del boceto sigue libre: añada un **Fijar** a un punto para
situarlo.

## Puntos centrales

Seleccione puntos y haga clic en **Punto central** (panel Dibujar) para
mostrarlos como una cruz, por ejemplo el lugar de un agujero.

## Modo relajado

**Modo relajado** (panel Cota) se activa o desactiva. Mientras está
activo, un arrastre cambia las cotas que sujetan las partes: toman los
tamaños nuevos. Las demás restricciones (horizontal, tangente, etc.) se
quedan.

## Crear bordado

Las curvas (no las curvas de construcción) forman regiones cerradas donde
se cruzan: un círculo dentro de un círculo forma un anillo y un disco;
una forma dentro de una región es una isla, y la región recibe un agujero
allí.

1. Haga clic en **Regiones** (pestaña Boceto), y después haga clic dentro
   de las regiones que quiere (quedan sombreadas; haga clic otra vez para
   quitarlas). Sin ninguna región elegida, los comandos toman todas las
   regiones.
2. En **Crear bordado** (panel Propiedades), o con un clic derecho, haga
   clic en **Relleno**, **Satén**, **Aplique** o **Vector**.

Para las curvas (también las de construcción): selecciónelas, y después
**Recta**, **Línea de satén** (el ancho está al lado) o, para dos curvas,
**Satén entre** (una columna de satén con las dos curvas como lados).

Los objetos nuevos toman las propiedades actuales y el hilo actual, y van
al final del orden de bordado. Cuando el boceto está cerrado, su panel
Propiedades crea bordado con todas sus regiones, o rectas a lo largo de
todas sus curvas.

## Objetos que siguen al boceto

Los objetos que crea a partir de un boceto siguen vinculados a él. Cuando
cambia una cota o una restricción, el programa los vuelve a crear.
Conservan sus ajustes de puntada, su hilo y su lugar en el orden de
bordado. Deshacer quita el cambio y los objetos nuevos en un paso.

- Para cambiar el boceto otra vez, haga doble clic en uno de sus objetos,
  o haga clic en **Editar boceto** (menú del clic derecho o panel
  Propiedades).
- Cuando mueve, gira o escala el boceto, sus objetos se mueven con él.
  Cuando mueve solo los objetos, se quedan a esa distancia del boceto.
- Si un cambio abre una región o borra una curva, sus objetos se quedan
  como están, y el programa muestra una advertencia. Cierre la región de
  nuevo, o haga clic en **Desvincular** para convertirlos en objetos
  normales.
- Los comandos de forma no aceptan objetos vinculados. Desvincúlelos
  primero. Un cambio a mano de la forma de un objeto vinculado no se
  conserva después del siguiente cambio del boceto.

## Arrastrar

Con **Seleccionar**, arrastre un punto o una línea: el boceto se mueve con
él y las restricciones se siguen cumpliendo. Arrastre el borde de un
círculo para cambiar su tamaño.
