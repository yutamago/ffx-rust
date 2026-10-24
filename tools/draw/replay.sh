#!/usr/bin/env bash
# Runs ffx-draw's replay mode in the game (docs/draw.md): a capture, then
# the captured scene drawn by ffx-draw in place of the game's, compared.
# Usage: tools/draw/replay.sh <run name> [script]
# Output: ~/ffx-work/m4/captures/<run name>/ (replay*/replay.txt).
set -euo pipefail
repo="$(cd "$(dirname "$0")/../.." && pwd)"
name="${1:?run name}"
script="${2:-$repo/tools/draw/replay-field.txt}"
out="$HOME/ffx-work/m4/captures/$name"
mkdir -p "$out"
win="$(wslpath -w "$out")"
cd "$repo"
FFX_GFX=1 FFX_GAME_DIR="${FFX_GAME_DIR:-$HOME/steam-games/FFX-X2-HD}" \
    cargo run --release -p ffx-harness --example replay -- "$script" "$win\\draw.trace"
