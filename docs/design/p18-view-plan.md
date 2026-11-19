# Plan: the 3D stitch view (phase P18, second part)

Date: 2026-11-19 · Status: Proposed. The owner asked for a thorough
upgrade of the engine of the 3D stitch view, and for this plan in phase
P18 (2026-11-19). The owner reads the plan and selects the order of the
milestones before the work starts. The first part of P18 (designs from
other programs, `docs/design/p18-plan.md`) does not change; the owner
decides which part starts first.

## Goal

The 3D thread look (P10 Milestone 3, ADR 0040) shows each stitch as a
round cord with light and shade. It was made to be as fast as the flat
look. It does not look like a sew-out. This plan makes the view show what
a person sees on the fabric: thread with height and sheen, on a fabric
with a surface, at each zoom, and fast.

### What the program has now

Measured on the development computer (release build, 2026-11-19):

- **The model** (`emb_render::thread3d`): a stitch is a straight round
  cord from one needle point to the next one. Each stitch has its light
  alone: an ambient part, a diffuse part and a highlight on its top. It is
  darker near its ends. Its shadow is a black copy at 12 %, moved away
  from the light. The stitches are painted in sew order.
- **The canvas** (`canvas::thread_mesh_3d`): the program makes a mesh of
  all visible stitches again for each frame, on the thread of the window:
  5 points across a stitch, 2 or 3 rows along it, one color for each
  point from a table of 64 directions. It takes 2.2 ms for 100,000
  stitches and 14 ms for 1,000,000 stitches. When a thread is narrower
  than about one pixel, a stitch is a thin line.
- **The pictures** (`emb_render::raster`): each pixel of each stitch gets
  the light of the model and the twist of the plies, on one processor.
  All threads are 0.4 mm wide.
- **The fabric**: a brightness pattern that repeats each 4 mm (plain
  weave, twill, knit, piqué, terry, felt, smooth), under the stitches.
- **A thread** (`emb_model::Thread`) has a color, a name, a brand and a
  code. It has no kind and no thickness.
- **The product view and the turntable** (ADR 0101, 0108) light a height
  field of the product; the embroidery on it is the flat picture of the
  stitches with a soft shadow.

### What a sew-out shows and the view does not

1. **Height.** Thread lies on thread. A satin column is round across its
   width, a fill is a flat field, underlay lifts the stitches over it,
   puff stands up, and a border over a fill is higher than the fill. In
   the view each stitch is at the same height; only the order of the
   paint shows what is on top.
2. **Sheen by direction.** The first thing that the eye reads in
   embroidery is that an area is light or dark by the angle of its
   stitches to the light. This makes a satin look like a satin, and it
   shows the pattern of a fill. In the view the highlight is a thin line
   on each cord: at a usual zoom an area has one flat color, and a fill
   looks like a hatching.
3. **The thread.** Rayon and polyester shine, cotton does not, metallic
   thread glitters, and a 60 weight thread is thinner than a 40 weight
   thread. In the view all threads are the same.
4. **The shape of a stitch.** A stitch comes out of the fabric, lies
   over what is under it and goes into the fabric. Short stitches stand
   up more than long ones. In the view a stitch is a straight stick.
5. **The fabric.** A fabric has a surface: threads over and under, the
   loops of a towel, the grain of leather. Thin stitches sink into a
   fabric with a pile. In the view the fabric is a flat pattern and the
   stitches lie on it as on paper.
6. **What is not thread.** The fabric of an appliqué, puff foam,
   rhinestones, the holes of cutwork, lace with no fabric. Each has its
   own flat drawing now, or none.
7. **Small zoom.** At the zoom that shows the whole design, the stitches
   are thin lines. Their sum has the wrong brightness, and fine patterns
   flicker when the view moves.
8. **Colors.** The picture mixes colors as numbers of the screen, not as
   light. The edges of a thread are too dark and the mix of two threads
   is wrong.
9. **The point of view.** The view is from straight above. The light is
   a setting in a window. A person turns a sew-out in the light to check
   it.
10. **No proof.** No test says how near the view is to a sew-out, and no
    test finds a change of the look.

### What stays

