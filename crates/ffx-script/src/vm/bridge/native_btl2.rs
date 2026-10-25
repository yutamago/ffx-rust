//! The rest of the battle natives (group 7, script round 18): the
//! handlers no shipped script calls, kept for mods. Like `native_btl`'s,
//! they act on the worker's battle actor (its index in the worker's battle
//! id byte, `0xff` for none) through the battle module.

// w, a, s: the worker, the args words and the stack, as everywhere
#![allow(clippy::many_single_char_names)]

use ffx_bridge::x87;

use super::native::Native;
use super::run::NativeHost;
use super::{atel_get_battle_id_adrs2, atel_pop_stack_float, atel_pop_stack_integer};

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

/// The worker's battle actor index (`0xff` for none).
fn actor(h: &impl NativeHost, w: u32) -> u32 {
    u32::from(h.read::<u8>(atel_get_battle_id_adrs2(w)))
}

/// `MsGetChr(index)` (`0x00794020`): the actor record.
fn chr(h: &mut impl NativeHost, i: u32) -> u32 {
    call(h, 0x0079_4020, &[i])
}

/// `0x007947f0(actor, selector, 0, 1)`: one actor's index, negative for
/// none.
fn one(h: &mut impl NativeHost, id: u32, sel: u32) -> u32 {
    call(h, 0x0079_47f0, &[id, sel, 0, 1])
}

/// `MsGetChrBit(actor, selector, 0, 1)` (`0x00794330`): a bitmask of
/// actors.
fn bits(h: &mut impl NativeHost, id: u32, sel: u32) -> u32 {
    call(h, 0x0079_4330, &[id, sel, 0, 1])
}

/// An out word: zeroed scratch (the original's stack local).
fn out(h: &mut impl NativeHost) -> u32 {
    let o = h.scratch();
    h.write(o, 0_u32);
    o
}

fn ld(h: &impl NativeHost, at: u32) -> f64 {
    x87::load(h.read(at))
}

/// `x` rounded to `f32` and loaded back (an `fstp m32`/`fld m32` pair).
fn r32(x: f64) -> f64 {
    x87::load(x87::store(x))
}

/// The low 16 bits (a `mov m16, r16`).
fn lo16(v: u32) -> u16 {
    let [a, b, _, _] = v.to_le_bytes();
    u16::from_le_bytes([a, b])
}

/// The actor's 20-slot `u16` list in `g_BtlItemMem2` (`*0x02310cd8`,
/// `0x478` bytes per actor; `0xff` is a free slot).
fn item_list(h: &impl NativeHost, id: u32) -> u32 {
    h.read::<u32>(h.rebase(0x0231_0cd8))
        .wrapping_add(id.wrapping_mul(0x478))
}

/// `Native_7123_ChrAttachWeapon` (`0x007a2980`, `0x7123`): `(weapon)` ->
/// `MsAttachWeapon(5, weapon)`'s `EAX` (`0x00793330`).
pub fn native_7123<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0079_3330, &[5, v])
}

/// `Native_70c4` (`0x007a2ba0`, `0x70c4`): `(selector)` -> 1 when either
/// weapon slot (`+0x592`, `+0x593`) of the selected actor holds a weapon
/// (`MsGetSaveWeapon(slot, 0)`, `0x007abbd0`) with bit 3 of its `+3`, else
/// 0.
pub fn native_70c4<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let id = actor(h, w);
    let sel = pop(h, s);
    let i = one(h, id, sel);
    let c = chr(h, i);
    let mut r = 0;
    for k in 0..2 {
        let slot = u32::from(h.read::<u8>(c.wrapping_add(0x592 + k)));
        let p = call(h, 0x007a_bbd0, &[slot, 0]);
        if p != 0 && h.read::<u8>(p.wrapping_add(3)) & 8 != 0 {
            r = 1;
        }
    }
    r
}

/// `Native_710c` (`0x007a2c10`, `0x710c`): with an actor, the signed byte
/// `0x0112ca0d` counts down to 0; -> its value (sign-extended).
pub fn native_710c<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = actor(h, w);
    let at = h.rebase(0x0112_ca0d);
    if id != 0xff {
        chr(h, id);
        let v = h.read::<i8>(at);
        if v > 0 {
            h.write(at, v - 1);
        }
    }
    i32::from(h.read::<i8>(at)).cast_unsigned()
}

/// `Native_70f1_ItemSlotsClear` (`0x007a2d30`, `0x70f1`): frees the actor's 20 item
/// slots (every `u16` 0xff); 0. The actor is not checked: `0xff` reaches
/// past the table, as in the original.
pub fn native_70f1<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let base = item_list(h, actor(h, w));
    for k in 0..10 {
        h.write(base.wrapping_add(4 * k), 0x00ff_00ff_u32);
    }
    0
}

/// `Native_705b_ChrGetCamWidth` (`0x007a2d80`, `0x705b`, float): `(selector)` -> the
/// selected actor's `0x00794a70(index)` float, 0.0 for none.
pub fn native_705b<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let id = actor(h, w);
    let sel = pop(h, s);
    let i = one(h, id, sel);
    if i.cast_signed() < 0 {
        return 0.0;
    }
    callf(h, 0x0079_4a70, &[i])
}

/// `Native_705c` (`0x007a2df0`, `0x705c`, float): `(selector)` -> the
/// selected actor's model scale: `0x0082ab30(model)` times the actor's
/// `f32` `+0x3a4`, rounded to `f32`; 0.0 without an actor or model.
pub fn native_705c<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let id = actor(h, w);
    let sel = pop(h, s);
    let i = one(h, id, sel);
    if i.cast_signed() < 0 {
        return 0.0;
    }
    let c = chr(h, i);
    let m = h.read::<u32>(c);
    if m == 0 {
        return 0.0;
    }
    let x = callf(h, 0x0082_ab30, &[m]);
    r32(x87::mul(x, ld(h, c.wrapping_add(0x3a4))))
}

/// `Native_70f2_ItemSlotAdd` (`0x007a2e60`, `0x70f2`): `(item)`: the item into the
/// actor's first free slot (20 `u16`, 0xff free); 0.
pub fn native_70f2<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let base = item_list(h, actor(h, w));
    for k in 0..20 {
        let at = base.wrapping_add(2 * k);
        if h.read::<u16>(at) == 0xff {
            h.write(at, lo16(v));
            break;
        }
    }
    0
}

/// `Native_709a` (`0x007a2eb0`, `0x709a`): 0 when the actor is the one in
/// the signed byte `0x0112c9e4`, else 1 (1 without an actor).
pub fn native_709a<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = actor(h, w);
    let mine = id != 0xff && i32::from(h.read::<i8>(h.rebase(0x0112_c9e4))).cast_unsigned() == id;
    u32::from(!mine)
}

