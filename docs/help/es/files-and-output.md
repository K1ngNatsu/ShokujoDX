---
id: files-and-output
title: Archivos y salida
group: production
keywords: abrir, guardar, proyecto, embproj, copia de seguridad, recuperar, fallo, importar, exportar, archivo de máquina, formato, dst, pes, vp3, usb, memoria, enviar, correo, zip, producir diseño, hoja de trabajo, hoja de aprobación, pdf, imprimir, imagen, png, svg, csv, código de barras, archivos de colores de hilo, inf, edr, col, lotes, cola, exportaciones, miniatura, icono, explorador, finder
---
# Archivos y salida

## Proyectos y archivos de máquina

- Un **proyecto** (`.embproj`) conserva todos los objetos y ajustes.
  **Archivo > Guardar** (Ctrl+S) lo guarda; **Guardar como** lo guarda con
  un nombre nuevo.
- Un **archivo de máquina** (DST, PES, VP3 y otros) solo tiene las
  puntadas, los colores (en la mayoría de los formatos) y las funciones de
  máquina. Conserve el proyecto para hacer cambios.
- **Archivo > Abrir** abre un proyecto, o un archivo de máquina como
  objetos de puntadas. El programa lee unos 40 formatos de máquina; vea
  [Formatos de archivo](file-formats).
- **Archivo > Diseños recientes** abre un diseño que usó antes.
- **Archivo > Cerrar** y **Cerrar todo** cierran diseños. El programa
  pregunta primero por cada diseño con cambios.

## Copias de seguridad

El programa guarda una copia de seguridad de cada diseño con cambios cada
2 minutos (**Opciones > Copia de seguridad automática**). Si el programa se
detiene, la ventana **Recuperar diseños** muestra las copias en el
siguiente inicio. Un informe de fallo va a la carpeta `crashes` de los
ajustes; **Ayuda > Acerca de** muestra dónde está.

## Exportar un archivo de máquina

**Diseño > Exportar** (Ctrl+E, o Archivo > Exportar archivo de máquina)
escribe las puntadas para la máquina. La ventana muestra el **Formato**
(el formato de la máquina es el predeterminado), el bastidor, el tamaño,
las puntadas, colores, cortes y paradas, la combinación de colores y el
tiempo de bordado.

| Formato | Máquinas |
|---|---|
| DST | Tajima, la mayoría de las máquinas de varias agujas |
| VP3 | Husqvarna Viking, Pfaff |
| PES | Brother, Babylock |
| PEC | Brother (la parte de máquina de PES) |
| EXP | Melco, Bernina |
| JEF | Janome |
| XXX | Singer |
| TBF | Tajima, con agujas y colores |
| TAP | Happy |
| U01 | Barudan, con la función lenta |

## Enviar a USB

- **Diseño > Enviar a USB** escribe el diseño en el formato de la máquina
  en la memoria USB de la máquina, y lo vuelve a leer para comprobarlo. La
  primera vez, elija la memoria y la carpeta.
- **Enviar a…** elige otra vez la memoria y la carpeta. El programa las
  conserva para cada máquina.

## Enviar por correo

**Diseño > Enviar por correo** pone el archivo de máquina, el proyecto y
una imagen en un archivo zip junto al diseño, y abre el programa de
correo. Adjunte el archivo zip al mensaje.

## Producir diseño

**Diseño > Producir diseño** crea todos los archivos de producción en un
paso, en una carpeta:

- los **Archivos de máquina** (uno o más formatos) y el archivo de
  proyecto;
- la **Hoja de trabajo de producción** y la **Hoja de aprobación** (PDF),
  con un **Código de barras en la hoja de trabajo** si lo quiere, y el
  tamaño del **Papel**;
- la **Imagen del diseño** (PNG, con el aspecto de hilo 3D si lo quiere) y
  (SVG);
- las **Propiedades del diseño** (CSV);
- los **Archivos de colores de hilo** (INF, EDR, COL) para DST y los otros
  formatos sin colores;
- la **Plantilla de strass** (SVG, PDF), para un diseño con piedras.

El programa conserva su elección para la próxima vez. El trabajo va a la
cola; el panel **Exportaciones** lo muestra, así puede seguir trabajando.
Vea [Tutorial: salida para la producción](tutorial-output).

## La hoja de trabajo y la hoja de aprobación

**Diseño > Vista previa de impresión** muestra la **Hoja de trabajo de
producción** y la **Hoja de aprobación**, página por página. **Notas para
las hojas de trabajo** añade texto, por ejemplo el trabajo, el cliente, la
tela y la aguja. **Guardar PDF** escribe las hojas; para imprimir,
imprima el PDF desde un visor de PDF.

La hoja de trabajo tiene el tamaño, las puntadas, el tiempo de bordado, el
bastidor y el lugar del diseño, cada color y parada, la tela, la
entretela y el recubrimiento, una imagen, y el orden de trabajo de los
strass.

## Producción por lotes

**Diseño > Producción por lotes** crea los archivos de Producir diseño
para muchos archivos de proyecto y de máquina a la vez. La biblioteca de
diseños también puede producir los diseños seleccionados.

## Otros archivos

- **Diseño > Exportar SVG**: la imagen del diseño como SVG, las puntadas
  como líneas en los colores de hilo, a tamaño real.
- **Arte > Exportar contornos**: las formas como trazados SVG o EMF.
- **Digitalizar > Archivo de plóter**: las líneas de corte de las piezas
  de aplique (SVG, DXF).
- **Especial > Plantilla**: la plantilla de strass (SVG, DXF, PDF).
- **Especial > Gráfico**: el gráfico de punto de cruz (PDF).
- **Diseño > Plantilla del bastidor**: el campo del bastidor a tamaño real
  (PDF).

## Imágenes en el administrador de archivos

Después de la instalación, el administrador de archivos (Explorer, Finder,
el administrador de archivos de Linux) muestra la imagen de cada archivo
de diseño como su icono: proyectos y archivos de máquina.

## Consejos

- Compruebe el formato de su máquina (la lista Máquina de la pestaña
  Diseño). La mayoría de las máquinas domésticas solo leen su propio
  formato.
- Compruebe la carpeta de la memoria USB. Enviar a USB usa la carpeta de
  su máquina.
- Vea [Preguntas y problemas](troubleshooting) si la máquina no lee el
  archivo.
