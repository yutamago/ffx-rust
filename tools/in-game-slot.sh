# Sourced by scripts whose games must run one after the other: re-runs the
# script inside one game slot (crates/ffx-harness/src/slot.rs), so all its
# games use that slot (instance, port, home). Waits for a free slot.
if [[ "${FFX_GAME_SLOT_HELD:-}" != 1 ]]; then
    _repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    cargo build -q --release --manifest-path "$_repo/Cargo.toml" -p ffx-harness --bin game-slot
    exec "$_repo/target/release/game-slot" run -- "$0" "$@"
fi
