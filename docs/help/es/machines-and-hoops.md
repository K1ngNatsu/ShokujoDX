---
id: machines-and-hoops
title: Máquinas y bastidores
group: production
keywords: máquina, topaz, husqvarna, brother, tajima, varias agujas, agujas, aguja, secuencia de agujas, cambio de hilo, plan de agujas, formato, velocidad, tiempo de bordado, tiempo de cambio de color, tiempo de corte, cortes dst, saltos para un corte, bastidor, marco, campo, margen, mis bastidores, bastidor propio, bastidor automático, girar bastidor, girar diseño, ángulo del bastidor, opciones de bastidor, colocación, centrado automático, punto de inicio, punto final, plantilla del bastidor, imprimir, varias posiciones
---
# Máquinas y bastidores

## La máquina

La lista **Máquina** (Diseño > Máquina y bastidor) define la máquina del
diseño. La máquina define:

- el formato de archivo de Exportar y Enviar a USB (por ejemplo VP3 para la
  Husqvarna Viking Designer Topaz 20, PES para Brother, DST para la mayoría
  de las máquinas de varias agujas);
- la puntada más larga, y los bastidores de la máquina;
- la velocidad y los tiempos para el tiempo de bordado.

**Ajustes de la máquina** (Diseño > Máquina y bastidor) tiene:

- **Velocidad** (puntadas por minuto), y el tiempo de un **Cambio de
  color**, un **Corte** y una **Parada**. La información del diseño y la
  hoja de trabajo los usan para el tiempo de bordado.
- **Cortes en archivos DST y TAP**: estos formatos no tienen código de
  corte, por eso unos pocos saltos cortos le dicen a la máquina que corte
  el hilo. Cambie **Saltos para un corte** y **Longitud de cada salto**
  solo si su máquina no corta, o corta demasiadas veces.
- **Cabezales**, **Enmarcar una pieza**, **Cambio de bastidores** y
  **Enmarcar las siguientes piezas mientras la máquina borda**: para el
  tiempo de producción de un pedido. Vea [Pedidos, presupuestos y tiempos de
  producción](quotes).

## Agujas de una máquina de varias agujas

Una máquina de varias agujas tiene un hilo en cada aguja y va sola a la
aguja del color siguiente. **Agujas** (Diseño > Máquina y bastidor)
muestra:

- **Agujas de la máquina**: el número de agujas. Las máquinas de varias
  agujas tienen 15; defina el número de su máquina. **Restablecer** usa el
  número del perfil de la máquina. Una máquina de una aguja no tiene plan
  de agujas.
- **Colores en orden de bordado**: la aguja de cada color. **Auto** deja un
  hilo en la aguja que lo tiene (un color que se borda dos veces usa una
  aguja) y pone un hilo nuevo en una aguja libre. Seleccione un número de
  aguja para dar al hilo su propia aguja; deshacer lo devuelve.
- **Agujas al inicio**: el hilo que debe tener cada aguja antes de empezar,
  y el hilo que hay ahora en la máquina. **poner** muestra los hilos que
  hay que cambiar.
- **Secuencia de agujas**: la aguja de cada color en el orden de bordado.
  Escríbala en la máquina para los archivos DST, TAP y EXP; los archivos
  TBF y U01 tienen las agujas.
- **Dejar estos hilos en la máquina**: después de bordar el diseño, sus
  hilos están en las agujas. El diseño siguiente los usa donde puede, así
  cambia menos hilos. **Borrar los hilos de la máquina** los olvida.

Si el diseño tiene más hilos que agujas la máquina, una aguja recibe un
hilo nuevo durante el diseño: un **cambio de hilo** (*). Los archivos TBF
y U01 detienen allí la máquina, la hoja de trabajo dice qué hilo va en qué
aguja, y el tiempo de bordado cuenta una parada. La información del diseño
muestra una advertencia.

La hoja de trabajo de producción tiene una columna **Aguja**, las
**Agujas al inicio**, la secuencia de agujas y los cambios de hilo. Un
archivo TBF o U01 abierto conserva sus agujas.

## El bastidor

La lista **Bastidor** define el bastidor. El lienzo muestra el campo del
bastidor (adonde puede ir la aguja) y el margen de seguridad.

- **Mis bastidores** van primero en la lista. Un bastidor que la máquina no
  puede usar muestra una advertencia.
- **Campo abierto** no tiene bastidor.
- **Bastidor automático** selecciona el bastidor más pequeño en el que cabe
  el diseño, de Mis bastidores (si no, de los bastidores de la máquina). El
  diseño gira 90° si así cabe en un bastidor más pequeño.
- **Mostrar bastidor** muestra u oculta el bastidor en el lienzo.
- El **margen de seguridad del bastidor** (Opciones) es una distancia
  dentro del campo. Las puntadas fuera del margen reciben una advertencia
  en la información del diseño; las puntadas fuera del campo son un error.

La parte superior del lienzo es la parte trasera de la máquina, y el ancho
del bastidor va de izquierda a derecha en la máquina.

## Girar el diseño o el bastidor

- **Girar diseño** gira todo el diseño 90° en sentido horario dentro del
  bastidor.
- **Girar bastidor a la izquierda** y **Girar bastidor a la derecha** giran
  el bastidor 15°, para que quepa un diseño inclinado. El archivo de
  máquina gira de vuelta las puntadas, así el diseño se borda como lo ve.
- **Opciones de bastidor > Ángulo del bastidor** define el ángulo como un
  número.

## Mis bastidores

**Diseño > Mis bastidores** lista los bastidores. Marque **Lo tengo** en
los bastidores que tiene: Bastidor automático y la lista de bastidores los
usan primero.

**Bastidor nuevo…** añade un bastidor propio: el **Nombre**, la **Forma**
(rectángulo, rectángulo redondeado, óvalo o círculo), el **Ancho** y la
**Altura** del campo, y el **Radio de esquina**. El tamaño es el campo que
se puede bordar, no el marco. Un bastidor de varias posiciones tiene
**posiciones**: el centro de cada posición a partir del centro del
bastidor. **Editar** y **Eliminar** cambian sus bastidores.

## Opciones de bastidor

**Diseño > Opciones de bastidor** tiene:

- **Colocación**: **Manual** (el bastidor se queda donde usted lo pone) o
  **Centrado automático** (el centro del bastidor sigue el centro del
  diseño).
- **Mostrar el bastidor**, y el **Ángulo del bastidor**.
- **Inicio y fin automáticos**: la aguja va al punto de inicio antes de la
  primera puntada, y al punto final después del último corte. Elija el
  centro del bastidor, un lugar del cuadro del diseño (por ejemplo **Arriba
  a la izquierda**) o **Desactivado**. Allí no se borda ninguna puntada.
  Los archivos VP3 no tienen saltos, así que allí el punto de inicio solo
  define dónde empieza el primer objeto.

## La plantilla del bastidor

**Diseño > Plantilla del bastidor** guarda el campo del bastidor a tamaño
real como PDF, con las líneas de centro y el diseño. Imprímala al 100 % y
póngala sobre la tela, para poner el diseño en el lugar correcto. Un
bastidor grande se imprime en más hojas.

## Diseños más grandes que el bastidor

Vea [Diseños más grandes que el bastidor](hooping).
