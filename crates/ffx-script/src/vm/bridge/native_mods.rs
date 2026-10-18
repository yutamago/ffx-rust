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
    atel_get_basic_work, atel_get_char_handle, atel_get_cur_ctrl_work, atel_get_mes_win_work,
    atel_get_pos_work_adrs, atel_get_real_var_adrs, atel_pop_stack_float, atel_pop_stack_integer,
    atel_push_stack_integer, script_worker_level,
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

// ---- group 0: the general natives ----

/// `AtelPushStackInteger(worker, stack, v)` (`0x0086e3f0`).
fn push(h: &mut impl NativeHost, s: u32, v: u32) {
    atel_push_stack_integer(h, s, v);
}

/// A handler that is a bare `jmp f`: `f(worker, args, stack)`'s `EAX`.
fn tail<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32, f: u32) -> u32 {
    call(h, f, &[w, a, s])
}

/// [`tail`] to a function that returns its value in `ST0`.
fn tailf<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32, f: u32) -> f64 {
    callf(h, f, &[w, a, s])
}

/// `(v)`: `f(v != 0)`; `v != 0`.
fn onoff<H: NativeHost>(h: &mut H, s: u32, f: u32) -> u32 {
    let on = u32::from(pop(h, s) != 0);
    call(h, f, &[on]);
    on
}

/// `(v)`: `f(v)`; `v`.
fn same<H: NativeHost>(h: &mut H, s: u32, f: u32) -> u32 {
    let v = pop(h, s);
    call(h, f, &[v]);
    v
}

/// `f()`; `r`.
fn void<H: NativeHost>(h: &mut H, f: u32, r: u32) -> u32 {
    call(h, f, &[]);
    r
}

/// Message window `i` (`AtelGetMesWinWork`, `0x0086be90`).
fn win(h: &mut impl NativeHost, i: u32) -> u32 {
    atel_get_mes_win_work(h, i.cast_signed())
}

/// The worker's party slot (`+0x40`, -1 for none).
fn party(h: &mut impl NativeHost, w: u32) -> u32 {
    h.read(w + 0x40)
}

/// `0x003c` (`0x00855d50`): `(v)`: `0x0086f8a0(worker, v)`; 0.
pub fn native_003c<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0086_f8a0, &[w, v]);
    0
}

/// `0x01d6` (`0x008564b0`): `(id)`: voice `id`'s stream position
/// (`SndStreamDVDPos`, `0x00887790`; 0 when it is -1) and the two words
/// `0x00885ba0(id, &a, &b)` gives through `0x00885bf0(0, &b, &a, id, pos)`
/// into the CD object's (`GetCD`, `0x0076d0d0`) method `+0x8c`; that
/// value.
pub fn native_01d6<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let cd = call(h, 0x0076_d0d0, &[]);
    let id = pop(h, s);
    let buf = h.scratch();
    let (pa, pb) = (buf + 4, buf);
    call(h, 0x0088_5ba0, &[id, pa, pb]);
    let pos = call(h, 0x0088_7790, &[id]);
    if pos == u32::MAX {
        return 0;
    }
    let x = call(h, 0x0088_5bf0, &[0, pb, pa, id, pos]);
    let m = h.read::<u32>(cd + 0x8c);
    h.call_n(m, &[x]);
    x
}

/// `0x000d` (`0x008565f0`): jumps to `0x0086bc70`.
pub fn native_000d<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0086_bc70)
}

/// `0x000e` (`0x008566a0`): jumps to `0x0086bc80`.
pub fn native_000e<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0086_bc80)
}

/// `0x000f` (`0x00856790`): jumps to `0x0086c1c0`.
pub fn native_000f<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0086_c1c0)
}

/// `0x00ea` (`0x008567d0`): bit 4 of the current ctrl work's byte `+1`.
pub fn native_00ea<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    let c = atel_get_cur_ctrl_work(h);
    u32::from(h.read::<u8>(c + 1) >> 4) & 1
}

/// `0x01d8` (`0x008567e0`): `(stream, entry, from, to, len)`:
/// `Vibrate_StartLevel(stream, entry, from, to, len)` (`0x00876050`), the
/// first four clamped to 0..7, 0..1, 0..0x7f, 0..0x7f; 1.
pub fn native_01d8<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [len, to, from, e, k] = pops::<5>(h, s);
    let c = |v: u32, hi: i32| v.cast_signed().clamp(0, hi).cast_unsigned();
    call(
        h,
        0x0087_6050,
        &[c(k, 7), c(e, 1), c(from, 0x7f), c(to, 0x7f), len],
    );
    1
}

/// `(i, a f, b f, c f, d f)`: `yiSetMIMeParamDObj(i, &(a, b, c, d))`
/// (`0x0090a4d0`); 1.
fn mime<H: NativeHost>(h: &mut H, s: u32) -> u32 {
    let [p1, p2, p3, p4] = popfs::<4>(h, s);
    let i = pop(h, s);
    let v = h.scratch();
    for (k, x) in [p4, p3, p2, p1].into_iter().enumerate() {
        h.write(v + 4 * u32::try_from(k).unwrap_or(0), x);
    }
    call(h, 0x0090_a4d0, &[i, v]);
    1
}

/// `0x01a1` (`0x00856b10`): `(i, a f, b f, c f, d f)`:
/// `yiSetMIMeParamDObj(i, &(a, b, c, d))` (`0x0090a4d0`); 1.
pub fn native_01a1<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    mime(h, s)
}

/// `0x01a2` (`0x00856e90`): pushes a 0 (the fourth float), then as
/// `0x01a1`; 1.
pub fn native_01a2<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    push(h, s, 0);
    mime(h, s)
}

/// `0x01a3` (`0x00857210`): pushes two 0s, then as `0x01a1`; 1.
pub fn native_01a3<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    push(h, s, 0);
    push(h, s, 0);
    mime(h, s)
}

/// `0x0031` (`0x00856ba0`): `Atel_SetWorkerFlag200(worker, 1)`
/// (`0x008712a0`), `0x0086a0e0()`; 1.
pub fn native_0031<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    call(h, 0x0087_12a0, &[w, 1]);
    void(h, 0x0086_a0e0, 1)
}

/// `0x0032` (`0x00856da0`): `Atel_SetWorkerFlag200(worker, 0)`,
/// `0x0086a0e0()`; 0.
pub fn native_0032<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    call(h, 0x0087_12a0, &[w, 0]);
    void(h, 0x0086_a0e0, 0)
}

/// `0x00f0` (`0x008576a0`): `(&v)`: `0x0086d190(worker, variable
/// address)`; 1.
pub fn native_00f0<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let p = atel_get_real_var_adrs(h, w, v);
    call(h, 0x0086_d190, &[w, p]);
    1
}

/// `0x0246` (`0x00857810`): `0x007fb730()`; 1.
pub fn native_0246<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    void(h, 0x007f_b730, 1)
}

/// `0x0247` (`0x00857910`): `0x007fb720()`; 1.
pub fn native_0247<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    void(h, 0x007f_b720, 1)
}

/// `0x0248` (`0x008579f0`): `0x007fb520()`; 1.
pub fn native_0248<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    void(h, 0x007f_b520, 1)
}

/// `0x0249` (`0x00857b60`): `0x007fb510()`; 1.
pub fn native_0249<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    void(h, 0x007f_b510, 1)
}

/// `0x015a` (`0x00857c60`): `(i, b, a)`: message window `i`'s `+0x16` =
/// `b` (`u16`), `+8` and `+0xc` = `a`; 0.
pub fn native_015a<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [a, b, i] = pops::<3>(h, s);
    let r = win(h, i);
    #[allow(clippy::cast_possible_truncation)] // a `mov word`
    h.write(r + 0x16, b as u16);
    h.write(r + 8, a);
    h.write(r + 0xc, a);
    0
}

/// `0x01ec` (`0x00857cf0`): `(b, a)`: `TkSetVibrateStream(b, a, stream)`
/// (`0x00875fb0`) with the current ctrl work's `+0x44` when it is set; 1.
pub fn native_01ec<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [a, b] = pops::<2>(h, s);
    let c = atel_get_cur_ctrl_work(h);
    let st = h.read::<u32>(c + 0x44);
    if st != 0 {
        call(h, 0x0087_5fb0, &[b, a, st]);
    }
    1
}

/// `0x007b` (`0x00857ea0`): `(i)` -> `ReturnConst_008b7cc0(text i)`
/// (`0x008b7cc0`), the text from `Msg_Text1` (`0x0086b170`) when
/// `0x0086bbc0()` is set, else `Msg_Text` (`0x0086bf30`).
pub fn native_007b<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    let f = if call(h, 0x0086_bbc0, &[]) != 0 {
        0x0086_b170
    } else {
        0x0086_bf30
    };
    let t = call(h, f, &[i]);
    call(h, 0x008b_7cc0, &[t])
}

/// `0x00ad` (`0x008581f0`): `(v)`: `AtelSetSaveDicIndex(v)`
/// (`0x00870ef0`); `v`.
pub fn native_00ad<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    same(h, s, 0x0087_0ef0)
}

/// `0x0147` (`0x00858250`): `(v)`: the worker's `+0xbc` = `v` with bit 31
/// set (0 stays 0), `+0xb4` = the `f32` at `0x00b41db0`; that word.
pub fn native_0147<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let mut v = pop(h, s);
    if v != 0 {
        v |= 0x8000_0000;
    }
    let k = x87::store(x87::load(h.read(h.rebase(0x00b4_1db0))));
    h.write(w + 0xbc, v);
    h.write(w + 0xb4, k);
    v
}

/// `0x00f7` (`0x00858290`, start): `(slot, n)`: args `+0` =
/// `AtelGetPartyWorker(slot)` (`0x0086a5e0`), `+4` = `n`, at least 1;
/// the args address, or the worker when `n` is not positive.
pub fn native_00f7<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    let slot = pop(h, s);
    let n = pop(h, s);
    let r = call(h, 0x0086_a5e0, &[slot]);
    h.write(a, r);
    if n.cast_signed() > 0 {
        h.write(a + 4, n);
        a
    } else {
        h.write(a + 4, 1_u32);
        r
    }
}

/// `0x00f7` (`0x00858400`, poll): done (5) for no worker or when
/// `0x00863520(worker, n)` is 0; else 0.
pub fn native_00f7_poll<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    let (c, n) = (h.read::<u32>(a), h.read::<u32>(a + 4));
    if c.cast_signed() < 0 || call(h, 0x0086_3520, &[c, n]) == 0 {
        5
    } else {
        0
    }
}

/// `0x00ae` (`0x00858320`): jumps to `AtelGetSaveDic` (`0x0086c410`).
pub fn native_00ae<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0086_c410)
}

/// `0x00b0` (`0x008585c0`): jumps to `AtelGetBattleSceneGroup`
/// (`0x0086a9a0`).
pub fn native_00b0<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0086_a9a0)
}

/// Message window `i`'s `+4` = `b`, `+6` = `a` (`u16`s); 0.
fn win_pair(h: &mut impl NativeHost, i: u32, b: u32, a: u32) -> u32 {
    let r = win(h, i);
    #[allow(clippy::cast_possible_truncation)] // `mov word`s
    {
        h.write(r + 4, b as u16);
        h.write(r + 6, a as u16);
    }
    0
}

/// `0x0068` (`0x00858640`): `(i, b, a)`: message window `i`'s `+4` = `b`,
/// `+6` = `a`; 0.
pub fn native_0068<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [a, b, i] = pops::<3>(h, s);
    win_pair(h, i, b, a)
}

/// `0x0067` (`0x00858850`): `(i, v)`: `v`'s low and high halves pushed
/// back, then as `0x0068` (the window's `+4` = low, `+6` = high); 0.
pub fn native_0067<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    push(h, s, v & 0xffff);
    push(h, s, v >> 16);
    let [a, b, i] = pops::<3>(h, s);
    win_pair(h, i, b, a)
}

/// `(on)` for the worker's party slot: `f(slot, on != 0)` when it has
/// one; `on != 0`.
fn slot_onoff<H: NativeHost>(h: &mut H, w: u32, s: u32, f: u32) -> u32 {
    let on = u32::from(pop(h, s) != 0);
    let p = party(h, w);
    if p != u32::MAX {
        call(h, f, &[p, on]);
    }
    on
}

/// `0x014b` (`0x008588e0`): `(on)`: `SndSetFoot3dStatus(party slot, on)`
/// (`0x008870b0`) when the worker has a slot; `on != 0`.
pub fn native_014b<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    slot_onoff(h, w, s, 0x0088_70b0)
}

/// `0x0258` (`0x00858a60`, start): `(on)`: when `on != 0` differs from
/// `0x0086bbc0()`, args `+0` = 0, `MsSetSaveConfigEnglish(on)`
/// (`0x00785ed0`) and `TOSwapInternationalEnvInit(0x0086bbc0())`
/// (`0x008b0920`, its `EAX`); else args `+0` = -1.
pub fn native_0258<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    let on = u32::from(pop(h, s) != 0);
    if on == call(h, 0x0086_bbc0, &[]) {
        h.write(a, u32::MAX);
        return a;
    }
    h.write(a, 0_u32);
    call(h, 0x0078_5ed0, &[on]);
    let e = call(h, 0x0086_bbc0, &[]);
    call(h, 0x008b_0920, &[e])
}

/// `0x0258` (`0x00858da0`, poll): done (5) once args `+0` is set; else
/// `TOSwapInternationalEnvExec()` (`0x008b0340`): 0 keeps waiting,
/// anything else sets args `+0` to -1 and is done.
pub fn native_0258_poll<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    if h.read::<u32>(a) == 0 {
        if call(h, 0x008b_0340, &[]) == 0 {
            return 0;
        }
        h.write(a, u32::MAX);
    }
    5
}

/// `0x014c` (`0x00858af0`): `(slot, on)`: `SndSetFoot3dStatus(slot, on !=
/// 0)` (`0x008870b0`); `on != 0`.
pub fn native_014c<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let on = u32::from(pop(h, s) != 0);
    let slot = pop(h, s);
    call(h, 0x0088_70b0, &[slot, on]);
    on
}

/// `0x00b5` (`0x00858cf0`): `(b)`: `Sg_GetTkAntiXYB(&x, &y, &b)`
/// (`0x0083adf0`), then `Sg_SetTkAntiXYB(x, y, b)` (`0x0083b110`) with
/// the popped `b`; `b`.
pub fn native_00b5<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let buf = h.scratch();
    call(h, 0x0083_adf0, &[buf, buf + 4, buf + 8]);
    let b = pop(h, s);
    let (x, y) = (h.read::<u32>(buf), h.read::<u32>(buf + 4));
    call(h, 0x0083_b110, &[x, y, b]);
    b
}

/// `0x00fc` (`0x00858dd0`): `(v)` -> `0x008700e0(v)`.
pub fn native_00fc<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    one(h, s, 0x0087_00e0)
}

/// `0x025a` (`0x008590e0`): jumps to `0x0086a7f0`.
pub fn native_025a<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0086_a7f0)
}

/// `(v f)`: the worker's position block (`AtelGetPosWorkAdrs`) `+off` =
/// `v`; `v`.
fn pos_set(h: &mut impl NativeHost, w: u32, s: u32, off: u32) -> f64 {
    let p = atel_get_pos_work_adrs(h, w);
    let v = popf(h, s);
    h.write(p + off, v);
    x87::load(v)
}

/// `(a f)`: the worker's position block `+off` = `Math_WrapAngle(a)`
/// (`0x0088bc20`); that angle.
fn pos_angle(h: &mut impl NativeHost, w: u32, s: u32, off: u32) -> f64 {
    let p = atel_get_pos_work_adrs(h, w);
    let v = popf(h, s);
    let r = x87::store(callf(h, 0x0088_bc20, &[v]));
    h.write(p + off, r);
    x87::load(r)
}

/// `0x0023` (`0x008591a0`, float): `(v f)`: position block `+0x20` = `v`;
/// `v`.
pub fn native_0023<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    pos_set(h, w, s, 0x20)
}

/// `0x0024` (`0x008592f0`, float): `(v f)`: position block `+0x24` = `v`;
/// `v`.
pub fn native_0024<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    pos_set(h, w, s, 0x24)
}

/// `0x0026` (`0x00859e50`, float): `(a f)`: position block `+0x30` = the
/// wrapped angle; that angle.
pub fn native_0026<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    pos_angle(h, w, s, 0x30)
}

/// `0x0027` (`0x00859f30`, float): `(a f)`: position block `+0x38` = the
/// wrapped angle; that angle.
pub fn native_0027<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    pos_angle(h, w, s, 0x38)
}

/// `0x0022` (`0x00859510`, float): `(a f)`: the current level's turn
/// block (`+0x600 + level * 0x4c`) `+0` = `Math_WrapAngle(a)`; that angle.
pub fn native_0022<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let p = w + u32::from(h.read::<u8>(w + 0x32)) * 0x4c + 0x600;
    let v = popf(h, s);
    let r = x87::store(callf(h, 0x0088_bc20, &[v]));
    h.write(p, r);
    x87::load(r)
}

