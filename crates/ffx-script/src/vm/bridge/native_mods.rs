//! The natives no shipped script calls, kept for mods (script round 19):
//! group 1's maths, and the rest of groups 0, 4, 5 and 6 as they are
//! ported.

// w, a, s: the worker, the args words and the stack, as everywhere
#![allow(clippy::many_single_char_names)]

use ffx_bridge::x87;

use super::native::Native;
use super::native_map::pop_st0;
use super::run::NativeHost;
use super::{atel_get_real_var_adrs, atel_pop_stack_float, atel_pop_stack_integer};

fn pop(h: &mut impl NativeHost, s: u32) -> u32 {
    atel_pop_stack_integer(h, s)
}

/// The top as `fstp m32` stores `AtelPopStackFloat`'s `ST0`.
fn popf(h: &mut impl NativeHost, s: u32) -> u32 {
    x87::store(x87::load(atel_pop_stack_float(h, s)))
}

/// `n` popped floats (`f32` bits), the first popped first.
fn popfs<const N: usize>(h: &mut impl NativeHost, s: u32) -> [u32; N] {
    core::array::from_fn(|_| popf(h, s))
}

fn call(h: &mut impl NativeHost, sym: u32, args: &[u32]) -> u32 {
    let f = h.rebase(sym);
    h.call_n(f, args)
}

fn callf(h: &mut impl NativeHost, sym: u32, args: &[u32]) -> f64 {
    let f = h.rebase(sym);
    h.call_float(f, args)
}

/// `x` rounded to `f32` and loaded back.
fn r32(x: f64) -> f64 {
    x87::load(x87::store(x))
}

// ---- group 1: maths ----

/// `0x100b` (`0x008778d0`): `(v)` -> `v << 15`.
pub fn native_100b<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    pop(h, s).wrapping_shl(15)
}

/// `AtelCall_Math_Ftoul_RI` (`0x008778f0`, `0x100c`, Fahrenheit's name):
/// `(x f)` -> `x` times the double at `0x00b51588`, rounded to `f32`,
/// truncated (`_ftol2_sse`).
pub fn atel_call_math_ftoul_ri<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let x = pop_st0(h, s);
    let k = f64::from_bits(h.read::<u64>(h.rebase(0x00b5_1588)));
    let v = r32(x87::mul(x, k));
    x87::ftol(h, v)
}

/// `(x f)` -> `f(x)`'s `EAX`.
fn int_of_f<H: NativeHost>(h: &mut H, s: u32, f: u32) -> u32 {
    let x = popf(h, s);
    call(h, f, &[x])
}

/// `(v)` -> `f(v)`'s `EAX`.
fn int_of_i<H: NativeHost>(h: &mut H, s: u32, f: u32) -> u32 {
    let v = pop(h, s);
    call(h, f, &[v])
}

/// `0x100d` (`0x00877920`): `(a f)` -> `Math_AngleTo15Bits(a)`
/// (`0x0088c040`).
pub fn native_100d<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    int_of_f(h, s, 0x0088_c040)
}

/// `0x100e` (`0x00877950`): `(a f)` -> `Math_AngleTo12Bits(a)`
/// (`0x0088bfe0`).
pub fn native_100e<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    int_of_f(h, s, 0x0088_bfe0)
}

/// `0x1008` (`0x00877bd0`): `(angle)` -> `Math_SinTable2(angle)`
/// (`0x0088c380`).
pub fn native_1008<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    int_of_i(h, s, 0x0088_c380)
}

/// `0x1009` (`0x00877bf0`): `(angle)` -> `Math_CosTable(angle)`
/// (`0x0088c120`).
pub fn native_1009<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    int_of_i(h, s, 0x0088_c120)
}

/// `0x100f` (`0x00877980`, float): `(a)` -> `Math_AngleFrom15Bits(a)`
/// (`0x0088c0e0`).
pub fn native_100f<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    let v = pop(h, s);
    callf(h, 0x0088_c0e0, &[v])
}

/// `0x1010` (`0x008779a0`, float): `(a)` -> `Math_AngleFrom12Bits(a)`
/// (`0x0088c0a0`).
pub fn native_1010<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    let v = pop(h, s);
    callf(h, 0x0088_c0a0, &[v])
}

/// `AtelCall_Math_Tan_RF` (`0x00877ad0`, `0x1003`, float, Fahrenheit's
/// name): `(v f)` -> `CItan(v)` (`0x0088c420`).
pub fn atel_call_math_tan_rf<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    let v = popf(h, s);
    callf(h, 0x0088_c420, &[v])
}

/// `AtelCall_Math_ATan_RF` (`0x00877b00`, `0x1004`, float, Fahrenheit's
/// name): `(v f)` -> `atan(v)` through the game's `_CIatan` (the import
/// thunk `0x00949d5c`) on the popped `ST0`, rounded to `f32`.
pub fn atel_call_math_atan_rf<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    let v = pop_st0(h, s);
    let f = h.rebase(0x0094_9d5c);
    r32(h.call_intrinsic1(f, v))
}

/// `AtelCall_Math_InvSqrt_RF` (`0x00877ba0`, `0x1007`, float, Fahrenheit's
/// name): `(v f)` -> `1 / sqrt(v)`, the root and the quotient each rounded
/// to `f32` (`_CIsqrt` is exact at the game's precision).
pub fn atel_call_math_inv_sqrt_rf<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    let v = pop_st0(h, s);
    r32(x87::div(1.0, r32(x87::sqrt(v))))
}

