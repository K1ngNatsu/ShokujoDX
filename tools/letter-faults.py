# Draws letters of the letter fault bench (phase P21): the outline of the
# source font (gray), the needle points of each column (colors; travel
# runs red), needle points more than 0.4 mm outside the outline (magenta
# rings), open area (orange) and the convex corners of the outline (green
# dots).
#
# With --plain (P22 Milestone 6): the letters as the eye sees them, with
# no marks of the measures: the area of the outline (light gray), the
# stitches as thread (0.3 mm wide) and the edge of the outline over them
# (a thin red line), for the judged set of letters.
#
# Use:
#   emb-cli letter-faults --proof faults.json
#   python tools/letter-faults.py faults.json out.png "Anton ttf M" "Block Sans emb A" ...
#   python tools/letter-faults.py faults.json out.png --plain --prefix "Anton emb" [--cols 6] [--cell 300]
#
# The names are "<embroidery font> emb <letter>" (the embroidery font) or
# "<embroidery font> ttf <letter>" (lettering in the source font).
# Needs Pillow (pip install pillow).

import argparse
import json

from PIL import Image, ImageChops, ImageDraw

COLORS = [
    (31, 119, 180),
    (44, 160, 44),
    (148, 103, 189),
    (140, 86, 75),
    (23, 190, 207),
    (188, 189, 34),
    (127, 127, 127),
]
# The width of a thread in --plain, mm.
THREAD_MM = 0.3


def main():
    ap = argparse.ArgumentParser(description="Pictures of the letter fault bench")
    ap.add_argument("faults")
    ap.add_argument("out")
    ap.add_argument("names", nargs="*")
    ap.add_argument("--plain", action="store_true", help="no marks of the measures: outline and thread")
    ap.add_argument("--prefix", default="", help="all letters whose name starts with this text")
    ap.add_argument("--cols", type=int, default=0)
    ap.add_argument("--cell", type=int, default=520)
    args = ap.parse_args()
    letters = json.load(open(args.faults, encoding="utf-8"))
    data = {d["name"]: d for d in letters}
    names = list(args.names)
    if args.prefix:
        names += [d["name"] for d in letters if d["name"].startswith(args.prefix)]
    cell = args.cell
    cols = args.cols or max(1, min(3, len(names)))
    rows = (len(names) + cols - 1) // cols
    img = Image.new("RGB", (cols * cell, max(rows, 1) * cell), "white")
    draw = ImageDraw.Draw(img)
    for k, name in enumerate(names):
        d = data.get(name)
        if d is None:
            print("not in the file:", name)
            continue
        ox, oy = (k % cols) * cell, (k // cols) * cell
        pts = [p for r in d["outline"] for p in r]
        xs, ys = [p[0] for p in pts], [p[1] for p in pts]
        x0, y0 = min(xs), min(ys)
        margin = 30 if args.plain else 60
        s = (cell - margin) / max(max(xs) - x0, max(ys) - y0)

        def at(p):
            return (ox + margin / 2 + (p[0] - x0) * s, oy + margin / 2 + 8 + (p[1] - y0) * s)

        if args.plain:
            width = max(1, round(THREAD_MM * s))
            # The area of the outline, even-odd (the holes stay white).
            mask = Image.new("1", img.size, 0)
            for r in d["outline"]:
                ring = Image.new("1", img.size, 0)
                ImageDraw.Draw(ring).polygon([at(p) for p in r], fill=1)
                mask = ImageChops.logical_xor(mask, ring)
            img.paste((222, 222, 222), mask=mask)
            for part in d["parts"]:
                color = (90, 90, 90) if part["kind"] == "run" else (20, 40, 110)
                line = [at(p) for p in part["pts"]]
                if len(line) > 1:
                    draw.line(line, fill=color, width=width)
            for r in d["outline"]:
                ring = [at(p) for p in r]
                draw.line(ring + ring[:1], fill=(235, 60, 60), width=1)
            draw.text((ox + 4, oy + 2), name, fill=(0, 0, 0))
            continue
        for r in d["outline"]:
            draw.polygon([at(p) for p in r], fill=(225, 225, 225), outline=(150, 150, 150))
        for p in d["open"]:
            x, y = at(p)
            draw.rectangle([x - 1, y - 1, x + 1, y + 1], fill=(255, 150, 0))
        for i, part in enumerate(d["parts"]):
            color = (220, 0, 0) if part["kind"] == "run" else COLORS[i % len(COLORS)]
            line = [at(p) for p in part["pts"]]
            if len(line) > 1:
                draw.line(line, fill=color, width=1)
        for p in d["corners"]:
            x, y = at(p)
            draw.ellipse([x - 3, y - 3, x + 3, y + 3], fill=(0, 170, 0))
        for p in d["spikes"]:
            x, y = at(p)
            draw.ellipse([x - 6, y - 6, x + 6, y + 6], outline=(230, 0, 230), width=2)
        draw.text((ox + 8, oy + 6), name, fill=(0, 0, 0))
    img.save(args.out)
    print("wrote", args.out)


if __name__ == "__main__":
    main()
