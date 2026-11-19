---
id: checking
title: Comprobar un diseño
group: production
keywords: comprobar, vista previa, reproductor de puntadas, reproducir, bordado, simular, cursor de puntada, desplazamiento, lista de puntadas, funciones, información del diseño, advertencias, mapa de densidad, capas, acumulación, aspecto de hilo, 3d, luz, penetraciones de aguja, saltos, cortes, producto, camiseta, maqueta, repeticiones, continuo, aprobación, tiempo de bordado, número de puntadas
---
# Comprobar un diseño

Compruebe cada diseño antes de bordarlo. El programa muestra cómo lo borda
la máquina, y le avisa de los fallos comunes.

## El reproductor de puntadas

**Ver > Reproductor de puntadas** muestra una barra debajo del lienzo.
Reproduce el bordado: las puntadas aparecen como las borda la máquina.

- **Reproducir el bordado** y **Pausa**, y la velocidad.
- **Al inicio**, **Atrás un color**, **Adelante un color**, **Al final**.
- Arrastre la barra para moverse por el bordado.

## El cursor de puntada

**Ver > Cursor de puntada** marca una puntada. Las puntadas siguientes se
ven más claras. Muévalo con las teclas:

- **,** y **.**: una puntada atrás o adelante (**Mayús**: 10, **Ctrl**:
  100, **Ctrl+Mayús**: 1000).
- **Re Pág**, **Av Pág**: el objeto anterior o el siguiente. **Ctrl**: el
  color anterior o el siguiente. **Mayús**: la función de máquina anterior
  o la siguiente. **Alt**: el segmento anterior o el siguiente.
- **Funciones** (Ver > Cursor de puntada) define el tipo de función al que
  van Mayús+Re Pág y Mayús+Av Pág: cualquier función, cortes, cambios de
  color, paradas, saltos o cambios de velocidad.
- **Centrar en la puntada** mueve la vista al cursor.

Con el cursor en un objeto, **Editar > Seleccionar > En el cursor de
puntada** lo selecciona, **Editar > Dividir en la aguja** lo corta allí, y
**Pegado especial** puede pegar en el cursor.

## La lista de puntadas

**Ver > Lista de puntadas** muestra el panel Lista de puntadas: cada
puntada, salto, corte, parada y cambio de color, con su lugar, su longitud
y su color. Haga clic en una fila para mover allí el cursor de puntada.
**Solo funciones** muestra solo los saltos, cortes, cambios de color y
paradas.

## Información del diseño

El panel **Información del diseño** muestra los números del diseño:

- el tamaño, los bastidores en los que cabe, el número de objetos,
  puntadas, colores, cambios de color, cortes, saltos y paradas;
- el tiempo de bordado a la velocidad de la máquina;
- la tela, el hilo superior y el hilo de bobina, y el costo del hilo;
- la densidad (el mayor número de capas de hilo);
- los colores en orden de bordado, con el hilo de cada uno.

Debajo de los números están las **Advertencias**. Haga clic en una
advertencia para seleccionar el objeto. El programa avisa de:

- puntadas fuera del campo del bastidor, o fuera del margen de seguridad;
- puntadas más cortas que la puntada más corta (el hilo se rompe);
- muchos cortes en un objeto (pruebe otro punto de inicio o final);
- una densidad demasiado alta;
- hilo que se acumula en un lugar (más capas que el límite);
- letras más pequeñas o más grandes que el rango de tamaños de su fuente;
- en un bastidor de gorra, un orden de bordado que no va del centro hacia
  fuera;
- en encaje independiente, una parte que no toca ninguna otra parte;
- objetos que no hacen puntadas.

## El mapa de densidad

**Ver > Mapa de densidad** colorea las puntadas según las capas de hilo:
rojo donde objetos superpuestos acumulan más capas que el límite
(**Opciones > Límite de densidad**, 5 capas al principio). La información
del diseño lista esos lugares. Quite las partes ocultas con **Arte >
Formas > Aplanar**, o haga la densidad más abierta.

## El aspecto de hilo

**Ver > Aspecto de hilo** muestra las puntadas como hilos, como se ven en
la tela. Su menú tiene **Plano** o **3D (luz y sombra)**, la dirección y
la altura de la luz, la fuerza de la sombra y la **Textura de la tela** (la
superficie de la tela del diseño en el color de la tela: los hilos de su
tejido, los rizos de una toalla o el grano del cuero, con la luz del
aspecto de hilo). **Luz
predeterminada** los restablece.

Con **Superficie fina** (activada al inicio), el aspecto 3D muestra la
superficie de las puntadas: el hilo sobre hilo es más alto, cada zona es
más clara o más oscura según la dirección de sus puntadas respecto a la
luz, y las partes altas proyectan sombras sobre la tela. Cada agujero de aguja es una pequeña
hendidura (mayor en el cuero). En una toalla o un forro polar sin
recubrimiento, las puntadas finas se hunden en el pelo (consulte [Telas,
estilos y plantillas](fabrics-and-styles)). El programa hace
la superficie por partes mientras trabaja; una parte que aún no está
lista muestra un momento el aspecto más rápido. Desactive **Superficie
fina** en un equipo lento.

