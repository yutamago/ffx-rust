//! The other natives no shipped script calls (script round 18), kept for
//! mods: the system natives of group 12 (footstep sounds through
//! `SndKickFoot`, voice streams, variable watches), and the rest of groups
//! 0, 1, 4, 5 and 8 as they are ported.

// w, a, s: the worker, the args words and the stack, as everywhere
#![allow(clippy::many_single_char_names)]

use ffx_bridge::x87;

use super::native::Native;
use super::native_map::pop_st0;
use super::run::NativeHost;
use super::{
    atel_get_basic_work, atel_get_cur_ctrl_work, atel_get_mes_win_work, atel_get_pos_work_adrs,
    atel_get_real_var_adrs, atel_pop_stack_float, atel_pop_stack_integer, atel_push_stack_integer,
    script_worker_level,
};

fn pop(h: &mut impl NativeHost, s: u32) -> u32 {
    atel_pop_stack_integer(h, s)
}

fn call(h: &mut impl NativeHost, sym: u32, args: &[u32]) -> u32 {
    let f = h.rebase(sym);
    h.call_n(f, args)
}

fn callf(h: &mut impl NativeHost, sym: u32, args: &[u32]) -> f64 {
    let f = h.rebase(sym);
    h.call_float(f, args)
}

/// `SndKickFoot(kind, party, kick, variation)` (`0x008866b0`).
const SND_KICK_FOOT: u32 = 0x0088_66b0;

/// `(c, b, a)` -> `SndKickFoot(kind, a, b, c)`; 1.
fn kick3<H: NativeHost>(h: &mut H, s: u32, kind: u32) -> u32 {
    let c = pop(h, s);
    let b = pop(h, s);
    let a = pop(h, s);
    call(h, SND_KICK_FOOT, &[kind, a, b, c]);
    1
}

/// `(a)` -> `SndKickFoot(kind, a, 0, 0)`; 1.
fn kick1<H: NativeHost>(h: &mut H, s: u32, kind: u32) -> u32 {
    let a = pop(h, s);
    call(h, SND_KICK_FOOT, &[kind, a, 0, 0]);
    1
}

/// `Native_c05a_SndKickEvent` (`0x008782f0`, `0xc05a`): `(event, variation)` ->
/// `SndKickFoot(0xff, event, 0, variation)` (kind 0xff plays sound event
/// `variation`); 1.
pub fn native_c05a<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let e = pop(h, s);
    call(h, SND_KICK_FOOT, &[0xff, e, 0, v]);
    1
}

/// `Native_c008` (`0x00878390`, `0xc008`): `(var)`: the variable's address
/// (`AtelGetRealVarAdrs`), unused; 1.
pub fn native_c008<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    atel_get_real_var_adrs(h, w, v);
    1
}

/// `Native_c00f` (`0x00878750`, `0xc00f`): as `0xc008`, returning 0.
pub fn native_c00f<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    atel_get_real_var_adrs(h, w, v);
    0
}

/// `Native_c05d` (`0x00878470`, `0xc05d`): `(v)` -> `0x00887ca0(v)`; v.
pub fn native_c05d<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0088_7ca0, &[v]);
    v
}

/// `Native_c010_StreamStandby` (`0x00878770`, `0xc010`): `(voice)` ->
/// `SndStandbyStream(voice)` (`0x00887570`); 1.
pub fn native_c010<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0088_7570, &[v]);
    1
}

/// `Native_c016_SepPlaySimple` (`0x00878860`, `0xc016`): `(v)` -> `0x00886e80(v)`; 1.
pub fn native_c016<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0088_6e80, &[v]);
    1
}

/// `Native_c017_SepStop` (`0x00878880`, `0xc017`): `(v)` -> `0x00886eb0(v)`; 1.
pub fn native_c017<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0088_6eb0, &[v]);
    1
}

/// `Native_c01a_SndKickFoot` (`0x008788e0`, `0xc01a`): `(party, kick, variation)` ->
/// `SndKickFoot(1, party, kick, variation)`; 1.
pub fn native_c01a<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    kick3(h, s, 1)
}

/// `Native_c01b_SndKickFoot` (`0x00878920`, `0xc01b`): as `0xc01a` with kind 0.
pub fn native_c01b<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    kick3(h, s, 0)
}

/// `Native_c01c_SndKickFoot` (`0x00878960`, `0xc01c`): `(party, kick)` ->
/// `SndKickFoot(5, party, kick, 0)`; 1.
pub fn native_c01c<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let b = pop(h, s);
    let a = pop(h, s);
    call(h, SND_KICK_FOOT, &[5, a, b, 0]);
    1
}

/// `Native_c01d_SndKickFoot` (`0x008789a0`, `0xc01d`): as `0xc01a` with kind 2.
pub fn native_c01d<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    kick3(h, s, 2)
}

/// `Native_c01e_SndKickFoot` (`0x008789e0`, `0xc01e`): as `0xc01a` with kind 3.
pub fn native_c01e<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    kick3(h, s, 3)
}

/// `Native_c01f_SndKickFoot` (`0x00878a20`, `0xc01f`): as `0xc01a` with kind 4.
pub fn native_c01f<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    kick3(h, s, 4)
}

/// `Native_c02d_SndKickFoot` (`0x00878a60`, `0xc02d`): `(party)` -> `SndKickFoot(6,
/// party, 0, 0)`; 1.
pub fn native_c02d<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    kick1(h, s, 6)
}

