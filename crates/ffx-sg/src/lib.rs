//! The main loop, fades, scene changes and menu requests (the PS2 `Sg` module, `FFXApplication`).
//!
//! The `main` module of `docs/rewrite-plan.md`: each original function
//! becomes a core (portable, on its own types) and a `bridge` function
//! over game memory (`docs/porting.md`), one module per area.

pub mod assert;
pub mod camera;
pub mod chpacket;
pub mod gs;
pub mod light;
pub mod quat;
pub mod scratch;
pub mod seq;
pub mod state;
pub mod vec;

/// The low byte of an argument the original reads as a byte (`mov al,
/// [ebp+8]`).
#[must_use]
pub const fn lo8(v: u32) -> u8 {
    v.to_le_bytes()[0]
}

/// The low half of an argument the original reads as a word.
#[must_use]
pub const fn lo16(v: u32) -> u16 {
    let [a, b, _, _] = v.to_le_bytes();
    u16::from_le_bytes([a, b])
}
