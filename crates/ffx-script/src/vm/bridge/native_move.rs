//! Movement, turning, worker, character-motion and screen natives in Rust
//! (the starts and setters of the move and turn blocks, the waits by
//! frames, map jumps, fades), most calling the movement module kept in the
//! original.

// w, a, s: the worker, the args words and the stack, as everywhere
#![allow(clippy::many_single_char_names)]

use ffx_bridge::{Global, x87};

use super::native::Native;
use super::native_map::pop_st0;
use super::run::NativeHost;
use super::{
    atel_get_basic_work, atel_get_battle_id_adrs2, atel_get_char_handle, atel_get_pos_work_adrs,
    atel_get_real_var_adrs, atel_pop_stack_float, atel_pop_stack_integer, atel_push_stack_integer,
    script_worker_level,
};
use crate::vm::worker;

fn pop(h: &mut impl NativeHost, s: u32) -> u32 {
    atel_pop_stack_integer(h, s)
}

fn popf(h: &mut impl NativeHost, s: u32) -> u32 {
    atel_pop_stack_float(h, s)
}

fn call(h: &mut impl NativeHost, sym: u32, args: &[u32]) -> u32 {
    let f = h.rebase(sym);
    h.call(f, args)
}

fn callf(h: &mut impl NativeHost, sym: u32, args: &[u32]) -> f64 {
    let f = h.rebase(sym);
    h.call_float(f, args)
}

fn lvl(h: &impl NativeHost, w: u32) -> u32 {
    u32::from(h.read::<u8>(w + worker::LEVEL))
}

/// The worker's move block (`+0x5b8 + level * 0x4c`).
fn move_block(h: &impl NativeHost, w: u32) -> u32 {
    w + 0x5b8 + lvl(h, w) * 0x4c
}

/// `AtelSetMoveHokanMode(worker, 0)` (`0x008703b0`).
const SET_MOVE_HOKAN_MODE: u32 = 0x0087_03b0;
/// `Math_Atan2XZ(x0, z0, x1, z1)` (`0x0088b430`).
const ATAN2_XZ: u32 = 0x0088_b430;
/// `MagicFile_GetMagicId()` (`0x009da3b0`).
const GET_MAGIC_ID: u32 = 0x009d_a3b0;

/// `Native_MoveSetTarget` (`0x008602f0`, `0x0015`): `(x, y, z)` floats:
/// `0x0086f780(worker, x, y, z)` (the move target, with the start at the
/// current position); 0.
pub fn native_move_set_target<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let z = popf(h, s);
    let y = popf(h, s);
    let x = popf(h, s);
    call(h, 0x0086_f780, &[w, x, y, z]);
    0
}

/// `Native_MoveStart` (`0x00856c30`, `0x0018`): `(a, mode, b)`: prepares
/// the move block for the mode (5, 6: `+8` = 0; 7: the start (`+0x1c`) at
/// the current position, `+0x18` and `+8` = 0; 9: the jump arc's `+0x34`
/// and `+0x18` from the height to go, `+8` = 0; others: `+0xc` becomes its
/// reciprocal unless 0), adds `0x800` to the mode when `+0x38` is 0, then
/// `0x008703e0(worker, b, mode, a)`; 0.
#[allow(clippy::similar_names)]
pub fn native_move_start<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let mut mode = pop(h, s);
    let b = pop(h, s);
    let m = move_block(h, w);
    call(h, SET_MOVE_HOKAN_MODE, &[w, 0]);
    let zero = x87::store(0.0);
    match (mode & 0xffff_007f).wrapping_sub(5) {
        0 | 1 => h.write(m + 8, zero),
        2 => {
            let p = atel_get_pos_work_adrs(h, w);
            for i in 0..3 {
                let v = h.read::<u32>(p + i * 4);
                h.write(m + 0x1c + i * 4, x87::store(x87::load(v)));
            }
            h.write(m + 0x18, zero);
            h.write(m + 8, zero);
        }
        4 => {
            let d = x87::store(x87::sub(
                x87::load(h.read(m + 0x2c)),
                x87::load(h.read(m + 0x20)),
            ));
            let d = x87::load(d);
            let v = x87::load(h.read(m + 0x18));
            let t = x87::load(x87::store(if d > 0.0 { x87::mul(v, -1.0) } else { v }));
            let u = x87::load(x87::store(x87::add(x87::sub(d, t), x87::sub(d, t))));
            if u < 0.0 {
                let u2 = x87::load(x87::store(-u));
                let t2 = x87::store(x87::sub(d, x87::mul(0.5, u2)));
                h.write(m + 0x34, x87::store(u2));
                h.write(m + 0x18, t2);
            } else {
                h.write(m + 0x34, x87::store(u));
                h.write(m + 0x18, x87::store(t));
            }
            h.write(m + 8, zero);
        }
        _ => {
            let c = x87::load(h.read(m + 0xc));
            if c != 0.0 {
                h.write(m + 0xc, x87::store(x87::div(1.0, c)));
            }
        }
    }
    if x87::load(h.read(m + 0x38)) == 0.0 {
        mode |= 0x800;
    }
    call(h, 0x0087_03e0, &[w, b, mode, a]);
    0
}

