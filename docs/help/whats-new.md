---
id: whats-new
title: What is new
group: start
keywords: new, changes, version, update, release, upgrade, news, latest, check for updates, new version
---
# What is new

This topic lists the changes of each version, the newest first.
**Help > What Is New** opens it. After an update, the start page has a
link to it.

## Version 0.6.0

**Tutorials and lessons.** The help has 31 tutorials in seven paths, from
the first design to production (**Help > All Tutorials**). Each tutorial
has pictures and a start design that a link opens. **Start the Lesson**
on the card of a tutorial makes it a lesson in the program: the
**Lesson** window shows one step at a time, marks the button or the place
on the canvas to click, and checks the design before it goes on. The
window has **◀ Back**, **Skip ▶** and **Stop the Lesson**, and a lesson
that stops keeps its place.

**Checks before you sew.** **View > Check > Travel That Shows** marks the
travel stitches that no later stitch covers, and Design Info says how
much travel shows. **Arrange > Sew Order > Quicker to Sew** merges
threads that look the same, removes objects that are too small to sew,
puts the color changes in a better order and sets the trim distance of
the design, in one undo step.

**Test swatches.** **Design > Output > Test Swatches** makes copies of the
selected objects with one setting in steps (for example the density),
each with a label of its value. Sew the sheet on your fabric, then use
the setting of the best copy.

**Satin lean and bands.** **Lean** in Object Properties leans the top
stitches of a satin column. **Edit > Stitch Types > Bands** sews a column
as bands of threads side by side, each in its color.

**Machine files.** **Edit > Machine Functions > Fix Stitches…** checks the
stitches of a machine file that you opened and fixes the faults that it
can, for example color changes to the same thread and stitches shorter
than the shortest stitch of the machine.

**Your open designs come back.** When you quit, the program keeps the
open designs with their changes that are not saved, and opens them
again at the next start (**Open tabs** in the Options, on at the start).
A file that you open while the program runs opens in a tab of that
program; `--new-window` starts a second program.

**Your own AI agent.** `emb-cli mcp` lets your own AI agent make, check,
export and show designs (help topic **Agents (MCP)**). When another
program saves a file that is open in a tab, the program asks: **Open
Again** or **Keep This Version**. The program has no AI in it and no
network code.

**Better stitches.** Less thread where the parts of a letter meet: the
underlay of a column stops where another part of the letter covers it.
Monograms at large sizes build up less thread, because each letter
chooses columns or fills by its size in the design. Columns whose
stitches run along their sides sew at their spacing. Travel in a tatami
fill goes along its rows, and travel in a letter goes along the edges of
its parts, so less travel shows.

**Panels and fields.** The side panels are two groups, one over the
other, so all their titles show on a small screen; only the open tab of
a group has its ×. A number field takes a decimal comma (`2,5`); in
English and Japanese, `5,000` is five thousand. A field also reads
full-width digits. **Edit Nodes** is in the menu of an object and in
Object Properties.

**Names in your language.** The names of the built-in motifs, split
patterns, stamps, borders, clipart items and families show in the
language of the program, and the searches find both names. **Edit >
Stitch Types > Families** opens the Families window.

**Faster and smaller.** The sew orders of the letters of a font of your
computer stay on the disk, so the font is quick again at the next start.
**Recognize Objects** is much faster with large files. The program is
about 7 MB smaller: the pictures of the help have the same pixels in
smaller files.

## Version 0.5.0

**Letters that keep the shape of the font.** The program makes better
letters from TrueType and OpenType fonts, and the embroidery fonts are
made again with it. The stitches of a column stay in the outline of the
letter, the ends of the columns go to the corners of the strokes, and
the stitches across a column do not cross. Where two strokes meet at a
corner, one goes on to the outer corner and the other ends square on its
side. A column that ends in another column sews before it. The font
packages of this version have the new letters too: import them again.

**Three fonts are back.** The program has 68 embroidery fonts. Slab
Heavy, Fat Face Serif and Shrikhand are back in the program: with the
better letters, their quality score is 87, 91 and 95. Great Vibes stays
in the font package **Fonts of earlier versions**. Fat Face Serif has
very thin strokes, so its smallest size is 42.5 mm. A design that has
one of the three fonts finds it in the program again.

