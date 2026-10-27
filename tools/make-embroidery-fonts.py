# Makes the bundled embroidery fonts again: data/fonts/embroidery/<name>.embfont
# and <name>-OFL.txt (ADR 0016, P11 Milestone 2).
#
# Each font is an OFL font. The first four come from data/fonts; the others
# from the google/fonts repository (https://github.com/google/fonts), commit
# e7db6ca603fd4862010383ff17ffd6cc4ffac1c0, folder ofl/<folder>/. A font with
# a Reserved Font Name gets a new name (the OFL does not let a Modified
# Version use it); the converter writes the source and the license in the
# notes of the font.
#
# Use:  python tools/make-embroidery-fonts.py <google-fonts-folder> [name ...]
#       (<google-fonts-folder>/ofl/<folder>/<file>.ttf; build emb-cli first:
#       cargo build --release -p emb-cli)

import os
import shutil
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "data", "fonts", "embroidery")

# (file name, family, source: "repo" or the google/fonts folder, TTF file, small text)
FONTS = [
    ("block-sans", "Block Sans", "repo", "Lato-Bold.ttf", False),
    ("anton", "Anton", "repo", "Anton-Regular.ttf", False),
    ("pacifico", "Pacifico", "repo", "Pacifico-Regular.ttf", False),
    ("varsity", "Varsity", "repo", "Graduate-Regular.ttf", False),
    ("bebas-neue", "Bebas Neue", "bebasneue", "BebasNeue-Regular.ttf", False),
    ("bangers", "Bangers", "bangers", "Bangers-Regular.ttf", False),
    ("archivo-black", "Archivo Black", "archivoblack", "ArchivoBlack-Regular.ttf", False),
    ("bold-script", "Bold Script", "lobster", "Lobster-Regular.ttf", False),
    ("slab-heavy", "Slab Heavy", "alfaslabone", "AlfaSlabOne-Regular.ttf", False),
    ("casual-script", "Casual Script", "courgette", "Courgette-Regular.ttf", False),
    # P11 Milestone 2: fonts for small text first, then the others.
    ("atkinson-hyperlegible", "Atkinson Hyperlegible", "atkinsonhyperlegible", "AtkinsonHyperlegible-Bold.ttf", True),
    ("soft-round", "Soft Round", "varelaround", "VarelaRound-Regular.ttf", True),
    ("round-sans", "Round Sans", "paytoneone", "PaytoneOne-Regular.ttf", True),
    ("round-heavy", "Round Heavy", "lilitaone", "LilitaOne-Regular.ttf", True),
    ("fjalla-one", "Fjalla One", "fjallaone", "FjallaOne-Regular.ttf", False),
    ("poster-bold", "Poster Bold", "passionone", "PassionOne-Bold.ttf", False),
    ("geo-slab", "Geo Slab", "arvo", "Arvo-Bold.ttf", False),
    ("friendly-serif", "Friendly Serif", "breeserif", "BreeSerif-Regular.ttf", False),
    ("slab-round", "Slab Round", "patuaone", "PatuaOne-Regular.ttf", False),
    ("fat-face-serif", "Fat Face Serif", "abrilfatface", "AbrilFatface-Regular.ttf", False),
    ("shrikhand", "Shrikhand", "shrikhand", "Shrikhand-Regular.ttf", False),
    ("great-vibes", "Great Vibes", "greatvibes", "GreatVibes-Regular.ttf", False),
    ("vintage-script", "Vintage Script", "grandhotel", "GrandHotel-Regular.ttf", False),
    ("brush-hand", "Brush Hand", "kaushanscript", "KaushanScript-Regular.ttf", False),
    ("retro-script", "Retro Script", "cookie", "Cookie-Regular.ttf", False),
    ("leckerli-one", "Leckerli One", "leckerlione", "LeckerliOne-Regular.ttf", False),
    ("damion", "Damion", "damion", "Damion-Regular.ttf", False),
    ("black-ops-one", "Black Ops One", "blackopsone", "BlackOpsOne-Regular.ttf", False),
    ("bungee", "Bungee", "bungee", "Bungee-Regular.ttf", False),
    ("retro-display", "Retro Display", "righteous", "Righteous-Regular.ttf", False),
]


def main():
    gfonts = sys.argv[1]
    only = set(sys.argv[2:])
    exe = os.path.join(ROOT, "target", "release", "emb-cli.exe" if os.name == "nt" else "emb-cli")
    for slug, family, source, ttf, small in FONTS:
        if only and slug not in only:
            continue
        if source == "repo":
            src = os.path.join(ROOT, "data", "fonts", ttf)
            ofl = os.path.join(ROOT, "data", "fonts", ttf.split("-")[0] + "-OFL.txt")
        else:
            src = os.path.join(gfonts, "ofl", source, ttf)
            ofl = os.path.join(gfonts, "ofl", source, "OFL.txt")
        out = os.path.join(OUT, slug + ".embfont")
        cmd = [exe, "font", src, out, "--name", family] + (["--small-text"] if small else [])
        print(subprocess.run(cmd, check=True, capture_output=True, text=True).stdout.strip())
        shutil.copyfile(ofl, os.path.join(OUT, slug + "-OFL.txt"))


if __name__ == "__main__":
    main()