/// `Native_TurnStart` (`0x00857520`, `0x0019`): `(a, mode, b)`: for mode 2
/// (a timed turn) the frames from the turn block's remaining angle
/// (`+0x2c`, truncated; `0x800` takes a full turn off a positive count,
/// `0x400` adds one to a negative), `+8` = 0 and `+0xc` = the absolute
/// value of speed (`+0x10`) times 0.8333 over the frames (0 for no
/// frames); modes 3 and 4 nothing; others
/// `+0xc` becomes its reciprocal unless 0. Then `0x00871030(worker, b,
/// mode, a)`; 0.
pub fn native_turn_start<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let mode = pop(h, s);
    let b = pop(h, s);
    let l = lvl(h, w).cast_signed();
    let t = h.read::<u32>(script_worker_level(h, w, l) + 0x48);
    call(h, SET_MOVE_HOKAN_MODE, &[w, 0]);
    let m = mode & 0xffff_001f;
    if m == 2 {
        let two_pi = x87::load(h.read(h.rebase(0x00b9_230c)));
        let k = h.read::<f64>(h.rebase(0x00b5_bc00));
        let f32_of = |r: u32| x87::load(x87::store(f64::from(r.cast_signed())));
        let mut r = x87::ftol(h, x87::load(h.read(t + 0x2c)));
        if mode & 0x800 != 0 {
            let x = f32_of(r);
            if x > 0.0 {
                r = x87::ftol(h, x87::sub(x, two_pi));
            }
        }
        if mode & 0x400 != 0 {
            let x = f32_of(r);
            if x < 0.0 {
                r = x87::ftol(h, x87::add(two_pi, x));
            }
        }
        h.write(t + 8, x87::store(0.0));
        let y = f32_of(r);
        let z = if y == 0.0 {
            0.0
        } else {
            x87::div(x87::mul(x87::load(h.read(t + 0x10)), k), y)
        };
        h.write(t + 0xc, x87::store(z.abs()));
    } else if !(3..=4).contains(&m) {
        let c = x87::load(h.read(t + 0xc));
        if c != 0.0 {
            h.write(t + 0xc, x87::store(x87::div(1.0, c)));
        }
    }
    call(h, 0x0087_1030, &[w, b, mode, a]);
    0
}

/// A turn-block speed (`+off` of the turn slot `w + 0x864 + level * 0x44`
/// area, as `0x00871560`/`0x00871580`/`0x008715a0` store it): `ST0` is the
/// speed.
fn turn_speed<H: NativeHost>(h: &mut H, w: u32, s: u32, f: u32) -> f64 {
    let v = popf(h, s);
    let l = lvl(h, w);
    callf(h, f, &[w, l, v])
}

/// `Native_TurnSetSpeed2` (`0x0085a5c0`, `0x002f`, float): `(speed)` into
/// the turn block's `+0x18` (`0x00871560`).
pub fn native_turn_set_speed2<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    turn_speed(h, w, s, 0x0087_1560)
}