/// `SndSetFoot3dStatus(party slot, on)` (`0x008870b0`), slot -1
/// included; `on`.
fn foot3d(h: &mut impl NativeHost, w: u32, on: u32) -> u32 {
    let p = party(h, w);
    call(h, 0x0088_70b0, &[p, on]);
    on
}

/// `0x0150` (`0x00859380`): `SndSetFoot3dStatus(party slot, 1)`; 1.
pub fn native_0150<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    foot3d(h, w, 1)
}

/// `0x0152` (`0x008594f0`): `SndSetFoot3dStatus(party slot, 0)`; 0.
pub fn native_0152<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    foot3d(h, w, 0)
}

/// `0x0153` (`0x00859600`): `(slot)`: `SndSetFoot3dStatus(slot, 0)`; 0.
pub fn native_0153<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let slot = pop(h, s);
    call(h, 0x0088_70b0, &[slot, 0]);
    0
}

/// `0x00b9` (`0x008593e0`): `r = Sg_EnableTkAnti(0)` (`0x0083add0`), then
/// `Sg_EnableTkAnti(r)`; `r`.
pub fn native_00b9<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    let r = call(h, 0x0083_add0, &[0]);
    call(h, 0x0083_add0, &[r]);
    r
}

/// `0x01e8` (`0x00859470`): `(v)`: `SndSepClear(v)` (`0x00886d80`); 1.
pub fn native_01e8<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    one(h, s, 0x0088_6d80);
    1
}

/// `0x01b4` (`0x00859490`): `TkSndIsReadShoutVoice() != 0`
/// (`0x00887b00`).
pub fn native_01b4<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    u32::from(call(h, 0x0088_7b00, &[]) != 0)
}

/// `0x01f1` (`0x00859620`, float): jumps to `AtelGetHalfTurnAngle`
/// (`0x00873ad0`).
pub fn native_01f1<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> f64 {
    tailf(h, w, a, s, 0x0087_3ad0)
}

/// `0x01b9` (`0x00859d20`): `0x007ad710(0)`'s `EAX`.
pub fn native_01b9<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call(h, 0x007a_d710, &[0])
}

/// `SndSetKinuStatus(party slot, on)` (`0x00887130`) when the worker has
/// a slot (1), else 0.
fn kinu(h: &mut impl NativeHost, w: u32, on: u32) -> u32 {
    let p = party(h, w);
    if p == u32::MAX {
        return 0;
    }
    call(h, 0x0088_7130, &[p, on]);
    1
}

/// `0x0162` (`0x00859e90`): `SndSetKinuStatus(party slot, 1)`; 1, 0
/// without a slot.
pub fn native_0162<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    kinu(h, w, 1)
}

/// `0x0164` (`0x00859f70`): `SndSetKinuStatus(party slot, 0)`; 1, 0
/// without a slot.
pub fn native_0164<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    kinu(h, w, 0)
}

/// `0x01be` (`0x0085a0e0`): jumps to `AtelGetWaterBattleEffect`
/// (`0x0086c8a0`).
pub fn native_01be<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0086_c8a0)
}

/// `0x0163` (`0x0085a0f0`): `(slot)`: `SndSetKinuStatus(slot, 1)`; 1.
pub fn native_0163<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let slot = pop(h, s);
    call(h, 0x0088_7130, &[slot, 1]);
    1
}

/// `0x0165` (`0x0085a1d0`): `(slot)`: `SndSetKinuStatus(slot, 0)`; 1.
pub fn native_0165<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let slot = pop(h, s);
    call(h, 0x0088_7130, &[slot, 0]);
    1
}

/// `0x00c3` (`0x0085a130`): 1 when the low five bits of the `u16` at `+2`
/// of the current level's (`Script_WorkerLevel`) `+0x48` block are set.
pub fn native_00c3<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let lvl = u32::from(h.read::<u8>(w + 0x32));
    let l = script_worker_level(h, w, lvl.cast_signed());
    let p = h.read::<u32>(l + 0x48);
    u32::from(h.read::<u16>(p + 2) & 0x1f != 0)
}

/// `0x01f5` (`0x0085a160`): `(on)`: `TkMenuSetDummyControlerFlag(on !=
/// 0)` (`0x008ca570`); `on != 0`.
pub fn native_01f5<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    onoff(h, s, 0x008c_a570)
}

/// `0x01f6` (`0x0085a2d0`): `(b)`: `TkMenuSetDummyControlerPending(1 <<
/// b)` (`0x008ca560`); `b`.
pub fn native_01f6<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let b = pop(h, s);
    call(h, 0x008c_a560, &[1_u32.wrapping_shl(b)]);
    b
}

/// `0x01ff` (`0x0085a300`): `SndSetFootStatus(party slot, 1)`
/// (`0x00887100`) and `SndSetKinuStatus(party slot, 1)`, each when the
/// worker has a slot; 1.
pub fn native_01ff<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    for f in [0x0088_7100, 0x0088_7130] {
        let p = party(h, w);
        if p != u32::MAX {
            call(h, f, &[p, 1]);
        }
    }
    1
}

/// `0x01c1` (`0x0085a420`): `0x00873c40()`; 1.
pub fn native_01c1<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    void(h, 0x0087_3c40, 1)
}

/// `0x0232` (`0x0085a4b0`, start): `(b)`: args `+0` = `1 << b`, `+4` =
/// `ReturnConst2_00a54750()` (2); that count.
pub fn native_0232<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    let b = pop(h, s);
    let n = call(h, 0x00a5_4750, &[]);
    h.write(a, 1_u32.wrapping_shl(b));
    h.write(a + 4, n);
    n
}

/// `0x0232` (`0x0085a5f0`, poll): the count decremented,
/// `TkMenuSetDummyControlerPending(args +0)`; done (5) when the count is
/// no longer positive.
pub fn native_0232_poll<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    let n = h.read::<u32>(a + 4).wrapping_sub(1);
    h.write(a + 4, n);
    let b = h.read::<u32>(a);
    call(h, 0x008c_a560, &[b]);
    if n.cast_signed() > 0 { 0 } else { 5 }
}

/// `0x01f7` (`0x0085a800`): `(a, b, c)`:
/// `AtelRegistBattleEndMovieFadeout(a, b, c)` (`0x0086e840`); 0.
pub fn native_01f7<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [c, b, a] = pops::<3>(h, s);
    call(h, 0x0086_e840, &[a, b, c]);
    0
}

/// `0x0227` (`0x0085ac20`): `(on)`: `0x00873dd0(on != 0)`; `on != 0`.
pub fn native_0227<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    onoff(h, s, 0x0087_3dd0)
}

/// `0x0228` (`0x0085b060`): `(flags, b, a)`: how far the current level's
/// turn target (`+0x884 + level * 0x44`) is from the position block's
/// yaw (`+0x34`), wrapped (`Math_WrapAngle`) and made absolute, over the
/// double at `0x00b92288` (flags bit 1) or `0x00b11338`, at most 1, times
/// `a` (bit 1) or `b`, truncated; each step rounded to `f32`.
pub fn native_0228<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let p = atel_get_pos_work_adrs(h, w);
    let k = u32::from(h.read::<u8>(w + 0x32));
    let [a, b, flags] = pops::<3>(h, s);
    let t = x87::load(h.read(w + k * 0x44 + 0x884));
    let d = x87::store(x87::sub(t, x87::load(h.read(p + 0x34))));
    let r = r32(callf(h, 0x0088_bc20, &[d]).abs());
    let (sym, n) = if flags & 2 != 0 {
        (0x00b9_2288, a)
    } else {
        (0x00b1_1338, b)
    };
    let q = r32(x87::div(r, f64::from_bits(h.read(h.rebase(sym)))));
    // fcom: anything not below 1.0 (NaN is "below") becomes 1.0
    let q = if q < 1.0 || q.is_nan() { q } else { 1.0 };
    #[allow(clippy::cast_precision_loss)] // fild then fstp m32
    let m = r32(f64::from(n.cast_signed()));
    let v = x87::mul(q, m);
    x87::ftol(h, v)
}

/// `0x015c` (`0x0085b530`): `(on)`: `TkSndSetFootKickStay(on != 0)`
/// (`0x00887b40`); `on != 0`.
pub fn native_015c<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    onoff(h, s, 0x0088_7b40)
}

/// `0x0113` (`0x0085b5c0`): jumps to `AtelGetSave20` (`0x0086d9d0`).
pub fn native_0113<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0086_d9d0)
}

/// `0x0168` (`0x0085b990`): `(v)`: `0x00886950(v)`; `v`.
pub fn native_0168<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    same(h, s, 0x0088_6950)
}

/// `0x01fd` (`0x0085ba20`): `(b, a)` -> `MsSetSaveCommandWithPrefix(b, a,
/// 0)` (`0x00874200`).
pub fn native_01fd<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [a, b] = pops::<2>(h, s);
    call(h, 0x0087_4200, &[b, a, 0])
}

/// `0x0169` (`0x0085bab0`): `(b, a)`: `0x008868e0(b, a)`; 1.
pub fn native_0169<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [a, b] = pops::<2>(h, s);
    call(h, 0x0088_68e0, &[b, a]);
    1
}

/// `0x0118` (`0x0085bae0`): `(i)`: worker `i`'s `+0x36` gets bit 10 and
/// loses bit 9; `i`.
pub fn native_0118<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    let b = atel_get_basic_work(h, i.cast_signed());
    let v = (h.read::<u16>(b + 0x36) & 0xfdff) | 0x400;
    h.write(b + 0x36, v);
    i
}

/// `0x01fe` (`0x0085bb20`): `(b, a)` -> `MsGetSaveCommand(b, a)`
/// (`0x00785020`).
pub fn native_01fe<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [a, b] = pops::<2>(h, s);
    call(h, 0x0078_5020, &[b, a])
}

/// `(v)`: `f(worker, current level, v)`; `v`.
fn level_set<H: NativeHost>(h: &mut H, w: u32, s: u32, f: u32) -> u32 {
    let v = pop(h, s);
    let l = u32::from(h.read::<u8>(w + 0x32));
    call(h, f, &[w, l, v]);
    v
}

/// `0x0072` (`0x0085bb50`): `(v)`: `SetField_00870520(worker, level, v)`
/// (the level's move block `+0x5bc` word); `v`.
pub fn native_0072<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    level_set(h, w, s, 0x0087_0520)
}

/// `0x0073` (`0x0085bc90`): `(v)`: `SetField_00871170(worker, level, v)`
/// (the level's turn block `+0x868` word); `v`.
pub fn native_0073<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    level_set(h, w, s, 0x0087_1170)
}

/// `0x00cd` (`0x0085bbe0`): 1 when the worker's party slot (`+0x40`) is
/// one of the three `MsGetSavePartyMember` (`0x007852f0`) gives and not
/// 0xff, else 0.
pub fn native_00cd<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let p = party(h, w);
    let buf = h.scratch();
    call(h, 0x0078_52f0, &[buf, buf + 4, buf + 8]);
    for k in 0..3 {
        let m = h.read::<u32>(buf + 4 * k);
        if p == m && m != 0xff {
            return 1;
        }
    }
    0
}

/// `0x020c` (`0x0085bcc0`): `(i)`: worker `i`'s `+0x38` gets bit 8; `i`
/// pushed back and popped, and that worker's `+0x38` gets bit 2; 1.
pub fn native_020c<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    let b = atel_get_basic_work(h, i.cast_signed());
    let v = h.read::<u16>(b + 0x38) | 0x100;
    h.write(b + 0x38, v);
    push(h, s, i);
    let j = pop(h, s);
    flag38(h, j, 4);
    1
}

/// Worker `i`'s `+0x38` gets `bits`.
fn flag38(h: &mut impl NativeHost, i: u32, bits: u16) {
    let b = atel_get_basic_work(h, i.cast_signed());
    let v = h.read::<u16>(b + 0x38) | bits;
    h.write(b + 0x38, v);
}

/// `0x0075` (`0x0085bf40`): `(v)`: `SetFields_0086f120(worker, v)`; `v`.
pub fn native_0075<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0086_f120, &[w, v]);
    v
}

/// `0x0079`, `0x0123`, `0x0124` (`0x0085c040`, `0x0085d440`,
/// `0x0085d530`): `(v)` -> `v` (popped and returned).
pub fn native_0079<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    pop(h, s)
}

/// `0x020e` (`0x0085c090`): `AtelGetSphereMonitor()` (`0x0086c4b0`), its
/// low word.
pub fn native_020e<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call(h, 0x0086_c4b0, &[]) & 0xffff
}

/// `0x003e` (`0x0085c470`): `(mask)`: `Atel_CopyTurnAngles(worker, mask)`
/// (`0x0086db80`); 0.
pub fn native_003e<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0086_db80, &[w, v]);
    0
}

/// `0x0170` (`0x0085c530`): `(b, a)`: the save block's (`0x0088c830()`)
/// `+0xb8` = `a`, `+0xba` = `b` (`u16`s), `TkBackupEventJump(block)`
/// (`0x0088e800`); `a`.
pub fn native_0170<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let r = call(h, 0x0088_c830, &[]);
    let [a, b] = pops::<2>(h, s);
    #[allow(clippy::cast_possible_truncation)] // `mov word`s
    {
        h.write(r + 0xb8, a as u16);
        h.write(r + 0xba, b as u16);
    }
    call(h, 0x0088_e800, &[r]);
    a
}

/// `0x007e` (`0x0085c580`): jumps to `AtelGetNowCtrlTopId`
/// (`0x0086c1a0`).
pub fn native_007e<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0086_c1a0)
}

/// The current level's (`Script_WorkerLevel`) `+0x48` block's `f32` at
/// `off`.
fn level48_f32(h: &mut impl NativeHost, w: u32, off: u32) -> f64 {
    let lvl = u32::from(h.read::<u8>(w + 0x32));
    let l = script_worker_level(h, w, lvl.cast_signed());
    let p = h.read::<u32>(l + 0x48);
    x87::load(h.read(p + off))
}

/// `0x0040` (`0x0085c740`, float): the current level's `+0x48` block's
/// `+0x3c`.
pub fn native_0040<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> f64 {
    level48_f32(h, w, 0x3c)
}

/// `0x0041` (`0x0085c840`, float): the current level's `+0x48` block's
/// `+0x40`.
pub fn native_0041<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> f64 {
    level48_f32(h, w, 0x40)
}

/// The worker's `+0x2e` (its index) pushed and popped back, then
/// `f(that worker, -1)`; 0.
fn stop_self<H: NativeHost>(h: &mut H, w: u32, s: u32, f: u32) -> u32 {
    let me = u32::from(h.read::<u16>(w + 0x2e));
    push(h, s, me);
    let i = pop(h, s);
    let b = atel_get_basic_work(h, i.cast_signed());
    call(h, f, &[b, u32::MAX]);
    0
}

/// `0x00d3` (`0x0085c7d0`): `Script_StopMove(worker, -1)` (`0x00863e10`)
/// through its own index pushed and popped; 0.
pub fn native_00d3<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    stop_self(h, w, s, 0x0086_3e10)
}

/// `0x00d4` (`0x0085c910`): `Script_StopTurn(worker, -1)` (`0x00863fe0`)
/// the same way; 0.
pub fn native_00d4<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    stop_self(h, w, s, 0x0086_3fe0)
}

/// `0x0211` (`0x0085c810`): `(i)`: `AtelSetHelpEventFlag(i, 0)`
/// (`0x008701f0`); `i`.
pub fn native_0211<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    call(h, 0x0087_01f0, &[i, 0]);
    i
}

/// `0x011e` (`0x0085caf0`): `(b, a)`: `0x00874ad0(b, a)`; `b`.
pub fn native_011e<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [a, b] = pops::<2>(h, s);
    call(h, 0x0087_4ad0, &[b, a]);
    b
}

/// `0x0214` (`0x0085cbe0`): `(i, e)` -> `Ch_OptposGetElement(model, e, 0)`
/// (`0x00835660`) for worker `i` when it is a character (`+0xaa` 1) with a
/// model (`+0x9c`), else 0.
pub fn native_0214<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [e, i] = pops::<2>(h, s);
    let b = atel_get_basic_work(h, i.cast_signed());
    if h.read::<u8>(b + 0xaa) != 1 {
        return 0;
    }
    let m = h.read::<u32>(b + 0x9c);
    if m == 0 {
        return 0;
    }
    call(h, 0x0083_5660, &[m, e, 0])
}

/// `0x0178` (`0x0085cc30`): `(i)` -> `AtelGetEffectFlag(i)`
/// (`0x0086b030`).
pub fn native_0178<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    one(h, s, 0x0086_b030)
}

/// `0x008a` (`0x0085d3b0`): jumps to `AtelGetEventMapCameraId`
/// (`0x0086b5d0`).
pub fn native_008a<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0086_b5d0)
}

