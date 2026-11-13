---
id: objects-and-stitches
title: Objetos y puntadas
group: design
keywords: objeto, puntada, recta, satén, relleno, tatami, aplique, letras, vector, objeto de puntadas, densidad, espaciado, base, ligera, fuerte, doble, compensación de tiro, longitud de puntada, dividir, puntadas cortas, líquido, omitir la última puntada, efectos, acordeón, puntada a mano, cordón, esquinas, inglete, salto automático, espaciado automático, borde dentado, trapunto, propiedades, posición, tamaño
---
# Objetos y puntadas

Un diseño es una lista de objetos. El programa crea las puntadas de cada
objeto a partir de su forma y sus ajustes. Cuando cambia la forma o un
ajuste, las puntadas cambian al momento.

## Tipos de objetos

- **Líneas**: puntadas rectas a lo largo de una línea (sencilla, triple, o
  una columna de un tipo de puntada a lo largo de la línea).
- **Columnas**: puntadas de satén a través de una columna, de un lado al
  otro. Buenas para bordes y letras de hasta unos 10 mm de ancho.
- **Rellenos**: filas de puntadas que cubren un área (tatami y otros tipos
  de relleno).
- **Aplique**: una línea de colocación, una parada, una fijación, una
  parada y un satén de cubierta. Un objeto de aplique con más piezas puede
  tener hasta cuatro capas de tela (Propiedades del objeto > Capas): cada
  capa se borda por turno, y las cubiertas van al final. Vea [Tutorial: un
  aplique](tutorial-applique).
- **Letras**: texto en una fuente. Cada letra tiene partes de satén y de
  relleno. Vea [Letras y monogramas](lettering).
- **Vectores**: gráficos sin puntadas. **Convertir** crea bordado a partir
  de ellos. Vea [Gráficos vectoriales y formas](artwork).
- **Objetos de puntadas**: puntadas de un archivo de máquina o de una
  puntada fotográfica. El programa las borda tal cual. Remodelar mueve,
  añade y borra puntos de aguja sueltos. **Reconocer contornos** (Arte >
  Convertir) los convierte en rellenos que puede escalar.
- **Punto de cruz** y **strass**: vea [Decoración especial](specialty).

## Propiedades del objeto

Seleccione un objeto para ver sus ajustes en el panel **Propiedades del
objeto**. Seleccione más objetos de un tipo para cambiarlos todos. Cada
ajuste tiene al lado una palabra que dice qué significa el valor, por
ejemplo «denso» o «largo». Ponga el puntero sobre el nombre de un ajuste
para ver qué hace.

- **Hilo**: el hilo del objeto.
- **Posición y tamaño**: el lugar, el ancho y el alto, con **Mantener
  proporciones**; **Girar**, **Espejo** y **Centrar en el bastidor**.
- **Funciones de máquina**: vea [Cambiar objetos](editing).

## Los ajustes de las puntadas

- **Densidad** (espaciado): la distancia entre filas o puntadas de satén.
  Más pequeña es más densa. 0,4 mm es lo normal. Demasiado densa vuelve
  rígida la tela y puede romper agujas.
- **Longitud de puntada**: la longitud de las puntadas de relleno y
  rectas. Las puntadas cortas siguen mejor las curvas; las largas son más
  rápidas.
- **Ángulo**: la dirección de las filas de un relleno. La herramienta
  **Ángulo** lo define con dos clics.
- **Compensación de tiro**: ancho extra, porque las puntadas encogen la
  tela. La tela elástica necesita más.
- **Dividir puntadas de más de**: las puntadas de satén más largas reciben
  puntos de aguja extra, para que no se enganchen.
- **Puntadas cortas en curvas**: puntadas más cortas en el interior de las
  curvas cerradas, para que la curva no quede demasiado densa.
- **Omitir la última puntada**: la columna termina una puntada antes, en
  el otro lado. Úselo donde una columna se une con la siguiente.
- **Líquido**: el ángulo de puntada cambia de forma uniforme a lo largo de
  la columna, también donde las líneas de ángulo de puntada lo giran
  bruscamente.
- **Relieve (espuma 3D)**: vea [Decoración especial](specialty).

## Base

La base son puntadas bajo las puntadas superiores que sujetan la tela y
levantan las puntadas superiores.

| Base | Uso |
|---|---|
| Normal | El programa elige la base según el tamaño del objeto. |
| Ligero | Menos base, para tela fina o delicada. |
| Fuerte | Más base, para tela blanda, elástica o de pelo alto. |
| Doble | Dos capas que se cruzan: dos zigzags en las columnas, dos capas de tatami en los rellenos. |
| Encaje | Para encaje independiente: una recta dos veces a lo largo del borde, después un zigzag o filas a través. |
| Ninguno | Sin base. Úselo para texto pequeño y detalles finos. |
| Personalizado | Capas de base definidas una a una. |

La tela del diseño (Diseño > Tela) cambia la base de los objetos que
tienen base automática.

## Efectos de las columnas

La parte **Efectos** de Propiedades del objeto cambia cómo se borda una
columna de satén:

- **Espaciado de acordeón**: el espaciado cambia del inicio al final, para
  sombrear. **Personalizado**: defina el espaciado en puntos a lo largo
  del objeto.
- **Puntada a mano**: los puntos de aguja se mueven un poco al azar, para
  que las puntadas parezcan cosidas a mano.
- **Cordón**: una línea, una parada para poner un cordón sobre ella, y
  después un zigzag abierto que sujeta el cordón.
- **Esquinas**: **Inglete** (el borde exterior acaba en punta), **Recto**
  (el borde exterior se corta recto) o **Solapar** (la columna termina y
  una nueva empieza sobre la esquina).
- **Salto automático**: sin puntos de aguja extra en las puntadas largas;
  las puntadas más largas de lo que permite la máquina van como saltos.
- **Espaciado medido en**: en qué lugar a lo ancho de la columna se mide
  el espaciado en las curvas (0 en el riel A, 0,5 en el centro, 1 en el
  riel B).
- **Espaciado automático**: las partes estrechas de la columna reciben un
  espaciado más abierto, las partes anchas uno más denso.
- **Borde dentado**: las puntadas del borde terminan a distancias al azar,
  para pelo o hierba.
- **Compensación de tiro** en porcentaje, y **Tiro del riel B**: más tiro
  para columnas más anchas, y un tiro propio para el lado B.
- **Trapunto**: los desplazamientos van por el borde, no a través del
  interior, para que el interior se pueda acolchar.

## Las propiedades actuales

Los objetos nuevos reciben las **propiedades actuales**: los ajustes de
la barra de propiedades y del panel Estilos. **Diseño > Hacer actual**
toma los ajustes de los objetos seleccionados; **Aplicar actual** da las
propiedades actuales a los objetos seleccionados. Vea [Telas, estilos y
plantillas](fabrics-and-styles).

## Tipos de puntada

Cada tipo de objeto tiene tipos de puntada: las líneas pueden ser rectas
de motivo o columnas, las columnas pueden ser zigzag, puntada E y más, los
rellenos pueden ser de contorno, espiral, de motivos y más. Vea [Tipos de
puntada](stitch-types).