/// `Native_TurnSetSpeed` (`0x0085a2a0`, `0x002e`, float): `+0x10`
/// (`0x00871580`).
pub fn native_turn_set_speed<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    turn_speed(h, w, s, 0x0087_1580)
}

/// `Native_SetTurnSpeed` (`0x0085a480`, `0x00e3`, float): `+0x14`
/// (`0x008715a0`).
pub fn native_set_turn_speed<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    turn_speed(h, w, s, 0x0087_15a0)
}

/// `(x)` -> `f(worker, x)`; `ST0` is `x`.
fn set_f<H: NativeHost>(h: &mut H, w: u32, s: u32, f: u32) -> f64 {
    let v = popf(h, s);
    call(h, f, &[w, v]);
    x87::load(v)
}

/// `Native_SetMoveSpeed` (`0x00857f40`, `0x006c`, float): `0x0086f350`.
pub fn native_set_move_speed<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    set_f(h, w, s, 0x0086_f350)
}

/// `Native_MoveSetSpeed` (`0x00857da0`, `0x0016`, float): `0x008715c0`.
pub fn native_move_set_speed<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    set_f(h, w, s, 0x0087_15c0)
}

/// `Native_SetArriveRadiusAll` (`0x00858020`, `0x002d`, float):
/// `0x0086f0d0`.
pub fn native_set_arrive_radius_all<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    set_f(h, w, s, 0x0086_f0d0)
}

/// `Native_SetArriveRadius` (`0x0085f040`, `0x0017`, float): `(r)` ->
/// `0x00870d10(worker, level, r)`; `ST0` is `r`.
pub fn native_set_arrive_radius<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let v = popf(h, s);
    let l = lvl(h, w);
    call(h, 0x0087_0d10, &[w, l, v]);
    x87::load(v)
}

/// `Native_SetYaw` (`0x0085f170`, `0x0095`, float): `(yaw)`: stops the
/// turn, the position's yaw (`+0x20`, `+0x34`) = the wrapped yaw, the
/// character (type 1, `+0xaa` 1) turned with it (`0x0082b1b0`,
/// `0x0082b520`) or `0x0086f4c0` for `+0xaa` 4 and 5; `ST0` the wrapped yaw.
pub fn native_set_yaw<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let p = atel_get_pos_work_adrs(h, w);
    call(h, 0x0086_3fe0, &[w, u32::MAX]);
    let v = popf(h, s);
    let y = x87::store(callf(h, super::native::MATH_WRAP_ANGLE, &[v]));
    h.write(p + 0x20, y);
    h.write(p + 0x34, y);
    call(h, SET_MOVE_HOKAN_MODE, &[w, 0]);
    let k = h.read::<u8>(w + 0xaa);
    if k == 1 {
        let c = h.read::<u32>(w + 0x9c);
        if c != 0 {
            let r = x87::store(x87::load(h.read(p + 0x20)));
            call(h, 0x0082_b1b0, &[c, r]);
            let c = h.read::<u32>(w + 0x9c);
            let r = x87::store(x87::load(h.read(p + 0x20)));
            call(h, 0x0082_b520, &[c, r]);
        }
    } else if k.wrapping_sub(4) <= 1 {
        call(h, 0x0086_f4c0, &[w]);
    }
    x87::load(y)
}

/// `Native_GetYawToMoveTarget` (`0x008587a0`, `0x001f`, float): the angle
/// from the worker's position to the move block's target (`+0x28`, `+0x30`),
/// `Math_Atan2XZ`.
pub fn native_get_yaw_to_move_target<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> f64 {
    let p = atel_get_pos_work_adrs(h, w);
    let m = move_block(h, w);
    let tx = x87::store(x87::load(h.read(m + 0x28)));
    let tz = x87::store(x87::load(h.read(m + 0x30)));
    let x = x87::store(x87::load(h.read(p)));
    let z = x87::store(x87::load(h.read(p + 8)));
    callf(h, ATAN2_XZ, &[x, z, tx, tz])
}

