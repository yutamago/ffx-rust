//! The battle natives in Rust (group 7): actor properties, commands,
//! targets, motions and counts on the worker's battle actor (its index in
//! the worker's battle id byte, `0xff` for none), calling the battle
//! module (`MsGetChr`, `MsGetChrBit`, `MsGetChrStatInfo`, ...).

// w, a, s: the worker, the args words and the stack, as everywhere
#![allow(clippy::many_single_char_names)]

use ffx_bridge::{Global, x87};

use super::native::Native;
use super::run::NativeHost;
use super::{atel_get_battle_id_adrs2, atel_pop_stack_float, atel_pop_stack_integer};

fn pop(h: &mut impl NativeHost, s: u32) -> u32 {
    atel_pop_stack_integer(h, s)
}

fn popf(h: &mut impl NativeHost, s: u32) -> u32 {
    atel_pop_stack_float(h, s)
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

/// `MsGetChrStatInfo(actor, property, selector, &out)` (`0x007b2dc0`).
const GET_STAT: u32 = 0x007b_2dc0;
/// `MsGetChrBit(actor, selector, ...)` (`0x00794330`): a bitmask of actors.
const GET_BIT: u32 = 0x0079_4330;
/// `0x007947f0(actor, selector, 0, 1)`: one actor's index, negative for none.
const GET_ONE: u32 = 0x0079_47f0;
/// `MsGetBattleScene()` (`0x00782970`).
const BATTLE_SCENE: u32 = 0x0078_2970;

/// An out word: zeroed scratch (the original's stack local, whose value
/// when the callee writes nothing is whatever the stack held).
fn out(h: &mut impl NativeHost) -> u32 {
    let o = h.scratch();
    h.write(o, 0_u32);
    o
}

/// `g_011363b8` (`0x011363b8`): set by `Native_SetActorsProperty` for
/// properties `0x54` and `0xed` in battle scene `0xd40000` on graphics
/// name hash `0x200f`.
pub const ACTOR_PROPERTY_LATCH: Global<u32> = Global::new(0x0113_63b8);

/// `Native_SetActorsProperty` (`0x007a50c0`, `0x7018`): `(selector,
/// property, value)` -> `MsSetChrStatInfo(actor, property, selector,
/// value)`'s `EAX`.
pub fn native_set_actors_property<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let p = pop(h, s);
    let sel = pop(h, s);
    let id = actor(h, w);
    if call(h, BATTLE_SCENE, &[]) == 0x00d4_0000
        && call(h, 0x0064_1da0, &[]) == 0x200f
        && (p == 0x54 || p == 0xed)
    {
        ACTOR_PROPERTY_LATCH.set(h, 1);
    }
    call(h, 0x007b_4b70, &[id, p, sel, v])
}

/// `Native_SetSelfFloatProperty` (`0x007a5930`, `0x70b2`): `(property,
/// value f)` -> `MsSetChrMotionInfo(actor, property, value)`; 0.
pub fn native_set_self_float_property<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let f = popf(h, s);
    let i = pop(h, s);
    let id = actor(h, w);
    call(h, 0x007b_4a90, &[id, i, f]);
    0
}

/// `Native_GetActorProperty` (`0x007a4d50`, `0x700f`): `(selector,
/// property)` -> `MsGetChrStatInfo(actor, property, selector)`.
pub fn native_get_actor_property<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let p = pop(h, s);
    let sel = pop(h, s);
    let id = actor(h, w);
    let o = out(h);
    call(h, GET_STAT, &[id, p, sel, o]);
    h.read(o)
}

/// `Native_GetSelfProperty` (`0x007a4ea0`, `0x70aa`): `(property)` -> the
/// same with selector -13 (self).
pub fn native_get_self_property<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let p = pop(h, s);
    let id = actor(h, w);
    let o = out(h);
    call(h, GET_STAT, &[id, p, 0xffff_fff3, o]);
    h.read(o)
}

/// `Native_BtlUseCommand` (`0x007a44b0`, `0x700b`): `(targets, command)`:
/// when the actor can act (`MsAutoCommandStatusCheck`) and `0x0112a8fa` is
/// clear, `0x007ac9c0(actor, command, MsGetChrBit(actor, targets, 0, 1, 0,
/// -1))`, 1; else 0.
pub fn native_btl_use_command<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let cmd = pop(h, s);
    let t = pop(h, s);
    let id = actor(h, w);
    if id == 0xff {
        return 0;
    }
    let c = chr(h, id);
    let k = u32::from(h.read::<u8>(c + 0x5cb) == 0);
    if call(h, 0x007b_24a0, &[c, 0, k, 1]) == 0 {
        return 0;
    }
    if h.read::<u8>(h.rebase(0x0112_a8fa)) != 0 {
        return 0;
    }
    let b = call(h, GET_BIT, &[id, t, 0, 1, 0, u32::MAX]);
    call(h, 0x007a_c9c0, &[id, cmd, b]);
    1
}

/// `Native_ActorWaitMotion` (`0x007a3980`, `0x7008`, poll): 5 without an
/// actor or a model, 0 while `+0x3f3` is set, else `0x007a8c50(model,
/// actor, 5)`.
pub fn native_actor_wait_motion<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = actor(h, w);
    if id == 0xff {
        return 5;
    }
    let c = chr(h, id);
    let m = h.read::<u32>(c);
    if h.read::<u8>(c + 0x3f3) != 0 {
        return 0;
    }
    if m == 0 {
        return 5;
    }
    call(h, 0x007a_8c50, &[m, c, 5])
}

