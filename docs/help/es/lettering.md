---
id: lettering
title: Letras y monogramas
group: design
keywords: letras, texto, nombre, fuente, recorte, superposición, fuente de bordado, truetype, opentype, rango de tamaños, texto pequeño, altura, espaciado entre letras, interlineado, interletraje, alinear, línea base, arco, círculo, línea, envolvente, arte de letras, orden de bordado, cortes, saltos, letra, remodelar, alfabeto, fuente de mapa de bits, monograma, iniciales, nombres de equipo, lista de nombres, csv, diseños de teclado, quiosco, tienda, inglete, esquinas en inglete, esquina, negrita, cursiva, subrayado, estilo, vertical, apiladas, dirección del texto
---
# Letras y monogramas

La pestaña **Letras** añade texto. El tutorial [Un nombre en un
arco](tutorial-lettering) muestra los primeros pasos.

## Añadir letras

1. Haga clic en **Letras > Letras**, y después haga clic en el lienzo
   donde empieza el texto.
2. En la ventana Letras, escriba el texto. **Intro** empieza una línea
   nueva. El lienzo muestra las letras mientras escribe.
3. Seleccione una fuente, defina la **Altura** y el **Tipo de puntada**.
   La lista de fuentes muestra cada nombre en su propia fuente. Señale un
   nombre, y el lienzo muestra su texto en esa fuente.
4. Haga clic en **Aceptar** (o **Ctrl+Intro**). **Esc** o **Cancelar**
   cierra la ventana y quita lo que muestra.

Para cambiar el texto después, haga doble clic en las letras, o haga clic
derecho en ellas y seleccione **Editar texto…**.

## Fuentes

Hay dos tipos de fuentes:

- Las **fuentes de bordado** tienen letras de bordado ya hechas. Se bordan
  mejor. Cada una tiene un **rango de tamaños** (la altura de las
  mayúsculas): la lista de fuentes lo muestra, y Propiedades del objeto
  avisa cuando el texto está fuera del rango. Con el programa vienen 55
  fuentes de bordado.
- Fuentes **TrueType** y OpenType del ordenador. El programa crea las
  letras a partir de sus contornos: satén para los trazos estrechos,
  relleno para los anchos (**Automático**), o todo **Satén** o todo
  **Relleno**.

**Letras > Fuentes** abre la lista de fuentes: busque, vea una vista
previa y el rango de tamaños, y muestre **Todo**, **Bordado**, **TrueType**
o **Para texto pequeño** (fuentes de bordado para letras desde 4 mm de
alto). **Usar para las letras seleccionadas** cambia la fuente de la
selección.

Diez de las fuentes de bordado son fuentes **afinadas**. Sus letras se
bordan con pocos cortes: la aguja pasa de un trazo al siguiente por debajo
de las puntadas. Cada una de sus letras se revisó. Las listas de fuentes
muestran primero las fuentes afinadas, y la lista de **Letras > Fuentes**
las marca.

Cada fuente de bordado tiene una **puntuación de calidad** de 0 a 100. La
puntuación dice lo bien que se bordan las letras de la fuente. La lista de
fuentes la muestra después del nombre de la fuente, y **Ordenar por >
Calidad** pone primero las fuentes con las mejores puntuaciones. Seleccione
una fuente para ver su puntuación y estas tres medidas:

- qué parte de las formas de las letras cubren las puntadas,
- qué parte de las columnas de satén es demasiado estrecha en el tamaño más
  pequeño de la fuente,
- qué parte de las columnas de satén tiene puntadas demasiado largas en el
  tamaño más grande de la fuente.

Cada fuente del programa tiene una puntuación de 65 o más. Una fuente que
usted convierte recibe su puntuación al convertirla. El rango de tamaños de
una fuente mantiene sus trazos finos lo bastante anchos en el tamaño más
pequeño; por eso una fuente con trazos finos y gruesos tiene un rango
pequeño.

