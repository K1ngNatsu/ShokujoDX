---
id: whats-new
title: Novedades
group: start
keywords: nuevo, novedades, cambios, versión, actualización, noticias, más reciente, buscar actualizaciones, versión nueva, new, changes, release, upgrade
---
# Novedades

Este tema enumera los cambios de cada versión, la más reciente primero.
**Ayuda > Novedades** lo abre. Después de una actualización, la página
de inicio tiene un enlace a él.

## Versión 0.6.0

**Tutoriales y lecciones.** La ayuda tiene 31 tutoriales en siete rutas,
desde el primer diseño hasta la producción (**Ayuda > Todos los
tutoriales**). Cada tutorial tiene imágenes y un diseño inicial que un
enlace abre. **Empezar la lección** en la tarjeta de un tutorial lo
convierte en una lección dentro del programa: la ventana **Lección**
muestra un paso cada vez, marca el botón o el lugar del lienzo en el que
debe hacer clic y comprueba el diseño antes de continuar. La ventana
tiene **◀ Atrás**, **Omitir ▶** y **Detener la lección**, y una lección que se
detiene conserva su lugar.

**Comprobaciones antes de bordar.** **Ver > Comprobar > Desplazamiento
visible** marca las puntadas de desplazamiento que ninguna puntada
posterior cubre, e Información del diseño dice cuánto desplazamiento se
ve. **Organizar > Orden de bordado > Bordar más rápido** une los hilos
que se ven iguales, quita los objetos demasiado pequeños para bordar,
pone los cambios de color en un orden mejor y fija la distancia de
corte del diseño, en un solo paso de deshacer.

**Muestras de prueba.** **Diseño > Producción > Muestras de prueba** hace
copias de los objetos seleccionados con un ajuste en pasos (por ejemplo
la densidad), cada una con una etiqueta de su valor. Borde la hoja en su
tela y luego use el ajuste de la mejor copia.

**Inclinación del satén y bandas.** **Inclinación** en Propiedades del
objeto inclina las puntadas superiores de una columna de satén. **Editar
> Tipos de puntada > Bandas** borda una columna como bandas de hilos una
al lado de otra, cada una en su color.

**Archivos de máquina.** **Editar > Funciones de máquina > Reparar
puntadas…** comprueba las puntadas de un archivo de máquina que usted
abrió y repara los fallos que puede, por ejemplo los cambios de color al
mismo hilo y las puntadas más cortas que la puntada más corta de la
máquina.

**Sus diseños abiertos vuelven.** Al salir, el programa conserva los
diseños abiertos con sus cambios sin guardar y los abre de nuevo en el
siguiente inicio (**Pestañas abiertas** en las Opciones, activado al
principio). Un archivo que usted abre mientras el programa funciona se
abre en una pestaña de ese programa; `--new-window` inicia un segundo
programa.

**Su propio agente de IA.** `emb-cli mcp` permite que su propio agente de
IA haga, compruebe, exporte y muestre diseños (tema de ayuda **Agentes
(MCP)**). Cuando otro programa guarda un archivo que está abierto en una
pestaña, el programa pregunta: **Abrir de nuevo** o **Conservar esta
versión**. El programa no tiene IA ni código de red.

**Mejores puntadas.** Menos hilo donde se unen las partes de una letra:
el relleno base de una columna se detiene donde otra parte de la letra
lo cubre. Los monogramas grandes acumulan menos hilo, porque cada letra
elige columnas o rellenos según su tamaño en el diseño. Las columnas
cuyas puntadas van a lo largo de sus lados bordan con su espaciado. El
desplazamiento en un relleno tatami va a lo largo de sus filas, y en una
letra a lo largo de los bordes de sus partes, así que se ve menos
desplazamiento.

**Paneles y campos.** Los paneles laterales son dos grupos, uno sobre el
otro, así que todos sus títulos se ven en una pantalla pequeña; solo la
pestaña abierta de un grupo tiene su ×. Un campo numérico acepta una coma
decimal (`2,5`); en inglés y en japonés, `5,000` es cinco mil. Un campo
también lee dígitos de ancho completo. **Editar nodos** está en el menú
de un objeto y en Propiedades del objeto.

**Nombres en su idioma.** Los nombres de los motivos, patrones de
división, sellos, bordes, elementos de clipart y familias integrados se
muestran en el idioma del programa, y las búsquedas encuentran los dos
nombres. **Editar > Tipos de puntada > Familias** abre la ventana
Familias.