**Faster sew orders.** Lettering in a TrueType or OpenType font of your
computer gets the sew order of its letters about four times faster: the
letters and digits of a serif font such as Georgia in 8 s, 33 s before.
The sew orders are the same as before.

**Panels and the ribbon.** The side panels start as one group at the
right, and a closed panel opens again in that group. **Reset the
Panels** (in the Options, and in **View > Panels**) puts the panels back
where they start. In a narrow window, the lists of a folded ribbon panel
(for example the machine and the hoop on the Design tab) open again.

**Guides from the rulers.** Right-click a ruler (on a touch screen, hold
it) for its menu: **Add Guide at** that place, the **Construction Line**
and **Construction Point** tools, and **Remove Guides**.

**Touch screens.** A bar under the canvas has the keys of the draw tools:
**Next** or **Finish**, **Remove Point**, **Cancel**, and **Done**, which
goes back to Select. So you can use each tool with no keyboard.

**Other changes.** **File > Check for Updates…** is there too. New
lettering goes back to the Select tool. **Recognize Objects** finds a
dense satin column with short stitches as a column, not as a run.

## Version 0.4.0

**The product view.** **Design > Product** shows the design on a product
that looks like cloth.

- The products have light, folds and the texture of their fabric, and the
  embroidery has a soft shadow.
- New products: a six-panel cap, a long-sleeve T-shirt, a hoodie, an
  apron, a beanie and a messenger bag.
- A garment has a front and a back. You can put more designs on one
  product, each at its own place.
- Two-tone products: a second color for the collar, the cuffs and other
  trim parts, and a list of garment colors.
- Your own product photos: mark the cloth of the photo, and the photo
  shows the product in each color. Mark the places for a design, and
  keep the photo in **My Products**.
- **Design > Turntable** turns the product in steps. You can save the
  turn as an animated picture or as four views.
- The approval sheet can show the design on the product.

**Lettering.** **Lettering > Font Test Sheet** makes a design with the
letters of a font at some sizes, to sew and to look at. In **Lettering >
Alphabets** you can correct a letter of an embroidery font: its sew order,
its shape and its settings. See [Lettering](lettering).

**Letters with few trims.** The letters of all the embroidery fonts of
the program are made again. The program finds the strokes of a letter as
a pen writes them: a stroke goes on through a junction, and it stops at a
sharp corner. The letters sew with few trims (the needle goes from one
stroke to the next one under the stitches), and their corners and dots
have stitches to the edge. Ten fonts are **tuned**, and each letter of
them was checked: Block Sans, Friendly Serif, Pacifico, Atkinson
Hyperlegible, Varsity, Bebas Neue, Soft Round, Geo Slab, Brush Hand and
Vintage Script. The font lists show them first. Lettering that is in a
design changes when you change its text or its font.

**More writing systems.** With a TrueType or OpenType font, lettering
now shapes its text: ligatures, the joined letters of script fonts, Arabic
(from right to left) and Devanagari. The fonts Tajawal (Arabic) and
Poppins (Devanagari) come with the program. See [Lettering](lettering).

**Stitches that follow the strokes.** With a TrueType or OpenType font
and the stitch type **Automatic**, the strokes of the letters are
columns: satin, with split stitches in strokes wider than 7 mm, or rows of
tatami if the strokes of the text are wider than 12 mm. All the letters
of a text get the same stitch. Before, a letter with one wide stroke was
all fill: in a bold font, most kanji and kana at 50 mm were fill. Where
two strokes cross, one goes through and the other stops at its sides, and
where a stroke ends on another, it stops a little inside it: no lumps of
thread there. Fonts with strokes that overlap (many fonts from the web)
now give letters with no holes and with no stitches outside the letter.
The letters of these fonts also get a sew order with few trims, as the
embroidery fonts of the program have: 刺繍東京 at 20 mm has 23 trims,
not 48. The height of Japanese, Chinese or Korean text is now the height
of its characters (it was the height of a Latin capital, so the characters
were about a quarter larger); lettering in designs that you saved before
keeps its size. Strokes thinner than 0.8 mm sew as runs, and the Lettering
window shows the smallest height at which the text sews well. See
[Lettering](lettering).

