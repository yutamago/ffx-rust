//! Helpers shared by the `harvested_<module>.rs` tests (docs/hooking.md
//! "Harvested call states"). Each of those files includes this module with
//! `mod common;`. It is not a test target: Cargo builds only `tests/*.rs` and
//! `tests/*/main.rs`. The layout check is in `harvested.rs`, so it runs once.

#![allow(dead_code)] // each test file uses a subset of these

use ffx_oracle::{CallState, Oracle};

pub fn int(s: &CallState, i: u32) -> i32 {
    s.arg(i).cast_signed()
}

/// `_CIatan` for the port's side: the host's `atan` stored as `f32` (the
/// original runs the CRT on the oracle's emulated x87; the `f32`s agree on
/// the harvested states).
pub fn crt_atan(_: &mut Oracle, x: f64) -> u32 {
    ffx_bridge::x87::store(x.atan())
}

/// `_CIsin` and `_CIcos` for the port's side: the host's, stored as `f32`
/// (`f32` bits in and out, as the battle vector code calls them).
pub fn crt_vector_trig(t: ffx_battle::pos::bridge::Trig, x: u32) -> u32 {
    use ffx_battle::pos::bridge::Trig;
    let x = f64::from(f32::from_bits(x));
    ffx_bridge::x87::store(match t {
        Trig::Sin => x.sin(),
        Trig::Cos => x.cos(),
        Trig::Atan => x.atan(),
    })
}

// effects (round 33): the thiscall callees run as the originals
pub fn this_call(m: &mut Oracle, f: u32, this: u32, a: &[u32]) -> u32 {
    m.call(f, ffx_oracle::Conv::Thiscall(this), a).unwrap().eax
}