/// `Native_AngleToPoint` (`0x00858970`, `0x00f4`, float): `(x, y, z)`: the
/// angle from the worker to `(x, z)` (`y` popped and dropped).
pub fn native_angle_to_point<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let p = atel_get_pos_work_adrs(h, w);
    let tz = popf(h, s);
    popf(h, s);
    let tx = popf(h, s);
    let x = x87::store(x87::load(h.read(p)));
    let z = x87::store(x87::load(h.read(p + 8)));
    callf(h, ATAN2_XZ, &[x, z, tx, tz])
}

/// `Native_SetTalkRange` (`0x00857180`, `0x0061`, float): `(r)`: the
/// position's `+0x40` = `r` truncated, as a float; worker `+0x36` |= 1;
/// `ST0` the stored value.
pub fn native_set_talk_range<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let p = atel_get_pos_work_adrs(h, w);
    let v = pop_st0(h, s);
    let r = x87::ftol(h, v);
    let f = x87::store(f64::from(r.cast_signed()));
    h.write(p + 0x40, f);
    let fl = h.read::<u16>(w + worker::FLAGS2);
    h.write(w + worker::FLAGS2, fl | 1);
    x87::load(f)
}

/// `Event_GetEntrance(worker)` (`0x0086bfe0`)'s float at `off`.
fn entrance(h: &mut impl NativeHost, w: u32, off: u32) -> f64 {
    let e = call(h, 0x0086_bfe0, &[w]);
    x87::load(h.read(e + off))
}

/// `Native_GetEntranceZ` (`0x0085cb20`, `0x0082`, float): `+0x14`.
pub fn native_get_entrance_z<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> f64 {
    entrance(h, w, 0x14)
}

/// `Native_GetEntranceYaw` (`0x0085cce0`, `0x0083`, float): `+8`.
pub fn native_get_entrance_yaw<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> f64 {
    entrance(h, w, 8)
}

/// `AtelCall_Math_Sqrt_RF` (`0x00877b70`, `0x1006`, float): `sqrt(x)`
/// rounded to `f32`.
pub fn native_math_sqrt<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    let v = pop_st0(h, s);
    x87::load(x87::store(x87::sqrt(v)))
}

/// `Native_WaitFrames_0000` (`0x0085c4e0`, start): `(frames)` into the
/// args word (40 for 10 in battle scene `0x610000` with magic `0x290`);
/// `EAX` the args address.
pub fn native_wait_frames<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    let n = pop(h, s);
    let scene = call(h, 0x0078_2970, &[]);
    let magic = call(h, GET_MAGIC_ID, &[]);
    let n = if scene == 0x0061_0000 && magic == 0x290 && n == 10 {
        0x28
    } else {
        n
    };
    h.write(a, n);
    a
}

/// `g_0112a008` (`0x0112a008`): with it set, the frame wait counts the
/// frames `0x00645d90()` reports while `0x0076efc0()` holds.
pub const WAIT_FRAMES_BY_TIME: Global<u32> = Global::new(0x0112_a008);

/// `Native_WaitFrames_Poll_0000` (`0x0085c6b0`): counts the args word down
/// (by one, or by `0x00645d90()` when [`WAIT_FRAMES_BY_TIME`] is set and
/// `0x0076efc0()` nonzero); 1 at 0 and below.
pub fn native_wait_frames_poll<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    let v = h.read::<u32>(a);
    let d = if WAIT_FRAMES_BY_TIME.get(h) != 0 && call(h, 0x0076_efc0, &[]) != 0 {
        call(h, 0x0064_5d90, &[])
    } else {
        1
    };
    let v = v.wrapping_sub(d);
    h.write(a, v);
    u32::from(v.cast_signed() <= 0)
}

/// `AtelSetEventJump2(map, entrance, flag)` (`0x0086ff40`).
fn jump<H: NativeHost>(h: &mut H, s: u32, flag: u32) -> u32 {
    let e = pop(h, s);
    let m = pop(h, s);
    call(h, 0x0086_ff40, &[m, e, flag]);
    0
}

