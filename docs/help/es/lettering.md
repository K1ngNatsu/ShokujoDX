---
id: lettering
title: Letras y monogramas
group: design
keywords: letras, texto, nombre, fuente, recorte, superposición, fuente de bordado, truetype, opentype, rango de tamaños, texto pequeño, altura, espaciado entre letras, interlineado, interletraje, alinear, línea base, arco, círculo, línea, envolvente, arte de letras, orden de bordado, cortes, saltos, letra, remodelar, alfabeto, fuente de mapa de bits, monograma, iniciales, nombres de equipo, lista de nombres, csv, diseños de teclado, quiosco, tienda, inglete, esquinas en inglete, esquina
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
  avisa cuando el texto está fuera del rango. Con el programa vienen unas
  treinta fuentes de bordado.
- Fuentes **TrueType** y OpenType del ordenador. El programa crea las
  letras a partir de sus contornos: satén para los trazos estrechos,
  relleno para los anchos (**Automático**), o todo **Satén** o todo
  **Relleno**.

**Letras > Fuentes** abre la lista de fuentes: busque, vea una vista
previa y el rango de tamaños, y muestre **Todo**, **Bordado**, **TrueType**
o **Para texto pequeño** (fuentes de bordado para letras desde 4 mm de
alto). **Usar para las letras seleccionadas** cambia la fuente de la
selección.

**Convertir en fuente de bordado** crea una fuente de bordado a partir de
una fuente TrueType, con un nombre, un rango de tamaños y un tipo de
puntada. Compruebe que la licencia de la fuente le permite cambiarla.

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
