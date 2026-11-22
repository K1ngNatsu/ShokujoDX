---
id: lettering
title: Lettering and monograms
group: design
keywords: lettering, text, name, font, knockout, overlap, overlapping text, embroidery font, truetype, opentype, size range, small text, height, letter spacing, line spacing, kerning, align, baseline, arc, circle, line, envelope, lettering art, sew order, trims, jumps, letter, reshape, alphabet, bitmap font, monogram, initials, team names, names list, csv, keyboard designs, kiosk, shop, mitre, mitre corners, corner, bold, italic, underline, style, vertical, stacked, text direction, test sheet, font test, letter correction, correct a letter, stroke order, arabic, devanagari, hindi, right to left, ligature, shaping, tajawal, poppins
---
# Lettering and monograms

The **Lettering** tab adds text. The tutorial [A name on an
arc](tutorial-lettering) shows the first steps.

## Add lettering

1. Click **Lettering > Lettering**, then click on the canvas where the
   text starts.
2. In the Lettering window, type the text. **Enter** starts a new line.
   The canvas shows the lettering as you type.
3. Select a font, set the **Height** and the **Stitch type**. The font
   list shows each name in its own font. Point to a name, and the canvas
   shows your text in that font. The height is the height of the capital
   letters; for Japanese, Chinese or Korean text, it is the height of its
   characters. For such text in a TrueType font, the window shows the
   smallest height at which its strokes sew well; with **Automatic**, a
   stroke thinner than 0.8 mm sews as a run.
4. Click **OK** (or **Ctrl+Enter**). **Esc** or **Cancel** closes the
   window and takes back what it shows. After **OK**, the tool is
   **Select** again.

To change the text later, double-click the lettering, or right-click it
and select **Edit text**.

## Fonts

There are two kinds of fonts:

- **Embroidery fonts** have ready-made embroidery letters. They sew best.
  Each has a **size range** (the capital height): the font list shows it,
  and Object Properties warns when the text is out of the range. 65
  embroidery fonts come with the program.
- **TrueType** and OpenType fonts of the computer. The program makes the
  letters from their outlines: columns that follow the strokes
  (**Automatic**: satin, with split stitches in strokes wider than 7 mm;
  if the strokes of the text are wider than 12 mm, rows of tatami), or all
  **Satin** or all **Fill**. Where two strokes cross, one goes through
  and the other stops at its sides, so the thread does not lie two times
  there.

**Lettering > Fonts** opens the font list: search, see a preview and the
size range, and show **All**, **Embroidery**, **TrueType**, **For small
text** (embroidery fonts for letters from 4 mm high) or **For large text**
(the fill fonts). **Use for the selected lettering** changes the font of
the selection.

The letters of the embroidery fonts of the program sew with few trims:
the needle goes from one stroke to the next one under the stitches. Ten
of the fonts are **tuned** fonts: each letter of them was checked. The
font lists show the tuned fonts first, and the font list of **Lettering >
Fonts** marks them. The letters of TrueType and OpenType fonts get the
same sew order when you use them the first time: the program finds it
while you type, and a new letter can show for a moment before it has its
order.

Ten of the embroidery fonts are **fill fonts**. Their names end with
"Fill", for example Slab Heavy Fill. Each letter of a fill font sews as a
fill, not as satin columns, so the full shape of a heavy letter has
stitches. Use the fill fonts for large text: their size range starts at
15 mm or more and goes to 300 mm. The **Fill** settings of the lettering
set how the letters sew. The quality score of a fill font has its own
measure (how much of the letters is too narrow for a fill at the smallest
size), so **Sort by > Quality** shows the fill fonts after the other
embroidery fonts.
The other embroidery fonts of the program have satin letters, and each of
them also has its letters as fills. Up to the largest size of its satin
columns, a font sews satin letters. Above that size it sews its fill
letters, so each font goes up to 300 mm and a word has one look. The
Fonts window shows the size of the change. **Stitch type** in the
Lettering window selects one of the two for the whole text: **Satin** or
**Fill**. The fonts for small text have satin letters only.

Each embroidery font has a **quality score** from 0 to 100. The score
says how well the letters of the font sew. The font list shows it after
the name of the font, and **Sort by > Quality** puts the fonts with the
best scores first. Select a font to see its score and these
measures:

- how much of the shapes of the letters the stitches cover,
- how much of the satin columns is too narrow at the smallest size of the
  font,
- how much of the satin columns has stitches that are too long at the
  largest size of the font,
- how much of the stitches is outside the shapes of the letters,
- how much of the letters has two layers of stitches.

Each font of the program has a score of 65 or more. A font that you
convert gets its score when you convert it. The size range of a font
keeps its thin strokes wide enough at the smallest size, so a font with
thin and thick strokes has a small range.

**More Fonts** opens the page of the program on the internet. The **font
packages** there have 70 more embroidery fonts: sans, display, serif,
script and hand fonts, and two Japanese fonts (Noto Sans JP and Zen Maru
Gothic, with the kana and the 2,136 jōyō kanji). Download a package, then
add it with **File > My Assets > Import a package**. The program downloads
nothing itself.

