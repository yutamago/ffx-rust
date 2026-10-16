#!/bin/bash
# Runs every replay with the Rust replacements and compares it with the
# original's recorded run (docs/harness.md): events for the traces, pixels
# for the menu's screenshots. The original runs are recorded once with
#   tools/replays/run-all.sh --record
# (twice for the menu, whose screenshots need two original runs).
set -u
repo="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$repo"
export FFX_GAME_DIR="${FFX_GAME_DIR:-$HOME/steam-games/FFX-X2-HD}"
win='C:\Users\USER\AppData\Local\Temp\ffx-snap'
lin=/mnt/c/Users/USER/AppData/Local/Temp/ffx-snap
log="$FFX_GAME_DIR/ffx-hook.log"
shots=/tmp/ffx-replay-shots
run() { # replay, tag, [original]
    local n
    n=$(wc -l < "$log")
    rm -f "$lin/$1-$2.trace" "$lin"/menu-*.bmp
    timeout 900 cargo run -q --release -p ffx-harness --example replay -- \
        "tools/replays/$1.txt" "$win\\$1-$2.trace" ${3:-} >/dev/null 2>&1
    tail -n +"$n" "$log" | grep -A4 -E 'FAULT|PANIC' | head -6
    if ls "$lin"/menu-*.bmp >/dev/null 2>&1; then
        mkdir -p "$shots/$2"
        cp "$lin"/menu-*.bmp "$shots/$2/"
    fi
}
bad=0
if [ "${1:-}" = --record ]; then
    for r in field-walk battle-attack menu-browse; do run "$r" orig original; done
    run menu-browse orig2 original
    exit 0
fi
for r in field-walk battle-attack menu-browse; do
    run "$r" rust
    echo "== $r: $(python3 tools/replay-compare.py "$lin/$r-orig.trace" "$lin/$r-rust.trace" 2>&1 | tail -1)"
    python3 tools/replay-compare.py "$lin/$r-orig.trace" "$lin/$r-rust.trace" >/dev/null 2>&1 || bad=1
done
python3 tools/shot-compare.py "$shots/orig" "$shots/orig2" "$shots/rust" || bad=1
exit $bad
