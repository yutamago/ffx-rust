#!/bin/bash
# Runs every replay with the Rust replacements and compares it with the
# original's recorded run (docs/harness.md): events for the traces, pixels
# for the screenshots. The replays run side by side, each in its own game
# slot (instance, port, home, output folder). The original runs are recorded
# once with
#   tools/replays/run-all.sh --record
# (twice: screenshots are compared outside what differs between them).
#
# Each call writes into its own folder, %TEMP%\ffx-snap\runs\<id>\<tag>\
# (traces, screenshots, the replay's output), kept for inspection; the
# recorded originals stay where the comparisons find them:
# ~/ffx-work/replay-shots/traces/<replay>-orig.trace, -orig2.trace and
# ~/ffx-work/replay-shots/orig, orig2 (FFX_REPLAY_SHOTS). Only the newest
# KEEP_RUNS run folders are kept; %TEMP%\ffx-snap is otherwise disposable.
#
# FFX_REPLAY_SERIAL=1 runs the replays one after the other instead.
set -u
repo="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$repo"
export FFX_GAME_DIR="${FFX_GAME_DIR:-$HOME/steam-games/FFX-X2-HD}"
temp_win="$(powershell.exe -NoProfile -Command '[IO.Path]::GetTempPath()' | tr -d '\r')"
temp_win="${temp_win%\\}"
win="$temp_win\\ffx-snap"
lin="$(wslpath "$win")"
shots="${FFX_REPLAY_SHOTS:-$HOME/ffx-work/replay-shots}"  # outlives a WSL restart (/tmp does not)
id="$(date +%Y%m%d-%H%M%S)-$$"
runs="$lin/runs/$id"
traces="$shots/traces"
mkdir -p "$traces"
# older recordings lived in %TEMP%\ffx-snap: take them over once
for f in "$lin"/*-orig.trace "$lin"/*-orig2.trace; do
    [ -f "$f" ] && [ ! -f "$traces/$(basename "$f")" ] && cp "$f" "$traces/"
done
# keep the newest KEEP_RUNS run folders (each is ~30 MB of screenshots)
ls -1dt "$lin"/runs/*/ 2>/dev/null | tail -n +"$((${KEEP_RUNS:-20} + 1))" | while read -r d; do rm -rf "$d"; done
replays=(field-walk battle-attack menu-browse)

# Build once, not three times side by side.
tools/build-hook.sh >/dev/null || exit 1
cargo build -q --release -p ffx-harness --example replay || exit 1
replay="$repo/target/release/examples/replay"

run() { # replay, tag, [original]
    local dir="$runs/$2"
    mkdir -p "$dir"
    timeout 900 "$replay" "tools/replays/$1.txt" "$win\\runs\\$id\\$2\\$1-$2.trace" ${3:-} \
        > "$dir/$1.out" 2>&1
    echo "$1 $2: exit $?" >> "$dir/$1.out"
    grep '^hook log:' "$dir/$1.out" | head -6
}
# Runs `run` for each replay with these arguments, side by side unless
# FFX_REPLAY_SERIAL=1.
each() {
    for r in "${replays[@]}"; do
        if [[ "${FFX_REPLAY_SERIAL:-}" == 1 ]]; then
            run "$r" "$@"
        else
            run "$r" "$@" &
        fi
    done
    wait
}
start=$SECONDS
if [ "${1:-}" = --record ]; then
    for tag in orig orig2; do
        each "$tag" original
        rm -rf "${shots:?}/$tag"
        mkdir -p "$shots/$tag"
        for r in "${replays[@]}"; do
            cp "$runs/$tag/$r-$tag.trace" "$traces/$r-$tag.trace"
            cp "$runs/$tag/$r"-*.bmp "$shots/$tag/" 2>/dev/null
        done
    done
    echo "recorded in $((SECONDS - start)) s ($runs)"
    exit 0
fi
each rust
echo "replays took $((SECONDS - start)) s ($runs)"
bad=0
for r in "${replays[@]}"; do
    # the pad word's range (press timing): field-walk 3, menu-browse 0
    case $r in field-walk) ign=(--ignore 3) ;; menu-browse) ign=(--ignore 0) ;; *) ign=() ;; esac
    cmp=(python3 tools/replay-compare.py "$traces/$r-orig.trace" "$runs/rust/$r-rust.trace" --also "$traces/$r-orig2.trace" "${ign[@]}")
    echo "== $r: $("${cmp[@]}" 2>&1 | tail -1)"
    "${cmp[@]}" >/dev/null 2>&1 || bad=1
done
python3 tools/shot-compare.py "$shots/orig" "$shots/orig2" "$runs/rust" --coarse field-walk || bad=1
exit $bad
