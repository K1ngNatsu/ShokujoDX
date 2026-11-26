---
id: tutorial-script
title: "Tutorial: a script that does the work"
group: tutorials
path: more
time: 20
level: practice
before: tutorial-sew-well
keywords: tutorial, script, scripts, rhai, automation, macro, check before output, satin density, spacing, all satins, one undo step, my scripts, tool
---
# Tutorial: a script that does the work

In this tutorial you run two sample scripts on the club crest: one checks
the design before output, and one gives all its satin columns the same
density in one step. Then you see how to make a script of your own.

## Open the start design

Click [Open the club crest](sample:club-crest). Its two satin columns, the
gold ring and the red chevron, have different spacings.

## Check the design before output

1. Click **Edit > Scripts > Scripts**. The **Scripts** window opens.
2. Choose **Check Before Output** in the **Script** list. The window shows
   what the script does and its settings: the limits of your shop.
3. Set **Most colors** to 3.
4. Click **Run Script**. Under **Messages**, the script says that the
   design has 4 colors: more than 3.

This script changes nothing. Run it before you make the machine file.

## The same density for all satins

1. Choose **Same Density for All Satins** in the **Script** list.
2. Set **Spacing (mm)** to 0.4.
3. Click **Run Script**. The message says that 2 satin columns now have a
   spacing of 0.4 mm.

![The Scripts window after the script](script-window)

With nothing selected, a script works on all the design. Select objects
first to change only them.

## One undo step

Click **Undo**: the two satin columns get their spacings of before. All
the changes of a script are one step. Click **Redo** to do them again.

If a script stops with an error, the design does not change.

## A script of your own

A script is a text file in the language Rhai, with the extension `.rhai`.

1. Choose a sample script, and click **Copy to My Scripts**. The program
   puts a copy in your scripts folder.
2. Click **Open Scripts Folder**, and change the copy in a text editor.
   The first lines that start with `//!` are its title, its description
   and its settings.
3. Click the round arrow next to the **Script** list to read the folder
   again.

A script can change only the design: it cannot open or save files, or
use the network. [Scripts](scripts) lists the functions.

## If it does not look right

- **The script changed nothing.** Look at the messages: the selection was
  perhaps not what the script needs. Select nothing to work on all the
  design.
- **The script stopped with an error.** The red text says why; the design
  did not change.
- **My script is not in the list.** Click the round arrow to read the
  folder again. The file must end with `.rhai`.

## Next

- [Scripts](scripts): the functions of a script
- [Tutorial: a badge in sketch mode](tutorial-sketch)
