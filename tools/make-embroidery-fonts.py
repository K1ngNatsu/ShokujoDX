# Makes the bundled embroidery fonts again: data/fonts/embroidery/<name>.embfont
# and <name>-OFL.txt (ADR 0016, P11 Milestone 2), and the font packages:
# data/font-packages/shokujodx-fonts-<package>.zip (P17 Milestone 7, ADR 0115).
#
# Each font is an OFL font. Some come from data/fonts; the others from the
# google/fonts repository (https://github.com/google/fonts), commit
# e7db6ca603fd4862010383ff17ffd6cc4ffac1c0, folder ofl/<folder>/. A font with
# a Reserved Font Name gets a new name (the OFL does not let a Modified
# Version use it); the converter writes the source and the license in the
# notes of the font.
#
# Quality (ADR 0115): the converter gives each font a score from 0 to 100.
# A font below LIMIT stops the tool: such a font is not in the program and
# not in a package. The fonts with the best scores are in the program; the
# others are in the font packages, which the user gets from the page of the
# release and adds with File > My Assets > Import a package.
#
# Routed letters (ADR 0113, ADR 0116): the converter makes the ends of the
# columns of each font reach the corners of the outline (--extend) and
# routes each letter: the order of its parts with the fewest trims, with
# travel runs under the parts that sew later (--route). The fill fonts
# (ADR 0117) are not routed.
#
# Letter corrections (ADR 0112): if data/fonts/embroidery/corrections/<name>.json
# is there, the converter applies it after the conversion, so a corrected
# letter stays corrected. A correction that does not apply stops the tool:
# correct the file (the letter of the new conversion has other parts).
#
# Use:  python tools/make-embroidery-fonts.py <google-fonts-folder> [--jobs N] [name ...]
#       (<google-fonts-folder>/ofl/<folder>/<file>.ttf; build emb-cli first:
#       cargo build --release -p emb-cli; --jobs: the number of fonts that
#       the tool makes at the same time, half of the processors if not given)

import concurrent.futures
import os
import re
import shutil
import subprocess
import sys
import zipfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "data", "fonts", "embroidery")
PACKAGES = os.path.join(ROOT, "data", "font-packages")
# The fonts of the packages before they go in the zip files.
WORK = os.path.join(ROOT, "target", "font-packages")

# The lowest score of a font of the program or of a package (the same as
# emb_letter::quality::LIMIT).
LIMIT = 65