/// `0x1011` (`0x00877c10`, float): `(x0 f, z0 f, x1 f, z1 f)` ->
/// `Math_Atan2XZ(x0, z0, x1, z1)` (`0x0088b430`).
pub fn native_1011<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    let [p1, p2, p3, p4] = popfs::<4>(h, s);
    callf(h, 0x0088_b430, &[p4, p3, p2, p1])
}

/// `0x1012` (`0x00877c80`, float): `(&yaw, &pitch, x0 f, y0 f, z0 f, x1
/// f, y1 f, z1 f)`: `Math_YawPitchTo(&yaw, &pitch, from, to)`
/// (`0x0088b4d0`) into the two variables (`AtelGetRealVarAdrs`); the yaw.
pub fn native_1012<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let [p1, p2, p3, p4, p5, p6] = popfs::<6>(h, s);
    let pitch = pop(h, s);
    let yaw = pop(h, s);
    let py = atel_get_real_var_adrs(h, w, yaw);
    let pp = atel_get_real_var_adrs(h, w, pitch);
    call(h, 0x0088_b4d0, &[py, pp, p6, p5, p4, p3, p2, p1]);
    x87::load(h.read(py))
}

/// `0x1014` (`0x00877d60`, float): `(cur f, target f, step f)` ->
/// `Math_AngleStep(cur, target, step)` (`0x0088b570`).
pub fn native_1014<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    let [p1, p2, p3] = popfs::<3>(h, s);
    callf(h, 0x0088_b570, &[p3, p2, p1])
}

/// `0x1016` (`0x00877e50`): `(r f, x0 f, y0 f, z0 f, x1 f, y1 f, z1 f)` ->
/// `Math_InRadius(r, x0 - x1, y0 - y1, z0 - z1)` (`0x0088b730`), each
/// difference rounded to `f32`.
pub fn native_1016<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [p1, p2, p3, p4, p5, p6, p7] = popfs::<7>(h, s);
    let d = |a: u32, b: u32| x87::store(x87::sub(x87::load(a), x87::load(b)));
    let (dz, dy, dx) = (d(p1, p4), d(p2, p5), d(p3, p6));
    call(h, 0x0088_b730, &[p7, dx, dy, dz])
}

/// `0x1017` (`0x00877ef0`, float): `(&out, &points, n, t f)`:
/// `TkMtCalcSpline(out, points, n, t)` (`0x0088a4a0`) with the two
/// variables' addresses; 0.0.
pub fn native_1017<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let t = popf(h, s);
    let n = pop(h, s);
    let pts = pop(h, s);
    let out = pop(h, s);
    let pp = atel_get_real_var_adrs(h, w, pts);
    let po = atel_get_real_var_adrs(h, w, out);
    call(h, 0x0088_a4a0, &[po, pp, n, t]);
    0.0
}

/// `0x1018` (`0x00877f60`, float): `(&out, a f, b f, c f)`:
/// `0x0088a3a0(out, a, b, c)` with the variable's address; 0.0.
pub fn native_1018<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let [p1, p2, p3] = popfs::<3>(h, s);
    let v = pop(h, s);
    let pv = atel_get_real_var_adrs(h, w, v);
    call(h, 0x0088_a3a0, &[pv, p3, p2, p1]);
    0.0
}

/// `0x101d` (`0x00878110`): `0x00878140(worker, args, stack)`'s record
/// `+4` = 1; 0 (`0x101c` sets 0).
pub fn native_101d<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    let r = call(h, 0x0087_8140, &[w, a, s]);
    h.write(r.wrapping_add(4), 1_u32);
    0
}

/// The Rust version of a round-19 handler, if there is one.
#[must_use]
pub fn find<H: NativeHost>(sym: u32) -> Option<Native<H>> {
    use Native::{Float, Int};
    Some(match sym {
        0x0087_78d0 => Int(native_100b),
        0x0087_78f0 => Int(atel_call_math_ftoul_ri),
        0x0087_7920 => Int(native_100d),
        0x0087_7950 => Int(native_100e),
        0x0087_7980 => Float(native_100f),
        0x0087_79a0 => Float(native_1010),
        0x0087_7ad0 => Float(atel_call_math_tan_rf),
        0x0087_7b00 => Float(atel_call_math_atan_rf),
        0x0087_7ba0 => Float(atel_call_math_inv_sqrt_rf),
        0x0087_7bd0 => Int(native_1008),
        0x0087_7bf0 => Int(native_1009),
        0x0087_7c10 => Float(native_1011),
        0x0087_7c80 => Float(native_1012),
        0x0087_7d60 => Float(native_1014),
        0x0087_7e50 => Int(native_1016),
        0x0087_7ef0 => Float(native_1017),
        0x0087_7f60 => Float(native_1018),
        0x0087_8110 => Int(native_101d),
        _ => return None,
    })
}

/// Every handler [`find`] knows, and whether it returns a float.
pub const PORTED: [(u32, bool); 18] = [
    (0x0087_78d0, false),
    (0x0087_78f0, false),
    (0x0087_7920, false),
    (0x0087_7950, false),
    (0x0087_7980, true),
    (0x0087_79a0, true),
    (0x0087_7ad0, true),
    (0x0087_7b00, true),
    (0x0087_7ba0, true),
    (0x0087_7bd0, false),
    (0x0087_7bf0, false),
    (0x0087_7c10, true),
    (0x0087_7c80, true),
    (0x0087_7d60, true),
    (0x0087_7e50, false),
    (0x0087_7ef0, true),
    (0x0087_7f60, true),
    (0x0087_8110, false),
];
