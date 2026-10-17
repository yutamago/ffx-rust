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
# %TEMP%\ffx-snap\<replay>-orig.trace, -orig2.trace and
# /tmp/ffx-replay-shots/orig, orig2.
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
shots=/tmp/ffx-replay-shots
id="$(date +%Y%m%d-%H%M%S)-$$"
runs="$lin/runs/$id"
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
            cp "$runs/$tag/$r-$tag.trace" "$lin/$r-$tag.trace"
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
    cmp=(python3 tools/replay-compare.py "$lin/$r-orig.trace" "$runs/rust/$r-rust.trace" --also "$lin/$r-orig2.trace")
    echo "== $r: $("${cmp[@]}" 2>&1 | tail -1)"
    "${cmp[@]}" >/dev/null 2>&1 || bad=1
done
python3 tools/shot-compare.py "$shots/orig" "$shots/orig2" "$runs/rust" --coarse field-walk || bad=1
exit $bad