**Más fuentes** abre la página del programa en internet. Los **paquetes de
fuentes** de allí tienen 68 fuentes de bordado más: fuentes sin serifas,
decorativas, con serifas, caligráficas y manuscritas. Descargue un paquete
y añádalo con **Archivo > Mis recursos > Importar un paquete**. El programa
no descarga nada por sí mismo.

Cuatro fuentes de versiones anteriores tienen una puntuación inferior a 65:
Slab Heavy, Fat Face Serif, Shrikhand y Great Vibes. Ya no están en el
programa. Un diseño que tiene una de ellas se queda como está. Para cambiar
su texto, añada el paquete de fuentes **Fonts of earlier versions**.

**Convertir en fuente de bordado** crea una fuente de bordado a partir de
una fuente TrueType, con un nombre, un rango de tamaños y un tipo de
puntada. Compruebe que la licencia de la fuente le permite cambiarla.

## Árabe, devanagari y ligaduras

Con una fuente TrueType u OpenType, el programa da forma al texto como lo
hace un procesador de textos:

- **Ligaduras y letras unidas.** Una fuente con ligaduras hace una sola
  letra de «fi» o «ff», y una fuente caligráfica usa las formas de sus
  letras que se unen.
- Las letras **árabes** se unen y tienen la forma que corresponde a su
  lugar en la palabra. El texto va de derecha a izquierda, también en una
  línea con palabras latinas o cifras, y se borda en el orden de lectura.
- Las letras **devanagari** forman conjuntos, y un signo vocálico que se
  escribe antes de su consonante va en ese lugar.

Dos fuentes para estos sistemas de escritura vienen con el programa:
**Tajawal** (árabe) y **Poppins** (devanagari). También puede usar una
fuente de su equipo. Si la fuente no tiene un carácter de su texto, la
ventana Letras muestra qué caracteres no tienen letra.

Una ligadura, o una letra con sus signos, es una sola letra del rótulo: se
mueve, se gira y se cambia como una. Las fuentes de bordado tienen una
letra para cada carácter y no dan forma al texto.

## Hoja de prueba de fuente

**Letras > Hoja de prueba de fuente** (o **Hoja de prueba** en la lista de
fuentes) crea un diseño nuevo con las letras de una fuente, para bordarlo y
verlo:

- **Menor**, **Medio** y **Mayor**: los tamaños del rango de tamaños de la
  fuente. Desactive un tamaño que no quiera, o escriba otra altura.
- **Caracteres**: las letras, cifras y signos, todos los caracteres de la
  fuente, una parte de ellos, o **Mi texto**.
- **Ancho de fila**: las filas de letras no son más anchas que esto.
  Empieza con el ancho del bastidor.

Cada tamaño tiene una etiqueta con el nombre de la fuente y el tamaño. La
ventana muestra el tamaño de la hoja. Si la hoja es más grande que el
bastidor, use menos tamaños o caracteres, o bórdela en más de una
colocación (vea [Diseños más grandes que el bastidor](hooping)).

## Los ajustes de las letras

Seleccione las letras. Propiedades del objeto tiene:

- **Texto**, **Fuente**, **Altura** (la altura de las mayúsculas).
- **Espaciado entre letras**: espacio extra después de cada letra.
  **Interlineado**: la distancia entre líneas, para texto de varias líneas.
- **Alinear**: **Izquierda**, **Centro** o **Derecha**: dónde empieza el
  texto en la línea base.
- **Línea base**: **Recto**, **Arco por encima**, **Arco por debajo**,
  **Círculo, en sentido horario**, **Círculo, en sentido antihorario** o
  **Línea**. Defina el **Radio** de un arco o un círculo.
- **Arte de letras** curva el texto de una línea base recta dentro de una
  forma (una envolvente): Puente, Valle, Arco por encima, Arco por debajo,
  Abombado, Pellizco, Banderín, Perspectiva, Rombo, Onda u Óvalo.
  **Cantidad** define cuánto se mueven los bordes.
- **Interletraje**: use los pares de letras de la fuente (por ejemplo
  «AV») para definir el espacio entre letras.