/// `Native_70b9` (`0x007a2ee0`, `0x70b9`, float): `(selector)` -> the
/// float `0x00795170(0xb, MsGetChrBit(actor, selector, 0, 1), 0, &out)`
/// writes.
pub fn native_70b9<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let id = actor(h, w);
    let sel = pop(h, s);
    let b = bits(h, id, sel);
    let o = out(h);
    call(h, 0x0079_5170, &[0xb, b, 0, o]);
    ld(h, o)
}

/// `Native_70f5_CheckUseCommand` (`0x007a2f80`, `0x70f5`): `(selector, command)` ->
/// `0x0078c690(index, 0x0079a4b0(command, 0), 0)` for the selected actor
/// (its `dbgPrintf` is an empty function).
pub fn native_70f5<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let cmd = pop(h, s);
    let sel = pop(h, s);
    let id = actor(h, w);
    let i = one(h, id, sel);
    let x = call(h, 0x0079_a4b0, &[cmd, 0]);
    call(h, 0x0078_c690, &[i, x, 0])
}

/// `f64::from(core::f32::consts::PI)`: the `fdiv qword` constant at
/// `0x00b92288`.
const PI_F32: f64 = 3.141_592_741_012_573_2;

/// An angle to degrees: `MsNearRag(0, x)` (`0x007c04c0`), times 180
/// (`0x00b48378`), over [`PI_F32`], rounded to `f32`.
fn near_degrees(h: &mut impl NativeHost, x: u32) -> f64 {
    let r = callf(h, 0x007c_04c0, &[x87::store(0.0), x]);
    r32(x87::div(x87::mul(r, 180.0), PI_F32))
}

/// `(selector)` -> the float `0x00795170(k, MsGetChrBit(actor, selector,
/// 0, 1), 0, &out)` writes.
fn bits_float<H: NativeHost>(h: &mut H, w: u32, s: u32, k: u32) -> f64 {
    let id = actor(h, w);
    let sel = pop(h, s);
    let b = bits(h, id, sel);
    let o = out(h);
    call(h, 0x0079_5170, &[k, b, 0, o]);
    ld(h, o)
}

/// `(b, a)`: the positions (`0x00795170(5, MsGetChrBit(actor, x, 0, 1),
/// 0, &v)`) of selectors `a` and `b`, `0x007c4990(&va, &vb, ...)`'s angle
/// (its third argument when `third`, else its fourth) in degrees.
fn angle_between<H: NativeHost>(h: &mut H, w: u32, s: u32, third: bool) -> f64 {
    let id = actor(h, w);
    let b = pop(h, s);
    let a = pop(h, s);
    let o = h.scratch();
    h.write_bytes(o, &[0; 36]);
    let (va, vb, r) = (o, o + 16, o + 32);
    let x = bits(h, id, a);
    call(h, 0x0079_5170, &[5, x, 0, va]);
    let x = bits(h, id, b);
    call(h, 0x0079_5170, &[5, x, 0, vb]);
    if third {
        call(h, 0x007c_4990, &[va, vb, r, 0, 0]);
    } else {
        call(h, 0x007c_4990, &[va, vb, 0, r, 0]);
    }
    let x = h.read(r);
    near_degrees(h, x)
}

/// `Native_70ba` (`0x007a3040`, `0x70ba`, float): `(selector)` -> the
/// float `0x00795170(0xa, MsGetChrBit(actor, selector, 0, 1), 0, &out)`
/// writes.
pub fn native_70ba<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    bits_float(h, w, s, 0xa)
}

/// `Native_709b_ChrGetPosY` (`0x007a3170`, `0x709b`, float): `(selector)` -> the
/// second float of the vector `0x007956e0(index, &v)` writes for the
/// selected actor.
pub fn native_709b<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let id = actor(h, w);
    let sel = pop(h, s);
    let i = one(h, id, sel);
    let o = h.scratch();
    h.write_bytes(o, &[0; 16]);
    call(h, 0x0079_56e0, &[i, o]);
    ld(h, o + 4)
}

/// `Native_70c2_ChrGetAngleTo` (`0x007a31d0`, `0x70c2`, float): `(a, b)` (selectors) ->
/// the angle from `a`'s position to `b`'s (`0x007c4990`'s third result)
/// in degrees.
pub fn native_70c2<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    angle_between(h, w, s, true)
}

/// `Native_709c` (`0x007a3330`, `0x709c`, float): as `0x70c2` with
/// `0x007c4990`'s fourth result (the other angle).
pub fn native_709c<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    angle_between(h, w, s, false)
}

/// `Native_70d8` (`0x007a3400`, `0x70d8`): `(v)`: the byte `0x0112ca06` =
/// v & 1; v.
pub fn native_70d8<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    h.write(h.rebase(0x0112_ca06), v.to_le_bytes()[0] & 1);
    v
}

/// `Native_70fa` (`0x007a3420`, `0x70fa`): `(v)`: the `u16` `0x01136410` =
/// v; 0.
pub fn native_70fa<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    h.write(h.rebase(0x0113_6410), lo16(v));
    0
}

/// `Native_709f_GetTargetDirDiff` (`0x007a3460`, `0x709f`, float): `(selector)` -> the
/// selected actor's target direction (`MsGetChrTargetDir(i, 0)`,
/// `0x007958c0`) minus its direction (`MsGetChrDir`, `0x00794d20`), each
/// `f32`, in degrees.
pub fn native_709f<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let id = actor(h, w);
    let sel = pop(h, s);
    let i = one(h, id, sel);
    let t = x87::store(callf(h, 0x0079_58c0, &[i, x87::store(0.0)]));
    let o = out(h);
    call(h, 0x0079_4d20, &[i, o]);
    let d = x87::store(x87::sub(x87::load(t), ld(h, o)));
    near_degrees(h, d)
}

/// `Native_70f7_BtlListAppend` (`0x007a3560`, `0x70f7`): `(v)`: appends v to the 16
/// `u16` at `0x011363f0` (count in the byte `0x011363ec`) while there is
/// room; v.
pub fn native_70f7<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let at = h.rebase(0x0113_63ec);
    let n = h.read::<u8>(at);
    if n < 0x10 {
        h.write(h.rebase(0x0113_63f0) + 2 * u32::from(n), lo16(v));
        h.write(at, n + 1);
    }
    v
}

/// `Native_7102` (`0x007a35d0`, `0x7102`): `(b, a)` (selectors): actor
/// `b`'s `+0x439` = 1 and `+0x438` = actor `a` (0xff for none); -> that
/// index (0xff for none).
pub fn native_7102<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let id = actor(h, w);
    let a = pop(h, s);
    let b = pop(h, s);
    let i = one(h, id, b);
    let j = one(h, id, a);
    let c = chr(h, i);
    h.write(c.wrapping_add(0x439), 1_u8);
    if j.cast_signed() < 0 {
        h.write(c.wrapping_add(0x438), 0xff_u8);
        0xff
    } else {
        h.write(c.wrapping_add(0x438), j.to_le_bytes()[0]);
        j
    }
}

