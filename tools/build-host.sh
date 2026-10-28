#!/usr/bin/env bash
# Builds ffx-rust.exe (crate ffx-host) for 32-bit Windows (docs/host.md).
# Usage: tools/build-host.sh [extra cargo args]
set -euo pipefail
repo="$(cd "$(dirname "$0")/.." && pwd)"
export PATH="$repo/tools/shims:$PATH"
export XWIN_ARCH=x86,x86_64
cd "$repo"
# One cargo-xwin at a time (tools/build-hook.sh).
mkdir -p "${XDG_CACHE_HOME:-$HOME/.cache}"
flock "${XDG_CACHE_HOME:-$HOME/.cache}/ffx-xwin.lock" cargo xwin build --release --target i686-pc-windows-msvc -p ffx-host "$@"
echo "built: $repo/target/i686-pc-windows-msvc/release/ffx-rust.exe"