- **Orden de bordado**: **En el orden del texto**, **Desde el centro**
  (para gorras) o **Unión más cercana**. **Cortar huecos de más de** define
  qué huecos reciben un corte; los huecos más cortos reciben una puntada de
  desplazamiento. **Saltos, sin cortes** es para máquinas sin cortahílos.
- Los ajustes **Satén** y **Relleno** de las letras: densidad,
  compensación de tiro y base.
- **Esquinas en inglete** (en Satén): las columnas que se encuentran en
  una esquina, como en «L», «E» o «7», se bordan como una columna que gira.
  Desactivado, una columna termina y la siguiente columna empieza sobre
  ella. Una esquina aguda, como arriba de una «A», y un trazo que termina
  en otro trazo, como en «T», no cambian. Las puntadas quedan dentro de la
  forma de la letra: una esquina que no puede girar dentro de ella queda
  como un final y un comienzo.

## Negrita, cursiva y subrayado

Los botones de **Estilo** están en **Propiedades del objeto** de las
letras:

- **B** (Negrita) e **I** (Cursiva) usan la variante negrita o cursiva de
  la fuente cuando la fuente la tiene. Si no, el programa crea el estilo:
  la negrita hace más ancha cada columna de satén y más grande cada
  relleno, la cursiva inclina las letras.
- **U** (Subrayado) añade una barra de satén debajo de cada línea de
  texto. Ajuste la **Separación del subrayado** (desde la línea base) y el
  **Ancho del subrayado**. En un arco o un trazado, la barra sigue la
  línea.

## Texto apilado

**Dirección del texto** en **Propiedades del objeto** pone **Apiladas, de
arriba abajo** o **Apiladas, de abajo arriba**: cada letra queda derecha,
una debajo de otra, como en un letrero. Cada línea del texto es una
columna, y las columnas van de izquierda a derecha. **Alinear** pone el
principio, el centro o el final de cada columna en el punto de inicio. El
texto apilado no tiene arco, envolvente ni subrayado. Para girar 90° una
línea entera de texto, gire el texto.

## Letras sobre una línea

**Letras sobre línea** pone letras sobre una línea que usted dibuja:
seleccione las letras, haga clic en el comando, y después haga clic en la
línea. O seleccione primero las dos. La línea pasa a ser la línea base.
Con la herramienta Remodelar, arrastre los nodos de la línea para
cambiarla.

## Cambiar una letra

Haga clic en **Remodelar**, y después en una letra. Arrastre la letra, o
defina su **Interletraje**, **Mover a lo largo**, **Mover hacia abajo**,
**Tamaño** y **Girar** en Propiedades del objeto. Una letra de una fuente
TrueType puede tener su propio tipo de puntada y su propia base.
**Restablecer letra** y **Restablecer todas las letras** quitan los
cambios hechos a mano.

## Letras superpuestas

Cuando dos textos se superponen, seleccione uno o los dos y haga clic en
**Arte > Formas > Quitar superposiciones** (o seleccione los dos y haga
clic en **Aplanar**). El texto de detrás recibe un **recorte**: no se cose
donde lo cubren objetos que se cosen después. Las letras de satén siguen
siendo de satén.

El recorte sigue cada cambio: mueva, gire o escriba en un texto, añada un
texto delante o cambie el orden de bordado, y el corte cambia con él. En
**Propiedades del objeto > Recorte**, **Coser alrededor de los objetos de
delante** activa o desactiva el recorte de las letras seleccionadas.
**Superposición** indica cuánto entran las letras debajo de los objetos de
delante.

## Alfabetos

**Letras > Alfabetos** crea sus propios alfabetos de bordado:

1. Haga clic en **Alfabeto nuevo** y escriba un nombre.
2. Defina la **Altura de letra** y **Línea base en**, y después haga clic
   en **Añadir guías**.
3. Dibuje la letra entre las guías con las herramientas de digitalizar.
4. Seleccione sus objetos, escriba su **Carácter** y haga clic en **Crear
   letra**.

**Importar** lee una fuente de bordado (.embfont) o una fuente de mapa de
bits (.bdf, .fon). **Exportar** escribe el alfabeto. **Renombrar** y
**Eliminar** lo cambian.