**Más rápido y más pequeño.** Los órdenes de bordado de las letras de una
fuente de su computadora se quedan en el disco, así que la fuente es
rápida de nuevo en el siguiente inicio. **Reconocer objetos** es mucho
más rápido con archivos grandes. El programa es unos 7 MB más pequeño:
las imágenes de la ayuda tienen los mismos píxeles en archivos más
pequeños.

## Versión 0.5.0

**Letras que mantienen la forma de la fuente.** El programa crea mejores
letras a partir de las fuentes TrueType y OpenType, y las fuentes de
bordado se han creado de nuevo con ello. Las puntadas de una columna se
quedan dentro del contorno de la letra, los extremos de las columnas
llegan a las esquinas de los trazos, y las puntadas que cruzan una
columna no se cruzan entre sí. Donde dos trazos se juntan en una
esquina, uno sigue hasta la esquina exterior y el otro termina recto en
su lado. Una columna que termina en otra columna se borda antes que
ella. Los paquetes de fuentes de esta versión también tienen las nuevas
letras: impórtelos de nuevo.

**Tres fuentes vuelven.** El programa tiene 68 fuentes de bordado. Slab
Heavy, Fat Face Serif y Shrikhand están de nuevo en el programa: con las
mejores letras, su puntuación de calidad es 87, 91 y 95. Great Vibes
sigue en el paquete de fuentes **Fonts of earlier versions**. Fat Face
Serif tiene trazos muy finos, por eso su tamaño más pequeño es 42,5 mm.
Un diseño que tiene una de las tres fuentes la encuentra de nuevo en el
programa.

**Órdenes de bordado más rápidos.** Las letras en una fuente TrueType u
OpenType de su ordenador reciben el orden de bordado unas cuatro veces
más rápido: las letras y los dígitos de una fuente con serifas como
Georgia en 8 s, antes 33 s. Los órdenes de bordado son los mismos que
antes.

**Paneles y cinta de opciones.** Los paneles laterales empiezan como un
grupo a la derecha, y un panel cerrado se abre de nuevo en ese grupo.
**Restablecer los paneles** (en las Opciones y en **Ver > Paneles**)
pone los paneles donde empiezan. En una ventana estrecha, las listas de
un panel plegado de la cinta (por ejemplo la máquina y el bastidor en la
pestaña Diseño) se abren de nuevo.

**Guías desde las reglas.** Haga clic derecho en una regla (en una
pantalla táctil, manténgala pulsada) para su menú: **Añadir guía en**
ese lugar, las herramientas **Línea de construcción** y **Punto de
construcción**, y **Quitar guías**.

**Pantallas táctiles.** Una barra bajo el lienzo tiene las teclas de las
herramientas de dibujo: **Siguiente** o **Terminar**, **Quitar punto**,
**Cancelar** y **Hecho**, que vuelve a Seleccionar. Así puede usar cada
herramienta sin teclado.

**Otros cambios.** **Archivo > Buscar actualizaciones…** también está
allí. Las letras nuevas vuelven a la herramienta Seleccionar.
**Reconocer objetos** encuentra una columna de satén densa con puntadas
cortas como columna, no como pespunte.

## Versión 0.4.0

**La vista de producto.** **Diseño > Producto** muestra el diseño sobre
un producto que parece tela.

- Los productos tienen luz, pliegues y la textura de su tela, y el
  bordado tiene una sombra suave.
- Productos nuevos: una gorra de seis paneles, una camiseta de manga
  larga, una sudadera con capucha, un delantal, un gorro y una bandolera.
- Una prenda tiene un frente y una espalda. Puede poner más diseños en un
  producto, cada uno en su lugar.
- Productos de dos tonos: un segundo color para el cuello, los puños y
  otras partes de ribete, y una lista de colores de prenda.
- Sus propias fotos de producto: marque la tela de la foto, y la foto
  muestra el producto en cada color. Marque los lugares para un diseño, y
  guarde la foto en **Mis productos**.
- **Diseño > Plato giratorio** gira el producto por pasos. Puede guardar
  el giro como imagen animada o como cuatro vistas.
- La hoja de aprobación puede mostrar el diseño sobre el producto.

**Letras.** **Letras > Hoja de prueba de fuente** crea un diseño con las
letras de una fuente en varios tamaños, para bordarlo y verlo. En **Letras >
Alfabetos** puede corregir una letra de una fuente de bordado: su orden de
bordado, su forma y sus ajustes. Consulte [Letras](lettering).

