---
id: arranging
title: Organizar y el orden de bordado
group: design
keywords: organizar, girar, rotar, reflejar, voltear, sesgar, inclinar, puntos de referencia, centrar, alinear, espaciar por igual, distribuir, mismo tamaño, copias, disposición, espejo combinado, matriz, corona, caleidoscopio, reflejar, agrupar, desagrupar, bloquear, desbloquear, orden de bordado, secuencia, cambios de color, orden más corto, orden para gorras, unión más cercana, orden de clic
---
# Organizar y el orden de bordado

La pestaña **Organizar** mueve y copia objetos, y define el orden en el
que se bordan.

## Transformar

- **Girar 90° a la izquierda**, **Girar 90° a la derecha** (Ctrl+R) y
  **Girar 180°** giran la selección.
- **Espejo izquierda-derecha** (H) y **Espejo arriba-abajo** (V) la
  reflejan.
- **Centrar en el bastidor** mueve la selección al centro del bastidor.
- **Línea de espejo**: haga clic en dos puntos de una línea; la selección
  se refleja respecto a ella. Mantenga **Ctrl** para conservar el
  original.
- **Puntos de referencia**: haga clic en dos puntos de la selección, y
  después en los lugares adonde van. La selección se mueve, gira y escala
  para encajar.
- **Girar** gira la selección el ángulo del campo de al lado (en sentido
  horario). **Sesgar** la inclina con los ángulos X e Y.

Para girar todo el diseño en el bastidor, use **Diseño > Girar diseño**.

## Alinear, espaciar y tamaño

Seleccione dos o más objetos. El primer objeto seleccionado es el que
siguen los demás.

- **Alinear**: bordes **Izquierda**, **Centro**, **Derecha**, **Arriba**,
  **Medio** y **Abajo**.
- **Espacio y tamaño**: **A través** y **Abajo** espacian por igual tres o
  más objetos. **Mismo ancho**, **Misma altura** y **Mismo tamaño** dan a
  la selección el tamaño del primer objeto seleccionado.

## Copias

- **Disposiciones** abre una ventana que crea copias de la selección:
  - **Espejo combinado**: una copia reflejada a la derecha, abajo, o ambas,
    con un espacio.
  - **Matriz**: filas y columnas de copias, con un espacio a lo ancho y a
    lo alto.
  - **Corona y caleidoscopio**: copias alrededor de un centro bajo la
    selección, a un radio. Un caleidoscopio tiene una copia reflejada en
    cada parte.
- **Espejo a la derecha**, **Espejo abajo** y **Espejo en ambos** añaden
  copias reflejadas al momento.
- **Reflejar**: haga clic en dos puntos de una línea; se añade una copia
  reflejada de la selección.

## Agrupar y bloquear

- **Agrupar**: un clic en un objeto selecciona todo el grupo. Un doble
  clic en un objeto de un grupo selecciona solo ese objeto, por ejemplo
  para cambiar su tipo de puntada; un clic en un objeto fuera del grupo
  vuelve a seleccionar grupos enteros. Cuando hay un grupo seleccionado, el
  panel Propiedades también tiene un botón para cada objeto del grupo.
  **Desagrupar** saca la selección de sus grupos.
- **Bloquear**: los objetos bloqueados no se pueden seleccionar ni
  cambiar. **Desbloquear todo** los desbloquea.

## El orden de bordado

El panel **Secuencia** muestra los objetos en el orden en que se bordan.
Un objeto que se borda después queda encima. Arrastre objetos en el
panel, o use los botones de su parte superior.

- **Al inicio**, **Atrás uno**, **Adelante uno** y **Al final** mueven la
  selección en el orden.
- **Atrás un color** y **Adelante un color** la mueven más allá de los
  objetos de un color.
- **Por orden de clic**: haga Ctrl+clic en los objetos en el orden que
  quiere, y después haga clic aquí.

## Menos cambios de color y cortes

Cada cambio de color detiene la máquina (o cambia la aguja), y cada corte
cuesta tiempo.

- **Menos cambios de color** une bloques de un color donde ningún objeto
  superpuesto estorba. Lo que está encima sigue encima. El panel Secuencia
  tiene el mismo comando (**Unir colores**).
- **Agrupar por color**: el mínimo de cambios de color. Puede poner un
  objeto debajo de otro que lo cubría, así que compruebe el diseño
  después.
- **Orden más corto** borda primero los objetos más cercanos de cada
  color, para que la aguja no vaya de un lado a otro: menos
  desplazamientos y menos cortes. Los colores y lo que está encima no
  cambian. La línea de estado dice cuántos cortes y cuánto desplazamiento
  ahorró.
- **Unión más cercana**: cada objeto termina en el punto más cercano al
  siguiente, que empieza allí. Así los huecos cortos reciben puntadas de
  desplazamiento, no cortes.

## Gorras

Una gorra se mueve en su marco cuando las puntadas tiran de ella. **Orden
para gorras** borda los objetos de cada color desde el centro hacia fuera
y de abajo arriba, y las letras desde la letra central hacia fuera, para
que la tela de la gorra no se mueva. Con un marco de gorras como bastidor,
la información del diseño avisa cuando el orden no es así.

## Los puntos de inicio y de fin

Las puntadas de un relleno o de un satén dependen de dónde está la aguja
cuando empieza el objeto, así que un cambio del orden de bordado puede
cambiarlas un poco. Remodelar muestra el inicio (verde) y el final (rojo)
del objeto seleccionado; arrástrelos para cambiar dónde empieza y
termina. **Diseño > Opciones de bastidor > Inicio y fin automáticos**
define adónde va la aguja antes de la primera puntada y después de la
última.
