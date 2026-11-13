---
id: artwork
title: Gráficos vectoriales y formas
group: design
keywords: posición del borde, dentro, fuera, centrado, vector, arte, gráfico, svg, pdf, ai, emf, wmf, importar, dibujar arte, convertir, a relleno, a satén, a contorno, a vector, reconocer contornos, formas, soldar, intersecar, excluir, combinar, frente menos fondo, fondo menos frente, aplanar, dividir, quitar superposiciones, desvincular, vinculado, conservar originales, superposición, trazado de borde, desplazamientos, contornos, añadir borde, exportar contornos, plóter
---
# Gráficos vectoriales y formas

Los gráficos vectoriales son formas sin puntadas. Se ven en el lienzo,
pero no bordan nada, y los archivos de máquina los dejan fuera. Los usa
como guía, o los convierte en bordado.

## Importar arte

El panel **Arte > Importar** lee archivos SVG, PDF, AI, EMF y WMF:

- **Importar como vectores**: los trazados pasan a ser gráficos
  vectoriales. Conviértalos después.
- **Importar arte**: los trazados pasan a ser objetos de bordado al
  momento (rellenos y líneas).
- **Importar como líneas**: el contorno de cada forma y cada trazo pasa a
  ser una línea de puntada recta.
- **Insertar puntadas** añade al diseño las puntadas de un archivo de
  máquina, como objetos de puntadas.

También puede arrastrar un archivo SVG o PDF a la ventana.

## Dibujar arte

Active **Arte > Dibujar arte**. Las herramientas de dibujo (Forma, Línea,
A mano alzada, Rectángulo, Elipse, Estrella) crean entonces gráficos
vectoriales en lugar de objetos de bordado. Desactívelo para dibujar
bordado de nuevo.

**Vectores** muestra u oculta el arte. **Atenuar vectores** lo muestra
pálido, para que las puntadas destaquen.

## Convertir

Seleccione vectores, y después:

- **A relleno**: cada forma cerrada pasa a ser un relleno, con sus
  agujeros.
- **A satén**: cada forma cerrada pasa a ser un satén. Una forma larga
  recibe una columna giratoria por su centro, el anillo de una O recibe
  una columna en anillo, y las demás formas reciben un relleno con el tipo
  de puntada satén. Ninguna puntada sale de la forma ni pasa sobre otra
  parte de ella. Una línea pasa a ser una columna de satén a lo largo de
  ella.
- **A contorno**: todos los trazados pasan a ser una línea de puntada
  recta.

Otros comandos del panel:

- **A vector**: los objetos de bordado seleccionados pasan a ser gráficos
  vectoriales (sus contornos, sin puntadas).
- **Reconocer contornos**: los objetos de puntadas (de un archivo de
  máquina o de una puntada fotográfica) pasan a ser rellenos del área que
  cubren sus puntadas, para que pueda escalarlos y conserven su densidad.

## Formas

Las operaciones de forma crean formas nuevas a partir de dos o más formas
seleccionadas (vectores, rellenos, apliques y otros objetos con área). La
forma del frente es la que se borda al final.

| Comando | Resultado |
|---|---|
| Soldar | Una forma con todas las formas seleccionadas. |
| Intersecar | Solo el área que cubren todas las formas seleccionadas. |
| Excluir | Las áreas que las formas no comparten. |
| Combinar | Un objeto con los vectores seleccionados, o con los objetos de aplique seleccionados. |
| Frente menos fondo | La forma del frente, con las demás recortadas de ella. |
| Fondo menos frente | La forma del fondo, con las demás recortadas de ella. |
| Aplanar | Cada forma sin las partes que ocultan las formas que tiene delante, para que nada se borde dos veces. |
| Dividir | Las formas cortadas en piezas donde se superponen; cada superposición va a la forma del frente. |
| Quitar superposiciones | Las formas seleccionadas recortadas de los objetos que tienen debajo. |

Las opciones del panel:

- **Conservar originales**: los objetos seleccionados se quedan; el
  resultado va después de ellos.
- **Mantener vinculado**: Aplanar, Dividir y Quitar superposiciones
  vuelven a cortar cuando sus objetos se mueven. **Desvincular** termina
  los vínculos de la selección; los objetos se quedan como están.
- **Superposición**: cuánto entra una pieza cortada bajo la forma que la
  cortó, para que no se vea tela entre las piezas. Quitar superposiciones
  y la cuchilla también la usan.

Las letras siguen siendo letras en Aplanar y Quitar superposiciones. Las
letras de delante recortan los objetos que tienen debajo con sus letras.
Las letras debajo de otros objetos reciben un recorte: sus puntadas dejan
fuera el área de los objetos de delante, y el texto, la fuente y las
letras se pueden seguir cambiando. Los otros comandos de Formas no usan
letras.

## Trazado de borde

**Arte > Trazado de borde** añade un borde alrededor de los objetos
seleccionados, un hueco alrededor de ellos, o ambos, recortados de los
objetos que tienen debajo. En el hueco se ve la tela.

- **Ancho del borde**, la **Puntada** (un tipo de columna, una recta o una
  recta triple) y el **Hilo**.
- **Posición del borde**: dónde se sitúa el borde respecto al filo de los
  objetos. **Fuera de la línea**: el borde rodea los objetos y cubre solo
  un poco de su filo (una columna pasa 0,4 mm sobre él, para que no se vea
  tela). **Centrado en la línea**: el centro del borde está en el filo.
  **Dentro de la línea**: el borde cubre el filo de los objetos, y el
  diseño no se hace más grande.
- **Orden de bordado**: **Objetos, luego borde** o **Borde, luego
  objetos**.
- **Ancho del hueco**: el espacio alrededor del borde (o alrededor de los
  objetos, sin borde) donde se ve la tela.

El trazado sigue vinculado: se vuelve a cortar cuando sus objetos se
mueven. Cámbielo en la ventana mientras sus objetos están seleccionados.
**Quitar trazado de borde** devuelve los objetos cortados.

Los objetos y su borde son un grupo, así un clic los selecciona todos.
Para cambiar las puntadas de uno de ellos (por ejemplo el tipo de
relleno), haga doble clic en él, o haga clic en su botón bajo **Objetos
del grupo** en el panel Propiedades. Separar y Unir no funcionan con
objetos vinculados: use primero **Arte > Desvincular**.

## Desplazamientos

**Arte > Desplazamientos** añade contornos a una distancia alrededor de
los objetos seleccionados (**Fuera**) o dentro de ellos (**Dentro**):

- **Distancia** del primer contorno, el número de **Contornos** y el
  **Espaciado** entre ellos.
- **Esquinas**: **Redondo**, **Agudo** o **Cortar**.
- **Crear**: una **Línea**, un **Vector** o un **Borde** de satén de un
  ancho.
- **Posición del borde** (para un borde): **Fuera de la línea** (lejos del
  centro de los objetos, para que un borde a distancia 0 no los cubra),
  **Centrado en la línea**, o **Dentro de la línea** (hacia el centro de
  los objetos). Fuera y dentro no cambian para los contornos dentro de los
  objetos.

**Añadir borde** abre la misma ventana, preparada para crear un borde de
satén, con la posición del borde que usó la última vez.

## Exportar contornos

**Arte > Exportar contornos** escribe las formas de los objetos como
trazados SVG o EMF, para programas vectoriales y cortadoras. Para las
piezas de aplique, use **Digitalizar > Archivo de plóter**.