/// `Native_c02e_SndKickFoot` (`0x00878a90`, `0xc02e`): as `0xc02d` with kind 7.
pub fn native_c02e<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    kick1(h, s, 7)
}

/// `Native_c020` (`0x00878ac0`, `0xc020`) and `Native_c021`
/// (`0x00878ae0`, `0xc021`): `(v)` -> v (popped and returned).
pub fn native_c020<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    pop(h, s)
}

/// `Native_c026` (`0x00878ce0`, `0xc026`): `(var, v)` ->
/// `0x00886a00(AtelGetRealVarAdrs(var), v)`; 1.
pub fn native_c026<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let x = pop(h, s);
    let p = atel_get_real_var_adrs(h, w, x);
    call(h, 0x0088_6a00, &[p, v]);
    1
}

/// `Native_c032` (`0x00878ef0`, `0xc032`): the worker's running level
/// (`Script_WorkerLevel`), unused; 1.
pub fn native_c032<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let lvl = i32::from(h.read::<u8>(w + 0x32));
    script_worker_level(h, w, lvl);
    1
}

/// `Native_c023_ChrGetPos` (`0x00878b40`, `0xc023`): `(worker, kind, x var, y var,
/// z var)`: the position of the given worker's character
/// (`0x00833c30(chr, kind, &matrix)`, its translation row `+0x30`) into
/// the three variables, 0.0 each without a character; 1.
pub fn native_c023<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let z = pop(h, s);
    let y = pop(h, s);
    let x = pop(h, s);
    let kind = pop(h, s);
    let other = pop(h, s);
    let px = atel_get_real_var_adrs(h, w, x);
    let py = atel_get_real_var_adrs(h, w, y);
    let pz = atel_get_real_var_adrs(h, w, z);
    let wk = atel_get_basic_work(h, other.cast_signed());
    let c = h.read::<u32>(wk + 0x9c);
    if c == 0 {
        for p in [px, py, pz] {
            h.write(p, 0_u32);
        }
    } else {
        let m = h.scratch();
        h.write_bytes(m, &[0; 64]);
        call(h, 0x0083_3c30, &[c, kind, m]);
        for (p, off) in [(px, 0x30), (py, 0x34), (pz, 0x38)] {
            let v = h.read::<u32>(m + off);
            h.write(p, v);
        }
    }
    1
}

/// `Native_c029` (`0x00878d90`, `0xc029`): `(flags, b, a)` -> `a` when
/// bit 1 of `flags` is set, else `b`. On the way it wraps the running
/// level's `+0x884` minus the position's yaw (`0x0088bc20`, a pure angle
/// wrap whose result is dropped; an infinity never leaves its loop).
pub fn native_c029<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let pos = atel_get_pos_work_adrs(h, w);
    let i = u32::from(h.read::<u8>(w + 0x32)) * 17;
    let a = pop(h, s);
    let b = pop(h, s);
    let flags = pop(h, s);
    let d = x87::sub(
        x87::load(h.read(w + i * 4 + 0x884)),
        x87::load(h.read(pos + 0x34)),
    );
    callf(h, 0x0088_bc20, &[x87::store(d)]);
    if flags & 2 != 0 { a } else { b }
}

/// `Native_c037` (`0x00878fa0`, `0xc037`): `(i)` -> an entry of the string
/// table at `0x008b9130()`: its base plus the `u16` offset `2i` (`2i + 1`
/// when `0x0086bad0()` is set).
pub fn native_c037<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let t = call(h, 0x008b_9130, &[]);
    let mut k = v.wrapping_add(v);
    if call(h, 0x0086_bad0, &[]) != 0 {
        k = k.wrapping_add(1);
    }
    u32::from(h.read::<u16>(t.wrapping_add(k.wrapping_mul(2)))).wrapping_add(t)
}

/// `Native_c038` (`0x00878fe0`, `0xc038`): `(c, b, var)`: on the CD
/// interface `GetCD()` (`0x0076d0d0`), its `+0x10` with `c`, then its
/// `+0` with `(b, the variable's address, 0, 0)`; 1.
pub fn native_c038<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let var = pop(h, s);
    let b = pop(h, s);
    let c = pop(h, s);
    let p = atel_get_real_var_adrs(h, w, var);
    let cd = call(h, 0x0076_d0d0, &[]);
    let f = h.read::<u32>(cd + 0x10);
    h.call_n(f, &[c]);
    let f = h.read::<u32>(cd);
    h.call_n(f, &[b, p, 0, 0]);
    1
}

/// `Native_c039` (`0x00879040`, `0xc039`): `(v)` -> the CD interface's
/// `+8` with v.
pub fn native_c039<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let cd = call(h, 0x0076_d0d0, &[]);
    let v = pop(h, s);
    let f = h.read::<u32>(cd + 8);
    h.call_n(f, &[v])
}

/// `Native_c03a_ShoutStreamStart` (`0x00879070`, `0xc03a`): `(a, b, volume)` ->
/// `SndStartShoutStream(a, b, volume)` (`0x008875d0`); 1.
pub fn native_c03a<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let b = pop(h, s);
    let a = pop(h, s);
    call(h, 0x0088_75d0, &[a, b, v]);
    1
}

