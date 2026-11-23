# Draws letters of the letter fault bench (phase P21): the outline of the
# source font (gray), the needle points of each column (colors; travel
# runs red), needle points more than 0.4 mm outside the outline (magenta
# rings), open area (orange) and the convex corners of the outline (green
# dots).
#
# Use:
#   emb-cli letter-faults --proof faults.json
#   python tools/letter-faults.py faults.json out.png "Anton ttf M" "Block Sans emb A" ...
#
# The names are "<embroidery font> emb <letter>" (the embroidery font) or
# "<embroidery font> ttf <letter>" (lettering in the source font).
# Needs Pillow (pip install pillow).

import json
import sys

from PIL import Image, ImageDraw

COLORS = [
    (31, 119, 180),
    (44, 160, 44),
    (148, 103, 189),
    (140, 86, 75),
    (23, 190, 207),
    (188, 189, 34),
    (127, 127, 127),
]
CELL = 520


def main():
    data = {d["name"]: d for d in json.load(open(sys.argv[1], encoding="utf-8"))}
    names = sys.argv[3:]
    cols = max(1, min(3, len(names)))
    rows = (len(names) + cols - 1) // cols
    img = Image.new("RGB", (cols * CELL, max(rows, 1) * CELL), "white")
    draw = ImageDraw.Draw(img)
    for k, name in enumerate(names):
        d = data.get(name)
        if d is None:
            print("not in the file:", name)
            continue
        ox, oy = (k % cols) * CELL, (k // cols) * CELL
        pts = [p for r in d["outline"] for p in r]
        xs, ys = [p[0] for p in pts], [p[1] for p in pts]
        x0, y0 = min(xs), min(ys)
        s = (CELL - 60) / max(max(xs) - x0, max(ys) - y0)

        def at(p):
            return (ox + 30 + (p[0] - x0) * s, oy + 40 + (p[1] - y0) * s)

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
    img.save(sys.argv[2])
    print("wrote", sys.argv[2])


if __name__ == "__main__":
    main()
