# Makes the program icon: data/icon/icon-<size>.png and data/icon/app.ico.
#
# The icon is Shokujo, the Weaver Star (ADR 0061): a wooden hoop on a night
# sky with the Milky Way, and at the middle of the dark fabric a gold star
# in satin stitches. A needle goes through the fabric under the star, and a
# gold thread goes from its eye to a point of the star. Small
# sizes (32 pixels and less) have a thicker hoop, a plain star and no
# needle or sky, so that they stay clear.
#
# Use:  python tools/make-icon.py   (needs Pillow)

import math
import os
import random

from PIL import Image, ImageDraw, ImageFilter

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "data", "icon")
SIZES = [16, 24, 32, 48, 64, 128, 256, 512, 1024]
ICO_SIZES = [16, 24, 32, 48, 64, 128, 256]
S = 2048  # drawing size; each icon is made smaller from it

GOLD, GOLD_LIGHT, GOLD_DARK = (232, 180, 70), (255, 228, 150), (168, 110, 30)


def bezier(p0, p1, p2, p3, t):
    u = 1 - t
    return tuple(u**3 * a + 3 * u * u * t * b + 3 * u * t * t * c + t**3 * d for a, b, c, d in zip(p0, p1, p2, p3))


def mix(a, b, f):
    return tuple(round(x + (y - x) * f) for x, y in zip(a, b))


def lerp(p, q, f):
    return (p[0] + (q[0] - p[0]) * f, p[1] + (q[1] - p[1]) * f)


def along(path, f):
    """The point at the part f (0 to 1) of the length of the polyline path."""
    lengths = [math.dist(a, b) for a, b in zip(path, path[1:])]
    goal = f * sum(lengths)
    for (a, b), n in zip(zip(path, path[1:]), lengths):
        if goal <= n or n == 0:
            return lerp(a, b, goal / n if n else 0)
        goal -= n
    return path[-1]


def sky(img, simple):
    """The rounded square: a night sky, the Milky Way and small stars."""
    m = S * 0.05
    bg = Image.new("RGBA", (S, S))
    bd = ImageDraw.Draw(bg)
    for y in range(S):
        bd.line([(0, y), (S, y)], fill=mix((40, 44, 104), (12, 14, 44), y / S) + (255,))
    if not simple:
        # The Milky Way: a band of light from the lower left to the top
        # right, with many small stars in it.
        p0, p1 = (-S * 0.05, S * 0.98), (S * 1.05, S * 0.02)
        band = Image.new("RGBA", (S, S), (0, 0, 0, 0))
        bdr = ImageDraw.Draw(band)
        bdr.line([p0, p1], fill=(150, 160, 255, 90), width=int(S * 0.34))
        bdr.line([p0, p1], fill=(210, 215, 255, 110), width=int(S * 0.12))
        bg.alpha_composite(band.filter(ImageFilter.GaussianBlur(S * 0.05)))
        rnd = random.Random(7)
        sd = ImageDraw.Draw(bg)
        ux, uy = (p1[0] - p0[0]) / math.dist(p0, p1), (p1[1] - p0[1]) / math.dist(p0, p1)
        for i in range(230):
            if i < 170:
                # In the band: near the middle line.
                f, off = rnd.random(), rnd.gauss(0, S * 0.07)
                x, y = p0[0] + (p1[0] - p0[0]) * f - uy * off, p0[1] + (p1[1] - p0[1]) * f + ux * off
            else:
                x, y = rnd.uniform(m, S - m), rnd.uniform(m, S - m)
            r = S * rnd.choice([0.002, 0.003, 0.003, 0.0045])
            a = rnd.randint(120, 240)
            sd.ellipse([x - r, y - r, x + r, y + r], fill=(235, 238, 255, a))
    mask = Image.new("L", (S, S), 0)
    ImageDraw.Draw(mask).rounded_rectangle([m, m, S - m, S - m], radius=S * 0.2, fill=255)
    img.paste(bg, (0, 0), mask)


def star_points(c, R, r):
    """The tips and the inner corners of a five-point star with a tip at the top."""
    tips = [(c[0] + R * math.cos(math.radians(-90 + 72 * k)), c[1] + R * math.sin(math.radians(-90 + 72 * k))) for k in range(5)]
    inner = [(c[0] + r * math.cos(math.radians(-54 + 72 * k)), c[1] + r * math.sin(math.radians(-54 + 72 * k))) for k in range(5)]
    return tips, inner