**Letras con pocos cortes.** Las letras de todas las fuentes de bordado
del programa se hicieron de nuevo. El programa encuentra los trazos de una
letra como los escribe una pluma: un trazo sigue a través de una unión, y
termina en una esquina aguda. Las letras se bordan con pocos cortes (la
aguja pasa de un trazo al siguiente por debajo de las puntadas), y sus
esquinas y puntos tienen puntadas hasta el borde. Diez fuentes están
**afinadas**, y cada una de sus letras se revisó: Block Sans, Friendly
Serif, Pacifico, Atkinson Hyperlegible, Varsity, Bebas Neue, Soft Round,
Geo Slab, Brush Hand y Vintage Script. Las listas de fuentes las muestran
primero. Las letras que ya están en un diseño cambian cuando cambia su
texto o su fuente.

**Más sistemas de escritura.** Con una fuente TrueType u OpenType, las
letras ahora dan forma a su texto: ligaduras, las letras unidas de las
fuentes caligráficas, árabe (de derecha a izquierda) y devanagari. Las
fuentes Tajawal (árabe) y Poppins (devanagari) vienen con el programa.
Consulte [Letras](lettering).

**Puntadas que siguen los trazos.** Con una fuente TrueType u OpenType y
el tipo de puntada **Automático**, los trazos de las letras son columnas:
satén, con puntadas divididas en los trazos de más de 7 mm de ancho, o
filas de tatami si los trazos del texto tienen más de 12 mm. Todas las
letras de un texto tienen la misma puntada. Antes, una letra con un trazo
ancho era toda de relleno: en una fuente negrita, la mayoría de los kanji
y kana de 50 mm eran de relleno. Donde se cruzan dos trazos, uno pasa y el
otro termina en sus lados; donde un trazo termina en otro, termina un poco
dentro de él: no hay bultos de hilo allí. Las letras de estas fuentes
también tienen un orden de bordado con pocos cortes, como las fuentes de
bordado del programa: 刺繍東京 a 20 mm tiene 23 cortes, no 48. La altura
de un texto japonés, chino o coreano es ahora la altura de sus caracteres
(antes era la altura de una mayúscula latina, así que los caracteres eran
alrededor de una cuarta parte más grandes); las letras de los diseños que
guardó antes mantienen su tamaño. Los trazos de menos de 0,8 mm se bordan
como puntada recta, y la ventana Letras muestra la altura más pequeña a la
que el texto se borda bien. Las fuentes con trazos que se superponen
(muchas fuentes de internet) dan ahora letras sin agujeros y sin puntadas
fuera de la letra. Consulte [Letras](lettering).

**Fuentes de bordado japonesas.** Un nuevo paquete de fuentes tiene dos
fuentes de bordado japonesas, Noto Sans JP (una gótica) y Zen Maru Gothic
(una gótica redondeada): letras latinas, los kana, los 2136 kanji jōyō y la
puntuación japonesa, cada letra con el orden de bordado de las fuentes del
programa. Por encima del tamaño más grande de sus columnas de satén, los
trazos se bordan como columnas de tatami, no como rellenos.

**Más fuentes, con una puntuación de calidad.** El programa tiene 65
fuentes de bordado (antes 30), y los paquetes de fuentes de la página del
programa tienen 70 más. Cada fuente de bordado tiene una puntuación de
calidad de 0 a 100: **Letras > Fuentes** la muestra y puede ordenar por
ella. Los rangos de tamaños de algunas fuentes son ahora más pequeños, para
que sus trazos finos no sean demasiado estrechos en el tamaño más pequeño.
Cuatro fuentes con una puntuación baja (Slab Heavy, Fat Face Serif,
Shrikhand y Great Vibes) están ahora en el paquete de fuentes **Fonts of
earlier versions**. Un diseño que tiene una de ellas se queda como está.

**Fuentes de relleno.** Diez de las fuentes de bordado tienen letras que
se bordan como rellenos: Block Sans Fill, Varsity Fill, Anton Fill, Archivo
Black Fill, Slab Heavy Fill, Poster Heavy Fill, Giant Round Fill, Funky
Slab Fill, Rubik Mono One Fill y Shrikhand Fill. Son para texto grande, de
15 mm a 300 mm, y para letras gruesas cuyas formas no tienen columnas
claras. En **Letras > Fuentes**, **Para texto grande** las muestra.
Consulte [Letras](lettering).

