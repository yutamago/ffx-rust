#!/bin/bash
# scene-gate-capture.sh <replay> <run> [original|only-m4|all|rust]: one run of a
# draw-gate replay (tools/replays/draw-gate-<replay>.txt, or draw-field)
# with the graphics layer, its captures copied to
# ${FFX_CAPTURES:-~/ffx-work/m4/captures}/<run>/<capture>.
#   original  no replacements (`replay ... original`)
#   only-m4   FFX_HOOK_ONLY: only the M4 port hooks (the ported! names of
#             ported/{h,p}<n>_*.rs, c2_new.rs, m4_rest.rs, phyre_render.rs),
#             the rest original
#   all       every hook (the default)
#   rust      ffx-rust.exe (FFX_HARNESS_EXE=rust, crates/ffx-host; the D3D11
#             layer's gate capture, crates/ffx-host/src/gfx/gate.rs, with
#             FFX_HOST_GFX_CAPTURE=1): capture.ffxgfx and frames.txt, no
#             hashes.txt (scene-gate.sh runs drawdiff alone then); its
#             ffx-rust.log is kept as <run>/ffx-rust.log
# FFX_HOOK_ONLY or FFX_HOOK_DISABLE set by the caller pass on (bisecting;
# with `all` only). docs/drawdiff.md, "Scene gates".
set -u
repo="$(cd "$(dirname "$0")/.." && pwd)"
replay_name="$1"; run="$2"; mode="${3:-all}"
script="$repo/tools/replays/draw-gate-$replay_name.txt"
[ "$replay_name" = draw-field ] && script="$repo/tools/replays/draw-field.txt"
[ -f "$script" ] || { echo "no replay $script" >&2; exit 2; }
case "$replay_name" in
    besaid) caps=(bsil0000 bsil0300) ;;
    kami) caps=(kami0100) ;;
    deck) caps=(menu) ;;
    battle) caps=(battle) ;;
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
        # plus renderWorldCulledSorted's port, which is on/off by name
        # (not a ported! entry: its address holds the capture wrapper)
        mkdir -p "$(dirname "$list")"
        cat crates/ffx-hook/src/ported/{h,p}[0-9]*_*.rs crates/ffx-hook/src/ported/{phyre_render,c2_new,m4_rest}.rs 2>/dev/null \
            | grep -oE '^\s*[A-Za-z_][A-Za-z_0-9]* @ 0x' | sed -E 's/^\s*//; s/ @ 0x//' \
            | { cat; echo Phyre_PRendering_PRenderer_renderWorldCulledSorted_port; } | sort -u > "$list"
        export FFX_HOOK_ONLY="@$(wslpath -w "$list")"
        echo "hooks: $(wc -l < "$list")"
        ;;
    all) ;;
    rust)
        export FFX_HARNESS_EXE=rust FFX_HOST_GFX_CAPTURE=1
        # emulated updates: the replay's and each reply's time
        export FFX_REPLAY_TIMEOUT="${FFX_REPLAY_TIMEOUT:-3600}"
        export FFX_HARNESS_REPLY_TIMEOUT="${FFX_HARNESS_REPLY_TIMEOUT:-3600}"
        ;;
    *) echo "mode: original, only-m4, all or rust" >&2; exit 2 ;;
esac
temp_win="$(powershell.exe -NoProfile -Command '[IO.Path]::GetTempPath()' | tr -d '\r')"
lin="$(wslpath "${temp_win%\\}")/ffx-draw-gates/$run"
rm -rf "$lin"; mkdir -p "$lin"
win="$(wslpath -w "$lin")"
if [ "$mode" = rust ]; then
    # run-game.sh installs this build (FFX_NO_BUILD: no second build there)
    [ "${FFX_NO_BUILD:-}" = 1 ] || tools/build-host.sh > /dev/null || exit 1
    export FFX_NO_BUILD=1
    limit=14400
else
    tools/build-hook.sh > /dev/null || exit 1
    limit=1800
fi
cargo build -q --release -p ffx-harness --example replay || exit 1
FFX_GFX=1 FFX_TEST_VOLUME=0 FFX_GAME_DIR="${FFX_GAME_DIR:-$HOME/steam-games/FFX-X2-HD}" timeout "$limit" \
    target/release/examples/replay "$script" "$win\\draw.trace" $arg > "$lin/replay.out" 2>&1
status=$?
rm -rf "$dst"; mkdir -p "$dst"
cp "$lin/replay.out" "$dst/"
if [ "$mode" = rust ]; then
    # the slot's ffx-rust.log, before the slot's next game replaces it
    slot="$(sed -n 's/^replay: slot \([0-9]*\),.*/\1/p' "$lin/replay.out" | head -1)"
    [ -n "$slot" ] && cp "${FFX_INSTANCES_DIR:-$HOME/ffx-work/instances}/$slot/ffx-rust.log" "$dst/" 2>/dev/null
fi
for c in "${caps[@]}"; do
    [ -d "$lin/$c" ] && cp -r "$lin/$c" "$dst/"
done
rm -rf "$lin"
echo "replay exit $status; $dst: $(ls "$dst" | tr '\n' ' ')"
exit $status