/// `Native_c03d_SetEffectFlag` (`0x008790d0`, `0xc03d`): `(bit, on)`: sets
/// (`AtelSetEffectFlag`) or clears (`0x00863860`) the effect flag; -> on.
pub fn native_c03d<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let on = pop(h, s);
    let bit = pop(h, s);
    if on == 0 {
        call(h, 0x0086_3860, &[bit]);
    } else {
        call(h, 0x0086_fa40, &[bit]);
    }
    on
}

/// `(v)` -> `f(v)`'s `EAX`.
fn forward<H: NativeHost>(h: &mut H, s: u32, f: u32) -> u32 {
    let v = pop(h, s);
    call(h, f, &[v])
}

/// `Native_c03e_SepIsPlaying` (`0x00879110`, `0xc03e`): `(v)` -> `0x00886680(v)`.
pub fn native_c03e<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    forward(h, s, 0x0088_6680)
}

/// `Native_c03f_SetSoundMode` (`0x00879130`, `0xc03f`): `(v)`: `0x00887180(v)`; v.
pub fn native_c03f<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0088_7180, &[v]);
    v
}

/// `Native_c041_SetSoundType` (`0x00879160`, `0xc041`): `(v)`: `0x008871c0(v)`; v.
pub fn native_c041<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0088_71c0, &[v]);
    v
}

/// `Native_c044` (`0x008791a0`, `0xc044`): `(v)`: `0x0088c7f0(v)`; 0.
pub fn native_c044<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    forward(h, s, 0x0088_c7f0);
    0
}

/// `Native_c045` (`0x008791c0`, `0xc045`): `(v)` -> `0x0088c800(v)`.
pub fn native_c045<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    forward(h, s, 0x0088_c800)
}

/// `Native_c046` (`0x008791e0`, `0xc046`): `(v)`: `0x0088c820(v)`; 0.
pub fn native_c046<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    forward(h, s, 0x0088_c820);
    0
}

/// The buffer the `0xc047`..`0xc04e` peeks and pokes address
/// (`0x0088c830()`), plus a popped offset.
fn peek_at<H: NativeHost>(h: &mut H, s: u32) -> u32 {
    let off = pop(h, s);
    call(h, 0x0088_c830, &[]).wrapping_add(off)
}

/// `Native_c047_SaveEventGetFloat` (`0x00879200`, `0xc047`, float): `(offset)` -> the `f32`
/// at the buffer `0x0088c830()` plus offset.
pub fn native_c047<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    let p = peek_at(h, s);
    x87::load(h.read(p))
}

/// `Native_c049_SaveEventGetU32` (`0x00879220`, `0xc049`): `(offset)` -> the word there.
pub fn native_c049<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let p = peek_at(h, s);
    h.read(p)
}

/// `Native_c04b_SaveEventGetI16` (`0x00879240`, `0xc04b`): `(offset)` -> the `i16` there,
/// sign-extended.
pub fn native_c04b<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let p = peek_at(h, s);
    i32::from(h.read::<i16>(p)).cast_unsigned()
}

/// `Native_c04d_SaveEventGetI8` (`0x00879260`, `0xc04d`): `(offset)` -> the `i8` there,
/// sign-extended.
pub fn native_c04d<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let p = peek_at(h, s);
    i32::from(h.read::<i8>(p)).cast_unsigned()
}

/// `Native_c048_SaveEventSetFloat` (`0x00879280`, `0xc048`, float): `(offset, v f)`: the
/// `f32` there = v; -> v.
pub fn native_c048<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    let v = atel_pop_stack_float(h, s);
    let p = peek_at(h, s);
    h.write(p, v);
    x87::load(v)
}

/// `(offset, v f)`: v truncated (`_ftol2_sse` on the popped `ST0`).
fn poke_value<H: NativeHost>(h: &mut H, s: u32) -> (u32, u32) {
    let x = pop_st0(h, s);
    let v = x87::ftol(h, x);
    (v, peek_at(h, s))
}

/// `Native_c04a_SaveEventSetU32` (`0x008792c0`, `0xc04a`): `(offset, v f)`: the word there
/// = v truncated; -> it.
pub fn native_c04a<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let (v, p) = poke_value(h, s);
    h.write(p, v);
    v
}

/// `Native_c04c_SaveEventSetU16` (`0x00879300`, `0xc04c`): as `0xc04a`, storing the low
/// 16 bits.
pub fn native_c04c<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let (v, p) = poke_value(h, s);
    let [a, b, _, _] = v.to_le_bytes();
    h.write(p, u16::from_le_bytes([a, b]));
    v
}

/// `Native_c04e_SaveEventSetU8` (`0x00879340`, `0xc04e`): as `0xc04a`, storing the low
/// byte.
pub fn native_c04e<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let (v, p) = poke_value(h, s);
    h.write(p, v.to_le_bytes()[0]);
    v
}

// ---- group 8: the map's parts, programs and objects ----

/// Pops one value per kind (`true` a float, as `AtelPopStackFloat` and an
/// `fstp m32` leave it; `false` an integer), the first popped first, and
/// calls `f` with them in push order; 1.
fn pop_mixed<H: NativeHost>(h: &mut H, s: u32, floats: &[bool], f: u32) -> u32 {
    let mut v: Vec<u32> = floats
        .iter()
        .map(|&fl| {
            if fl {
                atel_pop_stack_float(h, s)
            } else {
                pop(h, s)
            }
        })
        .collect();
    v.reverse();
    call(h, f, &v);
    1
}