/// `Native_BtlSelectTarget` (`0x007a5af0`, `0x7010`): `(a, b, c, d)`: three
/// rounds of `0x007acd40` (the last with `&out`) and `0x00796430`; `out`.
pub fn native_btl_select_target<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let p1 = pop(h, s);
    let p2 = pop(h, s);
    let p3 = pop(h, s);
    let p4 = pop(h, s);
    let id = actor(h, w);
    let r1 = call(h, 0x007a_cd40, &[id, p4, 4, 0, 5, 0, 0, 0]);
    let r2 = call(h, 0x007a_cd40, &[id, r1, 0x52, 0, 5, 0, 1, 0]);
    let o = out(h);
    let r3 = call(h, 0x007a_cd40, &[id, r2, p3, p2, 4, p1, 1, o]);
    call(h, 0x0079_6430, &[id, r3]);
    h.read(o)
}

/// `g_012fb7e0` (`0x012fb7e0`), see `native_cam::BTL_RANDOM_LATCH`.
const BTL_RANDOM_LATCH: Global<u32> = Global::new(0x012f_b7e0);

/// `Native_BtlCountActorsInBoth` (`0x007a6870`, `0x701e`): `(a, b)`: how
/// many actors both selectors pick; with magic `0x92`, the latch armed and
/// none, in scenes `0x235`/`0x236`/`0x238` the latch is cleared.
pub fn native_btl_count_actors_in_both<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let a1 = pop(h, s);
    let a2 = pop(h, s);
    let id = actor(h, w);
    let m1 = call(h, GET_BIT, &[id, a2, 0, 1]);
    let m2 = call(h, GET_BIT, &[id, a1, 0, 1]);
    let n = (0..31).filter(|i| (m1 >> i) & (m2 >> i) & 1 != 0).count();
    let n = u32::try_from(n).unwrap_or(0);
    if call(h, 0x009d_a3b0, &[]) == 0x92 && BTL_RANDOM_LATCH.get(h) == 1 && n == 0 {
        let sc = call(h, BATTLE_SCENE, &[]);
        if matches!(sc, 0x0235_0000 | 0x0236_0000 | 0x0238_0000) {
            BTL_RANDOM_LATCH.set(h, 0);
        }
    }
    n
}

/// `Native_ActorGetSlotRef` (`0x007a6b40`, `0x70a1`): `(selector)`: the
/// picked actor's `+0xc` when `0x0079aef0` accepts it, else `-6 - slot` for
/// its slot in the 18 bytes at `0x0112c895`, else `0xff`.
pub fn native_actor_get_slot_ref<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let p = pop(h, s);
    let id = actor(h, w);
    let r = call(h, GET_ONE, &[id, p, 0, 1]);
    if r.cast_signed() < 0 {
        return 0xff;
    }
    if call(h, 0x0079_aef0, &[r]) != 0 {
        let c = chr(h, r);
        return u32::from(h.read::<u8>(c + 0xc));
    }
    let t = h.rebase(0x0112_c895);
    for i in 0..0x12 {
        if u32::from(h.read::<u8>(t + i)) == r {
            return 0xffff_fffa_u32.wrapping_sub(i);
        }
    }
    0xff
}

/// `Native_7002` (`0x007a3530`, `0x7002`, start): `(a, b)` ->
/// `0x00781c90(a, 1, b)`'s `EAX`.
pub fn native_7002<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let b = pop(h, s);
    let a = pop(h, s);
    call(h, 0x0078_1c90, &[a, 1, b])
}

/// `Native_BtlRunEvent` (`0x007a5df0`, `0x703f`, start): `(event, a)`:
/// `0x00797bc0(max(event - 1, 0), 0xff, a, 0xffff, 1, 2, worker, &out)`,
/// `out` into the args word and returned.
pub fn native_btl_run_event<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    let x = pop(h, s);
    let e = pop(h, s).wrapping_sub(1);
    let e = if e.cast_signed() < 0 { 0 } else { e };
    let o = out(h);
    call(h, 0x0079_7bc0, &[e, 0xff, x, 0xffff, 1, 2, w, o]);
    let v = h.read::<u32>(o);
    h.write(a, v);
    v
}

/// `Native_BtlRunEvent_Poll` (`0x007a5ec0`; also `0x7097`'s `0x007a58c0`):
/// `0x007a8700(args)` (the args words' address).
pub fn native_btl_run_event_poll<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    call(h, 0x007a_8700, &[a])
}

/// `Native_ActorQueueMotion` (`0x007a2a20`, `0x7016`): `(motion)`: unless
/// the actor is busy (`+0x3f3`), `0x007ab6c0(actor)` and `+0x3f2` =
/// motion; 0.
pub fn native_actor_queue_motion<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let id = actor(h, w);
    let c = chr(h, id);
    if id != 0xff && h.read::<u8>(c + 0x3f3) == 0 {
        call(h, 0x007a_b6c0, &[c]);
        #[allow(clippy::cast_possible_truncation)] // the original stores BL
        h.write(c + 0x3f2, v as u8);
    }
    0
}

/// `Native_70ac` (`0x007a5650`, `0x70ac`, float): `(i)` ->
/// `0x007b2c90(actor, i)` (a float property).
pub fn native_70ac<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let v = pop(h, s);
    let id = actor(h, w);
    callf(h, 0x007b_2c90, &[id, v])
}

/// `Native_700c` (`0x007a4310`, `0x700c`): `0x007a9150(worker, args,
/// stack, 0.0)`; 0.
pub fn native_700c<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    call(h, 0x007a_9150, &[w, a, s, x87::store(0.0)]);
    0
}

/// `Native_7085` (`0x007a43a0`, `0x7085`): as `0x700c`, the returned
/// record's `+0x418` = 1; 0.
pub fn native_7085<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    let r = call(h, 0x007a_9150, &[w, a, s, x87::store(0.0)]);
    if r != 0 {
        h.write(r + 0x418, 1_u8);
    }
    0
}