/// `Native_MapChange` (`0x008581c0`, `0x0011`): `(map, entrance)`, flag 1; 0.
pub fn native_map_change<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    jump(h, s, 1)
}

/// `Native_MapJump` (`0x00858460`, `0x010b`): flag 0; 0.
pub fn native_map_jump<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    jump(h, s, 0)
}

/// `Native_WorkerDetach` (`0x0085cdc0`, `0x0005`): pushes 0 and goes on as
/// `Worker_Detach` (`0x0085ca00`).
pub fn native_worker_detach<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    atel_push_stack_integer(h, s, 0);
    call(h, 0x0085_ca00, &[w, a, s])
}

/// `Native_ChrLoadAlias` (`0x0085d3c0`, `0x0134`): `Native_SetModel`
/// (`0x0085d000`).
pub fn native_chr_load_alias<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    call(h, 0x0085_d000, &[w, a, s])
}

/// `Native_BitFlagSet` (`0x0085c3b0`, `0x0210`): `(bit)` ->
/// `0x008701f0(bit, 1)`; the bit.
pub fn native_bit_flag_set<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0087_01f0, &[v, 1]);
    v
}

/// `CT_0060_ret` (`0x00856920`, `0x0060`): `(v)` -> `0x008712a0(worker,
/// v)`, then `0x0086a0e0()`; the first's `EAX`.
pub fn native_0060<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let r = call(h, 0x0087_12a0, &[w, v]);
    call(h, 0x0086_a0e0, &[]);
    r
}

/// `Native_FootstepHookEnable` (`0x00859410`, `0x0151`): `(v)` ->
/// `0x008870b0(v, 1)`; 1.
pub fn native_footstep_hook_enable<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0088_70b0, &[v, 1]);
    1
}

/// `Native_003d` (`0x0085c3e0`, `0x003d`): `0x0086db80(worker, 0xff)`; 0.
pub fn native_003d<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    call(h, 0x0086_db80, &[w, 0xff]);
    0
}

/// `Native_SetMoveMOT` (`0x0085bdc0`, `0x0074`): `(v)` ->
/// `0x0086f080(worker, v)`; `v`.
pub fn native_set_move_mot<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0086_f080, &[w, v]);
    v
}

/// `Native_LineGetCrossSide` (`0x0085d410`, `0x008b`): `0x008632b0(worker,
/// -1)`, or when negative the worker's `+0x36` bits 1-2 (-1 for 3).
pub fn native_line_get_cross_side<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let r = call(h, 0x0086_32b0, &[w, u32::MAX]);
    if r.cast_signed() >= 0 {
        return r;
    }
    let k = (u32::from(h.read::<u16>(w + worker::FLAGS2)) >> 1) & 3;
    if k == 3 { u32::MAX } else { k }
}

/// `Native_0036` (`0x0085b4e0`, `0x0036`): `(worker)`: `0x00863e10(it,
/// level)` for each level 0..9; 0.
pub fn native_0036<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    let o = atel_get_basic_work(h, i.cast_signed());
    for k in 0..9 {
        call(h, 0x0086_3e10, &[o, k]);
    }
    0
}

/// `Native_GetWorkerPos` (`0x00856e20`, `0x00a3`): `(worker, &x, &y,
/// &z)`: `0x0086a2f0(worker, &x, &y, &z)` (the variable addresses resolved
/// z first); 0.
pub fn native_get_worker_pos<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let vz = pop(h, s);
    let vy = pop(h, s);
    let vx = pop(h, s);
    let o = pop(h, s);
    let x = atel_get_real_var_adrs(h, w, vx);
    let y = atel_get_real_var_adrs(h, w, vy);
    let z = atel_get_real_var_adrs(h, w, vz);
    call(h, 0x0086_a2f0, &[o, x, y, z]);
    0
}