macro_rules! int_forwards {
    ($($(#[$m:meta])* $name:ident => $f:expr;)*) => {$(
        $(#[$m])*
        pub fn $name<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
            forward(h, s, $f)
        }
    )*};
}

macro_rules! float_forwards {
    ($($(#[$m:meta])* $name:ident => $f:expr;)*) => {$(
        $(#[$m])*
        pub fn $name<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
            let v = pop(h, s);
            callf(h, $f, &[v])
        }
    )*};
}

macro_rules! mixed_calls {
    ($($(#[$m:meta])* $name:ident => $kinds:expr, $f:expr;)*) => {$(
        $(#[$m])*
        pub fn $name<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
            pop_mixed(h, s, &$kinds, $f)
        }
    )*};
}

int_forwards! {
    /// `Native_8012` (`0x0091b240`, `0x8012`): `(v)` -> `0x0091a440(v)`.
    native_8012 => 0x0091_a440;
    /// `Native_8013` (`0x0091b260`, `0x8013`): `(v)` -> `0x0091a490(v)`.
    native_8013 => 0x0091_a490;
    /// `Native_8017` (`0x0091b320`, `0x8017`): `(v)` -> `0x0091a580(v)`.
    native_8017 => 0x0091_a580;
    /// `Native_8018` (`0x0091b340`, `0x8018`): `(v)` -> `0x0091a5d0(v)`.
    native_8018 => 0x0091_a5d0;
    /// `Native_801a_MapPartGetWidth` (`0x0091b3a0`, `0x801a`): `(v)` -> `0x0091a620(v)`.
    native_801a => 0x0091_a620;
    /// `Native_801b_MapPartGetHeight` (`0x0091b3c0`, `0x801b`): `(v)` -> `0x0091a3f0(v)`.
    native_801b => 0x0091_a3f0;
    /// `Native_801c_MapPartAnimCount` (`0x0091b3e0`, `0x801c`): `(v)` -> `0x0091a2a0(v)`.
    native_801c => 0x0091_a2a0;
    /// `Native_8031_MapPartGetAlpha` (`0x0091b720`, `0x8031`): `(v)` -> `0x0091a2e0(v)`.
    native_8031 => 0x0091_a2e0;
    /// `Native_8065_CheckDobj` (`0x0091c780`, `0x8065`): `(v)` -> `0x009086b0(v)`.
    native_8065 => 0x0090_86b0;
}

float_forwards! {
    /// `Native_8015` (`0x0091b2c0`, `0x8015`, float): `(v)` ->
    /// `0x0091a670(v)`'s float.
    native_8015 => 0x0091_a670;
    /// `Native_8050_MapPartScaleX` (`0x0091c1a0`, `0x8050`, float): `(v)` ->
    /// `0x0091a530(v)`'s float.
    native_8050 => 0x0091_a530;
    /// `Native_8051_MapPartScaleY` (`0x0091c1c0`, `0x8051`, float): `(v)` ->
    /// `0x0091a4e0(v)`'s float.
    native_8051 => 0x0091_a4e0;
    /// `Native_8056_MapPartPosX` (`0x0091c330`, `0x8056`, float): `(v)` ->
    /// `0x0091a330(v)`'s float.
    native_8056 => 0x0091_a330;
    /// `Native_8057_MapPartPosY` (`0x0091c350`, `0x8057`, float): `(v)` ->
    /// `0x0091a380(v)`'s float.
    native_8057 => 0x0091_a380;
}

mixed_calls! {
    /// `Native_805a_FpResetStick` (`0x0091c3f0`, `0x805a`): `(v)`: `0x0072a120(v)`; 1.
    native_805a => [false], 0x0072_a120;
    /// `Native_8061` (`0x0091c580`, `0x8061`): `(v)`: `0x00908580(v)`; 1.
    native_8061 => [false], 0x0090_8580;
    /// `Native_8016` (`0x0091b2e0`, `0x8016`): `(a, b, c)`:
    /// `0x0091ac40(a, b, c)`; 1.
    native_8016 => [false, false, false], 0x0091_ac40;
    /// `Native_8019_MapPartSetSize` (`0x0091b360`, `0x8019`): `(a, b, c)`:
    /// `0x0091ac80(a, b, c)`; 1.
    native_8019 => [false, false, false], 0x0091_ac80;
    /// `Native_8023_BgSetScrOffset` (`0x0091b4e0`, `0x8023`): `(a f, b f)`:
    /// `0x0091ad40(a, b)`; 1.
    native_8023 => [true, true], 0x0091_ad40;
    /// `Native_8028_BgSetScrBank` (`0x0091b5d0`, `0x8028`): `(v f)`: `0x0091ad10(v)`; 1.
    native_8028 => [true], 0x0091_ad10;
    /// `Native_802a` (`0x0091b610`, `0x802a`): `(a, b f, c, d f)`:
    /// `0x0091aa00(a, b, c, d)`; 1.
    native_802a => [true, false, true, false], 0x0091_aa00;
    /// `Native_8052_MapPartSetPos` (`0x0091c1e0`, `0x8052`): `(a, b f, c f)`:
    /// `0x0091aae0(a, b, c)`; 1.
    native_8052 => [true, true, false], 0x0091_aae0;
    /// `Native_8055_MapPartMoveTo` (`0x0091c2d0`, `0x8055`): `(a, b f, c f, d)`:
    /// `0x0091a0f0(a, b, c, d)`; 1.
    native_8055 => [false, true, true, false], 0x0091_a0f0;
    /// `Native_8063` (`0x0091c600`, `0x8063`): `(a f, b f, c f, d f)`:
    /// `0x0090a200(a, b, c, d)`; 1.
    native_8063 => [true; 4], 0x0090_a200;
    /// `Native_8064` (`0x0091c6b0`, `0x8064`): six floats to
    /// `0x0090a230`; 1.
    native_8064 => [true; 6], 0x0090_a230;
}

/// `(v)`: a record filled by `0x009086d0(&r)`, word `k` = v, then
/// `0x0090a040(&r)`; 1.
fn record_word<H: NativeHost>(h: &mut H, s: u32, k: u32) -> u32 {
    let r = h.scratch();
    h.write_bytes(r, &[0; 16]);
    call(h, 0x0090_86d0, &[r]);
    let v = pop(h, s);
    h.write(r + 4 * k, v);
    call(h, 0x0090_a040, &[r]);
    1
}

/// `Native_8046_BackSetRed` (`0x0091bf10`, `0x8046`): `(v)`: the record from
/// `0x009086d0` with word 0 = v to `0x0090a040`; 1.
pub fn native_8046<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    record_word(h, s, 0)
}

/// `Native_8047_BackSetGreen` (`0x0091bf60`, `0x8047`): as `0x8046` with word 1.
pub fn native_8047<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    record_word(h, s, 1)
}

/// `Native_8048_BackSetBlue` (`0x0091bfb0`, `0x8048`): as `0x8046` with word 2.
pub fn native_8048<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    record_word(h, s, 2)
}

