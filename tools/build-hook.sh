#!/usr/bin/env bash
# Builds the 32-bit dinput8.dll proxy (and the hook smoke test) for FFX.exe.
# Usage: tools/build-hook.sh [extra cargo args]
set -euo pipefail
repo="$(cd "$(dirname "$0")/.." && pwd)"
export PATH="$repo/tools/shims:$PATH"
export XWIN_ARCH=x86,x86_64
cd "$repo"
# One cargo-xwin at a time: concurrent ones race setting up its MSVC CRT
# cache ("Failed to setup MSVC CRT"), e.g. games launched side by side.
mkdir -p "${XDG_CACHE_HOME:-$HOME/.cache}"
flock "${XDG_CACHE_HOME:-$HOME/.cache}/ffx-xwin.lock" cargo xwin build --release --target i686-pc-windows-msvc -p ffx-dinput8 -p ffx-hook --lib --examples "$@"
echo "built: $repo/target/i686-pc-windows-msvc/release/dinput8.dll"