def star_edge(c, R, r, v):
    """The distance from the center c to the edge of the star along the direction v."""
    tips, inner = star_points(c, R, r)
    outline = [p for pair in zip(tips, inner) for p in pair]
    best = 0.0
    for a, b in zip(outline, outline[1:] + outline[:1]):
        ex, ey = b[0] - a[0], b[1] - a[1]
        den = v[0] * ey - v[1] * ex
        if abs(den) < 1e-9:
            continue
        ax, ay = a[0] - c[0], a[1] - c[1]
        t = (ax * ey - ay * ex) / den
        s = (ax * v[1] - ay * v[0]) / den
        if t > 0 and 0 <= s <= 1:
            best = max(best, t)
    return best


def hole(img, q, u, w):
    """The small hole where the needle goes into the fabric: dark, with a faint lip."""
    n = (-u[1], u[0])
    def ell(rx, ry, dx=0.0, dy=0.0):
        return [(q[0] + dx + u[0] * rx * math.cos(t) + n[0] * ry * math.sin(t), q[1] + dy + u[1] * rx * math.cos(t) + n[1] * ry * math.sin(t))
                for t in [k * math.pi / 24 for k in range(48)]]
    layer = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    ld = ImageDraw.Draw(layer)
    ld.polygon(ell(w * 1.6, w * 1.35), fill=(80, 90, 160, 45))
    ld.polygon(ell(w * 1.1, w * 0.95), fill=(8, 10, 30, 190))
    img.alpha_composite(layer.filter(ImageFilter.GaussianBlur(S * 0.003)))

def needle_part(img, p, q, w, point, eye):
    """One part of the needle that shows above the fabric, from p to q. With
    point, p is the sharp point; with eye, q is the round end with the eye."""
    u = ((q[0] - p[0]) / math.dist(p, q), (q[1] - p[1]) / math.dist(p, q))
    n = (-u[1], u[0])
    if point:
        back = (p[0] + u[0] * S * 0.07, p[1] + u[1] * S * 0.07)
        shape = [p, (back[0] + n[0] * w, back[1] + n[1] * w), (q[0] + n[0] * w, q[1] + n[1] * w), (q[0] - n[0] * w, q[1] - n[1] * w), (back[0] - n[0] * w, back[1] - n[1] * w)]
    else:
        back = p
        shape = [(p[0] + n[0] * w, p[1] + n[1] * w), (q[0] + n[0] * w, q[1] + n[1] * w), (q[0] - n[0] * w, q[1] - n[1] * w), (p[0] - n[0] * w, p[1] - n[1] * w)]
    sh = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    ImageDraw.Draw(sh).polygon([(x + S * 0.01, y + S * 0.014) for x, y in shape], fill=(0, 0, 0, 120))
    img.alpha_composite(sh.filter(ImageFilter.GaussianBlur(S * 0.008)))
    d = ImageDraw.Draw(img)
    steel, edge, light = (214, 220, 228, 255), (92, 100, 112, 255), (244, 247, 250, 255)
    d.polygon(shape, fill=steel, outline=edge, width=int(S * 0.005))
    if eye:
        d.ellipse([q[0] - w, q[1] - w, q[0] + w, q[1] + w], fill=steel, outline=edge, width=int(S * 0.005))
        d.polygon([(q[0] + n[0] * w * 0.8, q[1] + n[1] * w * 0.8), (q[0] - n[0] * w * 0.8, q[1] - n[1] * w * 0.8),
                   (q[0] - u[0] * w - n[0] * w * 0.8, q[1] - u[1] * w - n[1] * w * 0.8), (q[0] - u[0] * w + n[0] * w * 0.8, q[1] - u[1] * w + n[1] * w * 0.8)], fill=steel)
        e0 = (q[0] - u[0] * S * 0.05, q[1] - u[1] * S * 0.05)
        e1 = (q[0] - u[0] * S * 0.012, q[1] - u[1] * S * 0.012)
        d.line([e0, e1], fill=(60, 66, 78, 255), width=int(S * 0.008))
    # A light line along the needle.
    d.line([(back[0] + n[0] * w * 0.35, back[1] + n[1] * w * 0.35), (q[0] + n[0] * w * 0.35 - u[0] * (S * 0.055 if eye else 0), q[1] + n[1] * w * 0.35 - u[1] * (S * 0.055 if eye else 0))], fill=light, width=int(S * 0.004))