**Una puntuación de calidad más estricta.** La puntuación de calidad de
una fuente de bordado ahora también cuenta las puntadas fuera de las
formas de las letras y los lugares con dos capas de puntadas. Algunas
fuentes tienen una puntuación unos puntos más baja. Ninguna letra de las
fuentes del programa ha cambiado. Cuando convierte una fuente con el tipo
de puntada **Automático**, una letra cuyas columnas dejan abierta una
parte de su forma pasa a ser un relleno, si la fuente es para letras
grandes (desde 15 mm).
Consulte [Letras](lettering).

**Letras grandes en cada fuente.** Cada fuente de bordado con letras de
satén tiene ahora también sus letras como rellenos. Por encima del tamaño
más grande de sus columnas de satén, una fuente borda letras de relleno,
y así llega a 300 mm. **Tipo de puntada** en la ventana Letras selecciona
satén o relleno para todo el texto.
Consulte [Letras](lettering).

**Tipos y grosores de hilo.** Un hilo tiene un tipo (poliéster, rayón,
algodón, metálico, lana o acrílico, brilla en la oscuridad) y un grosor
(40 es lo habitual). Ajústelos para el hilo actual en el panel Hilos. El
aspecto de hilo muestra un hilo de otro grosor más fino o más grueso, y un
hilo mate sin brillo. Un hilo de una carta da su tipo cuando el nombre de
la carta lo indica. En las imágenes del diseño, los colores de los bordes
de los hilos se mezclan ahora como la luz. Consulte [Colores e hilos](colors-and-threads).

**Un nuevo aspecto de hilo 3D.** El aspecto de hilo 3D del lienzo muestra
la superficie de las puntadas: el hilo sobre hilo es más alto, una zona es
más clara o más oscura según la dirección de sus puntadas respecto a la
luz, los hilos brillan según su tipo y las partes altas proyectan sombras
sobre la tela. Los diseños grandes también se mueven y se amplían más
rápido. **Ver > Aspecto de hilo > Calidad > Rápida** lo desactiva en un
equipo lento. Consulte [Comprobar un diseño](checking).

**La tela en el aspecto 3D.** La tela bajo las puntadas tiene una
superficie a su tamaño real: los hilos de un tafetán o de una sarga, las
mallas de un tejido de punto, las celdas del piqué, los rizos de una
toalla, el pelo del forro polar, el fieltro y el grano del cuero, con la
luz del aspecto de hilo. Cada agujero de aguja es una pequeña hendidura,
mayor en el cuero. En una toalla o un forro polar sin recubrimiento, las
puntadas finas se hunden en el pelo: la casilla **Recubrimiento** del
panel Estilos lo define. Una tela puede tener un **Segundo color** (los
hilos claros de un denim, la segunda fibra de una tela jaspeada). La
vista de producto muestra la misma tela. Consulte [Telas, estilos y
plantillas](fabrics-and-styles).

**Lo que no es hilo en el aspecto 3D.** La tela de aplique tiene la
superficie de su trama (**Superficie de la tela** en la ventana de
aplique) y un borde cortado, los diamantes de imitación tienen facetas
que reciben la luz, los agujeros del calado muestran lo que hay detrás de
la tela, el encaje no tiene tela, un relleno trapunto es más alto sobre
su relleno y, en la nueva tela **Quilt**, la guata se abomba entre las
líneas de puntadas. El punto de cruz tiene la nueva tela **Aida cloth**.
Las columnas de puff son redondas sobre su espuma. El hilo metálico tiene
pequeños destellos con mucho zoom, y **Ver > Aspecto de hilo > Luces
apagadas** solo muestra el hilo fosforescente. Consulte [Comprobar un
diseño](checking).

**Objetos de archivos de máquina.** **Arte > Convertir > Reconocer
objetos** convierte las puntadas de un archivo de máquina en objetos de
puntada recta, columnas de satén y rellenos, con los ajustes que
muestran las puntadas. Una ventana muestra el archivo y los objetos uno
al lado del otro, con los lugares que difieren; puede mantener objetos
sueltos como puntadas. Las opciones pueden hacerlo cada vez que se abre
un archivo de máquina. Con los contornos del mismo diseño (un archivo
vectorial), cada contorno pasa a ser un objeto con el tipo y los ajustes
de las puntadas que hay dentro. Consulte [Gráficos vectoriales y
formas](artwork).

**Archivos de Ink/Stitch.** **Archivo > Abrir** abre los archivos SVG de
Ink/Stitch como diseños: rellenos, columnas de satén y líneas con sus
ajustes, los comandos de corte, parada y puntos de inicio, y los colores
de hilo. Un ajuste que el programa no lee aparece en una nota. **Diseño >
Exportar para Ink/Stitch** escribe un diseño como un archivo así, y
**Letras > Fuentes > Fuente de Ink/Stitch…** convierte las fuentes de
letras de Ink/Stitch. Vea [Formatos de archivo](file-formats).

