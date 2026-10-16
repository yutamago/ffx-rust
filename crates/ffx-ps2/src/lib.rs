//! The PS2 emulation the PC port carries: the R5900 and VU0 registers in globals (`AsmEmu`, `FFXVu`).
//!
//! The `ps2-emu` module of `docs/rewrite-plan.md`: each original function
//! becomes a core (portable, on its own types) and a [`bridge`] function
//! over game memory (`docs/porting.md`).

/// The originals, where the game keeps their state (`ffx_bridge::Mem`).
pub mod bridge {}