## Corregir una letra de una fuente

Puede cambiar una letra de una fuente de bordado, también de una fuente que
viene con el programa. En **Letras > Alfabetos**, bajo **Correcciones de
letras**:

1. Seleccione la **Fuente** y escriba el **Carácter**.
2. Haga clic en **Colocar letra**. Los objetos de la letra están en el
   diseño, entre guías, en su orden de bordado.
3. Cambie los objetos: el orden (el panel Secuencia), la dirección
   (**Invertir dirección**), la forma, la base o la compensación de tiro.
4. Seleccione los objetos y haga clic en **Guardar corrección**.

Las letras nuevas usan la letra corregida. Las letras que ya están en un
diseño cambian cuando cambia su texto o su fuente. **Quitar corrección**
devuelve la letra de la fuente.

- **Mantener el orden de bordado**: la letra borda sus objetos en su orden,
  cada columna de su inicio a su final. Si está desactivado, el programa
  busca el orden.
- **Mantener los ajustes**: la letra conserva la base, la compensación de
  tiro y el ángulo de relleno de cada objeto.

Las correcciones de una fuente son un archivo en la carpeta `corrections`
de su carpeta de fuentes (**Mostrar archivo**). Una versión nueva del
programa las conserva.

## Monogramas

**Letras > Monograma** crea de una a tres iniciales en un estilo, con un
borde y adornos.

- **Iniciales**: por ejemplo «JMK». **Estilo**: Una letra, Bloque,
  Clásico (central grande), Apilado, Rombo u Óvalo. **Altura**: la altura de
  la inicial más grande.
- **Borde**: la **Forma** (círculo, óvalo, cuadrado, cuadrado redondeado,
  rombo, círculo doble), la **Puntada** (satén, puntada recta, recta
  triple), el **Ancho** y el **Espacio** hasta las iniciales.
- **Adornos**: punto, rombo, estrella, corazón, hoja o voluta, a la
  izquierda y a la derecha, arriba y abajo, o en los cuatro lados.
- **Plantillas** define todo esto a la vez. **Guardar como plantilla**
  conserva sus ajustes.

Con un monograma seleccionado, la ventana lo cambia.

## Nombres de equipo

**Letras > Nombres de equipo** crea un diseño con muchos nombres (y
números):

1. Seleccione un objeto de letras del diseño (por ejemplo el nombre), y
   haga clic en **Añadir las letras seleccionadas como columna**. Añada más
   columnas, por ejemplo un número.
2. Escriba los nombres, **Pegar una lista** (un nombre por línea) o
   **Importar CSV**. **Cant.** define cuántos de cada uno se bordan.
3. **Ajustar al ancho** hace más pequeño un nombre largo, para que tenga
   como mucho este ancho.
4. **Guardar cada nombre…** escribe un archivo de máquina para cada nombre,
   en una carpeta. **Todo en un diseño** pone todos los nombres en un
   diseño (una pestaña nueva), color por color, para una colocación.

## Diseños de teclado

**Letras > Diseños de teclado** escribe con una colección de diseños, uno
para cada tecla (por ejemplo una colección de imágenes pequeñas).
Seleccione la **Colección**, escriba el **Texto**, defina la **Altura** y
el **Espaciado**, y haga clic en **Añadir al diseño**. Para crear su
propia colección, seleccione objetos y haga clic en **Asignar la
selección a esta tecla**.

## El quiosco de letras

**Letras > Quiosco de letras** es una pantalla grande para una tienda: un
producto, un nombre, una fuente y un color de hilo, y después guardar,
producir o enviar a USB. Vea [Tutorial: un nombre en un
arco](tutorial-lettering).

## Consejos

- Por debajo de unos 5 mm, las letras pierden su forma. Use una fuente
  **Para texto pequeño**, o haga el texto más grande. El texto pequeño no
  necesita base.
- Las letras en relieve (puff) sobresalen de la tela: vea [Decoración
  especial](specialty).
- En las gorras, use el orden de bordado **Desde el centro**.