- **A picture needs no graphics card.** `emb-cli`, the tests and the
  software renderer of the program (ADR 0094) make the same pictures.
  The engine of the view is in the engine crate `emb-render`: no UI
  crate, no `unsafe` code, no C library (rules 5 and 9).
- **The same picture on each system** for the same input (trigonometry
  through `emb_geom::math`, ADR 0003), so a test can compare pictures.
- **The view does not change the design**, the stitch plan or a machine
  file. It shows the plan of `AppState::shown_plan`.
- **Editing is never slower than now.** While the new view is made, the
  look of today shows. A slow computer can keep the look of today.
- The flat thread look and the stitch view stay as they are.
- The behavior of the reference product is described in our own words.
  Photos of the sew-outs of the owner are for comparison only and do not
  go in the repository.

## The design in short

The main decision of the phase (an ADR in Milestone 3). ADR 0040 kept the
canvas on meshes that the processor makes for each frame. That limits
the look to what a few colored points for each stitch can show, and its
cost grows with the stitches of each frame. The upgrade makes a
**surface** of the embroidery one time, and lights it.

1. **Strands.** The stitch plan as the view needs it: for each stitch
   its thread (color, kind, thickness), its object, and what it is
   (underlay, top stitch, run, travel).
2. **The surface.** The design is cut into tiles (256 × 256 pixels at
   each zoom level). For each pixel of a tile the surface has a height,
   a direction of the surface, the direction of the thread, a base color
   and a material. The engine lays each stitch, in sew order, on what is
   under it.
3. **The light.** A second step makes the picture of a tile from its
   surface and the light: sheen by thread direction, shadows from the
   heights, and less light in the gaps. A change of the light needs only
   this step.
4. **Zoom levels.** A tile of a smaller zoom is made from the surface of
   four tiles of the larger zoom, not from their pictures. So an area of
   satin has the right brightness when a stitch is smaller than a pixel.
5. **One engine for all pictures.** The canvas shows the tiles as
   textures. The design picture, the product view, the turntable, the
   approval sheet and the thumbnails use the same tiles.
6. **The processor first.** Threads of the processor make the tiles in
   the background, as the stitch plan is made (ADR 0077). A graphics card
   can do the light step later (Milestone 9); it is never needed.

Budgets for the development computer (16 cores). Milestone 1 measures
the same numbers on a small computer and sets its budgets.

| What | Budget |
| --- | --- |
| Pan and zoom, tiles made, 1,000,000 stitches | 60 frames in a second; 30 with the software renderer |
| First full view of a design with 100,000 stitches | 0.3 s; the look of today shows until then |
| New tiles after a change of one object, design with 200,000 stitches | 0.1 s |
| A change of the light, window of 1920 × 1080 | 0.1 s |
| A design picture of 4000 × 4000 pixels, 100,000 stitches | 2 s |
| Memory of the tiles of all open designs | 256 MB by default (ADR 0077) |

This part of P18 has nine milestones.

## Milestone 1: measures and a reference set (S to M)

Nothing in the view changes. The work gets its tools.

1. **A reference set**: twelve small designs of the corpus that show
   each fault of the list: satin columns at eight angles, a fill with a
   border on it, underlay under a satin, lettering at 6 mm and at 60 mm,
   a dense fill over a fill, puff, an appliqué, a motif fill, a run, a
   blend of two colors, a towel fabric, a dark fabric.
2. **`emb-cli view-bench`** makes the pictures of the set at three zooms
   and prints the time and the memory of each step. A golden test keeps
   a hash of each picture (as the pictures of the windows, ADR 0110) and
   writes the pictures that changed.
3. **A picture compare** for the developer: two pictures side by side
   and their difference, and a number for the difference of the
   brightness of each area. It also takes a photo of a sew-out of the
   same design (scaled and placed by hand; the photo stays out of the
   repository).
4. **The numbers of today**: the times of the canvas and of the
   pictures for 10,000, 100,000 and 1,000,000 stitches, on the
   development computer, on a small computer and with the software
   renderer.
5. **The owner sews the reference set** on two fabrics and takes photos
   in the same light (one light from the top left, a camera from straight
   above).

