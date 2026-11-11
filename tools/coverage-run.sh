#!/bin/bash
# Runs the game in coverage mode (crates/ffx-hook/src/coverage.rs) through
# the three replays and the coverage session (tools/replays/coverage-session.txt)
# on every save entry, all on the original code (--hook-disable replacements,
# so ported functions run their original machine code too), then merges the
# hits into symbols/coverage.txt (tools/coverage.py).
#
# Usage: tools/coverage-run.sh [--rust] [--no-replays] [save slots (ffx_00N), default 0 1 2 3 5]
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
#
# --rust (the first argument): the 32-bit wording of the M5 stage 1 gate
# (docs/harness.md, "The stage 1 gate in the hooked game"). Every hook on
# (the Rust replacements, not `original`), FFX_MAGIC_RUST=1 unless set, every
# function of the dump armed except the movie libraries stage 2 replaces
# (tools/coverage.py candidates --rust); the hook arms only the entries no
# hook owns, so each hit is a function that ran its original code. Replays:
# run-all.sh's main and extra lists, then the audio list again with
# FFX_AUDIO_WAV (COVERAGE_AUDIO=0 leaves that out); no coverage sessions
# unless save slots are given. symbols/coverage.txt is left alone; the hits
# are named in $COVERAGE_DIR/rust-report.txt (tools/coverage.py report).
# COVERAGE_DRY=1 prints what would run (mode, candidates, replays,
# environment) and starts no game.
set -u
repo="$(cd "$(dirname "$0")/.." && pwd)"
rust=""
if [[ "${1:-}" == --rust ]]; then rust=1; shift; fi
replays=1
if [[ "${1:-}" == --no-replays ]]; then replays=0; shift; fi
if [[ -n "$rust" ]]; then
    export FFX_MAGIC_RUST="${FFX_MAGIC_RUST:-1}"
    if [[ -z "${COVERAGE_REPLAYS:-}" ]]; then
        list="$("$repo/tools/replays/run-all.sh" --list) $("$repo/tools/replays/run-all.sh" --extra --list)"
        COVERAGE_REPLAYS="$(echo $list)"
    fi
    if [[ "${COVERAGE_AUDIO:-1}" != 0 ]]; then
        audio_replays="$(echo $("$repo/tools/replays/run-all.sh" --audio --list))"
    fi
    sessions="${*:-}"
else
    sessions="${*:-0 1 2 3 5}"
fi
replay_list="${COVERAGE_REPLAYS:-field-walk battle-attack menu-browse}"
[[ $replays == 0 ]] && replay_list="" && audio_replays=""
if [[ "${COVERAGE_DRY:-}" == 1 ]]; then
    if [[ -n "$rust" ]]; then echo "mode: rust (every hook on, FFX_MAGIC_RUST=$FFX_MAGIC_RUST)"
    else echo "mode: original (--hook-disable replacements)"; fi
    if [[ -n "${COVERAGE_CANDIDATES:-}" ]]; then echo "candidates: $COVERAGE_CANDIDATES"
    elif [[ -n "$rust" ]]; then echo "candidates: tools/coverage.py candidates --rust"
    else echo "candidates: tools/coverage.py candidates${COVERAGE_ALL:+ --all}"; fi
    echo "replays: $replay_list"
    echo "audio replays (FFX_AUDIO_WAV): ${audio_replays:-}"
    echo "sessions (save entries): $sessions"
    echo "merge into symbols/coverage.txt: $([[ -z "${COVERAGE_ALL:-}${COVERAGE_CANDIDATES:-}$rust" ]] && echo yes || echo no)"
    exit 0
fi
source "$(dirname "$0")/in-game-slot.sh"
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
elif [ -n "$rust" ]; then
    python3 -I tools/coverage.py candidates --rust "$lin/cand.txt"
else
    python3 -I tools/coverage.py candidates ${COVERAGE_ALL:+--all} "$lin/cand.txt"
fi
rm -f "$lin/cand.txt.hits"
export FFX_COVERAGE="$lin/cand.txt"
# every launch builds the hook (tools/run-game.sh): once is enough here
if [[ "${FFX_NO_BUILD:-}" != 1 ]]; then
    tools/build-hook.sh >/dev/null || exit 1
    export FFX_NO_BUILD=1
fi
log="$FFX_GAME_DIR/ffx-hook.log"
# the replay player's third argument: `original` turns the replacements off
mode_arg=(original)
[[ -n "$rust" ]] && mode_arg=()
tags=()
run() { # script, tag
    local n h
    n=$(wc -l < "$log")
    # every launch arms all candidates again and appends its own first hits:
    # the lines this run adds are its hits ($lin/<tag>.hits)
    h=$(cat "$lin/cand.txt.hits" 2>/dev/null | wc -l)
    timeout 1800 cargo run -q --release -p ffx-harness --example replay -- \
        "$1" "$win\\$2.trace" ${mode_arg[@]+"${mode_arg[@]}"} > "$lin/$2.out" 2>&1
    local status=$?
    tail -n +"$((h + 1))" "$lin/cand.txt.hits" 2>/dev/null > "$lin/$2.hits"
    tags+=("$2")
    echo "== $2: exit $status, $(wc -l < "$lin/$2.hits") hits, $(sort -u "$lin/cand.txt.hits" 2>/dev/null | wc -l) distinct hits so far"
    tail -n +"$n" "$log" | grep -E 'coverage:|FAULT|PANIC' | head -4
}
for r in $replay_list; do run "tools/replays/$r.txt" "$r"; done
for r in ${audio_replays:-}; do
    FFX_AUDIO_WAV="$lin/$r-audio.wav" run "tools/replays/$r.txt" "$r-audio"
done
for s in $sessions; do
    for e in $s; do
        sed "s/^load direct 1\$/load direct $e/" tools/replays/coverage-session.txt > "$lin/session-$e.txt"
        run "$lin/session-$e.txt" "session-$e"
    done
done
if [[ -n "$rust" ]]; then
    files=()
    for t in "${tags[@]}"; do files+=("$lin/$t.hits"); done
    python3 -I tools/coverage.py report "${files[@]}" --out "$lin/rust-report.txt" | head -20
    echo "stage 1 gate (hooked game): $(sort -u "$lin/cand.txt.hits" 2>/dev/null | wc -l) FFX.exe functions ran their original code; $lin/rust-report.txt"
elif [[ -z "${COVERAGE_ALL:-}${COVERAGE_CANDIDATES:-}" ]]; then
    python3 -I tools/coverage.py merge "$lin/cand.txt.hits"
else
    echo "hits: $lin/cand.txt.hits ($(sort -u "$lin/cand.txt.hits" | wc -l) functions)"
fi
