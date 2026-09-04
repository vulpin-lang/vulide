#!/usr/bin/env bash
# Cross-compile VulIDE for Linux ARM64 (aarch64) from x86-64, static.
#
# Needs:  zig >= 0.14 on PATH,  cargo-zigbuild >= 0.23  (older versions choke on
#         the aarch64 --fix-cortex-a53-843419 linker arg),
#         rustup target add aarch64-unknown-linux-musl
#
# Ships a raw static binary in a .tar.gz rather than an AppImage: no FUSE
# dependency, which matters on minimal ARM systems (Pi, containers).
# Output: dist/VulIDE-<ver>-aarch64-linux.tar.gz
set -euo pipefail

cd "$(dirname "$0")/.."
TARGET=aarch64-unknown-linux-musl
DIST="$(pwd)/../dist"

rustup target add "$TARGET" >/dev/null 2>&1 || true
cargo zigbuild --release --target "$TARGET"

VER="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)-g$(git rev-parse --short HEAD)"
STAGE="$(mktemp -d)/VulIDE"
mkdir -p "$STAGE"
# The release profile + linker already strip this; host `strip` can't read a
# cross-arch ELF anyway.
install -m755 "target/$TARGET/release/vulide" "$STAGE/vulide"
cp packaging/README-linux-arm64.txt "$STAGE/README.txt"

mkdir -p "$DIST"
TGZ="$DIST/VulIDE-${VER}-aarch64-linux.tar.gz"
rm -f "$TGZ"
tar -C "$(dirname "$STAGE")" -czf "$TGZ" VulIDE
( cd "$DIST" && sha256sum "$(basename "$TGZ")" | tee -a SHA256SUMS )
echo "built $TGZ"