/// `0x0218` (`0x0085d3f0`, like `0x01ac`): `(i)` ->
/// `MsGetMonsterCapture(i)` (`0x00790af0`).
pub fn native_0218<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    one(h, s, 0x0079_0af0)
}

/// `0x024e` (`0x0085d4b0`): `(v)`: the current ctrl work's `+0x214` =
/// `v`; `v`.
pub fn native_024e<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let c = atel_get_cur_ctrl_work(h);
    let v = pop(h, s);
    h.write(c + 0x214, v);
    v
}

/// `0x0219` (`0x0085d500`, like `0x01af`): `(b, a)` ->
/// `AddMonsterCapture(b, a)` (`0x00790b90`).
pub fn native_0219<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [a, b] = pops::<2>(h, s);
    call(h, 0x0079_0b90, &[b, a])
}

/// `0x00f1` (`0x0085d620`): `(key)` -> `Script_LineActorSide(worker,
/// key)` (`0x008632b0`) when it is not negative; else, when `key` is the
/// position block's `+0x28` or that is 0xffff, the worker's `+0x36` bits
/// 1-2 (3 as -1); else the negative result.
pub fn native_00f1<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let key = pop(h, s);
    let r = call(h, 0x0086_32b0, &[w, key]);
    if r.cast_signed() >= 0 {
        return r;
    }
    let p = atel_get_pos_work_adrs(h, w);
    let t = h.read::<u16>(p + 0x28);
    if key != u32::from(t) && t != 0xffff {
        return r;
    }
    let x = u32::from(h.read::<u16>(w + 0x36) >> 1) & 3;
    if x == 3 { u32::MAX } else { x }
}

/// `0x017d` (`0x0085d680`): `(id)`: `SndMusicMultiStart(id)`
/// (`0x008868b0`), `AtelRegistMusicNumber(id)` (`0x00873d00`); 1.
pub fn native_017d<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let id = pop(h, s);
    call(h, 0x0088_68b0, &[id]);
    call(h, 0x0087_3d00, &[id]);
    1
}

/// The low word of `f()` (one of the pad getters around `0x0086af30`).
fn pad(h: &mut impl NativeHost, f: u32) -> u32 {
    call(h, f, &[]) & 0xffff
}

/// `Pad_MapButtons(0, 0, pad f)` (`0x00888c40`), its low word.
fn mapped(h: &mut impl NativeHost, f: u32) -> u32 {
    let b = pad(h, f);
    call(h, 0x0088_8c40, &[0, 0, b]) & 0xffff
}

/// `0x0045` (`0x0085d760`): the buttons of `0x0086af50()` through
/// `Pad_MapButtons`.
pub fn native_0045<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    mapped(h, 0x0086_af50)
}

/// `0x024f` (`0x0085d890`): the buttons of `0x0086af60()` through
/// `Pad_MapButtons`.
pub fn native_024f<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    mapped(h, 0x0086_af60)
}

/// `0x008c` (`0x0085d8b0`): the current ctrl work's `+0x1f4`.
pub fn native_008c<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    let c = atel_get_cur_ctrl_work(h);
    h.read(c + 0x1f4)
}

/// `0x008e` (`0x0085db40`, float): `(v f)`: position block `+0x1c` = `v`;
/// `v`.
pub fn native_008e<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    pos_set(h, w, s, 0x1c)
}

/// `0x021c` (`0x0085db90`): `(id)` -> `AtelCheckCaptMonster(2, 0, id, 0)`
/// (`0x00872aa0`).
pub fn native_021c<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let id = pop(h, s);
    call(h, 0x0087_2aa0, &[2, 0, id, 0])
}

/// `0x0048` (`0x0085dcc0`): `AtelGetControlerT()` (`0x0086af30`), the low
/// word.
pub fn native_0048<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    pad(h, 0x0086_af30)
}

/// `0x0049` (`0x0085ddc0`): `0x0086af50()`, the low word.
pub fn native_0049<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    pad(h, 0x0086_af50)
}

/// `0x0250` (`0x0085de60`): `0x0086af60()`, the low word.
pub fn native_0250<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    pad(h, 0x0086_af60)
}

/// `0x004a` (`0x0085dff0`): `AtelGetControler()` (`0x0086af70`), the low
/// word.
pub fn native_004a<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    pad(h, 0x0086_af70)
}

/// `0x004b` (`0x0085e0d0`): `0x0086af80()`, the low word.
pub fn native_004b<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    pad(h, 0x0086_af80)
}

/// `v` clamped to `lo..=hi` (signed).
fn clamp(v: u32, lo: i32, hi: i32) -> u32 {
    v.cast_signed().clamp(lo, hi).cast_unsigned()
}

/// `0x0186` (`0x0085dd50`): `(pitch, b)`: `SndMusicPitchDirect(pitch, b)`
/// (`0x00886930`), `pitch` clamped to -0x80..0x7f and `b` to 0..0xff; 1.
pub fn native_0186<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [b, p] = pops::<2>(h, s);
    call(h, 0x0088_6930, &[clamp(p, -0x80, 0x7f), clamp(b, 0, 0xff)]);
    1
}

/// `0x012c` (`0x0085ddd0`): `(b, a)`: the current ctrl work's `+4` = `a`,
/// `+6` = `b` (`u16`s); 1.
pub fn native_012c<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let c = atel_get_cur_ctrl_work(h);
    let [a, b] = pops::<2>(h, s);
    #[allow(clippy::cast_possible_truncation)] // `mov word`s
    {
        h.write(c + 6, b as u16);
        h.write(c + 4, a as u16);
    }
    1
}

/// `0x0187` (`0x0085df00`): `(id, pitch, frames)`:
/// `SndMusicPitchTranslate(id, pitch, frames)` (`0x00886940`), `pitch`
/// clamped to -0x80..0x7f and `frames` to 0..0xff; `id`.
pub fn native_0187<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [n, p, id] = pops::<3>(h, s);
    call(
        h,
        0x0088_6940,
        &[id, clamp(p, -0x80, 0x7f), clamp(n, 0, 0xff)],
    );
    id
}

/// `0x021f` (`0x0085dfc0`): `(i)`: `AtelSetLoveParamDisable(i, 0)`
/// (`0x00870250`); `i`.
pub fn native_021f<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    call(h, 0x0087_0250, &[i, 0]);
    i
}

/// `0x01c7` (`0x0085e000`): jumps to `0x008741d0`.
pub fn native_01c7<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0087_41d0)
}

/// `0x01cb` (`0x0085e0e0`): `(i)`: worker `i`'s `+0x38` gets bit 2; 1.
pub fn native_01cb<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    flag38(h, i, 4);
    1
}

/// `0x012f` (`0x0085e170`): `(i, n)`: the save block's (`0x0088c830()`)
/// counter `i` (clamped to 0..7, `+0x2c + 4 i`) less `n`, 0 when that is
/// negative; the new value.
pub fn native_012f<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [n, i] = pops::<2>(h, s);
    let r = call(h, 0x0088_c830, &[]);
    let p = r + clamp(i, 0, 7) * 4 + 0x2c;
    let x = h.read::<u32>(p).wrapping_sub(n);
    let v = if x.cast_signed() < 0 { 0 } else { x };
    h.write(p, v);
    v
}

/// `0x0222` (`0x0085e380`): `(i)` -> the word at `0x00c8756c + 4 i`.
pub fn native_0222<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    let t = h.rebase(0x00c8_756c);
    h.read(t.wrapping_add(i.wrapping_mul(4)))
}

/// 1 when bit `b` of `x` is set.
fn bit(x: u32, b: u32) -> u32 {
    u32::from(x & 1_u32.wrapping_shl(b) != 0)
}

/// `(b)`: bit `b` of [`mapped`]`(f)`.
fn mapped_bit<H: NativeHost>(h: &mut H, s: u32, f: u32) -> u32 {
    let b = pop(h, s);
    let x = mapped(h, f);
    bit(x, b)
}

/// `(b)`: bit `b` of [`pad`]`(f)`.
fn pad_bit<H: NativeHost>(h: &mut H, s: u32, f: u32) -> u32 {
    let b = pop(h, s);
    let x = pad(h, f);
    bit(x, b)
}

/// `0x004f` (`0x0085e3c0`): `(b)`: bit `b` of `0x0086af50()`'s buttons
/// through `Pad_MapButtons`.
pub fn native_004f<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    mapped_bit(h, s, 0x0086_af50)
}

/// `0x004e` (`0x0085ea80`): `(b)`: bit `b` of `0x0086af80()`'s buttons
/// through `Pad_MapButtons`.
pub fn native_004e<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    mapped_bit(h, s, 0x0086_af80)
}

/// `0x0053` (`0x0085eda0`): `(b)`: bit `b` of `0x0086af50()`.
pub fn native_0053<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    pad_bit(h, s, 0x0086_af50)
}

/// `0x0252` (`0x0085ef50`): `(b)`: bit `b` of `0x0086af60()`.
pub fn native_0252<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    pad_bit(h, s, 0x0086_af60)
}

/// `0x0052` (`0x0085f2a0`): `(b)`: bit `b` of `0x0086af80()`.
pub fn native_0052<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    pad_bit(h, s, 0x0086_af80)
}

/// `0x0223` (`0x0085e460`): `(on)`: bit 4 of the save menu word
/// (`MsGetSaveMenu`, `0x00785290`) = `on != 0`, `TkRefreshActuator()`
/// (`0x00872480`); `on != 0`.
pub fn native_0223<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let m = call(h, 0x0078_5290, &[]);
    let on = u32::from(pop(h, s) != 0);
    let v = (h.read::<u32>(m) & !0x10) | (on << 4);
    h.write(m, v);
    call(h, 0x0087_2480, &[]);
    on
}

/// `0x01cd` (`0x0085e5d0`): jumps to `AtelGetMusicNumber` (`0x00873b00`).
pub fn native_01cd<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0087_3b00)
}

/// `0x0131` (`0x0085e760`): `(i)`: clears bit `i` of the save block's
/// (`0x0088c830()`) 20-word bit set at `+0x4c` (nothing past it); 1.
pub fn native_0131<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let r = call(h, 0x0088_c830, &[]);
    let i = pop(h, s);
    if i >> 5 < 0x14 {
        let p = r + (i >> 5) * 4 + 0x4c;
        let v = h.read::<u32>(p) & !(1 << (i & 0x1f));
        h.write(p, v);
    }
    1
}

/// `0x0238` (`0x0085eb30`): `(i)`: bit `i` of the save block's bit set at
/// `+0x4c` (0 past it).
pub fn native_0238<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let r = call(h, 0x0088_c830, &[]);
    let i = pop(h, s);
    if i >> 5 >= 0x14 {
        return 0;
    }
    let x = h.read::<u32>(r + (i >> 5) * 4 + 0x4c);
    bit(x, i & 0x1f)
}

/// `0x0173` (`0x0085e7a0`, float): the position block's `+0x24`.
pub fn native_0173<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> f64 {
    let p = atel_get_pos_work_adrs(h, w);
    x87::load(h.read(p + 0x24))
}

/// `0x0224` (`0x0085e800`): bit 4 of the save menu word.
pub fn native_0224<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    let m = call(h, 0x0078_5290, &[]);
    (h.read::<u32>(m) >> 4) & 1
}

/// `(i)`: worker `i`'s position block `f32` at `off`.
fn pos_of(h: &mut impl NativeHost, s: u32, off: u32) -> f64 {
    let i = pop(h, s);
    let b = atel_get_basic_work(h, i.cast_signed());
    let p = atel_get_pos_work_adrs(h, b);
    x87::load(h.read(p + off))
}

/// `0x0176` (`0x0085ec30`, float): `(i)`: worker `i`'s position block
/// `+0x30`.
pub fn native_0176<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    pos_of(h, s, 0x30)
}

/// `0x0182` (`0x0085ed10`, float): `(i)`: worker `i`'s position block
/// `+0x38`.
pub fn native_0182<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    pos_of(h, s, 0x38)
}

/// `0x0175` (`0x0085eec0`, float): `(i)`: worker `i`'s position block
/// `+0x24`.
pub fn native_0175<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    pos_of(h, s, 0x24)
}

/// The worker's `+0x38` gets `bits`; 1.
fn self38(h: &mut impl NativeHost, w: u32, bits: u16) -> u32 {
    let v = h.read::<u16>(w + 0x38) | bits;
    h.write(w + 0x38, v);
    1
}

/// `SetFieldBits38_0085ed40` (`0x022a`): the worker's `+0x38` gets bit
/// 10; 1 (the auto-port's shim stays).
pub fn native_022a<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    self38(h, w, 0x400)
}

/// `SetFieldBits38_0085fb10` (`0x0230`): the worker's `+0x38` gets bit
/// 11; 1 (the auto-port's shim stays).
pub fn native_0230<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    self38(h, w, 0x800)
}

/// `0x018c` (`0x0085ed60`): `(id, pitch, x)`: `SndSePitchDirect(id,
/// pitch, x)` (`0x00886d20`); 1.
pub fn native_018c<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [x, p, id] = pops::<3>(h, s);
    call(h, 0x0088_6d20, &[id, p, x]);
    1
}

/// `0x018d` (`0x0085ef00`): `(id, a, pitch, b)`: `SndSePitchTranslate(id,
/// a, pitch, b)` (`0x00886d50`); 1.
pub fn native_018d<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [b, p, a, id] = pops::<4>(h, s);
    call(h, 0x0088_6d50, &[id, a, p, b]);
    1
}

/// `SetFields_0085eea0` (`0x022b` start): args `+0` = 0, `+4` = 10 (the
/// auto-port's shim stays); the args address.
pub fn native_022b<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    h.write(a, 0_u32);
    h.write(a + 4, 10_u32);
    a
}

/// `0x022b` (`0x0085efb0`, poll): state 0 counts `+4` down, then moves to
/// state 1; state 1 is done (5) once `GetGlobal01a85f70_00a547e0()` is
/// set; else 0.
pub fn native_022b_poll<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    let (st, n) = (h.read::<u32>(a), h.read::<u32>(a + 4));
    match st {
        0 if n != 0 => h.write(a + 4, n - 1),
        0 => h.write(a, 1_u32),
        1 if call(h, 0x00a5_47e0, &[]) != 0 => return 5,
        _ => {}
    }
    0
}

/// `0x0008`, `0xc02b` (`0x0085eef0`, `0x00878e50`): 0 (`xor eax, eax; ret`).
pub fn native_0008<H: NativeHost>(_h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    0
}

/// `0x01d3`, `0xc04f` (`0x00860720`, `0x00879380`): 1 (`mov eax, 1; ret`).
pub fn native_01d3<H: NativeHost>(_h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    1
}

/// `0x00dd` (`0x0085f070`): `MsEventEffectGetStat()` (`0x00788bd0`): 0 and
/// 6 give 0, 1, 3 and 5 give 1, the rest themselves.
pub fn native_00dd<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    match call(h, 0x0078_8bd0, &[]) {
        0 | 6 => 0,
        1 | 3 | 5 => 1,
        r => r,
    }
}

/// `0x018e` (`0x0085f110`): `(v)`: `0x00873d20(v)`,
/// `SndVagStreamVolumeDirect(v)` (`0x00887990`); 0.
pub fn native_018e<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0087_3d20, &[v]);
    call(h, 0x0088_7990, &[v]);
    0
}

/// `0x0009` (`0x0085f160`): `AtelGetCharHandle(worker)` (`0x0086ae80`).
pub fn native_0009<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    atel_get_char_handle(h, w)
}

/// `0x0208` (`0x0085f210`): `AtelBindEff2OffSome(worker, 1)`
/// (`0x00862680`) for a character (`+0xaa` 1) with a model; 1.
pub fn native_0208<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    if h.read::<u8>(w + 0xaa) == 1 && h.read::<u32>(w + 0x9c) != 0 {
        call(h, 0x0086_2680, &[w, 1]);
    }
    1
}

/// `GetFieldU8aa_0085f2d0` (`0x000a`): the worker's type (`+0xaa`; the
/// auto-port's shim stays).
pub fn native_000a<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    u32::from(h.read::<u8>(w + 0xaa))
}

/// `GetFieldU16a8_0085f360` (`0x000b`): the worker's `+0xa8` (`u16`; the
/// auto-port's shim stays).
pub fn native_000b<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    u32::from(h.read::<u16>(w + 0xa8))
}

/// `0x022c` (`0x0085f2e0`): `(i, v)`: `SetByte_0186a429_008ca370(i, v)`;
/// 1.
pub fn native_022c<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [v, i] = pops::<2>(h, s);
    call(h, 0x008c_a370, &[i, v]);
    1
}

/// `0x0190` (`0x0085f330`): `(v)`: `0x00873d10(v)`, `0x00887970(v)`; 0.
pub fn native_0190<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0087_3d10, &[v]);
    call(h, 0x0088_7970, &[v]);
    0
}

/// `0x0191` (`0x0085f5c0`): `(b, a)`: `0x00873d10(a)`, `0x00887980(b,
/// a)`; 0.
pub fn native_0191<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [a, b] = pops::<2>(h, s);
    call(h, 0x0087_3d10, &[a]);
    call(h, 0x0088_7980, &[b, a]);
    0
}