def satin_star(img, c, R, simple):
    """A gold star. Each point is one satin area: stitches go across it."""
    r = R * 0.42
    tips, inner = star_points(c, R, r)
    # The shadow of the star on the fabric.
    outline = [p for pair in zip(tips, inner) for p in pair]
    sh = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    ImageDraw.Draw(sh).polygon([(x + S * 0.008, y + S * 0.012) for x, y in outline], fill=(0, 0, 0, 120))
    img.alpha_composite(sh.filter(ImageFilter.GaussianBlur(S * 0.01)))
    d = ImageDraw.Draw(img)
    if simple:
        d.polygon(outline, fill=GOLD + (255,))
        return tips
    # The stitches are drawn on their own layer and cut to the star, so the
    # edges stay clean.
    layer = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    ld = ImageDraw.Draw(layer)
    ld.polygon(outline, fill=GOLD + (255,))
    lw = S * 0.0105
    for k in range(5):
        # The area of point k: the center, the inner corner before it, the tip and the inner corner after it.
        left = [c, inner[(k - 1) % 5], tips[k]]
        right = [c, inner[k], tips[k]]
        n = 30
        for i in range(n + 1):
            f = (i + 0.5) / (n + 1)
            a, b = along(left, f), along(right, f)
            a, b = lerp(b, a, 1.04), lerp(a, b, 1.04)
            ld.line([a, b], fill=GOLD + (255,), width=int(lw * 1.5))
            ld.line([lerp(a, b, 0.3), lerp(a, b, 0.7)], fill=GOLD_LIGHT + (255,), width=int(lw * 0.5))
            ld.line([a, lerp(a, b, 0.12)], fill=GOLD_DARK + (255,), width=int(lw * 1.0))
            ld.line([lerp(a, b, 0.88), b], fill=GOLD_DARK + (255,), width=int(lw * 1.0))
        # A thin dark line between two points of the star.
        ld.line([c, inner[k]], fill=GOLD_DARK + (255,), width=int(S * 0.005))
    mask = Image.new("L", (S, S), 0)
    ImageDraw.Draw(mask).polygon(outline, fill=255)
    img.paste(layer, (0, 0), mask)
    ImageDraw.Draw(img).polygon(outline, outline=GOLD_DARK + (255,), width=int(S * 0.005))
    return tips


