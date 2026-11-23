# Makes the sew-out kit of phase P21 (letter shapes, Milestone 6,
# docs/design/p21-plan.md): the letters of the fonts made again with the
# stroke finder of P21 (the outline is the edge of each column, one way to
# end a column, the split of the outline into strokes, the sew order at
# joints), with the files for the machine, a worksheet and a picture of
# each one, and a checklist.
#
# Make the fonts first (python tools/make-embroidery-fonts.py
# <google-fonts-folder>): the kit uses the fonts of data/fonts/embroidery
# and the Japanese fonts of target/font-packages/japanese.
#
# In the kit:
#   tuned/    the test sheets of the ten tuned fonts at their smallest,
#             middle and largest size (as the kit of P17)
#   joints/   the letters whose joints the split changed most, and the
#             letters with the most faults of the bench of 2026-11-23, in
#             the four fonts of the letter fault bench, at their middle size
#   kanji/    crossings and junctions of kanji at 25 mm, in the Japanese
#             embroidery font Noto Sans JP
#   shaped/   the corpus design shaped-text (its Arabic word is routed from
#             the right now)
#   CHECKLIST.md
#
# Use:  python tools/make-sewout-kit-p21.py [<folder>]
#       (the folder: out/p21-sewout if not given; build emb-cli first:
#       cargo build --release -p emb-cli)

import datetime
import importlib.util
import os
import shutil
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
# The helpers and the lists of the kit of P17.
_spec = importlib.util.spec_from_file_location("kit17", os.path.join(ROOT, "tools", "make-sewout-kit.py"))
kit17 = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(kit17)

JP_FONT = os.path.join(ROOT, "target", "font-packages", "japanese", "noto-sans-jp.embfont")

# The four fonts of the letter fault bench, and the texts of their sheets
# (the first that fits the hoop): the letters whose joints the split
# changed most ("N", "M", "W", "Z", "k", "x"), then the letters with the
# most faults of each font in the bench of 2026-11-23.
JOINTS = [
    ("block-sans", "Block Sans", ["NMWZkx rAEK", "NMWZkx r", "NMWZkx"]),
    ("anton", "Anton", ["NMWZkx tGcSs369", "NMWZkx tGcS", "NMWZkx"]),
    ("pacifico", "Pacifico", ["NMWZkx wm4", "NMWZkx", "NMkx"]),
    ("varsity", "Varsity", ["NMWZkx XIH", "NMWZkx X", "NMWZkx"]),
]
KANJI = "十木林東龍本中日"


def main():
    kit = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "out", "p21-sewout"))
    if not os.path.isfile(kit17.EXE):
        sys.exit("build emb-cli first: cargo build --release -p emb-cli")
    work = tempfile.mkdtemp(prefix="emb-kit-")
    rows = {"tuned": [], "joints": [], "kanji": [], "shaped": []}

    def make(group, name, what, project):
        folder = os.path.join(kit, group)
        os.makedirs(folder, exist_ok=True)
        named = os.path.join(work, name + ".embproj")
        if project != named:
            shutil.copyfile(project, named)
        rows[group].append(kit17.row(name, what, kit17.output(named, folder)))
        print("made %s/%s" % (group, name))

    for slug, family in kit17.TUNED:
        lo, mid, hi = kit17.sizes(slug)
        for label, size, texts in [("small", lo, kit17.SMALL), ("middle", mid, kit17.MIDDLE), ("large", hi, kit17.LARGE)]:
            project = os.path.join(work, "%s %s.embproj" % (slug, label))
            text = kit17.sheet(slug, project, [size], texts)
            what = "%s at %s mm: %s" % (family, kit17.text_of(size), "the basic characters" if text is None else '"%s"' % text)
            make("tuned", "%s %s" % (slug, label), what, project)
    for slug, family, texts in JOINTS:
        _, mid, _ = kit17.sizes(slug)
        project = os.path.join(work, "%s joints.embproj" % slug)
        text = kit17.sheet(slug, project, [mid], texts)
        make("joints", "%s joints" % slug, '%s at %s mm: "%s"' % (family, kit17.text_of(mid), text), project)
    if os.path.isfile(JP_FONT):
        project = os.path.join(work, "noto-sans-jp kanji.embproj")
        kit17.run(["font-sheet", JP_FONT, project, "--sizes", "25", "--text", KANJI, "--row", "180"])
        make("kanji", "noto-sans-jp kanji", "Noto Sans JP (font package) at 25 mm: %s" % KANJI, project)
    else:
        print("no Japanese font: make it first (tools/make-embroidery-fonts.py)")
    corpus = os.path.join(work, "corpus")
    os.makedirs(corpus)
    kit17.run(["corpus", corpus])
    make("shaped", "shaped-text", "Arabic, Devanagari and a script font, 20 mm", os.path.join(corpus, "shaped-text.embproj"))
    shutil.rmtree(work, ignore_errors=True)

    lines = CHECKLIST.replace("{date}", datetime.date.today().isoformat())
    for group, table in rows.items():
        lines = lines.replace("{%s}" % group, kit17.HEAD + "\n" + "\n".join(table))
    with open(os.path.join(kit, "CHECKLIST.md"), "w", encoding="utf-8", newline="\n") as f:
        f.write(lines)
    print("wrote %s: %d designs" % (kit, sum(len(t) for t in rows.values())))


CHECKLIST = """# P21 sew-out kit: checklist

Made on {date} by `tools/make-sewout-kit-p21.py`, with the fonts made
again in P21 Milestone 6. Machine: Husqvarna Viking Topaz 20 (the `.vp3`
files; DST files too), hoop 260 x 200 mm. Sew each design on a medium
woven fabric with a tear-away stabilizer, as a normal job.

The exit test of P21: no spikes, bulges or open corners.

## The tuned fonts (three sizes each)

{tuned}

## Joints and the letters with the most faults

{joints}

## Kanji

{kanji}

## Text from right to left

{shaped}

## What to look at

1. **Spikes**: a needle point out of the letter, at the end of a column
   or at a corner (before: the feet of "A", the top of "M", Pacifico "w",
   "m", "4").
2. **Bulges**: a column that is wider at a junction than its stroke
   (before: Anton "M", "W", "t", Pacifico "m").
3. **Open corners**: fabric that shows in a corner of a letter (before:
   Varsity "X", "x", Anton "t", "G").
4. **Joints**: where a stroke ends on another stroke, the end of the
   stroke sews first and the other stroke covers it, if that needs no
   trim. Where two strokes meet at a point (the "V" of "N", "M", "W",
   "w"), each stroke goes a little into the other. Is there a lump or a
   gap at a joint? Write the letters where a column end lies on top and
   shows.
5. **The serifs of Varsity**: a stem ends in a serif at each end, so one
   serif sews before the stem. Does the end of the stem show on that
   serif?
6. **Kanji**: at a crossing the thinner stroke stops at the sides of the
   stroke that goes through (十, 木, 林, 東). Is there a lump or a gap?
7. **Arabic**: each letter starts on its right and ends on its left,
   where the next one starts. Is there a long stitch between letters?
8. **Trims and travel**: does a travel stitch show anywhere?

Write the answers in the session or in a file next to this one, with
photos if you can.
"""


if __name__ == "__main__":
    main()
