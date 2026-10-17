//! The natives no shipped script calls, kept for mods (script round 19):
//! group 1's maths, and the rest of groups 0, 4, 5 and 6 as they are
//! ported.

// w, a, s: the worker, the args words and the stack, as everywhere
#![allow(clippy::many_single_char_names)]

use ffx_bridge::x87;

use super::native::Native;
use super::native_map::pop_st0;
use super::run::NativeHost;
use super::{
    atel_get_basic_work, atel_get_char_handle, atel_get_real_var_adrs, atel_pop_stack_float,
    atel_pop_stack_integer,
};

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

// ---- group 4: the map and lights (Sg_*) ----

/// `n` popped integers, the first popped first.
fn pops<const N: usize>(h: &mut impl NativeHost, s: u32) -> [u32; N] {
    core::array::from_fn(|_| pop(h, s))
}

/// `(v)` -> `f(v)`'s `EAX`.
fn one<H: NativeHost>(h: &mut H, s: u32, f: u32) -> u32 {
    let v = pop(h, s);
    call(h, f, &[v])
}

/// `(v)`: `f(v)`; 0.
fn one0<H: NativeHost>(h: &mut H, s: u32, f: u32) -> u32 {
    one(h, s, f);
    0
}

/// `(i, v f)`: `f(i, v)`; 0.
fn int_float0<H: NativeHost>(h: &mut H, s: u32, f: u32) -> u32 {
    let v = popf(h, s);
    let i = pop(h, s);
    call(h, f, &[i, v]);
    0
}

/// `(d, c, b, a)`: `f(d, c, b, a)`; 0.
fn four0<H: NativeHost>(h: &mut H, s: u32, f: u32) -> u32 {
    let [p1, p2, p3, p4] = pops::<4>(h, s);
    call(h, f, &[p4, p3, p2, p1]);
    0
}

/// `0x402b` (`0x00a77880`): `(i)` -> `Sg_PLightGetEnable(i)` (`0x00831ad0`).
pub fn native_402b<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    one(h, s, 0x0083_1ad0)
}

/// `0x402c` (`0x00a778a0`): `(i, r f)`: `Sg_PLightSetRadius(i, r)`
/// (`0x00831ce0`); 0.
pub fn native_402c<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    int_float0(h, s, 0x0083_1ce0)
}

/// `0x402d` (`0x00a778e0`, float): `(i)` -> `Sg_PLightGetRadius(i)`
/// (`0x00831b80`).
pub fn native_402d<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    let i = pop(h, s);
    callf(h, 0x0083_1b80, &[i])
}

/// `0x402e` (`0x00a77900`): `(i, light)`: `Sg_PLightSetLight(i, light)`
/// (`0x00831c80`); 0.
pub fn native_402e<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [l, i] = pops::<2>(h, s);
    call(h, 0x0083_1c80, &[i, l]);
    0
}

/// `0x402f` (`0x00a77930`): `(i)` -> `Sg_PLightGetLight(i)` (`0x00831b00`).
pub fn native_402f<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    one(h, s, 0x0083_1b00)
}

/// `0x4030` (`0x00a77950`): `(on)`: `Sg_SetCameraFollow(on)`
/// (`0x0082dab0`); 0.
pub fn native_4030<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    one0(h, s, 0x0082_dab0)
}

/// `0x4031` (`0x00a77970`): a jump to `Sg_GetCameraFollow_0082d850()`
/// (bit 1 of `g_CameraFollow` `0x00c496d5`, `Sg_SetCameraFollow`'s; it
/// takes no arguments).
pub fn native_4031<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call(h, 0x0082_d850, &[])
}

/// `0x4032` (`0x00a77980`): `(i, a f)`: `Sg_PLightSetAttenuation(i, a)`
/// (`0x00831bf0`); 0.
pub fn native_4032<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    int_float0(h, s, 0x0083_1bf0)
}

/// `0x4033` (`0x00a779c0`, float): `(i)` -> `Sg_PLightGetAttenuation(i)`
/// (`0x00831a40`).
pub fn native_4033<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    let i = pop(h, s);
    callf(h, 0x0083_1a40, &[i])
}

/// `0x4035` (`0x00a77a00`, float): `Sg_GetMapAlphaFast()` (`0x008205d0`),
/// dropped; 0.0.
pub fn native_4035<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> f64 {
    call(h, 0x0082_05d0, &[]);
    0.0
}

