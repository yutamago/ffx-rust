#!/bin/bash
# Runs the game in coverage mode (crates/ffx-hook/src/coverage.rs) through
# the three replays and the coverage session (tools/replays/coverage-session.txt)
# on every save entry, all on the original code (--hook-disable replacements,
# so ported functions run their original machine code too), then merges the
# hits into symbols/coverage.txt (tools/coverage.py).
#
# Usage: tools/coverage-run.sh [--no-replays] [save slots (ffx_00N), default 0 1 2 3 5]
# Hits add up: symbols/coverage.txt keeps the union of every run.
#
# COVERAGE_ALL=1 arms every function of the dump (PhyreEngine and the CRT
# included, tools/coverage.py candidates --all) and does not merge into
# symbols/coverage.txt: the hits stay in $COVERAGE_DIR/cand.txt.hits.
# Each run's own hits land in $COVERAGE_DIR/<tag>.hits as well.
# COVERAGE_CANDIDATES=<file> arms the functions of that file instead
# (`<address> <first byte>` lines, e.g. tools/phyre-classes.py
# --script-traps) and, like COVERAGE_ALL, leaves symbols/coverage.txt alone.
# COVERAGE_REPLAYS="a b ..." picks the replays (default field-walk
# battle-attack menu-browse); COVERAGE_DIR the Windows-drive folder.
set -u
source "$(dirname "$0")/in-game-slot.sh"
repo="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo"
export FFX_GAME_DIR="${FFX_GAME_DIR:-$HOME/steam-games/FFX-X2-HD}"
if [ -z "${COVERAGE_DIR:-}" ]; then
    temp_win="$(powershell.exe -NoProfile -Command '[IO.Path]::GetTempPath()' | tr -d '\r')"
    COVERAGE_DIR="$(wslpath -u "${temp_win%\\}")/ffx-coverage"
fi
lin="$COVERAGE_DIR"
mkdir -p "$lin"
win="$(wslpath -w "$lin")"
if [ -n "${COVERAGE_CANDIDATES:-}" ]; then
    cp "$COVERAGE_CANDIDATES" "$lin/cand.txt"
    echo "$(wc -l < "$lin/cand.txt") candidates from $COVERAGE_CANDIDATES"
else
    python3 -I tools/coverage.py candidates ${COVERAGE_ALL:+--all} "$lin/cand.txt"
fi
rm -f "$lin/cand.txt.hits"
export FFX_COVERAGE="$lin/cand.txt"
log="$FFX_GAME_DIR/ffx-hook.log"
run() { # script, tag
    local n h
    n=$(wc -l < "$log")
    # every launch arms all candidates again and appends its own first hits:
    # the lines this run adds are its hits ($lin/<tag>.hits)
    h=$(cat "$lin/cand.txt.hits" 2>/dev/null | wc -l)
    timeout 1800 cargo run -q --release -p ffx-harness --example replay -- \
        "$1" "$win\\$2.trace" original > "$lin/$2.out" 2>&1
    local status=$?
    tail -n +"$((h + 1))" "$lin/cand.txt.hits" 2>/dev/null > "$lin/$2.hits"
    echo "== $2: exit $status, $(wc -l < "$lin/$2.hits") hits, $(sort -u "$lin/cand.txt.hits" 2>/dev/null | wc -l) distinct hits so far"
    tail -n +"$n" "$log" | grep -E 'coverage:|FAULT|PANIC' | head -4
}
replays=1
if [[ "${1:-}" == --no-replays ]]; then replays=0; shift; fi
if [[ $replays == 1 ]]; then
    for r in ${COVERAGE_REPLAYS:-field-walk battle-attack menu-browse}; do run "tools/replays/$r.txt" "$r"; done
fi
for s in "${@:-0 1 2 3 5}"; do
    for e in $s; do
        sed "s/^load direct 1\$/load direct $e/" tools/replays/coverage-session.txt > "$lin/session-$e.txt"
        run "$lin/session-$e.txt" "session-$e"
    done
done
if [[ -z "${COVERAGE_ALL:-}${COVERAGE_CANDIDATES:-}" ]]; then
    python3 -I tools/coverage.py merge "$lin/cand.txt.hits"
else
    echo "hits: $lin/cand.txt.hits ($(sort -u "$lin/cand.txt.hits" | wc -l) functions)"
fi
