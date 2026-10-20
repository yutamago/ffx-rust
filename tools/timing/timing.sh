# Wall-time log for the repo's shell tools, the bash side of
# tools/timing_log.py (docs/workflow.md, "Build and test timing"):
#   . "$repo/tools/timing/timing.sh"; timing_start <tool> "$@"
# sets an EXIT trap that appends one line to ~/ffx-work/timing/<worktree>.tsv.
# A script with its own EXIT trap calls timing_finish from it instead.
# Prints nothing; FFX_TIMING=0 turns it off.
# No signal traps (they would change how the script dies): a script killed
# by a signal logs the status of its last finished command.
timing_start() {
    [ "${FFX_TIMING:-}" = 0 ] && return 0
    _timing_tool="$1"
    shift
    _timing_cmd="$_timing_tool $*"
    _timing_cmd="${_timing_cmd//$'\t'/ }"
    _timing_cmd="${_timing_cmd//$'\n'/ }"
    _timing_iso="$(date +%Y-%m-%dT%H:%M:%S%:z)"
    _timing_t0="$EPOCHREALTIME"
    _timing_load0="$(cut -d' ' -f1 /proc/loadavg 2>/dev/null || echo -)"
    _timing_parent="${FFX_TIMING_PARENT:--}"
    _timing_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
    _timing_wt="${FFX_TIMING_WORKTREE:-$(basename "$_timing_root")}"
    export FFX_TIMING_PARENT="$_timing_tool" FFX_TIMING_WORKTREE="$_timing_wt"
    trap 'timing_finish $?' EXIT
}

timing_finish() {
    local code="${1:-$?}"
    [ -n "${_timing_t0:-}" ] || return 0
    {
        local t1 wall load1 dir name
        t1="$EPOCHREALTIME"
        wall="$(awk -v a="$_timing_t0" -v b="$t1" 'BEGIN { printf "%.3f", b - a }')"
        load1="$(cut -d' ' -f1 /proc/loadavg 2>/dev/null || echo -)"
        dir="${FFX_TIMING_DIR:-$HOME/ffx-work/timing}"
        name="$_timing_wt"
        mkdir -p "$dir" &&
            printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$_timing_iso" "$wall" "$code" \
                "$_timing_tool" "$_timing_cmd" "$_timing_load0" "$load1" "$name" "$_timing_parent" \
                >> "$dir/$name.tsv"
    } 2>/dev/null
    _timing_t0=
    return 0
}
