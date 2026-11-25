---
id: tutorial-script
title: "Tutorial: un script que hace el trabajo"
group: tutorials
keywords: tutorial, script, scripts, rhai, automatización, macro, comprobar antes de producir, densidad de satén, separación, todos los satenes, un paso de deshacer, mis scripts, herramienta
---
# Tutorial: un script que hace el trabajo

En este tutorial ejecuta dos scripts de ejemplo en el escudo del club:
uno comprueba el diseño antes de la producción y otro da a todas sus
columnas de satén la misma densidad en un paso. Después ve cómo hacer un
script propio.

## Abrir el diseño inicial

Haga clic en [Abrir el escudo del club](sample:club-crest). Sus dos
columnas de satén, el anillo dorado y el galón rojo, tienen separaciones
distintas.

## Comprobar el diseño antes de la producción

1. Haga clic en **Editar > Scripts > Scripts**. Se abre la ventana
   **Scripts**.
2. Seleccione **Check Before Output** en la lista **Script**. La ventana
   muestra lo que hace el script y sus ajustes: los límites de su taller.
3. Ponga **Most colors** en 3.
4. Haga clic en **Ejecutar script**. En **Mensajes**, el script dice que
   el diseño tiene 4 colores: más de 3.

Este script no cambia nada. Ejecútelo antes de hacer el archivo de
máquina.

## La misma densidad para todos los satenes

1. Seleccione **Same Density for All Satins** en la lista **Script**.
2. Ponga **Spacing (mm)** en 0,4.
3. Haga clic en **Ejecutar script**. El mensaje dice que 2 columnas de
   satén tienen ahora una separación de 0,4 mm.

![La ventana Scripts después del script](script-window)

Sin nada seleccionado, un script trabaja en todo el diseño. Seleccione
antes unos objetos para cambiar solo esos.

## Un paso de deshacer

Haga clic en **Deshacer**: las dos columnas de satén recuperan sus
separaciones de antes. Todos los cambios de un script son un paso. Haga
clic en **Rehacer** para hacerlos otra vez.

Si un script se para con un error, el diseño no cambia.

## Un script propio

Un script es un archivo de texto en el lenguaje Rhai, con la extensión
`.rhai`.

1. Seleccione un script de ejemplo y haga clic en **Copiar a mis
   scripts**. El programa pone una copia en su carpeta de scripts.
2. Haga clic en **Abrir la carpeta de scripts** y cambie la copia en un
   editor de texto. Las primeras líneas que empiezan por `//!` son su
   título, su descripción y sus ajustes.
3. Haga clic en la flecha redonda junto a la lista **Script** para leer
   otra vez la carpeta.

Un script solo puede cambiar el diseño: no puede abrir ni guardar
archivos, ni usar la red. [Scripts](scripts) da la lista de las
funciones.

## Si no se ve bien

- **El script no cambió nada.** Lea los mensajes: quizá la selección no
  era la que necesita el script. No seleccione nada para trabajar en todo
  el diseño.
- **El script se paró con un error.** El texto rojo dice por qué; el
  diseño no cambió.
- **Mi script no está en la lista.** Haga clic en la flecha redonda para
  leer otra vez la carpeta. El archivo debe terminar en `.rhai`.

## Siguiente

- [Scripts](scripts): las funciones de un script
- [Tutorial: una insignia en el modo boceto](tutorial-sketch)
