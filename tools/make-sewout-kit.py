# Makes the sew-out kit of phase P17 (roadmap, "Review of P17", lane 1): the
# designs of the open exit tests of the phase, with the files for the
# machine, a worksheet and a picture of each one, and a checklist.
#
# The kit is made again after each change of the fonts (the test sheets
# show the letters of the fonts as they are now).
#
# In the kit:
#   tuned/     the test sheets of the ten tuned fonts at their smallest,
#              middle and largest size (Milestones 4 and 5)
#   routed/    three fonts that are routed and not tuned, at their middle
#              size (the fonts after ADR 0116)
#   fill/      fill fonts at 15, 40 and 100 mm (ADR 0117)
#   large/     fonts with satin letters at the largest size of their
#              columns and above it, where their letters are fills
#              (ADR 0120)
#   shaped/    a word in Arabic, in Devanagari and in a script font
#              (Milestone 6)
#   content/   motifs, patterns, stamps, borders and clipart of the content
#              families (Milestone 8)
#   CHECKLIST.md
#
# Use:  python tools/make-sewout-kit.py [<folder>]
#       (the folder: out/p17-sewout if not given; build emb-cli first:
#       cargo build --release -p emb-cli)

import datetime
import os
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
EXE = os.path.join(ROOT, "target", "release", "emb-cli.exe" if os.name == "nt" else "emb-cli")
FONTS = os.path.join(ROOT, "data", "fonts", "embroidery")
MACHINE = "husqvarna_topaz20"
# A sheet fits the hoop of 260 x 200 mm with some space.
FIT = (190.0, 250.0)

# The tuned fonts (the same list as in tools/make-embroidery-fonts.py).
TUNED = [
    ("block-sans", "Block Sans"),
    ("friendly-serif", "Friendly Serif"),
    ("pacifico", "Pacifico"),
    ("atkinson-hyperlegible", "Atkinson Hyperlegible"),
    ("varsity", "Varsity"),
    ("bebas-neue", "Bebas Neue"),
    ("soft-round", "Soft Round"),
    ("geo-slab", "Geo Slab"),
    ("brush-hand", "Brush Hand"),
    ("vintage-script", "Vintage Script"),
]
ROUTED = [("anton", "Anton"), ("archivo-black", "Archivo Black"), ("poppins", "Poppins")]
FILL = [("slab-heavy-fill", "Slab Heavy Fill"), ("varsity-fill", "Varsity Fill")]
# Fonts with satin letters for the test of the fill letters above the
# largest size of their columns (ADR 0120).
LARGE_FONTS = [("block-sans", "Block Sans"), ("poster-bold", "Poster Bold"), ("pacifico", "Pacifico")]

# The texts of a sheet, the longest first: the tool takes the first one
# that fits the hoop. None: the basic characters of the font.
SMALL = [None, "ABEGKMRSW aegkmnsxy 2358", "ARSaeg58"]
MIDDLE = ["ABEGKMRSW aegkmnsxy 2358", "ARSWaegms258", "ARSaeg58", "RSag8"]
LARGE = ["ARSag8", "RSag8", "RSa8", "Ra8", "Ra", "R"]


def run(args):
    done = subprocess.run([EXE] + args, capture_output=True, text=True, encoding="utf-8")
    if done.returncode != 0:
        sys.exit("emb-cli %s: %s" % (" ".join(args[:2]), done.stderr.strip() or done.stdout.strip()))
    return done.stdout


def size_range(slug):
    """The size range of a font, from its file."""
    with open(os.path.join(FONTS, slug + ".embfont"), encoding="utf-8") as f:
        text = f.read(4000)
    lo = float(re.search(r'"min_height_mm":([0-9.]+)', text).group(1))
    hi = float(re.search(r'"max_height_mm":([0-9.]+)', text).group(1))
    # A font with fill letters for large lettering (ADR 0120) goes to
    # 300 mm; its columns end where the fill letters start.
    fills = re.search(r'"fill_letters":\{"from_mm":([0-9.]+)', text)
    if fills:
        hi = float(fills.group(1))
    return lo, hi


def sizes(slug):
    """The smallest, the middle and the largest size of a font (as
    emb_letter::sheet::sizes)."""
    lo, hi = size_range(slug)
    mid = round((lo * hi) ** 0.5 * 2) / 2
    return lo, min(max(mid, lo), hi), hi