**Japanese embroidery fonts.** A new font package has two Japanese
embroidery fonts, Noto Sans JP (a gothic) and Zen Maru Gothic (a rounded
gothic): Latin letters, the kana, the 2,136 jōyō kanji and Japanese
punctuation, each letter with the sew order of the fonts of the program.
Above the largest size of their satin columns, the strokes sew as tatami
columns, not as fills.

**More fonts, with a quality score.** The program has 65 embroidery fonts
(30 before), and the font packages on the page of the program have 70
more. Each embroidery font has a quality score from 0 to 100:
**Lettering > Fonts** shows it and can sort by it. The size ranges of some
fonts are smaller now, so that their thin strokes are not too narrow at
the smallest size. Four fonts with a low score (Slab Heavy, Fat Face
Serif, Shrikhand and Great Vibes) are now in the font package **Fonts of
earlier versions**. A design that has one of them stays as it is.

**Fill fonts.** Ten of the embroidery fonts have letters that sew as
fills: Block Sans Fill, Varsity Fill, Anton Fill, Archivo Black Fill,
Slab Heavy Fill, Poster Heavy Fill, Giant Round Fill, Funky Slab Fill,
Rubik Mono One Fill and Shrikhand Fill. They are for large text, from
15 mm to 300 mm, and for heavy letters whose shapes have no clear
columns. In **Lettering > Fonts**, **For large text** shows them.
See [Lettering](lettering).

**A stricter quality score.** The quality score of an embroidery font
now also counts the stitches outside the shapes of the letters and the
places with two layers of stitches. Some fonts have a score that is a few
points lower. No letter of the fonts of the program changed. When you
convert a font with the stitch type **Automatic**, a letter whose columns
leave a part of its shape open becomes a fill, if the font is for large
letters (from 15 mm).
See [Lettering](lettering).

**Large lettering in each font.** Each embroidery font with satin
letters now also has its letters as fills. Above the largest size of its
satin columns, a font sews fill letters, so it goes up to 300 mm.
**Stitch type** in the Lettering window selects satin or fill for the
whole text.
See [Lettering](lettering).

**Kinds and weights of thread.** A thread has a kind (polyester, rayon,
cotton, metallic, wool or acrylic, glow in the dark) and a weight (40 is
the usual). Set them for the current thread in the Threads panel. The
thread look shows a thread of another weight thinner or thicker, and a
matte thread with no shine. A chart thread gives its kind when the name
of the chart tells it. In the design pictures, the colors at the edges of
the threads now mix as light does. See [Colors and threads](colors-and-threads).

**A new 3D thread look.** The 3D thread look of the canvas shows the
surface of the stitches: thread on thread is higher, an area is lighter or
darker by the direction of its stitches to the light, threads shine by
their kind, and high parts put shadows on the fabric. It also shows large
designs faster when you move and zoom the view. **View > Thread Look >
Quality > Fast** turns it off on a slow computer. See [Checking a
design](checking).

**The fabric in the 3D look.** The fabric under the stitches has a
surface at its real size: the threads of a plain weave or a twill, the
loops of a knit, the cells of piqué, the loops of a towel, the hair of
fleece, felt, and the grain of leather, in the light of the thread look.
Each needle hole is a small dent, larger on leather. On a towel or fleece
with no topping, thin stitches sink into the pile: the **Topping** check
box of the Styles panel sets it. A fabric can have a **Second color**
(the light threads of a denim, the second fiber of a heather). The
product view shows the same fabric. See [Fabrics, styles and
templates](fabrics-and-styles).

**What is not thread in the 3D look.** Appliqué fabric has the surface of
its weave (**Fabric surface** in the appliqué window) and a cut edge,
rhinestones have facets that take the light, the holes of cutwork show
what is behind the fabric, lace has no fabric, a trapunto fill is higher
over its padding, and on the new fabric **Quilt** the batting puffs up
between the lines of stitches. Cross stitch has the new fabric **Aida
cloth**. Puff columns are round over their foam. Metallic thread has
small glints at a large zoom, and **View > Thread Look > Lights off**
shows only glow-in-the-dark thread. See [Checking a design](checking).