Four fonts of earlier versions have a score below 65: Slab Heavy, Fat
Face Serif, Shrikhand and Great Vibes. They are not in the program now.
A design that has one of them stays as it is. To change its text, add the
font package **Fonts of earlier versions**.

**Convert to embroidery font** makes an embroidery font from a TrueType
font, with a name, a size range and a stitch type. Check that the license
of the font lets you change it.
With **Few trims**, the program finds the sew order of each letter, as
the fonts of the program have it: the needle goes from one stroke to the
next one under the stitches, and the columns reach the corners. This
takes longer; a bar shows how far the conversion is, and **Stop** ends
it.
With the stitch type **Automatic**, the program makes each letter with
satin columns and as a fill, and keeps the one that sews better. A fill is
better only in a font for large letters (a size range from 15 mm): there,
a letter whose columns leave a part of its shape open becomes a fill.

**Ink/Stitch Font…** converts a lettering font of Ink/Stitch to an
embroidery font: choose the folder of the font (with font.json and
ltr.svg). Each letter keeps its parts in the order of the font, and the
underlay, the pull compensation and the fill angle of each part; the
lettering object sets the spacing. After the conversion, a note shows the
license of the font. Check that the license lets you use the font.

## Arabic, Devanagari and ligatures

With a TrueType or OpenType font, the program shapes the text as a word
processor does:

- **Ligatures and joined letters.** A font with ligatures makes one letter
  from "fi" or "ff", and a script font uses the forms of its letters that
  join.
- **Arabic** letters join and have the form for their place in the word.
  The text goes from right to left, also in a line with Latin words or
  digits, and it sews in reading order.
- **Devanagari** letters make conjuncts, and a vowel sign that is written
  before its consonant goes there.

Two fonts for these writing systems come with the program: **Tajawal**
(Arabic) and **Poppins** (Devanagari). You can also use a font of your
computer. If the font does not have a character of your text, the
Lettering window shows which characters have no letter.

A ligature, or a letter with its marks, is one letter of the lettering:
you move, turn and change it as one. The embroidery fonts have one letter
for each character and do not shape the text.

## Font test sheet

**Lettering > Font Test Sheet** (or **Test Sheet** in the font list) makes
a new design with the letters of a font, to sew and to look at:

- **Smallest**, **Middle** and **Largest**: the sizes of the size range of
  the font. Clear a size that you do not want, or type another height.
- **Characters**: the letters, digits and marks, all characters of the
  font, a part of them, or **My text**.
- **Row width**: the rows of letters are not wider than this. It starts at
  the width of the hoop.

Each size has a label with the name of the font and the size. The window
shows the size of the sheet. If the sheet is larger than the hoop, use
fewer sizes or characters, or sew it in more than one hooping (see
[Designs larger than the hoop](hooping)).

## The settings of lettering

Select the lettering. Object Properties has:

- **Text**, **Font**, **Height** (the height of the capital letters).
- **Letter spacing**: extra space after each letter. **Line spacing**:
  the distance between lines, for text with more lines.
- **Align**: **Left**, **Center** or **Right**: where the text starts on
  the baseline.
- **Baseline**: **Straight**, **Arc over**, **Arc under**, **Circle,
  clockwise**, **Circle, counter-clockwise** or **Line**. Set the
  **Radius** of an arc or a circle.
- **Lettering art** bends text on a straight baseline into a shape (an
  envelope): Bridge, Valley, Arch over, Arch under, Bulge, Pinch, Pennant,
  Perspective, Diamond, Wave or Oval. **Amount** sets how far the edges
  move.
- **Kerning**: use the letter pairs of the font (for example "AV") to
  set the space between letters.
- **Sew Order**: **In text order**, **Center out** (for caps) or
  **Closest Join**. **Trim gaps longer than** sets which gaps get a trim;
  shorter gaps get a travel stitch. **Jumps, no trims** is for machines
  with no trimmer.
- The **Satin** and **Fill** settings of the letters: density, pull
  compensation and underlay.
- **Mitre corners** (under Satin): columns that meet at a corner, as in
  "L", "E" or "7", sew as one column that turns. Off, one column stops and
  the next column starts over it. A sharp corner, as at the top of an "A",
  and a stroke that ends on another stroke, as in "T", do not change. The
  stitches stay in the shape of the letter: a corner that cannot turn in
  it stays as a stop and a start. The letters of an embroidery font of the
  program or of a font package sew in the order of the font: there, the
  setting joins a corner only where its two strokes sew one after the
  other, so some corners stay a stop and a start.

## Bold, italic and underline

The **Style** buttons are in **Object Properties** of the lettering:

- **B** (Bold) and **I** (Italic) use the bold or italic face of the
  font when the font has one. Else the program makes the style: bold
  makes each satin column wider and each fill larger, italic slants the
  letters.