/// `Native_7000` (`0x007a8620`, `0x7000`): the actor's `+0xdf2` = 0; 0.
pub fn native_7000<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = actor(h, w);
    if id != 0xff {
        let c = chr(h, id);
        h.write(c + 0xdf2, 0_u8);
    }
    0
}

/// `Native_BtlGetBattleId` (`0x007a4e90`, `0x7024`): `MsGetBattleScene()`.
pub fn native_btl_get_battle_id<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call(h, BATTLE_SCENE, &[])
}

/// `Native_7006` (`0x007a3f50`, `0x7006`): `(a, b)`: with the actor's
/// `+0x41d` set, `0x00795eb0(actor, 1, 0, -1, b, a)`; 0.
pub fn native_7006<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    let id = actor(h, w);
    let c = chr(h, id);
    if h.read::<u8>(c + 0x41d) != 0 {
        call(h, 0x0079_5eb0, &[id, 1, 0, u32::MAX, b, a]);
    }
    0
}

/// `Native_7003` (`0x007a3d90`, `0x7003`): `(a, b)` ->
/// `0x00795eb0(actor, 2, 0, -1, b, a)`; 0.
pub fn native_7003<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    let id = actor(h, w);
    call(h, 0x0079_5eb0, &[id, 2, 0, u32::MAX, b, a]);
    0
}

/// `Native_707b` (`0x007a7470`, `0x707b`): `(selector, k)`: for the picked
/// actor, `0x0079fd40(it, k)`'s `EAX` (for `k` 9 only when `0x007a8920(it)`
/// is not 0, else 0); 0 for none.
pub fn native_707b<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let k = pop(h, s);
    let p = pop(h, s);
    let id = actor(h, w);
    let i = call(h, GET_ONE, &[id, p, 0, 1]);
    if i.cast_signed() < 0 {
        return 0;
    }
    if k == 9 {
        let r = call(h, 0x007a_8920, &[i]);
        if r == 0 {
            return r;
        }
    }
    call(h, 0x0079_fd40, &[i, k])
}

/// `Native_7048` (`0x007a53f0`, `0x7048`): `(i, x, y, z)` floats ->
/// `0x007ab0b0(i, 0, 0, &[x, y, z])`; 0.
pub fn native_7048<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let z = popf(h, s);
    let y = popf(h, s);
    let x = popf(h, s);
    let i = pop(h, s);
    let v = h.scratch();
    h.write(v, x);
    h.write(v + 4, y);
    h.write(v + 8, z);
    h.write(v + 12, 0_u32);
    call(h, 0x007a_b0b0, &[i, 0, 0, v]);
    0
}

/// `Native_ActorGetId` (`0x007a6a80`, `0x7021`): `(selector)`: the picked
/// actor's id (`+0xe`), -1 for none.
pub fn native_actor_get_id<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let p = pop(h, s);
    let id = actor(h, w);
    let i = call(h, GET_ONE, &[id, p, 0, 1]);
    if i.cast_signed() < 0 {
        return u32::MAX;
    }
    let c = chr(h, i);
    u32::from(h.read::<u16>(c + 0xe))
}

/// `g_011363e8` (`0x011363e8`): set by `Native_70a2`'s start.
pub const NATIVE_70A2_FLAG: Global<u32> = Global::new(0x0113_63e8);

/// `Native_70a2` (`0x007a7440`, `0x70a2`, start): `(v)`: the flag set, `v`
/// into the args word; `0x0079ccf0(v)`'s `EAX`.
pub fn native_70a2<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    NATIVE_70A2_FLAG.set(h, 1);
    let v = pop(h, s);
    h.write(a, v);
    call(h, 0x0079_ccf0, &[v])
}

/// `Native_702b` (`0x007a3c50`, `0x702b`): -1 when the actor's model is
/// busy (`0x008392c0`), else 0.
pub fn native_702b<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = actor(h, w);
    if id == 0xff {
        return 0;
    }
    let c = chr(h, id);
    let m = h.read::<u32>(c);
    if m == 0 || call(h, 0x0083_92c0, &[m]) == 0 {
        0
    } else {
        u32::MAX
    }
}

/// `Native_ActorSetFlag36` (`0x007a51e0`, `0x705d`): two words popped and
/// dropped; the actor's `+0x36` = 1; the actor index.
pub fn native_actor_set_flag36<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    pop(h, s);
    pop(h, s);
    let id = actor(h, w);
    let c = chr(h, id);
    h.write(c + 0x36, 1_u8);
    id
}

/// `MsAtelRequestSystem(v, 2, worker, &out)` once at a time: the guard
/// (`0x0112a8e4`) takes `mark` and drops back to 0 when the request fails;
/// `out` into the args word and returned.
fn request_system<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32, mark: u8) -> u32 {
    let v = pop(h, s);
    let o = out(h);
    let g = h.rebase(0x0112_a8e4);
    if h.read::<u8>(g) == 0 {
        h.write(g, mark);
        let r = call(h, 0x0079_81e0, &[v, 2, w, o]);
        if r.cast_signed() < 0 {
            h.write(g, 0_u8);
        }
    }
    let x = h.read::<u32>(o);
    h.write(a, x);
    x
}

/// `Native_7097` (`0x007a5780`, `0x7097`, start): `(v)`: [`request_system`]
/// with the guard value 1.
pub fn native_7097<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    request_system(h, w, a, s, 1)
}

/// `Native_7026` (`0x007a5d80`, `0x7026`): `(v)`: the actor's `+0x44f` =
/// `v`; 0.
pub fn native_7026<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let id = actor(h, w);
    if id != 0xff {
        let c = chr(h, id);
        #[allow(clippy::cast_possible_truncation)] // the original stores BL
        h.write(c + 0x44f, v as u8);
    }
    0
}