/// `Native_7103_ChrSetNeckTarget` (`0x007a3880`, `0x7103`): `(selector, x f, y f, z f)`:
/// for the selected actor's model (`0x00795080`), `0x008350a0(model, &v)`
/// (mode 3 and `v` to its `+0x4c0`); 0. The vector's fourth word is the
/// original's uninitialised stack; the port passes 0.
pub fn native_7103<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let id = actor(h, w);
    let z = atel_pop_stack_float(h, s);
    let y = atel_pop_stack_float(h, s);
    let x = atel_pop_stack_float(h, s);
    let sel = pop(h, s);
    let i = one(h, id, sel);
    let m = call(h, 0x0079_5080, &[i]);
    if m != 0 {
        let o = h.scratch();
        for (at, v) in (o..).step_by(4).zip([x, y, z, 0]) {
            h.write(at, v);
        }
        call(h, 0x0083_50a0, &[m, o]);
    }
    0
}

/// `Native_70fd` (`0x007a3a10`, `0x70fd`): `(b, a)`: unless the `u16`
/// `0x01136410` is set, the actor's `+0x714` = 1 (`u16`), `+0x726` and
/// `+0x727` = a, `+0x728` and `+0x729` = b (bytes); -> the actor index.
/// `MsGetChr` runs on `0xff` too.
pub fn native_70fd<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let id = actor(h, w);
    let c = chr(h, id);
    let a = pop(h, s);
    let b = pop(h, s);
    if h.read::<u16>(h.rebase(0x0113_6410)) == 0 {
        let (a, b) = (a.to_le_bytes()[0], b.to_le_bytes()[0]);
        h.write(c.wrapping_add(0x714), 1_u16);
        h.write(c.wrapping_add(0x726), a);
        h.write(c.wrapping_add(0x728), b);
        h.write(c.wrapping_add(0x727), a);
        h.write(c.wrapping_add(0x729), b);
    }
    id
}

/// `Native_710b_Poll` (`0x007a3a80`, `0x710b`, poll): 5 once the byte
/// `0x0112ca10` is set, else 0.
pub fn native_710b_poll<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    if h.read::<u8>(h.rebase(0x0112_ca10)) == 0 {
        0
    } else {
        5
    }
}

/// `Native_7004_BtlSetCenterScale` (`0x007a3aa0`, `0x7004`): `(v f)`: the `f32`
/// `0x0112bde4` = v; 0.
pub fn native_7004<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = atel_pop_stack_float(h, s);
    h.write(h.rebase(0x0112_bde4), v);
    0
}

/// `Native_70fe` (`0x007a3b80`, `0x70fe`): `(e, d, b, a)`: the actor's
/// `+0x714` = 1 (`u16`) and the bytes `+0x726` = a, `+0x728` = b, `+0x727`
/// = d, `+0x729` = e; 0. No actor check.
pub fn native_70fe<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let id = actor(h, w);
    let c = chr(h, id);
    let a = pop(h, s).to_le_bytes()[0];
    let b = pop(h, s).to_le_bytes()[0];
    let d = pop(h, s).to_le_bytes()[0];
    let e = pop(h, s).to_le_bytes()[0];
    h.write(c.wrapping_add(0x714), 1_u16);
    h.write(c.wrapping_add(0x726), a);
    h.write(c.wrapping_add(0x727), d);
    h.write(c.wrapping_add(0x729), e);
    h.write(c.wrapping_add(0x728), b);
    0
}

/// `Native_710d` (`0x007a3c20`, `0x710d`): the actor's `+0x411` = 0; 0.
pub fn native_710d<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = actor(h, w);
    if id != 0xff {
        let c = chr(h, id);
        h.write(c.wrapping_add(0x411), 0_u8);
    }
    0
}

/// `Native_70ff_BtlGetTargets` (`0x007a3d00`, `0x70ff`): the word `0x0112c8d0` = the
/// actor's `+0xdc0`; -> it. No actor check.
pub fn native_70ff<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = actor(h, w);
    let c = chr(h, id);
    let v = h.read::<u32>(c.wrapping_add(0xdc0));
    h.write(h.rebase(0x0112_c8d0), v);
    v
}

/// `(selector)`: the selected actor's model (`0x00795080`) gets `+0x4d4`
/// = `v` (`0x00834770`); -> the index. With `remap` and the word
/// `0x011363bc` set, selector 5 becomes 0.
fn model_4d4<H: NativeHost>(h: &mut H, w: u32, s: u32, remap: bool, v: u32) -> u32 {
    let id = actor(h, w);
    let mut sel = pop(h, s);
    if remap && h.read::<u32>(h.rebase(0x0113_63bc)) != 0 && sel == 5 {
        sel = 0;
    }
    let i = one(h, id, sel);
    let m = call(h, 0x0079_5080, &[i]);
    if m != 0 {
        call(h, 0x0083_4770, &[m, v]);
    }
    i
}

/// `Native_7110` (`0x007a3d30`, `0x7110`): `(selector)`: the selected
/// actor's model `+0x4d4` = 1 (selector 5 is 0 while the word
/// `0x011363bc` is set); -> the index.
pub fn native_7110<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    model_4d4(h, w, s, true, 1)
}

/// `Native_711b` (`0x007a3dd0`, `0x711b`): `(v)`: the byte `0x0112ca13` =
/// v & 1; -> v & 1.
pub fn native_711b<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s) & 1;
    h.write(h.rebase(0x0112_ca13), v.to_le_bytes()[0]);
    v
}

/// `Native_7122` (`0x007a3ef0`, `0x7122`): `(selector)`: the selected
/// actor's model `+0x4d4` = 1; -> the index.
pub fn native_7122<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    model_4d4(h, w, s, false, 1)
}

/// `Native_7111` (`0x007a3ff0`, `0x7111`): as `0x7110` with `+0x4d4` = 0.
pub fn native_7111<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    model_4d4(h, w, s, true, 0)
}

/// `Native_711a_ChrSetWind` (`0x007a4160`, `0x711a`): `(selector, a f, b f, c f)`:
/// for the selected actor's model (`0x00795080`), `0x00824c90(model, c,
/// b, a)`; 0.
pub fn native_711a<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let id = actor(h, w);
    let a = atel_pop_stack_float(h, s);
    let b = atel_pop_stack_float(h, s);
    let c = atel_pop_stack_float(h, s);
    let sel = pop(h, s);
    let i = one(h, id, sel);
    let m = call(h, 0x0079_5080, &[i]);
    if m != 0 {
        call(h, 0x0082_4c90, &[m, c, b, a]);
    }
    0
}