/// `0x022d` (`0x0085f4c0`, start): `(mask)`: args `+0` = `mask`; `mask`.
pub fn native_022d<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    let m = pop(h, s);
    h.write(a, m);
    m
}

/// `0x022d` (`0x0085f600`, poll): done (5) once a button of args `+0` is
/// down in `AtelGetControler()`'s buttons through `Pad_MapButtons`.
pub fn native_022d_poll<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    let m = h.read::<u32>(a);
    if m & mapped(h, 0x0086_af70) != 0 {
        5
    } else {
        0
    }
}

/// `0x000c` (`0x0085f640`): `(i)`: `Sg_SetPlayer(i)` (`0x0082dad0`); `i`.
pub fn native_000c<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    same(h, s, 0x0082_dad0)
}

/// `0x0183` (`0x0085f690`, float): `(a f)`: `Script_StopTurn(worker, -1)`
/// (`0x00863fe0`), position block `+0x38` = `Math_WrapAngle(a)`,
/// `AtelSetMoveHokanMode(worker, 0)` (`0x008703b0`), and
/// `AtelSetBindObject(worker)` (`0x0086f4c0`) for types 4 and 5; that
/// angle.
pub fn native_0183<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let p = atel_get_pos_work_adrs(h, w);
    call(h, 0x0086_3fe0, &[w, u32::MAX]);
    let v = popf(h, s);
    let r = x87::store(callf(h, 0x0088_bc20, &[v]));
    h.write(p + 0x38, r);
    call(h, 0x0087_03b0, &[w, 0]);
    let t = h.read::<u8>(w + 0xaa);
    if t != 1 && t.wrapping_sub(4) <= 1 {
        call(h, 0x0086_f4c0, &[w]);
    }
    x87::load(r)
}

/// `0x01df` (`0x0085f8d0`): `(i)`: worker `i`'s `+0x38` gets bit 3 when
/// it is a character (`+0xaa` 1); 1.
pub fn native_01df<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    let b = atel_get_basic_work(h, i.cast_signed());
    if h.read::<u8>(b + 0xaa) == 1 {
        let v = h.read::<u16>(b + 0x38) | 8;
        h.write(b + 0x38, v);
    }
    1
}

/// `0x013a` (`0x0085fb30`, start): `0x00860b00(worker, args, stack, 8)`'s
/// `EAX` (the choice window open, as `0x0139`'s start with 8).
pub fn native_013a<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    call(h, 0x0086_0b00, &[w, a, s, 8])
}

/// `0x013a` (`0x0085fcd0`, poll): jumps to `Native_MsgSelect_Poll`
/// (`0x00859c50`).
pub fn native_013a_poll<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0085_9c50)
}

/// `0x013a` (`0x0085fd40`, int, like `0x0139`'s):
/// `Native_MsgSelect_Result(worker, args, stack)` (`0x0085a6f0`), args
/// `+0` pushed and popped back -> `TOGetTable_01868a90(it)`
/// (`0x008b7c40`).
pub fn native_013a_result<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    call(h, 0x0085_a6f0, &[w, a, s]);
    let v = h.read::<u32>(a);
    push(h, s, v);
    let i = pop(h, s);
    call(h, 0x008b_7c40, &[i])
}

/// `0x0125` (`0x0085fc50`): `(b f, a f)`: `0x00870970(worker, b, a, 1)`;
/// 0.
pub fn native_0125<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let [a, b] = popfs::<2>(h, s);
    call(h, 0x0087_0970, &[w, b, a, 1]);
    0
}

/// `(&v, n)`: `f(&p, variable address, n as f32)`, then
/// `AtelSetPos3d(worker, p, 0)` (`0x00870b20`); 1.
fn pos_from<H: NativeHost>(h: &mut H, w: u32, s: u32, f: u32) -> u32 {
    let n = pop(h, s);
    #[allow(clippy::cast_precision_loss)] // fild then fstp m32
    let nf = x87::store(f64::from(n.cast_signed()));
    let v = pop(h, s);
    let pv = atel_get_real_var_adrs(h, w, v);
    let out = h.scratch();
    call(h, f, &[out, pv, nf]);
    let [x, y, z] = [0, 4, 8].map(|k| h.read::<u32>(out + k));
    call(h, 0x0087_0b20, &[w, x, y, z, 0]);
    1
}

/// `0x0099` (`0x0085fd80`): `(&points, n)`: `0x00862b20(&p, points, n)`,
/// then `AtelSetPos3d(worker, p, 0)`; 1.
pub fn native_0099<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    pos_from(h, w, s, 0x0086_2b20)
}

/// `0x009c` (`0x00860000`): `(&points, n)`: `0x00862b70(&p, points, n)`,
/// then `AtelSetPos3d(worker, p, 0)`; 1.
pub fn native_009c<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    pos_from(h, w, s, 0x0086_2b70)
}

/// `0x0233` (`0x0085fe10`): `(v)`: `TkMn2SetMainFrameRatioDiff(v)`
/// (`0x008ab260`); `v`.
pub fn native_0233<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    same(h, s, 0x008a_b260)
}

/// `0x0014` (`0x00860120`): `(b f, a f)`: `0x0086f750(worker, b, a)`; 0.
pub fn native_0014<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let [a, b] = popfs::<2>(h, s);
    call(h, 0x0086_f750, &[w, b, a]);
    0
}

/// `0x00ff` (`0x008602c0`): `(id, pan)`: `SndSepPanDirect(id, pan)`
/// (`0x00886df0`); `pan`.
pub fn native_00ff<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [p, id] = pops::<2>(h, s);
    call(h, 0x0088_6df0, &[id, p]);
    p
}

/// `0x003b` (`0x008604a0`): `(v)`: `0x0086f940(worker, v)`; 0.
pub fn native_003b<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0086_f940, &[w, v]);
    0
}

// ---- group 6: the camera and screen ----

/// The worker's camera handle (`native_cam::cam`: its `+0x3c` slot, filled
/// on first use through `MsAtelGetCameraID`).
fn cam(h: &mut impl NativeHost, w: u32) -> u32 {
    super::native_cam::cam(h, w)
}

/// `f(worker, args, stack, k...)`; 0.
fn fwd0<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32, f: u32, k: &[u32]) -> u32 {
    let mut args = [w, a, s, 0, 0];
    let n = 3 + k.len().min(2);
    args[3..n].copy_from_slice(&k[..n - 3]);
    call(h, f, &args[..n]);
    0
}

/// `0x6009` (`0x007b93e0`): `0x007bab90(worker, args, stack, 1)`; 0.
pub fn native_6009<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    fwd0(h, w, a, s, 0x007b_ab90, &[1])
}

/// `0x600b` (`0x007b9480`): `0x007bb2b0(worker, args, stack, 1, 1)`; 0.
pub fn native_600b<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    fwd0(h, w, a, s, 0x007b_b2b0, &[1, 1])
}

/// `0x600c` (`0x007b94c0`): `MsAtelCameraRefAddCircleArea_007bb370(worker,
/// args, stack, 1, 0)`; 0.
pub fn native_600c<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    fwd0(h, w, a, s, 0x007b_b370, &[1, 0])
}

/// `0x600d` (`0x007b9500`): `MsAtelCameraRefAddCircleArea_007bb370(worker,
/// args, stack, 1, 1)`; 0.
pub fn native_600d<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    fwd0(h, w, a, s, 0x007b_b370, &[1, 1])
}

/// `(on)`: `0x007bf1a0(cam, side, on & 1)`; the camera handle.
fn cam_bit<H: NativeHost>(h: &mut H, w: u32, s: u32, side: u32) -> u32 {
    let c = cam(h, w);
    let v = pop(h, s);
    call(h, 0x007b_f1a0, &[c, side, v & 1]);
    c
}

/// `0x600e` (`0x007b95a0`): `(on)`: `0x007bf1a0(cam, 1, on & 1)`; the
/// camera handle.
pub fn native_600e<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    cam_bit(h, w, s, 1)
}

/// `0x602c` (`0x007ba350`): `(on)`: `0x007bf1a0(cam, 0, on & 1)`; the
/// camera handle.
pub fn native_602c<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    cam_bit(h, w, s, 0)
}

/// `0x6018` (`0x007b9a80`): `0x007bb210(worker, args, stack, 1)`; 0.
pub fn native_6018<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    fwd0(h, w, a, s, 0x007b_b210, &[1])
}

/// `0x6036` (`0x007b8190`): `0x007bb210(worker, args, stack, 0)`; 0.
pub fn native_6036<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    fwd0(h, w, a, s, 0x007b_b210, &[0])
}

/// `0x6022` (`0x007b9e10`): `0x007bb5d0(worker, args, stack, 0, 0)`; 0.
pub fn native_6022<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    fwd0(h, w, a, s, 0x007b_b5d0, &[0, 0])
}

/// `0x6023` (`0x007b9e90`): `0x007bb5d0(worker, args, stack, 0, -1)`; 0.
pub fn native_6023<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    fwd0(h, w, a, s, 0x007b_b5d0, &[0, u32::MAX])
}

/// `0x6024` (`0x007b9f20`): `MsAtelCameraEyeAddArea_007bb410(worker,
/// args, stack, 0, -1)`; 0.
pub fn native_6024<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    fwd0(h, w, a, s, 0x007b_b410, &[0, u32::MAX])
}

/// `0x6025` (`0x007b9fa0`): `MsAtelCameraEyeAddArea_007bb410(worker,
/// args, stack, 0, 0)`; 0.
pub fn native_6025<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    fwd0(h, w, a, s, 0x007b_b410, &[0, 0])
}

/// `0x6026` (`0x007b9fc0`): `MsAtelCameraEyeAddArea_007bb410(worker,
/// args, stack, 0, 1)`; 0.
pub fn native_6026<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    fwd0(h, w, a, s, 0x007b_b410, &[0, 1])
}

/// `(frames)`: `MsCameraMoveFrame(cam, side, mode, frames, ty)`
/// (`0x007bde50`); 0.
fn cam_move<H: NativeHost>(h: &mut H, w: u32, s: u32, side: u32, mode: u32, ty: u32) -> u32 {
    let c = cam(h, w);
    let n = pop(h, s);
    call(h, 0x007b_de50, &[c, side, mode, n, ty]);
    0
}

/// `0x602f` (`0x007b7fe0`): `(frames)`: `MsCameraMoveFrame(cam, 0, 1,
/// frames, 1)`; 0.
pub fn native_602f<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    cam_move(h, w, s, 0, 1, 1)
}

/// `0x6031` (`0x007b80c0`): `(frames)`: `MsCameraMoveFrame(cam, 0, 1,
/// frames, 3)`; 0.
pub fn native_6031<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    cam_move(h, w, s, 0, 1, 3)
}

/// `0x6037` (`0x007b81b0`): `MsCameraResetDirVector(cam, 0)`
/// (`0x007bec90`); the camera handle.
pub fn native_6037<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let c = cam(h, w);
    call(h, 0x007b_ec90, &[c, 0]);
    c
}

/// `0x6039` (`0x007b82d0`): `Cam_IsMoving(cam, 0)` (`0x007bdd20`).
pub fn native_6039<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let c = cam(h, w);
    call(h, 0x007b_dd20, &[c, 0])
}

/// `0x603d` (`0x007ba000`): `0x007bac40(worker, args, stack, 0)`; 0.
pub fn native_603d<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    fwd0(h, w, a, s, 0x007b_ac40, &[0])
}

/// `0x603e` (`0x007b86d0`): `0x007bad10(worker, args, stack, 1)`; 0.
pub fn native_603e<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    fwd0(h, w, a, s, 0x007b_ad10, &[1])
}

/// `(id, v)`: `MsScreenMove(id, side, v, ty)` (`0x007bbdf0`); 0.
fn screen_move<H: NativeHost>(h: &mut H, s: u32, side: u32, ty: u32) -> u32 {
    let [v, id] = pops::<2>(h, s);
    call(h, 0x007b_bdf0, &[id, side, v, ty]);
    0
}

/// `0x6053` (`0x007b8c60`): `(id, v)`: `MsScreenMove(id, 0, v, 1)`; 0.
pub fn native_6053<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    screen_move(h, s, 0, 1)
}

/// `0x6054` (`0x007b8c90`): `(id, v)`: `MsScreenMove(id, 0, v, 3)`; 0.
pub fn native_6054<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    screen_move(h, s, 0, 3)
}

/// `0x6056` (`0x007b8d10`): `(id, v)`: `MsScreenMove(id, 1, v, 1)`; 0.
pub fn native_6056<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    screen_move(h, s, 1, 1)
}

/// `0x6057` (`0x007b8d40`): `(id, v)`: `MsScreenMove(id, 1, v, 3)`; 0.
pub fn native_6057<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    screen_move(h, s, 1, 3)
}

/// `0x6065` (`0x007b87d0`): `(p)`: `MsCameraPriority(cam, p)`
/// (`0x007be0a0`); the camera handle.
pub fn native_6065<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let c = cam(h, w);
    let p = pop(h, s);
    call(h, 0x007b_e0a0, &[c, p]);
    c
}

/// `0x6068` (`0x007b9460`): `MsAtelScreenShake(worker, args, stack, 2)`
/// (`0x007bb840`); 0.
pub fn native_6068<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    fwd0(h, w, a, s, 0x007b_b840, &[2])
}

/// `MsAtelCameraShake(worker, args, stack, side, kind)` (`0x007bb8c0`),
/// its `EAX`.
fn shake<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32, side: u32, kind: u32) -> u32 {
    call(h, 0x007b_b8c0, &[w, a, s, side, kind])
}

/// `0x606a` (`0x007b9090`): `MsAtelCameraShake(worker, args, stack, 1,
/// 3)`'s `EAX`.
pub fn native_606a<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    shake(h, w, a, s, 1, 3)
}

/// `0x606d` (`0x007b9170`): `MsAtelCameraShake(.., 1, 4)`'s `EAX`.
pub fn native_606d<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    shake(h, w, a, s, 1, 4)
}

/// `0x606f` (`0x007b91d0`): `MsAtelCameraShake(.., 0, 5)`'s `EAX`.
pub fn native_606f<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    shake(h, w, a, s, 0, 5)
}

/// `0x6070` (`0x007b9200`): `MsAtelCameraShake(.., 1, 5)`'s `EAX`.
pub fn native_6070<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    shake(h, w, a, s, 1, 5)
}

/// `0x6072` (`0x007b9240`): `MsAtelCameraShake(.., 0, 6)`'s `EAX`.
pub fn native_6072<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    shake(h, w, a, s, 0, 6)
}

/// `0x6073` (`0x007b9280`): `MsAtelCameraShake(.., 1, 6)`'s `EAX`.
pub fn native_6073<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    shake(h, w, a, s, 1, 6)
}

/// `0x6075` (`0x007b8b50`): `(b, a)`: `0x007bc870(b, a)`; 0.
pub fn native_6075<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [a, b] = pops::<2>(h, s);
    call(h, 0x007b_c870, &[b, a]);
    0
}

/// `0x6079` (`0x007b8dc0`): `(id)`: `MsScreenMoveReset(id, 0)`
/// (`0x007bc010`); 0.
pub fn native_6079<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let id = pop(h, s);
    call(h, 0x007b_c010, &[id, 0]);
    0
}

/// `0x607a` (`0x007b8de0`): `(id)`: `MsScreenMoveReset(id, 1)`; 0.
pub fn native_607a<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let id = pop(h, s);
    call(h, 0x007b_c010, &[id, 1]);
    0
}

/// `0x607c` (`0x007b8e00`, start, like `0x607b`'s): `(id)`:
/// `AtelGetBattleIdAdrs2(worker)` (`0x0086a990`, unused), args `+0` =
/// `id`; `id`.
pub fn native_607c<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    call(h, 0x0086_a990, &[w]);
    let id = pop(h, s);
    h.write(a, id);
    id
}

/// `0x607c` (`0x007b8e20`, poll): done (5) once `MsScreenMoveCheck(args
/// +0, 1)` (`0x007bbfc0`) is 0.
pub fn native_607c_poll<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    let id = h.read::<u32>(a);
    if call(h, 0x007b_bfc0, &[id, 1]) == 0 {
        5
    } else {
        0
    }
}

/// `0x6085` (`0x007b92a0`): `0x007ba460(worker, args, stack, 1)`; 0.
pub fn native_6085<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    fwd0(h, w, a, s, 0x007b_a460, &[1])
}

/// `0x6086` (`0x007b9d80`): `0x007ba460(worker, args, stack, 0)`; 0.
pub fn native_6086<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    fwd0(h, w, a, s, 0x007b_a460, &[0])
}

