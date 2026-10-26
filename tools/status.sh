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
#   --wait  run in the foreground (still behind the lock)
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

case "${1:-}" in
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
