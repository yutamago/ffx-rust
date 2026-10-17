//! The sea's UV scroll speed nudge (`0x00930c00`) of the `rcui` debug
//! screens (`g_RcuiWork`): a tenth of a step added to one of two speeds.

use ffx_bridge::x87;

/// `speed + step / 10`, as `fld step; fdiv 10.0; fadd speed; fstp`.
#[must_use]
pub fn nudge(speed: f32, step: f32) -> f32 {
    let v = x87::Reg::load(step) / x87::Reg::constant(10.0);
    v.add_m32(speed).store()
}

/// The originals, where the game keeps their state (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::Global;
    use ffx_graphic::host::{Conv, Host};

    /// `g_RcuiWork + 4`: set once the two speeds below were zeroed.
    pub const SPEED_SET: Global<u32> = Global::new(0x0193_c0f4);
    /// `g_RcuiWork + 0x2c`: the U and V speeds (`f32`), the second 4
    /// bytes on.
    pub const SPEED: u32 = 0x0193_c11c;
    /// `Yn_SeaSetUvSpd(u, v)`.
    pub const YN_SEA_SET_UV_SPD: u32 = 0x0091_aea0;

    /// `rcuiSeaAddUvSpd(axis, step)` (`0x00930c00`): the speeds zeroed the
    /// first time; speed `axis` (any index, 4 bytes apart) nudged by
    /// `step / 10` ([`super::nudge`]); both handed to `Yn_SeaSetUvSpd`.
    pub fn add_uv_spd(mem: &mut impl Host, axis: u32, step: f32) {
        let base = mem.rebase(SPEED);
        if SPEED_SET.get(mem) == 0 {
            SPEED_SET.set(mem, 1);
            mem.write(base.wrapping_add(4), 0f32);
            mem.write(base, 0f32);
        }
        let at = base.wrapping_add(axis.wrapping_mul(4));
        let speed: f32 = mem.read(at);
        mem.write(at, super::nudge(speed, step));
        let v: f32 = mem.read(base.wrapping_add(4));
        let u: f32 = mem.read(base);
        let (u, v) = (
            ffx_bridge::x87::round_trip(u),
            ffx_bridge::x87::round_trip(v),
        );
        mem.call_sym(YN_SEA_SET_UV_SPD, Conv::Cdecl, &[u.to_bits(), v.to_bits()]);
    }
}