/// `0x4037` (`0x00a77a70`): `(x, y, z)`: `Sg_GlobalGetWind(x, y, z)`
/// (`0x00824dd0`, the popped words as they are); 0.
pub fn native_4037<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [p1, p2, p3] = pops::<3>(h, s);
    call(h, 0x0082_4dd0, &[p3, p2, p1]);
    0
}

/// `0x403d` (`0x00a77bc0`): `(v)`: `Sg_MemSetFast(v)` (`0x0083c520`); 0.
pub fn native_403d<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    one0(h, s, 0x0083_c520)
}

/// `0x4000`, `0x4041`, `0x4042` (`0x00a77d20`, `0x00a77c60`,
/// `0x00a77c70`): empty handlers (`xor eax, eax; ret`); 0.
pub fn native_empty<H: NativeHost>(_h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    0
}

/// `0x4010` (`0x00a77f40`): `(v, a f, b f)`: all three popped and
/// dropped; 0.
pub fn native_4010<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    pop_st0(h, s);
    pop_st0(h, s);
    pop(h, s);
    0
}

/// `(i, x f, y f, z f)`: `f(i, x, y, z)`; 0.
fn int_xyz0<H: NativeHost>(h: &mut H, s: u32, f: u32) -> u32 {
    let [p1, p2, p3] = popfs::<3>(h, s);
    let i = pop(h, s);
    call(h, f, &[i, p3, p2, p1]);
    0
}

/// `0x4011` (`0x00a77f70`): `(i, x f, y f, z f)`: `Sg_LightSetDirXYZ(i,
/// x, y, z)` (`0x008364f0`); 0.
pub fn native_4011<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    int_xyz0(h, s, 0x0083_64f0)
}

/// `0x4012` (`0x00a77fd0`): `(i, x f, y f, z f)`: `0x00836400(i, x, y,
/// z)`; 0.
pub fn native_4012<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    int_xyz0(h, s, 0x0083_6400)
}

/// `0x4018` (`0x00a78160`): `(i)` -> `Sg_IdGetMapTable(i)` (`0x0083e960`).
pub fn native_4018<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    one(h, s, 0x0083_e960)
}

/// `0x4021` (`0x00a78300`): `(v)` -> `Sg_CacheDisposeChr(v)` (`0x00840e60`).
pub fn native_4021<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    one(h, s, 0x0084_0e60)
}

/// `0x4026` (`0x00a78450`): `(i, mask, x f, y f, z f)`:
/// `Sg_PLightSetPos(i, mask, x, y, z)` (`0x00831ca0`); 0.
pub fn native_4026<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [p1, p2, p3] = popfs::<3>(h, s);
    let [m, i] = pops::<2>(h, s);
    call(h, 0x0083_1ca0, &[i, m, p3, p2, p1]);
    0
}

/// `0x4027` (`0x00a784c0`): `(i, x, y, z)`: `Sg_PLightGetPos(i, x, y, z)`
/// (`0x00831b30`, the popped words as they are); 0.
pub fn native_4027<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    four0(h, s, 0x0083_1b30)
}

/// `0x4028` (`0x00a78510`): `(i, r, g, b)`: `Sg_PLightSetColor(i, r, g,
/// b)` (`0x00831c10`); 0.
pub fn native_4028<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    four0(h, s, 0x0083_1c10)
}

/// `0x4029` (`0x00a78560`): `(i, r, g, b)`: `Sg_PLightGetColor(i, r, g,
/// b)` (`0x00831a60`, the popped words as they are); 0.
pub fn native_4029<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    four0(h, s, 0x0083_1a60)
}

// ---- group 5: characters (Ch_*), shared with group 4's copies ----

/// The worker's model (`AtelGetCharHandle`, `0x0086ae80`).
fn chr(h: &mut impl NativeHost, w: u32) -> u32 {
    atel_get_char_handle(h, w)
}

/// Worker `i`'s model.
fn chr_of(h: &mut impl NativeHost, i: u32) -> u32 {
    let w = atel_get_basic_work(h, i.cast_signed());
    atel_get_char_handle(h, w)
}

/// `0x5002`, `0x501d` (`0x00a787a0`, `0x00a79310`, also `0x4049`,
/// `0x4064`): `(seq)`: `Ch_SetSequence(model, seq)` (`0x00837ac0`) for a
/// model; 0.
pub fn native_5002<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let m = chr(h, w);
    if m != 0 {
        call(h, 0x0083_7ac0, &[m, v]);
    }
    0
}

