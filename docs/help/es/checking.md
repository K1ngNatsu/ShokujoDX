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
la altura de la luz, la fuerza de la sombra y la **Textura de la tela** (el
tejido de la tela del diseño bajo el color de la tela). **Luz
predeterminada** los restablece.

## Otras vistas de las puntadas

**Ver > Mostrar** tiene **Saltos** (saltos y líneas de desplazamiento),
**Penetraciones de aguja** (un punto en cada penetración de la aguja) y
**Cortes y colores** (un símbolo en cada corte y cambio de color).

## El producto

**Diseño > Producto** muestra el diseño a su tamaño real en una camiseta,
un polo, una sudadera, una gorra, una bolsa, una toalla o un cojín, o en
una imagen de su propio producto (**Mi imagen…**, a su ancho real). Defina
el **Color** y dónde va el diseño (**Poner el diseño en**). **Mostrar
producto** lo muestra u oculta. **Imagen en el producto** guarda una
imagen PNG del diseño con el aspecto de hilo sobre el producto, para un
cliente. El producto no se borda.

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
