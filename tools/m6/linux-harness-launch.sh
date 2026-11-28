#!/usr/bin/env bash
# FFX_HARNESS_LAUNCHER (docs/host.md, "The wgpu back end"): starts the Linux ffx-rust headless (no window) under
# the harness, with run-game.sh's options. Environment: WGPU_EXE (ffx-rust),
# WGPU_LOG (its log); FFX_HOST_GFX / FFX_HOST_GFX_TRACE / FFX_HOST_GFX_SHOTS
# pass through. The headless platform's window is 1920x1080, so the sandbox's
# resolution is set to that (the game's targets then match its swap chain).
set -euo pipefail
home="" port="" fast="" seed=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --instance|--mod|--hook-disable) shift ;;
        --harness) port="$2"; shift ;;
        --home) home="$2"; shift ;;
        --fast) fast=1 ;;
        --seed) seed="$2"; shift ;;
        *) ;;
    esac
    shift
done
[[ -n "$home" ]] && find "$home" -name GameSetting.ini -exec sed -i 's/^Resolution=.*/Resolution=1920*1080/' {} +
export FFX_HARNESS="$port" FFX_HARNESS_HOME="$home"
[[ -n "$fast" ]] && export FFX_HARNESS_FAST=1
[[ -n "$seed" ]] && export FFX_HARNESS_SEED="$seed"
export FFX_GAME_DIR="${FFX_GAME_DIR:-$HOME/steam-games/FFX-X2-HD}"
setsid "$WGPU_EXE" > "${WGPU_LOG:-/dev/null}" 2>&1 < /dev/null &
echo "ffx-rust (Linux, headless) started, pid $!"
# Example (a scene's back-end trace on the null device, no window anywhere):
#   FFX_HARNESS_LAUNCHER=tools/m6/linux-harness-launch.sh WGPU_EXE=target/release/ffx-rust \
#   FFX_HOST_GFX=null FFX_HOST_GFX_TRACE=~/ffx-work/m6/native/wgpu/traces/kami0100.trace \
#   target/release/examples/replay tools/m6/scenes/kami0100.txt ~/ffx-work/m6/native/wgpu/runs/kami0100/h.trace