/// `Native_702e` (`0x007a6d70`, `0x702e`, float): `(v)` ->
/// `0x007a9a40(actor, v, 0)`, 0.0 without an actor.
pub fn native_702e<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let v = pop(h, s);
    let id = actor(h, w);
    if id == 0xff {
        return 0.0;
    }
    callf(h, 0x007a_9a40, &[id, v, 0])
}

/// `Native_702d` (`0x007a4e60`, `0x702d`): `0x007ab730(actor record)`; 0.
pub fn native_702d<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = actor(h, w);
    if id != 0xff {
        let c = chr(h, id);
        call(h, 0x007a_b730, &[c]);
    }
    0
}

/// `Native_7019` (`0x007a62d0`, `0x7019`): `0x007acc90(actor)`, 0 without one.
pub fn native_7019<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = actor(h, w);
    if id == 0xff {
        0
    } else {
        call(h, 0x007a_cc90, &[id])
    }
}

/// `Native_704a` (`0x007a58d0`, `0x704a`): `(selector)`: how many picked
/// actors have `+0x415` set.
pub fn native_704a<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let id = actor(h, w);
    let p = pop(h, s);
    let m = call(h, GET_BIT, &[id, p, 0, 1]);
    let mut n = 0;
    for i in 0..31u32 {
        if (m >> i) & 1 != 0 {
            let c = chr(h, i);
            if h.read::<u8>(c + 0x415) != 0 {
                n += 1;
            }
        }
    }
    n
}

/// `Native_701d` (`0x007a5bf0`, `0x701d`): the actor's `+0x420` (signed),
/// 0 without one.
pub fn native_701d<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = actor(h, w);
    if id == 0xff {
        return 0;
    }
    let c = chr(h, id);
    i32::from(h.read::<i8>(c + 0x420)).cast_unsigned()
}

/// `Native_700a` (`0x007a4100`, `0x700a`): `(b, f)`: the actor's `+0x538`
/// = f, `+0x50c` = b, `0x007ab300(actor)`; 0.
pub fn native_700a<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let f = popf(h, s);
    let b = pop(h, s);
    let id = actor(h, w);
    if id != 0xff {
        let c = chr(h, id);
        h.write(c + 0x538, x87::store(x87::load(f)));
        #[allow(clippy::cast_possible_truncation)] // the original stores BL
        h.write(c + 0x50c, b as u8);
        call(h, 0x007a_b300, &[id]);
    }
    0
}

/// `Native_701a` (`0x007a6390`, `0x701a`): `(a, b)` ->
/// `0x0078cd30(actor, a, b)`'s `EAX`.
pub fn native_701a<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let id = actor(h, w);
    let b = pop(h, s);
    let a = pop(h, s);
    call(h, 0x0078_cd30, &[id, a, b])
}

/// `Native_7037` (`0x007a6d30`, `0x7037`): `(b, a)` ->
/// `0x007ac840(actor, a, b, 1)`'s `EAX`.
pub fn native_7037<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    let id = actor(h, w);
    call(h, 0x007a_c840, &[id, a, b, 1])
}

/// `Native_706f` (`0x007a36c0`, `0x706f`): `0x007a8fc0(worker, args,
/// stack, 1)`'s `EAX`.
pub fn native_706f<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    call(h, 0x007a_8fc0, &[w, a, s, 1])
}

/// `Native_7104` (`0x007a6600`, `0x7104`): `(a, b, c)` -> `0x0079d2a0(a, b,
/// c)`'s `EAX`.
pub fn native_7104<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let c = pop(h, s);
    let b = pop(h, s);
    let a = pop(h, s);
    call(h, 0x0079_d2a0, &[a, b, c])
}

/// `Native_7009` (`0x007a3b20`, `0x7009`): `(b)`: `0x00796070(actor, b & 1
/// == 0)`; 0.
pub fn native_7009<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let b = pop(h, s) & 1;
    let id = actor(h, w);
    if id != 0xff {
        chr(h, id);
        call(h, 0x0079_6070, &[id, u32::from(b == 0)]);
    }
    0
}

/// `Native_7058` (`0x007a8160`, `0x7058`): `(selector, motion)`: 0 when the
/// picked actor's motion (`+0x3f0`) is `0x007a8c80(it, motion)`, else -1.
pub fn native_7058<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let m = pop(h, s);
    let p = pop(h, s);
    let id = actor(h, w);
    let i = call(h, GET_ONE, &[id, p, 0, 1]);
    if i.cast_signed() < 0 {
        return u32::MAX;
    }
    let c = chr(h, i);
    let r = call(h, 0x007a_8c80, &[c, m]);
    if u32::from(h.read::<u8>(c + 0x3f0)) == r {
        0
    } else {
        u32::MAX
    }
}

/// `Native_70a8` (`0x007a57e0`, `0x70a8`): `(selector, property, value
/// f)` -> `MsSetChrMotionInfo(picked, property, value)`; 0.
pub fn native_70a8<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let f = popf(h, s);
    let k = pop(h, s);
    let p = pop(h, s);
    let id = actor(h, w);
    let i = call(h, GET_ONE, &[id, p, 0, 1]);
    call(h, 0x007b_4a90, &[i, k, f]);
    0
}

/// `Native_706b` (`0x007a84c0`, `0x706b`): `(selector, a, b)`: on the
/// picked actor's model, `0x0082aea0(model, a, b & 1)`; 0.
pub fn native_706b<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let b = pop(h, s);
    let a = pop(h, s);
    let p = pop(h, s);
    let id = actor(h, w);
    let i = call(h, GET_ONE, &[id, p, 0, 1]);
    if i.cast_signed() >= 0 {
        let c = chr(h, i);
        let m = h.read::<u32>(c);
        if m != 0 {
            call(h, 0x0082_aea0, &[m, a, b & 1]);
        }
    }
    0
}

