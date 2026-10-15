#!/usr/bin/env bash
# Builds the 32-bit dinput8.dll proxy (and the hook smoke test) for FFX.exe.
# Usage: tools/build-hook.sh [extra cargo args]
set -euo pipefail
repo="$(cd "$(dirname "$0")/.." && pwd)"
export PATH="$repo/tools/shims:$PATH"
export XWIN_ARCH=x86,x86_64
cd "$repo"
cargo xwin build --release --target i686-pc-windows-msvc -p ffx-dinput8 -p ffx-hook --lib --examples "$@"
echo "built: $repo/target/i686-pc-windows-msvc/release/dinput8.dll"
