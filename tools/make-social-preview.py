# Makes the social preview of the repository: data/icon/social-preview.png
# (1280 x 640 pixels). Sites show this picture when a person shares a link
# to the repository. The owner uploads it on GitHub: Settings > General >
# Social preview (a PNG file smaller than 1 MB).
#
# All parts of the picture come from the project:
# - the logo (data/icon/icon-512.png, made by tools/make-icon.py);
# - the name of the program, sewn by the program: `emb-cli mcp` makes
#   lettering in a bundled embroidery font and draws it with the 3D thread
#   look;
# - the tagline of the README, in Lato (data/fonts, OFL).
# No picture comes from other software, and no picture is made by AI.
#
# Use:  cargo build --release -p emb-cli
#       python tools/make-social-preview.py [--out <file.png>]   (needs Pillow)

import argparse
import json
import os
import subprocess
import sys
import tempfile

from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
WIDTH, HEIGHT = 1280, 640
# The colors of the logo (data/icon/icon.svg): the tile and the star.
TILE = (43, 42, 51)
STAR = (156, 98, 220)
TEXT = (236, 234, 242)
SOFT = (176, 172, 190)
# The lettering of the name.
NAME = "ShokujoDX"
NAME_FONT = "Poppins"
NAME_HEIGHT_MM = 20.0
NAME_COLOR = "#c9a8f0"
PX_PER_MM = 10.0
TAGLINE = "Free, open-source embroidery digitizing for Windows, macOS and Linux"
DETAILS = "Lettering  ·  Picture to design  ·  3D stitch view  ·  GPL-3.0"


def emb_cli():
    """The release build of emb-cli."""
    name = "emb-cli.exe" if os.name == "nt" else "emb-cli"
    path = os.path.join(ROOT, "target", "release", name)
    if not os.path.isfile(path):
        sys.exit(f"{path} is not there: run `cargo build --release -p emb-cli` first")
    return path


class Session:
    """A session of `emb-cli mcp` (JSON-RPC, one message on each line)."""

    def __init__(self):
        self.p = subprocess.Popen(
            [emb_cli(), "mcp"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
        )
        self.next_id = 1
        self.request(
            "initialize",
            {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "make-social-preview", "version": "1"},
            },
        )

    def request(self, method, params):
        message = {"jsonrpc": "2.0", "id": self.next_id, "method": method, "params": params}
        self.next_id += 1
        self.p.stdin.write((json.dumps(message) + "\n").encode("utf-8"))
        self.p.stdin.flush()
        line = self.p.stdout.readline()
        if not line:
            sys.exit("emb-cli mcp stopped")
        answer = json.loads(line)
        if "error" in answer:
            sys.exit(f"{method}: {answer['error']}")
        return answer["result"]

    def call(self, tool, **arguments):
        result = self.request("tools/call", {"name": tool, "arguments": arguments})
        text = "".join(c.get("text", "") for c in result.get("content", []))
        if result.get("isError"):
            sys.exit(f"{tool}: {text}")
        return text

    def close(self):
        self.p.stdin.close()
        self.p.wait()


def sewn_name(folder):
    """The name of the program, sewn and drawn with the 3D thread look, on
    a clear background."""
    path = os.path.join(folder, "name.png")
    s = Session()
    s.call("new_design", name="Social preview", discard_changes=True)
    s.call("add_lettering", text=NAME, font=NAME_FONT, height_mm=NAME_HEIGHT_MM, color=NAME_COLOR)
    s.call("picture", path=path, px_per_mm=PX_PER_MM, background="none")
    s.close()
    return Image.open(path).convert("RGBA")


def font(name, size):
    return ImageFont.truetype(os.path.join(ROOT, "data", "fonts", name), size)


def trimmed(img):
    """The picture with no clear border."""
    box = img.getchannel("A").getbbox()
    return img.crop(box) if box else img


def compose(name):
    out = Image.new("RGBA", (WIDTH, HEIGHT), TILE + (255,))
    draw = ImageDraw.Draw(out)
    # A thin line of the star color at the bottom.
    draw.rectangle((0, HEIGHT - 10, WIDTH, HEIGHT), fill=STAR + (255,))

    # The logo at the left, in the middle of the height. Its tile has the
    # color of the background, so the star stands on the page.
    logo = Image.open(os.path.join(ROOT, "data", "icon", "icon-512.png")).convert("RGBA")
    logo_size = 360
    logo = logo.resize((logo_size, logo_size), Image.LANCZOS)
    left = 70
    out.alpha_composite(logo, (left, (HEIGHT - logo_size) // 2))

    # The text column at the right of the logo.
    x = left + logo_size + 50
    room = WIDTH - x - 70
    name = trimmed(name)
    scale = min(1.0, room / name.width, 150 / name.height)
    name = name.resize((round(name.width * scale), round(name.height * scale)), Image.LANCZOS)

    tag_font = font("Lato-Bold.ttf", 34)
    detail_font = font("Lato-Regular.ttf", 26)
    tag_lines = wrap(draw, TAGLINE, tag_font, room)
    line_h = 46
    block = name.height + 40 + line_h * len(tag_lines) + 24 + 34
    y = (HEIGHT - block) // 2
    out.alpha_composite(name, (x, y))
    y += name.height + 40
    for line in tag_lines:
        draw.text((x, y), line, font=tag_font, fill=TEXT + (255,))
        y += line_h
    y += 24
    draw.text((x, y), DETAILS, font=detail_font, fill=SOFT + (255,))
    return out.convert("RGB")


def wrap(draw, text, fnt, width):
    """The lines of a text that fit in a width."""
    lines, line = [], ""
    for word in text.split():
        trial = f"{line} {word}".strip()
        if draw.textlength(trial, font=fnt) <= width or not line:
            line = trial
        else:
            lines.append(line)
            line = word
    if line:
        lines.append(line)
    return lines


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", default=os.path.join(ROOT, "data", "icon", "social-preview.png"))
    args = parser.parse_args()
    with tempfile.TemporaryDirectory() as folder:
        picture = compose(sewn_name(folder))
    picture.save(args.out, optimize=True)
    size = os.path.getsize(args.out)
    print(f"Wrote {args.out} ({picture.width} x {picture.height}, {size / 1024:.0f} KB)")
    if size >= 1024 * 1024:
        sys.exit("The picture is 1 MB or larger: GitHub does not take it")


if __name__ == "__main__":
    main()