/// A byte of the worker's actor set from the popped word: `(v)`; 0.
fn set_byte<H: NativeHost>(h: &mut H, w: u32, s: u32, off: u32, mask: u32) -> u32 {
    let v = pop(h, s) & mask;
    let id = actor(h, w);
    if id != 0xff {
        let c = chr(h, id);
        #[allow(clippy::cast_possible_truncation)] // the original stores BL
        h.write(c + off, v as u8);
    }
    0
}

/// `Native_7029` (`0x007a3fb0`, `0x7029`): `(b)`: `+0x505` = b & 1; 0.
pub fn native_7029<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    set_byte(h, w, s, 0x505, 1)
}

/// `Native_701c` (`0x007a59d0`, `0x701c`): `(v)`: `+0x420` = v; 0.
pub fn native_701c<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    set_byte(h, w, s, 0x420, 0xff)
}

/// `Native_7033` (`0x007a3cc0`, `0x7033`): `(b)`: `+0x32` = b & 1; 0.
pub fn native_7033<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    set_byte(h, w, s, 0x32, 1)
}

/// `Native_70e0` (`0x007a8450`, `0x70e0`): 1 when the actor can act and its
/// partner (`+0xded`) has `+0xdf1` clear, else 0.
pub fn native_70e0<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = actor(h, w);
    if id == 0xff {
        return 0;
    }
    let c = chr(h, id);
    let t = i32::from(h.read::<i8>(c + 0xded)).cast_unsigned();
    let d = chr(h, t);
    u32::from(call(h, 0x007b_24a0, &[c, 0, 0, 1]) != 0 && h.read::<u8>(d + 0xdf1) == 0)
}

/// `Native_703c` (`0x007a5540`, `0x703c`, start): as [`native_7097`] with
/// the guard value 2.
pub fn native_703c<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    request_system(h, w, a, s, 2)
}

/// `Native_703b` (`0x007a6ef0`, `0x703b`): `(selector, a, b)`:
/// `0x0079b470(i, a, b)` for each picked actor; how many.
pub fn native_703b<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let b = pop(h, s);
    let a = pop(h, s);
    let p = pop(h, s);
    let id = actor(h, w);
    let m = call(h, GET_BIT, &[id, p, 0, 1]);
    let mut n = 0;
    for i in 0..31u32 {
        if (m >> i) & 1 != 0 {
            call(h, 0x0079_b470, &[i, a, b]);
            n += 1;
        }
    }
    n
}

/// `(selector, k)` -> `f(picked, k)`'s `EAX`, 0 for none.
fn picked_call<H: NativeHost>(h: &mut H, w: u32, s: u32, f: u32) -> u32 {
    let k = pop(h, s);
    let p = pop(h, s);
    let id = actor(h, w);
    let i = call(h, GET_ONE, &[id, p, 0, 1]);
    if i.cast_signed() < 0 {
        0
    } else {
        call(h, f, &[i, k])
    }
}

/// `Native_70c9` (`0x007a7740`, `0x70c9`): `0x0079fdf0(picked, k)`.
pub fn native_70c9<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    picked_call(h, w, s, 0x0079_fdf0)
}

/// `Native_709e` (`0x007a2fe0`, `0x709e`): `(selector, a, b)` ->
/// `0x007b74d0(picked, a, b)`.
pub fn native_709e<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let id = actor(h, w);
    let b = pop(h, s);
    let a = pop(h, s);
    let p = pop(h, s);
    let i = call(h, GET_ONE, &[id, p, 0, 1]);
    call(h, 0x007b_74d0, &[i, a, b])
}

/// `Native_707a` (`0x007a2a80`, `0x707a`): `(selector)` ->
/// `0x0079a920(picked, its selector -16 pick, 0, 0, 0)`.
pub fn native_707a<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let p = pop(h, s);
    let id = actor(h, w);
    let i = call(h, GET_ONE, &[id, p, 0, 1]);
    let j = call(h, GET_ONE, &[i, 0xffff_fff0, 0, 1]);
    call(h, 0x0079_a920, &[i, j, 0, 0, 0])
}

/// `Native_705a` (`0x007a49f0`, `0x705a`): `(targets, command)` as
/// `Native_BtlUseCommand` without the checks (`MsGetChrBit(.., 0, 1, 1,
/// -1)`); 1, 0 without an actor.
pub fn native_705a<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let cmd = pop(h, s);
    let t = pop(h, s);
    let id = actor(h, w);
    if id == 0xff {
        return 0;
    }
    let b = call(h, GET_BIT, &[id, t, 0, 1, 1, u32::MAX]);
    call(h, 0x007a_c9c0, &[id, cmd, b]);
    1
}

/// `Native_7017` (`0x007a4050`, `0x7017`): `(a, b)`: when
/// `0x0079eb10(actor, a, 7, b, 1)` holds, the actor's `+0xe12` counts up; 0.
pub fn native_7017<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let b = pop(h, s);
    let a = pop(h, s);
    let id = actor(h, w);
    if call(h, 0x0079_eb10, &[id, a, 7, b, 1]) != 0 {
        let c = chr(h, id);
        let n = h.read::<u8>(c + 0xe12);
        h.write(c + 0xe12, n.wrapping_add(1));
    }
    0
}

/// `Native_7059` (`0x007a6e40`, `0x7059`, float): as `0x702e` with 1 for
/// the third argument.
pub fn native_7059<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let v = pop(h, s);
    let id = actor(h, w);
    if id == 0xff {
        return 0.0;
    }
    callf(h, 0x007a_9a40, &[id, v, 1])
}

