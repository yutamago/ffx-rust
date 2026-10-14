#!/usr/bin/env bash
# Builds the 64-bit external renderer (ffx-render.exe) for Windows.
# Usage: tools/build-renderer.sh [extra cargo args]
set -euo pipefail
repo="$(cd "$(dirname "$0")/.." && pwd)"
export PATH="$repo/tools/shims:$PATH"
export XWIN_ARCH=x86_64
cd "$repo"
cargo xwin build --release --target x86_64-pc-windows-msvc -p ffx-render "$@"
echo "built: $repo/target/x86_64-pc-windows-msvc/release/ffx-render.exe"