# (file name, family, source: "repo" or the google/fonts folder, TTF file,
#  small text, package: None for a font of the program)
FONTS = [
    ("block-sans", "Block Sans", "repo", "Lato-Bold.ttf", False, None),
    ("anton", "Anton", "repo", "Anton-Regular.ttf", False, None),
    ("pacifico", "Pacifico", "repo", "Pacifico-Regular.ttf", False, None),
    ("varsity", "Varsity", "repo", "Graduate-Regular.ttf", False, None),
    ("bebas-neue", "Bebas Neue", "bebasneue", "BebasNeue-Regular.ttf", False, None),
    ("bangers", "Bangers", "bangers", "Bangers-Regular.ttf", False, None),
    ("archivo-black", "Archivo Black", "archivoblack", "ArchivoBlack-Regular.ttf", False, None),
    ("bold-script", "Bold Script", "lobster", "Lobster-Regular.ttf", False, None),
    ("casual-script", "Casual Script", "courgette", "Courgette-Regular.ttf", False, None),
    # P11 Milestone 2: fonts for small text first, then the others.
    ("atkinson-hyperlegible", "Atkinson Hyperlegible", "atkinsonhyperlegible", "AtkinsonHyperlegible-Bold.ttf", True, None),
    ("soft-round", "Soft Round", "varelaround", "VarelaRound-Regular.ttf", True, None),
    ("round-sans", "Round Sans", "paytoneone", "PaytoneOne-Regular.ttf", True, None),
    ("round-heavy", "Round Heavy", "lilitaone", "LilitaOne-Regular.ttf", True, None),
    ("fjalla-one", "Fjalla One", "fjallaone", "FjallaOne-Regular.ttf", False, None),
    ("poster-bold", "Poster Bold", "passionone", "PassionOne-Bold.ttf", False, None),
    ("geo-slab", "Geo Slab", "arvo", "Arvo-Bold.ttf", False, None),
    ("friendly-serif", "Friendly Serif", "breeserif", "BreeSerif-Regular.ttf", False, None),
    ("slab-round", "Slab Round", "patuaone", "PatuaOne-Regular.ttf", False, None),
    ("vintage-script", "Vintage Script", "grandhotel", "GrandHotel-Regular.ttf", False, None),
    ("brush-hand", "Brush Hand", "kaushanscript", "KaushanScript-Regular.ttf", False, None),
    ("retro-script", "Retro Script", "cookie", "Cookie-Regular.ttf", False, None),
    ("leckerli-one", "Leckerli One", "leckerlione", "LeckerliOne-Regular.ttf", False, None),
    ("damion", "Damion", "damion", "Damion-Regular.ttf", False, None),
    ("black-ops-one", "Black Ops One", "blackopsone", "BlackOpsOne-Regular.ttf", False, None),
    ("bungee", "Bungee", "bungee", "Bungee-Regular.ttf", False, None),
    ("retro-display", "Retro Display", "righteous", "Righteous-Regular.ttf", False, None),
    # P17 Milestone 7: the new fonts with a score of 85 or more.
    ("wide-tech", "Wide Tech", "audiowide", "Audiowide-Regular.ttf", False, None),
    ("plain-sans", "Plain Sans", "varela", "Varela-Regular.ttf", False, None),
    ("staatliches", "Staatliches", "staatliches", "Staatliches-Regular.ttf", False, None),
    ("square-tech", "Square Tech", "aldrich", "Aldrich-Regular.ttf", False, None),
    ("comic-neue", "Comic Neue", "comicneue", "ComicNeue-Bold.ttf", False, None),
    ("sturdy-sans", "Sturdy Sans", "armata", "Armata-Regular.ttf", False, None),
    ("patrick-hand", "Patrick Hand", "patrickhand", "PatrickHand-Regular.ttf", False, None),
    ("chakra-petch", "Chakra Petch", "chakrapetch", "ChakraPetch-Bold.ttf", False, None),
    ("titillium-web", "Titillium Web", "titilliumweb", "TitilliumWeb-Bold.ttf", False, None),
    ("sport-sans", "Sport Sans", "viga", "Viga-Regular.ttf", False, None),
    ("monoline-script", "Monoline Script", "sacramento", "Sacramento-Regular.ttf", False, None),
    ("amatic-sc", "Amatic SC", "amaticsc", "AmaticSC-Bold.ttf", False, None),
    ("friendly-hand", "Friendly Hand", "delius", "Delius-Regular.ttf", False, None),
    ("pangolin", "Pangolin", "pangolin", "Pangolin-Regular.ttf", False, None),
    ("soft-slab", "Soft Slab", "creteround", "CreteRound-Regular.ttf", False, None),
    ("itim", "Itim", "itim", "Itim-Regular.ttf", False, None),
    ("rajdhani", "Rajdhani", "rajdhani", "Rajdhani-Bold.ttf", False, None),
    ("racer-sans", "Racer Sans", "jockeyone", "JockeyOne-Regular.ttf", False, None),
    ("poppins", "Poppins", "poppins", "Poppins-SemiBold.ttf", False, None),
    ("indie-flower", "Indie Flower", "indieflower", "IndieFlower-Regular.ttf", False, None),
    ("wide-sans", "Wide Sans", "kronaone", "KronaOne-Regular.ttf", False, None),
    ("bai-jamjuree", "Bai Jamjuree", "baijamjuree", "BaiJamjuree-Bold.ttf", False, None),
    ("square-narrow", "Square Narrow", "squadaone", "SquadaOne-Regular.ttf", False, None),
    ("alata", "Alata", "alata", "Alata-Regular.ttf", False, None),
    ("neat-hand", "Neat Hand", "handlee", "Handlee-Regular.ttf", False, None),
    ("marker-hand", "Marker Hand", "gochihand", "GochiHand-Regular.ttf", False, None),
    ("barlow-condensed", "Barlow Condensed", "barlowcondensed", "BarlowCondensed-Bold.ttf", False, None),
    ("angle-sans", "Angle Sans", "quantico", "Quantico-Bold.ttf", False, None),
    ("balsamiq-sans", "Balsamiq Sans", "balsamiqsans", "BalsamiqSans-Bold.ttf", False, None),
    # The fill fonts (ADR 0117, see FILL): the same source fonts as the
    # fonts with their names, with all letters as fills.
    ("block-sans-fill", "Block Sans Fill", "repo", "Lato-Bold.ttf", False, None),
    ("varsity-fill", "Varsity Fill", "repo", "Graduate-Regular.ttf", False, None),
    ("anton-fill", "Anton Fill", "repo", "Anton-Regular.ttf", False, None),
    ("archivo-black-fill", "Archivo Black Fill", "archivoblack", "ArchivoBlack-Regular.ttf", False, None),
    ("slab-heavy-fill", "Slab Heavy Fill", "alfaslabone", "AlfaSlabOne-Regular.ttf", False, None),
    ("poster-heavy-fill", "Poster Heavy Fill", "bowlbyone", "BowlbyOne-Regular.ttf", False, None),
    ("giant-round-fill", "Giant Round Fill", "titanone", "TitanOne-Regular.ttf", False, None),
    ("funky-slab-fill", "Funky Slab Fill", "rammettoone", "RammettoOne-Regular.ttf", False, None),
    ("rubik-mono-one-fill", "Rubik Mono One Fill", "rubikmonoone", "RubikMonoOne-Regular.ttf", False, None),
    ("shrikhand-fill", "Shrikhand Fill", "shrikhand", "Shrikhand-Regular.ttf", False, None),
    # P17 Milestone 7: the font packages (a score from the limit to 84).
    ("barlow-semi-condensed", "Barlow Semi Condensed", "barlowsemicondensed", "BarlowSemiCondensed-Bold.ttf", False, "sans"),
    ("humanist-sans", "Humanist Sans", "ptsans", "PT_Sans-Web-Bold.ttf", False, "sans"),
    ("neucha", "Neucha", "neucha", "Neucha.ttf", False, "hand"),
    ("machine-sans", "Machine Sans", "russoone", "RussoOne-Regular.ttf", False, "sans"),
    ("barlow", "Barlow", "barlow", "Barlow-Bold.ttf", False, "sans"),
    ("shadows-into-light", "Shadows Into Light", "shadowsintolight", "ShadowsIntoLight.ttf", False, "hand"),
    ("architects-daughter", "Architects Daughter", "architectsdaughter", "ArchitectsDaughter-Regular.ttf", False, "hand"),
    ("rubik-mono-one", "Rubik Mono One", "rubikmonoone", "RubikMonoOne-Regular.ttf", False, "sans"),
    ("cabin-condensed", "Cabin Condensed", "cabincondensed", "CabinCondensed-Bold.ttf", False, "sans"),
    ("game-sans", "Game Sans", "play", "Play-Bold.ttf", False, "sans"),
    ("khand", "Khand", "khand", "Khand-Bold.ttf", False, "sans"),
    ("stage-round", "Stage Round", "concertone", "ConcertOne-Regular.ttf", False, "display"),
    ("sturdy-serif", "Sturdy Serif", "volkhov", "Volkhov-Bold.ttf", False, "serif"),
    ("sriracha", "Sriracha", "sriracha", "Sriracha-Regular.ttf", False, "hand"),
    ("mali", "Mali", "mali", "Mali-Bold.ttf", False, "hand"),
    ("fira-sans-condensed", "Fira Sans Condensed", "firasanscondensed", "FiraSansCondensed-Bold.ttf", False, "sans"),
    ("wide-heavy", "Wide Heavy", "daysone", "DaysOne-Regular.ttf", False, "sans"),
    ("classic-sans", "Classic Sans", "hammersmithone", "HammersmithOne-Regular.ttf", False, "sans"),
    ("bookish-slab", "Bookish Slab", "arbutusslab", "ArbutusSlab-Regular.ttf", False, "serif"),
    ("gloria-hallelujah", "Gloria Hallelujah", "gloriahallelujah", "GloriaHallelujah.ttf", False, "hand"),
    ("angular-sans", "Angular Sans", "germaniaone", "GermaniaOne-Regular.ttf", False, "sans"),
    ("stencil-sans", "Stencil Sans", "allertastencil", "AllertaStencil-Regular.ttf", False, "display"),
    ("amaranth", "Amaranth", "amaranth", "Amaranth-Bold.ttf", False, "sans"),
    ("kalam", "Kalam", "kalam", "Kalam-Bold.ttf", False, "hand"),
    ("mitr", "Mitr", "mitr", "Mitr-SemiBold.ttf", False, "sans"),
    ("caveat-brush", "Caveat Brush", "caveatbrush", "CaveatBrush-Regular.ttf", False, "hand"),
    ("prompt", "Prompt", "prompt", "Prompt-Bold.ttf", False, "sans"),
    ("zilla-slab", "Zilla Slab", "zillaslab", "ZillaSlab-Bold.ttf", False, "serif"),
    ("bold-script-two", "Bold Script Two", "lobstertwo", "LobsterTwo-Bold.ttf", False, "script"),
    ("alegreya-sans", "Alegreya Sans", "alegreyasans", "AlegreyaSans-Bold.ttf", False, "sans"),
    ("kanit", "Kanit", "kanit", "Kanit-Bold.ttf", False, "sans"),
    ("comic-narrow", "Comic Narrow", "acme", "Acme-Regular.ttf", False, "display"),
    ("fira-sans", "Fira Sans", "firasans", "FiraSans-Bold.ttf", False, "sans"),
    ("heavy-sans", "Heavy Sans", "candal", "Candal.ttf", False, "sans"),
    ("saira-stencil-one", "Saira Stencil One", "sairastencilone", "SairaStencilOne-Regular.ttf", False, "display"),
    ("pen-script", "Pen Script", "marckscript", "MarckScript-Regular.ttf", False, "script"),
    ("sansita", "Sansita", "sansita", "Sansita-Bold.ttf", False, "sans"),
    ("sketch-sans", "Sketch Sans", "londrinasolid", "LondrinaSolid-Regular.ttf", False, "display"),
    ("francois-one", "Francois One", "francoisone", "FrancoisOne-Regular.ttf", False, "sans"),
    ("compact-heavy", "Compact Heavy", "changaone", "ChangaOne-Regular.ttf", False, "display"),
    ("market-hand", "Market Hand", "chelseamarket", "ChelseaMarket-Regular.ttf", False, "display"),
    ("humanist-serif", "Humanist Serif", "ptserif", "PT_Serif-Web-Bold.ttf", False, "serif"),
    ("jazz-hand", "Jazz Hand", "boogaloo", "Boogaloo-Regular.ttf", False, "display"),
    ("suez-one", "Suez One", "suezone", "SuezOne-Regular.ttf", False, "serif"),
    ("wave-script", "Wave Script", "seaweedscript", "SeaweedScript-Regular.ttf", False, "script"),
    ("galada", "Galada", "galada", "Galada-Regular.ttf", False, "script"),
    ("poster-hand", "Poster Hand", "ranchers", "Ranchers-Regular.ttf", False, "display"),
    ("giant-round", "Giant Round", "titanone", "TitanOne-Regular.ttf", False, "display"),
    ("swash-serif", "Swash Serif", "berkshireswash", "BerkshireSwash-Regular.ttf", False, "serif"),
    ("petal-script", "Petal Script", "lilyscriptone", "LilyScriptOne-Regular.ttf", False, "script"),
    ("candy-sans", "Candy Sans", "bubblegumsans", "BubblegumSans-Regular.ttf", False, "display"),
    ("funky-slab", "Funky Slab", "rammettoone", "RammettoOne-Regular.ttf", False, "display"),
    ("sign-script", "Sign Script", "mrdafoe", "MrDafoe-Regular.ttf", False, "script"),
    ("citrus-round", "Citrus Round", "lemon", "Lemon-Regular.ttf", False, "display"),
    ("bouncy-brush", "Bouncy Brush", "kavoon", "Kavoon-Regular.ttf", False, "display"),
    ("speed-sans", "Speed Sans", "fugazone", "FugazOne-Regular.ttf", False, "sans"),
    ("fast-sans", "Fast Sans", "racingsansone", "RacingSansOne-Regular.ttf", False, "sans"),
    ("casual-bold", "Casual Bold", "carterone", "CarterOne.ttf", False, "display"),
    ("norican", "Norican", "norican", "Norican-Regular.ttf", False, "script"),
    ("blackletter", "Blackletter", "pirataone", "PirataOne-Regular.ttf", False, "display"),
    ("poster-heavy", "Poster Heavy", "bowlbyone", "BowlbyOne-Regular.ttf", False, "display"),
    ("diner-script", "Diner Script", "oleoscript", "OleoScript-Bold.ttf", False, "script"),
    ("stencil-serif", "Stencil Serif", "stardosstencil", "StardosStencil-Bold.ttf", False, "display"),
    ("playball", "Playball", "playball", "Playball-Regular.ttf", False, "script"),
    ("loop-script", "Loop Script", "sofia", "Sofia-Regular.ttf", False, "script"),
    ("pattaya", "Pattaya", "pattaya", "Pattaya-Regular.ttf", False, "script"),
    ("display-heavy", "Display Heavy", "coda", "Coda-ExtraBold.ttf", False, "display"),
    ("allura", "Allura", "allura", "Allura-Regular.ttf", False, "script"),
    # Fonts of earlier versions of the program with a score below the limit
    # (ADR 0115). They are not in the program now. The package lets a user
    # change a design that has them; the tool does not stop for them.
    ("slab-heavy", "Slab Heavy", "alfaslabone", "AlfaSlabOne-Regular.ttf", False, "earlier"),
    ("fat-face-serif", "Fat Face Serif", "abrilfatface", "AbrilFatface-Regular.ttf", False, "earlier"),
    ("shrikhand", "Shrikhand", "shrikhand", "Shrikhand-Regular.ttf", False, "earlier"),
    ("great-vibes", "Great Vibes", "greatvibes", "GreatVibes-Regular.ttf", False, "earlier"),
]