/// `Native_PartySwapIn` (`0x0085bdf0`, `0x00e7`): `(b, a)`: `-1` for `a`
/// becomes `0xff`; `0x00784d00(a, b)`; `a`.
pub fn native_party_swap_in<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let mut a = pop(h, s);
    let b = pop(h, s);
    if a == u32::MAX {
        a = 0xff;
    }
    call(h, 0x0078_4d00, &[a, b]);
    a
}

/// `Native_SetActorProperty` (`0x007a5360`, `0x70ab`): `(property,
/// value)` on the worker's battle actor: `0x007b4b70(actor, property, -13,
/// value)`'s `EAX`.
pub fn native_set_actor_property<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let p = pop(h, s);
    let id = u32::from(h.read::<u8>(atel_get_battle_id_adrs2(w)));
    call(h, 0x007b_4b70, &[id, p, 0xffff_fff3, v])
}

/// `Native_ActorPlayMotion` (`0x007a35a0`, `0x7007`): `0x007a8fc0(worker,
/// args, stack, 0)`'s `EAX`.
pub fn native_actor_play_motion<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    call(h, 0x007a_8fc0, &[w, a, s, 0])
}

/// `Native_ChrPlayMotion` (`0x00a79270`, `0x501a`) and
/// `Native_ChrPlayMotion2` (`0x00a7a780`, `0x5042`):
/// `Native_PlayMotionCommon(worker, stack, 1, 0)`; 0.
pub fn native_chr_play_motion<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    call(h, 0x00a7_aae0, &[w, s, 1, 0]);
    0
}

/// `Native_ChrPlayDefaultMotion` (`0x00a79e60`, `0x5032`): `0x00837180(chr)`
/// with a character; 0.
pub fn native_chr_play_default_motion<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let c = atel_get_char_handle(h, w);
    if c != 0 {
        call(h, 0x0083_7180, &[c]);
    }
    0
}

/// `Native_ChrSetMotion2` (`0x00a7a610`, `0x5040`): `(v)` ->
/// `0x00837ac0(chr, v)` with a character; 0.
pub fn native_chr_set_motion2<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let c = atel_get_char_handle(h, w);
    if c != 0 {
        call(h, 0x0083_7ac0, &[c, v]);
    }
    0
}

/// `Native_ChrSetMotionSeqLevel` (`0x00a79550`, `0x5022`): `(v)` ->
/// `0x00837570(chr, v)` with a character (read before the pop); 0.
pub fn native_chr_set_motion_seq_level<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let c = atel_get_char_handle(h, w);
    let v = pop(h, s);
    if c != 0 {
        call(h, 0x0083_7570, &[c, v]);
    }
    0
}

/// `Native_ChrLoad` (`0x00a79800`, `0x504e`): `(id)` -> `0x00829ef0(id)`'s
/// `EAX`.
pub fn native_chr_load<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0082_9ef0, &[v])
}

/// `Native_FadeIn` (`0x00a77d80`, `0x4004`): `(frames)` ->
/// `0x0082cdc0(frames, 1, 0)`; 0.
pub fn native_fade_in<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0082_cdc0, &[v, 1, 0]);
    0
}

/// `Native_FadeOut` (`0x00a77da0`, `0x4005`): `(frames, 0, 0)`; 0.
pub fn native_fade_out<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0082_cdc0, &[v, 0, 0]);
    0
}

/// `Native_MotionBlur` (`0x00a77e00`, `0x400a`): `(v)` -> `0x0082bee0(v)`; 0.
pub fn native_motion_blur<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0082_bee0, &[v]);
    0
}

/// `Native_SpriteSetPos` (`0x0085cc50`, `0x011f`): `(sprite, x, y)` ->
/// `0x00874b30(sprite, x, y)` (on language 0xb with graphics name hash
/// 0x199, (0x10e, 0xd5) becomes (0x122, 0xd5)); the sprite.
pub fn native_sprite_set_pos<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let y = pop(h, s);
    let mut x = pop(h, s);
    let sp = pop(h, s);
    if call(h, 0x008a_c2f0, &[]) == 0xb
        && call(h, 0x0064_1da0, &[]) == 0x199
        && x == 0x10e
        && y == 0xd5
    {
        x = 0x122;
    }
    call(h, 0x0087_4b30, &[sp, x, y]);
    sp
}

