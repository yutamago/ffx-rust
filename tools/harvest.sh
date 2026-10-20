#!/bin/bash
# Fills the harvested-state store (~/ffx-work/harvest, or $FFX_HARVEST) from
# the base snapshots with the standard drivers (docs/hooking.md,
# "Harvested call states"). Extra arguments go to every harvest run
# (e.g. --max 4).
set -euo pipefail
here=$(cd "$(dirname "$0")/.." && pwd)
: "${FFX_GAME_DIR:?set FFX_GAME_DIR}"
snaps=${FFX_SNAPSHOTS:-$HOME/ffx-work/snapshots}
cargo build --release -q --manifest-path "$here/Cargo.toml" -p ffx-oracle --example harvest
bin=$here/target/release/examples/harvest
out=${FFX_HARVEST:-$HOME/ffx-work/harvest}
mkdir -p "$out"
run() { # snapshot driver frames
    [ -f "$snaps/$1.snap" ] || { echo "no $snaps/$1.snap; skipped"; return; }
    "$bin" --snapshot "$snaps/$1.snap" --driver "$2" --frames "$3" --store "$out" "${@:4}" \
        > "$out/$1-$2.tsv"
}
for s in field battle; do
    run "$s" 0x00820ae0 60 "$@" # Sg_MainLoop: the game's frame
    run "$s" 0x0091a8c0 60 "$@" # Yn_FieldProc: the field's per-frame proc
done
run battle 0x00790c10 300 "$@" # MsBattleProcess
du -sh "$out"