/// `Native_7039` (`0x007a2b10`, `0x7039`): `0x0078cca0(actor, record)`
/// with an actor, then `0x007a8f20()`; 0.
pub fn native_7039<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = actor(h, w);
    let c = chr(h, id);
    if id != 0xff {
        call(h, 0x0078_cca0, &[id, c]);
    }
    call(h, 0x007a_8f20, &[]);
    0
}

/// `Native_DebugPrint` (`0x007a4910`, `0x7015`): `(v)`:
/// `dbgPrintf(0x00b54e44, v, v, actor)`; 0.
pub fn native_debug_print<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let id = actor(h, w);
    let v = pop(h, s);
    let f = h.rebase(0x00b5_4e44);
    call(h, 0x0062_f500, &[f, v, v, id]);
    0
}

/// `Native_7012` (`0x007a29a0`, `0x7012`): `+0x435` = 0 and, for state
/// (`+0xdfb`) 2, the state 3, else `+0x432` = 0; 0.
pub fn native_7012<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = actor(h, w);
    if id != 0xff {
        let c = chr(h, id);
        let k = h.read::<i8>(c + 0xdfb);
        h.write(c + 0x435, 0_u8);
        if k == 2 {
            h.write(c + 0xdfb, 3_u8);
        } else {
            h.write(c + 0x432, 0_u8);
        }
    }
    0
}

/// `Native_7086` (`0x007a2ce0`, `0x7086`): `0x0078e5a0(actor)`; 0.
pub fn native_7086<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = actor(h, w);
    call(h, 0x0078_e5a0, &[id]);
    0
}

/// `Native_7055` (`0x007a3eb0`, `0x7055`): `(b)`: `0x00796f20(record, 0, b
/// & 1)`; 0.
pub fn native_7055<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let b = pop(h, s) & 1;
    let id = actor(h, w);
    if id != 0xff {
        let c = chr(h, id);
        call(h, 0x0079_6f20, &[c, 0, b]);
    }
    0
}

/// `Native_7038` (`0x007a6dd0`, `0x7038`): `(b, a)` ->
/// `0x007acb40(actor, a, b)`.
pub fn native_7038<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    let id = actor(h, w);
    call(h, 0x007a_cb40, &[id, a, b])
}

/// `Native_7108` (`0x007a40a0`, `0x7108`): `(a, b)`: with `+0x41d` set,
/// `MsSetChrDirTarget(actor, 1, 1, -1, b, a)`; 0.
pub fn native_7108<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    let id = actor(h, w);
    let c = chr(h, id);
    if h.read::<u8>(c + 0x41d) != 0 {
        call(h, 0x0079_5eb0, &[id, 1, 1, u32::MAX, b, a]);
    }
    0
}

/// `Native_70e2` (`0x007a32a0`, `0x70e2`): `(selector, v)`: on the picked
/// actor's model, `0x0083d040(model, 4, v)`; 0.
pub fn native_70e2<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let p = pop(h, s);
    let id = actor(h, w);
    let i = call(h, GET_ONE, &[id, p, 0, 1]);
    let c = chr(h, i);
    let m = h.read::<u32>(c);
    if m != 0 {
        call(h, 0x0083_d040, &[m, 4, v]);
    }
    0
}

/// `Native_7047` (`0x007a5270`, `0x7047`): `(i, a)` ->
/// `0x007ab0b0(i, actor, a, 0)`; 0.
pub fn native_7047<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let id = actor(h, w);
    let a = pop(h, s);
    let i = pop(h, s);
    call(h, 0x007a_b0b0, &[i, id, a, 0]);
    0
}

/// `Native_7093` (`0x007a60f0`, `0x7093`): `0x00791780(actor, record)`, 0
/// without one.
pub fn native_7093<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = actor(h, w);
    if id == 0xff {
        return 0;
    }
    let c = chr(h, id);
    call(h, 0x0079_1780, &[id, c])
}

/// `Native_708f` (`0x007a6700`, `0x708f`): `(v)` -> `0x007a8720(actor, v,
/// 1)`.
pub fn native_708f<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let id = actor(h, w);
    call(h, 0x007a_8720, &[id, v, 1])
}

/// `Native_708c` (`0x007a3920`, `0x708c`): `(a, b)` -> `0x00782ba0(a, b)`.
pub fn native_708c<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let b = pop(h, s);
    let a = pop(h, s);
    call(h, 0x0078_2ba0, &[a, b])
}

/// `Native_70b5` (`0x007a2f40`, `0x70b5`): `+0x400` = `0xffff`; 0.
pub fn native_70b5<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = actor(h, w);
    if id != 0xff {
        let c = chr(h, id);
        h.write(c + 0x400, 0xffff_u16);
    }
    0
}

/// `Native_7031` (`0x007a2d00`, `0x7031`): `+0x432` = 1; 0.
pub fn native_7031<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = actor(h, w);
    let c = chr(h, id);
    if id != 0xff {
        h.write(c + 0x432, 1_u8);
    }
    0
}

/// `Native_7030` (`0x007a5090`, `0x7030`): `+0x412`, 0 without an actor.
pub fn native_7030<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = actor(h, w);
    if id == 0xff {
        return 0;
    }
    let c = chr(h, id);
    u32::from(h.read::<u8>(c + 0x412))
}

/// `Native_702a` (`0x007a5bc0`, `0x702a`): `+0x508` (signed), 0 without an
/// actor.
pub fn native_702a<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = actor(h, w);
    let c = chr(h, id);
    if id == 0xff {
        return 0;
    }
    i32::from(h.read::<i8>(c + 0x508)).cast_unsigned()
}

/// `Native_7041` (`0x007a56d0`, `0x7041`): nothing; 0.
pub fn native_7041<H: NativeHost>(_h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    0
}

