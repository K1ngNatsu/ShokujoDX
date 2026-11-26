"""A cross-check of the writers of machine files (P22 Milestone 5, ADR 0172).

The tool writes each design of the corpus in each format that the program
writes (with emb-cli), reads the files with pyembroidery (an open-source
library under the MIT license, not a part of the program), and compares
the needle points (to 0.1 mm) and the color changes with the stitch plan
of the program (the JSON stitch list of emb-cli). The report gives the
reason of each difference.

Needs pyembroidery: pip install pyembroidery (a download; ask the owner
first). Build emb-cli first: cargo build --release -p emb-cli.

Usage:
    python tools/check-writers.py [--out out/check-writers]
        [--formats dst,pes,...] [--designs name,name] [--cli <emb-cli>]
"""

import argparse
import json
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FORMATS = ["dst", "vp3", "pes", "pec", "exp", "jef", "xxx", "tbf", "tap", "u01"]
# The tolerance of a needle point, mm: the 0.1 mm unit of the files.
TOLERANCE_MM = 0.15


def cli_path(given):
    if given:
        return given
    exe = "emb-cli.exe" if os.name == "nt" else "emb-cli"
    return os.path.join(ROOT, "target", "release", exe)


def run(cli, *args):
    r = subprocess.run([cli, *args], capture_output=True, text=True)
    if r.returncode != 0:
        raise RuntimeError("emb-cli %s: %s" % (" ".join(args), r.stderr.strip()))
    return r.stdout


def plan_of(path):
    """The needle points (mm), the number of color changes and the number
    of stops of a JSON stitch list of emb-cli."""
    with open(path, encoding="utf-8") as f:
        data = json.load(f)
    points = [tuple(r["pos"]) for r in data["records"] if r["kind"] == "stitch"]
    changes = sum(1 for r in data["records"] if r["kind"] == "color_change")
    stops = sum(1 for r in data["records"] if r["kind"] == "stop")
    return points, changes, stops


def read_with_pyembroidery(pyemb, path):
    """The needle points (mm), the number of color changes and the number
    of stops that pyembroidery reads. A needle set after the first is a
    color change too (the formats of multi-needle machines)."""
    pattern = pyemb.read(path)
    if pattern is None:
        raise RuntimeError("pyembroidery cannot read the file")
    points, changes, needles, stops = [], 0, 0, 0
    for x, y, cmd in pattern.stitches:
        c = cmd & pyemb.COMMAND_MASK
        if c == pyemb.STITCH:
            points.append((x / 10.0, y / 10.0))
        elif c == pyemb.COLOR_CHANGE:
            changes += 1
        elif c == pyemb.STOP:
            stops += 1
        elif c == getattr(pyemb, "NEEDLE_SET", -1):
            needles += 1
    if needles > 1 and changes == 0:
        changes = needles - 1
    return points, changes, stops