- **U** (Underline) adds a satin bar under each line of text. Set the
  **Underline gap** (from the baseline) and the **Underline width**. On an
  arc or a path, the bar follows the line.

## Stacked text

**Text direction** in **Object Properties** sets **Stacked, top to
bottom** or **Stacked, bottom to top**: each letter stays upright, one
under the other, as on a sign. Each line of the text is a column, and the
columns go from left to right. **Align** puts the top, the middle or the
bottom of each column at the start point. Stacked text has no arc,
envelope or underline. To turn a whole line of text 90°, turn the text
instead.

## Lettering on a line

**Lettering on Line** puts lettering on a line that you draw: select the
lettering, click the command, then click the line. Or select both first.
The line becomes the baseline. With the Reshape tool, drag the nodes of
the line to change it.

## Change one letter

Click **Reshape**, then click a letter. Drag the letter, or set its
**Kerning**, **Move along**, **Move down**, **Size** and **Turn** in
Object Properties. A letter of a TrueType font can have its own stitch
type and underlay. **Reset letter** and **Reset all letters** remove the
changes by hand.

## Overlapping lettering

When two texts overlap, select one or both of them and click **Artwork >
Shaping > Remove overlaps** (or select both and click **Flatten**). The
text behind gets a **knockout**: it does not sew where objects that sew
after it cover it. Satin letters stay satin.

The knockout follows each change: move, turn or type in a text, add a
text in front or change the sew order, and the cut changes with it. In
**Object Properties > Knockout**, **Sew round objects in front** turns the
knockout on or off for the selected lettering. **Overlap** sets how far
the letters go under the objects in front.

## Alphabets

**Lettering > Alphabets** makes your own embroidery alphabets:

1. Click **New alphabet** and type a name.
2. Set the **Letter height** and **Baseline at**, then click **Add
   guides**.
3. Draw the letter between the guides with the digitizing tools.
4. Select its objects, type its **Character**, and click **Create
   letter**.

**Import** reads an embroidery font (.embfont) or a bitmap font (.bdf,
.fon). **Export** writes the alphabet. **Rename** and **Delete** change
it.

## Correct a letter of a font

You can change a letter of an embroidery font, also of a font that comes
with the program. In **Lettering > Alphabets**, under **Letter
corrections**:

1. Select the **Font** and type the **Character**.
2. Click **Place Letter**. The objects of the letter are in the design,
   between guides, in their sew order.
3. Change the objects: the order (the Sequence panel), the direction
   (**Reverse Direction**), the shape, the underlay or the pull
   compensation.
4. Select the objects and click **Save Correction**.

New lettering uses the corrected letter. Lettering that is in a design
changes when you change its text or its font. **Remove Correction** gives
the letter of the font back.

- **Keep the sew order**: the letter sews its objects in their order, each
  column from its start to its end. With this off, the program finds the
  order.
- **Keep the settings**: the letter keeps the underlay, the pull
  compensation and the fill angle of each object.

The corrections of a font are a file in the `corrections` folder of your
fonts folder (**Show File**). A new version of the program keeps them.

## Monograms

**Lettering > Monogram** makes one to three initials in a style, with a
border and ornaments.

- **Initials**: for example "JMK". **Style**: Single letter, Block,
  Classic (large middle), Stacked, Diamond or Oval. **Height**: the
  height of the largest initial.
- **Border**: the **Shape** (circle, oval, square, rounded square,
  diamond, double circle), the **Stitch** (satin, running stitch, triple
  run), the **Width** and the **Space** to the initials.
- **Ornaments**: dot, diamond, star, heart, leaf or scroll, at the left
  and right, above and below, or on all four sides.
- **Templates** sets all of these at once. **Save as Template** keeps
  your settings.

With a monogram selected, the window changes it.

## Team names

**Lettering > Team Names** makes one design with many names (and
numbers):

1. Select a lettering object of the design (for example the name), and
   click **Add the selected lettering as a column**. Add more columns, for
   example a number.
2. Type the names, **Paste a list** (one name for each line), or **Import
   CSV**. **Qty** sets how many of each to sew.
3. **Fit to width** makes a longer name smaller, so it is at most this
   wide.
4. **Save each name** writes one machine file for each name, in a folder.
   **All in one design** puts all names in one design (a new tab), color
   by color, for one hooping.

## Keyboard designs

**Lettering > Keyboard Designs** types with a collection of designs, one
for each key (for example a collection of small pictures). Select the
**Collection**, type the **Text**, set the **Height** and **Spacing**,
and click **Add to the design**. To make your own collection, select
objects and click **Map the selection to this key**.

## The lettering kiosk

**Lettering > Lettering Kiosk** is one large screen for a shop: a
product, a name, a font and a thread color, then save, output or send to
USB. See [Tutorial: a name on an arc](tutorial-lettering).

## Tips

- Below about 5 mm, letters lose their shape. Use a font **For small
  text**, or make the text larger. Small text needs no underlay.
- Puff lettering stands up from the fabric: see [Specialty
  decoration](specialty).
- On caps, use the sew order **Center out**.
