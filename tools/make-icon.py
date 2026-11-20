# Makes the program icon: data/icon/icon-<size>.png and data/icon/app.ico,
# from the logo data/icon/icon.svg (ADR 0140).
#
# The logo is the logo of the software of the shop that uses ShokujoDX: a
# purple star in an embroidery hoop, with a needle and a thread, on a dark
# tile with round corners. Its SVG file has one mask of polygons (straight
# lines only, `M`, `L` and `Z`): a white polygon adds to the shape, a black
# polygon removes from it, in their order. The tile is a rectangle with
# round corners (a clip path). This tool reads only such a file, so that it
# needs no SVG library: it stops with an error for other SVG data.
#
# Use:  python tools/make-icon.py   (needs Pillow)

import os
import re
import xml.etree.ElementTree as ET

from PIL import Image, ImageDraw

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "data", "icon")
SOURCE = os.path.join(OUT, "icon.svg")
SIZES = [16, 24, 32, 48, 64, 128, 256, 512, 1024]
ICO_SIZES = [16, 24, 32, 48, 64, 128, 256]
S = 4096  # drawing size; each icon is made smaller from it
NS = "{http://www.w3.org/2000/svg}"


def color(text):
    """An SVG color `rgb(r,g,b)` or `#rrggbb`."""
    m = re.fullmatch(r"rgb\((\d+),\s*(\d+),\s*(\d+)\)", text.strip())
    if m:
        return tuple(int(v) for v in m.groups())
    m = re.fullmatch(r"#([0-9a-fA-F]{6})", text.strip())
    if m:
        return tuple(int(m.group(1)[i : i + 2], 16) for i in (0, 2, 4))
    raise ValueError(f"not a color: {text!r}")


def polygons(d):
    """The polygons of path data with only `M`, `L` and `Z`."""
    out, cur = [], []
    for cmd, args in re.findall(r"([MLZmlz])([^MLZmlz]*)", d):
        nums = [float(v) for v in re.findall(r"-?\d+(?:\.\d+)?(?:[eE][-+]?\d+)?", args)]
        if cmd in "ML":
            if cmd == "M" and cur:
                out.append(cur)
                cur = []
            if len(nums) % 2:
                raise ValueError("odd number of coordinates")
            cur += list(zip(nums[0::2], nums[1::2]))
        elif cmd in "Zz":
            if cur:
                out.append(cur)
            cur = []
        else:
            raise ValueError(f"path command {cmd} is not supported")
    if cur:
        out.append(cur)
    return out


def read_logo(path):
    """The size of the logo, its tile (corner radius, color), the color of
    the shape, and the polygons of the shape with their fill (True: add)."""
    root = ET.parse(path).getroot()
    size = float(root.get("width"))
    if float(root.get("height")) != size:
        raise ValueError("the logo is not square")
    mask = root.find(f"{NS}defs/{NS}mask")
    shape = []
    for p in mask.findall(f"{NS}path"):
        fill = p.get("fill")
        if fill not in ("white", "black"):
            raise ValueError(f"a path of the mask has the fill {fill!r}")
        shape += [(poly, fill == "white") for poly in polygons(p.get("d"))]
    clip = root.find(f"{NS}defs/{NS}clipPath/{NS}rect")
    radius = float(clip.get("rx"))
    rects = root.findall(f"{NS}g/{NS}rect")
    tile = next(color(r.get("fill")) for r in rects if r.get("mask") is None)
    ink = next(color(r.get("fill")) for r in rects if r.get("mask") is not None)
    return size, radius, tile, ink, shape


def draw():
    size, radius, tile, ink, shape = read_logo(SOURCE)
    k = S / size
    mask = Image.new("L", (S, S), 0)
    d = ImageDraw.Draw(mask)
    for poly, add in shape:
        d.polygon([(x * k, y * k) for x, y in poly], fill=255 if add else 0)
    alpha = Image.new("L", (S, S), 0)
    ImageDraw.Draw(alpha).rounded_rectangle((0, 0, S - 1, S - 1), radius=radius * k, fill=255)
    img = Image.composite(Image.new("RGB", (S, S), ink), Image.new("RGB", (S, S), tile), mask)
    img.putalpha(alpha)
    return img


def main():
    os.makedirs(OUT, exist_ok=True)
    # Premultiplied alpha, so that the edge of the tile gets no light or
    # dark fringe when the picture is made smaller.
    big = draw().convert("RGBa")
    icons = {}
    for size in SIZES:
        icons[size] = big.resize((size, size), Image.LANCZOS).convert("RGBA")
        icons[size].save(os.path.join(OUT, f"icon-{size}.png"), optimize=True)
    # The .ico keeps each size as its own picture.
    icons[256].save(os.path.join(OUT, "app.ico"), sizes=[(s, s) for s in ICO_SIZES], append_images=[icons[s] for s in ICO_SIZES if s != 256])
    print("wrote", OUT)


if __name__ == "__main__":
    main()
