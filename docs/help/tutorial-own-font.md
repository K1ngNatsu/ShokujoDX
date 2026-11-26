---
id: tutorial-own-font
title: "Tutorial: an embroidery font of your own"
group: tutorials
path: lettering
time: 25
level: advanced
before: tutorial-fonts
keywords: tutorial, font, own font, convert, conversion, truetype, opentype, embroidery font, test sheet, font test sheet, correction, letter correction, correct a letter, alphabets, few trims, license, size range, quality score
---
# Tutorial: an embroidery font of your own

In this tutorial you make an embroidery font from a TrueType font, make
a test sheet of its letters to sew, and correct one letter.

An embroidery font sews better than a TrueType font: its letters are
made one time, with the columns, the sew order and the ends looked at,
and it has a size range and a quality score.

Check that the license of the font lets you change it. An OFL font (SIL
Open Font License) with a Reserved Font Name needs a new name.

## Convert the font

1. Click **Lettering > Lettering > Fonts**. The **Fonts** window opens.
2. Click **TrueType**: the list shows the TrueType fonts only.
3. Type `Graduate` in the search field, and click **Graduate** in the
   list. Graduate is a TrueType font of the program; use the font of your
   logo the same way.
4. Click **Convert to embroidery font…**.

![The convert settings](own-font-convert)

5. Type a **Name** for the embroidery font, for example `Graduate
   Embroidery`.
6. At **Stitch type**, keep **Automatic**: satin columns, and fills for
   strokes too wide for satin. **Fill** makes a fill font for large text.
7. Keep **Few trims**: the program finds the sew order of each letter
   with the fewest trims. The conversion takes longer: some seconds on a
   fast computer, a minute or more on a slow one.
8. Click **Convert**. A progress bar shows the steps. Then the window
   says "Embroidery font Graduate Embroidery is ready", with its quality
   score.

The new font is in the font lists, in the folder of your fonts. A new
version of the program keeps it.

## Make a test sheet

A test sheet has all the letters of the font at its smallest, middle and
largest size. Sew it on the fabric of your work before you sew much
text with the font.

1. In the **Fonts** window, with the font selected, click **Test
   Sheet…**. The **Font Test Sheet** window opens.
2. At **Characters**, select **Capital letters**. (**Letters, digits and
   marks** makes a larger sheet.)
3. Click **Make Test Sheet**. The sheet opens as a new design.

![A test sheet of capital letters](own-font-sheet)

Look at the letters with the thread look, and sew the sheet. Look for
gaps between the strokes, columns that are too narrow at the smallest
size, and loose satin at the largest size.

## Correct a letter

If one letter does not sew well, correct it. The correction stays when
you make the font again.

1. Click **New**: the letter goes in a new design.
2. Click **Lettering > Lettering > Alphabets**. The **Alphabets** window
   opens. **Letter corrections** is in its lower part.
3. Select the **Font**, and type the letter at **Character**, for
   example `A`.
4. Click **Place Letter**. The objects of the letter go in the design,
   between guides, in their sew order.

![A letter placed for a correction](own-font-letter)

5. Change the objects: their order (the Sequence panel), the direction
   of a column (**Edit > Lines and Columns > Reverse Direction**), their
   shape (**Digitize > Select > Reshape**) or their settings (Object
   Properties).
6. Select all the objects of the letter, and click **Save Correction**.

**Remove Correction** gives back the letter of the font. The corrections
are a file in the folder of your fonts (**Show File**).

## If it does not look right

- **Convert is gray.** Type a name first.
- **The quality score is low.** Strokes that are too thin or too wide
  for satin columns lose points. Try **Fill** for a heavy font, or use
  the font only in its size range.
- **The test sheet is larger than the hoop.** Select fewer sizes or
  characters, or set a smaller **Row width**.

## Next

- [Tutorial: Japanese text](tutorial-japanese)
- [Lettering and monograms](lettering): fonts, alphabets and letter
  corrections