/// `0x6089` (`0x007b9630`): `(p8, p7, p6, p5, p4 f, p3, p2 f, p1)`:
/// `MsScreenShake2(p8, p7, 9, p6, p5, p4, p3, p2, p1)` (`0x007c57b0`); 0.
pub fn native_6089<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let p1 = pop(h, s);
    let p2 = popf(h, s);
    let p3 = pop(h, s);
    let p4 = popf(h, s);
    let [p5, p6, p7, p8] = pops::<4>(h, s);
    call(h, 0x007c_57b0, &[p8, p7, 9, p6, p5, p4, p3, p2, p1]);
    0
}

// ---- group b: the movie callbacks the PC port left out ----

/// `0xb005`-`0xb007`, `0xb008`'s poll and int, `0xb00e`, `0xb00f`
/// (`0x0076ead0`, `0x0076eae0`, `0x0076eaf0`, `0x0076eb10`, `0x0076eb20`,
/// `0x0076ecc0`, `0x0076ecd0`): a "Virtuos Warning: Movie on Windows, PS3
/// & PS Vita is not impelemented yet!" line naming the PS2 callback
/// through the empty `dbgPrintf`; 0 (the poll keeps waiting).
pub fn native_b005<H: NativeHost>(_h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    0
}

/// `0xb008` (`0x0076eb00`, start): the same warning for
/// `movie_start_debug_init`; `dbgPrintf` leaves `EAX` as it was, which the
/// dispatch ignores (0 here).
pub fn native_b008<H: NativeHost>(_h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    0
}
// ---- group 7: battle ----

/// The worker's battle actor index (`AtelGetBattleIdAdrs2`, `0xff` for
/// none).
fn bactor(h: &impl NativeHost, w: u32) -> u32 {
    u32::from(h.read::<u8>(super::atel_get_battle_id_adrs2(w)))
}

/// `MsGetChr(i)` (`0x00794020`).
fn bchr(h: &mut impl NativeHost, i: u32) -> u32 {
    call(h, 0x0079_4020, &[i])
}

/// `Battle_FirstSelectedActor(actor, selector, 0, 1)` (`0x007947f0`).
fn first(h: &mut impl NativeHost, actor: u32, sel: u32) -> u32 {
    call(h, 0x0079_47f0, &[actor, sel, 0, 1])
}

/// The byte at the global `sym`, sign-extended.
fn gi8(h: &impl NativeHost, sym: u32) -> u32 {
    i32::from(h.read::<i8>(h.rebase(sym))).cast_unsigned()
}

/// The byte at the global `sym` = `v`.
fn set_g8(h: &mut impl NativeHost, sym: u32, v: u8) {
    let at = h.rebase(sym);
    h.write(at, v);
}

/// `0x7074` (`0x007a29f0`): `(v)` -> `MsDetachWeapon(actor, v)`
/// (`0x00793f10`).
pub fn native_7074<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let i = bactor(h, w);
    call(h, 0x0079_3f10, &[i, v])
}

/// `0x70f4` (`0x007a2ad0`): `(b, a)` -> `0x007a8b40(actor, b, a)`.
pub fn native_70f4<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let i = bactor(h, w);
    let [a, b] = pops::<2>(h, s);
    call(h, 0x007a_8b40, &[i, b, a])
}

/// `0x70e4` (`0x007a2b50`): `(b, a)` -> `MsGetDamageLimit(first of b,
/// first of a)` (`0x0079a860`, each through `Battle_FirstSelectedActor`).
pub fn native_70e4<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let [a, b] = pops::<2>(h, s);
    let i = bactor(h, w);
    let x = first(h, i, b);
    let y = first(h, i, a);
    call(h, 0x0079_a860, &[x, y])
}

/// `0x7080` (`0x007a2c70`): `(v)`: `0x008720e0(handle, v)` with
/// `MsGetChrHandle(2)` (`0x00795080`) when there is one; 0.
pub fn native_7080<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let c = call(h, 0x0079_5080, &[2]);
    if c != 0 {
        call(h, 0x0087_20e0, &[c, v]);
    }
    0
}

/// `0x70ad` (`0x007a2cd0`): jumps to `0x007ad6e0`.
pub fn native_70ad<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x007a_d6e0)
}

/// `0x7098` (`0x007a2dc0`): `MsCheckDefenseMotion(MsGetChr(actor))`
/// (`0x007af2f0`), 0 without an actor.
pub fn native_7098<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let i = bactor(h, w);
    if i == 0xff {
        return 0;
    }
    let c = bchr(h, i);
    call(h, 0x007a_f2f0, &[c])
}

/// `0x70f3` (`0x007a3300`): `(b, a)` -> `0x007a8ad0(b, a)`.
pub fn native_70f3<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [a, b] = pops::<2>(h, s);
    call(h, 0x007a_8ad0, &[b, a])
}

/// `0x70fc` (`0x007a3440`): the `u16` at `0x01136410`.
pub fn native_70fc<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    u32::from(h.read::<u16>(h.rebase(0x0113_6410)))
}

/// `0x70db` (`0x007a3450`): the `i8` at `0x0112ca06`.
pub fn native_70db<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    gi8(h, 0x0112_ca06)
}

/// `0x70f6` (`0x007a34e0`): the bytes `0x011363ec` and `0x011363ed` = 0
/// (the list `0x70f8` reads, emptied); 0.
pub fn native_70f6<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    set_g8(h, 0x0113_63ec, 0);
    set_g8(h, 0x0113_63ed, 0);
    0
}

/// `0x70d9` (`0x007a3500`): `Ch_ModelSetHide(model, 1, the i8 at
/// 0x0112ca06)` (`0x0082aea0`) for actor 13's model (`MsGetChr(0xd)`
/// `+0`) when it has one; that byte, read again.
pub fn native_70d9<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    let c = bchr(h, 0xd);
    let m = h.read::<u32>(c);
    let v = gi8(h, 0x0112_ca06);
    if m == 0 {
        return v;
    }
    call(h, 0x0082_aea0, &[m, 1, v]);
    gi8(h, 0x0112_ca06)
}

/// `0x70f8` (`0x007a36e0`): the next `u16` of the list at `0x011363f0`
/// (read index `0x011363ed`, count `0x011363ec`, the index advanced), 0
/// past the end.
pub fn native_70f8<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    let (pi, pn) = (h.rebase(0x0113_63ed), h.rebase(0x0113_63ec));
    let i = h.read::<u8>(pi);
    if i >= h.read::<u8>(pn) {
        return 0;
    }
    let v = h.read::<u16>(h.rebase(0x0113_63f0) + 2 * u32::from(i));
    h.write(pi, i.wrapping_add(1));
    u32::from(v)
}

/// `0x70a3` (`0x007a3730`): `Battle_ScriptPlayMotion(worker, args, stack,
/// 0)` (`0x007a8fc0`) with the actor's `+0x38` byte set to 1 for the call
/// and put back; `+0x3f3` = 1 when `+0x434` is set; the old byte
/// (sign-extended).
pub fn native_70a3<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    let i = bactor(h, w);
    let c = bchr(h, i);
    let old = h.read::<i8>(c + 0x38);
    h.write(c + 0x38, 1_u8);
    call(h, 0x007a_8fc0, &[w, a, s, 0]);
    let busy = h.read::<u8>(c + 0x434);
    h.write(c + 0x38, old);
    if busy != 0 {
        h.write(c + 0x3f3, 1_u8);
    }
    i32::from(old).cast_unsigned()
}

/// `0x70fb` (`0x007a37e0`): `(p5, p4, p3, p2, p1)` -> `0x007a8f60(actor,
/// p5, -16, p3, 0, p2)` when `0x007a8b40(actor, p5, p4)` and
/// `0x007a8ad0(p5, p1)` both hold, else (or for 0xff) 0xff.
pub fn native_70fb<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let i = bactor(h, w);
    let [p1, p2, p3, p4, p5] = pops::<5>(h, s);
    if call(h, 0x007a_8b40, &[i, p5, p4]) == 0 || call(h, 0x007a_8ad0, &[p5, p1]) == 0 {
        return 0xff;
    }
    call(
        h,
        0x007a_8f60,
        &[i, p5, (-16_i32).cast_unsigned(), p3, 0, p2],
    )
}

/// `0x70c6` (`0x007a3950`): `(v)` -> `0x007b7ed0(v)`.
pub fn native_70c6<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    one(h, s, 0x007b_7ed0)
}

/// `0x710a` (`0x007a3970`): the `i8` at `0x0112ca10`, which becomes 1.
pub fn native_710a<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    let v = gi8(h, 0x0112_ca10);
    set_g8(h, 0x0112_ca10, 1);
    v
}

/// `Native_BtlSetFlagBded` (`0x007a39d0`, `0x7001`): `(v)`: the byte
/// `0x0112bded` = `v & 1`; 0.
pub fn native_7001<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    #[allow(clippy::cast_possible_truncation)] // `and al, 1`
    set_g8(h, 0x0112_bded, (v & 1) as u8);
    0
}

/// `0x70c7` (`0x007a39f0`, float): `(v)` -> `0x007b7ef0(v)`'s `ST0`.
pub fn native_70c7<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    let v = pop(h, s);
    callf(h, 0x007b_7ef0, &[v])
}

/// `Atel_GetBattleCountF` (`0x007a3ac0`, `0x70f0`, float): the `i32` at
/// `0x0112c900` as `f32` (`helpers::atel_get_battle_count_f`, whose shim
/// it is).
pub fn native_70f0<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> f64 {
    super::helpers::atel_get_battle_count_f(h)
}

/// The first of the four `0x007b19d0(k)` records whose byte `+0` is set:
/// its byte `+off`; 0 for none.
fn first_record(h: &mut impl NativeHost, off: u32) -> u32 {
    for k in 0..4 {
        let p = call(h, 0x007b_19d0, &[k]);
        if h.read::<u8>(p) != 0 {
            return u32::from(h.read::<u8>(p + off));
        }
    }
    0
}

/// `0x710e` (`0x007a3c00`): the first set byte `+0` of the four
/// `0x007b19d0(k)` records, 0 for none.
pub fn native_710e<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    first_record(h, 0)
}

/// `0x710f` (`0x007a3c90`): that record's byte `+1`, 0 for none.
pub fn native_710f<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    first_record(h, 1)
}

/// `0x711e` (`0x007a3f40`): the `i8` at `0x0112ca15`.
pub fn native_711e<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    gi8(h, 0x0112_ca15)
}

/// `Native_BtlIsUnderwater` (`0x007a4550`, `0x7005`): the `i8` at
/// `0x0112c9ce`.
pub fn native_7005<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    gi8(h, 0x0112_c9ce)
}

/// `0x7090` (`0x007a49c0`): `(v)`: `MsEtCriticalEffect(actor, v)`
/// (`0x0079e590`); 0.
pub fn native_7090<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let i = bactor(h, w);
    call(h, 0x0079_e590, &[i, v]);
    0
}

/// `(c, b, a)`: the first actor of selector `c`.
fn pop3_first<H: NativeHost>(h: &mut H, w: u32, s: u32) -> (u32, u32, u32) {
    let [a, b, c] = pops::<3>(h, s);
    let i = bactor(h, w);
    (first(h, i, c), b, a)
}

/// `0x70d3` (`0x007a4a90`): `(sel, b, a)`: `MsSetChrEffect_0079eb10(x, b,
/// 7, a, 1)` for the first actor `x` of `sel`, whose `+0xe12` counts up
/// when it starts; 0.
pub fn native_70d3<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let (x, b, a) = pop3_first(h, w, s);
    if call(h, 0x0079_eb10, &[x, b, 7, a, 1]) != 0 {
        let c = bchr(h, x);
        let n = h.read::<u8>(c + 0xe12).wrapping_add(1);
        h.write(c + 0xe12, n);
    }
    0
}

/// `0x70e5` (`0x007a4c70`): `(sel, b, a)`: `0x0079eca0(first of sel, b,
/// a)`; 0.
pub fn native_70e5<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let (x, b, a) = pop3_first(h, w, s);
    call(h, 0x0079_eca0, &[x, b, a]);
    0
}

/// `0x70e6` (`0x007a4e00`): `(sel, b, a)`: `0x0079ec40(first of sel, b,
/// a)`; 0.
pub fn native_70e6<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let (x, b, a) = pop3_first(h, w, s);
    call(h, 0x0079_ec40, &[x, b, a]);
    0
}

/// `0x70d4` (`0x007a4f20`): `(sel, b, a)` -> `0x0079eba0(first of sel, b,
/// a, 1)`.
pub fn native_70d4<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let (x, b, a) = pop3_first(h, w, s);
    call(h, 0x0079_eba0, &[x, b, a, 1])
}

/// `0x7045` (`0x007a4da0`): `(sel)` -> `MsSetMove(first of sel, 0xc)`
/// (`0x007a9fc0`, `ST0`) over the double at `0x00b43d70`, rounded to
/// `f32`, truncated.
pub fn native_7045<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let i = bactor(h, w);
    let v = pop(h, s);
    let x = first(h, i, v);
    let r = callf(h, 0x007a_9fc0, &[x, 0xc]);
    let k = f64::from_bits(h.read(h.rebase(0x00b4_3d70)));
    let q = r32(x87::div(r, k));
    x87::ftol(h, q)
}

/// `0x70da` (`0x007a4df0`): the `i8` at `0x0112c9dc`.
pub fn native_70da<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    gi8(h, 0x0112_c9dc)
}

/// `0x7127` (`0x007a5240`): `(b, a)` -> `0x007969a0(b, a)`.
pub fn native_7127<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [a, b] = pops::<2>(h, s);
    call(h, 0x0079_69a0, &[b, a])
}

/// `0x70b6` (`0x007a52b0`): `(sel)` -> `MsSetMove(first of sel, 0xc)` over
/// `0x007a88e0(the actor's record)` (both `ST0`, the divisor rounded to
/// `f32`), rounded to `f32`, truncated.
pub fn native_70b6<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let i = bactor(h, w);
    let c = bchr(h, i);
    let v = pop(h, s);
    let x = first(h, i, v);
    let d = r32(callf(h, 0x007a_88e0, &[c]));
    let r = callf(h, 0x007a_9fc0, &[x, 0xc]);
    let q = r32(x87::div(r, d));
    x87::ftol(h, q)
}

/// `0x7077` (`0x007a5450`): `(sel, a)`: `MsGetTargetDist(x, a, 0)`
/// (`0x007a9a40`, 0.0 when the first actor `x` of `sel` is 0xff) over the
/// double at `0x00b49968` plus the one at `0x00b43d80`, rounded to `f32`,
/// at most the `f32` at `0x00b4ae10`, truncated.
pub fn native_7077<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let [a, b] = pops::<2>(h, s);
    let i = bactor(h, w);
    let x = first(h, i, b);
    let t = if x == 0xff {
        0.0
    } else {
        r32(callf(h, 0x007a_9a40, &[x, a, 0]))
    };
    let k1 = f64::from_bits(h.read(h.rebase(0x00b4_9968)));
    let k2 = f64::from_bits(h.read(h.rebase(0x00b4_3d80)));
    let v = r32(x87::add(x87::div(t, k1), k2));
    let cap = x87::load(h.read(h.rebase(0x00b4_ae10)));
    // fcom: only a larger value (not NaN) takes the cap
    let v = if v > cap { cap } else { v };
    x87::ftol(h, v)
}

/// `0x70a7` (`0x007a54f0`, float): `(sel, a)` -> `0x007b2c90(first of sel,
/// a)`'s `ST0`.
pub fn native_70a7<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let [a, b] = pops::<2>(h, s);
    let i = bactor(h, w);
    let x = first(h, i, b);
    callf(h, 0x007b_2c90, &[x, a])
}

/// `0x700e` (`0x007a5690`): `(sel)`: `MsSetDamage(actor, sel, 0)`
/// (`0x0078d980`) for an actor; 0.
pub fn native_700e<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let i = bactor(h, w);
    if i != 0xff {
        call(h, 0x0078_d980, &[i, v, 0]);
    }
    0
}

/// `0x703d` (`0x007a5980`): the byte `0x0112a8e4` = 0; 0.
pub fn native_703d<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    set_g8(h, 0x0112_a8e4, 0);
    0
}

/// `0x703e` (`0x007a5a10`, poll): with `0x0112a8e4` set, `0x0112a8e5`
/// drops to 0 when it is 1. Unless the `i8` at `0x0112c9e4` is negative
/// and `0x0112c9d0`, `0x0112c9d8`, `0x0112bde1`, `0x0112c944` are all 0,
/// the pair at `0x0112ca08` = (0xff, 0xde) and it waits (0). Else the
/// first of the 31 actors that is busy (`+0xdc8` with `+0x508`, `+0x432`
/// or `+0xdfb`: kind 0xdf) or acting (`+0x10` with `+0x436` or `+0xdf0`:
/// 0xdd) goes to `0x0112ca08` with its kind and it waits; none: done (5).
pub fn native_703e_poll<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    let g = |h: &H, sym: u32| h.read::<u8>(h.rebase(sym));
    if g(h, 0x0112_a8e4) != 0 {
        let v = g(h, 0x0112_a8e5);
        set_g8(h, 0x0112_a8e5, if v == 1 { 0 } else { v });
    }
    if h.read::<i8>(h.rebase(0x0112_c9e4)) >= 0
        || [0x0112_c9d0, 0x0112_c9d8, 0x0112_bde1, 0x0112_c944]
            .into_iter()
            .any(|x| g(h, x) != 0)
    {
        let at = h.rebase(0x0112_ca08);
        h.write(at, 0xdeff_u16);
        return 0;
    }
    for k in 0..0x1f_u8 {
        let c = bchr(h, u32::from(k));
        let b = |h: &H, off: u32| h.read::<u8>(c + off) != 0;
        let kind = if b(h, 0xdc8) && (b(h, 0x508) || b(h, 0x432) || b(h, 0xdfb)) {
            0xdf
        } else if b(h, 0x10) && (b(h, 0x436) || b(h, 0xdf0)) {
            0xdd
        } else {
            continue;
        };
        set_g8(h, 0x0112_ca08, k);
        set_g8(h, 0x0112_ca09, kind);
        return 0;
    }
    5
}