/// `Native_7011_BtlSetDebugCameraSleep` (`0x007a4280`, `0x7011`): `(v)`: the byte `0x0112a8fc` =
/// v & 1; 0.
pub fn native_7011<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    h.write(h.rebase(0x0112_a8fc), v.to_le_bytes()[0] & 1);
    0
}

/// `Native_70d0` (`0x007a4380`, `0x70d0`): `(v)`: the byte `0x0112c9de` =
/// v & 1; 0.
pub fn native_70d0<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    h.write(h.rebase(0x0112_c9de), v.to_le_bytes()[0] & 1);
    0
}

/// `Native_7112_ChrSetDirTarget` (`0x007a43d0`, `0x7112`): `(c, _ f, b, a)`:
/// `0x00795eb0(actor, c, 1, -1, b, a)`; 0. The float is popped and
/// dropped.
pub fn native_7112<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    atel_pop_stack_float(h, s);
    let c = pop(h, s);
    let id = actor(h, w);
    call(h, 0x0079_5eb0, &[id, c, 1, u32::MAX, b, a]);
    0
}

/// `Native_70c1_ChrGetDistance` (`0x007a4b90`, `0x70c1`): `(b, a)` (selectors) -> the
/// distance between their positions (`0x00795170(1, MsGetChrBit(actor, x,
/// 0, 1), -1, &v)`, `0x007c3030(&va, &vb)`) truncated by `_ftol2_sse`.
pub fn native_70c1<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let id = actor(h, w);
    let a = pop(h, s);
    let x = bits(h, id, a);
    let b = pop(h, s);
    let y = bits(h, id, b);
    let o = h.scratch();
    h.write_bytes(o, &[0; 32]);
    call(h, 0x0079_5170, &[1, x, u32::MAX, o]);
    call(h, 0x0079_5170, &[1, y, u32::MAX, o + 16]);
    let d = callf(h, 0x007c_3030, &[o, o + 16]);
    x87::ftol(h, d)
}

/// `Native_70df` (`0x007a4ee0`, `0x70df`): `(v)` -> 1 when the byte
/// `0x0112a8e0` is set and the `i16` at `*0x0112a9f0` (a pointer, 0 for
/// none) is `v >> 16` (arithmetic), else 0.
pub fn native_70df<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    if h.read::<u8>(h.rebase(0x0112_a8e0)) == 0 {
        return 0;
    }
    let p = h.read::<u32>(h.rebase(0x0112_a9f0));
    if p == 0 {
        return 0;
    }
    u32::from(i32::from(h.read::<i16>(p)) == v.cast_signed() >> 16)
}

/// `Native_705e_ResetBindEffect` (`0x007a5310`, `0x705e`): `0x00788e60(actor)`; 0.
pub fn native_705e<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = actor(h, w);
    call(h, 0x0078_8e60, &[id]);
    0
}

/// `Native_7020_CheckStartEffect` (`0x007a53b0`, `0x7020`): -> `0x007a92c0(actor, its
/// +0xdc0)`, 0 without an actor.
pub fn native_7020<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = actor(h, w);
    if id == 0xff {
        return 0;
    }
    let c = chr(h, id);
    let v = h.read::<u32>(c.wrapping_add(0xdc0));
    call(h, 0x007a_92c0, &[id, v])
}

/// `Native_7013_ChrSetSp` (`0x007a5840`, `0x7013`): `(v f)`: with an actor that has
/// a model, `0x0082b840(model, v)`; 0.
pub fn native_7013<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = atel_pop_stack_float(h, s);
    let id = actor(h, w);
    if id != 0xff {
        let c = chr(h, id);
        let m = h.read::<u32>(c);
        if m != 0 {
            call(h, 0x0082_b840, &[m, v]);
        }
    }
    0
}

/// A start that keeps the selected actor in the call's argument word:
/// `(selector)` -> `*args` = the index, returned.
fn start_selected<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    let sel = pop(h, s);
    let id = actor(h, w);
    let i = one(h, id, sel);
    h.write(a, i);
    i
}

/// A poll on the actor in the argument word: 5 once its byte `off` is
/// clear, else 0.
fn poll_clear<H: NativeHost>(h: &mut H, a: u32, off: u32) -> u32 {
    let i = h.read::<u32>(a);
    let c = chr(h, i);
    if h.read::<u8>(c.wrapping_add(off)) == 0 {
        5
    } else {
        0
    }
}

/// `Native_70bc_ChrWaitEffect7` (`0x007a5990`, `0x70bc`, start): `(selector)`: the
/// selected actor into the argument word; -> its index.
pub fn native_70bc<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    start_selected(h, w, a, s)
}

/// `Native_70bc_ChrWaitEffect7_Poll` (`0x007a5b90`, `0x70bc`, poll): 5 once that actor's
/// `+0xe12` is clear.
pub fn native_70bc_poll<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    poll_clear(h, a, 0xe12)
}

/// `Native_7027_ActorGetField44f` (`0x007a5e90`, `0x7027`): the actor's byte `+0x44f`, 0
/// without an actor.
pub fn native_7027<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = actor(h, w);
    if id == 0xff {
        return 0;
    }
    let c = chr(h, id);
    u32::from(h.read::<u8>(c.wrapping_add(0x44f)))
}

/// `Native_70bd_ChrWaitEffect8` (`0x007a5ed0`, `0x70bd`, start): as `0x70bc`'s start.
pub fn native_70bd<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    start_selected(h, w, a, s)
}

/// `Native_70bd_ChrWaitEffect8_Poll` (`0x007a6030`, `0x70bd`, poll): 5 once that actor's
/// `+0xe13` is clear.
pub fn native_70bd_poll<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    poll_clear(h, a, 0xe13)
}

/// `Native_7092_ChrGetGroundDist` (`0x007a5f10`, `0x7092`): `(selector)` -> the selected
/// actor's `0x00794fb0(index)` minus the second float of `0x007956e0(index,
/// &v)`, rounded to `f32`, times 100.0, truncated by `_ftol2_sse`.
pub fn native_7092<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let id = actor(h, w);
    let sel = pop(h, s);
    let i = one(h, id, sel);
    let o = h.scratch();
    h.write_bytes(o, &[0; 16]);
    call(h, 0x0079_56e0, &[i, o]);
    let x = callf(h, 0x0079_4fb0, &[i]);
    let d = r32(x87::sub(x, ld(h, o + 4)));
    x87::ftol(h, x87::mul(d, 100.0))
}

/// `Native_70d2_ChrGetEffect8Count` (`0x007a6130`, `0x70d2`): `(selector)` -> the selected
/// actor's byte `+0xe13`.
pub fn native_70d2<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let sel = pop(h, s);
    let id = actor(h, w);
    let i = one(h, id, sel);
    let c = chr(h, i);
    u32::from(h.read::<u8>(c.wrapping_add(0xe13)))
}

