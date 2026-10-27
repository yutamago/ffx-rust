#!/usr/bin/env bash
# Launches FFX.exe on Windows straight from the game folder on the WSL
# filesystem. No Steam needed: with Steam absent, SteamAPI_Init fails and the
# game runs with achievements off.
#
# Usage: tools/run-game.sh [--no-hooks] [--lang en|jp|fr|de|it|es|kr|ch]
#   default     builds and installs the ffx-hook proxy as DINPUT8.dll
#   --no-hooks  removes it and runs the unmodified game
#   --lang      game language (default en, or $FFX_LANG)
#   --log-opens log every file the game reads to ffx-hook.log
#   --harness P test harness on 127.0.0.1:P (see crates/ffx-hook/src/harness.rs)
#   --background show the window without taking the focus, behind other windows
#   --home DIR  the game's Documents folder (settings, saves) is DIR (harness only)
#   --hook-disable LIST  hooks not to install (names, comma-separated;
#               "replacements" = every pure replacement, for replay parity)
#   --seed N    seed the random number generators with N instead of the clock
#   --renderer  the external renderer draws the scene (ffx-render.exe, built and copied next to the game)
#   --gfx       graphics layer on (FFX_GFX; crates/ffx-hook/src/gfx)
#   --rust-map  the map world drawn through ffx-draw (FFX_RUST_MAP; with --gfx;
#               crates/ffx-hook/src/gfx/scene.rs, docs/draw.md)
#   --rust-map=build  the same, the supported maps built by the scene builder
#               (FFX_RUST_MAP=build; gfx/map_build.rs, docs/scene-builder.md)
#   --fast      no frame limiter: frames back to back, game time still 1/60 s each
#   --virtual-clock  (with --fast) game time never follows real time: exactly
#               one frame interval per frame, and reads are synchronous; implies
#               --fixed-addresses (for replays)
# FFX_COVERAGE=FILE in the environment: coverage mode (FILE.hits gets the hits)
# FFX_HOOK_HITS=FILE in the environment: hook-hit mode (FILE gets every hook
#               that ran, once; crates/ffx-hook/src/hook_hits.rs)
#   --fixed-addresses  start through ffx-launch.exe with bottom-up ASLR off, so
#               heap and VirtualAlloc addresses repeat between runs
#   --instance N  run from game instance N (tools/instance.sh: a hard-linked
#               copy of the game folder with its own DINPUT8.dll and
#               ffx-hook.log), so harness games run side by side
#   --mod DIR   (with --instance) add DIR as mods/<name of DIR> in the instance
#
# The language lives in Documents\Square Enix\FINAL FANTASY X&X-2 HD
# Remaster\GameSetting.ini, which the Steam launcher normally writes; the game
# itself creates it with Language=jp.
set -euo pipefail
repo="$(cd "$(dirname "$0")/.." && pwd)"
game="${FFX_GAME_DIR:-$HOME/steam-games/FFX-X2-HD}"
hooks=1
lang="${FFX_LANG:-en}"
instance=""
mods=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --no-hooks) hooks=0 ;;
        --lang) lang="$2"; shift ;;
        --log-opens) export FFX_HOOK_LOG_OPENS=1; export WSLENV="${WSLENV:+$WSLENV:}FFX_HOOK_LOG_OPENS" ;;
        --harness) export FFX_HARNESS="$2"; export WSLENV="${WSLENV:+$WSLENV:}FFX_HARNESS"; shift ;;
        --background) export FFX_HARNESS_BACKGROUND=1; export WSLENV="${WSLENV:+$WSLENV:}FFX_HARNESS_BACKGROUND" ;;
        --home) home="$2"; shift ;;
        --fast) export FFX_HARNESS_FAST=1; export WSLENV="${WSLENV:+$WSLENV:}FFX_HARNESS_FAST" ;;
        --virtual-clock) export FFX_HARNESS_VIRTUAL_CLOCK=1; export WSLENV="${WSLENV:+$WSLENV:}FFX_HARNESS_VIRTUAL_CLOCK"; fixed=1 ;;
        --fixed-addresses) fixed=1 ;;
        --instance) instance="$2"; shift ;;
        --mod) mods+=(--mod "$2"); shift ;;
        --call-log) export FFX_HARNESS_CALL_LOG="$2"; export WSLENV="${WSLENV:+$WSLENV:}FFX_HARNESS_CALL_LOG"; shift ;;
        --renderer) export FFX_RENDERER=1; export WSLENV="${WSLENV:+$WSLENV:}FFX_RENDERER:FFX_RENDER_TEST:FFX_GFX_REDIRECT:FFX_GFX_SHARE:FFX_GFX_FENCE:FFX_RENDER_BACKEND:FFX_RENDERER_SKIP:FFX_RENDER_EXPORT_MATERIALS:FFX_RENDER_GLOW_DUMP/p"; renderer=1 ;;
        --gfx) export FFX_GFX="${FFX_GFX:-1}"; export WSLENV="${WSLENV:+$WSLENV:}FFX_GFX:FFX_GFX_REDIRECT" ;;
        --rust-map) export FFX_RUST_MAP="${FFX_RUST_MAP:-1}" FFX_GFX="${FFX_GFX:-1}"; export WSLENV="${WSLENV:+$WSLENV:}FFX_RUST_MAP:FFX_GFX:FFX_GFX_REDIRECT" ;;
        --rust-map=build) export FFX_RUST_MAP=build FFX_GFX="${FFX_GFX:-1}"; export WSLENV="${WSLENV:+$WSLENV:}FFX_RUST_MAP:FFX_GFX:FFX_GFX_REDIRECT" ;;
        --seed) export FFX_HARNESS_SEED="$2"; export WSLENV="${WSLENV:+$WSLENV:}FFX_HARNESS_SEED"; shift ;;
        --hook-disable) export FFX_HOOK_DISABLE="$2"; export WSLENV="${WSLENV:+$WSLENV:}FFX_HOOK_DISABLE"; shift ;;
        *) echo "unknown option $1" >&2; exit 2 ;;
    esac
    shift