**Objects from machine files.** **Artwork > Convert > Recognize
Objects** changes the stitches of a machine file into running objects,
satin columns and fills, with the settings that the stitches show. A
window shows the file and the objects next to each other, with the
places that differ; you can keep single objects as stitches. Options can
do it each time that a machine file opens. With the outlines of the same
design (a vector file), each outline becomes an object with the kind and
the settings of the stitches inside it. See [Vector artwork and
shaping](artwork).

**Ink/Stitch files.** **File > Open** opens the SVG files of Ink/Stitch
as designs: fills, satin columns and lines with their settings, the
commands for trims, stops and start points, and the thread colors. A
setting that the program does not read shows in a note. **Design > Export
for Ink/Stitch** writes a design as such a file, and **Lettering > Fonts >
Ink/Stitch Font…** converts the lettering fonts of Ink/Stitch. See [File
formats](file-formats).

**EPS files.** **Artwork > Import** reads EPS and PostScript files, and AI
files of the old kind, with no other program. Text and pictures in such a
file are not imported. A CDR file gives a message: save it as SVG, PDF or
EPS in the program that made it. See [Vector artwork and
shaping](artwork).

**Stitch parts.** **View > Stitch Parts** colors the stitches of a
design by the kind of their part: runs, columns and the rows of fills,
with the underlay of each part. The program finds them in the stitches
of a machine file of another program: the first step to make objects
with stitch types of such a file. Lines show the running objects, the
satin columns and the fills (with their underlay settings) that the runs,
the columns and the rows become. See [Checking a design](checking).

**All pictures in the 3D look.** The design picture, the approval sheet
and the quote, the library, the thumbnails of the file manager, the
kiosk, the stitch pictures of Picture to Design and the preview of the
Fonts window show the stitches as the 3D view does. **Output Design** can
write the picture at up to 600 dots for each inch with **No background**
(for a web shop), and a **Sew-out animation (GIF)**. The **Quality** in
the Thread Look menu (**Fast**, **Good**, **Best**) is chosen at the
first start from a short measure of the computer. See [The 3D
view](3d-view).

**Check the look.** The **Check** panel of the View tab has new tools:
light presets and a light that you move (hold **L** and drag), a **Tilt
View** that shows the embroidery from an angle with its heights (on the
canvas or in a window), a **Magnifier** at 20 times, **Compare 3D** and
**Before and After**, and two checks of the surface: **Fabric Shows
Through** and **High Thread**. The product and the turntable show the
stitches with their heights and their sheen, in the light of the
product. See [Checking a design](checking).

**Designs with very many stitches.** The program could stop when the
canvas showed about 350,000 stitches or more at one time in the thread
look. Now the canvas shows them in a simpler look at that zoom, and in
the full look when you zoom in.

**More motifs, patterns, stamps, borders and clipart.** The program has
71 motifs (15 before), 48 program-split patterns (10), 39 carving stamps
(8), 50 decorative borders (12) and 45 clipart items (8). Most of them
come from **families**: a family makes an item from some numbers, for
example the number and the height of the waves. **Families…** in the
Motifs window, in Object Properties and in the Clipart panel makes an
item of your own. A border can have another shape (also a diamond, a
hexagon and an octagon) and another size of its stitches. See
[Stitch types](stitch-types).

**New versions.** The program can tell you when a new version is
available. See [Options](options).

**The stitch player in the 3D look.** With the 3D thread look, the stitch
player shows the new stitches in the 3D look close behind the needle,
with no flash of the flat look, and the view stays quick after a drag of
the slider of the player.

**A new icon.** The icon of the program is a purple star in a hoop, with
a needle and a thread.

## Version 0.3.0

- The first start asks for your machine and your hoop. The machine
  library has about thirty machines, and **Design > My Machines** keeps
  your own.
- The View options stay for the next start.
- Endless pan: a pan with the mouse does not stop at the edge of the
  canvas.
- Lettering has bold, italic, underline and stacked text.
- Sketch mode has a fixed origin, and a sketch and its objects move
  together.
- The Windows installer removes the older version first.