/// `Native_7060_StatusEffectCheck` (`0x007a62a0`, `0x7060`): `0x0079eff0(actor)`; 0.
pub fn native_7060<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = actor(h, w);
    if id != 0xff {
        call(h, 0x0079_eff0, &[id]);
    }
    0
}

/// `Native_7061_StatusEffectClear` (`0x007a6300`, `0x7061`): `0x0079f110(actor)`; 0.
pub fn native_7061<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = actor(h, w);
    if id != 0xff {
        call(h, 0x0079_f110, &[id]);
    }
    0
}

/// `Native_7064_BtlStreamStandby` (`0x007a63d0`, `0x7064`, start): `(v)`: v into the
/// argument word and the word `0x0112c24c`; -> v.
pub fn native_7064<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    h.write(a, v);
    h.write(h.rebase(0x0112_c24c), v);
    v
}

/// `Native_7064_BtlStreamStandby_Poll` (`0x007a64c0`, `0x7064`, poll): `0x0076d0d0()`,
/// then 5 when `0x007a8c20()` is set and `0x007b2c10(1)` is 0, else 0.
pub fn native_7064_poll<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call(h, 0x0076_d0d0, &[]);
    if call(h, 0x007a_8c20, &[]) != 0 && call(h, 0x007b_2c10, &[1]) == 0 {
        5
    } else {
        0
    }
}

/// `Native_7064_BtlStreamStandby_Result` (`0x007a65d0`, `0x7064`): for a non-negative v in
/// the argument word, the word `0x011363bc` = (v == `0x41fef504`) and
/// `0x00887570(v)`; 0.
pub fn native_7064_result<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    let v = h.read::<u32>(a);
    if v.cast_signed() >= 0 {
        h.write(h.rebase(0x0113_63bc), u32::from(v == 0x41fe_f504));
        call(h, 0x0088_7570, &[v]);
    }
    0
}

/// `Native_7105_Poll` (`0x007a6730`, `0x7105`, poll): 5 once the actor's
/// `+0xe17` is set, else 0. No actor check.
pub fn native_7105_poll<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = actor(h, w);
    let c = chr(h, id);
    if h.read::<u8>(c.wrapping_add(0xe17)) == 0 {
        0
    } else {
        5
    }
}

/// `Native_7002_BtlLabelExe_Poll` (`0x007a35c0`, `0x7002`, poll): 1 at once.
pub fn native_7002_poll<H: NativeHost>(_h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    1
}

/// The start of `0x7043` and `0x70a9`: `(win)`: the argument word = win
/// clamped to 0..6 (`Clamp0To6_007a86e0`), returned.
fn clamped_window<H: NativeHost>(h: &mut H, a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let r = call(h, 0x007a_86e0, &[v]);
    h.write(a, r);
    r
}

/// `Native_7043_MsgWaitClose` (`0x007a6a10`, `0x7043`, start): [`clamped_window`].
pub fn native_7043<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    clamped_window(h, a, s)
}

/// `Native_70a9_MsgWaitCloseStream` (`0x007a6c50`, `0x70a9`, start): [`clamped_window`].
pub fn native_70a9<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    clamped_window(h, a, s)
}

/// `Native_704c_ChrWaitMotionBit_Poll` (`0x007a7cf0`, `0x704c`, poll): 5 once no actor in
/// the argument word's bits is in motion (`MsAnyChrMotion_007a8be0`), else
/// 0.
pub fn native_704c_poll<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    let bits = h.read::<u32>(a);
    if call(h, 0x007a_8be0, &[bits]) == 0 {
        5
    } else {
        0
    }
}

/// `Native_706d_SoundStreamPlay_Poll` (`0x007a6db0`, `0x706d`, poll): 0 while the music
/// standby syncs (`SndSyncMusicStandby(0)`) outside the water
/// (`g_BtlUnderwater`), else 5.
pub fn native_706d_poll<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    if call(h, 0x0088_7900, &[0]) != 0 && h.read::<u8>(h.rebase(0x0112_c9ce)) == 0 {
        0
    } else {
        5
    }
}

/// `Native_706d_SoundStreamPlay_Int` (`0x007a6ea0`, `0x706d`, third slot): outside the
/// water and in a menuless battle status (`MsBattleStatusMenuless_007a8c20`)
/// the stream in the first argument word plays for the first actor the
/// second word selects (`Battle_FirstSelectedActor(actor, word 1, 0, 1)`,
/// `MsSoundStreamPlay(that, word 0)`): 0; else 1.
pub fn native_706d_int<H: NativeHost>(h: &mut H, w: u32, a: u32, _s: u32) -> u32 {
    if h.read::<u8>(h.rebase(0x0112_c9ce)) != 0 || call(h, 0x007a_8c20, &[]) == 0 {
        return 1;
    }
    let id = actor(h, w);
    let sel = h.read::<u32>(a.wrapping_add(4));
    let who = call(h, 0x0079_47f0, &[id, sel, 0, 1]);
    let stream = h.read::<u32>(a);
    call(h, 0x007a_0820, &[who, stream]);
    0
}

/// `Native_70a2_RegMotionRead_Poll` (`0x007a7500`, `0x70a2`, poll): 0 without
/// `g_BtlRegMotionFlag`; else 5 once `g_Native70a2Flag` (set by the start)
/// is 0, clearing it (and 0) while it is set.
pub fn native_70a2_poll<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    if h.read::<u8>(h.rebase(0x0112_c9cf)) == 0 {
        return 0;
    }
    let flag = h.rebase(0x0113_63e8);
    if h.read::<u32>(flag) == 0 {
        return 5;
    }
    h.write(flag, 0_u32);
    0
}

/// `Native_7043_MsgWaitClose_Poll` (`0x007a6a40`, `0x7043`, poll): on the handle in
/// the argument word: 5 when `0x008b7ca0(h)` is 0; else 0 while
/// `0x008b8530(h)` is 0, then `0x008ab8f0(h)` and 5.
pub fn native_7043_poll<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    let v = h.read::<u32>(a);
    if call(h, 0x008b_7ca0, &[v]) != 0 {
        if call(h, 0x008b_8530, &[v]) == 0 {
            return 0;
        }
        call(h, 0x008a_b8f0, &[v]);
    }
    5
}

/// `Native_70bf` (`0x007a6ae0`, `0x70bf`): `(v f)`: the actor's `f32`
/// `+0x3ac` = v and, with a model, `0x00832610(model, v)`; 0.
pub fn native_70bf<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = atel_pop_stack_float(h, s);
    let id = actor(h, w);
    if id != 0xff {
        let c = chr(h, id);
        h.write(c.wrapping_add(0x3ac), v);
        let m = h.read::<u32>(c);
        if m != 0 {
            call(h, 0x0083_2610, &[m, v]);
        }
    }
    0
}

