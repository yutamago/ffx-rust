#!/bin/bash
# Harvests real calls of the ported render chain (crates/ffx-prender) into
# the harvested-state store: graphicRender (0x00642bb0, PhyreScene_render
# for the frame: the map, character and other worlds, culling jobs run by
# the waiting thread) on field.snap and battle.snap. tools/harvest.sh's
# drivers (the game's update) never render, so these targets need this one.
# Extra arguments go to every run.
set -euo pipefail
here=$(cd "$(dirname "$0")/.." && pwd)
: "${FFX_GAME_DIR:?set FFX_GAME_DIR}"
snaps=${FFX_SNAPSHOTS:-$HOME/ffx-work/snapshots}
cargo build --release -q --manifest-path "$here/Cargo.toml" -p ffx-oracle --example harvest
bin=$here/target/release/examples/harvest
out=${FFX_HARVEST:-$HOME/ffx-work/harvest}
targets=$(mktemp)
cat > "$targets" <<'EOF'
0x0062be70
0x009f3c30
0x009f2580
0x009f3160
0x009f3660
0x009f2d70
0x009f17c0
0x009f0d20
0x009f10e0
0x009f1230
0x009f1c60
0x004e71d0
0x005ba690
0x005b5a50
EOF
for s in field battle; do
    [ -f "$snaps/$s.snap" ] || { echo "no $snaps/$s.snap; skipped"; continue; }
    "$bin" --snapshot "$snaps/$s.snap" --driver 0x00642bb0 --frames 2 --max 64 \
        --targets "$targets" --store "$out" "$@" | tail -16
done
rm -f "$targets"
