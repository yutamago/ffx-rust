#!/bin/bash
# Runs every replay with the Rust replacements and compares it with the
# original's recorded run (docs/harness.md): events for the traces, pixels
# for the screenshots. The replays run side by side, each in its own game
# slot (instance, port, home, output folder). The original runs are recorded
# once with
#   tools/replays/run-all.sh --record [replay...]
# (twice: screenshots are compared outside what differs between them).
# Named replays (`--record boot`) are recorded alone; the others' recorded
# traces and screenshots stay as they are. boot.txt has no trace ranges
# (screenshots only) and runs at the game's own frame rate (`realtime`), as
# does event-cliff.txt (a voiced cut scene that keeps time with its voice).
#
# Each call writes into its own folder, %TEMP%\ffx-snap\runs\<id>\<tag>\
# (traces, screenshots, the replay's output), kept for inspection; the
# recorded originals stay where the comparisons find them:
# ~/ffx-work/replay-shots/traces/<replay>-orig.trace, -orig2.trace and
# ~/ffx-work/replay-shots/orig, orig2 (FFX_REPLAY_SHOTS). Only the newest
# KEEP_RUNS run folders are kept; %TEMP%\ffx-snap is otherwise disposable.
#
# FFX_REPLAY_SERIAL=1 runs the replays one after the other instead.
#
# The `eye` replays' screenshots are recorded into orig/eye and orig2/eye,
# out of shot-compare.py's sight: random effects (an overdrive's lightning,
# fireflies, the idle pose under a menu) differ in a few pixels between any
# two runs, and the exact comparison would fail every run. Look at them
# (docs/harness.md, battle-overdrive); the run prints where they are.
# Hook-hit mode (docs/harness.md, "Hook-hit mode"):
#   tools/replays/run-all.sh --hits
# runs and compares the replays as usual with FFX_HOOK_HITS set (one hits
# file per replay in the run folder's hits/), then plays
# tools/replays/coverage-session.txt on the Rust side for each save entry in
# FFX_HITS_SAVES (default "0 1 2 3 5", the sandbox's saves), merges every hits file into
# symbols/hook-hits.txt (tools/hook-hits.py) and writes the report to
# FFX_HITS_REPORT (default ~/ffx-work/agents/coverage/hook-hits-report.txt).
# The exit code is the comparison's; the session is not a parity replay.
set -u
repo="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$repo"
. tools/timing/timing.sh && timing_start run-all.sh "$@"  # ~/ffx-work/timing (docs/workflow.md)
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
replays=(field-walk battle-attack menu-browse boot battle-magic battle-overdrive event-besaid event-cliff menu-shop menu-grid menu-save)
eye=(battle-overdrive event-cliff)
# rp-events2 (docs/harness.md): the Thunder Plains agency and the Besaid arrival
replays+=(event-agency event-arrival)
eye+=(event-arrival)  # voiced, real time

# Build once, not three times side by side.
tools/build-hook.sh >/dev/null || exit 1
cargo build -q --release -p ffx-harness --example replay || exit 1
replay="$repo/target/release/examples/replay"

run() { # replay, tag, [original]
    local dir="$runs/$2"
    mkdir -p "$dir"
    ${hits:+env FFX_HOOK_HITS="$hits/$1-$2.hits"} \
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
hits=""
if [ "${1:-}" = --hits ]; then
    shift
    hits="$runs/hits"
    mkdir -p "$hits"
fi
if [ "${1:-}" = --record ]; then
    shift
    [ $# -gt 0 ] && replays=("$@")
    for tag in orig orig2; do
        each "$tag" original
        mkdir -p "$shots/$tag/eye"
        for r in "${replays[@]}"; do
            dest="$shots/$tag"
            [[ " ${eye[*]} " == *" $r "* ]] && dest="$shots/$tag/eye"
            rm -f "${dest:?}/$r"-[0-9]*.bmp
            [ -f "$runs/$tag/$r-$tag.trace" ] && cp "$runs/$tag/$r-$tag.trace" "$traces/$r-$tag.trace"
            cp "$runs/$tag/$r"-*.bmp "$dest/" 2>/dev/null
        done
    done
    echo "recorded in $((SECONDS - start)) s ($runs)"
    exit 0
fi
each rust
echo "replays took $((SECONDS - start)) s ($runs)"
bad=0
for r in "${replays[@]}"; do
    [ "$r" = boot ] && continue  # screenshots only
    if [ ! -f "$traces/$r-orig.trace" ]; then
        echo "== $r: not recorded (tools/replays/run-all.sh --record $r)"; bad=1; continue
    fi
    # The player position in event-besaid (2) differs between original runs
    # (three of three pairs on 2026-10-24, sampled between two updates).
    # The rest that was left out here (the pad words of field-walk and
    # menu-browse, menu-save's ranges and, in hook-hit mode, its 0, 4 and
    # 7) were one-frame in-between values the trace caught while it sampled
    # on the render thread; three original runs, three Rust runs and a
    # hook-hit run agree on all of them now (docs/harness.md, "Frames and
    # threads", the replays' sections).
    # event-cliff runs at the game's own rate: Wakka's position and rotation
    # (6, 7) change every frame, and a frame that falls behind runs more
    # ticks, so the trace can miss values in between (a Rust run of
    # 2026-10-24 missed 4 of 197): compared by order (--sampled).
    # event-agency's player position (2) skipped a value in one of six
    # original runs (lockstep too): compared by order.
    case $r in
        event-besaid) ign=(--ignore 2) ;;
        event-cliff) ign=(--sampled 6,7) ;;
        event-agency) ign=(--sampled 2) ;;
        *) ign=() ;;
    esac
    cmp=(python3 tools/replay-compare.py "$traces/$r-orig.trace" "$runs/rust/$r-rust.trace" --also "$traces/$r-orig2.trace" "${ign[@]}")
    echo "== $r: $("${cmp[@]}" 2>&1 | tail -1)"
    "${cmp[@]}" >/dev/null 2>&1 || bad=1
done
# The Thunder Plains' shots of event-agency (4-6) are compared coarsely, as
# field-walk's: the rain differs between original runs (about 1,000 pixels
# outside the masked chocobo icon in some pairs of five, 2026-10-24).
python3 tools/shot-compare.py "$shots/orig" "$shots/orig2" "$runs/rust" \
    --coarse field-walk event-agency-4 event-agency-5 event-agency-6 || bad=1
for r in "${eye[@]}"; do
    echo "== $r screenshots, by eye: $runs/rust/$r-*.bmp against $shots/orig/eye and $shots/orig2/eye"
done
if [ -n "$hits" ]; then
    # the coverage session on every save entry, one game at a time
    for e in ${FFX_HITS_SAVES:-0 1 2 3 5}; do
        dir="$runs/session-$e"
        mkdir -p "$dir"
        sed "s/^load direct 1\$/load direct $e/" tools/replays/coverage-session.txt > "$dir/session.txt"
        FFX_HOOK_HITS="$hits/session-$e.hits" timeout 1800 "$replay" "$dir/session.txt" \
            "$win\\runs\\$id\\session-$e\\session.trace" > "$dir/session.out" 2>&1
        echo "== session $e: exit $?, $(cat "$hits/session-$e.hits" 2>/dev/null | wc -l) hooks ran"
        grep '^hook log:' "$dir/session.out" | head -6
    done
    python3 -I tools/hook-hits.py merge "$hits"/*.hits
    python3 -I tools/hook-hits.py report --only "$hits"/*.hits \
        --out "${FFX_HITS_REPORT:-$HOME/ffx-work/agents/coverage/hook-hits-report.txt}" | head -24
fi
exit $bad