/// `Native_707f` (`0x007a5e50`, `0x707f`): `(selector)` ->
/// `0x007ab2a0(picked)`; 0.
pub fn native_707f<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let id = actor(h, w);
    let p = pop(h, s);
    let i = call(h, GET_ONE, &[id, p, 0, 1]);
    call(h, 0x007a_b2a0, &[i]);
    0
}

/// `Native_70d5` (`0x007a7fa0`, `0x70d5`): `(selector)`: the picked index
/// into the worker's actor's `+0x6d1`; that index.
pub fn native_70d5<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let p = pop(h, s);
    let id = actor(h, w);
    let i = call(h, GET_ONE, &[id, p, 0, 1]);
    let c = chr(h, id);
    #[allow(clippy::cast_possible_truncation)] // the original stores BL
    h.write(c + 0x6d1, i as u8);
    i
}

/// `Native_7084` (`0x007a4440`, `0x7084`): `0x00788e30(record)`; 0.
pub fn native_7084<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = actor(h, w);
    let c = chr(h, id);
    call(h, 0x0078_8e30, &[c]);
    0
}

/// `Native_7034` (`0x007a5330`, `0x7034`): `(v)`: `0x0112c9f1` = v and,
/// when `0x0112a8e0` is 1, it becomes 2; `v`.
pub fn native_7034<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let g = h.rebase(0x0112_a8e0);
    let k = h.read::<u8>(g);
    #[allow(clippy::cast_possible_truncation)] // the original stores AL
    h.write(h.rebase(0x0112_c9f1), v as u8);
    if k == 1 {
        h.write(g, 2_u8);
    }
    v
}

/// `Native_7051` (`0x007a5890`, `0x7051`, poll): 5 once the actor's
/// `+0xe12` is 0.
pub fn native_7051<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = actor(h, w);
    let c = chr(h, id);
    if h.read::<u8>(c + 0xe12) == 0 { 5 } else { 0 }
}

/// `Native_7062` (`0x007a4580`, `0x7062`): `(a, b)` ->
/// `0x0079eba0(actor, a, b, 1)`.
pub fn native_7062<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let b = pop(h, s);
    let a = pop(h, s);
    let id = actor(h, w);
    call(h, 0x0079_eba0, &[id, a, b, 1])
}

/// The actor's `+0xdc0` bits with a selector's set: `(selector)`; the set.
fn bits_dc0<H: NativeHost>(h: &mut H, w: u32, s: u32, op: fn(u32, u32) -> u32) -> u32 {
    let id = actor(h, w);
    let p = pop(h, s);
    let m = call(h, GET_BIT, &[id, p, 0, 1]);
    let c = chr(h, id);
    let v = h.read::<u32>(c + 0xdc0);
    h.write(c + 0xdc0, op(v, m));
    m
}

/// `Native_70cd` (`0x007a7d60`, `0x70cd`): `+0xdc0` |= the set.
pub fn native_70cd<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    bits_dc0(h, w, s, |v, m| v | m)
}

/// `Native_70cc` (`0x007a7cb0`, `0x70cc`): `+0xdc0` = the set.
pub fn native_70cc<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    bits_dc0(h, w, s, |_, m| m)
}

/// `Native_70ce` (`0x007a7f20`, `0x70ce`): `+0xdc0` &= !the set.
pub fn native_70ce<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    bits_dc0(h, w, s, |v, m| v & !m)
}

/// `Native_70a5` (`0x007a76f0`, `0x70a5`): `(x, y, z)` floats into
/// `0x0112a8d0`..`0x0112a8d8` (z first), `0x0112a8dc` = 1.0,
/// `0x0112c9fd` = 1; 0.
pub fn native_70a5<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    h.write(h.rebase(0x0112_a8dc), x87::store(1.0));
    for o in [0x0112_a8d8, 0x0112_a8d4, 0x0112_a8d0] {
        let v = popf(h, s);
        h.write(h.rebase(o), v);
    }
    h.write(h.rebase(0x0112_c9fd), 1_u8);
    0
}