/// `0x7081` (`0x007a5d60`, poll): done (5) when the byte `0x0112c022` is
/// 1, else 0.
pub fn native_7081_poll<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    if h.read::<u8>(h.rebase(0x0112_c022)) == 1 {
        5
    } else {
        0
    }
}

/// `0x70f9` (`0x007a5fc0`): `(p5, p4, p3, p2, p1)` -> `0x007a8f60(actor,
/// p5, p4, p3, p2, p1)`.
pub fn native_70f9<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let [p1, p2, p3, p4, p5] = pops::<5>(h, s);
    let i = bactor(h, w);
    call(h, 0x007a_8f60, &[i, p5, p4, p3, p2, p1])
}

/// `Clamp0To6_007a86e0(v)`.
fn win6(h: &mut impl NativeHost, v: u32) -> u32 {
    call(h, 0x007a_86e0, &[v])
}

/// Battle text `k` of the table at `tab` (`+0` holds `u16` offsets, two
/// per entry by `MsGetSaveConfigHiragana()` (`0x007851f0`)): its address.
fn btext(h: &mut impl NativeHost, tab: u32, k: u32) -> u32 {
    let hira = call(h, 0x0078_51f0, &[]);
    let off = h.read::<u16>(tab + hira.wrapping_add(k.wrapping_mul(2)).wrapping_mul(4));
    tab + u32::from(off)
}

/// `0x7042` (`0x007a6060`): `(win, k, x, y, anchor)`: `0x0112c250` = `k`;
/// `TODrawAMES(win clamped 0..6, text k, x, y, anchor)` (`0x0089b140`)
/// from the table at `*0x0112a9b8` when there is one; 0.
pub fn native_7042<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [p1, p2, p3, k, wv] = pops::<5>(h, s);
    let win = win6(h, wv);
    let tab = h.read::<u32>(h.rebase(0x0112_a9b8));
    let at = h.rebase(0x0112_c250);
    h.write(at, k);
    if tab != 0 {
        let t = btext(h, tab, k);
        call(h, 0x0089_b140, &[win, t, p3, p2, p1]);
    }
    0
}

/// `0x711f` (`0x007a6200`): as `0x7042` from the table at `*0x0112a998`,
/// only when `MsGetBattleMessageFlag()` (`0x00784e40`) is set and the word
/// at `0x01841d20` is not 0; 0.
pub fn native_711f<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [p1, p2, p3, k, wv] = pops::<5>(h, s);
    let win = win6(h, wv);
    let tab = h.read::<u32>(h.rebase(0x0112_a998));
    let at = h.rebase(0x0112_c250);
    h.write(at, k);
    if tab != 0 && call(h, 0x0078_4e40, &[]) != 0 {
        let t = btext(h, tab, k);
        if h.read::<u32>(h.rebase(0x0184_1d20)) != 0 {
            call(h, 0x0089_b140, &[win, t, p3, p2, p1]);
        }
    }
    0
}

/// `(win, k)`: `0x0112c250` = `k`; `f(win clamped, text k)` from the table
/// at `*tab_sym`, with the message-flag checks of `0x711f` when `flag`.
fn line<H: NativeHost>(h: &mut H, s: u32, tab_sym: u32, flag: bool, f: u32) -> u32 {
    let [k, wv] = pops::<2>(h, s);
    let win = win6(h, wv);
    let tab = h.read::<u32>(h.rebase(tab_sym));
    let at = h.rebase(0x0112_c250);
    h.write(at, k);
    if tab == 0 || (flag && call(h, 0x0078_4e40, &[]) == 0) {
        return 0;
    }
    let t = btext(h, tab, k);
    if flag && h.read::<u32>(h.rebase(0x0184_1d20)) == 0 {
        return 0;
    }
    call(h, f, &[win, t]);
    0
}

/// `0x7072` (`0x007a6330`): `(win, k)`: `TODrawAMES_EVJIMAKU(win, text
/// k)` (`0x0089b230`) from `*0x0112a9b8`; 0.
pub fn native_7072<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    line(h, s, 0x0112_a9b8, false, 0x0089_b230)
}

/// `0x70b7` (`0x007a6640`): `(win, k)`:
/// `TODrawAMES_EVJIMAKU_ALBHED(win, text k)` (`0x0089b290`) from
/// `*0x0112a9b8`; 0.
pub fn native_70b7<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    line(h, s, 0x0112_a9b8, false, 0x0089_b290)
}

/// `0x7121` (`0x007a6790`): `(win, k)`: as `0x70b7` from `*0x0112a998`
/// with `0x711f`'s checks; 0.
pub fn native_7121<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    line(h, s, 0x0112_a998, true, 0x0089_b290)
}

/// `0x7069` (`0x007a6170`, `Native_BtlFindActor`): `(p4, p3, p2, p1)` ->
/// `MsActTargetSearch(0, p4, p3, p2, 7, p1, 0, &pick)` (`0x007acd40`)'s
/// pick (an out word, zeroed first).
pub fn native_7069<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [p1, p2, p3, p4] = pops::<4>(h, s);
    let o = h.scratch();
    h.write(o, 0_u32);
    call(h, 0x007a_cd40, &[0, p4, p3, p2, 7, p1, 0, o]);
    h.read(o)
}

/// `0x7095` (`0x007a61d0`): `MsGetChrMoveFrameRest(actor)` (`0x007a9320`),
/// 0 without an actor.
pub fn native_7095<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let i = bactor(h, w);
    if i == 0xff {
        return 0;
    }
    call(h, 0x007a_9320, &[i])
}

/// `0x7120` (`0x007a65a0`): `(b, a)`: `0x0079d090(a)`, `b` dropped; 0.
pub fn native_7120<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [a, _b] = pops::<2>(h, s);
    call(h, 0x0079_d090, &[a]);
    0
}

/// `0x7065` (`0x007a6760`, poll): `GetCD()`; done (5) once `0x007a8c20()`
/// is set and `msVoiceCheckStanby(0)` (`0x007b2c10`) is 0.
pub fn native_7065_poll<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call(h, 0x0076_d0d0, &[]);
    if call(h, 0x007a_8c20, &[]) != 0 && call(h, 0x007b_2c10, &[0]) == 0 {
        5
    } else {
        0
    }
}

/// `0x7065` (`0x007a6950`, int): `SndStartStream()` (`0x00887630`); 0.
pub fn native_7065<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    void(h, 0x0088_7630, 0)
}

/// `0x7116` (`0x007a6930`): `MsEffectSetGuard(actor)` (`0x0079e510`).
pub fn native_7116<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let i = bactor(h, w);
    call(h, 0x0079_e510, &[i])
}

/// `0x7066` (`0x007a6960`): jumps to `0x007b2c00`.
pub fn native_7066<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x007b_2c00)
}

/// `0x7067` (`0x007a6a30`): `msVoiceIsStreaming() != 0` (`0x007b2be0`).
pub fn native_7067<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    u32::from(call(h, 0x007b_2be0, &[]) != 0)
}

/// `0x7068` (`0x007a6ad0`, poll): 1 once `msVoiceIsStreaming()` is 0, else
/// 0.
pub fn native_7068_poll<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    u32::from(call(h, 0x007b_2be0, &[]) == 0)
}

/// `0x706c` (`0x007a6bc0`): `(sel, k)`: for the first actor `x` of `sel`,
/// the record of its `+0x14` actor: with `+0xf2c` set and `k` at most
/// 0x1f, sound `k` (`+0xeac + 4 k`) plays (`MsSEP3Dregist(3, x, se, 0,
/// 0)`, `0x007a0a30`) unless it is 0 or the last one (`0x0112c8e0`, then
/// updated); 0.
pub fn native_706c<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let [k, sel] = pops::<2>(h, s);
    let i = bactor(h, w);
    let x = first(h, i, sel);
    if x.cast_signed() < 0 {
        return 0;
    }
    let c = bchr(h, x);
    let o = u32::from(h.read::<u8>(c + 0x14));
    let d = bchr(h, o);
    if h.read::<u8>(d + 0xf2c) == 0 || k > 0x1f {
        return 0;
    }
    let se = h.read::<u32>(d + k * 4 + 0xeac);
    let last = h.rebase(0x0112_c8e0);
    if se == 0 || h.read::<u32>(last) == se {
        return 0;
    }
    call(h, 0x007a_0a30, &[3, x, se, 0, 0]);
    h.write(last, se);
    0
}

/// `0x70c0` (`0x007a6c70`, float): `(sel)` -> `0x00794990(first of sel)`'s
/// `ST0`.
pub fn native_70c0<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let v = pop(h, s);
    let i = bactor(h, w);
    let x = first(h, i, v);
    callf(h, 0x0079_4990, &[x])
}

/// `0x7044` (`0x007a6e10`): `(win)`: `TOCloseMessageWindow(win clamped
/// 0..6)` (`0x008ab8f0`); 0.
pub fn native_7044<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let k = win6(h, v);
    call(h, 0x008a_b8f0, &[k]);
    0
}

/// `0x7056` (`0x007a6e80`): `(v)`: `MsBattleSpecialInit(v)`
/// (`0x0079de60`); 0.
pub fn native_7056<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    one0(h, s, 0x0079_de60)
}

/// `0x7070` (`0x007a7010`): `SetByte_00c58ef4(1)` (`0x0089bc60`); 0.
pub fn native_7070<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call(h, 0x0089_bc60, &[1]);
    0
}

/// `0x70b4` (`0x007a7130`): for an actor, `+0xf65` = 8,
/// `MsResetWeaponEffectAttribute(actor, record)` (`0x0079ea90`), then
/// `0x007ad5b0(actor, +0xf64, +0xf65)`; 0.
pub fn native_70b4<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let i = bactor(h, w);
    if i == 0xff {
        return 0;
    }
    let c = bchr(h, i);
    h.write(c + 0xf65, 8_u8);
    call(h, 0x0079_ea90, &[i, c]);
    let (b65, b64) = (h.read::<u8>(c + 0xf65), h.read::<u8>(c + 0xf64));
    call(h, 0x007a_d5b0, &[i, u32::from(b64), u32::from(b65)]);
    0
}

/// `0x7079` (`0x007a7200`): for an actor, the equipment refresh:
/// `MsResetAutoStatus(record)`, `MsSetRamChrParam(actor)`,
/// `MsSetChrWeak(actor, -1)`, `MsSetAutoStatus(record)`,
/// `MsStatusEffectCheck(actor)`, `MsSetWeaponEffectAttribute(actor,
/// record)`; 0.
pub fn native_7079<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let i = bactor(h, w);
    if i == 0xff {
        return 0;
    }
    let c = bchr(h, i);
    call(h, 0x0079_b1a0, &[c]);
    call(h, 0x0079_c5f0, &[i]);
    call(h, 0x0078_d7f0, &[i, u32::MAX]);
    call(h, 0x0079_b290, &[c]);
    call(h, 0x0079_eff0, &[i]);
    call(h, 0x0079_ed40, &[i, c]);
    0
}

/// `0x7094` (`0x007a72c0`): `(b f, a f, v)`:
/// `MsSetChrTransparentVisible(actor, b, a, v, record +0x423)`
/// (`0x00796550`) for an actor; 0.
pub fn native_7094<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let i = bactor(h, w);
    let v = pop(h, s);
    let [a, b] = popfs::<2>(h, s);
    if i != 0xff {
        let c = bchr(h, i);
        let k = u32::from(h.read::<u8>(c + 0x423));
        call(h, 0x0079_6550, &[i, b, a, v, k]);
    }
    0
}

/// `Native_BtlSetPadOrientation` (`0x007a7340`, `0x7099`): `(v)`: the byte
/// `0x0112c9cd` = `v & 3`; that value.
pub fn native_7099<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s) & 3;
    #[allow(clippy::cast_possible_truncation)] // two bits
    set_g8(h, 0x0112_c9cd, v as u8);
    v
}

/// `0x704d` (`0x007a74e0`): 0 (`xor eax, eax; ret`).
pub fn native_704d<H: NativeHost>(_h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    0
}

/// `(sel, a)` -> `f(first of sel, a)`, 0 when there is none (negative).
fn sel_call<H: NativeHost>(h: &mut H, w: u32, s: u32, f: u32) -> u32 {
    let [a, sel] = pops::<2>(h, s);
    let i = bactor(h, w);
    let x = first(h, i, sel);
    if x.cast_signed() < 0 {
        return 0;
    }
    call(h, f, &[x, a])
}

/// `0x70c8` (`0x007a75e0`): `(sel, a)` -> `0x0079fda0(first of sel, a)`,
/// 0 for none.
pub fn native_70c8<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    sel_call(h, w, s, 0x0079_fda0)
}

/// `0x70ca` (`0x007a7ab0`): `(sel, se)` -> `MsRegSEplay2(first of sel,
/// se)` (`0x007a0140`), 0 for none.
pub fn native_70ca<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    sel_call(h, w, s, 0x007a_0140)
}

/// `0x70cb` (`0x007a7b40`): `(sel, se)` -> `MsRegSEplay3(first of sel,
/// se)` (`0x007a0170`), 0 for none.
pub fn native_70cb<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    sel_call(h, w, s, 0x007a_0170)
}

/// `Native_BtlGetTurnActor` (`0x007a76e0`, `0x708d`): the `i8` at
/// `0x0112c9e2`.
pub fn native_708d<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    gi8(h, 0x0112_c9e2)
}

/// `0x70ae` (`0x007a7790`): the `i8` at `0x0112c9e3`.
pub fn native_70ae<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    gi8(h, 0x0112_c9e3)
}

/// `0x70af` (`0x007a79a0`): jumps to `MsGetVoiceEventFlag` (`0x007986a0`).
pub fn native_70af<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0079_86a0)
}

/// `0x70de` (`0x007a79b0`): jumps to `0x00785210`.
pub fn native_70de<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0078_5210)
}

/// `0x70e8` (`0x007a7a90`): jumps to `MsGetGIL` (`0x00784e80`).
pub fn native_70e8<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0078_4e80)
}

/// `0x70e9` (`0x007a7b00`): jumps to `0x007b2b30`.
pub fn native_70e9<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x007b_2b30)
}

/// `0x70c5` (`0x007a7c20`): 1 when the byte `0x0112c9f5` is set.
pub fn native_70c5<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    u32::from(h.read::<u8>(h.rebase(0x0112_c9f5)) != 0)
}

/// `0x7089` (`0x007a7c40`): `(sel, se, frames)` -> `MsRegSEfade(first of
/// sel, se, frames)` (`0x007a00d0`), 0 for none.
pub fn native_7089<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let [n, se, sel] = pops::<3>(h, s);
    let i = bactor(h, w);
    let x = first(h, i, sel);
    if x.cast_signed() < 0 {
        return 0;
    }
    call(h, 0x007a_00d0, &[x, se, n])
}

/// `0x70eb` (`0x007a7d10`): `brnd(0x11)` (`0x007988f0`).
pub fn native_70eb<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call(h, 0x0079_88f0, &[0x11])
}

/// `0x7100` (`0x007a7d20`): `(sel)` -> `0x007a8be0(MsGetChrBit(actor, sel,
/// 0, 1))` (`0x00794330`).
pub fn native_7100<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let i = bactor(h, w);
    let v = pop(h, s);
    let m = call(h, 0x0079_4330, &[i, v, 0, 1]);
    call(h, 0x007a_8be0, &[m])
}

/// `0x7124` (`0x007a7ee0`): `(id)`: `0x0112c250` = `id`;
/// `MsSetVoiceId(id)` (`0x007b2b50`) when `MsGetBattleMessageFlag()`; `id`.
pub fn native_7124<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let at = h.rebase(0x0112_c250);
    h.write(at, v);
    if call(h, 0x0078_4e40, &[]) != 0 {
        call(h, 0x007b_2b50, &[v]);
    }
    v
}

/// `0x7101` (`0x007a7f60`): the `i8` at `0x0112a905`.
pub fn native_7101<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    gi8(h, 0x0112_a905)
}