**Exit test:** `emb-cli view-bench` runs in CI; the report of the
milestone has the numbers of today and a picture of each fault next to
its photo.

## Milestone 2: light as light, and threads with a kind (M)

1. **Linear light.** All mixing of colors in `emb-render` is done in
   linear light, and the result goes to the colors of the screen at the
   end. The flat look keeps its colors (a test compares them).
2. **A thread has a kind and a thickness.** `Thread::kind`: rayon,
   polyester, cotton (matte), metallic, wool or acrylic (thick, matte),
   and glow. `Thread::weight`: the weight number of the thread (40 by
   default); the engine has the width of each weight in mm. A project
   file of an earlier version reads with the defaults.
3. **The thread charts** give the kind and the weight of their lines
   where the maker publishes them, each value with its source. A chart
   line with no source has the default.
4. **The Threads panel** shows and changes the kind and the weight of a
   thread. A change of the kind changes only the view; a change of the
   weight also gives a note if the density of the objects of this thread
   does not fit (the density is for a 40 weight thread).
5. **The model of today uses them**: the width of a stitch from the
   weight, and a highlight by kind. This is the first change that the
   user sees.

**Exit test:** a design with a 40 weight and a 60 weight thread shows two
widths; a matte thread has no highlight; the pictures of the reference
set change only at the edges of threads (linear light), and the report
shows them next to the pictures before.

## Milestone 3: the surface (L)

The largest step. An ADR records the design.

1. **Strands** (`emb_render::strand`): the stitch plan with the thread,
   the object and the role of each stitch. The plan gives the role
   (underlay, top, travel) where it has it; a machine file of another
   program gives none.
2. **A stitch as a shape**: it comes out of the fabric, goes up to its
   height in a short curve, and goes down at its end. A short stitch is
   an arch. Its section is a flat oval (a thread under tension is not
   round), wider for a thick thread.
3. **The height field**: the engine lays the stitches in sew order. The
   height of a stitch at a place is the height of what is under it there,
   plus its own thickness. Thread presses thread: a stack of many layers
   is lower than their sum (the measure of the density map, ADR 0044,
   gives the layers).
4. **The shape of a column**: the stitches of a satin make one round
   body across its width, and the stitches at its edges go into the
   fabric at a steep angle. Underlay under a column makes it higher.
5. **Tiles**: the surface of a tile comes from the strands that touch
   it. Tiles are made on all processors. The same input gives the same
   bytes on each system.
6. **The first light**: the light of today, on the directions of the
   surface in place of the cord of each stitch, so the milestone has a
   picture to look at.

**Exit test:** in the pictures of the reference set, a border on a fill
is higher than the fill, a satin with underlay is higher than one with
none, and puff is the highest. A test reads these heights from the
surface. The surface of a design with 100,000 stitches at 10 pixels for
each mm is made in less than 1 s.

## Milestone 4: the light on the surface (L)

1. **Sheen by the direction of the thread**: a model of light on fibers
   (a highlight that is long across the thread and short along it), with
   the strength and the width of the highlight from the kind of the
   thread. An area of satin is light or dark by its angle to the light.
2. **Shadows from the heights**: a high part puts a soft shadow on a low
   part and on the fabric, with a length from the height of the light.
   The black copy of each stitch goes.
3. **Less light in the gaps**: the places between stitches and the
   needle holes are darker (from the height field), so a fill has its
   texture.
4. **The twist of the plies** and small random changes of each thread
   (width, brightness) at a large zoom. The values come from the place of
   the stitch, so the picture is the same each time.
5. **Zoom levels**: a level is made from the surface of the level under
   it: the mean height, the mean color, and the spread of the directions
   of the threads. The light uses the spread: an area with all threads in
   one direction keeps its sheen, a mixed area is matte.
6. **Smooth edges**: each pixel has the part of it that the thread
   covers; no steps at the edges of a thread, and no flicker of thin
   lines.
7. **The bright end of the light**: a strong highlight does not go to
   flat white; a tone curve keeps the color of the thread.

