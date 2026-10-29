//! The main loop, fades, scene changes and menu requests (the PS2 `Sg` module, `FFXApplication`).
//!
//! The `main` module of `docs/rewrite-plan.md`: each original function
//! becomes a core (portable, on its own types) and a `bridge` function
//! over game memory (`docs/porting.md`), one module per area.

pub mod abmap;
pub mod app;
pub mod assert;
pub mod cache;
pub mod camera;
pub mod cdread;
pub mod chpacket;
pub mod cloth;
pub mod control;
pub mod decode;
pub mod envelope;
pub mod gs;
pub mod help;
pub mod light;
pub mod m5main3_cam;
pub mod m5main3_cd;
pub mod m5main3_gu;
pub mod m5main3_menu;
pub mod m5main3_misc;
pub mod m5main3_small;
pub mod m5main_app;
pub mod m5main_dbg;
pub mod m5main_dbg2;
pub mod m5main_gui;
pub mod m5main_init;
pub mod m5main_loop;
pub mod m5main_mem;
pub mod m5main_mem2;
pub mod m5main_task;
pub mod player;
pub mod quat;
pub mod savecmd;
pub mod scratch;
pub mod seq;
pub mod spline;
pub mod state;
pub mod triangle;
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