/// `0x70cf` (`0x007a7f70`): the `i8` at `*0x0112a9c0 + 1`.
pub fn native_70cf<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    let p = h.read::<u32>(h.rebase(0x0112_a9c0));
    i32::from(h.read::<i8>(p + 1)).cast_unsigned()
}

/// `0x7113` (`0x007a8090`): the byte `0x0112c9e5` = 1; 0.
pub fn native_7113<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    set_g8(h, 0x0112_c9e5, 1);
    0
}

/// `0x7114` (`0x007a80a0`): the byte `0x0112c9e5` = 2; 0.
pub fn native_7114<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    set_g8(h, 0x0112_c9e5, 2);
    0
}

// ---- groups 8, 9, c, d: map parts, system, the rest ----

/// `0x8069` (`0x0091af30`): `(b, a)`: `pppFpSetStickMapDObj(a, b)`
/// (`0x0072a5e0`); 1.
pub fn native_8069<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [a, b] = pops::<2>(h, s);
    call(h, 0x0072_a5e0, &[a, b]);
    1
}

/// `0x800e` (`0x0091b190`): jumps to `Yn_MapPartCount` (`0x0091a3d0`).
pub fn native_800e<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0091_a3d0)
}

/// `0x8024` (`0x0091b530`, float): jumps to `Yn_SceneGetB8`
/// (`0x0091a700`).
pub fn native_8024<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> f64 {
    tailf(h, w, a, s, 0x0091_a700)
}

/// `0x8025` (`0x0091b540`, float): jumps to `Yn_SceneGetBC`
/// (`0x0091a730`).
pub fn native_8025<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> f64 {
    tailf(h, w, a, s, 0x0091_a730)
}

/// `0x8029` (`0x0091b600`, float): jumps to `Yn_SceneGetAC`
/// (`0x0091a6d0`).
pub fn native_8029<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> f64 {
    tailf(h, w, a, s, 0x0091_a6d0)
}

/// `0x8033` (`0x0091b770`, float): jumps to `Yn_GuideMapGetLimit`
/// (`0x00919ca0`).
pub fn native_8033<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> f64 {
    tailf(h, w, a, s, 0x0091_9ca0)
}

/// `0x8027` (`0x0091b5c0`): `0x0091a900()`; 1.
pub fn native_8027<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    void(h, 0x0091_a900, 1)
}

/// `0x802b` (`0x0091b670`): `0x0091a8d0()`; 1.
pub fn native_802b<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    void(h, 0x0091_a8d0, 1)
}

/// `0x8034` (`0x0091b780`): `0x00919b70()`; 1.
pub fn native_8034<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    void(h, 0x0091_9b70, 1)
}

/// `0x8062` (`0x0091c5a0`): `0x00908440()`; 1.
pub fn native_8062<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    void(h, 0x0090_8440, 1)
}

/// `0x8068` (`0x0091c890`): `Yn_FPm173End()` (`0x00919ab0`); 1.
pub fn native_8068<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    void(h, 0x0091_9ab0, 1)
}

/// `(part, v f)`: `Yn_MapPartSet44_0091abc0(part, x, y)` (`0x0091abc0`)
/// with `(x, y)` from `v` by `pick`; 1.
fn part44<H: NativeHost>(h: &mut H, s: u32, pick: fn(u32) -> (u32, u32)) -> u32 {
    let v = popf(h, s);
    let part = pop(h, s);
    let (x, y) = pick(v);
    call(h, 0x0091_abc0, &[part, x, y]);
    1
}

/// `0x804c` (`0x0091c090`): `(part, v f)`:
/// `Yn_MapPartSet44_0091abc0(part, v, v)`; 1.
pub fn native_804c<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    part44(h, s, |v| (v, v))
}

/// `0x804d` (`0x0091c0d0`): `(part, v f)`:
/// `Yn_MapPartSet44_0091abc0(part, v, 0.0)`; 1.
pub fn native_804d<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    part44(h, s, |v| (v, 0))
}

/// `0x804e` (`0x0091c110`): `(part, v f)`:
/// `Yn_MapPartSet44_0091abc0(part, 0.0, v)`; 1.
pub fn native_804e<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    part44(h, s, |v| (0, v))
}

/// `0x8058` (`0x0091c370`): `(pc, b, a)`: `pppFpSetStickChr(a,
/// AtelGetCharHandleFromBindPC(pc), b)` (`0x0072a530`, `0x0086ae90`); 1.
pub fn native_8058<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [a, b, pc] = pops::<3>(h, s);
    let c = call(h, 0x0086_ae90, &[pc]);
    call(h, 0x0072_a530, &[a, c, b]);
    1
}

/// `0x9000` (`0x00907e60`): an `"atel_test_print!"` line through the empty
/// `dbgPrintf`; 0.
pub fn native_9000<H: NativeHost>(_h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    0
}

/// `0xc059` (`0x008782a0`): `(i, on)` -> `MsGetSaveCommandMenuPanel(i)`
/// (`0x007850b0`) before `MsSetSaveCommandMenuPanel(i, on != 0)`
/// (`0x00785d40`).
pub fn native_c059<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let on = u32::from(pop(h, s) != 0);
    let i = pop(h, s);
    let r = call(h, 0x0078_50b0, &[i]);
    call(h, 0x0078_5d40, &[i, on]);
    r
}

/// `0xc019` (`0x00878550`): `(map)`: `AtelMapLoadNoCache(map)`
/// (`0x0086dd30`); 0.
pub fn native_c019<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    one0(h, s, 0x0086_dd30)
}

/// `0xc00e` (`0x00878700`): `(id, b, c, d)`: `SqsSePlay(id, b, c, d)`
/// (`0x0081e4e0`); `id`.
pub fn native_c00e<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let [d, c, b, id] = pops::<4>(h, s);
    call(h, 0x0081_e4e0, &[id, b, c, d]);
    id
}

/// `0xc011` (`0x00878790`): `SndStartStream()` (`0x00887630`); 1.
pub fn native_c011<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    void(h, 0x0088_7630, 1)
}

/// `0xc012` (`0x008787a0`): `SndGetStreamState() != 0` (`0x008864b0`).
pub fn native_c012<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    u32::from(call(h, 0x0088_64b0, &[]) != 0)
}

/// `0xc033` (`0x00878f10`): jumps to `Ch_EnableGetTexture`
/// (`0x00830320`).
pub fn native_c033<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0083_0320)
}

/// `0xc035` (`0x00878f50`): `(pc)` -> `AtelGetCharHandleFromBindPC(pc)`
/// (`0x0086ae90`).
pub fn native_c035<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    one(h, s, 0x0086_ae90)
}

/// `0xc040` (`0x00879150`): jumps to `0x00886430`.
pub fn native_c040<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0088_6430)
}

/// `0xc042` (`0x00879180`): jumps to `SndGetSoundType` (`0x00886440`).
pub fn native_c042<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0088_6440)
}

/// `0xc043` (`0x00879190`): `SndSepClearAll()` (`0x00886db0`); 1.
pub fn native_c043<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    void(h, 0x0088_6db0, 1)
}

/// `0xc050` (`0x00879390`): `movie_stop_prog()` (`0x0076f090`) when
/// `movie_run_check()` (`0x0076f060`); 1.
pub fn native_c050<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    if call(h, 0x0076_f060, &[]) != 0 {
        call(h, 0x0076_f090, &[]);
    }
    1
}

/// `0xd000` (`0x00a445f0`): `(v)`: popped for an `"abiritymap_debug(%d)"`
/// line through the empty `dbgPrintf`; 0.
pub fn native_d000<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    pop(h, s);
    0
}

/// The Rust version of a round-19 handler, if there is one.
#[must_use]
#[allow(clippy::too_many_lines)] // one arm per handler
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
        0x0085_eef0 | 0x0087_8e50 => Int(native_0008),
        0x0085_f160 => Int(native_0009),
        0x0085_f2d0 => Int(native_000a),
        0x0085_f360 => Int(native_000b),
        0x0085_f640 => Int(native_000c),
        0x0085_65f0 => Int(native_000d),
        0x0085_66a0 => Int(native_000e),
        0x0085_6790 => Int(native_000f),
        0x0086_0120 => Int(native_0014),
        0x0085_9510 => Float(native_0022),
        0x0085_91a0 => Float(native_0023),
        0x0085_92f0 => Float(native_0024),
        0x0085_9e50 => Float(native_0026),
        0x0085_9f30 => Float(native_0027),
        0x0085_6ba0 => Int(native_0031),
        0x0085_6da0 => Int(native_0032),
        0x0086_04a0 => Int(native_003b),
        0x0085_5d50 => Int(native_003c),
        0x0085_c470 => Int(native_003e),
        0x0085_c740 => Float(native_0040),
        0x0085_c840 => Float(native_0041),
        0x0085_d760 => Int(native_0045),
        0x0085_dcc0 => Int(native_0048),
        0x0085_ddc0 => Int(native_0049),
        0x0085_dff0 => Int(native_004a),
        0x0085_e0d0 => Int(native_004b),
        0x0085_ea80 => Int(native_004e),
        0x0085_e3c0 => Int(native_004f),
        0x0085_f2a0 => Int(native_0052),
        0x0085_eda0 => Int(native_0053),
        0x0085_8850 => Int(native_0067),
        0x0085_8640 => Int(native_0068),
        0x0085_bb50 => Int(native_0072),
        0x0085_bc90 => Int(native_0073),
        0x0085_bf40 => Int(native_0075),
        0x0085_c040 | 0x0085_d440 | 0x0085_d530 => Int(native_0079),
        0x0085_7ea0 => Int(native_007b),
        0x0085_c580 => Int(native_007e),
        0x0085_d3b0 => Int(native_008a),
        0x0085_d8b0 => Int(native_008c),
        0x0085_db40 => Float(native_008e),
        0x0085_fd80 => Int(native_0099),
        0x0086_0000 => Int(native_009c),
        0x0085_81f0 => Int(native_00ad),
        0x0085_8320 => Int(native_00ae),
        0x0085_85c0 => Int(native_00b0),
        0x0085_8cf0 => Int(native_00b5),
        0x0085_93e0 => Int(native_00b9),
        0x0085_a130 => Int(native_00c3),
        0x0085_bbe0 => Int(native_00cd),
        0x0085_c7d0 => Int(native_00d3),
        0x0085_c910 => Int(native_00d4),
        0x0085_f070 => Int(native_00dd),
        0x0085_67d0 => Int(native_00ea),
        0x0085_76a0 => Int(native_00f0),
        0x0085_d620 => Int(native_00f1),
        0x0085_8290 => Int(native_00f7),
        0x0085_8400 => Int(native_00f7_poll),
        0x0085_8dd0 => Int(native_00fc),
        0x0086_02c0 => Int(native_00ff),
        0x0085_b5c0 => Int(native_0113),
        0x0085_bae0 => Int(native_0118),
        0x0085_caf0 => Int(native_011e),
        0x0085_fc50 => Int(native_0125),
        0x0085_ddd0 => Int(native_012c),
        0x0085_e170 => Int(native_012f),
        0x0085_e760 => Int(native_0131),
        0x0085_fb30 => Int(native_013a),
        0x0085_fcd0 => Int(native_013a_poll),
        0x0085_fd40 => Int(native_013a_result),
        0x0085_8250 => Int(native_0147),
        0x0085_88e0 => Int(native_014b),
        0x0085_8af0 => Int(native_014c),
        0x0085_9380 => Int(native_0150),
        0x0085_94f0 => Int(native_0152),
        0x0085_9600 => Int(native_0153),
        0x0085_7c60 => Int(native_015a),
        0x0085_b530 => Int(native_015c),
        0x0085_9e90 => Int(native_0162),
        0x0085_a0f0 => Int(native_0163),
        0x0085_9f70 => Int(native_0164),
        0x0085_a1d0 => Int(native_0165),
        0x0085_b990 => Int(native_0168),
        0x0085_bab0 => Int(native_0169),
        0x0085_c530 => Int(native_0170),
        0x0085_e7a0 => Float(native_0173),
        0x0085_eec0 => Float(native_0175),
        0x0085_ec30 => Float(native_0176),
        0x0085_cc30 => Int(native_0178),
        0x0085_d680 => Int(native_017d),
        0x0085_ed10 => Float(native_0182),
        0x0085_f690 => Float(native_0183),
        0x0085_dd50 => Int(native_0186),
        0x0085_df00 => Int(native_0187),
        0x0085_ed60 => Int(native_018c),
        0x0085_ef00 => Int(native_018d),
        0x0085_f110 => Int(native_018e),
        0x0085_f330 => Int(native_0190),
        0x0085_f5c0 => Int(native_0191),
        0x0085_6b10 => Int(native_01a1),
        0x0085_6e90 => Int(native_01a2),
        0x0085_7210 => Int(native_01a3),
        0x0085_9490 => Int(native_01b4),
        0x0085_9d20 => Int(native_01b9),
        0x0085_a0e0 => Int(native_01be),
        0x0085_a420 => Int(native_01c1),
        0x0085_e000 => Int(native_01c7),
        0x0085_e0e0 => Int(native_01cb),
        0x0085_e5d0 => Int(native_01cd),
        0x0086_0720 | 0x0087_9380 => Int(native_01d3),
        0x0085_64b0 => Int(native_01d6),
        0x0085_67e0 => Int(native_01d8),
        0x0085_f8d0 => Int(native_01df),
        0x0085_9470 => Int(native_01e8),
        0x0085_7cf0 => Int(native_01ec),
        0x0085_9620 => Float(native_01f1),
        0x0085_a160 => Int(native_01f5),
        0x0085_a2d0 => Int(native_01f6),
        0x0085_a800 => Int(native_01f7),
        0x0085_ba20 => Int(native_01fd),
        0x0085_bb20 => Int(native_01fe),
        0x0085_a300 => Int(native_01ff),
        0x0085_f210 => Int(native_0208),
        0x0085_bcc0 => Int(native_020c),
        0x0085_c090 => Int(native_020e),
        0x0085_c810 => Int(native_0211),
        0x0085_cbe0 => Int(native_0214),
        0x0085_d3f0 => Int(native_0218),
        0x0085_d500 => Int(native_0219),
        0x0085_db90 => Int(native_021c),
        0x0085_dfc0 => Int(native_021f),
        0x0085_e380 => Int(native_0222),
        0x0085_e460 => Int(native_0223),
        0x0085_e800 => Int(native_0224),
        0x0085_ac20 => Int(native_0227),
        0x0085_b060 => Int(native_0228),
        0x0085_ed40 => Int(native_022a),
        0x0085_eea0 => Int(native_022b),
        0x0085_efb0 => Int(native_022b_poll),
        0x0085_f2e0 => Int(native_022c),
        0x0085_f4c0 => Int(native_022d),
        0x0085_f600 => Int(native_022d_poll),
        0x0085_fb10 => Int(native_0230),
        0x0085_a4b0 => Int(native_0232),
        0x0085_a5f0 => Int(native_0232_poll),
        0x0085_fe10 => Int(native_0233),
        0x0085_eb30 => Int(native_0238),
        0x0085_7810 => Int(native_0246),
        0x0085_7910 => Int(native_0247),
        0x0085_79f0 => Int(native_0248),
        0x0085_7b60 => Int(native_0249),
        0x0085_d4b0 => Int(native_024e),
        0x0085_d890 => Int(native_024f),
        0x0085_de60 => Int(native_0250),
        0x0085_ef50 => Int(native_0252),
        0x0085_8a60 => Int(native_0258),
        0x0085_8da0 => Int(native_0258_poll),
        0x0085_90e0 => Int(native_025a),
        0x007b_93e0 => Int(native_6009),
        0x007b_9480 => Int(native_600b),
        0x007b_94c0 => Int(native_600c),
        0x007b_9500 => Int(native_600d),
        0x007b_95a0 => Int(native_600e),
        0x007b_9a80 => Int(native_6018),
        0x007b_9e10 => Int(native_6022),
        0x007b_9e90 => Int(native_6023),
        0x007b_9f20 => Int(native_6024),
        0x007b_9fa0 => Int(native_6025),
        0x007b_9fc0 => Int(native_6026),
        0x007b_a350 => Int(native_602c),
        0x007b_7fe0 => Int(native_602f),
        0x007b_80c0 => Int(native_6031),
        0x007b_8190 => Int(native_6036),
        0x007b_81b0 => Int(native_6037),
        0x007b_82d0 => Int(native_6039),
        0x007b_a000 => Int(native_603d),
        0x007b_86d0 => Int(native_603e),
        0x007b_8c60 => Int(native_6053),
        0x007b_8c90 => Int(native_6054),
        0x007b_8d10 => Int(native_6056),
        0x007b_8d40 => Int(native_6057),
        0x007b_87d0 => Int(native_6065),
        0x007b_9460 => Int(native_6068),
        0x007b_9090 => Int(native_606a),
        0x007b_9170 => Int(native_606d),
        0x007b_91d0 => Int(native_606f),
        0x007b_9200 => Int(native_6070),
        0x007b_9240 => Int(native_6072),
        0x007b_9280 => Int(native_6073),
        0x007b_8b50 => Int(native_6075),
        0x007b_8dc0 => Int(native_6079),
        0x007b_8de0 => Int(native_607a),
        0x007b_8e00 => Int(native_607c),
        0x007b_8e20 => Int(native_607c_poll),
        0x007b_92a0 => Int(native_6085),
        0x007b_9d80 => Int(native_6086),
        0x007b_9630 => Int(native_6089),
        0x0076_ead0 | 0x0076_eae0 | 0x0076_eaf0 | 0x0076_eb10 | 0x0076_eb20 | 0x0076_ecc0
        | 0x0076_ecd0 => Int(native_b005),
        0x0076_eb00 => Int(native_b008),
        0x007a_39d0 => Int(native_7001),
        0x007a_4550 => Int(native_7005),
        0x007a_5690 => Int(native_700e),
        0x007a_5980 => Int(native_703d),
        0x007a_5a10 => Int(native_703e_poll),
        0x007a_6060 => Int(native_7042),
        0x007a_6e10 => Int(native_7044),
        0x007a_4da0 => Int(native_7045),
        0x007a_74e0 => Int(native_704d),
        0x007a_6e80 => Int(native_7056),
        0x007a_6760 => Int(native_7065_poll),
        0x007a_6950 => Int(native_7065),
        0x007a_6960 => Int(native_7066),
        0x007a_6a30 => Int(native_7067),
        0x007a_6ad0 => Int(native_7068_poll),
        0x007a_6170 => Int(native_7069),
        0x007a_6bc0 => Int(native_706c),
        0x007a_7010 => Int(native_7070),
        0x007a_6330 => Int(native_7072),
        0x007a_29f0 => Int(native_7074),
        0x007a_5450 => Int(native_7077),
        0x007a_7200 => Int(native_7079),
        0x007a_2c70 => Int(native_7080),
        0x007a_5d60 => Int(native_7081_poll),
        0x007a_7c40 => Int(native_7089),
        0x007a_76e0 => Int(native_708d),
        0x007a_49c0 => Int(native_7090),
        0x007a_72c0 => Int(native_7094),
        0x007a_61d0 => Int(native_7095),
        0x007a_2dc0 => Int(native_7098),
        0x007a_7340 => Int(native_7099),
        0x007a_3730 => Int(native_70a3),
        0x007a_54f0 => Float(native_70a7),
        0x007a_2cd0 => Int(native_70ad),
        0x007a_7790 => Int(native_70ae),
        0x007a_79a0 => Int(native_70af),
        0x007a_7130 => Int(native_70b4),
        0x007a_52b0 => Int(native_70b6),
        0x007a_6640 => Int(native_70b7),
        0x007a_6c70 => Float(native_70c0),
        0x007a_7c20 => Int(native_70c5),
        0x007a_3950 => Int(native_70c6),
        0x007a_39f0 => Float(native_70c7),
        0x007a_75e0 => Int(native_70c8),
        0x007a_7ab0 => Int(native_70ca),
        0x007a_7b40 => Int(native_70cb),
        0x007a_7f70 => Int(native_70cf),
        0x007a_4a90 => Int(native_70d3),
        0x007a_4f20 => Int(native_70d4),
        0x007a_3500 => Int(native_70d9),
        0x007a_4df0 => Int(native_70da),
        0x007a_3450 => Int(native_70db),
        0x007a_79b0 => Int(native_70de),
        0x007a_2b50 => Int(native_70e4),
        0x007a_4c70 => Int(native_70e5),
        0x007a_4e00 => Int(native_70e6),
        0x007a_7a90 => Int(native_70e8),
        0x007a_7b00 => Int(native_70e9),
        0x007a_7d10 => Int(native_70eb),
        0x007a_3ac0 => Float(native_70f0),
        0x007a_3300 => Int(native_70f3),
        0x007a_2ad0 => Int(native_70f4),
        0x007a_34e0 => Int(native_70f6),
        0x007a_36e0 => Int(native_70f8),
        0x007a_5fc0 => Int(native_70f9),
        0x007a_37e0 => Int(native_70fb),
        0x007a_3440 => Int(native_70fc),
        0x007a_7d20 => Int(native_7100),
        0x007a_7f60 => Int(native_7101),
        0x007a_3970 => Int(native_710a),
        0x007a_3c00 => Int(native_710e),
        0x007a_3c90 => Int(native_710f),
        0x007a_8090 => Int(native_7113),
        0x007a_80a0 => Int(native_7114),
        0x007a_6930 => Int(native_7116),
        0x007a_3f40 => Int(native_711e),
        0x007a_6200 => Int(native_711f),
        0x007a_65a0 => Int(native_7120),
        0x007a_6790 => Int(native_7121),
        0x007a_7ee0 => Int(native_7124),
        0x007a_5240 => Int(native_7127),
        0x0087_8700 => Int(native_c00e),
        0x0087_8790 => Int(native_c011),
        0x0087_87a0 => Int(native_c012),
        0x0087_8550 => Int(native_c019),
        0x0087_8f10 => Int(native_c033),
        0x0087_8f50 => Int(native_c035),
        0x0087_9150 => Int(native_c040),
        0x0087_9180 => Int(native_c042),
        0x0087_9190 => Int(native_c043),
        0x0087_9390 => Int(native_c050),
        0x0087_82a0 => Int(native_c059),
        0x0091_b190 => Int(native_800e),
        0x0091_b530 => Float(native_8024),
        0x0091_b540 => Float(native_8025),
        0x0091_b5c0 => Int(native_8027),
        0x0091_b600 => Float(native_8029),
        0x0091_b670 => Int(native_802b),
        0x0091_b770 => Float(native_8033),
        0x0091_b780 => Int(native_8034),
        0x0091_c090 => Int(native_804c),
        0x0091_c0d0 => Int(native_804d),
        0x0091_c110 => Int(native_804e),
        0x0091_c370 => Int(native_8058),
        0x0091_c5a0 => Int(native_8062),
        0x0091_c890 => Int(native_8068),
        0x0091_af30 => Int(native_8069),
        0x0090_7e60 => Int(native_9000),
        0x00a4_45f0 => Int(native_d000),
        _ => return None,
    })
}