**Exit test:** the eight satin columns of the reference set have eight
different brightness values in the order of the photo; the mean
brightness of each area of each design is the same within 3 % at each
zoom level; the developer's compare of each reference picture with its
photo is in the report, and the owner says of each one if it is "like
the sew-out", "near" or "not like it".

## Milestone 5: the canvas on tiles (L)

1. **A tile cache** in the app: the tiles of each open design by plan,
   level, place and look. A budget of memory (ADR 0077); the tiles that
   were not used for the longest time go first.
2. **Threads in the background** make the tiles that the view needs:
   first the level of the screen, in the middle of the window first,
   then one level each way. The canvas shows a tile of another level
   until the right one is there, and the look of today until the first
   tile is there.
3. **A change of the design** makes only the tiles again that the
   changed stitches touch (the stitch cache of each object says which
   objects changed). A drag shows the look of today for the moving
   objects and the tiles for the rest.
4. **The stitch player** and the travel through the design (stitch by
   stitch) show the tiles up to a stitch: the surface keeps, for each
   pixel, the number of the last stitch that changed it at some steps of
   the plan, and the stitches after the last step are drawn as today.
5. **The other marks stay sharp**: selection, needle points, jumps,
   function marks and the density map are drawn over the tiles as today.
6. **Views**: the split view, the other views and the overview use the
   same cache.

**Exit test:** the budgets of the table for pan and zoom, the first
view and a change of one object, on the development computer and on the
small computer of Milestone 1 (with its budgets); the smoke test of
each corpus design passes with the new view; no picture of the canvas
shows a missing tile after the work is done.

## Milestone 6: the fabric (M)

1. **A surface for each fabric**: a height and a direction for each
   place of the fabric, not only a brightness: the over and under of a
   weave at its real size, the ribs of a knit, the cells of piqué, the
   loops of terry, the hair of fleece, the grain of leather. The light
   of Milestone 4 lights it. The size of the weave comes from the
   fabric of the design.
2. **Stitches sink in.** On a fabric with a pile (towel, fleece) a thin
   stitch is partly under the pile, and a topping (the fabric of the
   design names it) keeps it up. The view shows why a laydown fill or a
   topping is needed.
3. **Needle holes**: a small dark dent where the thread goes in, larger
   on leather and vinyl.
4. **The fabric of the product**: the product view (ADR 0101) uses the
   same fabric surface, so a design looks the same on the canvas and on
   the product.
5. **More colors of a fabric**: a heather (two colors of fiber) and a
   denim (a dark and a light thread) from two colors.

**Exit test:** the reference design on towel shows thin lettering that
is partly lost, and the same design with the topping setting shows it
clear, as the two photos of the owner show; each of the seven fabrics
has a picture at three zooms in the report.

## Milestone 7: what is not thread (M)

1. **Appliqué fabric**: a piece with a thickness and a cut edge, with
   the fabric surface of its kind, under its cover stitches.
2. **Puff**: the foam lifts the satin over it; the ends of the column
   are closed by the cap stitches (ADR 0050).
3. **Rhinestones** (ADR 0039): stones with facets that take the light,
   at their real size.
4. **Cutwork** (ADR 0051) and **lace** (ADR 0048): a hole shows what is
   behind the fabric (a background color or the product); lace has no
   fabric at all.
5. **Cross stitch** (ADR 0038): two crossed stitches on an even weave.
6. **Trapunto and quilting**: the fabric is higher between the lines
   of stitches.
7. **Metallic and glow thread**: small bright points that change with
   the light; a "lights off" look for glow thread.

**Exit test:** one reference design for each item, each with a picture
at two zooms in the report; the owner compares the ones that have a
sew-out (appliqué, puff, cross stitch, rhinestones on a transfer).

## Milestone 8: the viewer (M to L)

The view becomes a tool to check a design before it is sewn.

1. **Tilt and turn**: a mode of the canvas (and a window of its own)
   that shows the embroidery from an angle, with its real heights, or
   with the heights made larger to see them. The engine makes the
   picture from the height field with the processor; while the view
   moves, it uses fewer pixels.
2. **The light by hand**: drag the light over the design with a key
   held. Light presets: from the top left (the default), flat light from
   above, a low light from the side (it shows each height and each gap),
   and two lights.
