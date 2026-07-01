#!/usr/bin/env bash
# Builds the Linux installers (.deb, .rpm, .AppImage) locally with the optimized
# release profile and collects them in release/_artifacts/.
#
# Requirements: bun, the Rust toolchain, and the Tauri Linux system deps
# (libwebkit2gtk-4.1, libgtk-3, libayatana-appindicator3, librsvg2, patchelf).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

bun install --frozen-lockfile
bun run tauri build --bundles deb,rpm,appimage

OUT="release/_artifacts"
mkdir -p "$OUT"
find src-tauri/target/release/bundle -type f \
  \( -name '*.deb' -o -name '*.rpm' -o -name '*.AppImage' \) \
  -exec cp -v {} "$OUT/" \;

echo
echo "Artifacts collected in $OUT:"
ls -lh "$OUT"
