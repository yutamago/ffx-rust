# Sourced by scripts that run the game several times: re-runs the script
# inside the game slot (crates/ffx-harness/src/slot.rs), so its games run
# back to back without another agent's in between. Waits for its turn.
if [[ "${FFX_GAME_SLOT_HELD:-}" != 1 ]]; then
    _repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    cargo build -q --release --manifest-path "$_repo/Cargo.toml" -p ffx-harness --bin game-slot
    exec "$_repo/target/release/game-slot" run -- "$0" "$@"
fi