/// `0x5004` (`0x00a788d0`, also `0x404b`): 1 when the model's `+0x736` is
/// set, 0 (also without a model).
pub fn native_5004<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let m = chr(h, w);
    if m == 0 {
        return 0;
    }
    u32::from(h.read::<u8>(m + 0x736) != 0)
}

/// `0x5053` (`0x00a78900`, also `0x409a`): `(i, id)` ->
/// `Ch_ElemGetDirect(worker i's model, id)` (`0x00833bf0`), 0 without a
/// model.
pub fn native_5053<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let id = pop(h, s);
    let i = pop(h, s);
    let m = chr_of(h, i);
    if m == 0 {
        return 0;
    }
    call(h, 0x0083_3bf0, &[m, id])
}

/// `(v)` -> `f(model, v)`'s `EAX`, 0 without a model (the pop first).
fn chr_pop<H: NativeHost>(h: &mut H, w: u32, s: u32, f: u32) -> u32 {
    let v = pop(h, s);
    let m = chr(h, w);
    if m == 0 {
        return 0;
    }
    call(h, f, &[m, v])
}

/// `0x5055` (`0x00a78a10`, also `0x409c`): `(v)` -> `Ch_ModelGetHide(model,
/// v)` (`0x0082add0`), 0 without a model.
pub fn native_5055<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    chr_pop(h, w, s, 0x0082_add0)
}

/// `(b, a)` with the model taken first: `f(model, b, a)`'s `EAX`, `none`
/// without a model.
fn chr_pop2<H: NativeHost>(h: &mut H, w: u32, s: u32, f: u32, none: Option<u32>) -> u32 {
    let m = chr(h, w);
    let a = pop(h, s);
    let b = pop(h, s);
    if m == 0 {
        return 0;
    }
    let r = call(h, f, &[m, b, a]);
    none.unwrap_or(r)
}

/// `0x5059` (`0x00a78bd0`, also `0x40a0`): `(slot, motion)`:
/// `Ch_SetSysMotion(model, slot, motion)` (`0x00837d00`) for a model; 0.
pub fn native_5059<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    chr_pop2(h, w, s, 0x0083_7d00, Some(0))
}

/// `0x505b` (`0x00a78ce0`, also `0x40a2`): `(b, a)` ->
/// `Ch_TextureSetImageBase(model, b, a)` (`0x00831390`), 0 without a model.
pub fn native_505b<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    chr_pop2(h, w, s, 0x0083_1390, None)
}

/// `0x506c` (`0x00a797b0`, also `0x40b3`): `(bits, mask)`:
/// `Ch_PLightSetEnableMask(model, bits, mask)` (`0x00830b00`) for a model;
/// 0.
pub fn native_506c<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    chr_pop2(h, w, s, 0x0083_0b00, Some(0))
}

/// `0x500a` (`0x00a78c20`, also `0x4051`): `(h f)`:
/// `Ch_SetHitHeight(model, h)` (`0x0082b320`) for a model; 0.
pub fn native_500a<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let m = chr(h, w);
    let v = popf(h, s);
    if m != 0 {
        call(h, 0x0082_b320, &[m, v]);
    }
    0
}

/// `0x5011` (`0x00a78e50`, also `0x4058`): `(v)`:
/// `ChN_DisposeSystemMGRP(model's u16 +0, v)` (`0x00836830`) for a model; 0.
pub fn native_5011<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let m = chr(h, w);
    let v = pop(h, s);
    if m != 0 {
        let id = u32::from(h.read::<u16>(m));
        call(h, 0x0083_6830, &[id, v]);
    }
    0
}

/// `0x5062` (`0x00a79030`, start, also `0x40a9`): `(id, n, wait)`: args
/// `+0` = id, `+4` = n, `+8` = wait, `+0xc` = 0; `EAX` the id.
pub fn native_5062<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    h.write(a + 0xc, 0_u32);
    let wait = pop(h, s);
    h.write(a + 8, wait);
    let n = pop(h, s);
    h.write(a + 4, n);
    let id = pop(h, s);
    h.write(a, id);
    id
}

/// `0x5062` (`0x00a79070`, poll): `ChN_ReadSystemMGRPsync(id, n)`
/// (`0x00836a50`): with `+8` set its result goes to `+0xc` and the poll
/// is done (5); else done when it is 0.
pub fn native_5062_poll<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    let id = h.read::<u32>(a);
    let n = h.read::<u32>(a + 4);
    let r = call(h, 0x0083_6a50, &[id, n]);
    if h.read::<u32>(a + 8) != 0 {
        h.write(a + 0xc, r);
        return 5;
    }
    if r == 0 { 5 } else { 0 }
}

