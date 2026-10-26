#!/bin/bash
# scene-gate-capture.sh <replay> <run> [original|only-m4|all]: one run of a
# draw-gate replay (tools/replays/draw-gate-<replay>.txt, or draw-field)
# with the graphics layer, its captures copied to
# ${FFX_CAPTURES:-~/ffx-work/m4/captures}/<run>/<capture>.
#   original  no replacements (`replay ... original`)
#   only-m4   FFX_HOOK_ONLY: only the M4 port hooks (the ported! names of
#             ported/{h,p}<n>_*.rs, c2_new.rs, m4_rest.rs, phyre_render.rs),
#             the rest original
#   all       every hook (the default)
# FFX_HOOK_ONLY or FFX_HOOK_DISABLE set by the caller pass on (bisecting;
# with `all` only). docs/drawdiff.md, "Scene gates".
set -u
repo="$(cd "$(dirname "$0")/.." && pwd)"
replay_name="$1"; run="$2"; mode="${3:-all}"
script="$repo/tools/replays/draw-gate-$replay_name.txt"
[ "$replay_name" = draw-field ] && script="$repo/tools/replays/draw-field.txt"
[ -f "$script" ] || { echo "no replay $script" >&2; exit 2; }
case "$replay_name" in
    besaid) caps=(bsil0000 bsil0300 menu) ;;
    kami) caps=(kami0100 battle) ;;
    draw-field) caps=(fw-101 fw-459 besaid-101) ;;
    *) caps=() ;;
esac
dst="${FFX_CAPTURES:-$HOME/ffx-work/m4/captures}/$run"
cd "$repo" || exit 1
arg=""
case "$mode" in
    original) arg=original ;;
    only-m4)
        list="$dst.hooks.txt"
        mkdir -p "$(dirname "$list")"
        cat crates/ffx-hook/src/ported/{h,p}[0-9]*_*.rs crates/ffx-hook/src/ported/{phyre_render,c2_new,m4_rest}.rs 2>/dev/null \
            | grep -oE '^\s*[A-Za-z_][A-Za-z_0-9]* @ 0x' | sed -E 's/^\s*//; s/ @ 0x//' | sort -u > "$list"
        export FFX_HOOK_ONLY="@$(wslpath -w "$list")"
        echo "hooks: $(wc -l < "$list")"
        ;;
    all) ;;
    *) echo "mode: original, only-m4 or all" >&2; exit 2 ;;
esac
temp_win="$(powershell.exe -NoProfile -Command '[IO.Path]::GetTempPath()' | tr -d '\r')"
lin="$(wslpath "${temp_win%\\}")/ffx-draw-gates/$run"
rm -rf "$lin"; mkdir -p "$lin"
win="$(wslpath -w "$lin")"
tools/build-hook.sh > /dev/null || exit 1
cargo build -q --release -p ffx-harness --example replay || exit 1
FFX_GFX=1 FFX_TEST_VOLUME=0 FFX_GAME_DIR="${FFX_GAME_DIR:-$HOME/steam-games/FFX-X2-HD}" timeout 1800 \
    target/release/examples/replay "$script" "$win\\draw.trace" $arg > "$lin/replay.out" 2>&1
status=$?
rm -rf "$dst"; mkdir -p "$dst"
cp "$lin/replay.out" "$dst/"
for c in "${caps[@]}"; do
    [ -d "$lin/$c" ] && cp -r "$lin/$c" "$dst/"
done
rm -rf "$lin"
echo "replay exit $status; $dst: $(ls "$dst" | tr '\n' ' ')"
exit $status