/// `Native_70a9_MsgWaitCloseStream_Poll` (`0x007a6cb0`, `0x70a9`, poll): on the handle in the
/// argument word: 5 when its state `0x008b7ca0(h)` is 0; when
/// `0x008b8530(h)` is 1, or `0x008864b0()` is 0 and the state is 3,
/// `0x008ab8f0(h)` and 5; else 0.
pub fn native_70a9_poll<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    let v = h.read::<u32>(a);
    let t = call(h, 0x008b_7ca0, &[v]);
    if t == 0 {
        return 5;
    }
    if call(h, 0x008b_8530, &[v]) != 1 && (call(h, 0x0088_64b0, &[]) != 0 || t != 3) {
        return 0;
    }
    call(h, 0x008a_b8f0, &[v]);
    5
}

/// `Native_706d_SoundStreamPlay` (`0x007a6d00`, `0x706d`, start): `(b, a)`: the argument
/// words = a, b; -> b.
pub fn native_706d<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    let x = pop(h, s);
    let y = pop(h, s);
    h.write(a, x);
    h.write(a.wrapping_add(4), y);
    y
}

/// `Native_704f_SelSetFieldDdd` (`0x007a70c0`, `0x704f`): `(selector, v)`: every actor
/// below 31 in `MsGetChrBit(actor, selector, 0, 1)` gets the byte `+0xddd`
/// = v; 0.
pub fn native_704f<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s).to_le_bytes()[0];
    let sel = pop(h, s);
    let id = actor(h, w);
    let mask = bits(h, id, sel);
    for i in 0..31 {
        if (mask.cast_signed() >> i) & 1 != 0 {
            let c = chr(h, i);
            h.write(c.wrapping_add(0xddd), v);
        }
    }
    0
}

/// `Native_707c_Poll` (`0x007a7260`, `0x707c`, poll): 5 without an actor;
/// on the actor (or, when its `+0x11` is clear, the actor in its `+0x14`),
/// 5 when `+0xf2c` is set or `+0xf2e` is clear, else 0.
pub fn native_707c_poll<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = actor(h, w);
    if id == 0xff {
        return 5;
    }
    let mut c = chr(h, id);
    if h.read::<u8>(c.wrapping_add(0x11)) == 0 {
        let o = u32::from(h.read::<u8>(c.wrapping_add(0x14)));
        c = chr(h, o);
    }
    if h.read::<u8>(c.wrapping_add(0xf2c)) != 0 || h.read::<u8>(c.wrapping_add(0xf2e)) == 0 {
        5
    } else {
        0
    }
}

/// `Native_70a0` (`0x007a73d0`, `0x70a0`): `(v)`: the byte `0x0112c9fc` =
/// v & 1; -> its old value (sign-extended).
pub fn native_70a0<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let at = h.rebase(0x0112_c9fc);
    let old = i32::from(h.read::<i8>(at)).cast_unsigned();
    let v = pop(h, s);
    h.write(at, v.to_le_bytes()[0] & 1);
    old
}

/// `Native_704e_ActorSetField3f5` (`0x007a7530`, `0x704e`): `(v)`: the actor's bytes
/// `+0x3f5` and `+0x3f6` = v; 0.
pub fn native_704e<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let id = actor(h, w);
    let v = pop(h, s).to_le_bytes()[0];
    if id != 0xff {
        let c = chr(h, id);
        h.write(c.wrapping_add(0x3f5), v);
        h.write(c.wrapping_add(0x3f6), v);
    }
    0
}

/// `Native_706a` (`0x007a7580`, `0x706a`): `(selector)` -> 0 when the
/// selected actor's word `+0xdc0` is `1 << index`, else -1 (-1 for none).
pub fn native_706a<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let sel = pop(h, s);
    let id = actor(h, w);
    let i = one(h, id, sel);
    if i.cast_signed() < 0 {
        return u32::MAX;
    }
    let c = chr(h, i);
    if 1_u32.wrapping_shl(i) == h.read::<u32>(c.wrapping_add(0xdc0)) {
        0
    } else {
        u32::MAX
    }
}

/// `Native_70a6` (`0x007a77a0`, `0x70a6`): `(v)`: the byte `0x0112c9ff` =
/// v; v.
pub fn native_70a6<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    h.write(h.rebase(0x0112_c9ff), v.to_le_bytes()[0]);
    v
}

/// `Native_70b8` (`0x007a7860`, `0x70b8`): `(v)`: the byte `0x0112ca00` =
/// v; v.
pub fn native_70b8<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    h.write(h.rebase(0x0112_ca00), v.to_le_bytes()[0]);
    v
}

/// `Native_7075_WeaponSetMotion` (`0x007a78f0`, `0x7075`, start): `(selector, b, a)`: for
/// every actor below 31 in `MsGetChrBit(actor, selector, 0, 1)` with a
/// command record (`0x00795bb0(i, 0)`), `0x007ab360(record +0xc, b, a, 0,
/// 1, worker, &out)`; the argument word = 1 when any `out` was set, else
/// 0; -> the argument word's address (the original's leftover `EAX`).
pub fn native_7075<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    let id = actor(h, w);
    chr(h, id);
    let x = pop(h, s);
    let y = pop(h, s);
    let sel = pop(h, s);
    let mask = bits(h, id, sel);
    let mut found = 0;
    // one local for every call: a call that writes nothing keeps the last
    let o = out(h);
    for i in 0..31 {
        if (mask.cast_signed() >> i) & 1 == 0 {
            continue;
        }
        let p = call(h, 0x0079_5bb0, &[i, 0]);
        if p == 0 {
            continue;
        }
        let m = u32::from(h.read::<u8>(p.wrapping_add(0xc)));
        call(h, 0x007a_b360, &[m, y, x, 0, 1, w, o]);
        if h.read::<u32>(o) != 0 {
            found = 1;
        }
    }
    h.write(a, found);
    a
}

/// `Native_7075_WeaponSetMotion_Poll` (`0x007a7aa0`, `0x7075`, poll): `0x007a8700(args)`.
pub fn native_7075_poll<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    call(h, 0x007a_8700, &[a])
}

/// `Native_70ee` (`0x007a7b90`, `0x70ee`): `(b, a)` -> by `a + b`
/// (signed): below 0x20 `0x30de`, below 0x30 `0x30df`, below 0x3f
/// `0x30e0`, below 0x50 `0x30e1`, else `0x30e2`.
pub fn native_70ee<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let x = pop(h, s);
    let y = pop(h, s);
    match y.wrapping_add(x).cast_signed() {
        ..0x20 => 0x30de,
        0x20..0x30 => 0x30df,
        0x30..0x3f => 0x30e0,
        0x3f..0x50 => 0x30e1,
        _ => 0x30e2,
    }
}

