#!/usr/bin/env bash
# Starts FFX.exe or ffx-rust.exe on the Steam Deck under Proton, from SSH
# (docs/m6.md, "The Steam Deck"). It takes tools/run-game.sh's options, so the
# harness uses it as its launcher (FFX_HARNESS_LAUNCHER=tools/deck/deck-run.sh;
# tools/deck/deck-replays.sh sets that up), and it can be run by hand.
#
# Usage: tools/deck/deck-run.sh [--rust] [--no-hooks] [--instance N] [--harness PORT]
#            [--home DIR] [--fast] [--virtual-clock] [--fixed-addresses] [--seed N]
#            [--hook-disable LIST] [--lang L] [--gfx] [--rust-map[=build]]
#            [--background] [--log-opens] [--call-log FILE]
#
# On the Deck the game runs from instance N (~/$DECK_WORK/instances/N, made
# from the game folder by links, refreshed at every start; default 0), in our
# own Proton prefix (STEAM_COMPAT_DATA_PATH=~/$DECK_WORK/compatdata, never
# app 359870's), with its Documents folder in ~/$DECK_WORK/homes/N: a copy of
# --home DIR (this machine's folder, as the harness prepares it) or else of a
# fresh sandbox home (tools/sandbox/GameSetting.ini). Steam is kept out
# (DECK_STEAM), the sound goes to a null sink (DECK_SOUND), the window to a
# headless gamescope when the Deck has one (DECK_DISPLAY), at nice 19, and
# not at all while the user plays (DECK_FORCE=1 overrides). --harness PORT
# forwards this machine's 127.0.0.1:PORT to the game's harness on the Deck.
#
# Variables passed on to the game as run-game.sh passes them (FFX_HOOK_ONLY,
# FFX_HOST_*, FFX_AUDIO_WAV, ...). Path variables must name Deck paths; those
# starting with / become Z: paths. FFX_HOST_PROFILE=1 (with --rust) writes
# ffx-rust.profile.tsv in the instance (ffx.sym is installed beside it).
#
# DECK_DRY_RUN=1 prints what it would do; returns as soon as the game started.
set -euo pipefail
# shellcheck source=tools/deck/lib.sh
. "$(dirname "$0")/lib.sh"
deck_load_env

exe=ffx
hooks=1
instance=0
harness=""
home=""
lang="${FFX_LANG:-en}"
fixed=""
vars=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --rust) exe=rust ;;
        --no-hooks) hooks=0 ;;
        --instance) instance="$2"; shift ;;
        --harness) harness="$2"; shift ;;
        --home) home="$2"; shift ;;
        --lang) lang="$2"; shift ;;
        --background) vars+=(FFX_HARNESS_BACKGROUND=1) ;;
        --fast) vars+=(FFX_HARNESS_FAST=1) ;;
        --virtual-clock) vars+=(FFX_HARNESS_VIRTUAL_CLOCK=1); fixed=1 ;;
        --fixed-addresses) fixed=1 ;;
        --seed) vars+=("FFX_HARNESS_SEED=$2"); shift ;;
        --hook-disable) vars+=("FFX_HOOK_DISABLE=$2"); shift ;;
        --log-opens) vars+=(FFX_HOOK_LOG_OPENS=1) ;;
        --call-log) vars+=("FFX_HARNESS_CALL_LOG=$2"); shift ;;
        --gfx) vars+=("FFX_GFX=${FFX_GFX:-1}") ;;
        --rust-map) vars+=("FFX_RUST_MAP=${FFX_RUST_MAP:-1}" "FFX_GFX=${FFX_GFX:-1}") ;;
        --rust-map=build) vars+=(FFX_RUST_MAP=build "FFX_GFX=${FFX_GFX:-1}") ;;
        --renderer) echo "deck-run: the external renderer is parked and Windows-only" >&2; exit 2 ;;
        --mod) echo "deck-run: mods are not on the Deck yet" >&2; exit 2 ;;
        *) echo "deck-run: unknown option $1 (usage in the header)" >&2; exit 2 ;;
    esac
    shift
done
[[ "$instance" =~ ^[0-9]+$ ]] || { echo "deck-run: instance must be a number: $instance" >&2; exit 2; }
case "$lang" in jp|en|fr|de|it|es|kr|ch) ;; *) echo "deck-run: unknown language $lang" >&2; exit 2 ;; esac
deck_need_host

# What run-game.sh hands the game through WSLENV, here as VAR=value.
pass=(FFX_HARNESS_CLOTH_ORDER FFX_HOOK_ONLY FFX_COVERAGE FFX_HOOK_HITS FFX_FMOD_LOG FFX_IGGY_LOG
    FFX_AUDIO_WAV FFX_MAGIC_EMU FFX_MAGIC_RUST FFX_RUST_AUDIO FFX_RUST_AUDIO_OUT FFX_AUDIO_NRT
    FFX_GFX_REDIRECT FFX_GFX_BOOT_CAPTURE
    FFX_HOST_GFX_RECORD FFX_HOST_GFX_CAPTURE FFX_HOST_NOTES FFX_HOST_TRACE FFX_HOST_WATCH
    FFX_HOST_DISABLE FFX_HOST_ONLY FFX_HOST_IGGY_OUT FFX_HOST_GFX FFX_HOST_HANG_SECS)
for v in "${pass[@]}"; do
    [[ -n "${!v:-}" ]] && vars+=("$v=${!v}")
done
args=()
if [[ -n "${FFX_HOST_PROFILE:-}" ]]; then
    if [[ $exe == rust ]]; then args+=(--arg --profile --arg ffx-rust.profile.tsv); else
        echo "deck-run: FFX_HOST_PROFILE needs --rust; ignored" >&2
    fi
fi

# The Documents folder: the harness's home, or a fresh sandbox one; the
# language set in it as run-game.sh does.
if [[ -z "$home" ]]; then
    home="$DECK_LOCAL_WORK/manual-home"
    rm -rf "$home"
    mkdir -p "$home/Square Enix/FINAL FANTASY X&X-2 HD Remaster"
    cp "$deck_repo/tools/sandbox/GameSetting.ini" "$home/Square Enix/FINAL FANTASY X&X-2 HD Remaster/GameSetting.ini"
fi
settings="$home/Square Enix/FINAL FANTASY X&X-2 HD Remaster/GameSetting.ini"
if [[ -f "$settings" ]]; then
    sed -i "s/^Language=.*/Language=$lang/" "$settings"
else
    mkdir -p "$(dirname "$settings")"
    printf 'Language=%s\n' "$lang" > "$settings"
fi
deck_put "$home" "$DECK_WORK/homes/$instance"

launch=(launch "$instance" "$exe" --home "homes/$instance")
[[ $hooks == 0 ]] && launch+=(--no-hooks)
[[ -n "$fixed" ]] && launch+=(--fixed)
[[ -n "$harness" ]] && launch+=(--harness "$harness")
deck_remote "${launch[@]}" "${args[@]}" "${vars[@]}"
if [[ -n "$harness" ]]; then
    deck_forward "$harness"
fi
