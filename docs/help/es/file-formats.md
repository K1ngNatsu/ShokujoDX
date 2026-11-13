---
id: file-formats
title: Formatos de archivo
group: reference
keywords: formato, archivo, extensión, leer, escribir, abrir, importar, exportar, dst, vp3, pes, pec, exp, jef, xxx, sew, hus, shv, tbf, tap, u01, dsb, dsz, 10o, dat, embproj, svg, pdf, ai, emf, wmf, png, jpeg, bmp, tiff, gif, gpl, csv, dxf, zip
---
# Formatos de archivo

## Archivos de máquina que escribe el programa

| Formato | Máquinas | Colores |
|---|---|---|
| DST | Tajima, la mayoría de las máquinas de varias agujas | no |
| VP3 | Husqvarna Viking, Pfaff | sí |
| PES | Brother, Babylock | sí |
| PEC | Brother (la parte de máquina de PES) | sí |
| EXP | Melco, Bernina | no |
| JEF | Janome | sí |
| XXX | Singer | sí |
| TBF | Tajima, con agujas y colores | sí |
| TAP | Happy | no |
| U01 | Barudan, con la función lenta | no |

Para un formato sin colores, **Producir diseño** puede escribir archivos
de colores de hilo (INF, EDR, COL) junto al archivo de máquina. Los
archivos TBF y U01 tienen la aguja de cada color (vea **Diseño >
Agujas**); para DST, TAP y EXP la hoja de trabajo da la secuencia de
agujas que hay que escribir en la máquina. DST y TAP no tienen código de
corte: vea **Diseño > Ajustes de la máquina**.

## Archivos de máquina que lee el programa

**Archivo > Abrir** e **Insertar puntadas** leen estos formatos, como
objetos de puntadas:

DST, VP3, PES, PEC, EXP, JEF, XXX, SEW, HUS, SHV, TBF, TAP, DSB, DSZ,
10O, 100, U01, DAT, EXY, FXY, GT, STX, PHB, PHC, BRO, EMD, INB, JPX, KSM,
MAX, MIT, NEW, PCD, PCQ, PCS, PCM, SPX, STC, ZHS y ZXY.

Un archivo de máquina tiene penetraciones de aguja, no formas. Para
escalarlo y conservar la densidad, use **Arte > Convertir > Reconocer
contornos**.

## Otros archivos

| Archivo | Leer | Escribir | Uso |
|---|---|---|---|
| EMBPROJ | sí | sí | El proyecto de este programa: todos los objetos y ajustes. |
| SVG | sí | sí | Arte vectorial de entrada; contornos, la imagen del diseño y archivos de plóter de salida. |
| PDF | sí | sí | Arte vectorial de entrada; hojas de trabajo, gráficos y plantillas de salida. |
| AI, EMF, WMF | sí | EMF | Arte vectorial. |
| PNG, JPEG, BMP, TIFF, GIF | sí | PNG | Imágenes de fondo de entrada; imágenes del diseño de salida. |
| DXF | no | sí | Archivos de plóter de apliques y plantillas de strass. |
| CSV | sí | sí | Cartas de hilos y nombres de equipo de entrada; propiedades del diseño y la lista de diseños de salida. |
| GPL | sí | no | Cartas de hilos (paletas de GIMP). |
| EMBFONT, BDF, FON | sí | EMBFONT | Fuentes de bordado y alfabetos; fuentes de mapa de bits. |
| TTF, OTF | sí | no | Fuentes TrueType y OpenType del ordenador. |
| ZIP | sí | sí | Paquetes de recursos; los archivos de Enviar por correo. |

## Nombres de máquinas y formatos

Los nombres de máquinas, formatos de archivo y marcas de hilo de este
programa son marcas de sus propietarios. Solo indican qué archivos lee y
escribe el programa. Los fabricantes no han probado ni aprobado el
programa. Pruebe cada diseño nuevo primero en un retal.
