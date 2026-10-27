#!/bin/bash
# The SWF pilot's round 2 measurements: the pilot's log replayed into
# Ruffle once per drawing variant (stage quality, supersampling, forced
# smoothing), each compared with Iggy (swf-compare), and a table of the
# differing pixels per screen: $PILOT/variants/table.md (PILOT defaults to
# ~/ffx-work/m5/swf-pilot; never in git).
#   tools/swf-pilot/measure.sh [variant...]
# A variant is name=iggy-replay arguments, e.g. ss4='--quality low --ss 4'.
# FFX_SWF_BIN: the folder with iggy-replay and swf-compare (default the
# worktree's target/release/examples; a fork build passes its own).
# The replay is paced to game time (stock Ruffle's getTimer reads the wall
# clock: an unpaced replay that renders slowly takes other menu paths);
# FFX_SWF_REPLAY_ARGS adds arguments to every replay (--virtual-clock).
set -u
repo="$(cd "$(dirname "$0")/../.." && pwd)"
pilot="${FFX_SWF_PILOT:-$HOME/ffx-work/m5/swf-pilot}"
bin="${FFX_SWF_BIN:-$repo/target/release/examples}"
export FFX_GAME_DIR="${FFX_GAME_DIR:-$HOME/steam-games/FFX-X2-HD}"
if [ $# -eq 0 ]; then
    set -- "high=" "low=--quality low" "medium=--quality medium" "best=--quality best" \
        "8x8=--quality 8x8" "8x8linear=--quality 8x8linear" "16x16=--quality 16x16" \
        "16x16linear=--quality 16x16linear" "ss2=--quality low --ss 2" "ss4=--quality low --ss 4" \
        "ss4high=--ss 4"
fi
screens="pilot-esc-idle pilot-esc-open-05 pilot-sys-idle pilot-video pilot-keyboard pilot-load-idle pilot-popup"
for v in "$@"; do
    name="${v%%=*}"
    args="${v#*=}"
    dir="$pilot/variants/$name"
    mkdir -p "$dir"
    echo "iggy-replay $args" > "$dir/args"
    # shellcheck disable=SC2086
    "$bin/iggy-replay" "$pilot/rec/pilot.iggy" "$dir/ruffle" ${FFX_SWF_REPLAY_ARGS:-} $args > "$dir/replay.log" 2>&1 || { echo "$name: replay failed"; continue; }
    "$bin/swf-compare" "$pilot/rec" "$dir/ruffle" "$dir" "$repo/tools/swf-pilot/page.txt" > "$dir/compare.log" 2>&1
    echo "$name done"
done
# the table: % of pixels off by more than 8 levels, per screen and variant
{
    printf '| variant | arguments |'
    for s in $screens; do printf ' %s |' "${s#pilot-}"; done
    printf '\n|---|---|'
    for s in $screens; do printf -- '---|'; done
    printf '\n'
    for d in "$pilot"/variants/*/; do
        n="$(basename "$d")"
        [ -f "$d/stats.tsv" ] || continue
        printf '| %s | `%s` |' "$n" "$(grep -o 'iggy-replay.*' "$d/args" 2>/dev/null)"
        for s in $screens; do
            printf ' %s |' "$(awk -F'\t' -v s="$s" '$1 == s { split($2, a, "%"); print a[1] }' "$d/stats.tsv")"
        done
        printf '\n'
    done
} > "$pilot/variants/table.md"
cat "$pilot/variants/table.md"