/// `Native_704c_ChrWaitMotionBit` (`0x007a7bf0`, `0x704c`, start): `(selector)`: the
/// argument word = `MsGetChrBit(actor, selector, 0, 1)`; -> it.
pub fn native_704c<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    let id = actor(h, w);
    let sel = pop(h, s);
    let b = bits(h, id, sel);
    h.write(a, b);
    b
}

/// `Native_70e3_BtlAllocMemory` (`0x007a7da0`, `0x70e3`): `(v)`: once (while the word
/// `0x0112be98` is 0): the word `0x0112be9c` = v, `0x0083c4a0(0x40)`,
/// `0x0112be98` = `0x00687020(v)`, `0x00799f00(it, v)`; 0.
pub fn native_70e3<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let at = h.rebase(0x0112_be98);
    if h.read::<u32>(at) == 0 {
        h.write(h.rebase(0x0112_be9c), v);
        call(h, 0x0083_c4a0, &[0x40]);
        let r = call(h, 0x0068_7020, &[v]);
        h.write(at, r);
        call(h, 0x0079_9f00, &[r, v]);
    }
    0
}

/// `Native_70b3_SetVoiceWait` (`0x007a7df0`, `0x70b3`): `(v)` -> `0x007b2b60(v)`.
pub fn native_70b3<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x007b_2b60, &[v])
}

/// `Native_70dd` (`0x007a7f80`, `0x70dd`): `(v)`: the word `0x0112c8fc` =
/// v; v.
pub fn native_70dd<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    h.write(h.rebase(0x0112_c8fc), v);
    v
}

/// `Native_70d6_CalcDamageCommand` (`0x007a80b0`, `0x70d6`): `(c, b, a)`: with `i` and `j`
/// the actors selected by `c` and `b`, `r = 0x0078ce50(i, 0, -1, 0, &a)`
/// (a command record), then `0x00789bf0(chr i, chr j, r, r +0x28, r
/// +0x2a, 0, r +0x23, 0, 0, 0, 0)`'s `EAX`.
pub fn native_70d6<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    let c = pop(h, s);
    let id = actor(h, w);
    let i = one(h, id, c);
    let j = one(h, id, b);
    let o = out(h);
    h.write(o, a);
    let r = call(h, 0x0078_ce50, &[i, 0, u32::MAX, 0, o]);
    let ci = chr(h, i);
    let cj = chr(h, j);
    let byte = |h: &H, off: u32| u32::from(h.read::<u8>(r.wrapping_add(off)));
    let (x, y, z) = (byte(h, 0x28), byte(h, 0x2a), byte(h, 0x23));
    call(h, 0x0078_9bf0, &[ci, cj, r, x, y, 0, z, 0, 0, 0, 0])
}

/// `Native_7119` (`0x007a8430`, `0x7119`): `(v)`: the byte `0x0112ca12` =
/// v; v.
pub fn native_7119<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    h.write(h.rebase(0x0112_ca12), v.to_le_bytes()[0]);
    v
}

/// `Native_7126` (`0x007a8550`, `0x7126`): `(i)`: actor i's `+0x3e` = 1
/// and the same for both of its parts (`0x00795b10(i, k)`); -> i, or 0xff
/// for a negative i.
pub fn native_7126<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    if i.cast_signed() < 0 {
        return 0xff;
    }
    let c = chr(h, i);
    h.write(c.wrapping_add(0x3e), 1_u8);
    for k in 0..2 {
        let p = call(h, 0x0079_5b10, &[i, k]);
        h.write(p.wrapping_add(0x3e), 1_u8);
    }
    i
}

/// `Native_70e1` (`0x007a85a0`, `0x70e1`): -> 1 when the actor in the
/// actor's signed byte `+0xded` has `+0xdf1` clear, else 0 (0 without an
/// actor). The original compares the sign-extended byte with 0xff, which
/// never matches, so -1 reaches `MsGetChr` too.
pub fn native_70e1<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = actor(h, w);
    if id == 0xff {
        return 0;
    }
    let c = chr(h, id);
    let o = i32::from(h.read::<i8>(c.wrapping_add(0xded))).cast_unsigned();
    if o == 0xff {
        return 0;
    }
    let c = chr(h, o);
    u32::from(h.read::<u8>(c.wrapping_add(0xdf1)) == 0)
}

/// `Native_7073_ChrAttachWeapon` (`0x007a85f0`, `0x7073`): `(weapon)` ->
/// `MsAttachWeapon(actor, weapon)`'s `EAX` (`0x00793330`).
pub fn native_7073<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let id = actor(h, w);
    call(h, 0x0079_3330, &[id, v])
}

/// `Native_7125_ChrAttachWeapon` (`0x007a8650`, `0x7125`): for actor 4,
/// `MsAttachWeapon(4, 0)`; 0.
pub fn native_7125<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    if actor(h, w) == 4 {
        call(h, 0x0079_3330, &[4, 0]);
    }
    0
}

/// `Native_70e7_SetCommandTargets` (`0x007a8680`, `0x70e7`): `(command)`: the actor's word
/// `+0xdc0` = `0x007996d0(actor, command, bit 1 of the record's +0x1a
/// clear, 1)`, the record `0x0078ce50(actor, 0, -1, 0, &command)` (which
/// may change the command); -> that word. No actor check.
pub fn native_70e7<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let id = actor(h, w);
    let c = chr(h, id);
    let v = pop(h, s);
    let o = out(h);
    h.write(o, v);
    let r = call(h, 0x0078_ce50, &[id, 0, u32::MAX, 0, o]);
    let f = u32::from(!h.read::<u8>(r.wrapping_add(0x1a)) >> 1 & 1);
    let v = h.read::<u32>(o);
    let x = call(h, 0x0079_96d0, &[id, v, f, 1]);
    h.write(c.wrapping_add(0xdc0), x);
    x
}