## Otras vistas de las puntadas

**Ver > Mostrar** tiene **Saltos** (saltos y líneas de desplazamiento),
**Penetraciones de aguja** (un punto en cada penetración de la aguja) y
**Cortes y colores** (un símbolo en cada corte y cambio de color).

## El producto

**Diseño > Producto** muestra el diseño a su tamaño real en una camiseta
(de manga corta o larga), un polo, una sudadera, una sudadera con
capucha, una gorra, un gorro, un delantal, una bolsa, una toalla o un
cojín, o en
una imagen de su propio producto (**Mi imagen…**, a su ancho real). Defina
el **Color** y dónde va el diseño (**Poner el diseño en**). **Mostrar
producto** lo muestra u oculta. **Imagen en el producto** guarda una
imagen PNG del diseño con el aspecto de hilo sobre el producto, para un
cliente. El producto no se borda. Un producto dibujado tiene luz y
sombra, pliegues y la textura de su tejido, con la luz del aspecto de
hilo; en la imagen, el bordado tiene una sombra suave sobre él; en una
gorra, la imagen muestra el diseño un poco curvado alrededor de la copa.
**Añadir un diseño…** muestra un diseño más (un proyecto o un archivo
de máquina) en el producto, por ejemplo un nombre en una manga con este
diseño en el pecho izquierdo: defina su lugar en la lista **Otros
diseños en el producto**. Se muestra en el lienzo y en la imagen, y no
se borda. Una prenda tiene delantera y espalda: **Lado** en la ventana
Producto (o **Girar producto**, Ctrl+Mayús+T) muestra el otro lado, y
el diseño va al mismo lugar en él. Cada otro diseño está en un lado.
Con **Ambos lados en la imagen**, la imagen guardada muestra la
delantera y la espalda una al lado de la otra. Las muestras bajo
**Color** son los colores habituales de las prendas lisas. Con **Color
del ribete**, las partes de ribete del producto (por ejemplo el cuello
de un polo, los paneles delanteros o la visera de una gorra, los puños
de una sudadera) reciben un segundo color; desmarque una parte en
**Partes del ribete** para darle el color del producto.

Para una foto de su propio producto, **Editar la foto…** abre la ventana
**Foto del producto**. **Elegir color** y un clic en el producto marcan la
zona de ese color (también las costuras y puntadas en ella; **Tolerancia**
define lo cerca que debe estar un color); **Quitar color**, **Pincel** y
**Borrador** cambian la máscara. La parte marcada toma el color del
producto con la luz y la sombra de la foto, así una foto de un producto
blanco muestra el producto en cada color. **Marcar un lugar** y un clic
marcan un lugar habitual para un diseño; dele un nombre y el tamaño de
su diseño más grande. **Guardar en Mis productos** guarda la foto con su
ancho real, su máscara y sus lugares; elíjala de nuevo en **Mis
productos**. **Exportar…** e **Importar…** llevan un producto a otro
ordenador.

**Diseño > Plato giratorio** muestra un producto dibujado con sus diseños
desde más lados: gira en pasos de 15°. Use las flechas o el control
deslizante, arrastre la imagen o haga clic en **Reproducir**. El programa
crea las 24 vistas en segundo plano después de cada cambio, así un giro
es rápido también en un ordenador lento. El producto tiene una forma
suave y redonda, hecha a partir de su dibujo; el lienzo queda plano.
**Guardar GIF…** guarda el giro como imagen animada, y **Guardar cuatro
vistas…** guarda la parte delantera, dos cuartos de giro y la espalda una
al lado de la otra. Una foto de un producto no puede girar.

## Repeticiones

**Diseño > Repeticiones** muestra el diseño otra vez en **Columnas** y
**Filas**, para comprobar la unión de un borde que continúa (un diseño
continuo) o de diseños que se superponen.

- **Paso a la derecha** y **Paso hacia abajo** definen el lugar de las
  copias. Un paso también hacia abajo da una fila inclinada; un paso
  también hacia el lado da un medio salto. **Lado a lado** define pasos
  del tamaño del diseño.
- **Unir el final con el inicio** define un inicio automático a la
  izquierda y un fin automático a la derecha del diseño, para que la aguja
  vaya de copia en copia.
- **Mostrar repeticiones** las muestra u oculta. Las copias no se bordan
  hasta que haga clic en **Crear repeticiones**: entonces son objetos,
  después del diseño.

## Vista previa de impresión

**Diseño > Vista previa de impresión** muestra la hoja de trabajo de
producción y la hoja de aprobación. Vea [Archivos y salida](files-and-output).
