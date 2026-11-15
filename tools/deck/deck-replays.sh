#!/usr/bin/env bash
# Runs tools/replays/run-all.sh's replay lists on the Steam Deck under Proton
# and compares them with the Deck's own references: the original's runs under
# Proton there (M6's Linux baseline; docs/m6.md, "The Steam Deck").
#
# Usage: tools/deck/deck-replays.sh [--extra|--audio] [--record] [replay...]
#   --record            the original code (FFX.exe, the hook's replacements
#                       off), twice, stored as the Deck's references when
#                       done (as run-all.sh --record does)
#   (default)           the Rust side: FFX.exe with the hook's replacements,
#                       or ffx-rust.exe with FFX_HARNESS_EXE=rust
#   --extra, --audio    run-all.sh's other lists
#
# How: this machine runs the harness (target/release/examples/replay) as for
# local replays, with FFX_HARNESS_LAUNCHER=tools/deck/deck-run.sh: each game
# starts on the Deck in our instance and prefix, and its harness port comes
# here through the SSH connection (127.0.0.1:$DECK_HARNESS_PORT). The game
# writes traces, screenshots and captures on the Deck (~/$DECK_WORK/runs/<id>);
# they are copied back to $DECK_LOCAL_WORK/runs/<id> after each replay, and
# run-all.sh compares them there (FFX_REPLAY_COMPARE_ONLY) against
# $DECK_LOCAL_WORK/replay-shots-nrt, replay-shots-extra-nrt and replay-audio.
# One game at a time (the Deck has one slot: $DECK_LOCAL_WORK/game-slot).
# The sandbox homes hold this machine's saves, as for local replays.
#
# DECK_NO_INSTALL=1 skips tools/deck/deck-install.sh (DECK_NO_BUILD=1: install
# without building). DECK_DRY_RUN=1 prints the plan and the commands.
# Exit: run-all.sh's comparison (0 all equal), or 2-4 for setup failures.
set -uo pipefail
# shellcheck source=tools/deck/lib.sh
. "$(dirname "$0")/lib.sh"
deck_load_env
repo="$deck_repo"
cd "$repo" || exit 2

mode=()
record=""
case "${1:-}" in --extra|--audio) mode=("$1"); shift ;; esac
if [[ "${1:-}" == --record ]]; then
    record=1
    shift
fi
names=("$@")
if [[ -n "$record" && "${FFX_HARNESS_EXE:-}" == rust ]]; then
    echo "--record runs the original code: unset FFX_HARNESS_EXE" >&2
    exit 2
fi
deck_need_host

id="$(date +%Y%m%d-%H%M%S)-$$"
local_run="$DECK_LOCAL_WORK/runs/$id"
remote_rel="$DECK_WORK/runs/$id"
mkdir -p "$local_run"

