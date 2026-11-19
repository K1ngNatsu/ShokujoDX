# Draws contact sheets of the library items (the review of P17, lane 2):
# the motifs, the program-split patterns, the carving stamps, the
# decorative borders and the clipart items of the content families (ADR
# 0118), each kind as one picture with the name of each item.
#
# Look at the sheets after a change of a content family or of a built-in
# item. The program does not make these pictures.
#
# Use:
#   emb-cli content items.json
#   python tools/content-sheet.py items.json <folder>
#
# The folder gets motifs.png, splits.png, stamps.png, borders.png and
# clipart.png. Needs Pillow (pip install pillow).

import json
import os
import sys

from PIL import Image, ImageDraw, ImageFont

INK = (30, 60, 150)
POINT = (200, 30, 30)
FRAME = (220, 220, 220)


def font():
    try:
        return ImageFont.truetype("arial.ttf", 13)
    except OSError:
        return ImageFont.load_default()


def sheet(count, columns, width, height):
    rows = (count + columns - 1) // columns
    image = Image.new("RGB", (columns * width, max(rows, 1) * height), "white")
    return image, ImageDraw.Draw(image)


def cell(index, columns, width, height, draw, name, text):
    x, y = (index % columns) * width, (index // columns) * height
    draw.rectangle([x, y, x + width - 1, y + height - 1], outline=FRAME)
    draw.text((x + 4, y + 2), name, fill="black", font=text)
    return x, y


def motifs(items, text):
    """Each motif three times along a line, with its needle points."""
    columns, width, height = 8, 220, 110
    image, draw = sheet(len(items), columns, width, height)
    for i, m in enumerate(items):
        x0, y0 = cell(i, columns, width, height, draw, m["name"], text)
        length = 62
        for repeat in range(3):
            p = [(x0 + 16 + (repeat + x) * length, y0 + 62 + y * length) for x, y in m["points"]]
            draw.line(p, fill=INK, width=2)
            for q in p:
                draw.ellipse([q[0] - 1.5, q[1] - 1.5, q[0] + 1.5, q[1] + 1.5], fill=POINT)
    return image


def splits(items, text):
    """Each pattern as the needle points of its rows, 0.4 mm from one row
    to the next, 9 pixels for each mm."""
    columns, width, height, mm = 8, 200, 190, 9.0
    image, draw = sheet(len(items), columns, width, height)
    for i, s in enumerate(items):
        x0, y0 = cell(i, columns, width, height, draw, "%s (%g mm)" % (s["name"], s["width"]), text)
        rows = s["rows"]
        row = 0
        while row * 0.4 * mm < height - 30:
            y = y0 + 22 + row * 0.4 * mm
            draw.line([(x0 + 8, y), (x0 + width - 8, y)], fill=(205, 215, 235), width=1)
            tile = 0
            while tile * s["width"] * mm < width - 16:
                for part in rows[row % len(rows)]:
                    x = x0 + 8 + (tile + part) * s["width"] * mm
                    if x < x0 + width - 8:
                        draw.ellipse([x - 1.6, y - 1.6, x + 1.6, y + 1.6], fill=(20, 40, 120))
                tile += 1
            row += 1
    return image


def stamps(items, text):
    columns, width, height, side = 8, 150, 170, 120
    image, draw = sheet(len(items), columns, width, height)
    for i, s in enumerate(items):
        x0, y0 = cell(i, columns, width, height, draw, s["name"], text)
        draw.rectangle([x0 + 15, y0 + 30, x0 + 15 + side, y0 + 30 + side], outline=(235, 235, 235))
        for line in s["lines"]:
            draw.line([(x0 + 15 + (x + 0.5) * side, y0 + 30 + (y + 0.5) * side) for x, y in line], fill=INK, width=2)
    return image


def borders(items, text):
    """Each border as its stitches round a box of 34 x 20 mm."""
    columns, width, height = 6, 260, 190
    image, draw = sheet(len(items), columns, width, height)
    for i, b in enumerate(items):
        x0, y0 = cell(i, columns, width, height, draw, "%s (%s)" % (b["name"], b["shape"]), text)
        points = [p for line in b["lines"] for p in line]
        if not points:
            continue
        wide = max(abs(p[0]) for p in points) * 2
        high = max(abs(p[1]) for p in points) * 2
        scale = min((width - 30) / max(wide, 1), (height - 45) / max(high, 1))
        for line in b["lines"]:
            draw.line([(x0 + width / 2 + x * scale, y0 + 105 + y * scale) for x, y in line], fill=INK, width=1)
    return image


def clipart(items, text):
    columns, width, height = 8, 190, 190
    image, draw = sheet(len(items), columns, width, height)
    for i, c in enumerate(items):
        x0, y0 = cell(i, columns, width, height, draw, c["name"], text)
        points = [p for o in c["objects"] for ring in (o.get("fill") or o.get("run")) for p in ring]
        wide = max(abs(p[0]) for p in points) * 2
        high = max(abs(p[1]) for p in points) * 2
        scale = min(160 / max(wide, 1), 150 / max(high, 1), 3.0)
        at = lambda p: (x0 + width / 2 + p[0] * scale, y0 + 105 + p[1] * scale)
        for o in c["objects"]:
            color = tuple(o["color"])
            if "fill" in o:
                for ring in o["fill"]:
                    draw.polygon([at(p) for p in ring], fill=color)
                for ring in o["holes"]:
                    draw.polygon([at(p) for p in ring], fill="white")
            else:
                for ring in o["run"]:
                    draw.line([at(p) for p in ring], fill=color, width=2)
    return image


def main():
    if len(sys.argv) != 3:
        sys.exit("use: python tools/content-sheet.py <items.json> <folder>")
    with open(sys.argv[1], encoding="utf-8") as f:
        items = json.load(f)
    folder = sys.argv[2]
    os.makedirs(folder, exist_ok=True)
    text = font()
    for kind, draw in [("motifs", motifs), ("splits", splits), ("stamps", stamps), ("borders", borders), ("clipart", clipart)]:
        path = os.path.join(folder, kind + ".png")
        draw(items[kind], text).save(path)
        print("wrote %s: %d items" % (path, len(items[kind])))


if __name__ == "__main__":
    main()
