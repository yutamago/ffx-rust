#!/usr/bin/env bash
# drawdiff gate for a replay run (tools/draw/replay.sh): each replayed
# frame against the capture it came from, scene part, with the scene's
# allow list. Usage: tools/draw/diff-run.sh <run name> [reference dir...]
# (reference dirs: the captures replay.sh was given as extras).
set -uo pipefail
repo="$(cd "$(dirname "$0")/../.." && pwd)"
run="$HOME/ffx-work/m4/captures/${1:?run name}"
shift
allow="$repo/crates/ffx-drawdiff/allow/cdsp0000.txt"
drawdiff="$repo/target/release/drawdiff"
status=0
pair() {
    echo "== $1 vs $2"
    "$drawdiff" "$1" "$2/replay.ffxgfx" --part scene --allow "$allow" | tail -1
    local s=${PIPESTATUS[0]}
    echo "exit $s"
    [[ $s == 0 ]] || status=1
}
pair "$run/cap" "$run/replay"
pair "$run/cap" "$run/replay-later"
pair "$run/cap2" "$run/replay2"
for ref in "$@"; do
    pair "$ref" "$run/replay-$(basename "$ref")"
done
exit $status