done
# FFX_HOOK_ONLY (ffx-dinput8): only these replacements are installed
if [[ -n "${FFX_HOOK_ONLY:-}" ]]; then
    export WSLENV="${WSLENV:+$WSLENV:}FFX_HOOK_ONLY"
fi
# Coverage mode (crates/ffx-hook/src/coverage.rs): FFX_COVERAGE names the
# candidate list; /p turns the WSL path into a Windows one for the game.
if [[ -n "${FFX_COVERAGE:-}" ]]; then
    export WSLENV="${WSLENV:+$WSLENV:}FFX_COVERAGE/p"
fi
# Hook-hit mode (crates/ffx-hook/src/hook_hits.rs): FFX_HOOK_HITS names the
# file the hits are appended to (a WSL path, best on /mnt/c).
if [[ -n "${FFX_HOOK_HITS:-}" ]]; then
    export WSLENV="${WSLENV:+$WSLENV:}FFX_HOOK_HITS/p"
fi
# FMOD call log (crates/ffx-hook/src/fmod_log.rs): FFX_FMOD_LOG names the
# file every FMOD call is written to (a WSL path, best on /mnt/c).
if [[ -n "${FFX_FMOD_LOG:-}" ]]; then
    export WSLENV="${WSLENV:+$WSLENV:}FFX_FMOD_LOG/p"
fi
# Iggy call log (crates/ffx-hook/src/iggy_log.rs): FFX_IGGY_LOG names the
# file every Iggy call and AS3 callback is written to (a WSL path, best on
# /mnt/c).
if [[ -n "${FFX_IGGY_LOG:-}" ]]; then
    export WSLENV="${WSLENV:+$WSLENV:}FFX_IGGY_LOG/p"