def text_of(mm):
    return ("%.1f" % mm).rstrip("0").rstrip(".")


def sheet(slug, project, size_list, texts):
    """Writes the test sheet of a font: the first of `texts` that fits the
    hoop. Returns the text (None: the basic characters)."""
    for text in texts:
        args = ["font-sheet", os.path.join(FONTS, slug + ".embfont"), project]
        args += ["--sizes", ",".join(text_of(s) for s in size_list)]
        args += ["--chars", "basic"] if text is None else ["--text", text]
        out = run(args)
        m = re.search(r"([0-9]+) x ([0-9]+) mm", out)
        if m and float(m.group(1)) <= FIT[0] and float(m.group(2)) <= FIT[1]:
            return text
    sys.exit("%s at %s mm: no text fits the hoop" % (slug, size_list))


def output(project, folder):
    """The machine files (VP3 and DST), the worksheet and the picture of a
    project. Returns the numbers of the design."""
    run(["output", project, "--dir", folder, "--machine", MACHINE, "--format", "vp3,dst", "--worksheet", "--picture"])
    text = run(["stitch", project, "--machine", MACHINE])
    num = lambda key: int(re.search(r"^%s: ([0-9]+)" % key, text, flags=re.M).group(1))
    m = re.search(r"size ([0-9.]+) x ([0-9.]+) mm", text)
    return {
        "stitches": num("stitches"),
        "trims": num("trims"),
        "colors": num("colors"),
        "size": "%.0f x %.0f mm" % (float(m.group(1)), float(m.group(2))),
    }


def row(name, what, n):
    return "| `%s` | %s | %s | %d | %d | %d |" % (name, what, n["size"], n["stitches"], n["colors"], n["trims"])


HEAD = "| Design | What | Size | Stitches | Colors | Trims |\n| --- | --- | --- | --- | --- | --- |"


def main():
    kit = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "out", "p17-sewout"))
    if not os.path.isfile(EXE):
        sys.exit("build emb-cli first: cargo build --release -p emb-cli")
    work = tempfile.mkdtemp(prefix="emb-kit-")
    rows = {"tuned": [], "routed": [], "fill": [], "large": [], "shaped": [], "content": []}

    def make(group, name, what, project):
        folder = os.path.join(kit, group)
        os.makedirs(folder, exist_ok=True)
        named = os.path.join(work, name + ".embproj")
        if project != named:
            shutil.copyfile(project, named)
        rows[group].append(row(name, what, output(named, folder)))
        print("made %s/%s" % (group, name))

    for slug, family in TUNED:
        lo, mid, hi = sizes(slug)
        for label, size, texts in [("small", lo, SMALL), ("middle", mid, MIDDLE), ("large", hi, LARGE)]:
            project = os.path.join(work, "%s %s.embproj" % (slug, label))
            text = sheet(slug, project, [size], texts)
            what = "%s at %s mm: %s" % (family, text_of(size), "the basic characters" if text is None else '"%s"' % text)
            make("tuned", "%s %s" % (slug, label), what, project)
    for slug, family in ROUTED:
        _, mid, _ = sizes(slug)
        project = os.path.join(work, "%s middle.embproj" % slug)
        text = sheet(slug, project, [mid], MIDDLE)
        make("routed", "%s middle" % slug, '%s at %s mm: "%s"' % (family, text_of(mid), text), project)
    for slug, family in FILL:
        lo, _ = size_range(slug)
        project = os.path.join(work, "%s small.embproj" % slug)
        text = sheet(slug, project, [lo, 40.0], ["RSaeg58", "RSag8", "RSa8"])
        make("fill", "%s small" % slug, '%s at %s and 40 mm: "%s"' % (family, text_of(lo), text), project)
    for slug, family in LARGE_FONTS:
        _, hi = size_range(slug)
        above = round(hi * 1.15)
        project = os.path.join(work, "%s switch.embproj" % slug)
        text = sheet(slug, project, [hi, above], ["Rag", "Ra", "R"])
        what = '%s at %s mm (satin columns) and at %s mm (fills): "%s"' % (family, text_of(hi), text_of(above), text)
        make("large", "%s switch" % slug, what, project)
    project = os.path.join(work, "slab-heavy-fill large.embproj")
    text = sheet("slab-heavy-fill", project, [100.0], ["Ra", "R"])
    make("fill", "slab-heavy-fill large", 'Slab Heavy Fill at 100 mm: "%s"' % text, project)

    corpus = os.path.join(work, "corpus")
    os.makedirs(corpus)
    run(["corpus", corpus])
    make("shaped", "shaped-text", "Arabic, Devanagari and a script font, 20 mm", os.path.join(corpus, "shaped-text.embproj"))
    make("content", "content-families", "12 motifs, 5 split patterns, 5 stamps, 4 borders, 3 clipart items", os.path.join(corpus, "content-families.embproj"))
    make("content", "content-clipart", "5 clipart items, each in a border", os.path.join(corpus, "content-clipart.embproj"))
    make("fill", "embroidery-fonts-8", 'The ten fill fonts at their smallest size: "Team 26"', os.path.join(corpus, "embroidery-fonts-8.embproj"))
    shutil.rmtree(work, ignore_errors=True)

    lines = CHECKLIST.replace("{date}", datetime.date.today().isoformat())
    for group, table in rows.items():
        lines = lines.replace("{%s}" % group, HEAD + "\n" + "\n".join(table))
    with open(os.path.join(kit, "CHECKLIST.md"), "w", encoding="utf-8", newline="\n") as f:
        f.write(lines)
    print("wrote %s: %d designs" % (kit, sum(len(t) for t in rows.values())))


