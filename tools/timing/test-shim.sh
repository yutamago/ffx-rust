#!/bin/bash
# Pass-through tests for the cargo timing shim (tools/timing/cargo-shim.c),
# against a fake cargo, so no real build runs:
#   tools/timing/test-shim.sh <dir to run in> [shim binary]
# Run it once in a directory outside ffx-decomp (nothing may be logged) and
# once inside a checkout (one line per run). Builds the shim into a temporary
# directory unless a binary is given.
set -u -m   # -m: background jobs keep SIGINT, as in an interactive shell
here="$(cd "$(dirname "$0")" && pwd)"
run_dir="$(cd "$1" && pwd)"
tmp="$(mktemp -d "${TMPDIR:-/tmp}/ffx-timing-test.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT
shim="${2:-$tmp/cargo}"
[ -n "${2:-}" ] || cc -O2 -o "$shim" "$here/cargo-shim.c" || exit 1

fake="$tmp/fake-cargo"
cat > "$fake" <<'EOF'
#!/bin/bash
[ "$1" = test ] && shift
case "$1" in
exit) exit "$2" ;;
order) for i in 1 2 3 4 5; do echo "out $i"; echo "err $i" >&2; done ;;
stdin) cat ;;
slow) echo first; sleep 1; echo second ;;
sleep) echo $$ > "$2"; exec sleep 30 ;;
selfkill) kill -"$2" $$ ;;
parent) echo "${FFX_TIMING_PARENT:-unset}" ;;
argv0) echo "$0" ;;
*) echo "args: $*" ;;
esac
EOF
chmod +x "$fake"
export FFX_TIMING_REAL_CARGO="$fake" FFX_TIMING_DIR="$tmp/log"
unset FFX_TIMING FFX_TIMING_PARENT
cd "$run_dir"

pass=0 fail=0
check() { # name, expected, actual
    if [ "$2" == "$3" ]; then pass=$((pass + 1)); else fail=$((fail + 1)); echo "FAIL $1: expected [$2] got [$3]"; fi
}
lines() { cat "$tmp"/log/*.tsv 2>/dev/null | wc -l; }

for c in 0 1 3 101 255; do
    "$shim" test exit $c; check "exit $c" $c $?
done
check "stdout+stderr order" "$("$fake" test order 2>&1)" "$("$shim" test order 2>&1)"
check "stdout only" "$("$fake" test order 2>/dev/null)" "$("$shim" test order 2>/dev/null)"
check "stderr only" "$("$fake" test order 2>&1 >/dev/null)" "$("$shim" test order 2>&1 >/dev/null)"
check "stdin" "hello" "$(echo hello | "$shim" test stdin)"
check "args with spaces" "args: -p a b" "$("$shim" test -p "a b")"

# no buffering: the first line arrives before the child's sleep ends
start=$EPOCHREALTIME
first=$("$shim" test slow | { read -r l; echo "$EPOCHREALTIME"; cat >/dev/null; })
check "unbuffered" 1 "$(echo "$first - $start < 0.8" | bc)"

# signals sent to the shim reach the child; the shim dies the same way
for sig in INT TERM HUP; do
    "$shim" test sleep "$tmp/pid" & spid=$!
    for _ in $(seq 50); do [ -s "$tmp/pid" ] && break; sleep 0.1; done
    kill -$sig $spid; wait $spid; code=$?
    cpid=$(cat "$tmp/pid"); rm -f "$tmp/pid"
    check "kill -$sig shim: status" $((128 + $(kill -l $sig))) $code
    check "kill -$sig shim: child gone" 1 "$(kill -0 "$cpid" 2>/dev/null; echo $?)"
done
# Ctrl-C style: SIGINT to the whole process group
"$shim" test sleep "$tmp/pid" & spid=$!
for _ in $(seq 50); do [ -s "$tmp/pid" ] && break; sleep 0.1; done
kill -INT -- -$spid; wait $spid; code=$?
check "SIGINT to the group: status" 130 $code
check "SIGINT to the group: child gone" 1 "$(kill -0 "$(cat "$tmp/pid")" 2>/dev/null; echo $?)"
rm -f "$tmp/pid"
# a child killed by a signal from elsewhere
"$shim" test selfkill TERM; check "child killed by TERM" 143 $?
"$shim" test selfkill KILL; check "child killed by KILL" 137 $?
n=$(lines); "$shim" metadata --format-version 1 >/dev/null; "$shim" --version >/dev/null
check "metadata/--version not logged" "$n" "$(lines)"
check "argv[0]" "$fake" "$("$shim" test argv0)"

n=$(lines)
check "FFX_TIMING_PARENT for the child" "$( [ "$n" -gt 0 ] && echo cargo || echo unset)" "$("$shim" test parent)"
n=$(lines)
FFX_TIMING=0 "$shim" test exit 0; check "FFX_TIMING=0 not logged" "$n" "$(lines)"
FFX_TIMING_PARENT=cargo "$shim" test exit 0; check "nested cargo not logged" "$n" "$(lines)"
FFX_TIMING_PARENT=mutants.py "$shim" test exit 0

# overhead per call, against the fake directly
t0=$EPOCHREALTIME; for _ in $(seq 100); do "$fake" exit 0; done; t1=$EPOCHREALTIME
for _ in $(seq 100); do "$shim" test exit 0; done; t2=$EPOCHREALTIME
echo "overhead per call: $(echo "($t2 - $t1 - ($t1 - $t0)) * 10" | bc -l | cut -c1-6) ms (fake alone $(echo "($t1 - $t0) * 10" | bc -l | cut -c1-6) ms)"

echo "logged lines: $(lines)"
if [ "$(lines)" -gt 0 ]; then
    echo "sample:"; grep -m1 'cargo test -p a b' "$tmp"/log/*.tsv
    grep -m1 'mutants.py$' "$tmp"/log/*.tsv
    bad=$(awk -F'\t' 'NF != 9' "$tmp"/log/*.tsv | wc -l)
    check "9 columns per line" 0 "$bad"
fi
echo "$pass passed, $fail failed"
[ $fail -eq 0 ]