/// `Native_LightSet` (`0x00a78030`, `0x4013`): `(light, r, g, b, pitch,
/// yaw)` -> `Light_Set(light, r, g, b, pitch, yaw)`; 0.
pub fn native_light_set<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let mut v = [0u32; 6];
    for x in v.iter_mut().rev() {
        *x = pop(h, s);
    }
    call(h, 0x0083_6520, &v);
    0
}

/// `Native_SetModel` (`0x0085d000`, `0x0001`): `(model)`: detaches what the
/// worker had (`Worker_Detach` with 0 pushed), allocates the character
/// (`TkCh_Allocate2(model, id)` into `+0x9c`), sets its battle number,
/// sound id and yaw from the position, `+0xa8` = model and `+0xaa` = 1,
/// places it (`AtelSetPos3d`), outside a battle its sound and footsteps,
/// and binds the effects `+0x38` bits 5-7 ask for; 0.
pub fn native_set_model<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    let c = call(h, 0x0086_aff0, &[]);
    let p = atel_get_pos_work_adrs(h, w);
    atel_push_stack_integer(h, s, 0);
    call(h, 0x0085_ca00, &[w, a, s]);
    let m = pop(h, s);
    let id = u32::from(h.read::<u16>(w + worker::ID));
    let ch = call(h, 0x0087_2210, &[m, id]);
    h.write(w + worker::CHAR_HANDLE, ch);
    call(h, 0x0087_2160, &[w]);
    let snd = |h: &mut H| u32::from(h.read::<u8>(w + 0xaf));
    let (ch, k) = (h.read::<u32>(w + worker::CHAR_HANDLE), snd(h));
    call(h, 0x0083_5b50, &[ch, k]);
    for f in [0x0082_b1b0, 0x0082_b520] {
        let yaw = x87::store(x87::load(h.read(p + 0x20)));
        let ch = h.read::<u32>(w + worker::CHAR_HANDLE);
        call(h, f, &[ch, yaw]);
    }
    #[allow(clippy::cast_possible_truncation)] // the original stores SI
    h.write(w + 0xa8, m as u16);
    h.write(w + 0xaa, 1_u8);
    let pos: Vec<u32> = (0..3)
        .map(|i| x87::store(x87::load(h.read(p + i * 4))))
        .collect();
    call(h, 0x0087_0b20, &[w, pos[0], pos[1], pos[2], 0]);
    if h.read::<u16>(c + 0x1a) == 0 {
        if h.read::<u8>(w + 0xaa) == 1 {
            let ch = h.read::<u32>(w + worker::CHAR_HANDLE);
            if ch != 0 {
                let k = snd(h);
                call(h, 0x0083_5b50, &[ch, k]);
            }
        }
        let id = u32::from(h.read::<u16>(w + worker::ID));
        let k = snd(h);
        call(h, 0x0088_5fd0, &[k, id]);
    }
    for (bit, k) in [(0x20u8, 0u32), (0x40, 1), (0x80, 2)] {
        if h.read::<u8>(w + 0x38) & bit != 0 {
            call(h, 0x0086_2700, &[w, k]);
        }
    }
    0
}