/// The Rust version of a battle handler, if there is one.
#[must_use]
pub fn find<H: NativeHost>(sym: u32) -> Option<Native<H>> {
    use Native::{Float, Int};
    Some(match sym {
        0x007a_50c0 => Int(native_set_actors_property),
        0x007a_5930 => Int(native_set_self_float_property),
        0x007a_4d50 => Int(native_get_actor_property),
        0x007a_4ea0 => Int(native_get_self_property),
        0x007a_44b0 => Int(native_btl_use_command),
        0x007a_3980 => Int(native_actor_wait_motion),
        0x007a_5af0 => Int(native_btl_select_target),
        0x007a_6870 => Int(native_btl_count_actors_in_both),
        0x007a_6b40 => Int(native_actor_get_slot_ref),
        0x007a_3530 => Int(native_7002),
        0x007a_5df0 => Int(native_btl_run_event),
        0x007a_5ec0 | 0x007a_58c0 | 0x007a_5680 => Int(native_btl_run_event_poll),
        0x007a_2a20 => Int(native_actor_queue_motion),
        0x007a_5650 => Float(native_70ac),
        0x007a_4310 => Int(native_700c),
        0x007a_43a0 => Int(native_7085),
        0x007a_8620 => Int(native_7000),
        0x007a_4e90 => Int(native_btl_get_battle_id),
        0x007a_3f50 => Int(native_7006),
        0x007a_3d90 => Int(native_7003),
        0x007a_7470 => Int(native_707b),
        0x007a_53f0 => Int(native_7048),
        0x007a_6a80 => Int(native_actor_get_id),
        0x007a_7440 => Int(native_70a2),
        0x007a_3c50 => Int(native_702b),
        0x007a_51e0 => Int(native_actor_set_flag36),
        0x007a_5780 => Int(native_7097),
        0x007a_5d80 => Int(native_7026),
        0x007a_6d70 => Float(native_702e),
        0x007a_4e60 => Int(native_702d),
        0x007a_62d0 => Int(native_7019),
        0x007a_58d0 => Int(native_704a),
        0x007a_5bf0 => Int(native_701d),
        0x007a_4100 => Int(native_700a),
        0x007a_6390 => Int(native_701a),
        0x007a_6d30 => Int(native_7037),
        0x007a_36c0 => Int(native_706f),
        0x007a_6600 => Int(native_7104),
        0x007a_3b20 => Int(native_7009),
        0x007a_8160 => Int(native_7058),
        0x007a_57e0 => Int(native_70a8),
        0x007a_84c0 => Int(native_706b),
        0x007a_3fb0 => Int(native_7029),
        0x007a_59d0 => Int(native_701c),
        0x007a_3cc0 => Int(native_7033),
        0x007a_8450 => Int(native_70e0),
        0x007a_5540 => Int(native_703c),
        0x007a_6ef0 => Int(native_703b),
        0x007a_7740 => Int(native_70c9),
        0x007a_2fe0 => Int(native_709e),
        0x007a_2a80 => Int(native_707a),
        0x007a_49f0 => Int(native_705a),
        0x007a_4050 => Int(native_7017),
        0x007a_6e40 => Float(native_7059),
        0x007a_2b10 => Int(native_7039),
        0x007a_4910 => Int(native_debug_print),
        0x007a_29a0 => Int(native_7012),
        0x007a_2ce0 => Int(native_7086),
        0x007a_3eb0 => Int(native_7055),
        0x007a_6dd0 => Int(native_7038),
        0x007a_40a0 => Int(native_7108),
        0x007a_32a0 => Int(native_70e2),
        0x007a_5270 => Int(native_7047),
        0x007a_60f0 => Int(native_7093),
        0x007a_6700 => Int(native_708f),
        0x007a_3920 => Int(native_708c),
        0x007a_2f40 => Int(native_70b5),
        0x007a_2d00 => Int(native_7031),
        0x007a_5090 => Int(native_7030),
        0x007a_5bc0 => Int(native_702a),
        0x007a_56d0 => Int(native_7041),
        0x007a_5e50 => Int(native_707f),
        0x007a_7fa0 => Int(native_70d5),
        0x007a_4440 => Int(native_7084),
        0x007a_5330 => Int(native_7034),
        0x007a_5890 => Int(native_7051),
        0x007a_4580 => Int(native_7062),
        0x007a_7d60 => Int(native_70cd),
        0x007a_7cb0 => Int(native_70cc),
        0x007a_7f20 => Int(native_70ce),
        0x007a_76f0 => Int(native_70a5),
        _ => return None,
    })
}

/// Every handler [`find`] has, with whether it returns a float.
pub const PORTED: [(u32, bool); 83] = [
    (0x007a_50c0, false),
    (0x007a_5930, false),
    (0x007a_4d50, false),
    (0x007a_4ea0, false),
    (0x007a_44b0, false),
    (0x007a_3980, false),
    (0x007a_5af0, false),
    (0x007a_6870, false),
    (0x007a_6b40, false),
    (0x007a_3530, false),
    (0x007a_5df0, false),
    (0x007a_5ec0, false),
    (0x007a_58c0, false),
    (0x007a_2a20, false),
    (0x007a_5650, true),
    (0x007a_4310, false),
    (0x007a_43a0, false),
    (0x007a_8620, false),
    (0x007a_4e90, false),
    (0x007a_3f50, false),
    (0x007a_3d90, false),
    (0x007a_7470, false),
    (0x007a_53f0, false),
    (0x007a_6a80, false),
    (0x007a_7440, false),
    (0x007a_3c50, false),
    (0x007a_51e0, false),
    (0x007a_5780, false),
    (0x007a_5d80, false),
    (0x007a_6d70, true),
    (0x007a_4e60, false),
    (0x007a_62d0, false),
    (0x007a_58d0, false),
    (0x007a_5bf0, false),
    (0x007a_4100, false),
    (0x007a_6390, false),
    (0x007a_6d30, false),
    (0x007a_36c0, false),
    (0x007a_6600, false),
    (0x007a_3b20, false),
    (0x007a_8160, false),
    (0x007a_57e0, false),
    (0x007a_84c0, false),
    (0x007a_3fb0, false),
    (0x007a_59d0, false),
    (0x007a_3cc0, false),
    (0x007a_8450, false),
    (0x007a_5540, false),
    (0x007a_6ef0, false),
    (0x007a_7740, false),
    (0x007a_2fe0, false),
    (0x007a_2a80, false),
    (0x007a_49f0, false),
    (0x007a_4050, false),
    (0x007a_6e40, true),
    (0x007a_2b10, false),
    (0x007a_4910, false),
    (0x007a_29a0, false),
    (0x007a_2ce0, false),
    (0x007a_3eb0, false),
    (0x007a_6dd0, false),
    (0x007a_40a0, false),
    (0x007a_32a0, false),
    (0x007a_5270, false),
    (0x007a_60f0, false),
    (0x007a_6700, false),
    (0x007a_3920, false),
    (0x007a_2f40, false),
    (0x007a_2d00, false),
    (0x007a_5090, false),
    (0x007a_5bc0, false),
    (0x007a_56d0, false),
    (0x007a_5e50, false),
    (0x007a_7fa0, false),
    (0x007a_4440, false),
    (0x007a_5330, false),
    (0x007a_5890, false),
    (0x007a_4580, false),
    (0x007a_7d60, false),
    (0x007a_7cb0, false),
    (0x007a_7f20, false),
    (0x007a_76f0, false),
    (0x007a_5680, false),
];