/// `0x5062`, `0x5064` (`0x00a79120`, `0x00a79300`, int): the args word
/// `+0xc`.
pub fn native_5062_result<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    h.read::<u32>(a + 0xc)
}

/// `0x5064` (`0x00a79240`, start, also `0x40ab`): `(id, wait)`: args `+0`
/// = id, `+8` = wait, `+0xc` = 0; `EAX` the id.
pub fn native_5064<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    h.write(a + 0xc, 0_u32);
    let wait = pop(h, s);
    h.write(a + 8, wait);
    let id = pop(h, s);
    h.write(a, id);
    id
}

/// `0x5064` (`0x00a79290`, poll): `Ch_DataReadSync(id)` (`0x0082a040`):
/// with `+8` set its result goes to `+0xc` and the poll is done (5); else
/// 1 when it is 0, else 0.
pub fn native_5064_poll<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    let id = h.read::<u32>(a);
    let r = call(h, 0x0082_a040, &[id]);
    if h.read::<u32>(a + 8) != 0 {
        h.write(a + 0xc, r);
        return 5;
    }
    u32::from(r == 0)
}

/// `0x5023`, `0x5024` (`0x00a795b0`, `0x00a79620`, also `0x406a`,
/// `0x406b`): the model asked for and one argument popped, both unused; 0.
pub fn native_5023<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    chr(h, w);
    pop(h, s);
    0
}

/// `0x502b` (`0x00a79b70`, also `0x4072`): the model asked for, unused; 0.
pub fn native_502b<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    chr(h, w);
    0
}

/// `f(model)`'s `EAX`, 0 without a model.
fn chr_get<H: NativeHost>(h: &mut H, m: u32, f: u32) -> u32 {
    if m == 0 {
        return 0;
    }
    call(h, f, &[m])
}

/// `0x506b` (`0x00a796c0`, also `0x40b2`): `Ch_PLightGetEnable(model)`
/// (`0x00830ad0`), 0 without a model.
pub fn native_506b<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let m = chr(h, w);
    chr_get(h, m, 0x0083_0ad0)
}

/// `0x5070` (`0x00a79c30`, also `0x40b7`): `Ch_GetHideCalc(model)`
/// (`0x0082abe0`), 0 without a model.
pub fn native_5070<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let m = chr(h, w);
    chr_get(h, m, 0x0082_abe0)
}

/// `0x502e` (`0x00a79d10`, also `0x4075`): `Ch_IdGetMap(model)`
/// (`0x0083d840`), 0 without a model.
pub fn native_502e<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let m = chr(h, w);
    chr_get(h, m, 0x0083_d840)
}

/// `0x502f` (`0x00a79d60`, also `0x4076`): `(i)` -> `Ch_IdGetMap(worker
/// i's model)`, 0 without a model.
pub fn native_502f<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    let m = chr_of(h, i);
    chr_get(h, m, 0x0083_d840)
}

/// `0x5074` (`0x00a79d90`, also `0x40bb`): `Ch_GetOverlapHit(model)`
/// (`0x0082ac70`), 0 without a model.
pub fn native_5074<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let m = chr(h, w);
    chr_get(h, m, 0x0082_ac70)
}

/// `0x5030` (`0x00a79db0`, also `0x4077`): `Ch_IdGetSound(model)`
/// (`0x0083d900`), 0 without a model.
pub fn native_5030<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let m = chr(h, w);
    chr_get(h, m, 0x0083_d900)
}

/// `0x5031` (`0x00a79e00`, also `0x4078`): `(i)` -> `Ch_IdGetSound(worker
/// i's model)`, 0 without a model.
pub fn native_5031<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    let m = chr_of(h, i);
    chr_get(h, m, 0x0083_d900)
}

/// `0x5077` (`0x00a79e80`, also `0x40be`): `Ch_MouthGetAutoTalk(model)`
/// (`0x00834740`), 0 without a model.
pub fn native_5077<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let m = chr(h, w);
    chr_get(h, m, 0x0083_4740)
}

/// Writes the `f32` at `from` to the variable address `to` when it is not
/// null (`fld`/`fstp`).
fn out_f32(h: &mut impl NativeHost, from: u32, to: u32) {
    if to != 0 {
        let v = x87::store(x87::load(h.read(from)));
        h.write(to, v);
    }
}

