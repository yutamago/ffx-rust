#!/usr/bin/env bash
# Prints the environment cargo-xwin builds TARGET with (`eval "$(tools/xwin-env.sh TARGET)"`,
# then plain `cargo build/clippy --target TARGET`). Only cargo-xwin's set-up of
# its MSVC CRT cache races ("Failed to setup MSVC CRT"), so that runs under
# the lock (`cargo xwin env` sets the cache up); the builds then run side by
# side: a lock around every whole build queued agents' builds for 30+ minutes
# (2026-10-29).
set -euo pipefail
target="${1:?TARGET}"
repo="$(cd "$(dirname "$0")/.." && pwd)"
export PATH="$repo/tools/shims:$PATH"
export XWIN_ARCH=x86,x86_64
mkdir -p "${XDG_CACHE_HOME:-$HOME/.cache}"
flock "${XDG_CACHE_HOME:-$HOME/.cache}/ffx-xwin.lock" cargo xwin env --target "$target" 2>/dev/null
