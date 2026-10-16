//! The main loop, fades, scene changes and menu requests (the PS2 `Sg` module, `FFXApplication`).
//!
//! The `main` module of `docs/rewrite-plan.md`: each original function
//! becomes a core (portable, on its own types) and a [`bridge`] function
//! over game memory (`docs/porting.md`).

/// The originals, where the game keeps their state (`ffx_bridge::Mem`).
pub mod bridge {}