/// `Native_8053_MapPartSetX` (`0x0091c230`, `0x8053`): `(i, v f)`: `0x0091aae0(i, v,
/// 0x0091a380(i))` (the last rounded to `f32`); 1.
pub fn native_8053<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = atel_pop_stack_float(h, s);
    let i = pop(h, s);
    let r = x87::store(callf(h, 0x0091_a380, &[i]));
    call(h, 0x0091_aae0, &[i, v, r]);
    1
}

/// `Native_8054_MapPartSetY` (`0x0091c280`, `0x8054`): `(i, v f)`: `0x0091aae0(i,
/// 0x0091a330(i, v), v)` (rounded to `f32`); 1.
pub fn native_8054<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = atel_pop_stack_float(h, s);
    let i = pop(h, s);
    let r = x87::store(callf(h, 0x0091_a330, &[i, v]));
    call(h, 0x0091_aae0, &[i, r, v]);
    1
}

// ---- group 0: the field natives no shipped script calls ----

/// `Native_013c_MsgShowWaitEx` (`0x00855d70`, `0x013c`, start): `Msg_ShowSetup` with
/// mode 8 (the window, text, open and choice in one; its poll and result
/// are `Native_MsgSelect`'s, `0x00855ec0` and `0x00856030` being bare
/// jumps to them).
pub fn native_013c<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    super::native_msg::msg_show_setup(h, w, a, s, 8)
}

/// `Native_0101_SndSepPanTranslate` (`0x00855d90`, `0x0101`): `(id, pan, frames)` ->
/// `SndSepPanTranslate(id, pan, frames)` (`0x00886e20`); frames.
pub fn native_0101<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let c = pop(h, s);
    let b = pop(h, s);
    let a = pop(h, s);
    call(h, 0x0088_6e20, &[a, b, c]);
    c
}

/// `Native_01c8_DisposeAnimOwnDobj` (`0x00855e50`, `0x01c8`): `(v)`: only for a worker
/// attached as kind 5 (`+0xaa`): pushes its `+0xa8` and v, pops them back
/// (a full stack drops pushes, as in the original) and calls
/// `yiDisposeAnimDObj(+0xa8, v)` (`0x009084b0`); 1, else 0.
pub fn native_01c8<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    if h.read::<u8>(w + 0xaa) != 5 {
        return 0;
    }
    let k = u32::from(h.read::<u16>(w + 0xa8));
    atel_push_stack_integer(h, s, k);
    atel_push_stack_integer(h, s, v);
    let x = pop(h, s);
    let y = pop(h, s);
    call(h, 0x0090_84b0, &[y, x]);
    1
}

/// `Native_00a0_MsgSetMacroType` (`0x00855ed0`, `0x00a0`): `(win, n, t)` ->
/// `TOMacroSetType(win, n, t)` (`0x008b8960`); win.
pub fn native_00a0<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let t = pop(h, s);
    let n = pop(h, s);
    let win = pop(h, s);
    call(h, 0x008b_8960, &[win, n, t]);
    win
}

/// `Native_019a_AttachChrDobj` (`0x00856040`, `0x019a`): `(obj, _, slot)`: with a
/// character, `yiAttachChrDObj(obj, chr, slot)` (`0x00908350`); 1.
pub fn native_019a<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let slot = pop(h, s);
    pop(h, s);
    let obj = pop(h, s);
    let c = h.read::<u32>(w + 0x9c);
    if c != 0 {
        call(h, 0x0090_8350, &[obj, c, slot]);
    }
    1
}