/// The Rust version of one of these handlers, if there is one.
#[must_use]
pub fn find<H: NativeHost>(sym: u32) -> Option<Native<H>> {
    use Native::{Float, Int};
    Some(match sym {
        0x0085_6c30 => Int(native_move_start),
        0x0085_7520 => Int(native_turn_start),
        0x0086_02f0 => Int(native_move_set_target),
        0x0085_a5c0 => Float(native_turn_set_speed2),
        0x0085_a2a0 => Float(native_turn_set_speed),
        0x0085_a480 => Float(native_set_turn_speed),
        0x0085_7f40 => Float(native_set_move_speed),
        0x0085_7da0 => Float(native_move_set_speed),
        0x0085_8020 => Float(native_set_arrive_radius_all),
        0x0085_f040 => Float(native_set_arrive_radius),
        0x0085_f170 => Float(native_set_yaw),
        0x0085_87a0 => Float(native_get_yaw_to_move_target),
        0x0085_8970 => Float(native_angle_to_point),
        0x0085_7180 => Float(native_set_talk_range),
        0x0085_cb20 => Float(native_get_entrance_z),
        0x0085_cce0 => Float(native_get_entrance_yaw),
        0x0087_7b70 => Float(native_math_sqrt),
        0x0085_c4e0 => Int(native_wait_frames),
        0x0085_c6b0 => Int(native_wait_frames_poll),
        0x0085_81c0 => Int(native_map_change),
        0x0085_8460 => Int(native_map_jump),
        0x0085_cdc0 => Int(native_worker_detach),
        0x0085_d3c0 => Int(native_chr_load_alias),
        0x0085_c3b0 => Int(native_bit_flag_set),
        0x0085_6920 => Int(native_0060),
        0x0085_9410 => Int(native_footstep_hook_enable),
        0x0085_c3e0 => Int(native_003d),
        0x0085_bdc0 => Int(native_set_move_mot),
        0x0085_d410 => Int(native_line_get_cross_side),
        0x0085_b4e0 => Int(native_0036),
        0x0085_6e20 => Int(native_get_worker_pos),
        0x0085_bdf0 => Int(native_party_swap_in),
        0x007a_5360 => Int(native_set_actor_property),
        0x007a_35a0 => Int(native_actor_play_motion),
        0x00a7_9270 | 0x00a7_a780 => Int(native_chr_play_motion),
        0x00a7_9e60 => Int(native_chr_play_default_motion),
        0x00a7_a610 => Int(native_chr_set_motion2),
        0x00a7_9550 => Int(native_chr_set_motion_seq_level),
        0x00a7_9800 => Int(native_chr_load),
        0x00a7_7d80 => Int(native_fade_in),
        0x00a7_7da0 => Int(native_fade_out),
        0x00a7_7e00 => Int(native_motion_blur),
        0x0085_cc50 => Int(native_sprite_set_pos),
        0x00a7_8030 => Int(native_light_set),
        0x0085_d000 => Int(native_set_model),
        _ => return super::native_btl::find(sym),
    })
}

/// Every handler [`find`] has, with whether it returns a float.
pub const PORTED: [(u32, bool); 46] = [
    (0x0085_cc50, false),
    (0x00a7_8030, false),
    (0x0085_d000, false),
    (0x0085_6c30, false),
    (0x0085_7520, false),
    (0x0086_02f0, false),
    (0x0085_a5c0, true),
    (0x0085_a2a0, true),
    (0x0085_a480, true),
    (0x0085_7f40, true),
    (0x0085_7da0, true),
    (0x0085_8020, true),
    (0x0085_f040, true),
    (0x0085_f170, true),
    (0x0085_87a0, true),
    (0x0085_8970, true),
    (0x0085_7180, true),
    (0x0085_cb20, true),
    (0x0085_cce0, true),
    (0x0087_7b70, true),
    (0x0085_c4e0, false),
    (0x0085_c6b0, false),
    (0x0085_81c0, false),
    (0x0085_8460, false),
    (0x0085_cdc0, false),
    (0x0085_d3c0, false),
    (0x0085_c3b0, false),
    (0x0085_6920, false),
    (0x0085_9410, false),
    (0x0085_c3e0, false),
    (0x0085_bdc0, false),
    (0x0085_d410, false),
    (0x0085_b4e0, false),
    (0x0085_6e20, false),
    (0x0085_bdf0, false),
    (0x007a_5360, false),
    (0x007a_35a0, false),
    (0x00a7_9270, false),
    (0x00a7_a780, false),
    (0x00a7_9e60, false),
    (0x00a7_a610, false),
    (0x00a7_9550, false),
    (0x00a7_9800, false),
    (0x00a7_7d80, false),
    (0x00a7_7da0, false),
    (0x00a7_7e00, false),
];