# The font packages: (name, title, what is in it).
PACKS = [
    ("sans", "Sans fonts", "Plain fonts with no serifs: text, names and numbers."),
    ("display", "Display fonts", "Heavy and decorative fonts for short texts, and stencil fonts."),
    ("serif", "Serif and slab fonts", "Fonts with serifs."),
    ("script", "Script fonts", "Joined and flowing fonts."),
    ("hand", "Hand fonts", "Fonts that look as if written by hand."),
    (
        "earlier",
        "Fonts of earlier versions",
        "Fonts that were in the program before version 0.4. Their quality score is below the limit "
        "of the program: some letters have parts with no stitches. Use them to change a design "
        "that has them.",
    ),
]
# The package whose fonts can be below the limit.
BELOW_LIMIT = "earlier"


# The tuned fonts (P17 Milestone 5, ADR 0113; the same list as
# emb_letter::bundled::TUNED): each letter of their basic set was looked at
# on proof pictures, and the font lists show them first (--tuned). After a
# change of the converter, look at the proof pictures of these fonts again
# (emb-cli font-audit --proof).
TUNED = {
    "block-sans",
    "friendly-serif",
    "pacifico",
    "atkinson-hyperlegible",
    "varsity",
    "bebas-neue",
    "soft-round",
    "geo-slab",
    "brush-hand",
    "vintage-script",
}