/// `Native_009f_MsgSetMacroValue` (`0x00856090`, `0x009f`): `(win, n, v)` ->
/// `TOMacroSetValue(win, n, v)` (`0x008b8980`); win.
pub fn native_009f<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let n = pop(h, s);
    let win = pop(h, s);
    call(h, 0x008b_8980, &[win, n, v]);
    win
}

/// `Native_00e5_CtrlSetByte1Bit3` (`0x00856130`, `0x00e5`): `(on)`: bit 3 of the current
/// ctrl work's `+1` = on != 0; -> that bit.
pub fn native_00e5<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let c = atel_get_cur_ctrl_work(h);
    let on = pop(h, s) != 0;
    let b = h.read::<u8>(c + 1);
    h.write(c + 1, (b & !8) | (u8::from(on) << 3));
    u32::from(on)
}

/// `Native_01d5` (`0x008561e0`, `0x01d5`): `(a, b)`: the CD interface's
/// `+0x88(a, b)`; 1.
pub fn native_01d5<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let cd = call(h, 0x0076_d0d0, &[]);
    let b = pop(h, s);
    let a = pop(h, s);
    let f = h.read::<u32>(cd + 0x88);
    h.call_n(f, &[a, b]);
    1
}

/// `Native_00a1_MsgWinSetColor` (`0x00856220`, `0x00a1`): `(win, v)`: the message
/// window's `+0x10` = v; v.
pub fn native_00a1<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let win = pop(h, s);
    let m = atel_get_mes_win_work(h, win.cast_signed());
    h.write(m + 0x10, v);
    v
}

/// `Native_019b_AttachChrOwnDobj` (`0x008563e0`, `0x019b`): `(b, a)`: pushes the worker's
/// `+0xa8`, b and a, pops them back (a, b dropped, then `+0xa8`) and, with
/// a character, `yiAttachChrDObj(+0xa8, chr, a)`; 1.
pub fn native_019b<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    let k = u32::from(h.read::<u16>(w + 0xa8));
    atel_push_stack_integer(h, s, k);
    atel_push_stack_integer(h, s, b);
    atel_push_stack_integer(h, s, a);
    let x = pop(h, s);
    pop(h, s);
    let y = pop(h, s);
    let c = h.read::<u32>(w + 0x9c);
    if c != 0 {
        call(h, 0x0090_8350, &[y, c, x]);
    }
    1
}

/// `Native_001e_FloatDropTop` (`0x00856490`, `0x001e`, float): `(v f, _ f)` -> v as
/// `AtelPopStackFloat` leaves it (the top is popped and dropped).
pub fn native_001e<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    atel_pop_stack_float(h, s);
    pop_st0(h, s)
}

/// `Native_023e_SepLock` (`0x00856640`, `0x023e`): `(id)`: `SndSepLock(id)`
/// (`0x00886dd0`); id.
pub fn native_023e<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0088_6dd0, &[v]);
    v
}

/// `Native_01d7` (`0x00856740`, `0x01d7`): `(v)`: the CD interface's
/// `+0x8c(v)`; v.
pub fn native_01d7<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let cd = call(h, 0x0076_d0d0, &[]);
    let v = pop(h, s);
    let f = h.read::<u32>(cd + 0x8c);
    h.call_n(f, &[v]);
    v
}

/// `Native_01db_VibrateStop` (`0x00856e00`, `0x01db`): `(v)`: `0x00876290(v)`; v.
pub fn native_01db<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0087_6290, &[v]);
    v
}

/// `Native_01ef_VibrateStop2` (`0x00857320`, `0x01ef`): pushes 2 and pops it back (the
/// old top when the stack is full), `0x00876290(it)`; -> it.
pub fn native_01ef<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    atel_push_stack_integer(h, s, 2);
    let v = pop(h, s);
    call(h, 0x0087_6290, &[v]);
    v
}

/// `Native_01dd` (`0x00857660`, `0x01dd`, start): `(v)`: v into the
/// argument word; v. Its poll (`0x008577f0`) is a copy of
/// `Native_01de_VibrateWait3_Poll`.
pub fn native_01dd<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    h.write(a, v);
    v
}

/// `Native_01ea_VibrateSetStreamExt` (`0x00857aa0`, `0x01ea`): `(var)`: `0x008715e0(the
/// variable's address)`; 1.
pub fn native_01ea<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let p = atel_get_real_var_adrs(h, w, v);
    call(h, 0x0087_15e0, &[p]);
    1
}

/// `Native_00af_SetBattleSceneGroup` (`0x00858440`, `0x00af`): `(v)`: `0x0086f3c0(v)`; v.
pub fn native_00af<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0086_f3c0, &[v]);
    v
}

/// `(i)` -> the word at `off` of the record `0x00785330(i)`.
fn record_field<H: NativeHost>(h: &mut H, s: u32, off: u32) -> u32 {
    let i = pop(h, s);
    let r = call(h, 0x0078_5330, &[i]);
    h.read(r.wrapping_add(off))
}

/// `Native_01a9` (`0x00858500`, `0x01a9`): `(i)` -> the word `+0x50` of
/// the record `0x00785330(i)`.
pub fn native_01a9<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    record_field(h, s, 0x50)
}

/// `Native_01aa` (`0x00858690`, `0x01aa`): as `0x01a9` with `+0x58`.
pub fn native_01aa<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    record_field(h, s, 0x58)
}