# The list: run-all.sh's own (it checks the names as well).
mapfile -t list < <(FFX_REPLAY_COMPARE_ONLY="$local_run" tools/replays/run-all.sh "${mode[@]}" --list)
if [[ ${#names[@]} -gt 0 ]]; then
    for r in "${names[@]}"; do
        [[ " ${list[*]} " == *" $r "* ]] || { echo "deck-replays: $r is not on this list (${list[*]})" >&2; exit 2; }
    done
    list=("${names[@]}")
fi

if [[ "${DECK_NO_INSTALL:-}" != 1 ]]; then
    "$deck_tools/deck-install.sh" ${DECK_NO_BUILD:+--no-build} || exit 3
fi
if ! deck_dry; then
    cargo build -q --release -p ffx-harness --example replay || exit 2
fi
replay="$repo/target/release/examples/replay"

# The Deck's home, for the Windows paths the game writes to.
deck_home="$(deck_remote paths | sed -n 's/^HOME=//p')"
[[ -n "$deck_home" ]] || { echo "deck-replays: the Deck did not answer" >&2; exit 3; }
remote_abs="$deck_home/$remote_rel"
winpath() { local p="$1"; printf 'Z:%s' "${p//\//\\}"; }

host=""
[[ "${FFX_HARNESS_EXE:-}" == rust ]] && host=1
timeout_s="${FFX_REPLAY_TIMEOUT:-${host:+3600}}"
timeout_s="${timeout_s:-900}"
[[ -n "$host" ]] && export FFX_HARNESS_REPLY_TIMEOUT="${FFX_HARNESS_REPLY_TIMEOUT:-3600}"

# The harness's settings for Deck games: its own slot pool, port, folders.
export FFX_HARNESS_LAUNCHER="$deck_tools/deck-run.sh"
export FFX_GAME_SLOT_DIR="$DECK_LOCAL_WORK/game-slot"
export FFX_GAME_SLOTS=1
export FFX_HARNESS_PORT="$DECK_HARNESS_PORT"
export FFX_HARNESS_DIR="$DECK_LOCAL_WORK/harness"
export FFX_INSTANCES_DIR="$DECK_LOCAL_WORK/instances-local"
# a new prefix's first start takes a minute or more
export FFX_HARNESS_STARTUP="${FFX_HARNESS_STARTUP:-600}"

if [[ -n "$record" ]]; then tags=(orig orig2); else tags=(rust); fi
start=$SECONDS
echo "deck-replays: ${#list[@]} replays x ${#tags[@]} on $(deck_target), $([[ -n "$host" ]] && echo ffx-rust.exe || echo FFX.exe); run $local_run"
for tag in "${tags[@]}"; do
    for r in "${list[@]}"; do
        mkdir -p "$local_run/$tag"
        # one folder per replay on the Deck (the trace's folder, its $OUT):
        # only that one comes back afterwards
        deck_mkdir "$remote_rel/$tag/$r"
        trace="$(winpath "$remote_abs/$tag/$r/$r-$tag.trace")"
        extra_env=()
        if [[ "${mode[*]}" == --audio ]]; then
            extra_env+=("FFX_AUDIO_WAV=$remote_abs/$tag/$r/$r-$tag.wav")
        fi
        orig=()
        [[ -n "$record" ]] && orig=(original)
        cmd=(env "${extra_env[@]}" FFX_REPLAY_TIMEOUT="$timeout_s" timeout 14400 "$replay" "tools/replays/$r.txt" "$trace" "${orig[@]}")
        if deck_dry; then
            printf '[dry]'; printf ' %q' "${cmd[@]}"; echo
            # what the harness then hands the launcher (Game::launch, slot 0)
            launch=(--instance 0 --harness "$DECK_HARNESS_PORT" --fast --seed 1 --virtual-clock)
            [[ -n "$record" ]] && launch+=(--hook-disable replacements)
            launch+=(--home "$FFX_HARNESS_DIR/0/home" --background)
            [[ -n "$host" ]] && launch+=(--rust)
            echo "[dry] the harness calls: tools/deck/deck-run.sh ${launch[*]}"
            "$deck_tools/deck-run.sh" "${launch[@]}"
            continue
        fi
        "${cmd[@]}" > "$local_run/$tag/$r.out" 2>&1
        echo "$r $tag: exit $?" >> "$local_run/$tag/$r.out"
        tail -1 "$local_run/$tag/$r.out"
        deck_get "$remote_rel/$tag/$r" "$local_run/$tag"
        # the game's own log from the instance (FAULT and PANIC lines shown)
        if [[ -n "$host" ]]; then
            deck_remote fetchlog 0 ffx-rust.log > "$local_run/$tag/$r.ffx-rust.log"
            grep -h -E '^(overlay calls in all|hitches:|bad_accesses:)' "$local_run/$tag/$r.ffx-rust.log" | sed "s/^/$r: /"
        else
            deck_remote fetchlog 0 ffx-hook.log > "$local_run/$tag/$r.ffx-hook.log"
        fi
        grep -h -A4 -E 'FAULT|PANIC' "$local_run/$tag/$r".ffx-*.log 2>/dev/null | head -12 | sed "s/^/$r log: /"
    done
done
echo "replays took $((SECONDS - start)) s ($local_run)"
deck_dry && exit 0

shots="$DECK_LOCAL_WORK/replay-shots-nrt"
export FFX_REPLAY_SHOTS="$shots"
export FFX_REPLAY_SHOTS_EXTRA="$DECK_LOCAL_WORK/replay-shots-extra-nrt"
export FFX_REPLAY_AUDIO="$DECK_LOCAL_WORK/replay-audio"
FFX_REPLAY_COMPARE_ONLY="$local_run" tools/replays/run-all.sh "${mode[@]}" ${record:+--record} "${names[@]}"