/// Every handler [`find`] knows, and whether it returns a float.
pub const PORTED: [(u32, bool); 395] = [
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
    (0x0085_eef0, false),
    (0x0085_f160, false),
    (0x0085_f2d0, false),
    (0x0085_f360, false),
    (0x0085_f640, false),
    (0x0085_65f0, false),
    (0x0085_66a0, false),
    (0x0085_6790, false),
    (0x0086_0120, false),
    (0x0085_9510, true),
    (0x0085_91a0, true),
    (0x0085_92f0, true),
    (0x0085_9e50, true),
    (0x0085_9f30, true),
    (0x0085_6ba0, false),
    (0x0085_6da0, false),
    (0x0086_04a0, false),
    (0x0085_5d50, false),
    (0x0085_c470, false),
    (0x0085_c740, true),
    (0x0085_c840, true),
    (0x0085_d760, false),
    (0x0085_dcc0, false),
    (0x0085_ddc0, false),
    (0x0085_dff0, false),
    (0x0085_e0d0, false),
    (0x0085_ea80, false),
    (0x0085_e3c0, false),
    (0x0085_f2a0, false),
    (0x0085_eda0, false),
    (0x0085_8850, false),
    (0x0085_8640, false),
    (0x0085_bb50, false),
    (0x0085_bc90, false),
    (0x0085_bf40, false),
    (0x0085_c040, false),
    (0x0085_7ea0, false),
    (0x0085_c580, false),
    (0x0085_d3b0, false),
    (0x0085_d8b0, false),
    (0x0085_db40, true),
    (0x0085_fd80, false),
    (0x0086_0000, false),
    (0x0085_81f0, false),
    (0x0085_8320, false),
    (0x0085_85c0, false),
    (0x0085_8cf0, false),
    (0x0085_93e0, false),
    (0x0085_a130, false),
    (0x0085_bbe0, false),
    (0x0085_c7d0, false),
    (0x0085_c910, false),
    (0x0085_f070, false),
    (0x0085_67d0, false),
    (0x0085_76a0, false),
    (0x0085_d620, false),
    (0x0085_8290, false),
    (0x0085_8400, false),
    (0x0085_8dd0, false),
    (0x0086_02c0, false),
    (0x0085_b5c0, false),
    (0x0085_bae0, false),
    (0x0085_caf0, false),
    (0x0085_d440, false),
    (0x0085_d530, false),
    (0x0085_fc50, false),
    (0x0085_ddd0, false),
    (0x0085_e170, false),
    (0x0085_e760, false),
    (0x0085_fb30, false),
    (0x0085_fcd0, false),
    (0x0085_fd40, false),
    (0x0085_8250, false),
    (0x0085_88e0, false),
    (0x0085_8af0, false),
    (0x0085_9380, false),
    (0x0085_94f0, false),
    (0x0085_9600, false),
    (0x0085_7c60, false),
    (0x0085_b530, false),
    (0x0085_9e90, false),
    (0x0085_a0f0, false),
    (0x0085_9f70, false),
    (0x0085_a1d0, false),
    (0x0085_b990, false),
    (0x0085_bab0, false),
    (0x0085_c530, false),
    (0x0085_e7a0, true),
    (0x0085_eec0, true),
    (0x0085_ec30, true),
    (0x0085_cc30, false),
    (0x0085_d680, false),
    (0x0085_ed10, true),
    (0x0085_f690, true),
    (0x0085_dd50, false),
    (0x0085_df00, false),
    (0x0085_ed60, false),
    (0x0085_ef00, false),
    (0x0085_f110, false),
    (0x0085_f330, false),
    (0x0085_f5c0, false),
    (0x0085_6b10, false),
    (0x0085_6e90, false),
    (0x0085_7210, false),
    (0x0085_9490, false),
    (0x0085_9d20, false),
    (0x0085_a0e0, false),
    (0x0085_a420, false),
    (0x0085_e000, false),
    (0x0085_e0e0, false),
    (0x0085_e5d0, false),
    (0x0086_0720, false),
    (0x0085_64b0, false),
    (0x0085_67e0, false),
    (0x0085_f8d0, false),
    (0x0085_9470, false),
    (0x0085_7cf0, false),
    (0x0085_9620, true),
    (0x0085_a160, false),
    (0x0085_a2d0, false),
    (0x0085_a800, false),
    (0x0085_ba20, false),
    (0x0085_bb20, false),
    (0x0085_a300, false),
    (0x0085_f210, false),
    (0x0085_bcc0, false),
    (0x0085_c090, false),
    (0x0085_c810, false),
    (0x0085_cbe0, false),
    (0x0085_d3f0, false),
    (0x0085_d500, false),
    (0x0085_db90, false),
    (0x0085_dfc0, false),
    (0x0085_e380, false),
    (0x0085_e460, false),
    (0x0085_e800, false),
    (0x0085_ac20, false),
    (0x0085_b060, false),
    (0x0085_ed40, false),
    (0x0085_eea0, false),
    (0x0085_efb0, false),
    (0x0085_f2e0, false),
    (0x0085_f4c0, false),
    (0x0085_f600, false),
    (0x0085_fb10, false),
    (0x0085_a4b0, false),
    (0x0085_a5f0, false),
    (0x0085_fe10, false),
    (0x0085_eb30, false),
    (0x0085_7810, false),
    (0x0085_7910, false),
    (0x0085_79f0, false),
    (0x0085_7b60, false),
    (0x0085_d4b0, false),
    (0x0085_d890, false),
    (0x0085_de60, false),
    (0x0085_ef50, false),
    (0x0085_8a60, false),
    (0x0085_8da0, false),
    (0x0085_90e0, false),
    (0x007b_93e0, false),
    (0x007b_9480, false),
    (0x007b_94c0, false),
    (0x007b_9500, false),
    (0x007b_95a0, false),
    (0x007b_9a80, false),
    (0x007b_9e10, false),
    (0x007b_9e90, false),
    (0x007b_9f20, false),
    (0x007b_9fa0, false),
    (0x007b_9fc0, false),
    (0x007b_a350, false),
    (0x007b_7fe0, false),
    (0x007b_80c0, false),
    (0x007b_8190, false),
    (0x007b_81b0, false),
    (0x007b_82d0, false),
    (0x007b_a000, false),
    (0x007b_86d0, false),
    (0x007b_8c60, false),
    (0x007b_8c90, false),
    (0x007b_8d10, false),
    (0x007b_8d40, false),
    (0x007b_87d0, false),
    (0x007b_9460, false),
    (0x007b_9090, false),
    (0x007b_9170, false),
    (0x007b_91d0, false),
    (0x007b_9200, false),
    (0x007b_9240, false),
    (0x007b_9280, false),
    (0x007b_8b50, false),
    (0x007b_8dc0, false),
    (0x007b_8de0, false),
    (0x007b_8e00, false),
    (0x007b_8e20, false),
    (0x007b_92a0, false),
    (0x007b_9d80, false),
    (0x007b_9630, false),
    (0x0076_ead0, false),
    (0x0076_eae0, false),
    (0x0076_eaf0, false),
    (0x0076_eb00, false),
    (0x0076_eb10, false),
    (0x0076_eb20, false),
    (0x0076_ecc0, false),
    (0x0076_ecd0, false),
    (0x007a_39d0, false),
    (0x007a_4550, false),
    (0x007a_5690, false),
    (0x007a_5980, false),
    (0x007a_5a10, false),
    (0x007a_6060, false),
    (0x007a_6e10, false),
    (0x007a_4da0, false),
    (0x007a_74e0, false),
    (0x007a_6e80, false),
    (0x007a_6760, false),
    (0x007a_6950, false),
    (0x007a_6960, false),
    (0x007a_6a30, false),
    (0x007a_6ad0, false),
    (0x007a_6170, false),
    (0x007a_6bc0, false),
    (0x007a_7010, false),
    (0x007a_6330, false),
    (0x007a_29f0, false),
    (0x007a_5450, false),
    (0x007a_7200, false),
    (0x007a_2c70, false),
    (0x007a_5d60, false),
    (0x007a_7c40, false),
    (0x007a_76e0, false),
    (0x007a_49c0, false),
    (0x007a_72c0, false),
    (0x007a_61d0, false),
    (0x007a_2dc0, false),
    (0x007a_7340, false),
    (0x007a_3730, false),
    (0x007a_54f0, true),
    (0x007a_2cd0, false),
    (0x007a_7790, false),
    (0x007a_79a0, false),
    (0x007a_7130, false),
    (0x007a_52b0, false),
    (0x007a_6640, false),
    (0x007a_6c70, true),
    (0x007a_7c20, false),
    (0x007a_3950, false),
    (0x007a_39f0, true),
    (0x007a_75e0, false),
    (0x007a_7ab0, false),
    (0x007a_7b40, false),
    (0x007a_7f70, false),
    (0x007a_4a90, false),
    (0x007a_4f20, false),
    (0x007a_3500, false),
    (0x007a_4df0, false),
    (0x007a_3450, false),
    (0x007a_79b0, false),
    (0x007a_2b50, false),
    (0x007a_4c70, false),
    (0x007a_4e00, false),
    (0x007a_7a90, false),
    (0x007a_7b00, false),
    (0x007a_7d10, false),
    (0x007a_3ac0, true),
    (0x007a_3300, false),
    (0x007a_2ad0, false),
    (0x007a_34e0, false),
    (0x007a_36e0, false),
    (0x007a_5fc0, false),
    (0x007a_37e0, false),
    (0x007a_3440, false),
    (0x007a_7d20, false),
    (0x007a_7f60, false),
    (0x007a_3970, false),
    (0x007a_3c00, false),
    (0x007a_3c90, false),
    (0x007a_8090, false),
    (0x007a_80a0, false),
    (0x007a_6930, false),
    (0x007a_3f40, false),
    (0x007a_6200, false),
    (0x007a_65a0, false),
    (0x007a_6790, false),
    (0x007a_7ee0, false),
    (0x007a_5240, false),
    (0x0087_8700, false),
    (0x0087_8790, false),
    (0x0087_87a0, false),
    (0x0087_8550, false),
    (0x0087_8e50, false),
    (0x0087_8f10, false),
    (0x0087_8f50, false),
    (0x0087_9150, false),
    (0x0087_9180, false),
    (0x0087_9190, false),
    (0x0087_9380, false),
    (0x0087_9390, false),
    (0x0087_82a0, false),
    (0x0091_b190, false),
    (0x0091_b530, true),
    (0x0091_b540, true),
    (0x0091_b5c0, false),
    (0x0091_b600, true),
    (0x0091_b670, false),
    (0x0091_b770, true),
    (0x0091_b780, false),
    (0x0091_c090, false),
    (0x0091_c0d0, false),
    (0x0091_c110, false),
    (0x0091_c370, false),
    (0x0091_c5a0, false),
    (0x0091_c890, false),
    (0x0091_af30, false),
    (0x0090_7e60, false),
    (0x00a4_45f0, false),
];
