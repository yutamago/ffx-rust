#!/usr/bin/env bash
# Launches FFX.exe on Windows straight from the game folder on the WSL
# filesystem. No Steam needed: with Steam absent, SteamAPI_Init fails and the
# game runs with achievements off.
#
# Usage: tools/run-game.sh [--no-hooks] [--lang en|jp|fr|de|it|es|kr|ch]
#   default     builds and installs the ffx-hook proxy as DINPUT8.dll
#   --no-hooks  removes it and runs the unmodified game
#   --lang      game language (default en)
#   --log-opens log every file the game reads to ffx-hook.log
#   --harness P test harness on 127.0.0.1:P (see crates/ffx-hook/src/harness.rs)
#   --background show the window without taking the focus
#   --home DIR  the game's Documents folder (settings, saves) is DIR (harness only)
#
# The language lives in Documents\Square Enix\FINAL FANTASY X&X-2 HD
# Remaster\GameSetting.ini, which the Steam launcher normally writes; the game
# itself creates it with Language=jp.
set -euo pipefail
repo="$(cd "$(dirname "$0")/.." && pwd)"
game="${FFX_GAME_DIR:-$HOME/steam-games/FFX-X2-HD}"
hooks=1
lang=en
while [[ $# -gt 0 ]]; do
    case "$1" in
        --no-hooks) hooks=0 ;;
        --lang) lang="$2"; shift ;;
        --log-opens) export FFX_HOOK_LOG_OPENS=1; export WSLENV="${WSLENV:+$WSLENV:}FFX_HOOK_LOG_OPENS" ;;
        --harness) export FFX_HARNESS="$2"; export WSLENV="${WSLENV:+$WSLENV:}FFX_HARNESS"; shift ;;
        --background) export FFX_HARNESS_BACKGROUND=1; export WSLENV="${WSLENV:+$WSLENV:}FFX_HARNESS_BACKGROUND" ;;
        --home) home="$2"; shift ;;
        *) echo "unknown option $1" >&2; exit 2 ;;
    esac
    shift
done
case "$lang" in jp|en|fr|de|it|es|kr|ch) ;; *) echo "unknown language $lang" >&2; exit 2 ;; esac

if [[ -n "${home:-}" ]]; then
    mkdir -p "$home"
    export FFX_HARNESS_HOME="$(wslpath -w "$home")"
    export WSLENV="${WSLENV:+$WSLENV:}FFX_HARNESS_HOME"
    docs="$home"
else
    docs="$(wslpath "$(powershell.exe -NoProfile -Command '[Environment]::GetFolderPath("MyDocuments")' | tr -d '\r')")"
fi
settings="$docs/Square Enix/FINAL FANTASY X&X-2 HD Remaster/GameSetting.ini"
if [[ -f "$settings" ]]; then
    sed -i "s/^Language=.*/Language=$lang/" "$settings"
else
    mkdir -p "$(dirname "$settings")"
    printf 'Language=%s\n' "$lang" > "$settings"
fi

# Windows only loads images from the WSL filesystem if they are executable.
chmod +x "$game"/*.exe "$game"/*.dll
# The WSL filesystem is case-sensitive; FFX.exe imports these in upper case.
ln -f "$game/msvcr110.dll" "$game/MSVCR110.dll"
ln -f "$game/msvcp110.dll" "$game/MSVCP110.dll"

if [[ $hooks == 0 ]]; then
    rm -f "$game/DINPUT8.dll"
else
    "$repo/tools/build-hook.sh" >/dev/null
    install -m 755 "$repo/target/i686-pc-windows-msvc/release/dinput8.dll" "$game/DINPUT8.dll"
    echo "hooks installed; log: $game/ffx-hook.log"
fi

cd "$game"
./FFX.exe > /dev/null 2>&1 &
echo "FFX.exe started (language $lang, pid $!)"
