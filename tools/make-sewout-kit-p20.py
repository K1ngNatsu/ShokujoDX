# Makes the sew-out kit of phase P20, Milestones 3, 4 and 6 (docs/design/p20-plan.md):
# test sheets of Japanese characters in the test font of the lettering
# bench, with the files for the machine, a worksheet and a picture of each
# one, and a checklist.
#
# The font is the test font of the bench
# (crates/emb-cli/tests/data/noto-sans-jp-bold-subset.ttf, Noto Sans JP
# Bold, SIL Open Font License), so the kit has only its characters.
#
# In the kit:
#   sheet-25mm   20 characters at 25 mm: crossings, T and L junctions,
#                corners, slanted strokes, kana
#   sheet-50mm   4 of them at 50 mm
#   words-40mm   the words 刺繍 and 東京 at 40 mm (one stitch for the text)
#   noto-sans-jp-20mm, zen-maru-gothic-20mm
#                the Japanese embroidery fonts of the font package
#                (Milestone 6): words and characters at 20 mm. Make the
#                fonts first: python tools/make-embroidery-fonts.py
#                <google-fonts-folder> noto-sans-jp zen-maru-gothic
#   CHECKLIST.md
#
# Use:  python tools/make-sewout-kit-p20.py [<folder>]
#       (the folder: out/p20-sewout if not given; build emb-cli first:
#       cargo build --release -p emb-cli)

import datetime
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
EXE = os.path.join(ROOT, "target", "release", "emb-cli.exe" if os.name == "nt" else "emb-cli")
FONT = os.path.join(ROOT, "crates", "emb-cli", "tests", "data", "noto-sans-jp-bold-subset.ttf")
MACHINE = "husqvarna_topaz20"
# A sheet fits the hoop of 260 x 200 mm with some space.
FIT = (190.0, 250.0)

SHEETS = [
    ("sheet-25mm", 25.0, "十東木本中田日刺繍龍書サャあおわタネ女海",
     "crossings (十 東 木 本 中 田 サ ャ あ お), T and L junctions and corners (日 中 田 タ ネ), many strokes (刺 繍 龍 書), slanted strokes (女 海 わ)"),
    ("sheet-50mm", 50.0, "東刺本あ", "four of them, large: split satin stitches (strokes wider than 7 mm)"),
]
WORDS = [("words-40mm", 40.0, "刺繍 東京", "two words: all the letters of a text have the same stitch")]
# The Japanese embroidery fonts (Milestone 6), from the work folder of the
# font tool.
JP_FONTS = os.path.join(ROOT, "target", "font-packages", "japanese")
JP_TEXT = "刺繍東京ありがとうサムライ龍山田太郎さくら日本"
EMB_SHEETS = [
    ("noto-sans-jp-20mm", "noto-sans-jp", 20.0, "Noto Sans JP (embroidery font of the package) at 20 mm"),
    ("zen-maru-gothic-20mm", "zen-maru-gothic", 20.0, "Zen Maru Gothic (embroidery font of the package) at 20 mm"),
]


def run(args):
    done = subprocess.run([EXE] + args, capture_output=True, text=True, encoding="utf-8")
    if done.returncode != 0:
        sys.exit("emb-cli %s: %s" % (" ".join(args[:2]), done.stderr.strip() or done.stdout.strip()))
    return done.stdout


def text_of(mm):
    return ("%.1f" % mm).rstrip("0").rstrip(".")


def sheet(project, size, text, font=FONT):
    out = run(["font-sheet", font, project, "--sizes", text_of(size), "--text", text, "--row", "180"])
    m = re.search(r"([0-9]+) x ([0-9]+) mm", out)
    if not (m and float(m.group(1)) <= FIT[0] and float(m.group(2)) <= FIT[1]):
        sys.exit("%s: the sheet does not fit the hoop: %s" % (project, out.strip()))


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
        "size": "%.0f x %.0f mm" % (float(m.group(1)), float(m.group(2))),
    }


