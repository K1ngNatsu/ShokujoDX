# Makes the Japanese test font of the lettering bench (P20 Milestone 1,
# ADR 0141): crates/emb-cli/tests/data/noto-sans-jp-bold-subset.ttf.
#
# The source is the variable font Noto Sans JP of Google Fonts
# (https://github.com/google/fonts, ofl/notosansjp/NotoSansJP[wght].ttf,
# version 2.004, SIL Open Font License; the Reserved Font Name is "Source",
# so the subset keeps the name Noto Sans JP). The tool takes its Bold
# instance (wght 700) and keeps only the characters of the reference set
# and of the test words of crates/emb-cli/src/lettering_bench.rs, and "H"
# (the capital height of the font). The license text is
# crates/emb-cli/tests/data/noto-sans-jp-OFL.txt.
#
# A change of the reference set needs a new test font: run the tool, then
# accept the snapshot of the bench with a note in the commit message.
#
# Use:  python tools/make-test-font-jp.py <NotoSansJP[wght].ttf>
#       (needs fontTools: pip install fonttools)

import os
import re
import sys

from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BENCH = os.path.join(ROOT, "crates", "emb-cli", "src", "lettering_bench.rs")
OUT = os.path.join(ROOT, "crates", "emb-cli", "tests", "data", "noto-sans-jp-bold-subset.ttf")
SETS = ["HIRAGANA", "KATAKANA", "KANJI_FIRST_YEAR", "KANJI_MORE", "PUNCTUATION"]
QUOTE = chr(34)
BACKSLASH = chr(92)


def characters():
    """The characters of the reference set and of the test words."""
    text = open(BENCH, encoding="utf-8").read()
    chars = set()
    for name in SETS:
        head = "pub const " + name + ": &str = " + QUOTE
        start = text.index(head) + len(head)
        end = text.index(QUOTE + ";", start)
        # A Rust string continues on the next line after a backslash.
        chars |= set(text[start:end].replace(BACKSLASH + "\n", ""))
    w0 = text.index("pub const WORDS:")
    w1 = text.index("];", w0)
    words = re.findall(QUOTE + "([^" + QUOTE + "]*)" + QUOTE, text[w0:w1])
    chars |= set("".join(words))
    chars |= set("H ")
    return chars


def main():
    if len(sys.argv) != 2:
        sys.exit("use: python tools/make-test-font-jp.py <NotoSansJP[wght].ttf>")
    chars = characters()
    # The time of the source font, not of now: the same file at each run.
    font = TTFont(sys.argv[1], recalcTimestamp=False)
    bold = instancer.instantiateVariableFont(font, {"wght": 700}, updateFontNames=True)
    opts = subset.Options()
    opts.layout_features = ["*"]
    opts.name_IDs = ["*"]
    opts.name_languages = ["*"]
    opts.notdef_outline = True
    opts.glyph_names = False
    sub = subset.Subsetter(options=opts)
    sub.populate(unicodes=[ord(c) for c in sorted(chars)])
    sub.subset(bold)
    bold.save(OUT)
    print("wrote", OUT, "with", len(chars), "characters")


if __name__ == "__main__":
    main()
