#!/bin/bash
# Coverage sweep over the developer saves (docs/harness.md, "Developer
# saves"): for each save, one game on the original code (--hook-disable
# replacements) with every function of the dump armed (coverage mode,
# COVERAGE_ALL) and the spell overlays in Unicorn (FFX_MAGIC_EMU=1, whose
# unload reports name the overlay entry points that ran), playing
# crates/ffx-harness/examples/devsave_sweep.rs: the save loaded by the
# developers' loader, the scene, the main menu, a battle of the area where
# safe. One game after the other, in one game slot.
#
# Usage: tools/devsave-sweep.sh OUT [n...]   (default: 1..110)
# OUT (a WSL path) gets per save: <n>.txt (the session's report), <n>.hits
# (the functions it reached first, `0x<addr>` lines as coverage.rs writes
# them; every launch arms all candidates again, so these are all of its
# hits) and <n>.out (the example's output). Saves already done (a report
# with an `end` line) are skipped, so a stopped sweep resumes.
# The hits file itself lives on the Windows drive (DEVSAVE_COVERAGE_DIR,
# default %TEMP%\ffx-devsave-cov), where the hook appends to it.
set -u
source "$(dirname "$0")/in-game-slot.sh"
repo="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo"
out="${1:?usage: tools/devsave-sweep.sh OUT [n...]}"; shift
mkdir -p "$out"
export FFX_GAME_DIR="${FFX_GAME_DIR:-$HOME/steam-games/FFX-X2-HD}"
if [ -z "${DEVSAVE_COVERAGE_DIR:-}" ]; then
    temp_win="$(powershell.exe -NoProfile -Command '[IO.Path]::GetTempPath()' | tr -d '\r')"
    DEVSAVE_COVERAGE_DIR="$(wslpath -u "${temp_win%\\}")/ffx-devsave-cov"
fi
lin="$DEVSAVE_COVERAGE_DIR"
mkdir -p "$lin"
[ -s "$lin/cand.txt" ] || python3 -I tools/coverage.py candidates --all "$lin/cand.txt"
export FFX_COVERAGE="$lin/cand.txt"
export FFX_HOOK_DISABLE=replacements
export FFX_MAGIC_EMU=1
cargo build -q --release -p ffx-harness --example devsave_sweep || exit 2
saves=("$@")
[ ${#saves[@]} -eq 0 ] && saves=($(seq 1 110))
# Alone (DEVSAVE_ALONE=1, the default): a machine shared with other agents'
# games runs one game at a time, so each save waits until no FFX.exe or
# ffx-rust.exe runs (tasklist.exe), whoever started it.
others() {
    tasklist.exe /NH /FO CSV 2>/dev/null | tr -d '\r' | grep -i -E '^"(FFX|ffx-rust)\.exe"' | wc -l
}
for n in "${saves[@]}"; do
    if grep -q '^end' "$out/$n.txt" 2>/dev/null; then continue; fi
    # turn-taking: other agents create a pause file (any name starting with
    # it, e.g. devsaves.pause.verify) when they want the machine's game
    # slot; the sweep waits between saves until none is left
    pause="${DEVSAVE_PAUSE_FILE:-$HOME/ffx-work/devsaves.pause}"
    if compgen -G "$pause*" > /dev/null; then
        echo "-- $n: paused ($(compgen -G "$pause*" | tr '\n' ' '))"
        while compgen -G "$pause*" > /dev/null; do sleep 10; done
    fi
    # a machine-room check (DEVSAVE_ROOM, default ~/ffx-work/pc2-room.sh,
    # PC2's: GPU, memory, CPU, exclusive turns) replaces the one-game rule
    # where it exists: start only when it exits 0
    room="${DEVSAVE_ROOM:-$HOME/ffx-work/pc2-room.sh}"
    if [ -x "$room" ] || [ -f "$room" ]; then
        waited=0
        until bash "$room" > /dev/null 2>&1; do
            [ $waited = 0 ] && echo "-- $n: waiting for room ($room)"
            waited=1
            sleep 15
        done
    elif [[ "${DEVSAVE_ALONE:-1}" == 1 ]]; then
        waited=0
        while [ "$(others)" -gt 0 ]; do
            [ $waited = 0 ] && echo "-- $n: waiting for another game to end"
            waited=1
            sleep 15
        done
    fi
    h=$(cat "$lin/cand.txt.hits" 2>/dev/null | wc -l)
    start=$(date +%s)
    timeout 1200 target/release/examples/devsave_sweep "$n" "$out/$n.txt" > "$out/$n.out" 2>&1
    status=$?
    tail -n +"$((h + 1))" "$lin/cand.txt.hits" 2>/dev/null > "$out/$n.hits"
    grep -q '^end' "$out/$n.txt" 2>/dev/null || printf 'save\t%s\nend\tno report (exit %s)\n' "$n" "$status" > "$out/$n.txt"
    echo "== $n: exit $status, $(($(date +%s) - start)) s, $(wc -l < "$out/$n.hits") hits, $(sort -u "$lin/cand.txt.hits" | wc -l) distinct so far; $(grep -E '^(end|battle)' "$out/$n.txt" | cut -f2 | tr '\n' ' ' | cut -c1-160)"
done