CHECKLIST = """# P17 sew-out kit: checklist

Machine: Husqvarna Viking Topaz 20 (the `.vp3` files), hoop 260 x 200 mm.
Each design also has a `.dst` file for the Tajima. Medium woven fabric
with a medium tear-away stabilizer, if a design does not say another one.
Each design has a worksheet (PDF) and a picture (PNG).

Made on {date}. Make the kit again after a change of the fonts:
`python tools/make-sewout-kit.py`.

Nothing of phase P17 is sewn yet. The limits of the phase (the quality
scores, the size ranges, the travel runs under the stitches) come from
pictures. This kit is for the open exit tests of Milestones 4, 5, 6 and
8, and for the fonts after Milestone 7.

**The shortest session** (about ten designs): `block-sans small`,
`block-sans middle`, `block-sans large`, `pacifico middle`,
`friendly-serif middle`, `anton middle`, `slab-heavy-fill small`,
`shaped-text`, `content-families`, `content-clipart`.

Send a photo of each sewn design, with the answers to the questions.

## 1. Tuned fonts (`tuned/`, Milestones 4 and 5)

The ten tuned fonts, each at its smallest, middle and largest size. The
exit test of Milestone 5 is all thirty designs on the Topaz 20 and on the
Tajima; the three sizes of Block Sans and the middle size of a script and
of a serif font show the most.

{tuned}

| Check | Look for |
| --- | --- |
| Travel runs | The needle goes from one stroke to the next one under the stitches. No thread of a run shows beside a column or through it. Look again with a light thread on a dark fabric, if you can. |
| Joins | A join under the strokes can be one stitch of up to 9.8 mm. No loose stitch shows between two strokes. |
| Cut columns | The stem of "H" and of "E" is cut where a bar meets it. Does the cut show as a line across the stem? |
| Trims | The trims in a letter are the trims of the worksheet: the dot of "i" and "j" and the two parts of a mark have a trim, the other letters have none. |
| Corners | The corners of "E", "L", "Z", "N" and "M" have stitches to the edge: no gap, and no hard lump of two layers. |
| Smallest size | The thin strokes sew clean (no thread break, no hole in the fabric) and the letters are easy to read. Is the smallest size of the range too small, or can it be smaller? |
| Largest size | The long satin stitches are split. Are they loose? Is the largest size of the range too large? |
| Script fonts | The letters of a word join with no trim (Pacifico, Brush Hand, Vintage Script). |
| The shapes | Mark each letter that looks wrong. A letter correction in Lettering > Alphabets can correct it (Milestone 4). |

## 2. Routed fonts that are not tuned (`routed/`)

All fonts are routed now, but only the ten tuned fonts were looked at
letter by letter. These three show if the others are as good.

{routed}

| Check | Look for |
| --- | --- |
| The same checks | Travel runs, joins, cut columns, trims and corners, as for the tuned fonts. |
| Compare | Is a font that is not tuned worse than a tuned font? In which letters? |

## 3. Fill fonts (`fill/`, ADR 0117)

The letters of a fill font are fills, not satin columns. They are for
large text and for heavy fonts.

{fill}

| Check | Look for |
| --- | --- |
| The edge | The edge of a fill letter at 15 mm is rougher than the edge of a satin letter. Is it acceptable? From which size does a fill letter look good? |
| Holes | The holes of "a", "e", "g", "R" and "8" stay open at 15 mm. |
| Narrow parts | The serifs and the thin parts of a letter at 15 mm have stitches and no hole in the fabric. |
| The large letter | The 100 mm letter stays flat (no puckers), and its edge is where the worksheet shows it. |
| Underlay | No underlay shows at the edge of a letter. |
| The ten fonts | `embroidery-fonts-8` has each fill font at its smallest size. Which fonts are not good at this size? |

## 3b. Large lettering in a font with satin letters (`large/`, ADR 0120)

A font with satin letters also has each letter as a fill. Up to the
largest size of its columns the letters are satin columns; above it they
are fills, and the font goes to 300 mm. Each design has the same letters
at the largest size of the columns and at a size 15 % above it.

{large}

| Check | Look for |
| --- | --- |
| The two sizes | Which of the two looks better: the large satin letters with their long, split stitches, or the fill letters? |
| The place of the change | Is the change to fills at the right size? Must it be at a smaller size (where the satin stitches get loose) or at a larger one? |
| The fill letters | The same checks as for the fill fonts: the edge, the holes, the narrow parts, no underlay at the edge. |

## 4. Shaped text (`shaped/`, Milestone 6)

Three words, 20 mm high: "salam" in Arabic (Tajawal), "kshatriya" in
Devanagari (Poppins), and "office tt" in a script font (Pacifico).

{shaped}

| Check | Look for |
| --- | --- |
| Arabic | The letters join as in the picture, and the word reads from right to left. |
| Devanagari | The top line of the word is one line. The vowel sign that is written before its consonant is before it. The two conjuncts are the shapes of the picture. |
| Script font | The "ff" of "office" is one shape. The two "t" of "tt" have different shapes. |
| Stitches | These letters are made when the text is typed (they are not embroidery fonts). Are the columns clean? Where are there trims? |

## 5. Content families (`content/`, Milestone 8)

`content-families`: twelve motif runs (on the worksheet from the top:
Double wave, Sharp zigzag, Key, Arcade, Daisy, Vine, Shell, Snowflake,
Cross stitch, Stairs, Double heart, and a star with seven points), five
fills with split patterns (Hearts, Lattice, Stars, Long brick, and
flowers with six petals), five fills with stamps (Snowflake, Sunburst,
Nested hexagons, Clover, Tight spiral), four borders (Satin hexagon,
Greek key, Double line diamond, Daisies on an octagon) and three clipart
items (Gear, Flower, Sun).

`content-clipart`: five clipart items (Cog, Four-leaf clover, Tall crown,
Pointed shield, Tall tree), each in a border (Satin octagon, Pearls,
Zigzag, Rope circle, Hexagon chain). This design has many thread changes.

{content}

| Check | Look for |
| --- | --- |
| Motifs | Each motif has the shape of the worksheet. Mark a motif whose stitches are too long, too short or too dense. |
| Split patterns | The picture of each pattern shows in the fill at arm's length. Do the single needle points between the shapes (hearts, stars, flowers) look like noise? |
| Long stitches | No stitch in a pattern fill is loose (the longest is 6 mm). |
| Stamps | The lines of each stamp show in the fill. |
| Borders | The corners of the Greek key and of the zigzag border are clean. The motifs of a border do not lie on each other where the border closes. |
| Clipart | The fills are flat, the outlines lie on the edges of the fills, and no fabric shows between the rays and the disc of the sun. |

## 6. Not a sew-out

- **Update check (Milestone 1):** with a real release on the page of the
  program, Help > Check for Updates shows the notice in a program with an
  older version number.
- **Milestone 3** (the industrial machine and the reports of testers) is
  set aside; this kit does not close it.
"""


if __name__ == "__main__":
    main()