/// The Rust version of a round-18 battle handler, if there is one.
#[must_use]
pub fn find<H: NativeHost>(sym: u32) -> Option<Native<H>> {
    use Native::{Float, Int};
    Some(match sym {
        0x007a_2980 => Int(native_7123),
        0x007a_2ba0 => Int(native_70c4),
        0x007a_2c10 => Int(native_710c),
        0x007a_2c50 => Int(super::native_move::native_01b2),
        0x007a_2d30 => Int(native_70f1),
        0x007a_2d80 => Float(native_705b),
        0x007a_2df0 => Float(native_705c),
        0x007a_2e60 => Int(native_70f2),
        0x007a_2eb0 => Int(native_709a),
        0x007a_2ee0 => Float(native_70b9),
        0x007a_2f80 => Int(native_70f5),
        0x007a_3040 => Float(native_70ba),
        0x007a_3170 => Float(native_709b),
        0x007a_31d0 => Float(native_70c2),
        0x007a_3330 => Float(native_709c),
        0x007a_3400 => Int(native_70d8),
        0x007a_3420 => Int(native_70fa),
        0x007a_3460 => Float(native_709f),
        0x007a_3560 => Int(native_70f7),
        0x007a_35d0 => Int(native_7102),
        0x007a_3880 => Int(native_7103),
        0x007a_3a10 => Int(native_70fd),
        0x007a_3a80 => Int(native_710b_poll),
        0x007a_3aa0 => Int(native_7004),
        0x007a_3b80 => Int(native_70fe),
        0x007a_3c20 => Int(native_710d),
        0x007a_3d00 => Int(native_70ff),
        0x007a_3d30 => Int(native_7110),
        0x007a_3dd0 => Int(native_711b),
        0x007a_3ef0 => Int(native_7122),
        0x007a_3ff0 => Int(native_7111),
        0x007a_4160 => Int(native_711a),
        0x007a_4280 => Int(native_7011),
        0x007a_4380 => Int(native_70d0),
        0x007a_43d0 => Int(native_7112),
        0x007a_4b90 => Int(native_70c1),
        0x007a_4ee0 => Int(native_70df),
        0x007a_5310 => Int(native_705e),
        0x007a_53b0 => Int(native_7020),
        0x007a_5840 => Int(native_7013),
        0x007a_5990 => Int(native_70bc),
        0x007a_5b90 => Int(native_70bc_poll),
        0x007a_5e90 => Int(native_7027),
        0x007a_5ed0 => Int(native_70bd),
        0x007a_6030 => Int(native_70bd_poll),
        0x007a_5f10 => Int(native_7092),
        0x007a_6130 => Int(native_70d2),
        0x007a_62a0 => Int(native_7060),
        0x007a_6300 => Int(native_7061),
        0x007a_63d0 => Int(native_7064),
        0x007a_64c0 => Int(native_7064_poll),
        0x007a_65d0 => Int(native_7064_result),
        0x007a_6730 => Int(native_7105_poll),
        0x007a_35c0 => Int(native_7002_poll),
        0x007a_6a10 => Int(native_7043),
        0x007a_6a40 => Int(native_7043_poll),
        0x007a_6c50 => Int(native_70a9),
        0x007a_6db0 => Int(native_706d_poll),
        0x007a_6ea0 => Int(native_706d_int),
        0x007a_7500 => Int(native_70a2_poll),
        0x007a_7cf0 => Int(native_704c_poll),
        0x007a_6ae0 => Int(native_70bf),
        0x007a_6cb0 => Int(native_70a9_poll),
        0x007a_6d00 => Int(native_706d),
        0x007a_70c0 => Int(native_704f),
        0x007a_7260 => Int(native_707c_poll),
        0x007a_73d0 => Int(native_70a0),
        0x007a_7530 => Int(native_704e),
        0x007a_7580 => Int(native_706a),
        0x007a_77a0 => Int(native_70a6),
        0x007a_7860 => Int(native_70b8),
        0x007a_78f0 => Int(native_7075),
        0x007a_7aa0 => Int(native_7075_poll),
        0x007a_7b90 => Int(native_70ee),
        0x007a_7bf0 => Int(native_704c),
        0x007a_7da0 => Int(native_70e3),
        0x007a_7df0 => Int(native_70b3),
        0x007a_7f80 => Int(native_70dd),
        0x007a_80b0 => Int(native_70d6),
        0x007a_8430 => Int(native_7119),
        0x007a_8550 => Int(native_7126),
        0x007a_85a0 => Int(native_70e1),
        0x007a_85f0 => Int(native_7073),
        0x007a_8650 => Int(native_7125),
        0x007a_8680 => Int(native_70e7),
        _ => return super::native_sys::find(sym),
    })
}

/// Every handler [`find`] has, with whether it returns a float.
pub const PORTED: [(u32, bool); 85] = [
    (0x007a_2980, false),
    (0x007a_2ba0, false),
    (0x007a_2c10, false),
    (0x007a_2c50, false),
    (0x007a_2d30, false),
    (0x007a_2d80, true),
    (0x007a_2df0, true),
    (0x007a_2e60, false),
    (0x007a_2eb0, false),
    (0x007a_2ee0, true),
    (0x007a_2f80, false),
    (0x007a_3040, true),
    (0x007a_3170, true),
    (0x007a_31d0, true),
    (0x007a_3330, true),
    (0x007a_3400, false),
    (0x007a_3420, false),
    (0x007a_3460, true),
    (0x007a_3560, false),
    (0x007a_35d0, false),
    (0x007a_3880, false),
    (0x007a_3a10, false),
    (0x007a_3a80, false),
    (0x007a_3aa0, false),
    (0x007a_3b80, false),
    (0x007a_3c20, false),
    (0x007a_3d00, false),
    (0x007a_3d30, false),
    (0x007a_3dd0, false),
    (0x007a_3ef0, false),
    (0x007a_3ff0, false),
    (0x007a_4160, false),
    (0x007a_4280, false),
    (0x007a_4380, false),
    (0x007a_43d0, false),
    (0x007a_4b90, false),
    (0x007a_4ee0, false),
    (0x007a_5310, false),
    (0x007a_53b0, false),
    (0x007a_5840, false),
    (0x007a_5990, false),
    (0x007a_5b90, false),
    (0x007a_5e90, false),
    (0x007a_5ed0, false),
    (0x007a_6030, false),
    (0x007a_5f10, false),
    (0x007a_6130, false),
    (0x007a_62a0, false),
    (0x007a_6300, false),
    (0x007a_63d0, false),
    (0x007a_64c0, false),
    (0x007a_65d0, false),
    (0x007a_6730, false),
    (0x007a_6a40, false),
    (0x007a_6ae0, false),
    (0x007a_6cb0, false),
    (0x007a_6d00, false),
    (0x007a_70c0, false),
    (0x007a_7260, false),
    (0x007a_73d0, false),
    (0x007a_7530, false),
    (0x007a_7580, false),
    (0x007a_77a0, false),
    (0x007a_7860, false),
    (0x007a_78f0, false),
    (0x007a_7aa0, false),
    (0x007a_7b90, false),
    (0x007a_7bf0, false),
    (0x007a_7da0, false),
    (0x007a_7df0, false),
    (0x007a_7f80, false),
    (0x007a_80b0, false),
    (0x007a_8430, false),
    (0x007a_8550, false),
    (0x007a_85a0, false),
    (0x007a_85f0, false),
    (0x007a_8650, false),
    (0x007a_8680, false),
    (0x007a_35c0, false),
    (0x007a_6a10, false),
    (0x007a_6c50, false),
    (0x007a_6db0, false),
    (0x007a_6ea0, false),
    (0x007a_7500, false),
    (0x007a_7cf0, false),
];