CHECKLIST = """# Sew-out of P20 Milestones 3 and 4: crossings, junctions and sew order

Made on {date} by `tools/make-sewout-kit-p20.py`, with the font Noto Sans
JP Bold (the test font of the lettering bench). Machine files for the
Husqvarna Topaz 20 (VP3) and as DST. Sew each design on a medium woven
fabric with a tear-away stabilizer, as a normal job.

| Design | What | Size | Stitches | Trims |
| --- | --- | --- | --- | --- |
{rows}

## What to look at

1. **Crossings** (十, 東, 本, サ, あ, お): one stroke goes through and the
   other stops at its two sides. Is there a lump where the strokes cross?
   Is there a gap or fabric that shows at the sides of the stroke that
   goes through?
2. **The stroke that goes through** is the longer one today. Does it look
   right, or should the other stroke go through (for example, the
   horizontal stroke of 十)? Write the characters where it looks wrong.
3. **T and L junctions and corners** (日, 中, 田, タ, ネ): the end of a
   stroke stops a little inside the other stroke. Is there a lump or a gap
   at a corner?
4. **Strokes that meet at a slant** (女, 海, 本, わ): the end of the stroke
   follows the side of the other stroke. Is there a lump or a gap?
5. **Many strokes** (刺, 繍, 龍, 書) at 25 mm: are the small parts clear?
6. **The large sheet** (50 mm): the satin stitches of strokes wider than
   7 mm are split. Do the split stitches look good, or should wide strokes
   be tatami?
7. **The words** (40 mm): all strokes have the same stitch. Do the letters
   look the same, and the mirrored strokes (the legs of 東 and 本)?
8. **The ends of strokes** that the font cuts at a slant (the ends of 女
   and わ, the start of 東): the last stitches turn to the slant of the
   end, so the corners have stitches. Do the ends look like the font?
9. **The sew order** (Milestone 4): the needle goes from one stroke to the
   next under strokes that sew later, so there are few trims. Does a
   travel stitch show anywhere, or pull the fabric?
10. **The Japanese embroidery fonts** (Milestone 6: Noto Sans JP, a gothic,
   and Zen Maru Gothic, a rounded gothic, from the font package): do the
   words sew well at 20 mm? Which font looks better on fabric? (Known:
   the あ of Zen Maru Gothic has a small gap in its inner stroke.)

Write the answers in the session or in a file next to this one, with
photos if you can.
"""


def main():
    kit = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "out", "p20-sewout"))
    if not os.path.isfile(EXE):
        sys.exit("build emb-cli first: cargo build --release -p emb-cli")
    os.makedirs(kit, exist_ok=True)
    rows = []
    for name, size, text, what in SHEETS + WORDS:
        folder = os.path.join(kit, name)
        os.makedirs(folder, exist_ok=True)
        project = os.path.join(folder, name + ".embproj")
        sheet(project, size, text)
        n = output(project, folder)
        rows.append("| `%s` | %s | %s | %d | %d |" % (name, what, n["size"], n["stitches"], n["trims"]))
        print("made", name)
    for name, slug, size, what in EMB_SHEETS:
        font = os.path.join(JP_FONTS, slug + ".embfont")
        if not os.path.isfile(font):
            print("no %s: make the Japanese fonts first (tools/make-embroidery-fonts.py)" % name)
            continue
        folder = os.path.join(kit, name)
        os.makedirs(folder, exist_ok=True)
        project = os.path.join(folder, name + ".embproj")
        sheet(project, size, JP_TEXT, font)
        n = output(project, folder)
        rows.append("| `%s` | %s | %s | %d | %d |" % (name, what, n["size"], n["stitches"], n["trims"]))
        print("made", name)
    with open(os.path.join(kit, "CHECKLIST.md"), "w", encoding="utf-8", newline="\n") as f:
        f.write(CHECKLIST.format(date=datetime.date.today().isoformat(), rows="\n".join(rows)))
    print("wrote the kit to", kit)


if __name__ == "__main__":
    main()
