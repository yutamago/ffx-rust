#!/bin/bash
# Runs the 64-bit Windows build of iggy-replay (headless, no window: wgpu
# picks D3D12 or Vulkan) on an Iggy log, in %TEMP%\ffx-swf-win, to check
# the Windows target works.
#   tools/swf-pilot/win-replay.sh <log> [iggy-replay args...]
set -u
repo="$(cd "$(dirname "$0")/../.." && pwd)"
exe="$repo/target/x86_64-pc-windows-msvc/release/examples/iggy-replay.exe"
log="$1"
shift
temp_win="$(powershell.exe -NoProfile -Command '[IO.Path]::GetTempPath()' | tr -d '\r')"
dir="$(wslpath "${temp_win%\\}\\ffx-swf-win")"
mkdir -p "$dir"
cp "$exe" "$log" "$dir/"
vbf="$(wslpath -w "${FFX_GAME_DIR:-$HOME/steam-games/FFX-X2-HD}/data/FFX_Data.vbf")"
cd "$dir" || exit 1
timeout 900 ./iggy-replay.exe "$(basename "$log")" out --vbf "$vbf" "$@"