/// `Native_01e1` (`0x008586e0`, `0x01e1`): for a worker attached as kind 1
/// (`+0xaa`) with a character, `0x0082a9a0(chr)`; else 0.
pub fn native_01e1<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    if h.read::<u8>(w + 0xaa) != 1 {
        return 0;
    }
    let c = h.read::<u32>(w + 0x9c);
    if c == 0 {
        return 0;
    }
    call(h, 0x0082_a9a0, &[c])
}

/// `Native_01ae_RemoveMonsterCapture` (`0x00858df0`, `0x01ae`): `(v)` -> `0x00790b90(v, -1)`.
pub fn native_01ae<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0079_0b90, &[v, u32::MAX])
}

/// One of the three words `0x0083adf0(&a, &b, &c)` can write (the others
/// 0). The original makes its local with `push ecx`; the port zeroes it.
fn adf0_word<H: NativeHost>(h: &mut H, k: usize) -> u32 {
    let o = h.scratch();
    h.write(o, 0_u32);
    let mut args = [0; 3];
    args[k] = o;
    call(h, 0x0083_adf0, &args);
    h.read(o)
}

/// `Native_00b6_GetTkAntiX` (`0x00858e20`, `0x00b6`): -> the first word of
/// `0x0083adf0`.
pub fn native_00b6<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    adf0_word(h, 0)
}

/// `Native_00b7_GetTkAntiY` (`0x00859080`, `0x00b7`): -> the second.
pub fn native_00b7<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    adf0_word(h, 1)
}

/// `Native_00b8_GetTkAntiB` (`0x00859220`, `0x00b8`): -> the third.
pub fn native_00b8<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    adf0_word(h, 2)
}

/// The Rust version of a round-18 handler of the other groups, if there
/// is one.
#[must_use]
#[allow(clippy::too_many_lines)] // one arm per handler
pub fn find<H: NativeHost>(sym: u32) -> Option<Native<H>> {
    use Native::{Float, Int};
    Some(match sym {
        0x0087_82f0 => Int(native_c05a),
        0x0087_8390 => Int(native_c008),
        0x0087_8750 => Int(native_c00f),
        0x0087_8470 => Int(native_c05d),
        0x0087_8770 => Int(native_c010),
        0x0087_8860 => Int(native_c016),
        0x0087_8880 => Int(native_c017),
        0x0087_88e0 => Int(native_c01a),
        0x0087_8920 => Int(native_c01b),
        0x0087_8960 => Int(native_c01c),
        0x0087_89a0 => Int(native_c01d),
        0x0087_89e0 => Int(native_c01e),
        0x0087_8a20 => Int(native_c01f),
        0x0087_8a60 => Int(native_c02d),
        0x0087_8a90 => Int(native_c02e),
        0x0087_8ac0 | 0x0087_8ae0 => Int(native_c020),
        0x0087_8ce0 => Int(native_c026),
        0x0087_8ef0 => Int(native_c032),
        0x0087_8b40 => Int(native_c023),
        0x0087_8d90 => Int(native_c029),
        0x0087_8fa0 => Int(native_c037),
        0x0087_8fe0 => Int(native_c038),
        0x0087_9040 => Int(native_c039),
        0x0087_9070 => Int(native_c03a),
        0x0087_90d0 => Int(native_c03d),
        0x0087_9110 => Int(native_c03e),
        0x0087_9130 => Int(native_c03f),
        0x0087_9160 => Int(native_c041),
        0x0087_91a0 => Int(native_c044),
        0x0087_91c0 => Int(native_c045),
        0x0087_91e0 => Int(native_c046),
        0x0087_9200 => Float(native_c047),
        0x0087_9220 => Int(native_c049),
        0x0087_9240 => Int(native_c04b),
        0x0087_9260 => Int(native_c04d),
        0x0087_9280 => Float(native_c048),
        0x0087_92c0 => Int(native_c04a),
        0x0087_9300 => Int(native_c04c),
        0x0087_9340 => Int(native_c04e),
        0x0091_b240 => Int(native_8012),
        0x0091_b260 => Int(native_8013),
        0x0091_b320 => Int(native_8017),
        0x0091_b340 => Int(native_8018),
        0x0091_b3a0 => Int(native_801a),
        0x0091_b3c0 => Int(native_801b),
        0x0091_b3e0 => Int(native_801c),
        0x0091_b720 => Int(native_8031),
        0x0091_c780 => Int(native_8065),
        0x0091_b2c0 => Float(native_8015),
        0x0091_c1a0 => Float(native_8050),
        0x0091_c1c0 => Float(native_8051),
        0x0091_c330 => Float(native_8056),
        0x0091_c350 => Float(native_8057),
        0x0091_c3f0 => Int(native_805a),
        0x0091_c580 => Int(native_8061),
        0x0091_b2e0 => Int(native_8016),
        0x0091_b360 => Int(native_8019),
        0x0091_b4e0 => Int(native_8023),
        0x0091_b5d0 => Int(native_8028),
        0x0091_b610 => Int(native_802a),
        0x0091_c1e0 => Int(native_8052),
        0x0091_c2d0 => Int(native_8055),
        0x0091_c600 => Int(native_8063),
        0x0091_c6b0 => Int(native_8064),
        0x0091_bf10 => Int(native_8046),
        0x0091_bf60 => Int(native_8047),
        0x0091_bfb0 => Int(native_8048),
        0x0091_c230 => Int(native_8053),
        0x0091_c280 => Int(native_8054),
        0x0085_5d70 => Int(native_013c),
        0x0085_5ec0 => Int(super::native_msg::native_msg_select_poll),
        0x0085_6030 => Int(super::native_msg::native_msg_select_result),
        0x0085_5d90 => Int(native_0101),
        0x0085_5e50 => Int(native_01c8),
        0x0085_5ed0 => Int(native_00a0),
        0x0085_6040 => Int(native_019a),
        0x0085_6090 => Int(native_009f),
        0x0085_6130 => Int(native_00e5),
        0x0085_61e0 => Int(native_01d5),
        0x0085_6220 => Int(native_00a1),
        0x0085_63e0 => Int(native_019b),
        0x0085_6490 => Float(native_001e),
        0x0085_6600 => Int(super::native_move::native_set_touch_range),
        0x0085_6640 => Int(native_023e),
        0x0085_6740 => Int(native_01d7),
        0x0085_6e00 => Int(native_01db),
        0x0085_7320 => Int(native_01ef),
        0x0085_7660 => Int(native_01dd),
        0x0085_77f0 => Int(super::native_move::native_01de_poll),
        0x0085_7aa0 => Int(native_01ea),
        0x0085_8440 => Int(native_00af),
        0x0085_8500 => Int(native_01a9),
        0x0085_8690 => Int(native_01aa),
        0x0085_86e0 => Int(native_01e1),
        0x0085_8df0 => Int(native_01ae),
        0x0085_8e20 => Int(native_00b6),
        0x0085_9080 => Int(native_00b7),
        0x0085_9220 => Int(native_00b8),
        _ => return super::native_chr::find(sym),
    })
}

