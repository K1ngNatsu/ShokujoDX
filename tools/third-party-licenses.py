# Writes data/licenses/THIRD-PARTY.txt: the licenses of the Rust crates that
# the program is built from (ADR 0043). The packages copy this file next to
# the program, because the MIT, BSD and Apache licenses need their notices
# in each copy of the program.
#
# For each crate: its name, version, license and authors, and the license
# files of the crate (the same text is written only once). The crates that
# have no license file of their own use the standard text at the end.
# The end also has the bundled data: the embroidery fonts and the thread
# color charts, with their sources.
#
# Use:  python tools/third-party-licenses.py   (after a change of Cargo.lock;
#       a test of emb-app fails if the file does not have all crates)

import json
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "data", "licenses", "THIRD-PARTY.txt")
NAMES = re.compile(r"^(licen[cs]e|copying|notice|copyright|unlicense)", re.I)

# The public repositories of the sources of the thread charts. The key is
# the project name at the start of the "Source:" comment of a .gpl file.
CHART_SOURCES = {
    "pyembroidery": "https://github.com/EmbroidePy/pyembroidery",
    "Ink/Stitch": "https://github.com/inkstitch/inkstitch",
}


def license_files(pkg):
    d = os.path.dirname(pkg["manifest_path"])
    files = []
    for top, dirs, names in os.walk(d):
        rel = os.path.relpath(top, d)
        if rel.count(os.sep) > 1 or any(p in rel.lower() for p in ("test", "example", "bench", "target")):
            continue
        for n in sorted(names):
            # The font licenses of the default fonts of egui are in fonts/.
            if NAMES.match(n) or (rel == "fonts" and n.lower().endswith(".txt")):
                files.append(os.path.join(top, n))
    return sorted(files)


def standard_text(pkgs, spdx, names):
    for p in pkgs:
        for f in license_files(p):
            if any(x in os.path.basename(f).upper() for x in names):
                return open(f, encoding="utf-8", errors="replace").read().strip()
    return None


def bundled_data_lines():
    lines = []
    font_dir = os.path.join(ROOT, "data", "fonts", "embroidery")
    fonts = sorted(n[:-len(".embfont")] for n in os.listdir(font_dir)
                   if n.endswith(".embfont"))
    lines.append("Bundled data in ShokujoDX")
    lines.append("=======================")
    lines.append("")
    lines.append("Embroidery fonts (data/fonts/embroidery)")
    lines.append("----------------------------------------")
    lines.append(f"The program bundles {len(fonts)} embroidery fonts. Each font is")
    lines.append("made from an open font (under the SIL Open Font License 1.1) and")
    lines.append("converted to the .embfont format of the program. A font with a")
    lines.append("Reserved Font Name has a new name. The license text of the source")
    lines.append("font is next to each font as <name>-OFL.txt.")
    lines.append("")
    lines.append("Outline fonts (data/fonts)")
    lines.append("--------------------------")
    lines.append("Anton, Graduate, Lato and Pacifico (SIL Open Font License 1.1),")
    lines.append("with the license text next to each font as <name>-OFL.txt.")
    lines.append("")
    lines.append("Thread color charts (data/threads)")
    lines.append("----------------------------------")
    lines.append("The .gpl palettes come from these public sources (each file names")
    lines.append("its source and commit in a comment at the top). The thread brand")
    lines.append("names are the trademarks of their owners and only name the threads:")
    seen = []
    for top, dirs, names in os.walk(os.path.join(ROOT, "data", "threads")):
        for n in sorted(names):
            if not n.endswith(".gpl"):
                continue
            with open(os.path.join(top, n), encoding="utf-8", errors="replace") as f:
                for line in f:
                    m = re.match(r"#\s*Source:\s*(.+)", line)
                    if m:
                        name = " ".join(m.group(1).split()).split(",")[0].strip()
                        if name not in seen:
                            seen.append(name)
                        break
    for name in seen:
        url = CHART_SOURCES.get(name.split(" (")[0])
        lines.append(f"- {name}" + (f", {url}" if url else ""))
    return lines


def main():
    meta = json.loads(subprocess.run(["cargo", "metadata", "--format-version", "1", "--locked"],
                                     cwd=ROOT, capture_output=True, check=True).stdout)
    used = {n["id"] for n in meta["resolve"]["nodes"]}
    pkgs = sorted((p for p in meta["packages"] if p["id"] in used and p["source"]),
                  key=lambda p: (p["name"], p["version"]))
    texts = {}  # text -> list of crates (texts that differ only in spaces are one)
    first = {}
    lines = []
    for p in pkgs:
        authors = ", ".join(a.split("<")[0].strip() for a in p.get("authors") or []) or "the authors"
        lines.append(f"{p['name']} {p['version']}  ({p.get('license') or 'see its license file'})  {authors}")
        for f in license_files(p):
            t = open(f, encoding="utf-8", errors="replace").read().strip()
            key = " ".join(t.split())
            first.setdefault(key, t)
            texts.setdefault(key, []).append(f"{p['name']} {p['version']}")
    with open(OUT, "w", encoding="utf-8", newline="\n") as o:
        o.write("Third-party software in ShokujoDX\n")
        o.write("============================================\n\n")
        o.write("The program is built from these Rust crates. Each crate has its own\n")
        o.write("license, given by its SPDX expression; where it gives a choice, the\n")
        o.write("program uses the crate under the first license that is not GPL.\n\n")
        o.write("\n".join(lines) + "\n")
        for key, users in texts.items():
            o.write("\n\n" + "-" * 72 + "\n")
            o.write("Used by: " + ", ".join(users) + "\n\n")
            o.write(first[key] + "\n")
        for spdx, keys in (("MIT", ("MIT",)), ("Apache-2.0", ("APACHE",))):
            t = standard_text(pkgs, spdx, keys)
            if t:
                o.write("\n\n" + "-" * 72 + "\n")
                o.write(f"Standard text of {spdx}, for the crates with no license file of their own\n\n")
                o.write(t + "\n")
        o.write("\n\n" + "-" * 72 + "\n\n")
        o.write("\n".join(bundled_data_lines()) + "\n")
    print(f"wrote {OUT}: {len(pkgs)} crates, {len(texts)} license texts")


if __name__ == "__main__":
    sys.exit(main())
