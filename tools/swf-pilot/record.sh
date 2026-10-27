#!/bin/bash
# Records the SWF pilot's Iggy screens (tools/swf-pilot/pilot.txt) on the
# original code with the Iggy call log on (crates/ffx-hook/src/iggy_log.rs),
# through the harness (it waits for a game slot; background mode). The
# screenshots, trace and log are copied to $FFX_SWF_PILOT/rec
# (default ~/ffx-work/m5/swf-pilot; never in git).
#   tools/swf-pilot/record.sh [script]
# FFX_SWF_PILOT_RUST=1 runs the Rust replacements instead (the log then
# also holds GDraw's antialiasing ramp and the first vertex buffers, which
# the hook's GDraw ports note), into rec-rust/.
set -u
repo="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$repo"
export FFX_GAME_DIR="${FFX_GAME_DIR:-$HOME/steam-games/FFX-X2-HD}"
script="${1:-tools/swf-pilot/pilot.txt}"
name="$(basename "$script" .txt)"
out="${FFX_SWF_PILOT:-$HOME/ffx-work/m5/swf-pilot}/rec${FFX_SWF_PILOT_RUST:+-rust}"
mode=original
[ -n "${FFX_SWF_PILOT_RUST:-}" ] && mode=""
temp_win="$(powershell.exe -NoProfile -Command '[IO.Path]::GetTempPath()' | tr -d '\r')"
win="${temp_win%\\}\\ffx-swf-pilot"
lin="$(wslpath "$win")"
rm -rf "$lin"
mkdir -p "$lin" "$out"
tools/build-hook.sh >/dev/null || exit 1
cargo build -q --release -p ffx-harness --example replay || exit 1
FFX_IGGY_LOG="$lin/$name.iggy" timeout 1200 target/release/examples/replay \
    "$script" "$win\\$name.trace" $mode > "$lin/$name.out" 2>&1
rc=$?
cp "$lin"/* "$out/"
echo "$name: exit $rc, $(ls "$lin"/*.bmp 2>/dev/null | wc -l) shots, $(wc -l < "$lin/$name.iggy" 2>/dev/null) log lines -> $out"
exit $rc