/// Every handler [`find`] has, with whether it returns a float.
pub const PORTED: [(u32, bool); 99] = [
    (0x0087_82f0, false),
    (0x0087_8390, false),
    (0x0087_8750, false),
    (0x0087_8470, false),
    (0x0087_8770, false),
    (0x0087_8860, false),
    (0x0087_8880, false),
    (0x0087_88e0, false),
    (0x0087_8920, false),
    (0x0087_8960, false),
    (0x0087_89a0, false),
    (0x0087_89e0, false),
    (0x0087_8a20, false),
    (0x0087_8a60, false),
    (0x0087_8a90, false),
    (0x0087_8ac0, false),
    (0x0087_8ae0, false),
    (0x0087_8ce0, false),
    (0x0087_8ef0, false),
    (0x0087_8b40, false),
    (0x0087_8d90, false),
    (0x0087_8fa0, false),
    (0x0087_8fe0, false),
    (0x0087_9040, false),
    (0x0087_9070, false),
    (0x0087_90d0, false),
    (0x0087_9110, false),
    (0x0087_9130, false),
    (0x0087_9160, false),
    (0x0087_91a0, false),
    (0x0087_91c0, false),
    (0x0087_91e0, false),
    (0x0087_9200, true),
    (0x0087_9220, false),
    (0x0087_9240, false),
    (0x0087_9260, false),
    (0x0087_9280, true),
    (0x0087_92c0, false),
    (0x0087_9300, false),
    (0x0087_9340, false),
    (0x0091_b240, false),
    (0x0091_b260, false),
    (0x0091_b320, false),
    (0x0091_b340, false),
    (0x0091_b3a0, false),
    (0x0091_b3c0, false),
    (0x0091_b3e0, false),
    (0x0091_b720, false),
    (0x0091_c780, false),
    (0x0091_b2c0, true),
    (0x0091_c1a0, true),
    (0x0091_c1c0, true),
    (0x0091_c330, true),
    (0x0091_c350, true),
    (0x0091_c3f0, false),
    (0x0091_c580, false),
    (0x0091_b2e0, false),
    (0x0091_b360, false),
    (0x0091_b4e0, false),
    (0x0091_b5d0, false),
    (0x0091_b610, false),
    (0x0091_c1e0, false),
    (0x0091_c2d0, false),
    (0x0091_c600, false),
    (0x0091_c6b0, false),
    (0x0091_bf10, false),
    (0x0091_bf60, false),
    (0x0091_bfb0, false),
    (0x0091_c230, false),
    (0x0091_c280, false),
    (0x0085_5d70, false),
    (0x0085_5ec0, false),
    (0x0085_6030, false),
    (0x0085_5d90, false),
    (0x0085_5e50, false),
    (0x0085_5ed0, false),
    (0x0085_6040, false),
    (0x0085_6090, false),
    (0x0085_6130, false),
    (0x0085_61e0, false),
    (0x0085_6220, false),
    (0x0085_63e0, false),
    (0x0085_6490, true),
    (0x0085_6600, false),
    (0x0085_6640, false),
    (0x0085_6740, false),
    (0x0085_6e00, false),
    (0x0085_7320, false),
    (0x0085_7660, false),
    (0x0085_77f0, false),
    (0x0085_7aa0, false),
    (0x0085_8440, false),
    (0x0085_8500, false),
    (0x0085_8690, false),
    (0x0085_86e0, false),
    (0x0085_8df0, false),
    (0x0085_8e20, false),
    (0x0085_9080, false),
    (0x0085_9220, false),
];
