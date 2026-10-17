#!/bin/bash
# Runs every replay with the Rust replacements and compares it with the
# original's recorded run (docs/harness.md): events for the traces, pixels
# for the menu's screenshots. The original runs are recorded once with
#   tools/replays/run-all.sh --record
# (twice: screenshots are compared outside what differs between them).
set -u
source "$(dirname "$0")/../in-game-slot.sh"
repo="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$repo"
export FFX_GAME_DIR="${FFX_GAME_DIR:-$HOME/steam-games/FFX-X2-HD}"
win='C:\Users\USER\AppData\Local\Temp\ffx-snap'
lin=/mnt/c/Users/USER/AppData/Local/Temp/ffx-snap
log="$FFX_GAME_DIR/ffx-hook.log"
shots="${FFX_REPLAY_SHOTS:-$HOME/ffx-work/replay-shots}"  # outlives a WSL restart (/tmp does not)
run() { # replay, tag, [original]
    local n
    n=$(wc -l < "$log")
    rm -f "$lin/$1-$2.trace" "$lin/$1"-*.bmp
    timeout 900 cargo run -q --release -p ffx-harness --example replay -- \
        "tools/replays/$1.txt" "$win\\$1-$2.trace" ${3:-} >/dev/null 2>&1
    tail -n +"$n" "$log" | grep -A4 -E 'FAULT|PANIC' | head -6
    if ls "$lin/$1"-*.bmp >/dev/null 2>&1; then
        mkdir -p "$shots/$2"
        cp "$lin/$1"-*.bmp "$shots/$2/"
    fi
}
bad=0
if [ "${1:-}" = --record ]; then
    rm -rf "$shots/orig" "$shots/orig2"
    for r in field-walk battle-attack menu-browse; do run "$r" orig original; run "$r" orig2 original; done
    exit 0
fi
rm -rf "$shots/rust"
for r in field-walk battle-attack menu-browse; do
    run "$r" rust
    # the pad word's range (press timing): field-walk 3, menu-browse 0
    case $r in field-walk) ign=(--ignore 3) ;; menu-browse) ign=(--ignore 0) ;; *) ign=() ;; esac
    cmp=(python3 tools/replay-compare.py "$lin/$r-orig.trace" "$lin/$r-rust.trace" --also "$lin/$r-orig2.trace" "${ign[@]}")
    echo "== $r: $("${cmp[@]}" 2>&1 | tail -1)"
    "${cmp[@]}" >/dev/null 2>&1 || bad=1
done
python3 tools/shot-compare.py "$shots/orig" "$shots/orig2" "$shots/rust" --coarse field-walk || bad=1
exit $bad