/// `0x5027` (`0x00a79860`, also `0x406e`): `(i, element, x f, y f, z f,
/// &ox, &oy, &oz)`: `Ch_ElemGetPos(worker i's model, element, &(x, y, z,
/// 1.0), &out)` (`0x00833d10`) and the result's three words into the
/// variables (null ones skipped; nothing without a model); 0.
pub fn native_5027<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let [i1, i2, i3] = pops::<3>(h, s);
    let [f1, f2, f3] = popfs::<3>(h, s);
    let e = pop(h, s);
    let i = pop(h, s);
    let oz = atel_get_real_var_adrs(h, w, i3);
    let oy = atel_get_real_var_adrs(h, w, i2);
    let ox = atel_get_real_var_adrs(h, w, i1);
    let m = chr_of(h, i);
    if m == 0 {
        return 0;
    }
    let buf = h.scratch();
    let (v, out) = (buf, buf + 0x10);
    for (k, x) in [f3, f2, f1, 1f32.to_bits()].into_iter().enumerate() {
        h.write(v + 4 * u32::try_from(k).unwrap_or(0), x);
    }
    call(h, 0x0083_3d10, &[m, e, v, out]);
    out_f32(h, out, oz);
    out_f32(h, out + 4, oy);
    out_f32(h, out + 8, ox);
    0
}

/// `0x5038` (`0x00a7a250`, also `0x407f`): `(i, &x, &y, &z)`: worker i's
/// model's `+0x6b4`, `+0x6b8`, `+0x6bc` into the variables (null ones
/// skipped); 0.
pub fn native_5038<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let [i1, i2, i3, i] = pops::<4>(h, s);
    let px = atel_get_real_var_adrs(h, w, i3);
    let py = atel_get_real_var_adrs(h, w, i2);
    let pz = atel_get_real_var_adrs(h, w, i1);
    let m = chr_of(h, i);
    if m == 0 {
        return 0;
    }
    out_f32(h, m + 0x6b4, px);
    out_f32(h, m + 0x6b8, py);
    out_f32(h, m + 0x6bc, pz);
    0
}

/// `0x507f` (`0x00a7a380`, also `0x40c6`): `(i, v f)`: `0x0082ae30(model,
/// i, v)` for a model; 0.
pub fn native_507f<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = popf(h, s);
    let i = pop(h, s);
    let m = chr(h, w);
    if m != 0 {
        call(h, 0x0082_ae30, &[m, i, v]);
    }
    0
}

/// `Native_PlayMotionCommon(worker, stack, mode, other)`; 0.
fn play<H: NativeHost>(h: &mut H, w: u32, s: u32, mode: u32, other: u32) -> u32 {
    super::native_chr::native_play_motion_common(h, w, s, mode, other);
    0
}

/// `0x508a`, `0x508c` (`0x00a7a720`, `0x00a7a7a0`, also `0x40d1`,
/// `0x40d3`): `Native_PlayMotionCommon(worker, stack, 2, 1)`; 0.
pub fn native_508a<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    play(h, w, s, 2, 1)
}

/// `0x5041` (`0x00a7a740`, also `0x4088`): `Native_PlayMotionCommon(worker,
/// stack, 0, 0)`; 0.
pub fn native_5041<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    play(h, w, s, 0, 0)
}

