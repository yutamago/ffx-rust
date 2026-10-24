#!/usr/bin/env bash
# Runs ffx-draw's replay mode in the game (docs/draw.md): a capture, then
# the captured scene drawn by ffx-draw in place of the game's, compared.
# Usage: tools/draw/replay.sh <run name> [capture dir...]
# Each extra capture dir (e.g. ~/ffx-work/m4/captures/fw1-a) is replayed
# too, at the end, into <run>/replay-<its name>.
# Output: ~/ffx-work/m4/captures/<run name>/ (replay*/replay.txt,
# replay*/replay.ffxgfx for drawdiff).
set -euo pipefail
repo="$(cd "$(dirname "$0")/../.." && pwd)"
name="${1:?run name}"
shift
out="$HOME/ffx-work/m4/captures/$name"
mkdir -p "$out"
win="$(wslpath -w "$out")"
script="$out/script.txt"
cp "$repo/tools/draw/replay-field.txt" "$script"
for dir in "$@"; do
    echo "gfxreplay $(wslpath -w "$dir")\\capture.ffxgfx \$OUT\\replay-$(basename "$dir")" >>"$script"
done
cd "$repo"
FFX_GFX=1 FFX_GAME_DIR="${FFX_GAME_DIR:-$HOME/steam-games/FFX-X2-HD}" \
    cargo run --release -p ffx-harness --example replay -- "$script" "$win\\draw.trace"