fi
# FMOD's mixed output to a WAV file (crates/ffx-hook/src/audio_wav.rs,
# docs/harness.md "Audio capture"): FFX_AUDIO_WAV names it (a WSL path, best
# on /mnt/c); <file>.marks beside it says where the harness's audiomarks are.
if [[ -n "${FFX_AUDIO_WAV:-}" ]]; then
    export WSLENV="${WSLENV:+$WSLENV:}FFX_AUDIO_WAV/p"
fi
# Without a file, games on the virtual clock run FMOD the same way with no
# output (NOSOUND_NRT, docs/harness.md "Sound in lockstep");
# FFX_AUDIO_NRT=0 keeps the sound card's real-time mixer instead.
if [[ -n "${FFX_AUDIO_NRT:-}" ]]; then
    export WSLENV="${WSLENV:+$WSLENV:}FFX_AUDIO_NRT"
fi
case "$lang" in jp|en|fr|de|it|es|kr|ch) ;; *) echo "unknown language $lang" >&2; exit 2 ;; esac
if [[ -n "$instance" ]]; then
    game="$("$repo/tools/instance.sh" "$instance" ${mods[@]+"${mods[@]}"})"
elif [[ ${#mods[@]} -gt 0 ]]; then
    echo "--mod needs --instance" >&2; exit 2
fi

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
# (On a Windows drive both names are the same file already.)
for dll in msvcr110 msvcp110; do
    if [[ ! "$game/$dll.dll" -ef "$game/${dll^^}.dll" ]]; then
        ln -f "$game/$dll.dll" "$game/${dll^^}.dll"
    fi
done

if [[ $hooks == 0 ]]; then
    rm -f "$game/DINPUT8.dll"
else
    "$repo/tools/build-hook.sh" >/dev/null
    # rm first: never write through a hard link into the game folder.
    rm -f "$game/DINPUT8.dll"
    install -m 755 "$repo/target/i686-pc-windows-msvc/release/dinput8.dll" "$game/DINPUT8.dll"
    if [[ -n "${renderer:-}" ]]; then
        "$repo/tools/build-renderer.sh" >/dev/null
        rm -f "$game/ffx-render.exe"
        install -m 755 "$repo/target/x86_64-pc-windows-msvc/release/ffx-render.exe" "$game/ffx-render.exe"
        for dll in dxcompiler.dll dxil.dll amd_fidelityfx_loader_dx12.dll amd_fidelityfx_upscaler_dx12.dll LICENSE-FidelityFX.txt; do
            if [[ -f "$repo/target/x86_64-pc-windows-msvc/release/$dll" ]]; then
                rm -f "$game/$dll"
                install -m 755 "$repo/target/x86_64-pc-windows-msvc/release/$dll" "$game/$dll"
            fi
        done
    fi
    echo "hooks installed; log: $game/ffx-hook.log"
fi

# A game refuses to start while another holds its single-instance mutex
# (9B8F7468-..., Game_Main). With the harness the hook gives that mutex a
# name per port, so harness games only clash with one running from the same
# folder (same DINPUT8.dll and log) or on the same port (the game slots
# handle that). Without the harness, any game outside the instances holds
# the real mutex, and so does one stuck while exiting (no path then).
# A slot holder's game from this folder may still be exiting when the slot
# passes on (its replay returned before FFX.exe was gone): with the slot
# held, wait up to a minute for it before calling it a clash.
for _ in $(seq 30); do
    games="$(powershell.exe -NoProfile -Command \
        'Get-Process FFX -ErrorAction SilentlyContinue | ForEach-Object { "path=" + $_.Path }; exit 0' | tr -d '\r')"
    here="$(wslpath -w "$game/FFX.exe")"
    instances="$(wslpath -w "${FFX_INSTANCES_DIR:-$HOME/ffx-work/instances}")"
    clash=""
    while IFS= read -r line; do
        [[ "$line" == path=* ]] || continue
        path="${line#path=}"
        if [[ "${path,,}" == "${here,,}" ]]; then
            clash="a game from this folder is running"
        elif [[ -z "${FFX_HARNESS:-}" && ( -z "$path" || "${path,,}" != "${instances,,}"\\* ) ]]; then
            clash="FFX.exe is already running (or stuck exiting)"
        fi
    done <<< "$games"
    [[ "$clash" == "a game from this folder is running" && "${FFX_GAME_SLOT_HELD:-}" == 1 ]] || break
    sleep 2
done
# Still there after a minute with the slot ours: an orphan. Its holder let
# the slot go without ending it (a replay stopped by a signal or a timeout:
# the slot's lock goes with the process, FFX.exe stays), and a game whose
# main thread is stuck (inside a `waitfor` that never matches, say) does not
# answer the harness's `quit` (Game::end_orphan), so it would hold the
# instance folder for good and every later launch in this slot would fail
# here (2026-10-27, replay-flakes3: two such games failed another agent's
# launches three times). Only the slot's holder starts games from this
# folder, so end it by its path and go on.
if [[ "$clash" == "a game from this folder is running" && "${FFX_GAME_SLOT_HELD:-}" == 1 ]]; then
    echo "an orphaned game from this folder is still running; ending it" >&2
    powershell.exe -NoProfile -Command \
        "Get-Process FFX -ErrorAction SilentlyContinue | Where-Object { \$_.Path -ieq '${here//\'/\'\'}' } | Stop-Process -Force; exit 0" \
        > /dev/null 2>&1
    clash=""
    for _ in $(seq 15); do
        games="$(powershell.exe -NoProfile -Command \
            'Get-Process FFX -ErrorAction SilentlyContinue | ForEach-Object { "path=" + $_.Path }; exit 0' | tr -d '\r')"
        clash=""
        while IFS= read -r line; do
            [[ "${line,,}" == "path=${here,,}" ]] && clash="a game from this folder is running and did not end"
        done <<< "$games"
        [[ -z "$clash" ]] && break
        sleep 1
    done
fi
if [[ -n "$clash" ]]; then
    echo "$clash; not starting another" >&2
    exit 1
fi

# The game slots (crates/ffx-harness/src/slot.rs): tests and agents take a
# slot there; don't start a game under a running test. Instance N's slot
# lock is lock-N; the main folder waits until no slot is in use (every
# slot holder also holds "lock", shared).
slots="${FFX_GAME_SLOT_DIR:-$HOME/ffx-work/game-slot}"
if [[ -n "$instance" ]]; then slot_lock="$slots/lock-$instance"; else slot_lock="$slots/lock"; fi
if [[ "${FFX_GAME_SLOT_HELD:-}" != 1 && -f "$slot_lock" ]] && ! flock -n "$slot_lock" true; then
    echo "the game slot is in use ($(cat "$slot_lock")); queue with: target/release/game-slot run -- $0 $*" >&2
    exit 1
fi

cd "$game"
if [[ -n "${fixed:-}" ]]; then
    PATH="$repo/tools/shims:$PATH" XWIN_ARCH=x86,x86_64 flock "${XDG_CACHE_HOME:-$HOME/.cache}/ffx-xwin.lock" cargo xwin build -q --release \
        --manifest-path "$repo/Cargo.toml" --target x86_64-pc-windows-msvc -p ffx-launch >/dev/null
    rm -f "$game/ffx-launch.exe"
    install -m 755 "$repo/target/x86_64-pc-windows-msvc/release/ffx-launch.exe" "$game/ffx-launch.exe"
    pid="$(./ffx-launch.exe FFX.exe | tr -d '\r')"
    echo "FFX.exe started (language $lang, pid $pid, fixed addresses)"
else
    ./FFX.exe > /dev/null 2>&1 &
    echo "FFX.exe started (language $lang, pid $!)"
fi
