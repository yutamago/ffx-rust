//! The C runtime imports the oracle lacks, replaced at their thunks by the
//! same exact stand-ins in both oracles: `_CIcos` as `x²` (`fmul st, st`),
//! `_CIsin` as `2x` (`fadd st, st`), `_CIatan` as `-x` (`fchs`), `_CIsqrt`
//! as the CPU's (`fsqrt`, IEEE-exact like the CRT's). The Rust gets the
//! same through its callback ([`stand_in`]), so the arithmetic around the
//! calls is compared exactly; the CRT's own functions are the game's.

use ffx_battle::pos::bridge::Trig;
use ffx_oracle::Oracle;

/// Patches the four thunks in one oracle.
pub fn patch(o: &mut Oracle) {
    for (at, code) in [
        (0x0094_98c2, [0xd8, 0xc8, 0xc3]),
        (0x0094_98c8, [0xd8, 0xc0, 0xc3]),
        (0x0094_9d5c, [0xd9, 0xe0, 0xc3]),
        (0x0094_97b8, [0xd9, 0xfa, 0xc3]),
    ] {
        let at = o.rebase(at);
        o.write(at, &code).unwrap();
    }
}

/// The stand-ins as the Rust gets them (`f32` bits in and out).
pub fn stand_in(t: Trig, x: u32) -> u32 {
    use ffx_bridge::x87::{add, load, mul, store};
    match t {
        Trig::Cos => store(mul(load(x), load(x))),
        Trig::Sin => store(add(load(x), load(x))),
        Trig::Atan => store(-load(x)),
    }
}

/// An `f32` in an 80-bit x87 register (as `fld m32` left it), as bits.
pub fn f80_to_f32(r: [u8; 10]) -> u32 {
    let mant = u64::from_le_bytes(r[..8].try_into().unwrap());
    let se = u16::from_le_bytes([r[8], r[9]]);
    let sign = u32::from(se >> 15) << 31;
    let exp = i32::from(se & 0x7fff);
    if exp == 0x7fff {
        let frac = u32::try_from((mant >> 40) & 0x7f_ffff).unwrap();
        return sign | 0x7f80_0000 | frac;
    }
    if mant == 0 {
        return sign;
    }
    // exact: it came from an f32 (24 significant bits)
    #[allow(clippy::cast_precision_loss)]
    let x = (mant as f64) * 2f64.powi(exp - 16383 - 63);
    #[allow(clippy::cast_possible_truncation)]
    let f = x as f32;
    sign | f.to_bits()
}
