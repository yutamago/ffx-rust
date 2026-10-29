#!/usr/bin/env bash
# Regenerates the status page ($FFX_WORK/status/index.html, tools/status.py)
# and the hook map (hook-map.html, tools/hook-map.py)
# in the background and returns at once. Run it after merging into main.
#
# Requests while a run is going are folded into one more run after it, so
# any number of agents can call it at any time. The log is status.log next
# to the page.
#
# Usage: tools/status.sh [--wait] [status.py args]
#        tools/status.sh --watch [SECONDS]
#   --wait   run in the foreground (still behind the lock)
#   --watch  keep the page live: a detached loop that writes live.js (agents,
#            queue, machine, timing; status.py --live) every SECONDS (60) and
#            the whole page when main moves. One loop at a time. It runs as
#            the systemd user unit ffx-status-watch where there is one (a
#            process started from a session's shell dies with the session's
#            scope when the session restarts); stop it with
#            systemctl --user stop ffx-status-watch (or pkill -f 'status.sh --watch-loop').
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
out="${FFX_WORK:-$HOME/ffx-work}/status"
mkdir -p "$out"
pending="$out/.pending" lock="$out/.lock" log="$out/status.log"

run() {
    # the loop around the lock catches a request made just before release
    while [ -e "$pending" ]; do
        exec 9>"$lock"
        flock -n 9 || return 0
        while [ -e "$pending" ]; do
            rm -f "$pending"
            python3 "$here/status.py" "$@" >>"$log" 2>&1 || echo "$(date +%T) status.py failed" >>"$log"
            # the hook map next to it (tools/hook-map.py)
            python3 -I "$here/hook-map.py" --html "$out/hook-map.html" >>"$log" 2>&1 || echo "$(date +%T) hook-map.py failed" >>"$log"
        done
        exec 9>&-
    done
}

watch() {
    exec 8>"$out/.watch.lock"
    flock -n 8 || { echo "status watch: already running" >>"$log"; return 0; }
    echo "$(date +%T) status watch: every ${1}s" >>"$log"
    local head last=""
    while true; do
        head=$(git -C "$here/.." rev-parse main 2>/dev/null || true)
        # main moved, or an older status.py (a worktree branched before this
        # page) wrote data.js without the phases: write it again
        if [ "$head" != "$last" ] || ! grep -q '"milestones"' "$out/data.js" 2>/dev/null; then
            touch "$pending"
            run
            # still pending: another run holds the lock; try again next round
            [ -e "$pending" ] || last=$head
        fi
        python3 "$here/status.py" --live >>"$log" 2>&1 || echo "$(date +%T) status.py --live failed" >>"$log"
        sleep "$1"
    done
}

case "${1:-}" in
    --watch-loop)  # the detached copy below
        watch "${2:-60}"
        ;;
    --watch)
        if systemctl --user is-active -q ffx-status-watch 2>/dev/null; then
            echo "status page: ffx-status-watch already runs -> $out/index.html"
        elif command -v systemd-run >/dev/null && systemctl --user show-environment >/dev/null 2>&1; then
            systemctl --user reset-failed ffx-status-watch 2>/dev/null || true
            systemd-run --user -q --collect --unit=ffx-status-watch --setenv=PATH="$PATH" \
                ${FFX_WORK:+--setenv=FFX_WORK="$FFX_WORK"} "$here/status.sh" --watch-loop "${2:-60}"
            echo "status page: live every ${2:-60} s -> $out/index.html (stop: systemctl --user stop ffx-status-watch)"
        else
            setsid -f "$here/status.sh" --watch-loop "${2:-60}" </dev/null >/dev/null 2>&1
            echo "status page: live every ${2:-60} s -> $out/index.html (stop: pkill -f 'status.sh --watch-loop')"
        fi
        ;;
    --run)  # the detached copy below
        shift
        run "$@"
        ;;
    --wait)
        shift
        touch "$pending"
        run "$@"
        tail -n 1 "$log"
        ;;
    *)
        touch "$pending"
        # its own session: the run outlives the agent's shell
        setsid -f "$here/status.sh" --run "$@" </dev/null >/dev/null 2>&1
        echo "status page: updating in the background -> $out/index.html"
        ;;
esac
