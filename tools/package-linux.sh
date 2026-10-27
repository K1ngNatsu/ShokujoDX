#!/usr/bin/env bash
# Makes the Linux packages (ADR 0043):
#   dist/shokujodx_<version>_<arch>.deb   for Debian, Ubuntu and Linux Mint
#   dist/<name>-linux-<arch>.AppImage                runs on most other systems
#
# <name> is ShokujoDX-<version> for a commit with the tag v<version>,
# else ShokujoDX-<version>-<commit> (a test build). The .deb gets
# "~<commit>" after the version for a test build.
#
# Build on an old system (Ubuntu 22.04): the program then runs on systems
# with the same or a newer C library. The AppImage needs appimagetool
# (APPIMAGETOOL=<path>, or appimagetool on the PATH); with no appimagetool
# the script makes only the .deb.
#
# Use:  tools/package-linux.sh [--skip-tests]

set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

if [ "${1:-}" != "--skip-tests" ]; then
    cargo test --workspace
fi
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)
build=$(git rev-parse --short=7 HEAD)
if git tag --points-at HEAD | grep -qx "v$version"; then
    name="ShokujoDX-$version"
    deb_version="$version"
else
    name="ShokujoDX-$version-$build"
    deb_version="$version~$build"
fi
cargo build --release -p emb-app
arch=$(uname -m)
deb_arch=$(dpkg --print-architecture 2>/dev/null || echo amd64)

# One file tree for both packages: usr/bin, usr/share.
app="dist/$name-linux/AppDir"
rm -rf "dist/$name-linux"
share="$app/usr/share"
mkdir -p "$app/usr/bin" "$share/applications" "$share/mime/packages" "$share/metainfo" "$share/thumbnailers" \
    "$share/doc/shokujodx/licenses"
install -m 755 target/release/emb-app "$app/usr/bin/shokujodx"
install -m 644 packaging/linux/shokujodx.desktop "$share/applications/"
install -m 644 packaging/linux/shokujodx.xml "$share/mime/packages/"
install -m 644 packaging/linux/shokujodx.metainfo.xml "$share/metainfo/"
# Pictures of design files in the file manager (P11 Milestone 3).
install -m 644 packaging/linux/shokujodx.thumbnailer "$share/thumbnailers/"
for s in 16 24 32 48 64 128 256 512; do
    d="$share/icons/hicolor/${s}x${s}"
    mkdir -p "$d/apps" "$d/mimetypes"
    install -m 644 "data/icon/icon-$s.png" "$d/apps/shokujodx.png"
    install -m 644 "data/icon/icon-$s.png" "$d/mimetypes/application-x-embroidery-project.png"
done

doc="$share/doc/shokujodx"
install -m 644 docs/testing/TESTERS.md "$doc/README.md"
install -m 644 docs/testing/FEEDBACK.md "$doc/FEEDBACK.md"
install -m 644 LICENSE "$doc/licenses/LICENSE-GPL-3.0.txt"
install -m 644 data/licenses/THIRD-PARTY.txt "$doc/licenses/THIRD-PARTY.txt"
for f in data/fonts/*OFL.txt; do
    install -m 644 "$f" "$doc/licenses/TrueType-font-$(basename "$f")"
done
for f in data/fonts/embroidery/*OFL.txt; do
    install -m 644 "$f" "$doc/licenses/embroidery-font-$(basename "$f")"
done
{
    echo "Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/"
    echo "Upstream-Name: ShokujoDX"
    echo
    echo "Files: *"
    echo "License: GPL-3.0-or-later"
    echo " See /usr/share/common-licenses/GPL-3 and licenses/LICENSE-GPL-3.0.txt."
    echo " The fonts have the SIL Open Font License: see the files in licenses/."
} > "$doc/copyright"

# The program must start (it writes its version and stops).
started=$("$app/usr/bin/shokujodx" --version)
case "$started" in *"ShokujoDX $version"*) ;; *) echo "The program did not start: $started"; exit 1 ;; esac

# The .deb. The window system libraries are loaded when the window opens,
# so they are dependencies by name (X11 or Wayland, Vulkan or OpenGL).
deb="dist/$name-linux/deb"
mkdir -p "$deb/DEBIAN"
cp -r "$app/usr" "$deb/"
size=$(du -sk "$deb/usr" | cut -f1)
cat > "$deb/DEBIAN/control" <<EOF
Package: shokujodx
Version: $deb_version
Section: graphics
Priority: optional
Architecture: $deb_arch
Maintainer: ${DEB_MAINTAINER:-ShokujoDX project <shokujodx@users.noreply.invalid>}
Installed-Size: $size
Depends: libc6 (>= 2.35), libgcc-s1, libxkbcommon0, libegl1 | libvulkan1, libwayland-client0 | libx11-6
Recommends: libvulkan1, libegl1, libx11-6, libxcursor1, libxrandr2, libxi6, libxkbcommon-x11-0, libwayland-cursor0, libwayland-egl1, xdg-desktop-portal
Description: Embroidery digitizer
 Draw shapes, lettering and appliqué, or start from a picture or SVG
 artwork. The program makes the stitches and writes the files of
 embroidery machines (DST, PES, VP3, JEF, EXP and more).
EOF
out="dist/shokujodx_${deb_version}_${deb_arch}.deb"
dpkg-deb --build --root-owner-group "$deb" "$out"
echo "Wrote $out"

# The AppImage: the same tree with AppRun, the .desktop file and the icon at the top.
tool="${APPIMAGETOOL:-$(command -v appimagetool || true)}"
if [ -z "$tool" ]; then
    echo "No appimagetool: no AppImage."
    exit 0
fi
cat > "$app/AppRun" <<'EOF'
#!/bin/sh
here="$(dirname "$(readlink -f "$0")")"
exec "$here/usr/bin/shokujodx" "$@"
EOF
chmod 755 "$app/AppRun"
cp packaging/linux/shokujodx.desktop "$app/"
cp data/icon/icon-256.png "$app/shokujodx.png"
image="dist/$name-linux-$arch.AppImage"
ARCH="$arch" "$tool" "$app" "$image"
echo "Wrote $image"
