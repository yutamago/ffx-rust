//! `ffx-battle`: random numbers.

use super::{call0, mem, ported};

ported! {
    /// `rnd`: the global random generator.
    rnd @ 0x0079_89a0 => extern "C" fn rnd() -> u32 {
        crate::harness::log_call("rnd", 0);
        ffx_battle::rng::bridge::rnd(&mut mem())
    }
    /// `brnd(index)`: one of the 68 indexed generators.
    brnd @ 0x0079_88f0 => extern "C" fn brnd(index: u32) -> u32 {
        crate::harness::log_call("brnd", index);
        ffx_battle::rng::bridge::brnd(&mut mem(), index)
    }
    /// `MsRandMize(n)`: seeds every generator from `Rng_ClockSeed`.
    MsRandMize @ 0x0079_8890 => extern "C" fn ms_rand_mize(n: u32) {
        let clock = call0(0x0079_8950);
        ffx_battle::rng::bridge::ms_rand_mize(&mut mem(), clock, n);
    }
}
