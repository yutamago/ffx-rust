#!/bin/bash
# scene-gate.sh <scene> <run A> <run B>: the draw-diff gate of one scene
# (M4, docs/drawdiff.md "Scene gates"). A is the reference (the original),
# B the candidate; runs are folders under ${FFX_CAPTURES:-~/ffx-work/m4/captures}
# made by tools/scene-gate-capture.sh. Two checks, both against the
# scene's allow file crates/ffx-drawdiff/allow/gate/<scene>.txt (ALLOW=
# overrides it):
# - drawdiff: the scene streams (order, states, decoded constants, ring
#   bytes, texture and buffer keys);
# - tools/compare-captures.py: the hashes.txt records (target and back
#   buffer contents, held scene inputs), with the allow file's noise.
# Scenes: bsil0000, bsil0300 (replay besaid), kami0100 (replay kami),
# menu (replay deck), battle (replay battle), draw-field (fw-101, fw-459, besaid-101; as
# ~/ffx-work/m4/draw-field-gate.sh). Exit 0 when both pass, 3 when two
# battle runs loaded in other update counts (rerun).
set -u
repo="$(cd "$(dirname "$0")/.." && pwd)"
[ $# -ge 3 ] || { sed -n '2,15p' "$0"; exit 2; }
scene="$1"; a="$2"; b="$3"
case "$scene" in
    draw-field) caps=(fw-101 fw-459 besaid-101) ;;
    bsil0000 | bsil0300 | kami0100 | menu | battle) caps=("$scene") ;;
    *) echo "unknown scene $scene" >&2; exit 2 ;;
esac
c="${FFX_CAPTURES:-$HOME/ffx-work/m4/captures}"
allow="${ALLOW:-$repo/crates/ffx-drawdiff/allow/gate/$scene.txt}"
[ -f "$allow" ] || { echo "no allow file $allow" >&2; exit 2; }
[ -x "${DRAWDIFF:-}" ] || DRAWDIFF="$repo/target/release/drawdiff"
[ -x "$DRAWDIFF" ] || (cd "$repo" && cargo build -q --release -p ffx-drawdiff) || exit 2
# battle: the load's length (updates from `wait 10` to the phase byte)
# must match, or the runs show other moments of the intro
if [ "$scene" = battle ]; then
    load() {
        awk '/^wait 10 -> /{w=$NF} /^waitfor 0x0112a8e0:1 01 /{print $NF - w; exit}' "$c/$1/replay.out" 2>/dev/null
    }
    la="$(load "$a")"; lb="$(load "$b")"
    echo "battle load: $a ${la:-?} updates, $b ${lb:-?}"
    if [ -z "$la" ] || [ "$la" != "$lb" ]; then
        echo "gate $scene $a $b: NOT COMPARABLE (the battle loads took other update counts: rerun one)"
        exit 3
    fi
fi
out="$(mktemp -d)"
bad=0
for cap in "${caps[@]}"; do
    echo "=== $cap"
    "$DRAWDIFF" "$c/$a/$cap" "$c/$b/$cap" --allow "$allow" --limit "${LIMIT:-2}" > "$out/$cap.txt" 2>&1 || bad=1
    grep -E "^frame|^  (missing|extra|changed|moved) |^    |^result|^error" "$out/$cap.txt" | head -"${LINES_MAX:-30}"
done
python3 -I "$repo/tools/compare-captures.py" "$c/$a" "$c/$b" --allow "$allow" "${caps[@]}" || bad=1
rm -rf "$out"
echo "gate $scene $a $b: $([ $bad = 0 ] && echo PASS || echo FAIL)"
exit $bad
