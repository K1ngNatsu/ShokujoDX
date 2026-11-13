---
id: troubleshooting
title: Preguntas y problemas
group: start
keywords: problema, pregunta, preguntas frecuentes, fruncido, hueco, corte, salto, rotura, rígido, texto pequeño, la máquina no lee, archivo, lento, cierre, falta, color equivocado, demasiadas puntadas, superposición, acumulación, svg, densidad, mapa de densidad, capas, imagen, foto, imagen a diseño, objetos, el hilo se rompe, nido de pájaro, bucles, el contorno no coincide, registro, desplazamiento, pelo, toalla, se hunde, descentrado, bastidor, demasiado grande, no cabe, las letras se cierran, botón atenuado, no puedo hacer clic, panel perdido
---
# Preguntas y problemas

## La tela se frunce

Las puntadas juntan la tela. Use una entretela más fuerte (una recortable
para tejidos de punto) y coloque la tela tensa en el bastidor. Defina la
tela correcta (**Diseño > Tela**). Haga el relleno menos denso, o use una
base más ligera.

## Hay huecos entre un relleno y su borde

Las puntadas encogen el relleno. Aumente la **compensación de tiro**, o
deje que el borde pase un poco sobre el relleno. **Arte > Trazado de
borde** añade un borde que pasa bajo el filo del relleno; ponga su
**Posición del borde** en **Centrado en la línea** o **Dentro de la
línea** para cubrir más del filo. Imagen a diseño tiene un ajuste
**Superposición** por la misma razón.

## Los contornos no coinciden con los rellenos

La tela se movió mientras la máquina bordaba. Use una entretela más fuerte
y coloque la tela tensa. Borde cada contorno justo después del relleno que
tiene debajo, no al final del diseño. Más base ayuda a que el relleno
conserve su forma.

## El diseño tiene demasiados cortes

Cada corte cuesta tiempo. Mantenga juntos los objetos de un color en el
orden de bordado (**Organizar > Menos cambios de color**). **Organizar >
Orden más corto** pone primero los objetos más cercanos de cada color.
**Unión más cercana** da a los huecos cortos puntadas de desplazamiento,
no cortes. **Cortar después: Desplazamiento oculto** (Funciones de
máquina) borda puntadas de desplazamiento bajo los colores que se bordan
después, en lugar de un corte. La información del diseño avisa de los
objetos con muchos cortes: pruebe otro punto de inicio o de fin.

## Imagen a diseño crea demasiados objetos

Una foto tiene muchas áreas pequeñas de cada color, y cada área separada
de las demás necesita un corte. El asistente da a sus objetos
**Desplazamiento oculto**: donde los colores que se bordan después cubren
el camino, la aguja va allí con puntadas de desplazamiento y sin corte. En
el asistente:

- **Fondo:** use **Recortar el motivo**, para que solo se borde el motivo.
  **Quitar el color del borde** funciona bien solo con un fondo liso.
- **Simplificar:** haga las motas más grandes (**Quitar motas menores
  que**), use menos detalle (un **Detalle (mm por píxel)** mayor), o use
  más **Igualar**.
- **Terminar:** las comprobaciones muestran los cortes y los objetos muy
  pequeños, y un botón cambia los ajustes por usted.

Un logotipo o un dibujo con colores planos da el mejor resultado. Para la
foto de una persona, **Puntada fotográfica** suele ser mejor.

## El satén de un SVG se amontona en la máquina

Convierta de nuevo los vectores con **A satén**, o ponga el tipo de
puntada del relleno en **Satén**: así cada forma se borda por partes, sin
ninguna puntada fuera de ella. Para controlar la dirección de las
puntadas, use **Editar > Línea de dirección** en el objeto.

## El texto pequeño no se borda bien

Las fuentes de bordado tienen un rango de tamaños; la lista de fuentes lo
muestra. Por debajo de unos 5 mm, las letras pierden su forma. Use una
fuente hecha para texto pequeño (**Fuentes**, luego **Para texto
pequeño**: estas fuentes llegan hasta 4 mm), o haga el texto más grande.
Ponga la base del texto pequeño en **Ninguno** o **Ligero**.

## Las letras se cierran o se ven pesadas

El satén es demasiado denso para el tamaño, o el hilo es demasiado grueso.
Haga más abierta la densidad de las letras (Propiedades del objeto >
Satén > Densidad), use una base más ligera, o haga el texto más grande.