def compare(ours, theirs):
    """None if the needle points agree (after a move of the whole design
    and a turn of the y axis, which the report gives), else the reason."""
    (p, c, stops), (q, d, t) = ours, theirs
    notes = []
    if (d, t) != (c, stops) and d + t == c + stops:
        # The same commands, read as the other kind.
        if t == 0:
            # A format with no stop writes a stop as a color change.
            notes.append("%d stops read as color changes" % stops)
        elif d == 0:
            # PEC: threads of one palette color; a change to the same
            # color is a stop.
            notes.append("%d color changes to the same palette color read as stops" % c)
        else:
            # JEF: a stop is a change to color 0, the next stop a change
            # back to the color of the thread (as pyembroidery writes it),
            # which pyembroidery reads as a color change.
            notes.append("%d stops and %d color changes read as %d and %d" % (stops, c, t, d))
        d = c
    if len(p) != len(q):
        return "needle points: %d in the plan, %d read" % (len(p), len(q))
    if not p:
        return None if c == d else "color changes: %d in the plan, %d read" % (c, d)
    best = None
    for flip in (1.0, -1.0):
        dx = q[0][0] - p[0][0]
        dy = q[0][1] - flip * p[0][1]
        worst, at = 0.0, 0
        for i, (a, b) in enumerate(zip(p, q)):
            e = max(abs(b[0] - (a[0] + dx)), abs(b[1] - (flip * a[1] + dy)))
            if e > worst:
                worst, at = e, i
        if best is None or worst < best[0]:
            best = (worst, at, flip, dx, dy)
    worst, at, flip, dx, dy = best
    if worst > TOLERANCE_MM:
        return "needle point %d differs by %.2f mm (plan %s, read %s)" % (
            at, worst, fmt(p[at]), fmt(q[at]))
    if flip < 0:
        notes.append("y axis turned")
    if abs(dx) > TOLERANCE_MM or abs(dy) > TOLERANCE_MM:
        notes.append("moved by %.1f, %.1f mm" % (dx, dy))
    if c != d:
        return "color changes: %d in the plan, %d read" % (c, d) + (
            " (" + ", ".join(notes) + ")" if notes else "")
    return ("same (" + ", ".join(notes) + ")") if notes else None


def fmt(p):
    return "(%.1f, %.1f)" % p


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--out", default=os.path.join(ROOT, "out", "check-writers"))
    ap.add_argument("--formats", default=",".join(FORMATS))
    ap.add_argument("--designs", default="")
    ap.add_argument("--cli", default="")
    args = ap.parse_args()
    try:
        import pyembroidery as pyemb
    except ImportError:
        sys.exit("needs pyembroidery: pip install pyembroidery (ask the owner first: it is a download)")
    cli = cli_path(args.cli)
    if not os.path.exists(cli):
        sys.exit("build emb-cli first: cargo build --release -p emb-cli")
    formats = [f for f in args.formats.split(",") if f]
    corpus = os.path.join(args.out, "corpus")
    files = os.path.join(args.out, "files")
    os.makedirs(corpus, exist_ok=True)
    os.makedirs(files, exist_ok=True)
    run(cli, "corpus", corpus)
    names = sorted(n[:-len(".embproj")] for n in os.listdir(corpus) if n.endswith(".embproj"))
    if args.designs:
        wanted = set(args.designs.split(","))
        names = [n for n in names if n in wanted]
    lines = []
    totals = {f: [0, 0] for f in formats}
    for name in names:
        design = os.path.join(corpus, name + ".embproj")
        plan_json = os.path.join(files, name + ".json")
        run(cli, "convert", design, plan_json)
        ours = plan_of(plan_json)
        for ext in formats:
            path = os.path.join(files, "%s.%s" % (name, ext))
            totals[ext][0] += 1
            try:
                run(cli, "convert", design, path)
                theirs = read_with_pyembroidery(pyemb, path)
                why = compare(ours, theirs)
            except Exception as e:  # a file that one side cannot write or read
                why = "error: %s" % e
            if why is None or why.startswith("same"):
                totals[ext][1] += 1
            if why is not None:
                lines.append("%s.%s: %s" % (name, ext, why))
    try:
        from importlib.metadata import version
        pyemb_version = version("pyembroidery")
    except Exception:
        pyemb_version = "(version not known)"
    report = ["Cross-check of the writers with pyembroidery %s" % pyemb_version,
              "%d designs, formats %s, tolerance %.2f mm" % (len(names), ",".join(formats), TOLERANCE_MM),
              ""]
    for ext in formats:
        n, ok = totals[ext]
        report.append("%-4s %3d of %3d the same" % (ext, ok, n))
    report.append("")
    report.append("Differences and notes:" if lines else "No differences.")
    report.extend(lines)
    text = "\n".join(report) + "\n"
    with open(os.path.join(args.out, "report.txt"), "w", encoding="utf-8") as f:
        f.write(text)
    print(text)


if __name__ == "__main__":
    main()
