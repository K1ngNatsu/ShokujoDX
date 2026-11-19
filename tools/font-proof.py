# Draws proof pictures of the letters of an embroidery font (P17 Milestone 5,
# ADR 0113), to look at each letter before a font becomes a tuned font.
#
# For each letter: the outline of the source font (gray; where gray shows,
# the letter has no stitches), the parts in sew order (colors; the number
# of a part is at its start, a dot is at its end), the lines across a column
# (the stitch direction), and the travel runs (red). Above the letter: the
# part of the outline that the parts cover, and the trims in the letter at
# the smallest, middle and largest size.
#
# Use:
#   emb-cli font-audit <font.embfont> --source <font.ttf> --proof proof.json
#   python tools/font-proof.py proof.json out.png [letters] [columns] [cell]
#
# `letters`: only these letters ("-" for all). `columns`: letters in a row
# (8). `cell`: the size of the square of one letter in pixels (230).
# Needs Pillow (pip install pillow).

import json
import math
import sys

from PIL import Image, ImageChops, ImageDraw, ImageFont

COLORS = [
    (31, 119, 180),
    (44, 160, 44),
    (255, 127, 14),
    (148, 103, 189),
    (140, 86, 75),
    (23, 190, 207),
    (188, 189, 34),
    (227, 119, 194),
    (127, 127, 127),
]
RUN = (220, 0, 0, 255)


def along(line, part):
    """The point of the polyline `line` at the part `part` (0 to 1) of its length."""
    seg = [math.dist(line[i], line[i + 1]) for i in range(len(line) - 1)]
    want, done = part * (sum(seg) or 1.0), 0.0
    for i, s in enumerate(seg):
        if done + s >= want and s > 0:
            k = (want - done) / s
            return (line[i][0] + (line[i + 1][0] - line[i][0]) * k, line[i][1] + (line[i + 1][1] - line[i][1]) * k)
        done += s
    return line[-1]


def main():
    src, dst = sys.argv[1], sys.argv[2]
    only = sys.argv[3] if len(sys.argv) > 3 and sys.argv[3] != "-" else None
    cols = int(sys.argv[4]) if len(sys.argv) > 4 else 8
    cell = int(sys.argv[5]) if len(sys.argv) > 5 else 230
    data = json.load(open(src, encoding="utf-8"))
    letters = [l for l in data["letters"] if only is None or l["letter"] in only]
    rows = (len(letters) + cols - 1) // cols
    img = Image.new("RGB", (cols * cell, max(rows, 1) * cell), "white")
    try:
        font = ImageFont.truetype("arial.ttf", 13)
        small = ImageFont.truetype("arial.ttf", 11)
    except OSError:
        font = small = ImageFont.load_default()
    for n, l in enumerate(letters):
        ox, oy = (n % cols) * cell, (n // cols) * cell
        pts = [p for part in l["parts"] for r in part["rings"] for p in r]
        pts += [p for part in l["parts"] for p in part.get("line") or []]
        for r in l.get("outline") or []:
            pts += r
        if not pts:
            continue
        x0, x1 = min(p[0] for p in pts), max(p[0] for p in pts)
        y0, y1 = min(p[1] for p in pts), max(p[1] for p in pts)
        s = min((cell - 30) / max(x1 - x0, 1e-6), (cell - 44) / max(y1 - y0, 1e-6))

        def at(p):
            return (15 + (p[0] - x0) * s, 30 + (p[1] - y0) * s)

        # The outline: outer rings with their holes.
        if l.get("outline"):
            mask = Image.new("L", (cell, cell), 0)
            for r in l["outline"]:
                ring = Image.new("L", (cell, cell), 0)
                ImageDraw.Draw(ring).polygon([at(p) for p in r], fill=255)
                mask = ImageChops.difference(mask, ring)
            img.paste((205, 205, 205), (ox, oy), mask)
        order = 0
        for part in l["parts"]:
            over = Image.new("RGBA", (cell, cell), (0, 0, 0, 0))
            od = ImageDraw.Draw(over)
            if part["kind"] == "run":
                line = [at(p) for p in part["line"]]
                od.line(line, fill=RUN, width=2)
                od.ellipse([line[0][0] - 3, line[0][1] - 3, line[0][0] + 3, line[0][1] + 3], outline=RUN)
                img.paste(over, (ox, oy), over)
                continue
            c = COLORS[order % len(COLORS)]
            order += 1
            if len(part["rings"]) == 1 and part.get("rails"):
                # Strips between the rails, so a column that lies on itself
                # has no hole.
                ra, rb = part["rails"]
                steps = 80
                strip = Image.new("L", (cell, cell), 0)
                sd = ImageDraw.Draw(strip)
                for i in range(steps):
                    quad = [
                        at(along(ra, i / steps)),
                        at(along(ra, (i + 1) / steps)),
                        at(along(rb, (i + 1) / steps)),
                        at(along(rb, i / steps)),
                    ]
                    sd.polygon(quad, fill=255)
                over.paste(c + (95,), (0, 0), strip)
                for line in ([at(p) for p in ra], [at(p) for p in rb], [at(ra[0]), at(rb[0])], [at(ra[-1]), at(rb[-1])]):
                    od.line(line, fill=c + (255,), width=1)
            elif len(part["rings"]) == 1:
                od.polygon([at(p) for p in part["rings"][0]], fill=c + (95,), outline=c + (255,))
            else:
                # A ring column or a fill with holes: the area between its rings.
                area = Image.new("L", (cell, cell), 0)
                for r in part["rings"]:
                    ring = Image.new("L", (cell, cell), 0)
                    ImageDraw.Draw(ring).polygon([at(p) for p in r], fill=255)
                    area = ImageChops.difference(area, ring)
                over.paste(c + (95,), (0, 0), area)
                for r in part["rings"]:
                    od.line([at(p) for p in r] + [at(r[0])], fill=c + (255,), width=1)
            for a, b in part["rungs"]:
                od.line([at(a), at(b)], fill=c + (150,), width=1)
            if part["start"]:
                sx, sy = at(part["start"])
                ex, ey = at(part["end"])
                od.ellipse([ex - 3, ey - 3, ex + 3, ey + 3], fill=c + (255,))
                od.rectangle([sx - 7, sy - 7, sx + 7, sy + 7], fill=(255, 255, 255, 235), outline=c + (255,))
                od.text((sx - 3, sy - 7), str(order), fill=(0, 0, 0, 255), font=small)
            img.paste(over, (ox, oy), over)
        dr = ImageDraw.Draw(img)
        cover = l.get("coverage")
        title = "%s  %s  t%s" % (
            l["letter"],
            "" if cover is None else "%.0f%%" % (cover * 100),
            "/".join(str(t) for t in l["trims"]),
        )
        dr.text((ox + 6, oy + 4), title, fill=(0, 0, 0), font=font)
        flags = " ".join(f for f in l["flags"] if f not in ("wide", "narrow"))
        dr.text((ox + 6, oy + 17), flags, fill=(200, 0, 0), font=small)
        dr.rectangle([ox, oy, ox + cell - 1, oy + cell - 1], outline=(225, 225, 225))
    img.save(dst)
    print("wrote %s: %d letters, %d x %d" % (dst, len(letters), img.size[0], img.size[1]))


if __name__ == "__main__":
    main()