## Las puntadas se hunden en una toalla o en forro polar

El pelo sale entre las puntadas. Añada **Especial > Capa base** bajo el
diseño, use un recubrimiento soluble en agua y defina la tela **Towel
(terry)** o **Fleece and sweatshirt**. Vea [Tutorial: un monograma en una
toalla](tutorial-towel).

## El diseño es rígido y la aguja se rompe

Las puntadas son demasiado densas, o hay muchas capas superpuestas. Para
encontrar los lugares, active **Ver > Mapa de densidad**: colorea el
diseño según las capas de hilo, y los lugares por encima del límite salen
en rojo. La información del diseño lista cada lugar; haga clic en uno para
ir a él. El límite está en **Opciones** (5 capas al principio).

Aumente la densidad (más abierta). Quite las partes ocultas de los objetos
de abajo con **Arte > Formas > Aplanar**.

## El hilo se rompe o hace bucles bajo la tela

- Las puntadas muy cortas rompen el hilo: la información del diseño avisa
  de las puntadas más cortas que la puntada más corta.
- Demasiadas capas en un lugar (vea el mapa de densidad).
- En la máquina: compruebe el enhebrado, la tensión, la aguja (una aguja
  de bordar nueva del tamaño correcto) y la bobina.

## La máquina no lee el archivo

- Compruebe el formato de su máquina (la lista **Máquina** de la pestaña
  Diseño). La mayoría de las máquinas domésticas solo leen su propio
  formato.
- Compruebe la carpeta de la memoria USB. **Enviar a USB** usa la carpeta
  de su máquina.
- Compruebe que el diseño cabe en el bastidor de la máquina: la
  información del diseño dice si hay puntadas fuera del campo del
  bastidor.

## El diseño no está en el centro en la máquina

La máquina pone el diseño donde dice su archivo, a partir del centro del
bastidor. **Organizar > Centrar en el bastidor** mueve la selección al
centro del bastidor. Use **Diseño > Plantilla del bastidor** para marcar
el lugar en la tela. **Opciones de bastidor > Inicio y fin automáticos**
define adónde va primero la aguja.

## El diseño es más grande que mi bastidor

**Bastidor automático** selecciona un bastidor más grande si la máquina
tiene uno. **Diseño > Girar diseño** gira el diseño 90°, y **Girar el
bastidor** gira el bastidor. O borde el diseño por partes: vea [Diseños
más grandes que el bastidor](hooping).

## Los colores en la máquina son incorrectos

Formatos como DST no tienen colores. La máquina muestra sus propios
colores. **Producir diseño** puede escribir archivos de colores de hilo
junto al archivo de máquina. Compruebe también qué combinación de colores
es la actual (pestaña Color): Exportar escribe la combinación actual.

## Un archivo de máquina queda demasiado denso al escalarlo

Un archivo de máquina tiene puntos de aguja, no formas. Use primero **Arte
> Convertir > Reconocer contornos**: las puntadas pasan a ser rellenos,
que conservan su densidad al escalarlos.

## Un botón está atenuado

El comando necesita algo antes, por ejemplo una selección, una imagen de
fondo o dos objetos. Ponga el puntero sobre el botón: la descripción dice
qué hace, y la ayuda (F1) dice qué necesita.

## No encuentro un comando

Pulse **Ctrl+K** y escriba una parte de su nombre. El tema de ayuda de
cada pestaña de la cinta lista todos sus comandos. En el modo simple,
algunos comandos solo están en la cinta completa: desactive el modo
simple (Ver > Modo simple).

## Un panel ha desaparecido

La × de un panel lo cierra. **Ver > Paneles** lo abre de nuevo.

## ¿Por qué el programa empieza con una página?

Es la página de inicio. Desmarque en ella **Mostrar al inicio** si quiere
el diseño vacío al empezar.

## El programa se cierra o va lento

El programa guarda copias de seguridad y las ofrece en el próximo inicio.
Hay un informe de error en la carpeta `crashes` de los ajustes (**Ayuda >
Acerca de** muestra dónde). Envíelo con sus comentarios. Un diseño con
muchísimas puntadas o una imagen de fondo grande es más lento; oculte el
aspecto del hilo mientras trabaja.
