#!/usr/bin/env bash
# Build VulIDE-<ver>-x86_64.AppImage from a static musl binary.
#
# Needs: rustup with the x86_64-unknown-linux-musl target, and appimagetool on
# PATH (or set $APPIMAGETOOL). The crate is pure Rust, so the musl build needs
# no musl-gcc. Output lands in dist/.
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT="$(pwd)"
TARGET=x86_64-unknown-linux-musl
DIST="$ROOT/../dist"
APPDIR="$(mktemp -d)/VulIDE.AppDir"
APPIMAGETOOL="${APPIMAGETOOL:-appimagetool}"

rustup target add "$TARGET" >/dev/null 2>&1 || true
cargo build --release --target "$TARGET"

VER="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)-g$(git rev-parse --short HEAD)"
mkdir -p "$APPDIR/usr/bin" \
         "$APPDIR/usr/share/applications" \
         "$APPDIR/usr/share/icons/hicolor/256x256/apps"

install -m755 "target/$TARGET/release/vulide" "$APPDIR/usr/bin/vulide"
strip "$APPDIR/usr/bin/vulide"

install -m755 packaging/AppRun          "$APPDIR/AppRun"
install -m644 packaging/vulide.desktop  "$APPDIR/vulide.desktop"
install -m644 packaging/vulide.desktop  "$APPDIR/usr/share/applications/vulide.desktop"
install -m644 packaging/vulide.png      "$APPDIR/vulide.png"
install -m644 packaging/vulide.png      "$APPDIR/usr/share/icons/hicolor/256x256/apps/vulide.png"

mkdir -p "$DIST"
OUT="$DIST/VulIDE-${VER}-x86_64.AppImage"
APPIMAGE_EXTRACT_AND_RUN=1 ARCH=x86_64 "$APPIMAGETOOL" "$APPDIR" "$OUT"
( cd "$DIST" && sha256sum "$(basename "$OUT")" | tee -a SHA256SUMS )
echo "built $OUT"
