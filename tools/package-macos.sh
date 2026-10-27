#!/usr/bin/env bash
# Makes the macOS package (ADR 0043):
#   dist/<name>-macos.dmg   "ShokujoDX.app" for Apple silicon and Intel Macs
#
# <name> is ShokujoDX-<version> for a commit with the tag v<version>,
# else ShokujoDX-<version>-<commit> (a test build).
#
# Runs on macOS with the Rust targets aarch64-apple-darwin and
# x86_64-apple-darwin. The app has an ad hoc signature only (no Apple
# developer ID): the guide says how to open it the first time.
#
# Use:  tools/package-macos.sh [--skip-tests]

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
else
    name="ShokujoDX-$version-$build"
fi

# Apple silicon needs macOS 11; Intel Macs from macOS 10.15.
MACOSX_DEPLOYMENT_TARGET=11.0 cargo build --release -p emb-app -p emb-thumbnail --target aarch64-apple-darwin
MACOSX_DEPLOYMENT_TARGET=10.15 cargo build --release -p emb-app -p emb-thumbnail --target x86_64-apple-darwin

stage="dist/$name-macos"
app="$stage/ShokujoDX.app"
rm -rf "$stage"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources/licenses"

lipo -create -output "$app/Contents/MacOS/ShokujoDX" \
    target/aarch64-apple-darwin/release/emb-app target/x86_64-apple-darwin/release/emb-app
sed "s/@VERSION@/$version/g" packaging/macos/Info.plist > "$app/Contents/Info.plist"
printf 'APPL????' > "$app/Contents/PkgInfo"

# The icon from the pictures of data/icon.
iconset="dist/AppIcon.iconset"
rm -rf "$iconset"
mkdir -p "$iconset"
for s in 16 32 128 256 512; do
    cp "data/icon/icon-$s.png" "$iconset/icon_${s}x${s}.png"
    cp "data/icon/icon-$((s * 2)).png" "$iconset/icon_${s}x${s}@2x.png"
done
iconutil -c icns -o "$app/Contents/Resources/AppIcon.icns" "$iconset"
rm -rf "$iconset"

cp LICENSE "$app/Contents/Resources/licenses/LICENSE (GPL-3.0).txt"
cp data/licenses/THIRD-PARTY.txt "$app/Contents/Resources/licenses/"
for f in data/fonts/*OFL.txt; do
    cp "$f" "$app/Contents/Resources/licenses/TrueType font $(basename "$f")"
done
for f in data/fonts/embroidery/*OFL.txt; do
    cp "$f" "$app/Contents/Resources/licenses/embroidery font $(basename "$f")"
done

# The Quick Look extension (ADR 0046): pictures of design files in Finder.
appex="$app/Contents/PlugIns/Thumbnails.appex/Contents"
mkdir -p "$appex/MacOS"
sed "s/@VERSION@/$version/g" packaging/macos/thumbnails/Info.plist > "$appex/Info.plist"
for arch in arm64 x86_64; do
    if [ "$arch" = arm64 ]; then min=11.0; else min=10.15; fi
    swiftc -O -parse-as-library -application-extension -module-name Thumbnails \
        -target "$arch-apple-macos$min" \
        -Xlinker -e -Xlinker _NSExtensionMain \
        -framework AppKit -framework QuickLookThumbnailing \
        packaging/macos/thumbnails/ThumbnailProvider.swift -o "dist/Thumbnails-$arch"
done
lipo -create -output "$appex/MacOS/Thumbnails" dist/Thumbnails-arm64 dist/Thumbnails-x86_64
rm -f dist/Thumbnails-arm64 dist/Thumbnails-x86_64
lipo -create -output "$appex/MacOS/emb-thumbnail-helper" \
    target/aarch64-apple-darwin/release/emb-thumbnail-helper target/x86_64-apple-darwin/release/emb-thumbnail-helper

# Ad hoc signatures, inside out: Apple silicon does not start a program with
# no signature, and the extension and its helper need the sandbox.
codesign --force --sign - --entitlements packaging/macos/thumbnails/helper.entitlements "$appex/MacOS/emb-thumbnail-helper"
codesign --force --sign - --entitlements packaging/macos/thumbnails/extension.entitlements "$app/Contents/PlugIns/Thumbnails.appex"
codesign --force --sign - "$app"
codesign --verify --deep --strict "$app"

# The program must start (it writes its version and stops).
started=$("$app/Contents/MacOS/ShokujoDX" --version)
case "$started" in *"ShokujoDX $version"*) ;; *) echo "The program did not start: $started"; exit 1 ;; esac

cp docs/testing/TESTERS.md "$stage/READ ME FIRST.txt"
cp docs/testing/FEEDBACK.md "$stage/FEEDBACK FORM.txt"
ln -s /Applications "$stage/Applications"
dmg="dist/$name-macos.dmg"
rm -f "$dmg"
hdiutil create -volname "ShokujoDX" -srcfolder "$stage" -ov -format UDZO "$dmg"
echo "Wrote $dmg ($(du -h "$dmg" | cut -f1))"