**Archivos EPS.** **Arte > Importar** lee archivos EPS y PostScript, y
archivos AI del tipo antiguo, sin otro programa. El texto y las imágenes
de un archivo así no se importan. Un archivo CDR da un mensaje: guárdelo
como SVG, PDF o EPS en el programa que lo creó. Consulte [Gráficos
vectoriales y formas](artwork).

**Partes de puntadas.** **Ver > Partes de puntadas** colorea las
puntadas de un diseño según el tipo de su parte: rectas, columnas y las
filas de los rellenos, con la base de cada parte. El programa las
encuentra en las puntadas de un archivo de máquina de otro programa: el
primer paso para hacer objetos con tipos de puntada de un archivo así.
Las líneas muestran los objetos de puntada recta, las columnas de satén y
los rellenos (con sus ajustes de base) que salen de las rectas, de las
columnas y de las filas.
Consulte [Comprobar un diseño](checking).

**Todas las imágenes con el aspecto 3D.** La imagen del diseño, la hoja
de aprobación y el presupuesto, la biblioteca, las miniaturas del
explorador de archivos, el quiosco, las imágenes de puntadas de Imagen a
diseño y la vista previa de la ventana Fuentes muestran las puntadas como
la vista 3D. **Producir diseño** puede escribir la imagen a hasta 600
puntos por pulgada con **Sin fondo** (para una tienda web) y una
**Animación del bordado (GIF)**. La **Calidad** del menú del aspecto de
hilo (**Rápida**, **Buena**, **Mejor**) se elige en el primer inicio con
una medida corta del equipo. Consulte [La vista 3D](3d-view).

**Comprobar el aspecto.** El grupo **Comprobar** de la pestaña Ver tiene
herramientas nuevas: luces predefinidas y una luz que se mueve (mantenga
pulsada **L** y arrastre), una **Vista inclinada** que muestra el
bordado en ángulo con sus alturas (en el lienzo o en una ventana), una
**Lupa** a 20 veces, **Comparar 3D** y **Antes y después**, y dos
comprobaciones de la superficie: **Se ve la tela** e **Hilo alto**. El
producto y el plato giratorio muestran las puntadas con sus alturas y su
brillo, con la luz del producto. Consulte [Comprobar un
diseño](checking).

**Diseños con muchísimas puntadas.** El programa podía cerrarse cuando el
lienzo mostraba unas 350 000 puntadas o más a la vez con el aspecto de
hilo. Ahora el lienzo las muestra con un aspecto más simple con ese zoom,
y con el aspecto completo cuando usted acerca el zoom.

**Más motivos, patrones, sellos, bordes y clipart.** El programa tiene
71 motivos (antes 15), 48 patrones de división programada (10), 39 sellos
de relieve (8), 50 bordes decorativos (12) y 45 elementos de clipart (8).
La mayoría vienen de **familias**: una familia crea un elemento a partir de
unos números, por ejemplo el número y la altura de las ondas.
**Familias…** en la ventana Motivos, en Propiedades del objeto y en el
panel Clipart crea un elemento propio. Un borde puede tener otra forma
(también un rombo, un hexágono y un octágono) y otro tamaño de sus
puntadas. Consulte [Tipos de puntada](stitch-types).

**Versiones nuevas.** El programa puede avisarle cuando hay una versión
nueva disponible. Consulte [Opciones](options).

**El reproductor de puntadas en el aspecto 3D.** Con el aspecto de hilo
3D, el reproductor de puntadas muestra las puntadas nuevas en el aspecto
3D justo detrás de la aguja, sin un destello del aspecto plano, y la vista
sigue rápida después de arrastrar el control deslizante del reproductor.

**Un icono nuevo.** El icono del programa es una estrella morada en un
bastidor, con una aguja y un hilo.

## Versión 0.3.0

- El primer inicio pregunta por su máquina y su bastidor. La biblioteca
  de máquinas tiene unas treinta máquinas, y **Diseño > Mis máquinas**
  guarda las suyas.
- Las opciones de Vista se conservan para el siguiente inicio.
- Desplazamiento sin fin: un desplazamiento con el ratón no se detiene en
  el borde del lienzo.
- Las letras tienen negrita, cursiva, subrayado y texto apilado.
- El modo de boceto tiene un origen fijo, y un boceto y sus objetos se
  mueven juntos.
- El instalador de Windows quita primero la versión anterior.
