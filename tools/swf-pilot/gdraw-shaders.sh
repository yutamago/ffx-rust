#!/bin/bash
# Disassembles GDraw's (Iggy's) shaders from FFX.exe with D3DDisassemble
# (the Windows build of ffx-dxbc's `dxbc --dis`), into
# $FFX_SWF_PILOT/gdraw-shaders.txt (default ~/ffx-work/m5/swf-pilot).
# The shader bytecode lies in FFX.exe's data at file 0x86bd18..0x884298
# (tools/swf-pilot/gdraw-facts.py lists each one).
set -u
repo="$(cd "$(dirname "$0")/../.." && pwd)"
game="${FFX_GAME_DIR:-$HOME/steam-games/FFX-X2-HD}"
out="${FFX_SWF_PILOT:-$HOME/ffx-work/m5/swf-pilot}"
temp_win="$(powershell.exe -NoProfile -Command '[IO.Path]::GetTempPath()' | tr -d '\r')"
dir="$(wslpath "${temp_win%\\}\\ffx-gdraw-shaders")"
mkdir -p "$dir"
cp "$repo/target/x86_64-pc-windows-msvc/release/dxbc.exe" "$dir/"
# the GDraw shader block only
dd if="$game/FFX.exe" of="$dir/gdraw.bin" bs=1 skip=$((0x86bd18)) count=$((0x884298 - 0x86bd18)) status=none
cd "$dir" || exit 1
./dxbc.exe gdraw.bin --dis > "$out/gdraw-shaders.txt" 2>&1
echo "$(grep -c '^==' "$out/gdraw-shaders.txt") shaders -> $out/gdraw-shaders.txt (offsets from file 0x86bd18)"
cd / && rm -rf "$dir"
