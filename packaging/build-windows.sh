#!/usr/bin/env bash
# Cross-compile VulIDE for Windows x86-64 from Linux — no mingw, no Docker.
#
# Needs:  zig >= 0.14 on PATH,  cargo-zigbuild  (cargo install cargo-zigbuild),
#         rustup target add x86_64-pc-windows-gnullvm
#
# The crate is pure Rust (crossterm handles Windows natively), so zig/lld does
# the whole job. Output: dist/VulIDE-<ver>-x86_64-windows.zip
set -euo pipefail

cd "$(dirname "$0")/.."
TARGET=x86_64-pc-windows-gnullvm
DIST="$(pwd)/../dist"

rustup target add "$TARGET" >/dev/null 2>&1 || true
cargo zigbuild --release --target "$TARGET"

VER="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)-g$(git rev-parse --short HEAD)"
STAGE="$(mktemp -d)/VulIDE"
mkdir -p "$STAGE"
cp "target/$TARGET/release/vulide.exe" "$STAGE/vulide.exe"
cp packaging/README-windows.txt "$STAGE/README.txt"

mkdir -p "$DIST"
ZIP="$DIST/VulIDE-${VER}-x86_64-windows.zip"
rm -f "$ZIP"
( cd "$(dirname "$STAGE")" && zip -r -9 "$ZIP" VulIDE >/dev/null )
( cd "$DIST" && sha256sum "$(basename "$ZIP")" | tee -a SHA256SUMS )
echo "built $ZIP"