def draw(simple):
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    sky(img, simple)
    d = ImageDraw.Draw(img)

    c = S / 2
    cy = S / 2
    ring = S * (0.075 if simple else 0.055)
    r_out = S * (0.375 if simple else 0.36)
    r_in = r_out - ring
    # Shadow of the hoop on the background.
    sh = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    ImageDraw.Draw(sh).ellipse([c - r_out, cy - r_out + S * 0.02, c + r_out, cy + r_out + S * 0.02], fill=(0, 0, 0, 130))
    img.alpha_composite(sh.filter(ImageFilter.GaussianBlur(S * 0.015)))
    # Fabric: dark indigo, a little lighter at the middle.
    fab = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    fd = ImageDraw.Draw(fab)
    for i in range(40):
        f = i / 39
        rr = r_in * (1 - f)
        fd.ellipse([c - rr, cy - rr, c + rr, cy + rr], fill=mix((22, 26, 66), (44, 52, 112), f) + (255,))
    img.alpha_composite(fab)

    # The Weaver Star is at the middle of the hoop. A needle sews in the
    # fabric across the star: its eye is at the upper right and its point
    # comes up at the lower left. The gold thread goes from the eye to the
    # right point of the star.
    star_c, star_r = (c, cy), r_in * (0.76 if simple else 0.68)
    tips, _ = star_points(star_c, star_r, star_r * 0.42)
    if not simple:
        a = math.radians(-50)
        u = (math.cos(a), math.sin(a))
        w = S * 0.011
        # The eye end lies over the hoop, so the needle can be long.
        eye_end = (c + u[0] * S * 0.345, cy + u[1] * S * 0.345)
        tip = (c - u[0] * S * 0.30, cy - u[1] * S * 0.30)
        # The needle goes down into the fabric some way before the star and
        # comes up again a little after it; under the fabric it is hidden.
        t_in = star_edge(star_c, star_r, star_r * 0.42, u) + S * 0.075
        t_out = star_edge(star_c, star_r, star_r * 0.42, (-u[0], -u[1])) + S * 0.04
        down = (c + u[0] * t_in, cy + u[1] * t_in)
        up = (c - u[0] * t_out, cy - u[1] * t_out)
        for q in (down, up):
            hole(img, q, u, w)
        needle_part(img, tip, up, w, point=True, eye=False)

    satin_star(img, star_c, star_r, simple)
    d = ImageDraw.Draw(img)

    # Wooden ring: lighter at the top, with a thin dark edge on each side.
    for i in range(int(ring)):
        f = i / ring
        col = mix((222, 168, 104), (176, 118, 62), abs(f - 0.35) * 1.4)
        r = r_out - i
        d.ellipse([c - r, cy - r, c + r, cy + r], outline=col + (255,), width=2)
    d.ellipse([c - r_out, cy - r_out, c + r_out, cy + r_out], outline=(120, 76, 36, 255), width=int(S * 0.006))
    d.ellipse([c - r_in, cy - r_in, c + r_in, cy + r_in], outline=(120, 76, 36, 255), width=int(S * 0.006))
    # The screw clamp at the top of the hoop.
    cw, ch = S * (0.13 if simple else 0.11), S * (0.075 if simple else 0.06)
    top = cy - r_out - ch * 0.55
    d.rounded_rectangle([c - cw / 2, top, c + cw / 2, top + ch], radius=ch * 0.25, fill=(196, 140, 80, 255), outline=(120, 76, 36, 255), width=int(S * 0.006))
    if not simple:
        d.rounded_rectangle([c - cw * 0.12, top - ch * 0.55, c + cw * 0.12, top + ch * 0.1], radius=ch * 0.12, fill=(190, 196, 204, 255), outline=(96, 104, 114, 255), width=int(S * 0.005))
        # The part of the needle above the fabric, over the hoop, and the
        # gold thread: from the eye in a loop down to the right point of the star.
        needle_part(img, down, eye_end, w, point=False, eye=True)
        d = ImageDraw.Draw(img)
        e = (eye_end[0] - u[0] * S * 0.03, eye_end[1] - u[1] * S * 0.03)
        t1 = tips[1]
        thread = [bezier(e, (e[0] + S * 0.10, e[1] + S * 0.01), (t1[0] + S * 0.11, t1[1] - S * 0.05), t1, k / 40) for k in range(41)]
        d.line(thread, fill=GOLD_DARK + (255,), width=int(S * 0.013), joint="curve")
        d.line(thread, fill=GOLD_LIGHT + (255,), width=int(S * 0.007), joint="curve")
        # Twinkles round the hoop. The Milky Way goes from the lower left to
        # the upper right; the cowherd star of the legend (Altair, lower
        # right) is across it from the upper left.
        for tx, ty, tr in ((0.17 * S, 0.17 * S, S * 0.03), (0.84 * S, 0.84 * S, S * 0.026), (0.86 * S, 0.12 * S, S * 0.018), (0.12 * S, 0.88 * S, S * 0.016)):
            for dx, dy in ((1, 0), (0, 1)):
                d.polygon([(tx - dx * tr, ty - dy * tr), (tx + dy * tr * 0.18, ty + dx * tr * 0.18), (tx + dx * tr, ty + dy * tr), (tx - dy * tr * 0.18, ty - dx * tr * 0.18)], fill=(255, 244, 210, 255))
    return img


def main():
    os.makedirs(OUT, exist_ok=True)
    full, simple = draw(False), draw(True)
    icons = {}
    for size in SIZES:
        src = simple if size <= 32 else full
        icons[size] = src.resize((size, size), Image.LANCZOS)
        icons[size].save(os.path.join(OUT, f"icon-{size}.png"), optimize=True)
    # The .ico keeps each size as its own picture.
    icons[256].save(os.path.join(OUT, "app.ico"), sizes=[(s, s) for s in ICO_SIZES], append_images=[icons[s] for s in ICO_SIZES if s != 256])
    print("wrote", OUT)


if __name__ == "__main__":
    main()
