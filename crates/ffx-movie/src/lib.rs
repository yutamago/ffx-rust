//! Movies (`movie`, `PhyFMV`).
//!
//! The `movie` module of `docs/rewrite-plan.md`: each original function
//! becomes a core (portable, on its own types) and a [`bridge`] function
//! over game memory (`docs/porting.md`).

/// The originals, where the game keeps their state (`ffx_bridge::Mem`).
pub mod bridge {}