/// `0x5043`, `0x5045` (`0x00a7a7c0`, `0x00a7a830`, also `0x408a`,
/// `0x408c`): `Native_PlayMotionCommon(worker, stack, 0, 1)`; 0.
pub fn native_5043<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    play(h, w, s, 0, 1)
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
        0x00a7_7880 => Int(native_402b),
        0x00a7_78a0 => Int(native_402c),
        0x00a7_78e0 => Float(native_402d),
        0x00a7_7900 => Int(native_402e),
        0x00a7_7930 => Int(native_402f),
        0x00a7_7950 => Int(native_4030),
        0x00a7_7970 => Int(native_4031),
        0x00a7_7980 => Int(native_4032),
        0x00a7_79c0 => Float(native_4033),
        0x00a7_7a00 => Float(native_4035),
        0x00a7_7a70 => Int(native_4037),
        0x00a7_7bc0 => Int(native_403d),
        0x00a7_7d20 | 0x00a7_7c60 | 0x00a7_7c70 => Int(native_empty),
        0x00a7_7f40 => Int(native_4010),
        0x00a7_7f70 => Int(native_4011),
        0x00a7_7fd0 => Int(native_4012),
        0x00a7_8160 => Int(native_4018),
        0x00a7_8300 => Int(native_4021),
        0x00a7_8450 => Int(native_4026),
        0x00a7_84c0 => Int(native_4027),
        0x00a7_8510 => Int(native_4028),
        0x00a7_8560 => Int(native_4029),
        0x00a7_87a0 | 0x00a7_9310 => Int(native_5002),
        0x00a7_88d0 => Int(native_5004),
        0x00a7_8900 => Int(native_5053),
        0x00a7_8a10 => Int(native_5055),
        0x00a7_8bd0 => Int(native_5059),
        0x00a7_8ce0 => Int(native_505b),
        0x00a7_97b0 => Int(native_506c),
        0x00a7_8c20 => Int(native_500a),
        0x00a7_8e50 => Int(native_5011),
        0x00a7_9030 => Int(native_5062),
        0x00a7_9070 => Int(native_5062_poll),
        0x00a7_9120 | 0x00a7_9300 => Int(native_5062_result),
        0x00a7_9240 => Int(native_5064),
        0x00a7_9290 => Int(native_5064_poll),
        0x00a7_95b0 | 0x00a7_9620 => Int(native_5023),
        0x00a7_9b70 => Int(native_502b),
        0x00a7_96c0 => Int(native_506b),
        0x00a7_9c30 => Int(native_5070),
        0x00a7_9d10 => Int(native_502e),
        0x00a7_9d60 => Int(native_502f),
        0x00a7_9d90 => Int(native_5074),
        0x00a7_9db0 => Int(native_5030),
        0x00a7_9e00 => Int(native_5031),
        0x00a7_9e80 => Int(native_5077),
        0x00a7_9860 => Int(native_5027),
        0x00a7_a250 => Int(native_5038),
        0x00a7_a380 => Int(native_507f),
        0x00a7_a720 | 0x00a7_a7a0 => Int(native_508a),
        0x00a7_a740 => Int(native_5041),
        0x00a7_a7c0 | 0x00a7_a830 => Int(native_5043),
        _ => return None,
    })
}

/// Every handler [`find`] knows, and whether it returns a float.
pub const PORTED: [(u32, bool); 77] = [
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
    (0x00a7_7880, false),
    (0x00a7_78a0, false),
    (0x00a7_78e0, true),
    (0x00a7_7900, false),
    (0x00a7_7930, false),
    (0x00a7_7950, false),
    (0x00a7_7970, false),
    (0x00a7_7980, false),
    (0x00a7_79c0, true),
    (0x00a7_7a00, true),
    (0x00a7_7a70, false),
    (0x00a7_7bc0, false),
    (0x00a7_7d20, false),
    (0x00a7_7c60, false),
    (0x00a7_7c70, false),
    (0x00a7_7f40, false),
    (0x00a7_7f70, false),
    (0x00a7_7fd0, false),
    (0x00a7_8160, false),
    (0x00a7_8300, false),
    (0x00a7_8450, false),
    (0x00a7_84c0, false),
    (0x00a7_8510, false),
    (0x00a7_8560, false),
    (0x00a7_87a0, false),
    (0x00a7_9310, false),
    (0x00a7_88d0, false),
    (0x00a7_8900, false),
    (0x00a7_8a10, false),
    (0x00a7_8bd0, false),
    (0x00a7_8ce0, false),
    (0x00a7_97b0, false),
    (0x00a7_8c20, false),
    (0x00a7_8e50, false),
    (0x00a7_9030, false),
    (0x00a7_9070, false),
    (0x00a7_9120, false),
    (0x00a7_9300, false),
    (0x00a7_9240, false),
    (0x00a7_9290, false),
    (0x00a7_95b0, false),
    (0x00a7_9620, false),
    (0x00a7_9b70, false),
    (0x00a7_96c0, false),
    (0x00a7_9c30, false),
    (0x00a7_9d10, false),
    (0x00a7_9d60, false),
    (0x00a7_9d90, false),
    (0x00a7_9db0, false),
    (0x00a7_9e00, false),
    (0x00a7_9e80, false),
    (0x00a7_9860, false),
    (0x00a7_a250, false),
    (0x00a7_a380, false),
    (0x00a7_a720, false),
    (0x00a7_a7a0, false),
    (0x00a7_a740, false),
    (0x00a7_a7c0, false),
    (0x00a7_a830, false),
];
