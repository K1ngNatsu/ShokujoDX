# Builds the browser trial of ShokujoDX (roadmap P15, step 1; branch
# trial/browser) into target/web: the program as WebAssembly, its
# JavaScript glue (wasm-bindgen) and the page web/index.html.
#
# Use:  python tools/build-web.py [--opt] [--serve [<port>]]
#       (needs the Rust target wasm32-unknown-unknown and wasm-bindgen-cli of
#       the version of wasm-bindgen in Cargo.lock; the tool looks for it in
#       target/tools/bin first:
#       cargo install wasm-bindgen-cli --version <v> --locked --root target/tools)
#       --opt makes the program smaller and quicker with wasm-opt (binaryen;
#       target/tools/bin first: cargo install wasm-opt --locked --root
#       target/tools). --serve starts a local web server on the folder (port
#       8765 if not given) for a test in a browser.

import os
import re
import shutil
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "target", "web")
WASM = os.path.join(ROOT, "target", "wasm32-unknown-unknown", "release", "emb-app.wasm")


def tool(name):
    exe = name + ".exe" if os.name == "nt" else name
    local = os.path.join(ROOT, "target", "tools", "bin", exe)
    return local if os.path.isfile(local) else exe


# The features of WebAssembly that Rust uses for the target by default.
WASM_FEATURES = [
    "--enable-bulk-memory",
    "--enable-nontrapping-float-to-int",
    "--enable-sign-ext",
    "--enable-mutable-globals",
    "--enable-reference-types",
    "--enable-multivalue",
]


def lock_version():
    text = open(os.path.join(ROOT, "Cargo.lock"), encoding="utf-8").read()
    m = re.search(r'name = "wasm-bindgen"\nversion = "([^"]+)"', text)
    return m.group(1) if m else "?"


def main():
    args = sys.argv[1:]
    subprocess.run(
        ["cargo", "build", "--release", "--target", "wasm32-unknown-unknown", "-p", "emb-app"],
        cwd=ROOT,
        check=True,
    )
    os.makedirs(OUT, exist_ok=True)
    try:
        subprocess.run([tool("wasm-bindgen"), "--target", "web", "--no-typescript", "--out-dir", OUT, WASM], check=True)
    except FileNotFoundError:
        sys.exit("wasm-bindgen is not found; install wasm-bindgen-cli %s (see the head of this file)" % lock_version())
    if "--opt" in args:
        wasm = os.path.join(OUT, "emb-app_bg.wasm")
        before = os.path.getsize(wasm)
        try:
            subprocess.run([tool("wasm-opt"), "-O2", *WASM_FEATURES, wasm, "-o", wasm], check=True)
        except FileNotFoundError:
            sys.exit("wasm-opt is not found (see the head of this file)")
        print("wasm-opt: %.1f MB to %.1f MB" % (before / 1e6, os.path.getsize(wasm) / 1e6))
    shutil.copyfile(os.path.join(ROOT, "web", "index.html"), os.path.join(OUT, "index.html"))
    icon = os.path.join(ROOT, "data", "icon", "icon-256.png")
    if os.path.isfile(icon):
        shutil.copyfile(icon, os.path.join(OUT, "icon.png"))
    for name in sorted(os.listdir(OUT)):
        size = os.path.getsize(os.path.join(OUT, name))
        print("%-24s %8.1f MB" % (name, size / 1e6))
    if "--serve" in args:
        at = args.index("--serve")
        port = args[at + 1] if at + 1 < len(args) else "8765"
        print("http://localhost:%s/" % port)
        subprocess.run([sys.executable, "-m", "http.server", port, "--directory", OUT])


if __name__ == "__main__":
    main()