# The fill fonts (ADR 0117): the converter makes all their letters as
# fills (--stitch fill), for large text and for heavy fonts whose letters
# have no clear columns. They have no route: a letter is one fill, or a few.
FILL = {
    "block-sans-fill",
    "varsity-fill",
    "anton-fill",
    "archivo-black-fill",
    "slab-heavy-fill",
    "poster-heavy-fill",
    "giant-round-fill",
    "funky-slab-fill",
    "rubik-mono-one-fill",
    "shrikhand-fill",
}


def write_package(name, title, about):
    """Writes the zip file of a package from the fonts in WORK. The same
    fonts give the same bytes (no dates, the files in the order of their
    names)."""
    folder = os.path.join(WORK, name)
    rows = [f for f in FONTS if f[5] == name]
    missing = [f[0] for f in rows if not os.path.isfile(os.path.join(folder, f[0] + ".embfont"))]
    if missing:
        sys.exit("package %s: make these fonts first: %s" % (name, " ".join(missing)))
    lines = [
        "ShokujoDX font package: " + title,
        "",
        about,
        "",
        "To add the fonts: File > My Assets > Import a package, and choose this zip file.",
        "",
        "Each font is a Modified Version of a font under the SIL Open Font License,",
        "Version 1.1: the letters are embroidery objects. A font whose license has a",
        "Reserved Font Name has a new name. The license text of each font is next to it",
        "(<name>-OFL.txt); the notes in each font file name its source.",
        "",
        "Font (file): source font",
    ]
    for slug, family, _source, ttf, _small, _pack in sorted(rows, key=lambda f: f[1].lower()):
        lines.append("  %s (%s.embfont): %s" % (family, slug, ttf))
    files = [("README.txt", (chr(10).join(lines) + chr(10)).encode("utf-8"))]
    for slug in sorted(f[0] for f in rows):
        for ext in (".embfont", "-OFL.txt"):
            with open(os.path.join(folder, slug + ext), "rb") as f:
                files.append(("fonts/" + slug + ext, f.read()))
    os.makedirs(PACKAGES, exist_ok=True)
    path = os.path.join(PACKAGES, "shokujodx-fonts-%s.zip" % name)
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as z:
        for file, data in files:
            info = zipfile.ZipInfo(file, date_time=(1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o644 << 16
            z.writestr(info, data, compresslevel=9)
    print("wrote %s: %d fonts, %.1f MB" % (path, len(rows), os.path.getsize(path) / 1e6))


def make_font(exe, gfonts, row):
    """Makes one font. Gives the text that the converter printed and an
    error text (empty if the font is made)."""
    slug, family, source, ttf, small, pack = row
    if source == "repo":
        src = os.path.join(ROOT, "data", "fonts", ttf)
        ofl = os.path.join(ROOT, "data", "fonts", ttf.split("-")[0] + "-OFL.txt")
    else:
        src = os.path.join(gfonts, "ofl", source, ttf)
        ofl = os.path.join(gfonts, "ofl", source, "OFL.txt")
    folder = os.path.join(WORK, pack) if pack else OUT
    os.makedirs(folder, exist_ok=True)
    out = os.path.join(folder, slug + ".embfont")
    cmd = [exe, "font", src, out, "--name", family] + (["--small-text"] if small else [])
    if slug in FILL:
        cmd += ["--stitch", "fill"]
    else:
        cmd += ["--extend", "--route"]
    if slug in TUNED:
        cmd += ["--tuned"]
    fixes = os.path.join(OUT, "corrections", slug + ".json")
    if os.path.isfile(fixes):
        cmd += ["--corrections", fixes]
    done = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8")
    if done.returncode != 0:
        return "", done.stderr.strip() or done.stdout.strip()
    text = done.stdout.strip()
    score = re.search("^quality ([0-9]+):", done.stdout, flags=re.M)
    if not score:
        return text, "%s: the converter gave no quality score" % slug
    if int(score.group(1)) < LIMIT and pack != BELOW_LIMIT:
        os.remove(out)
        return text, "%s: the score %s is below the limit %d" % (slug, score.group(1), LIMIT)
    shutil.copyfile(ofl, os.path.join(folder, slug + "-OFL.txt"))
    return text, ""


def main():
    args = sys.argv[1:]
    jobs = max(1, (os.cpu_count() or 2) // 2)
    if "--jobs" in args:
        at = args.index("--jobs")
        jobs = max(1, int(args[at + 1]))
        del args[at : at + 2]
    gfonts = args[0]
    only = set(args[1:])
    exe = os.path.join(ROOT, "target", "release", "emb-cli.exe" if os.name == "nt" else "emb-cli")
    rows = [f for f in FONTS if not only or f[0] in only]
    # The converter routes a font in 30 to 90 seconds, most of it on one
    # processor: some fonts at the same time.
    with concurrent.futures.ThreadPoolExecutor(jobs) as pool:
        results = list(pool.map(lambda row: make_font(exe, gfonts, row), rows))
    errors = []
    for text, error in results:
        if text:
            print(text)
        if error:
            errors.append(error)
    if errors:
        sys.exit(chr(10).join(errors))
    for name, title, about in PACKS:
        if any(f[5] == name for f in rows):
            write_package(name, title, about)


if __name__ == "__main__":
    main()