3. **A magnifier**: a round window at the pointer with the thread at
   20 times.
4. **Compare**: the split view with the stitch view on one side and the
   3D view on the other, with the same place and zoom; and a slider
   between the view before and after a change of a setting.
5. **Check views** that use the surface: the places where the fabric
   shows through a fill (from the heights and the gaps, not only from the
   density), and the places where the thread is higher than a limit
   (a risk of thread breaks).
6. **The turntable and the product view** (ADR 0108) show the embroidery
   with its height and its sheen on the product; the light of the product
   and the light of the stitches are one light.

**Exit test:** the owner checks a design with the low light and the
tilt, finds a place where the fabric shows through, changes the density
and sees the place closed; the tilt view of a design with 100,000
stitches moves at 20 frames in a second or more on the development
computer.

## Milestone 9: all pictures, quality settings and the graphics card (M)

1. **All pictures of the program** use the engine: the design picture,
   `emb-cli output`, the approval sheet and the product page, the
   library pictures, the pictures in the file manager (ADR 0046, a fast
   level), the kiosk, the wizard preview and the font previews. The
   worksheets keep the flat look (they are for the operator).
2. **Export**: a picture up to 600 dots for each inch with no
   background, for a web shop; a moving picture of the design sewing
   stitch by stitch (as the turntable writes its pictures).
3. **Quality**: **Fast** (the look of today, no tiles), **Good**
   (tiles, no shadows from the heights) and **Best**. The program selects
   the first value from a short measure at the first start, and the user
   can change it. The software renderer starts with Fast.
4. **The graphics card, if it helps**: after the measures of Milestone 5
   the owner decides. If the light step or the tilt view is too slow on
   the processor, a shader does the light step from the same surface
   (a paint callback of the UI library). The pictures of the two ways
   are the same within a small limit (a test), and the program works
   with no graphics card as before.
5. **Help**: a topic for the view (what it shows, the light, the tilt,
   the check views, the kinds of thread), in the four languages, and the
   entries of What Is New.
6. **An audit of the part**, as for each phase: a probe with wrong
   values (a thread weight of 0, a design of 1 km), the memory with ten
   open designs, the rules of the spec.

**Exit test:** five test users get twelve pairs of a photo of a sew-out
and a picture of the view (the same design, the same size) and say which
one is the photo and what gave it away; the list of what gave it away is
the list of the work that is left. The budgets of the table hold, and
the program starts and edits with the software renderer.

## Order and size

Milestones 1 to 5 are the engine and must be done in this order.
Milestones 6 and 7 can come in each order after Milestone 4. Milestone 8
needs Milestones 4 and 5. Milestone 9 comes last. The largest steps are
Milestones 3, 4 and 5; the whole part is of the size of a large phase (L
to XL).

After Milestone 2 the user sees thread widths and kinds. After Milestone
4 the pictures have the new look. After Milestone 5 the canvas has it.

## Risks

- **Speed on a small computer.** The processor makes the tiles. The
  budgets of the small computer are set in Milestone 1, and the quality
  setting Fast is the look of today.
- **A look that is not like a sew-out.** A model with more parts is not
  always more true. Each milestone compares with photos of the same
  design, and the owner gives the answer.
- **Thread data.** The makers of some thread charts do not publish the
  kind and the weight of each line. The default (polyester or rayon by
  the brand line, 40 weight) is then a guess, and the chart says so.
- **Memory.** A surface has more data than a picture (about 12 bytes for
  each pixel before the light). The cache has a budget, and a level that
  is not used is not kept.
- **Two renderers.** If the graphics card gets the light step, two ways
  must give the same picture. Milestone 9 does this only if the measures
  ask for it.

## Not planned

- The pull of the stitches on the fabric (gaps at the ends of a column,
  a puckered fabric). The view shows the design as it is digitized. A
  simulation of the fabric is a proposal for later.
- A view for virtual reality, and ray tracing.
- A film of the design with a moving camera (the moving picture of the
  stitches and the turntable stay).
- Sequins, beads and chenille: they wait for a tester with such a
  machine (P9 Milestones 6 and 7).
