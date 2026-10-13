#!/usr/bin/env bash
# Launches FFX.exe on Windows straight from the game folder on the WSL
# filesystem. No Steam needed: with Steam absent, SteamAPI_Init fails and the
# game runs with achievements off.
#
# Usage: tools/run-game.sh [--no-hooks]
#   default     builds and installs the ffx-hook proxy as DINPUT8.dll
#   --no-hooks  removes it and runs the unmodified game
set -euo pipefail
repo="$(cd "$(dirname "$0")/.." && pwd)"
game="${FFX_GAME_DIR:-$HOME/steam-games/FFX-X2-HD}"

# Windows only loads images from the WSL filesystem if they are executable.
chmod +x "$game"/*.exe "$game"/*.dll
# The WSL filesystem is case-sensitive; FFX.exe imports these in upper case.
ln -f "$game/msvcr110.dll" "$game/MSVCR110.dll"
ln -f "$game/msvcp110.dll" "$game/MSVCP110.dll"

if [[ "${1:-}" == "--no-hooks" ]]; then
    rm -f "$game/DINPUT8.dll"
else
    "$repo/tools/build-hook.sh" >/dev/null
    install -m 755 "$repo/target/i686-pc-windows-msvc/release/dinput8.dll" "$game/DINPUT8.dll"
    echo "hooks installed; log: $game/ffx-hook.log"
fi

cd "$game"
./FFX.exe &
echo "FFX.exe started (pid $!)"
