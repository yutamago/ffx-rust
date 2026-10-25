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
    atel_get_real_var_adrs, atel_pop_stack_float, atel_pop_stack_integer, atel_push_stack_float,
    atel_push_stack_integer, script_worker_level,
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
    super::helpers::atel_start_move_block(h, w, b, mode, a);
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
    super::helpers::atel_start_turn_block(h, w, b, mode, a);
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
        let f = h.scratch();
        super::helpers::atel_set_bind_object(h, w, f);
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
    super::helpers::atel_set_help_event_flag(h, v, 1);
    v
}

/// `CT_0060_ret` (`0x00856920`, `0x0060`): `(v)` -> `0x008712a0(worker,
/// v)`, then `0x0086a0e0()`; the first's `EAX`.
pub fn native_0060<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let r = call(h, 0x0087_12a0, &[w, v]);
    super::helpers::atel_uc_restore(h);
    r
}

/// `Native_FootstepHookEnable` (`0x00859410`, `0x0151`): `(v)` ->
/// `0x008870b0(v, 1)`; 1.
pub fn native_footstep_hook_enable<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0088_70b0, &[v, 1]);
    1
}

/// `Native_003d_CopyTurnAnglesAll` (`0x0085c3e0`, `0x003d`): `0x0086db80(worker, 0xff)`; 0.
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

/// `Native_0036_WorkerStopMoveAll` (`0x0085b4e0`, `0x0036`): `(worker)`: `0x00863e10(it,
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
    super::helpers::script_get_worker_pos(h, o, x, y, z);
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
    super::native_chr::native_play_motion_common(h, w, s, 1, 0);
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
    let f = h.scratch();
    super::helpers::atel_set_pos_3d(h, w, [pos[0], pos[1], pos[2]], 0, f);
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
            super::helpers::atel_bind_eff2_on(h, w, k);
        }
    }
    0
}

/// `Native_0006_ChrAttachModel` (`0x0085e620`, `0x0006`): `(slot, model, motion)`: the
/// slot detached (`0x00862780` with it pushed and popped again); with a
/// character and a model, the attachment (`0x00872190(model)`) into
/// `+0xaf4 + slot * 4` with `motion` in `+0xafc + slot * 4`, its sound id
/// 15, bound to the character (`0x008722b0`) with its flags (`+0xb08`
/// bits 0 and 1), the slot's bit in `+0xb14` and the model in `+0xb04 +
/// slot * 2`; 1, or 0 without a character or model.
pub fn native_0006<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let motion = pop(h, s);
    let model = pop(h, s);
    let slot = pop(h, s);
    atel_push_stack_integer(h, s, slot);
    let k = pop(h, s);
    call(h, 0x0086_2780, &[w, k]);
    if h.read::<u32>(w + worker::CHAR_HANDLE) == 0 || model == 0 {
        return 0;
    }
    let at = w.wrapping_add(slot.wrapping_mul(4));
    let c = call(h, 0x0087_2190, &[model]);
    h.write(at + 0xaf4, c);
    h.write(at + 0xafc, motion);
    call(h, 0x0083_5b50, &[c, 0xf]);
    let c = h.read::<u32>(at + 0xaf4);
    if c == 0 {
        return 1;
    }
    let (mot, ch) = (
        h.read::<u32>(at + 0xafc),
        h.read::<u32>(w + worker::CHAR_HANDLE),
    );
    let id = u32::from(h.read::<u16>(w + 0xa8));
    call(h, 0x0087_22b0, &[id, c, ch, mot, slot]);
    for f in [0x0082_b240, 0x0082_aa70, 0x0082_b820] {
        let c = h.read::<u32>(at + 0xaf4);
        call(h, f, &[c, 0]);
    }
    let fl = h.read::<u32>(at + 0xb08);
    let c = h.read::<u32>(at + 0xaf4);
    call(h, 0x0082_aa30, &[c, fl & 1]);
    let fl = h.read::<u32>(at + 0xb08);
    let c = h.read::<u32>(at + 0xaf4);
    call(h, 0x0082_b970, &[c, (fl >> 1) & 1]);
    // shl dx, cl: a 16-bit shift by the count mod 32
    let bit = 1u32.checked_shl(slot & 0x1f).unwrap_or(0);
    #[allow(clippy::cast_possible_truncation)] // the original ORs DX
    let bit = bit as u16;
    let b = h.read::<u16>(w + 0xb14);
    h.write(w + 0xb14, b | bit);
    #[allow(clippy::cast_possible_truncation)] // the original stores SI
    h.write(w.wrapping_add(slot.wrapping_mul(2)) + 0xb04, model as u16);
    1
}

/// `Native_00f5_PitchToPoint` (`0x00858f40`, `0x00f5`, float): `(x, y, z)`: the second
/// result of `0x0088b4d0(&a, &b, the worker's position, z, y, x)` (the
/// original's two stack locals; here zeroed scratch).
pub fn native_00f5<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let p = atel_get_pos_work_adrs(h, w);
    let x = popf(h, s);
    let y = popf(h, s);
    let z = popf(h, s);
    let o = h.scratch();
    h.write(o, 0_u32);
    h.write(o + 4, 0_u32);
    let pos: Vec<u32> = (0..3)
        .map(|i| x87::store(x87::load(h.read(p + i * 4))))
        .collect();
    let f = h.rebase(0x0088_b4d0);
    h.call_n(f, &[o, o + 4, pos[0], pos[1], pos[2], z, y, x]);
    x87::load(h.read(o + 4))
}

/// `Native_00cb_PartyRemove` (`0x0085b820`, `0x00cb`): `(chr)`: `0x007869b0(chr, 0)`,
/// then the three party slots (`0x007852f0`) with `chr` replaced by `0xff`
/// handed to `0x00786950`; `chr`.
pub fn native_00cb<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0078_69b0, &[v, 0]);
    let o = h.scratch();
    for i in 0..3 {
        h.write(o + i * 4, 0_u32);
    }
    call(h, 0x0078_52f0, &[o, o + 4, o + 8]);
    let mut p = [0u32; 3];
    for (i, x) in p.iter_mut().enumerate() {
        let k = h.read::<u32>(o + u32::try_from(i).unwrap_or(0) * 4);
        *x = if k == v && k != 0xff { 0xff } else { k };
    }
    call(h, 0x0078_6950, &p);
    v
}

/// `g_022fb4d4` (`0x022fb4d4`): the menu wait's frames (`CT_401D_init`
/// sets 1).
pub const MENU_WAIT: Global<u32> = Global::new(0x022f_b4d4);
/// `g_023cbbf0` (`0x023cbbf0`), printed by the menu natives' debug lines.
const MENU_DEBUG: Global<u32> = Global::new(0x023c_bbf0);

/// `CT_401D_init` (`0x00a78210`, `0x401d`, start): `(menu)`: a debug line;
/// `Sg_MenuSetReserve(menu)` unless it is `0x4008000f`-`0x40080011` and
/// `getMegasAdding()` is 0; [`MENU_WAIT`] = 1; the last call's `EAX`.
pub fn native_menu_open<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let f = h.rebase(0x00b7_c588);
    let d = MENU_DEBUG.get(h);
    call(h, 0x0062_f500, &[f, d]);
    let r = if (0x4008_000f..=0x4008_0011).contains(&v) {
        let g = call(h, 0x0086_0a90, &[]);
        if g == 0 {
            g
        } else {
            call(h, 0x0082_1750, &[v])
        }
    } else {
        call(h, 0x0082_1750, &[v])
    };
    MENU_WAIT.set(h, 1);
    r
}

/// `Native_MenuOpen_Poll` (`0x00a78270`): a debug line; 1 once
/// [`MENU_WAIT`] (counted down) was 0, else 0.
pub fn native_menu_open_poll<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    let f = h.rebase(0x00b7_c598);
    let d = MENU_DEBUG.get(h);
    call(h, 0x0062_f500, &[f, d]);
    let c = MENU_WAIT.get(h);
    MENU_WAIT.set(h, c.wrapping_sub(1));
    u32::from(c == 0)
}

/// `Native_MenuOpen_Result` (`0x00a782a0`): a debug line with the menu
/// result (`0x00821740`); that result.
pub fn native_menu_open_result<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    let r = call(h, 0x0082_1740, &[]);
    let f = h.rebase(0x00b7_c5a8);
    let d = MENU_DEBUG.get(h);
    call(h, 0x0062_f500, &[f, d, r]);
    call(h, 0x0082_1740, &[])
}

/// `Native_GetEntranceX` (`0x0085c7b0`, `0x0080`, float): `+0xc`.
pub fn native_get_entrance_x<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> f64 {
    entrance(h, w, 0xc)
}

/// `Native_GetEntranceY` (`0x0085c8f0`, `0x0081`, float): `+0x10`.
pub fn native_get_entrance_y<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> f64 {
    entrance(h, w, 0x10)
}

/// `Native_002c_LevelSet5f8` (`0x0085a1a0`, `0x002c`, float): `(v)` ->
/// `0x00871500(worker, level, v)`'s `ST0`.
pub fn native_002c<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    turn_speed(h, w, s, 0x0087_1500)
}

/// `Native_002b_LevelSet5f4` (`0x0085a0b0`, `0x002b`, float): `0x00871520`.
pub fn native_002b<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    turn_speed(h, w, s, 0x0087_1520)
}

/// `Native_0021_MoveSetYaw` (`0x00859430`, `0x0021`, float): `(yaw)`: the wrapped yaw
/// into the move block's `+0x44`; `ST0` it.
pub fn native_0021<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let m = move_block(h, w);
    let v = popf(h, s);
    let y = x87::store(callf(h, super::native::MATH_WRAP_ANGLE, &[v]));
    h.write(m + 0x44, y);
    x87::load(y)
}

/// `Native_015e_MinimapSetForceDraw` (`0x00857980`, `0x015e`): `(v)` -> `0x008701c0(v)`.
pub fn native_015e<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0087_01c0, &[v])
}

/// `Native_0119_ChrGetEquipModel` (`0x0085bd70`, `0x0119`): `(on)` clamped to 0..=1 ->
/// `AtelGetElemId(the worker's bound character, on)`.
pub fn native_0119<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s).cast_signed().clamp(0, 1).cast_unsigned();
    let pc = h.read::<u32>(w + worker::BIND_PC);
    super::helpers::atel_get_elem_id(h, pc, v)
}

/// `Native_SetTriggerMode` (`0x0085c320`, `0x007f`): `(mode)`: for trigger
/// workers (types 2, 3, 5, 6) bits 13-14 of `+0x34` = mode; 1, else 0.
pub fn native_set_trigger_mode<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let hdr = h.read::<u32>(w);
    if !matches!(h.read::<u8>(hdr), 2 | 3 | 5 | 6) {
        return 0;
    }
    let f = h.read::<u16>(w + 0x34);
    #[allow(clippy::cast_possible_truncation)] // the original works on AX
    let x = ((v << 13) as u16 ^ f) & 0x6000;
    h.write(w + 0x34, f ^ x);
    1
}

/// `Native_SetPlayer` (`0x0085c9a0`, `0x0043`): `0x0086f6a0(worker id)`;
/// the id.
pub fn native_set_player<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = u32::from(h.read::<u16>(w + worker::ID));
    super::helpers::event_set_player(h, id);
    id
}

/// `Native_0091_GetYaw` (`0x0085e0b0`, `0x0091`, float): the position's yaw
/// (`+0x34`).
pub fn native_0091<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> f64 {
    let p = atel_get_pos_work_adrs(h, w);
    x87::load(h.read(p + 0x34))
}

/// `Native_0004_WorkerSetType4` (`0x0085ecb0`, `0x0004`): `(v)`: `+0xa8` = v, `+0xaa` =
/// 4; 0.
pub fn native_0004<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    #[allow(clippy::cast_possible_truncation)] // the original stores AX
    h.write(w + 0xa8, v as u16);
    h.write(w + 0xaa, 4_u8);
    0
}

/// `Native_00e0_SndSepStop` (`0x00855f60`, `0x00e0`): `(v)` -> `0x00886eb0(v)`; `v`.
pub fn native_00e0<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0088_6eb0, &[v]);
    v
}

/// A position float set from the stack (`+off`), worker `+0x36` |= 1.
fn pos_set<H: NativeHost>(h: &mut H, w: u32, s: u32, off: u32) -> u32 {
    let p = atel_get_pos_work_adrs(h, w);
    let v = popf(h, s);
    h.write(p + off, v);
    let fl = h.read::<u16>(w + worker::FLAGS2);
    h.write(w + worker::FLAGS2, fl | 1);
    v
}

/// `Native_SetTalkAngle` (`0x008574f0`, `0x0063`, float): `+0x3c`.
pub fn native_set_talk_angle<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    x87::load(pos_set(h, w, s, 0x3c))
}

/// `Native_SetTouchRange` (`0x00856460`, `0x0085`): `+0x48`; 1.
pub fn native_set_touch_range<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    pos_set(h, w, s, 0x48);
    1
}

/// `Native_SetTYZAngle` (`0x008577c0`, `0x008d`, float): `+0x4c`.
pub fn native_set_tyz_angle<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    x87::load(pos_set(h, w, s, 0x4c))
}

/// `Native_SetRadius44` (`0x008572e0`, `0x0062`, float): `(r)` truncated
/// into the position's `+0x44`; worker `+0x36` |= 1; `ST0` the stored value.
pub fn native_set_radius44<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let p = atel_get_pos_work_adrs(h, w);
    let v = pop_st0(h, s);
    let r = x87::ftol(h, v);
    let f = x87::store(f64::from(r.cast_signed()));
    h.write(p + 0x44, f);
    let fl = h.read::<u16>(w + worker::FLAGS2);
    h.write(w + worker::FLAGS2, fl | 1);
    x87::load(f)
}

/// `Native_00a7_WorkerClearFlag34Bit1` (`0x00857820`, `0x00a7`): `(worker)`: its `+0x34` bit 1
/// cleared; 0.
pub fn native_00a7<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    let o = atel_get_basic_work(h, i.cast_signed());
    let f = h.read::<u16>(o + 0x34);
    h.write(o + 0x34, f & 0xfffd);
    0
}

/// `Native_0058_TriggerSetPointDefault` (`0x00855f80`, `0x0058`): pushes the float at `0x00b92340`
/// and goes on as `Native_0059_TriggerSetPoint` (`0x00855cd0`).
pub fn native_0058<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    let f = h.read::<u32>(h.rebase(0x00b9_2340));
    atel_push_stack_float(h, s, f);
    call(h, 0x0085_5cd0, &[w, a, s])
}

/// `Native_00fb_MsgWinTableByte2` (`0x00858ac0`, `0x00fb`): `(window)` -> `0x008b8010(window)`.
pub fn native_00fb<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x008b_8010, &[v])
}

/// `Native_0180_CurMusicVolumeTranslate` (`0x0085c000`, `0x0180`): `(a, b)`: `0x00886b40(i, a, b)`
/// for `i = 0x00873b00()` when not negative; `i`.
pub fn native_0180<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let b = pop(h, s);
    let a = pop(h, s);
    let i = call(h, 0x0087_3b00, &[]);
    if i.cast_signed() >= 0 {
        call(h, 0x0088_6b40, &[i, a, b]);
    }
    i
}

/// `Native_SetTurnSpeedAll` (`0x0085b030`, `0x00e4`, float): `(v)` ->
/// `0x0086f300(worker, v)` (its `ST0` dropped); `ST0` `v`.
pub fn native_set_turn_speed_all<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let v = popf(h, s);
    callf(h, 0x0086_f300, &[w, v]);
    x87::load(v)
}

/// `Native_SetMoveParam3C` (`0x0085ab80`, `0x006d`, float): `0x0086f1c0`.
pub fn native_set_move_param3c<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    set_f(h, w, s, 0x0086_f1c0)
}

/// `Native_SetMoveParam40` (`0x0085acd0`, `0x006e`, float): `0x0086f170`.
pub fn native_set_move_param40<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    set_f(h, w, s, 0x0086_f170)
}

/// `Native_SetTurnParam10` (`0x0085af20`, `0x006f`, float): `0x0086f2b0`.
pub fn native_set_turn_param10<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    set_f(h, w, s, 0x0086_f2b0)
}

/// `Native_SetTurnParam18` (`0x0085b230`, `0x0070`, float): `0x0086f260`.
pub fn native_set_turn_param18<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    set_f(h, w, s, 0x0086_f260)
}

/// `Native_SetTurnParam1C` (`0x0085b480`, `0x0071`, float): `0x0086f210`.
pub fn native_set_turn_param1c<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    set_f(h, w, s, 0x0086_f210)
}

/// `Native_GetOwnWorkerType` (`0x0085f5a0`, `0x0076`):
/// `0x0086a7b0(worker id)`.
pub fn native_get_own_worker_type<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = u32::from(h.read::<u16>(w + worker::ID));
    call(h, 0x0086_a7b0, &[id])
}

/// `Native_00aa_SetBattleStatus` (`0x00857a60`, `0x00aa`): `(v)` -> `0x0086f3d0(v)`; `v`.
pub fn native_00aa<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0086_f3d0, &[v]);
    v
}

/// `g_01325b40` (`0x01325b40`): whether the music `Native_MusicPlay` last
/// started is track `0x25`.
pub const MUSIC_IS_25: Global<u32> = Global::new(0x0132_5b40);

/// `Native_MusicPlay` (`0x008596d0`, `0x0104`): `(track)`: `0x008869e0`,
/// `0x00886a20`, `0x00873d00` on it; 1.
pub fn native_music_play<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    MUSIC_IS_25.set(h, u32::from(v == 0x25));
    for f in [0x0088_69e0, 0x0088_6a20, 0x0087_3d00] {
        call(h, f, &[v]);
    }
    1
}

/// `Native_0007_ChrDetachModel` (`0x0085c8c0`, `0x0007`): `(slot)` ->
/// `0x00862780(worker, slot)`; 1.
pub fn native_0007<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0086_2780, &[w, v]);
    1
}

/// `Native_012a_SpriteSetFlag` (`0x0085c890`, `0x012a`): `(b, a)` -> `0x00874970(b, a)`; `b`.
pub fn native_012a<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    call(h, 0x0087_4970, &[b, a]);
    b
}

/// `Native_0133_WorkerTriggerSetActive` (`0x00860290`, `0x0133`): as `0x0085fe30`.
pub fn native_0133<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    call(h, 0x0085_fe30, &[w, a, s])
}

/// `Native_016f_BackupEventJump` (`0x0085c430`, `0x016f`): `(v)`: the record
/// `0x0088c830()` gets `+0xb8` = v and `+0xba` = `0x0086c1c0()`, then
/// `0x0088e800(it)`; `v`.
pub fn native_016f<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let g = call(h, 0x0088_c830, &[]);
    let v = pop(h, s);
    #[allow(clippy::cast_possible_truncation)] // the original stores SI, AX
    {
        h.write(g + 0xb8, v as u16);
        let x = call(h, 0x0086_c1c0, &[]);
        h.write(g + 0xba, x as u16);
    }
    call(h, 0x0088_e800, &[g]);
    v
}

/// `Native_0236_MsgGetText` (`0x00860350`, `0x0236`): `(msg)`: its text from the
/// second table (`Msg_Text1`) when `0x0086bbc0()` is set, else `Msg_Text`.
pub fn native_0236<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let f = if call(h, 0x0086_bbc0, &[]) == 0 {
        0x0086_bf30
    } else {
        0x0086_b170
    };
    call(h, f, &[v])
}

/// An `f32` through an x87 register (`fld`/`fstp`).
fn cp(v: u32) -> u32 {
    x87::store(x87::load(v))
}

/// An `f64` rounded to `f32` and back (an `fstp` to a float local).
fn r32(x: f64) -> f64 {
    x87::load(x87::store(x))
}

/// Worker `i`'s basic work (`AtelGetBasicWork`, `0x0086a830`).
fn work(h: &mut impl NativeHost, i: u32) -> u32 {
    atel_get_basic_work(h, i.cast_signed())
}

/// The worker type byte (its header's first byte).
fn kind(h: &impl NativeHost, w: u32) -> u8 {
    let hdr = h.read::<u32>(w);
    h.read::<u8>(hdr)
}

/// `g_CurrentMap` (`0x01330844`).
const CURRENT_MAP: u32 = 0x0133_0844;

/// `Native_SetPos_0126` (`0x0085fea0`, `0x0126`): `(x, y, z)` floats:
/// `0x00870b20(worker, x, y, z, 1)`; on map `0x9f`, a `z` strictly between
/// `[0x00b5bc08]` and `[0x00b5bc10]` also sets [`MUSIC_IS_25`]; 0.
pub fn native_set_pos_0126<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let z = popf(h, s);
    let y = popf(h, s);
    let x = popf(h, s);
    if h.read::<u32>(h.rebase(CURRENT_MAP)) == 0x9f {
        let lo = h.read::<f64>(h.rebase(0x00b5_bc08));
        let hi = x87::load(h.read(h.rebase(0x00b5_bc10)));
        let v = x87::load(z);
        if v > lo && hi > v {
            MUSIC_IS_25.set(h, 1);
        }
    }
    let f = h.scratch();
    super::helpers::atel_set_pos_3d(h, w, [cp(x), cp(y), cp(z)], 1, f);
    0
}

/// `Native_GetTurnFrames` (`0x0085ad00`, `0x00c8`): `(mode, frames,
/// frames2)`: the turn left (the turn block target `+0x20` minus the yaw,
/// wrapped, absolute) as a part of pi (mode bit 1, `[0x00b92288]`, eased
/// with `[0x00b48340]`) or of `[0x00b11338]`, clamped to 0..=1 and scaled
/// by `frames` (bit 1) or `frames2`, truncated.
pub fn native_get_turn_frames<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let p = atel_get_pos_work_adrs(h, w);
    let l = lvl(h, w);
    let a = pop(h, s);
    let b = pop(h, s);
    let m = pop(h, s);
    let t = x87::load(h.read(w + 0x884 + l * 0x44));
    let d = r32(x87::sub(t, x87::load(h.read(p + 0x34))));
    let d = r32(callf(h, super::native::MATH_WRAP_ANGLE, &[x87::store(d)]).abs());
    let (q, n) = if m & 2 != 0 {
        let x = r32(x87::div(d, h.read::<f64>(h.rebase(0x00b9_2288))));
        let u = r32(x87::sub(1.0, x));
        let v = r32(x87::mul(u, h.read::<f64>(h.rebase(0x00b4_8340))));
        (r32(x87::sub(1.0, v)), a)
    } else {
        let x = r32(x87::div(d, h.read::<f64>(h.rebase(0x00b1_1338))));
        let u = r32(x87::sub(1.0, x));
        (r32(x87::sub(1.0, u)), b)
    };
    // a NaN stays (both compares fail), as in the original
    let c = q.clamp(0.0, 1.0);
    let f = r32(f64::from(n.cast_signed()));
    x87::ftol(h, x87::mul(c, f))
}

/// `Native_ChrGetAttachBone` (`0x0085ce30`, `0x0213`): `(bone)`: for a
/// worker with a character (`+0xaa` 1, `+0x9c`), `0x00835660(chr, bone,
/// 0)`, else 0. (The worker id is pushed and popped back first.)
pub fn native_chr_get_attach_bone<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let id = u32::from(h.read::<u16>(w + worker::ID));
    atel_push_stack_integer(h, s, id);
    atel_push_stack_integer(h, s, v);
    let v = pop(h, s);
    let id = pop(h, s);
    let o = work(h, id);
    if h.read::<u8>(o + 0xaa) == 1 {
        let c = h.read::<u32>(o + 0x9c);
        if c != 0 {
            return call(h, 0x0083_5660, &[c, v, 0]);
        }
    }
    0
}

/// The party slot `v` (`0xf` from 8 up) into the worker's character
/// (`0x00835b50`) and `+0xaf`, and `0x00885fd0(slot, id)`, unless the
/// current ctrl work's `+0x1a` is set.
fn party_slot<H: NativeHost>(h: &mut H, w: u32, c: u32, slot: u32) {
    if h.read::<u16>(c + 0x1a) != 0 {
        return;
    }
    if h.read::<u8>(w + 0xaa) == 1 {
        let ch = h.read::<u32>(w + 0x9c);
        if ch != 0 {
            call(h, 0x0083_5b50, &[ch, slot]);
        }
    }
    #[allow(clippy::cast_possible_truncation)] // the original stores BL
    h.write(w + 0xaf, slot as u8);
    let id = u32::from(h.read::<u16>(w + worker::ID));
    call(h, 0x0088_5fd0, &[slot, id]);
}

/// `Native_SetPartyIndex` (`0x0085cb40`, `0x0042`): `(i)`: a type-1 worker
/// binds character `i` (`+0x40`, `0x00872160`); `0x008712a0(worker, i ==
/// 0)`, `0x0086a0e0()`, then the party slot (`i`, `0xf` from 8); `i`.
pub fn native_set_party_index<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let c = super::atel_get_cur_ctrl_work(h);
    let v = pop(h, s);
    if kind(h, w) == 1 {
        h.write(w + worker::BIND_PC, v);
        call(h, 0x0087_2160, &[w]);
    }
    call(h, 0x0087_12a0, &[w, u32::from(v == 0)]);
    super::helpers::atel_uc_restore(h);
    let slot = if v < 8 { v } else { 0xf };
    party_slot(h, w, c, slot);
    v
}

/// `Native_0158_PartySlotSet` (`0x0085cfa0`, `0x0158`): `(slot)`: the party slot alone;
/// `slot`.
pub fn native_0158<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let c = super::atel_get_cur_ctrl_work(h);
    party_slot(h, w, c, v);
    v
}

/// `Native_GetPitchToMoveTarget` (`0x00858d30`, `0x0020`, float): the second
/// result of `0x0088b4d0` from the position to the move target (`+0x5e0`).
pub fn native_get_pitch_to_move_target<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> f64 {
    let p = atel_get_pos_work_adrs(h, w);
    let m = move_block(h, w);
    let o = h.scratch();
    h.write(o, 0_u32);
    h.write(o + 4, w);
    let mut args = vec![o, o + 4];
    for i in 0..3 {
        args.push(x87::store(x87::load(h.read(p + i * 4))));
    }
    for i in 0..3 {
        args.push(x87::store(x87::load(h.read(m + 0x28 + i * 4))));
    }
    let f = h.rebase(0x0088_b4d0);
    h.call_n(f, &args);
    x87::load(h.read(o + 4))
}

/// `Native_PartyAdd` (`0x0085b700`, `0x00ca`): `(chr)`: `0x007869b0(chr, 1)`;
/// Kimahri (6) on map `0x43d` also gets `0x00784d00(6, 3)`; 15 sets
/// `[0x01325ac8]` = 3; `chr`.
pub fn native_party_add<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0078_69b0, &[v, 1]);
    if v == 6 {
        #[allow(clippy::cast_possible_truncation)] // the original compares AX
        if call(h, 0x0078_7360, &[]) as u16 == 0x43d {
            call(h, 0x0078_4d00, &[v, 3]);
        }
    } else if v == 0xf {
        h.write(h.rebase(0x0132_5ac8), 3_u32);
    }
    v
}

/// `Native_TriggerSetLine` (`0x0085f370`, `0x0054`): `(x0, y0, z0, x1, y1,
/// z1)`: for line triggers (types 2, 5) the two ends (`+0`, `+0x30`),
/// `+0x1c` = `[0x00b92340]`, `+0x34` |= `0x80`; 1, else 0.
pub fn native_trigger_set_line<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let p = atel_get_pos_work_adrs(h, w);
    let v: Vec<u32> = (0..6).map(|_| popf(h, s)).collect();
    if !matches!(kind(h, w), 2 | 5) {
        return 0;
    }
    for (o, i) in [(0, 5), (4, 4), (8, 3), (0x30, 2), (0x34, 1), (0x38, 0)] {
        h.write(p + o, cp(v[i]));
    }
    let r = h.read::<u32>(h.rebase(0x00b9_2340));
    h.write(p + 0x1c, cp(r));
    let f = h.read::<u16>(w + 0x34);
    h.write(w + 0x34, f | 0x80);
    let f = h.read::<u16>(w + worker::FLAGS2);
    h.write(w + worker::FLAGS2, f | 1);
    1
}

/// `Native_LineExtend` (`0x0085c130`, `0x007a`): `(back, front)`: a line
/// trigger's (types 2, 5) ends moved out along the line by `back` and
/// `front` (divided by its length, `CIsqrt(Math_LengthSq3(..))`); 1, else 0.
#[allow(clippy::similar_names)]
pub fn native_line_extend<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let p = atel_get_pos_work_adrs(h, w);
    let a = popf(h, s);
    let b = popf(h, s);
    if !matches!(kind(h, w), 2 | 5) {
        return 0;
    }
    let rd = |h: &H, o: u32| r32(x87::load(h.read(p + o)));
    let (x0, y0, z0) = (rd(h, 0), rd(h, 4), rd(h, 8));
    let (x1, y1, z1) = (rd(h, 0x30), rd(h, 0x34), rd(h, 0x38));
    let dx = r32(x87::sub(x1, x0));
    let dy = r32(x87::sub(y1, y0));
    let dz = r32(x87::sub(z1, z0));
    let sq = callf(
        h,
        0x0088_c3c0,
        &[x87::store(dx), x87::store(dy), x87::store(dz)],
    );
    let len = r32(callf(h, 0x0088_c400, &[x87::store(sq)]));
    if len > 0.0 {
        let b = r32(x87::div(x87::load(b), len));
        let a = r32(x87::div(x87::load(a), len));
        h.write(p, x87::store(x87::sub(x0, x87::mul(b, dx))));
        h.write(p + 4, x87::store(x87::sub(y0, x87::mul(b, dy))));
        h.write(p + 8, x87::store(x87::sub(z0, x87::mul(b, dz))));
        h.write(p + 0x30, x87::store(x87::add(x87::mul(a, dx), x1)));
        h.write(p + 0x34, x87::store(x87::add(x87::mul(dy, a), y1)));
        h.write(p + 0x38, x87::store(x87::add(x87::mul(dz, a), z1)));
    }
    let f = h.read::<u16>(w + worker::FLAGS2);
    h.write(w + worker::FLAGS2, f | 1);
    1
}

/// `Worker_Detach` (`0x0085ca00`, `0x019d`): `(v)`: ends the worker's
/// attachment by `+0xaa` (1: its character's bind (`0x00862780` on slots
/// 0 and 1, `0x008722f0`, `+0x38` bit 4 cleared, `+0x9c` = 0; a nonzero
/// `v` also `0x00824f00(+0xa8, 0)`; the original's `dbgPrintf` line, empty
/// in this build, is left out); 2: the camera
/// (`0x007ba9c0`); 4, 5: `0x0086f4c0`); `+0xaa`, `+0xa8` = 0; 0.
pub fn worker_detach<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    match h.read::<u8>(w + 0xaa) {
        4 | 5 => {
            let f = h.scratch();
            super::helpers::atel_set_bind_object(h, w, f);
        }
        1 => {
            if h.read::<u32>(w + 0x9c) != 0 {
                for i in 0..2 {
                    atel_push_stack_integer(h, s, i);
                    let x = pop(h, s);
                    call(h, 0x0086_2780, &[w, x]);
                }
                let c = h.read::<u32>(w + 0x9c);
                call(h, 0x0087_22f0, &[c]);
                let f = h.read::<u16>(w + 0x38);
                h.write(w + 0x38, f & 0xffef);
                if v != 0 {
                    let m = u32::from(h.read::<u16>(w + 0xa8));
                    call(h, 0x0082_4f00, &[m, 0]);
                }
                h.write(w + 0x9c, 0_u32);
            }
        }
        2 => {
            call(h, 0x007b_a9c0, &[w]);
        }
        _ => {}
    }
    h.write(w + 0xaa, 0_u8);
    h.write(w + 0xa8, 0_u16);
    0
}

/// `Native_AttachCamera` (`0x0085e920`, `0x0002`): `(camera, mode, _)`:
/// [`worker_detach`] first, then `+0xaa` = 2, `+0xa8` = camera,
/// `0x007ba9a0(worker, camera, mode)`; camera 1 sets `+0x36` bit 4 (else
/// cleared); trigger workers get mode 2 (`+0x34` bits 13-14); 0.
pub fn native_attach_camera<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    pop(h, s);
    let mode = pop(h, s);
    let cam = pop(h, s);
    atel_push_stack_integer(h, s, 0);
    worker_detach(h, w, a, s);
    h.write(w + 0xaa, 2_u8);
    #[allow(clippy::cast_possible_truncation)] // the original stores DI
    h.write(w + 0xa8, cam as u16);
    call(h, 0x007b_a9a0, &[w, cam, mode]);
    let f = h.read::<u16>(w + worker::FLAGS2);
    h.write(
        w + worker::FLAGS2,
        if cam == 1 { f | 0x10 } else { f & 0xffef },
    );
    atel_push_stack_integer(h, s, 2);
    let m = pop(h, s);
    if matches!(kind(h, w), 2 | 3 | 5 | 6) {
        let f = h.read::<u16>(w + 0x34);
        #[allow(clippy::cast_possible_truncation)] // the original works on AX
        let x = ((m << 13) as u16 ^ f) & 0x6000;
        h.write(w + 0x34, f ^ x);
    }
    0
}

/// `Native_SetPitch` (`0x0085f460`, `0x0096`, float): `(pitch)`:
/// `0x00863fe0(worker, -1)`, the wrapped pitch into the position's `+0x24`
/// and `+0x30`; attached workers (`+0xaa` 4, 5) `0x0086f4c0`; `ST0` it.
pub fn native_set_pitch<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let p = atel_get_pos_work_adrs(h, w);
    call(h, 0x0086_3fe0, &[w, u32::MAX]);
    let v = popf(h, s);
    let y = x87::store(callf(h, super::native::MATH_WRAP_ANGLE, &[v]));
    h.write(p + 0x24, y);
    h.write(p + 0x30, y);
    if h.read::<u8>(w + 0xaa).wrapping_sub(4) <= 1 {
        let f = h.scratch();
        super::helpers::atel_set_bind_object(h, w, f);
    }
    x87::load(y)
}

/// `Native_01d1` (`0x0085c5b0`, `0x00d1`): `0x00863e10(worker, i)` for the
/// nine levels (the worker id pushed and popped back first); 0.
pub fn native_00d1<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let id = u32::from(h.read::<u16>(w + worker::ID));
    atel_push_stack_integer(h, s, id);
    let id = pop(h, s);
    let o = work(h, id);
    for i in 0..9 {
        call(h, 0x0086_3e10, &[o, i]);
    }
    0
}

/// `Native_01ba_ChrGetEquipModel` (`0x0085beb0`, `0x01ba`): `(worker, on)`: the worker's
/// bound character (0..=7) gets `AtelGetElemId(chr, on)` (`on` clamped to
/// 0..=1); else 0.
pub fn native_01ba<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s).cast_signed().clamp(0, 1).cast_unsigned();
    let i = pop(h, s);
    let o = work(h, i);
    let pc = h.read::<u32>(o + worker::BIND_PC);
    if pc > 7 {
        return 0;
    }
    super::helpers::atel_get_elem_id(h, pc, v)
}

/// `Native_015b_GiveEventReward_Result` (`0x0085b4c0`): the current ctrl work's
/// `+3` bit 2 cleared, `0x0086a130()`; 1.
pub fn native_015b_result<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    let c = super::atel_get_cur_ctrl_work(h);
    let b = h.read::<u8>(c + 3);
    h.write(c + 3, b & 0xfb);
    super::helpers::atel_uc_resume(h);
    1
}

/// `Native_015b_GiveEventReward_Poll` (`0x0085af70`): `Native_MsgWaitEx_Poll`, then the
/// args' state `+0xa`: 0 waits for its done bit (then 1); 1 (with `+8`
/// set: `0x00821750(0x10001)`) becomes 2; 2 ends (1); 0 otherwise.
pub fn native_015b_poll<H: NativeHost>(h: &mut H, w: u32, a: u32, _s: u32) -> u32 {
    let r = call(h, 0x0085_ae90, &[w, a]);
    match h.read::<u16>(a + 0xa) {
        0 => {
            if r & 1 != 0 {
                h.write(a + 0xa, 1_u16);
            }
            0
        }
        1 => {
            if h.read::<u16>(a + 8) != 0 {
                call(h, 0x0082_1750, &[0x1_0001]);
            }
            h.write(a + 0xa, 2_u16);
            0
        }
        2 => 1,
        _ => 0,
    }
}

/// `Native_MusicFadeOut` (`0x0085e290`, `0x0188`): `(frames)`: 0 or less
/// stops the current music (`0x00886a60`, `0x00886800`); else fades it
/// (`0x00886af0`), then restarts the music saved in `[0x00c526d0]` (low
/// byte the track, next the volume) once; current music -1; `frames`.
pub fn native_music_fade_out<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let i = call(h, 0x0087_3b00, &[]);
    if v.cast_signed() <= 0 {
        call(h, 0x0088_6a60, &[i]);
        call(h, 0x0088_6800, &[i]);
        call(h, 0x0087_3d00, &[u32::MAX]);
        return v;
    }
    if i.cast_signed() >= 0 {
        let j = call(h, 0x0087_3b00, &[]);
        call(h, 0x0088_6af0, &[j, v]);
        call(h, 0x0087_3d00, &[u32::MAX]);
    }
    let saved = h.rebase(0x00c5_26d0);
    let x = h.read::<u32>(saved);
    if x != u32::MAX {
        let t = x & 0xff;
        let vol = (x.cast_signed() >> 8).cast_unsigned() & 0xff;
        for f in [0x0088_6960, 0x0088_69e0, 0x0088_6a20] {
            call(h, f, &[t]);
        }
        call(h, 0x0088_6ab0, &[t, vol]);
        call(h, 0x0087_3d00, &[t]);
        h.write(saved, u32::MAX);
    }
    v
}

/// `Native_TriggerSetPoint` (`0x008603e0`, `0x0057`): `(x, y, z)`: for
/// point triggers (types 3, 6) the position, `+0x34` |= `0x80`; 1, else 0.
pub fn native_trigger_set_point<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let p = atel_get_pos_work_adrs(h, w);
    let z = popf(h, s);
    let y = popf(h, s);
    let x = popf(h, s);
    if !matches!(kind(h, w), 3 | 6) {
        return 0;
    }
    h.write(p, cp(x));
    h.write(p + 4, cp(y));
    h.write(p + 8, cp(z));
    let f = h.read::<u16>(w + 0x34);
    h.write(w + 0x34, f | 0x80);
    let f = h.read::<u16>(w + worker::FLAGS2);
    h.write(w + worker::FLAGS2, f | 1);
    1
}

/// `Native_00f6_WaitWorkerSignal` (`0x00857df0`, start): `(n, worker)` into the args words
/// (`n` at least 1); `EAX` the args address.
pub fn native_00f6<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    let n = pop(h, s);
    h.write(a, i);
    h.write(a + 4, if n.cast_signed() > 0 { n } else { 1 });
    a
}

/// `Native_00f6_WaitWorkerSignal_Poll` (`0x00857ff0`): done (5) for a negative worker or
/// once `0x00863520(worker, n)` is 0.
pub fn native_00f6_poll<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    let i = h.read::<u32>(a);
    let n = h.read::<u32>(a + 4);
    if i.cast_signed() < 0 || super::helpers::atel_has_signal_from_level(h, i, n) == 0 {
        5
    } else {
        0
    }
}

/// `Native_0055_TriggerSetRadius` (`0x0085fa20`, `0x0055`): `(r)`: for triggers (types 2, 3,
/// 5, 6) the position's `+0x1c`; 1, else 0.
pub fn native_0055<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let p = atel_get_pos_work_adrs(h, w);
    let v = popf(h, s);
    if !matches!(kind(h, w), 2 | 3 | 5 | 6) {
        return 0;
    }
    h.write(p + 0x1c, cp(v));
    let f = h.read::<u16>(w + worker::FLAGS2);
    h.write(w + worker::FLAGS2, f | 1);
    1
}

/// The position differences `a - b` of two workers' position blocks.
fn diff(h: &impl NativeHost, pa: u32, pb: u32, o: u32) -> f64 {
    let x = x87::load(h.read(pa + o));
    let y = x87::load(h.read(pb + o));
    r32(x87::sub(x, y))
}

/// `Native_00a4_WorkerDistance` (`0x008566c0`, `0x00a4`, float): `(a, b)`: the distance
/// between workers `a` and `b`.
pub fn native_00a4<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    let a = pop(h, s);
    let b = pop(h, s);
    let ob = work(h, b);
    let oa = work(h, a);
    let pb = atel_get_pos_work_adrs(h, ob);
    let pa = atel_get_pos_work_adrs(h, oa);
    let (dx, dy, dz) = (diff(h, pb, pa, 0), diff(h, pb, pa, 4), diff(h, pb, pa, 8));
    let q = x87::add(
        x87::add(x87::mul(dx, dx), x87::mul(dy, dy)),
        x87::mul(dz, dz),
    );
    r32(x87::sqrt(q))
}

/// `Native_00a5_WorkerDistanceXZ` (`0x00856990`, `0x00a5`, float): `(a, b)`: as `0x00a4` in
/// the x-z plane.
pub fn native_00a5<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    let a = pop(h, s);
    let b = pop(h, s);
    let ob = work(h, b);
    let oa = work(h, a);
    let pb = atel_get_pos_work_adrs(h, ob);
    let pa = atel_get_pos_work_adrs(h, oa);
    let (dx, dz) = (diff(h, pb, pa, 0), diff(h, pb, pa, 8));
    r32(x87::sqrt(x87::add(x87::mul(dx, dx), x87::mul(dz, dz))))
}

/// `Native_MusicLoad` (`0x00858e50`, start): `(track)` into the args; it.
pub fn native_music_load<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    h.write(a, v);
    v
}

/// `Native_MusicLoad_Poll` (`0x00859040`): done (5) once `[0x012fb808]`
/// is 1 or less.
pub fn native_music_load_poll<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    if h.read::<u32>(h.rebase(0x012f_b808)).cast_signed() > 1 {
        0
    } else {
        5
    }
}

/// `Native_MusicLoad_Result` (`0x00859250`): `0x00886960(track)`; track.
pub fn native_music_load_result<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    let v = h.read::<u32>(a);
    call(h, 0x0088_6960, &[v]);
    v
}

/// `Native_MusicWaitLoaded` (`0x00859d70`, start): nothing (`EAX` as it was;
/// the dispatch ignores it).
pub fn native_music_wait_loaded<H: NativeHost>(_h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    0
}

/// `Native_MusicWaitLoaded_Poll` (`0x00859df0`): done (5) at once.
pub fn native_music_wait_loaded_poll<H: NativeHost>(_h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    5
}

/// `Native_MusicWaitLoaded_Result` (`0x00859ee0`): 1.
pub fn native_music_wait_loaded_result<H: NativeHost>(
    _h: &mut H,
    _w: u32,
    _a: u32,
    _s: u32,
) -> u32 {
    1
}

/// `Native_017a_SetEffectFlag` (`0x0085d210`, `0x017a`): `(v)`: `0x0086fa40(v)`; on map
/// `0x5e` in room `0x186`, 10 clears and 9 sets `[0x012fb8b4]`; `v`.
pub fn native_017a<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    if h.read::<u32>(h.rebase(CURRENT_MAP)) == 0x5e {
        #[allow(clippy::cast_possible_truncation)] // the original compares AX
        if call(h, 0x0078_7360, &[]) as u16 == 0x186 {
            if v == 0xa {
                h.write(h.rebase(0x012f_b8b4), 0_u32);
            } else if v == 9 {
                h.write(h.rebase(0x012f_b8b4), 1_u32);
            }
        }
    }
    call(h, 0x0086_fa40, &[v]);
    v
}

/// A handler that only jumps to `f` with the same arguments.
fn tail<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32, f: u32) -> u32 {
    call(h, f, &[w, a, s])
}

/// `Native_0087_GetNowEventJumpId` (`0x0085cf90`, `0x0087`): jumps to `0x0086c1c0`.
pub fn native_0087<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0086_c1c0)
}

/// `Native_0086_GetEventJumpMap` (`0x0085cdb0`, `0x0086`): jumps to `0x0086c1f0`.
pub fn native_0086<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0086_c1f0)
}

/// `Native_0088_GetLastEventJumpMapNo` (`0x0085d160`, `0x0088`): jumps to `0x0086bca0`.
pub fn native_0088<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0086_bca0)
}

/// `Native_010e_GetGlobal011307a0` (`0x0085af50`, `0x010e`): jumps to `0x00906b10`.
pub fn native_010e<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0090_6b10)
}

/// `Native_002a_TurnSetPitch` (`0x00859bb0`, `0x002a`, float): `(pitch)`: wrapped into
/// the turn block's `+0x28`, and the wrapped rest to the position's pitch
/// (`+0x38`) into `+0x34`; `ST0` the pitch.
pub fn native_002a<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let p = atel_get_pos_work_adrs(h, w);
    let v = popf(h, s);
    let y = x87::store(callf(h, super::native::MATH_WRAP_ANGLE, &[v]));
    let t = w + 0x864 + lvl(h, w) * 0x44;
    h.write(t + 0x28, y);
    let d = x87::store(x87::sub(x87::load(h.read(p + 0x38)), x87::load(y)));
    let r = x87::store(callf(h, super::native::MATH_WRAP_ANGLE, &[d]));
    h.write(t + 0x34, r);
    x87::load(y)
}

/// `Native_0059_TriggerSetPoint` (`0x00855cd0`, `0x0059`): `(r, h, z1)`: for point triggers
/// (types 3, 6) the position's `+0x30` = z1, `+0x38` = h, `+0x1c` = r; 1,
/// else 0.
pub fn native_0059<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let p = atel_get_pos_work_adrs(h, w);
    let a = popf(h, s);
    let b = popf(h, s);
    let c = popf(h, s);
    if !matches!(kind(h, w), 3 | 6) {
        return 0;
    }
    h.write(p + 0x30, cp(c));
    h.write(p + 0x38, cp(b));
    h.write(p + 0x1c, cp(a));
    let f = h.read::<u16>(w + worker::FLAGS2);
    h.write(w + worker::FLAGS2, f | 1);
    1
}

/// `Native_Random16` (`0x00857680`, `0x00a9`): `0x007988f0(2)`'s low word.
pub fn native_random16<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call(h, 0x0079_88f0, &[2]) & 0xffff
}

/// `Native_013e_WorkerSetFlag36Bit15` (`0x00856890`, `0x013e`): `(on)`: `+0x36` bit 15; `on != 0`.
pub fn native_013e<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = u32::from(pop(h, s) != 0);
    let f = h.read::<u16>(w + worker::FLAGS2);
    #[allow(clippy::cast_possible_truncation)] // a bit
    h.write(w + worker::FLAGS2, (f & 0x7fff) | ((v as u16) << 15));
    v
}

/// Bit `i` of `0x00888c40(0, 0, x)`'s low word.
fn flag_bit<H: NativeHost>(h: &mut H, x: u32, i: u32) -> u32 {
    let r = call(h, 0x0088_8c40, &[0, 0, x & 0xffff]) & 0xffff;
    u32::from(r & (1 << (i & 31)) != 0)
}

/// `Native_004c_PadIsHeld` (`0x0085e130`, `0x004c`): `(i)`: bit `i` of the flag
/// word for `0x0086af30()`.
pub fn native_004c<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    let x = call(h, 0x0086_af30, &[]);
    flag_bit(h, x, i)
}

/// `Native_004d_PadIsPressed` (`0x0085e820`, `0x004d`): `(i)`: bit `i` of the flag
/// word for `0x0086af70()`; 0 for worker 11 on story `0x31` with
/// `[0x0112d6c9]` bit 5.
pub fn native_004d<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    if h.read::<u32>(h.rebase(0x012f_bc40)) == 0x31
        && h.read::<u8>(h.rebase(0x0112_d6c9)) & 0x20 != 0
        && h.read::<u16>(w + worker::ID) == 0xb
    {
        return 0;
    }
    let x = call(h, 0x0086_af70, &[]);
    flag_bit(h, x, i)
}

/// `Native_SetMapMarkerPos` (`0x0085c2d0`, `0x016e`): `(x, y)`: the record
/// `0x0088c830()`'s `+0xb4` = x, `+0xb6` = y; 1.
pub fn native_set_map_marker_pos<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let g = call(h, 0x0088_c830, &[]);
    let y = pop(h, s);
    let x = pop(h, s);
    #[allow(clippy::cast_possible_truncation)] // the original stores AX, DI
    {
        h.write(g + 0xb4, x as u16);
        h.write(g + 0xb6, y as u16);
    }
    1
}

/// `(track)`: the current music cleared when it is `track`, then `f(track)`;
/// 1.
fn music_end<H: NativeHost>(h: &mut H, s: u32, f: u32) -> u32 {
    let v = pop(h, s);
    if call(h, 0x0087_3b00, &[]) == v {
        call(h, 0x0087_3d00, &[u32::MAX]);
    }
    call(h, f, &[v]);
    1
}

/// `Native_0103_MusicClearIfTrack` (`0x008593a0`, `0x0103`): `0x00886800`.
pub fn native_0103<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    music_end(h, s, 0x0088_6800)
}

/// `Native_016d_MusicStop` (`0x008595b0`, `0x016d`): `0x00886a60`.
pub fn native_016d<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    music_end(h, s, 0x0088_6a60)
}

/// `Native_01e9_MsgCloseWait` (`0x008590f0`, start): `(window, wait)` into the args;
/// `0x008640f0(window)`.
pub fn native_01e9<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    let b = pop(h, s);
    let m = pop(h, s);
    h.write(a, m);
    h.write(a + 4, b);
    call(h, 0x0086_40f0, &[m])
}

/// `Native_01e9_MsgCloseWait_Poll` (`0x008592c0`): done (5) without the wait flag or
/// once the window's state (`+0x14`) is 0 or 4.
pub fn native_01e9_poll<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    let i = h.read::<u32>(a);
    let m = super::atel_get_mes_win_work(h, i.cast_signed());
    if h.read::<u32>(a + 4) == 0 {
        return 5;
    }
    match h.read::<u16>(m + 0x14) {
        0 | 4 => 5,
        _ => 0,
    }
}

/// `Native_01e9_Result` (`0x008595f0`): the window.
pub fn native_01e9_result<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    h.read(a)
}

/// `Native_MinimapSetCorner` (`0x00857c40`, `0x00f3`): `0x00919e20(v)`; `v`.
pub fn native_minimap_set_corner<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0091_9e20, &[v]);
    v
}

/// `Native_00a8_WorkerSetFlag34Bit1` (`0x00857950`, `0x00a8`): `(worker)`: its `+0x34` bit 1
/// set; 1.
pub fn native_00a8<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    let o = work(h, i);
    let f = h.read::<u16>(o + 0x34);
    h.write(o + 0x34, f | 2);
    1
}

/// `Native_014a_SetBattleEndStatus` (`0x00858740`, `0x014a`): `(c, b, a)`:
/// `0x0086fc20(c, b, a)`; `c`.
pub fn native_014a<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    let c = pop(h, s);
    call(h, 0x0086_fc20, &[c, b, a]);
    c
}

/// `Native_0215_SetLegendAbility` (`0x0085d170`, `0x0215`): `(i, v)`: for `i` in 0..=17,
/// `0x008c3170(i, v)` with `v` clamped to 1..=2; else 0.
pub fn native_0215<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let i = pop(h, s);
    if i.cast_signed() > 0x11 || i.cast_signed() < 0 {
        return 0;
    }
    let v = v.cast_signed().clamp(1, 2).cast_unsigned();
    call(h, 0x008c_3170, &[i, v])
}

/// `Native_0132_WorkerTriggerSetActive` (`0x0085fe30`, `0x0132`): `(worker, on)`: for a trigger
/// worker (types 2, 3, 5, 6) `+0x34` bit 7 = on; 1, else 0.
pub fn native_0132<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = u16::from(pop(h, s) != 0);
    let i = pop(h, s);
    let o = work(h, i);
    if !matches!(kind(h, o), 2 | 3 | 5 | 6) {
        return 0;
    }
    let f = h.read::<u16>(o + 0x34);
    h.write(o + 0x34, f ^ (((v << 7) ^ f) & 0x80));
    let f = h.read::<u16>(o + worker::FLAGS2);
    h.write(o + worker::FLAGS2, f | 1);
    1
}

/// `Native_00f2_MinimapSetDrawFlag` (`0x00857850`, `0x00f2`): `(v)` -> `0x00870190(v)`.
pub fn native_00f2<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0087_0190, &[v])
}

/// `Native_015d_MinimapSetBlizFlag` (`0x00857a80`, `0x015d`): `(v)` -> `0x00870160(v)`.
pub fn native_015d<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0087_0160, &[v])
}

/// `Native_0225` (`0x0085ea10`, `0x0225`): `0x00870f60(v)`; `v`.
pub fn native_0225<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0087_0f60, &[v]);
    v
}

/// `Native_012d_SaveCounterGet` (`0x0085df80`, `0x012d`): `(i)`: word `i` (clamped to
/// 0..=7) of the record `0x0088c830()`'s `+0x2c`.
pub fn native_012d<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let g = call(h, 0x0088_c830, &[]);
    let i = v.cast_signed().clamp(0, 7).cast_unsigned();
    h.read(g + 0x2c + i * 4)
}

/// `Native_00ce_WorkerSetFieldB16` (`0x0085c2b0`, `0x00ce`): `(v)` into `+0xb16`; `v`.
pub fn native_00ce<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    #[allow(clippy::cast_possible_truncation)] // the original stores AX
    h.write(w + 0xb16, v as u16);
    v
}

/// `Native_017e_MusicCrossStart` (`0x0085d830`, `0x017e`): `(track, v)`:
/// `0x00886830(track, v)` when `0x008863f0()` is not negative, else
/// `0x008869e0` and `0x00886a20`; current music = track; 1.
pub fn native_017e<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let t = pop(h, s);
    if call(h, 0x0088_63f0, &[]).cast_signed() >= 0 {
        call(h, 0x0088_6830, &[t, v]);
    } else {
        call(h, 0x0088_69e0, &[t]);
        call(h, 0x0088_6a20, &[t]);
    }
    call(h, 0x0087_3d00, &[t]);
    1
}

/// `Native_0025_PosSetYaw` (`0x00859d80`, `0x0025`, float): `(yaw)` wrapped into the
/// position's `+0x34`; `ST0` it.
pub fn native_0025<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let p = atel_get_pos_work_adrs(h, w);
    let v = popf(h, s);
    let y = x87::store(callf(h, super::native::MATH_WRAP_ANGLE, &[v]));
    h.write(p + 0x34, y);
    x87::load(y)
}

/// `Native_0197_WorkerAttachDobj` (`0x0085edd0`, `0x0197`): `(v)`: `+0xa8` = v, `+0xaa` =
/// 5; 0.
pub fn native_0197<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    #[allow(clippy::cast_possible_truncation)] // the original stores AX
    h.write(w + 0xa8, v as u16);
    h.write(w + 0xaa, 5_u8);
    0
}

/// `Native_021b_CheckCaptMonster` (`0x0085d8c0`, `0x021b`): `(b, a)` ->
/// `0x00872aa0(1, b, a, 0)`.
pub fn native_021b<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    call(h, 0x0087_2aa0, &[1, b, a, 0])
}

/// `Native_0128_WorkerInRadius` (`0x0085d7b0`, `0x0128`): `(worker, angle)` ->
/// `0x0088b730(angle, the offset from that worker)`.
pub fn native_0128<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let f = popf(h, s);
    let i = pop(h, s);
    let o = work(h, i);
    let pw = atel_get_pos_work_adrs(h, w);
    let po = atel_get_pos_work_adrs(h, o);
    let d: Vec<u32> = [0, 4, 8]
        .iter()
        .map(|&k| x87::store(diff(h, pw, po, k)))
        .collect();
    call(h, 0x0088_b730, &[cp(f), d[0], d[1], d[2]])
}

/// `Native_0069_MsgWinSetBytes1e` (`0x008589d0`, `0x0069`): `(window, b, a)`: the window's
/// `+0x1e` = b, `+0x1f` = a; `window`.
pub fn native_0069<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    let i = pop(h, s);
    let m = super::atel_get_mes_win_work(h, i.cast_signed());
    #[allow(clippy::cast_possible_truncation)] // the original stores BL, CL
    {
        h.write(m + 0x1e, b as u8);
        h.write(m + 0x1f, a as u8);
    }
    i
}

/// `Native_0139_MsgChoice` (`0x0085f730`, start): `0x00860b00(worker, args, stack, 0)`.
pub fn native_0139<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    call(h, 0x0086_0b00, &[w, a, s, 0])
}

/// `Native_0139_MsgChoice_Poll` (`0x0085f850`): jumps to `Native_MsgSelect_Poll`.
pub fn native_0139_poll<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0085_9c50)
}

/// `Native_0139_MsgChoice_Result` (`0x0085f900`): `Native_MsgSelect_Result`, then the
/// first args word pushed and popped back and handed to `0x008b7c40`.
pub fn native_0139_result<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    call(h, 0x0085_a6f0, &[w, a, s]);
    let v = h.read::<u32>(a);
    atel_push_stack_integer(h, s, v);
    let v = pop(h, s);
    call(h, 0x008b_7c40, &[v])
}

/// `Native_00b1_MoveSetField18` (`0x00856a50`, `0x00b1`, float): `(v)` into the move
/// block's `+0x18`; `ST0` it.
pub fn native_00b1<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let m = move_block(h, w);
    let v = popf(h, s);
    let v = cp(v);
    h.write(m + 0x18, v);
    x87::load(v)
}

/// `Native_00e9_CtrlSetByte1Bit4` (`0x00856660`, `0x00e9`): `(on)`: the current ctrl
/// work's `+1` bit 4; `on != 0`.
pub fn native_00e9<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let c = super::atel_get_cur_ctrl_work(h);
    let v = u32::from(pop(h, s) != 0);
    let b = h.read::<u8>(c + 1);
    #[allow(clippy::cast_possible_truncation)] // a bit
    h.write(c + 1, b ^ ((((v as u8) << 4) ^ b) & 0x10));
    v
}

/// `Native_0117_WorkerSetFlag36Bits` (`0x0085bc50`, `0x0117`): `(worker)`: its `+0x36` bit 10
/// cleared, bits 9 and 12 set; `worker`.
pub fn native_0117<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let o = work(h, v);
    let f = h.read::<u16>(o + worker::FLAGS2);
    h.write(o + worker::FLAGS2, (f & 0xfbff) | 0x1200);
    v
}

/// `Native_LangGroupNotJpAsia` (`0x00858210`, `0x0253`): 1 unless the
/// language group (`0x00641100`) is 0, 3 or 4.
pub fn native_lang_group_not_jp_asia<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    u32::from(!matches!(call(h, 0x0064_1100, &[]), 0 | 3 | 4))
}

/// `Native_01da_VibrateStartLevel3` (`0x00856a80`, `0x01da`): `(d, c, b, a)`:
/// `0x00876050(3, d, c, b, a)`; 1.
pub fn native_01da<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    let c = pop(h, s);
    let d = pop(h, s);
    call(h, 0x0087_6050, &[3, d, c, b, a]);
    1
}

/// `Native_0181` (`0x0085e550`, `0x0181`, float): the position's `+0x38`.
pub fn native_0181<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> f64 {
    let p = atel_get_pos_work_adrs(h, w);
    x87::load(h.read(p + 0x38))
}

/// `Native_015f_ImportantSet` (`0x0085b650`, `0x015f`): `0x0088e790(v)`; `v`.
pub fn native_015f<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0088_e790, &[v]);
    v
}

/// A message window's `+0x28` = `x`; the window.
fn win_28<H: NativeHost>(h: &mut H, s: u32, x: u32) -> u32 {
    let i = pop(h, s);
    let m = super::atel_get_mes_win_work(h, i.cast_signed());
    h.write(m + 0x28, x);
    i
}

/// `Native_0257` (`0x008588b0`, `0x0257`): `(window)`: its `+0x28` = 0.
pub fn native_0257<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    win_28(h, s, 0)
}

/// `Native_0256` (`0x00858710`, `0x0256`): `(window)`: its `+0x28` =
/// `0x8000`.
pub fn native_0256<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    win_28(h, s, 0x8000)
}

/// `Native_0093_GetWorkerYaw` (`0x0085e8f0`, `0x0093`, float): `(worker)`: its yaw
/// (position `+0x34`).
pub fn native_0093<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    let i = pop(h, s);
    let o = work(h, i);
    let p = atel_get_pos_work_adrs(h, o);
    x87::load(h.read(p + 0x34))
}

/// `Native_0130_SaveCounterSet` (`0x0085e410`, `0x0130`): `(i, v)`: word `i` (clamped to
/// 0..=7) of the record `0x0088c830()`'s `+0x2c` = `v` (0 when negative);
/// the clamped `i`.
pub fn native_0130<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let i = pop(h, s).cast_signed().clamp(0, 7).cast_unsigned();
    let g = call(h, 0x0088_c830, &[]);
    h.write(g + 0x2c + i * 4, if v.cast_signed() < 0 { 0 } else { v });
    i
}

/// `Native_00ed` (`0x008571c0`, `0x00ed`): the worker's bound character
/// (`+0x40`).
pub fn native_00ed<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    h.read(w + worker::BIND_PC)
}

/// `Native_017f_CurMusicVolumeDirect` (`0x0085bd10`, `0x017f`): `(v)`: the current music `i`
/// (`0x00873b00`) gets `0x00886ab0(i, v)` when not negative; `i`.
pub fn native_017f<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let i = call(h, 0x0087_3b00, &[]);
    if i.cast_signed() >= 0 {
        call(h, 0x0088_6ab0, &[i, v]);
    }
    i
}

/// `Native_0136_GiveGil` (`0x0085ee40`, `0x0136`): `(v)`: 1 when
/// `0x007859a0(-v)` is not negative.
pub fn native_0136<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    u32::from(call(h, 0x0078_59a0, &[v.wrapping_neg()]).cast_signed() >= 0)
}

/// `Native_0137_PayGil` (`0x0085ef80`, `0x0137`): `(v)`: 1 when `0x007859a0(v)`
/// is not negative.
pub fn native_0137<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    u32::from(call(h, 0x0078_59a0, &[v]).cast_signed() >= 0)
}

/// `Native_0157_SndFootOffSlot` (`0x00859dc0`, `0x0157`): `0x00887100(v, 0)`; 1.
pub fn native_0157<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0088_7100, &[v, 0]);
    1
}

/// `Native_0155_SndFootOnSlot` (`0x00859d00`, `0x0155`): `0x00887100(v, 1)`; 1.
pub fn native_0155<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0088_7100, &[v, 1]);
    1
}

/// `(c, b, a)`: `f(c, b, a)`; `a`.
fn call3<H: NativeHost>(h: &mut H, s: u32, f: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    let c = pop(h, s);
    call(h, f, &[c, b, a]);
    a
}

/// `Native_0100_SndSepVolumeTranslate` (`0x00860460`, `0x0100`): `0x00886ff0(c, b, a)`; `a`.
pub fn native_0100<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    call3(h, s, 0x0088_6ff0)
}

/// `Native_016b_MusicVolumeTranslate` (`0x0085be70`, `0x016b`): `0x00886b40(c, b, a)`; `a`.
pub fn native_016b<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    call3(h, s, 0x0088_6b40)
}

/// `Native_005a_TriggerSetPointYaw` (`0x00856170`, `0x005a`): `(yaw)`: for point triggers
/// (types 3, 6) the position's `+0x34`; 0.
pub fn native_005a<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let p = atel_get_pos_work_adrs(h, w);
    let v = popf(h, s);
    if matches!(kind(h, w), 3 | 6) {
        h.write(p + 0x34, cp(v));
        let f = h.read::<u16>(w + worker::FLAGS2);
        h.write(w + worker::FLAGS2, f | 1);
    }
    0
}

/// `Native_00be_CtrlSetBit7` (`0x00859950`, `0x00be`): `(on)`: the current ctrl
/// work's bit 7; `on != 0`.
pub fn native_00be<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let c = super::atel_get_cur_ctrl_work(h);
    let v = u32::from(pop(h, s) != 0);
    let b = h.read::<u8>(c);
    #[allow(clippy::cast_possible_truncation)] // a bit
    h.write(c, (b & 0x7f) | ((v as u8) << 7));
    v
}

/// `Native_01a4_DobjSetMimeParam1` (`0x00857360`, `0x01a4`): three zeros pushed, four floats
/// popped (the zeros and one script float) and an integer `i`:
/// `0x0090a4d0(i, &[the float, 0, 0, 0])`; 1.
pub fn native_01a4<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    for _ in 0..3 {
        atel_push_stack_integer(h, s, 0);
    }
    let f1 = popf(h, s);
    let f2 = popf(h, s);
    let f3 = popf(h, s);
    let f4 = popf(h, s);
    let i = pop(h, s);
    let o = h.scratch();
    for (k, v) in [f4, f3, f2, f1].into_iter().enumerate() {
        #[allow(clippy::cast_possible_truncation)] // 0..4
        h.write(o + k as u32 * 4, cp(v));
    }
    call(h, 0x0090_a4d0, &[i, o]);
    1
}

/// `(v)`: the low byte of `0x00888c00(0, 0, x)` for `x = f(v)`.
fn byte_888c00<H: NativeHost>(h: &mut H, x: u32) -> u32 {
    call(h, 0x0088_8c00, &[0, 0, x]) & 0xff
}

/// `Native_0121_PadGetBindFlag` (`0x0085d1d0`, `0x0121`): `(i)` clamped to 0..=13: the
/// flag byte of `0x00888cf0(0, 0, i)`.
pub fn native_0121<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s).cast_signed().clamp(0, 0xd).cast_unsigned();
    let x = call(h, 0x0088_8cf0, &[0, 0, v]);
    byte_888c00(h, x)
}

/// `Native_0122_PadGetAnalog` (`0x0085d310`, `0x0122`): `(i)` clamped to 0..=13:
/// `0x00888c00(0, 0, i)`'s low byte.
pub fn native_0122<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s).cast_signed().clamp(0, 0xd).cast_unsigned();
    byte_888c00(h, v)
}

/// `Native_01c9_DisposeAnimDobj` (`0x00860730`, `0x01c9`): `(b, a)`: `0x009084b0(b, a)`; 1.
pub fn native_01c9<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    call(h, 0x0090_84b0, &[b, a]);
    1
}

/// `Native_009a_PosSetField28` (`0x00860390`, `0x009a`): `(v)`: for types 5 and 6 the
/// position's `+0x28` = v; `v`.
pub fn native_009a<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let p = atel_get_pos_work_adrs(h, w);
    if kind(h, w).wrapping_sub(5) <= 1 {
        #[allow(clippy::cast_possible_truncation)] // the original stores DI
        h.write(p + 0x28, v as u16);
    }
    v
}

/// `Native_00de_EventEffectWaitEnd_Poll` (`0x0085f280`): `Atel_EffectEndPoll(worker id)`
/// (`0x00863350`, which reads no argument).
pub fn native_00de_poll<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    super::helpers::atel_effect_end_poll(h)
}

/// `Native_0094_CurLevelSet5ec` (`0x0085f0b0`, `0x0094`, float): `0x0086f040(worker, v)`;
/// `ST0` `v`.
pub fn native_0094<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    set_f(h, w, s, 0x0086_f040)
}

/// `Native_01eb_VibrateSetCommon` (`0x00858070`, `0x01eb`): `0x00875fb0(1, v,
/// 0x0088d590())`; 1.
pub fn native_01eb<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let x = call(h, 0x0088_d590, &[]);
    call(h, 0x0087_5fb0, &[1, v, x]);
    1
}

/// `Native_021a_CheckCaptMonster` (`0x0085d780`, `0x021a`): `(b, a)` ->
/// `0x00872aa0(0, b, a, 0)`.
pub fn native_021a<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    call(h, 0x0087_2aa0, &[0, b, a, 0])
}

/// `Native_0207_BindEffBreathOn` (`0x0085f010`, `0x0207`): a worker with a character gets
/// `0x00862700(worker, 1)`; 1.
pub fn native_0207<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    if h.read::<u8>(w + 0xaa) == 1 && h.read::<u32>(w + 0x9c) != 0 {
        super::helpers::atel_bind_eff2_on(h, w, 1);
    }
    1
}

/// `Native_0090_PosGetField20` (`0x0085e270`, `0x0090`, float): the position's `+0x20`.
pub fn native_0090<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> f64 {
    let p = atel_get_pos_work_adrs(h, w);
    x87::load(h.read(p + 0x20))
}

/// `Native_01cc_GetNowPlayMusic` (`0x0085e400`, `0x01cc`): jumps to `0x008863f0`.
pub fn native_01cc<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0088_63f0)
}

/// `Native_019f_IsCtrlRunning` (`0x008567a0`, `0x019f`): `(v)` -> `0x0086c530(v)`.
pub fn native_019f<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0086_c530, &[v])
}

/// `Native_0196_FpBindAttached` (`0x0085fce0`, `0x0196`): `(v)`: attached workers (`+0xaa`
/// 4, 5) get `AtelBindFp(worker, v, +0xa8)`; 1.
pub fn native_0196<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    if h.read::<u8>(w + 0xaa).wrapping_sub(4) <= 1 {
        let m = u32::from(h.read::<u16>(w + 0xa8));
        super::helpers::atel_bind_fp(h, w, v, m);
    }
    1
}

/// `Native_0109_Snd3dCalc` (`0x0085a870`, `0x0109`): `(i)`: the record
/// `0x008773f0(i)` gets `0x00876930(it, 1)` when its first word is set; 1.
pub fn native_0109<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    let r = call(h, 0x0087_73f0, &[i]);
    if h.read::<u32>(r) != 0 {
        call(h, 0x0087_6930, &[r, 1]);
    }
    1
}

/// `Native_0108_Snd3dSetPos` (`0x0085a370`, `0x0108`): `(i, second, x, y, z)`: the
/// record `0x008773f0(i)` with an object (`+4`, its `+0` bit 1 cleared)
/// gets the point at `+0x14` (second) or `+8`; 1.
pub fn native_0108<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let z = popf(h, s);
    let y = popf(h, s);
    let x = popf(h, s);
    let m = pop(h, s);
    let i = pop(h, s);
    let r = call(h, 0x0087_73f0, &[i]);
    let c = h.read::<u32>(r + 4);
    if c != 0 {
        let f = h.read::<u16>(c);
        h.write(c, f & 0xfffd);
        let o = if m != 0 { 0x14 } else { 8 };
        h.write(r + o, cp(x));
        h.write(r + o + 4, cp(y));
        h.write(r + o + 8, cp(z));
    }
    1
}

/// `Native_00db_EventEffectRead` (`0x0085e9e0`, start): `(v)` into the args,
/// `0x00860fd0()`, `0x0086fa80(1)`; the current ctrl work.
pub fn native_00db<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    h.write(a, v);
    call(h, 0x0086_0fd0, &[]);
    call(h, 0x0086_fa80, &[1]);
    super::atel_get_cur_ctrl_work(h)
}

/// `Native_00db_EventEffectRead_Poll` (`0x0085eaf0`): `0x00872470(v)`, then done (5) when
/// `0x00788bd0()` is 3, or 1 on map `0x10e`.
pub fn native_00db_poll<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    let v = h.read::<u32>(a);
    call(h, 0x0087_2470, &[v]);
    let r = call(h, 0x0078_8bd0, &[]);
    let done = r == 3 || (r == 1 && h.read::<u32>(h.rebase(CURRENT_MAP)) == 0x10e);
    if done {
        super::atel_get_cur_ctrl_work(h);
        5
    } else {
        0
    }
}

/// `Native_011d_SpriteSet13` (`0x0085c650`, `0x011d`): `(b, a)`: `0x008749b0(b, a)`; `b`.
pub fn native_011d<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    call(h, 0x0087_49b0, &[b, a]);
    b
}

/// `Native_00dc_EventEffectStart` (`0x0085ee00`, start): nothing (`EAX` as it was; the
/// dispatch ignores it).
pub fn native_00dc<H: NativeHost>(_h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    0
}

/// `Native_00dc_EventEffectStart_Poll` (`0x0085ee70`): `Atel_EffectStartPoll(worker id)`
/// (`0x008633e0`; bit 0: done).
pub fn native_00dc_poll<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let id = u32::from(h.read::<u16>(w + worker::ID));
    let r = super::helpers::atel_effect_start_poll(h, id);
    if r & 1 != 0 {
        super::atel_get_cur_ctrl_work(h);
    }
    r
}

/// `Native_025c` (`0x00859400`, `0x025c`): jumps to `0x0086c270`.
pub fn native_025c<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0086_c270)
}

/// `Native_00bd_SkipRequestLevels` (`0x00859840`, `0x00bd`): `(n)` (at least 1):
/// `0x00871730(worker id, n - 1)`; `n`.
pub fn native_00bd<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let mut v = pop(h, s);
    if v.cast_signed() <= 0 {
        v = 1;
    }
    let id = u32::from(h.read::<u16>(w + worker::ID));
    call(h, 0x0087_1730, &[id, v - 1]);
    v
}

/// `Native_01ad_AddMonsterCapture` (`0x00858b30`, `0x01ad`): `(v)` -> `0x00790b90(v, 1)`.
pub fn native_01ad<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0079_0b90, &[v, 1])
}

/// `Native_01e2` (`0x00858940`, `0x01e2`): jumps to `0x009085a0`.
pub fn native_01e2<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0090_85a0)
}

/// `Native_0106_Snd3dSetVar` (`0x0085a010`, `0x0106`): `(i, b, var)`: the record
/// `0x008773f0(i)` gets `+0` = b, `+4` = `0x00a44700(the variable's
/// address)`; 1.
pub fn native_0106<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let b = pop(h, s);
    let i = pop(h, s);
    let x = atel_get_real_var_adrs(h, w, v);
    let r = call(h, 0x0087_73f0, &[i]);
    h.write(r, b);
    let y = call(h, 0x00a4_4700, &[x]);
    h.write(r + 4, y);
    1
}

/// `Native_0177` (`0x0085c9c0`, `0x0177`): `(on)`: `+0x36` bit 6 = on, and
/// bit 15 set when on; `on != 0`.
pub fn native_0177<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = u16::from(pop(h, s) != 0);
    let f = h.read::<u16>(w + worker::FLAGS2);
    let mut f = f ^ (((v << 6) ^ f) & 0x40);
    if v != 0 {
        f |= 0x8000;
    }
    h.write(w + worker::FLAGS2, f);
    u32::from(v)
}

/// `Native_0160` (`0x0085b900`, `0x0160`): `(v)` -> `0x00790800(v)`.
pub fn native_0160<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0079_0800, &[v])
}

/// `Native_0010_GetNowEventJumpIndex` (`0x00856910`, `0x0010`): jumps to `0x0086c1d0`.
pub fn native_0010<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0086_c1d0)
}

/// `Native_023c_AllHelpEventFlags` (`0x0085c600`, `0x023c`): `(b, n)`: 1 when
/// `0x0086b9e0(b + i)` is set for all `i` below `n`, else 0.
pub fn native_023c<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let n = pop(h, s);
    let b = pop(h, s);
    let mut i = 0_u32;
    while i.cast_signed() < n.cast_signed() {
        if call(h, 0x0086_b9e0, &[i.wrapping_add(b)]) == 0 {
            break;
        }
        i += 1;
    }
    u32::from(i == n)
}

/// `Native_023b_GetHelpEventFlag` (`0x0085c490`, `0x023b`): `(v)` -> `0x0086b9e0(v)`.
pub fn native_023b<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0086_b9e0, &[v])
}

/// `Native_01d0` (`0x0085fbb0`, `0x01d0`): the worker id pushed and popped
/// back; its `+0x38` bit 3 set when it has a character (`+0xaa` 1); 1.
pub fn native_01d0<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let id = u32::from(h.read::<u16>(w + worker::ID));
    atel_push_stack_integer(h, s, id);
    let id = pop(h, s);
    let o = work(h, id);
    if h.read::<u8>(o + 0xaa) == 1 {
        let f = h.read::<u16>(o + 0x38);
        h.write(o + 0x38, f | 8);
    }
    1
}

/// `Native_01ce_BindEffDrawOn` (`0x0085e7c0`, `0x01ce`): a worker with a character gets
/// `0x007fb290(chr)`, `0x007fbab0(chr)` and `+0x38` bit 4; 1.
pub fn native_01ce<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    if h.read::<u8>(w + 0xaa) == 1 {
        let c = h.read::<u32>(w + 0x9c);
        if c != 0 {
            call(h, 0x007f_b290, &[c]);
            let c = h.read::<u32>(w + 0x9c);
            call(h, 0x007f_bab0, &[c]);
            let f = h.read::<u16>(w + 0x38);
            h.write(w + 0x38, f | 0x10);
        }
    }
    1
}

/// `Native_01f9` (`0x0085ac50`, `0x01f9`): `(chr)`: `Native_00cb_PartyRemove` on it,
/// then `0x00786aa0(chr, 0)`; 1.
pub fn native_01f9<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    atel_push_stack_integer(h, s, v);
    native_00cb(h, w, a, s);
    call(h, 0x0078_6aa0, &[v, 0]);
    1
}

/// The float at `+off` of `0x0086c020(worker, i)`.
fn rec_86c020<H: NativeHost>(h: &mut H, w: u32, s: u32, off: u32) -> f64 {
    let i = pop(h, s);
    let r = call(h, 0x0086_c020, &[w, i]);
    x87::load(h.read(r + off))
}

/// `Native_00bf_GetEntranceAtX` (`0x00859d30`, `0x00bf`, float): `(i)`: `+0xc` of
/// `0x0086c020(worker, i)`.
pub fn native_00bf<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    rec_86c020(h, w, s, 0xc)
}

/// `Native_00c0_GetEntranceAtY` (`0x00859e00`, `0x00c0`, float): `+0x10`.
pub fn native_00c0<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    rec_86c020(h, w, s, 0x10)
}

/// `Native_00c1_GetEntranceAtZ` (`0x00859ef0`, `0x00c1`, float): `+0x14`.
pub fn native_00c1<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    rec_86c020(h, w, s, 0x14)
}

/// `Native_00c2_GetEntranceAtYaw` (`0x0085a070`, `0x00c2`, float): `+8`.
pub fn native_00c2<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    rec_86c020(h, w, s, 8)
}

/// `Native_017b_ClearEffectFlag` (`0x0085d3d0`, `0x017b`): `0x00863860(v)`; `v`.
pub fn native_017b<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0086_3860, &[v]);
    v
}

/// `Native_016c_MusicStopCurrent` (`0x008594c0`, `0x016c`): the current music (when not
/// negative) `0x00886a60`, current music -1; 1.
pub fn native_016c<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    let i = call(h, 0x0087_3b00, &[]);
    if i.cast_signed() >= 0 {
        call(h, 0x0088_6a60, &[i]);
    }
    call(h, 0x0087_3d00, &[u32::MAX]);
    1
}

/// `Native_00ec_PartyGetSlot` (`0x00856bc0`, `0x00ec`): `(chr)`: its party slot (0..=2,
/// `0x007852f0`), -1 when absent.
pub fn native_00ec<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    if v == u32::MAX {
        return v;
    }
    let o = h.scratch();
    for i in 0..3 {
        h.write(o + i * 4, 0_u32);
    }
    call(h, 0x0078_52f0, &[o, o + 4, o + 8]);
    for i in 0..3 {
        let c = h.read::<u32>(o + i * 4);
        if v == c && c != 0xff {
            return i;
        }
    }
    u32::MAX
}

/// `Native_0089_GetLastEventJumpId` (`0x0085d280`, `0x0089`): jumps to `0x0086bc70`.
pub fn native_0089<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0086_bc70)
}

/// `Native_01e3_DisableHamonEffect` (`0x00858ab0`, `0x01e3`): jumps to `0x00908450`.
pub fn native_01e3<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0090_8450)
}

/// `Native_01bd_SetWaterBattleEffect` (`0x00859ff0`, `0x01bd`): `0x008715f0(v)`; 1.
pub fn native_01bd<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    super::helpers::atel_set_water_battle_effect(h, v);
    1
}

/// `Native_0050_PadIsHeldRaw` (`0x0085ec60`, `0x0050`): `(i)`: bit `i` of
/// `0x0086af30()`'s low word.
pub fn native_0050<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    let x = call(h, 0x0086_af30, &[]) & 0xffff;
    u32::from(x & (1 << (i & 31)) != 0)
}

/// `Native_0221` (`0x0085e110`, `0x0221`): `(i)`: word `i` of the table at
/// `0x00c86e28`.
pub fn native_0221<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    let t = h.rebase(0x00c8_6e28);
    h.read(t.wrapping_add(i.wrapping_mul(4)))
}

/// `(v)` to a speed: `K2 / (v * K1)` (`[0x00b92220]`, `[0x00b92350]`) or,
/// for 0, `zero`; into `0x00877450`, then `0x00861000(v, b)`; `v`.
fn speed_861000<H: NativeHost>(h: &mut H, s: u32, one: bool, b: u32) -> u32 {
    let v = pop(h, s);
    let f = if v == 0 {
        if one {
            x87::store(1.0)
        } else {
            cp(h.read(h.rebase(0x00b9_2340)))
        }
    } else {
        let r = r32(f64::from(v.cast_signed()));
        let k1 = h.read::<f64>(h.rebase(0x00b9_2220));
        let x = x87::mul(r, k1);
        let q = if one {
            x87::div(1.0, x)
        } else {
            x87::div(h.read::<f64>(h.rebase(0x00b9_2350)), x)
        };
        x87::store(q)
    };
    call(h, 0x0087_7450, &[cp(f)]);
    call(h, 0x0086_1000, &[v, b]);
    v
}

/// `Native_0143_BgSepFadeOut` (`0x00857ad0`, `0x0143`): `(frames)`: `0x00877450` with
/// `[0x00b92350] / (frames * [0x00b92220])` (`[0x00b92340]` for 0), then
/// `0x00861000(frames, 0)`; `frames`.
pub fn native_0143<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    speed_861000(h, s, false, 0)
}

/// `Native_0144_BgSepFadeIn` (`0x00857d40`, `0x0144`): `(frames)`: `0x00877450` with
/// `1 / (frames * [0x00b92220])` (1 for 0), then `0x00861000(frames,
/// 0x7f)`; `frames`.
pub fn native_0144<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    speed_861000(h, s, true, 0x7f)
}

/// `Native_025b_UnlockAchievement` (`0x00859270`, `0x025b`): `(v)`: `0x00822380` with 0x17
/// as 0x16 and 0x19 as 0x18 (0x16 itself does nothing); the value used.
pub fn native_025b<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let v = match v {
        0x16 => return v,
        0x17 => 0x16,
        0x19 => 0x18,
        _ => v,
    };
    call(h, 0x0082_2380, &[v]);
    v
}

/// `(c, b, a)` for an attached worker (`+0xaa` 5): `0x00908330(c, b, a)`;
/// 1, else 0.
fn attached_908330<H: NativeHost>(h: &mut H, w: u32, a: u32, b: u32, c: u32) -> u32 {
    if h.read::<u8>(w + 0xaa) != 5 {
        return 0;
    }
    call(h, 0x0090_8330, &[c, b, a]);
    1
}

/// `Native_0198_AttachAnimDobj` (`0x0085ffa0`, `0x0198`): `(c, b, a)`: for `+0xaa` 5,
/// `0x00908330(c, b, a)`; 1, else 0.
pub fn native_0198<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    let c = pop(h, s);
    attached_908330(h, w, a, b, c)
}

/// `Native_0199_AttachAnimOwnDobj` (`0x00860200`, `0x0199`): `(b, a)`: as `0x0198` with
/// `+0xa8` first (pushed with `b` and `a` and popped back).
pub fn native_0199<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    if h.read::<u8>(w + 0xaa) != 5 {
        return 0;
    }
    let m = u32::from(h.read::<u16>(w + 0xa8));
    for v in [m, b, a] {
        atel_push_stack_integer(h, s, v);
    }
    let a = pop(h, s);
    let b = pop(h, s);
    let c = pop(h, s);
    attached_908330(h, w, a, b, c)
}

/// `Native_0141_MsgSetTextVar` (`0x00857a00`, `0x0141`): `(window, v, var)`: the
/// window's `+8`, `+0xc` = the variable's address, `+0x16` = v; 0.
pub fn native_0141<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let var = pop(h, s);
    let v = pop(h, s);
    let i = pop(h, s);
    let x = atel_get_real_var_adrs(h, w, var);
    let m = super::atel_get_mes_win_work(h, i.cast_signed());
    h.write(m + 8, x);
    h.write(m + 0xc, x);
    #[allow(clippy::cast_possible_truncation)] // the original stores CX
    h.write(m + 0x16, v as u16);
    0
}

/// `Native_00cf_WorkerSetFieldAf0` (`0x0085c400`, `0x00cf`): `(v)` into `+0xaf0`; `v`.
pub fn native_00cf<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    h.write(w + 0xaf0, v);
    v
}

/// `Native_00d0` (`0x0085c4b0`, `0x00d0`): `+0xaf0`.
pub fn native_00d0<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    h.read(w + 0xaf0)
}

/// The worker id pushed and popped back; that worker's `+0x38` |= `bit`.
fn own_38<H: NativeHost>(h: &mut H, w: u32, s: u32, bit: u16) {
    let id = u32::from(h.read::<u16>(w + worker::ID));
    atel_push_stack_integer(h, s, id);
    let id = pop(h, s);
    let o = work(h, id);
    let f = h.read::<u16>(o + 0x38);
    h.write(o + 0x38, f | bit);
}

/// `Native_020b` (`0x0085be30`, `0x020b`): `+0x38` bits 8 and 2; 1.
pub fn native_020b<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let f = h.read::<u16>(w + 0x38);
    h.write(w + 0x38, f | 0x100);
    own_38(h, w, s, 4);
    1
}

/// `Native_01ca` (`0x0085e340`, `0x01ca`): `+0x38` bit 2; 1.
pub fn native_01ca<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    own_38(h, w, s, 4);
    1
}

/// `strcat(d, src)` in the game's memory.
fn strcat<H: NativeHost>(h: &mut H, d: u32, src: u32) {
    let mut e = d;
    while h.read::<u8>(e) != 0 {
        e = e.wrapping_add(1);
    }
    let mut i = 0_u32;
    loop {
        let c = h.read::<u8>(src.wrapping_add(i));
        h.write(e.wrapping_add(i), c);
        if c == 0 {
            break;
        }
        i += 1;
    }
}

/// `Native_0237_BuildChoiceText` (`0x008604c0`, `0x0237`): `(dst, list, mask, n)`: builds
/// a choice text into the string variable `dst` from the `n` string
/// pointers of the variable `list`: a header (`0x00b5b9f8`), each entry
/// (separated by `0x00b5b9f0`) with the enabled/disabled colour
/// (`0x00b5b9e8`/`0x00b5b9ec`) where the mask bit changes and a choice
/// code (`0x10`, `'0' + k`) before the enabled ones, a trailer
/// (`0x00b5b9f4`), then the text copied again after its terminator; the
/// choice count `k << 8`.
pub fn native_0237<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let pd = pop(h, s);
    let pl = pop(h, s);
    let mask = pop(h, s);
    let n = pop(h, s);
    let dst = atel_get_real_var_adrs(h, w, pd);
    let mut list = atel_get_real_var_adrs(h, w, pl);
    h.write(dst, 0_u8);
    let mut prev = u32::MAX;
    let mut k = 0_u32;
    let head = h.rebase(0x00b5_b9f8);
    strcat(h, dst, head);
    let mut i = 0_u32;
    while i.cast_signed() < n.cast_signed() {
        let p = h.read::<u32>(list);
        list = list.wrapping_add(4);
        let bit = u32::from(mask & (1 << (i & 31)) != 0);
        if bit != prev {
            let c = h.rebase(if bit != 0 { 0x00b5_b9e8 } else { 0x00b5_b9ec });
            strcat(h, dst, c);
        }
        prev = bit;
        if bit != 0 {
            let o = h.scratch();
            #[allow(clippy::cast_possible_truncation)] // the original adds to AL
            h.write(
                o,
                u32::from_le_bytes([0x10, (k as u8).wrapping_add(0x30), 0, 0]),
            );
            strcat(h, dst, o);
            k += 1;
        }
        strcat(h, dst, p);
        if i.cast_signed() < n.wrapping_sub(1).cast_signed() {
            let sep = h.rebase(0x00b5_b9f0);
            strcat(h, dst, sep);
        }
        i += 1;
    }
    let tail = h.rebase(0x00b5_b9f4);
    strcat(h, dst, tail);
    let mut len = 0_u32;
    while h.read::<u8>(dst.wrapping_add(len)) != 0 {
        len += 1;
    }
    let mut j = 0_u32;
    loop {
        let c = h.read::<u8>(dst.wrapping_add(j));
        h.write(dst.wrapping_add(j + len + 1), c);
        if c == 0 {
            break;
        }
        j += 1;
    }
    k << 8
}

/// `Native_0112_PartyPopMembers` (`0x0085b520`, `0x0112`): jumps to `0x0086ddb0`.
pub fn native_0112<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0086_ddb0)
}

/// `Native_0111_PartyPushMembers` (`0x0085b4b0`, `0x0111`): jumps to `0x0086e310`.
pub fn native_0111<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0086_e310)
}

/// `Native_01fc` (`0x0085b170`, `0x01fc`): `(b, a)` ->
/// `0x00874200(b, a, 1)`.
pub fn native_01fc<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    call(h, 0x0087_4200, &[b, a, 1])
}

/// `Native_0051_PadIsPressedRaw` (`0x0085f0e0`, `0x0051`): `(i)`: bit `i` of
/// `0x0086af70()`'s low word.
pub fn native_0051<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    let x = call(h, 0x0086_af70, &[]) & 0xffff;
    u32::from(x & (1 << (i & 31)) != 0)
}

/// `Native_01d9_SetChr6Name` (`0x008582e0`, `0x01d9`): the text `0x0078fcf0(3,
/// 0x202c, 0x007851f0())` copied to `0x00784fb0(6)`; 1.
pub fn native_01d9<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    let x = call(h, 0x0078_51f0, &[]);
    let src = call(h, 0x0078_fcf0, &[3, 0x202c, x]);
    let d = call(h, 0x0078_4fb0, &[6]);
    let mut i = 0_u32;
    loop {
        let c = h.read::<u8>(src.wrapping_add(i));
        h.write(d.wrapping_add(i), c);
        if c == 0 {
            break;
        }
        i += 1;
    }
    1
}

/// `Native_00e8_PartyGetMembers` (`0x00856270`, `0x00e8`): `(a, b, c)` variables: the
/// three party slots (`0x007852f0`) into `c`, `b`, `a`, empty (`0xff`) as
/// -1; 1.
pub fn native_00e8<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let p1 = pop(h, s);
    let p2 = pop(h, s);
    let p3 = pop(h, s);
    let a = atel_get_real_var_adrs(h, w, p1);
    let b = atel_get_real_var_adrs(h, w, p2);
    let c = atel_get_real_var_adrs(h, w, p3);
    call(h, 0x0078_52f0, &[c, b, a]);
    for x in [c, b, a] {
        if h.read::<u32>(x) == 0xff {
            h.write(x, u32::MAX);
        }
    }
    1
}

/// `Native_0030_TurnSet880` (`0x0085a7c0`, `0x0030`, float): `(v)` ->
/// `0x00871540(worker, level, v)`'s `ST0`.
pub fn native_0030<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let v = popf(h, s);
    let l = lvl(h, w);
    callf(h, 0x0087_1540, &[w, l, cp(v)])
}

/// A model slot `i` (`+0xaf4`) flag: `f(model, on)` when the slot is used
/// (`+0xb14` bit `i`), and bit `bit` of `+0xb08 + i * 4`; `on != 0`.
fn slot_flag<H: NativeHost>(h: &mut H, w: u32, s: u32, f: u32, bit: u32) -> u32 {
    let on = u32::from(pop(h, s) != 0);
    let i = pop(h, s);
    let m = h.read::<u32>(w.wrapping_add(i.wrapping_mul(4)).wrapping_add(0xaf4));
    if m != 0 {
        let used = u32::from(h.read::<u16>(w + 0xb14));
        if used & (1 << (i & 31)) != 0 {
            call(h, f, &[m, on]);
        }
    }
    let o = w.wrapping_add(i.wrapping_mul(4)).wrapping_add(0xb08);
    let x = h.read::<u32>(o);
    let b = if bit == 0 { on } else { on << 1 };
    h.write(o, x ^ ((b ^ x) & (1 << bit)));
    on
}

/// `Native_01d4` (`0x00855dd0`, `0x01d4`): `(slot, on)`: `0x0082aa30` on
/// the slot's model, bit 0.
pub fn native_01d4<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    slot_flag(h, w, s, 0x0082_aa30, 0)
}

/// `Native_022f` (`0x00855fb0`, `0x022f`): `(slot, on)`: `0x0082b970` on
/// the slot's model, bit 1.
pub fn native_022f<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    slot_flag(h, w, s, 0x0082_b970, 1)
}

/// `Native_0174_PosGetField30` (`0x0085e3a0`, `0x0174`, float): the position's `+0x30`.
pub fn native_0174<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> f64 {
    let p = atel_get_pos_work_adrs(h, w);
    x87::load(h.read(p + 0x30))
}

/// `Native_0161_ImportantReset` (`0x0085b760`, `0x0161`): `0x0088e780(v)`; `v`.
pub fn native_0161<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0088_e780, &[v]);
    v
}

/// `(b, a)`: `f(b, a)`; `a`.
fn call2a<H: NativeHost>(h: &mut H, s: u32, f: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    call(h, f, &[b, a]);
    a
}

/// `Native_016a_MusicVolumeDirect` (`0x0085bbb0`, `0x016a`): `0x00886ab0(b, a)`; `a`.
pub fn native_016a<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    call2a(h, s, 0x0088_6ab0)
}

/// `Native_00fe_SndSepVolumeDirect` (`0x00860090`, `0x00fe`): `0x00886f60(b, a)`; `a`.
pub fn native_00fe<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    call2a(h, s, 0x0088_6f60)
}

/// `Native_0044_PadHeld` (`0x0085d4e0`, `0x0044`): the flag word
/// `0x00888c40(0, 0, 0x0086af30())`.
pub fn native_0044<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    let x = call(h, 0x0086_af30, &[]) & 0xffff;
    call(h, 0x0088_8c40, &[0, 0, x]) & 0xffff
}

/// `Native_024d` (`0x00856dc0`, `0x024d`): `(on)`: the current ctrl
/// work's `+0x210` bit 6; `on != 0`.
pub fn native_024d<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = u32::from(pop(h, s) != 0);
    let c = super::atel_get_cur_ctrl_work(h);
    let x = h.read::<u32>(c + 0x210);
    h.write(c + 0x210, x ^ (((v << 6) ^ x) & 0x40));
    v
}

/// `Native_0209_BindEffWaterOn` (`0x0085f430`, `0x0209`): a worker with a character gets
/// `0x00862700(worker, 2)`; 1.
pub fn native_0209<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    if h.read::<u8>(w + 0xaa) == 1 && h.read::<u32>(w + 0x9c) != 0 {
        super::helpers::atel_bind_eff2_on(h, w, 2);
    }
    1
}

/// `Native_010d_ImportantSetIndexed` (`0x0085ac90`, `0x010d`): `(i)` up to `0xfe`:
/// `0x0088e790(0xa004 + i)`; 1, else 0.
pub fn native_010d<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    if v > 0xfe {
        return 0;
    }
    call(h, 0x0088_e790, &[v + 0xa004]);
    1
}

/// `Native_0194` (`0x0085f8b0`, `0x0194`): `0x0086f060(v)`; `v`.
pub fn native_0194<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    super::helpers::atel_set_save_event_d1(h, v);
    v
}

/// `Native_010c_MapChangeDefault` (`0x00858610`, `0x010c`): `(map, entrance)`:
/// `AtelSetEventJump2(map, entrance, 0)`; 0.
pub fn native_010c<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    call(h, 0x0086_ff40, &[b, a, 0]);
    0
}

/// `Native_00ab_MapChange` (`0x00858330`, `0x00ab`): `(map, entrance)`:
/// `AtelSetEventJump2(map, entrance, 1)`; 0.
pub fn native_00ab<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    call(h, 0x0086_ff40, &[b, a, 1]);
    0
}

/// `Native_011a_LoadEventTexList` (`0x0085c0a0`, `0x011a`): `(c, b, a)`:
/// `0x00874b70(&c, b, &a)` on the variables' addresses; 1.
pub fn native_011a<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    let c = pop(h, s);
    let x = atel_get_real_var_adrs(h, w, a);
    let y = atel_get_real_var_adrs(h, w, c);
    call(h, 0x0087_4b70, &[y, b, x]);
    1
}

/// `Native_021e` (`0x00858e40`, `0x021e`): `0x0090a420()`; 1.
pub fn native_021e<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call(h, 0x0090_a420, &[]);
    1
}

/// `Native_014e_GetAngleFromCenter` (`0x00858ea0`, `0x014e`, float): `(x, y, r)`: inside the
/// radius `r` around (128, 128) 0, else the angle `0x00889e70(x - 128,
/// y - 128)` (0 becoming `[0x00b368a0]`).
pub fn native_014e<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    let a = pop(h, s);
    let b = pop(h, s);
    let r = pop(h, s);
    let x = a.wrapping_sub(0x80);
    let y = b.wrapping_sub(0x80);
    let r2 = r.wrapping_mul(r).cast_signed();
    let d2 = y
        .wrapping_mul(y)
        .wrapping_add(x.wrapping_mul(x))
        .cast_signed();
    if r2 > d2 {
        return 0.0;
    }
    let fy = x87::store(f64::from(y.cast_signed()));
    let fx = x87::store(f64::from(x.cast_signed()));
    let f = r32(callf(h, 0x0088_9e70, &[cp(fx), cp(fy)]));
    if f == 0.0 {
        x87::load(cp(h.read(h.rebase(0x00b3_68a0))))
    } else {
        f
    }
}

/// `Native_024b_ChrSetHideEvent` (`0x00857e30`, `0x024b`): `(slot, on)`: `0x0082b2c0(model,
/// on)` for a used slot with a model and motion (`+0xb04` not `0xff`);
/// `on != 0`.
pub fn native_024b<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let on = u32::from(pop(h, s) != 0);
    let i = pop(h, s);
    let used = u32::from(h.read::<u16>(w + 0xb14));
    if used & (1 << (i & 31)) != 0
        && h.read::<u16>(w.wrapping_add(i.wrapping_mul(2)).wrapping_add(0xb04)) != 0xff
    {
        let m = h.read::<u32>(w.wrapping_add(i.wrapping_mul(4)).wrapping_add(0xaf4));
        if m != 0 {
            call(h, 0x0082_b2c0, &[m, on]);
        }
    }
    on
}

/// `Native_01fa` (`0x0085af60`, `0x01fa`): jumps to `0x008bef00`.
pub fn native_01fa<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x008b_ef00)
}

/// `Native_01fb` (`0x0085af10`, `0x01fb`): jumps to `0x008beef0`.
pub fn native_01fb<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x008b_eef0)
}

/// `Native_0216_Poll` (`0x0085b5d0`): as `0x015b`'s with the state at
/// `+0xe`, done with 5.
pub fn native_0216_poll<H: NativeHost>(h: &mut H, w: u32, a: u32, _s: u32) -> u32 {
    let r = call(h, 0x0085_ae90, &[w, a]);
    match h.read::<u16>(a + 0xe) {
        0 => {
            if r & 1 != 0 {
                h.write(a + 0xe, 1_u16);
            }
            0
        }
        1 => {
            h.write(a + 0xe, 2_u16);
            0
        }
        2 => 5,
        _ => 0,
    }
}

/// `Native_0216_Result` (`0x0085b920`): the current ctrl work's `+3` bit 2
/// cleared, `0x0086a130()`, then `0x00874200(args +8, args +0xc, 1)`.
pub fn native_0216_result<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    let x = h.read::<u32>(a + 8);
    let y = u32::from(h.read::<u16>(a + 0xc));
    let c = super::atel_get_cur_ctrl_work(h);
    let b = h.read::<u8>(c + 3);
    h.write(c + 3, b & 0xfb);
    super::helpers::atel_uc_resume(h);
    call(h, 0x0087_4200, &[x, y, 1])
}

/// `Native_01f8_MusicFadeOutTrack` (`0x0085e570`, `0x01f8`): `(track, frames)`: 0 or less
/// stops the current music (`0x00886a60`, `0x00886800`); else a track not
/// negative fades (`0x00886af0(track, frames)`); `frames`.
pub fn native_01f8<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let b = pop(h, s);
    let t = pop(h, s);
    if b.cast_signed() <= 0 {
        let i = call(h, 0x0087_3b00, &[]);
        call(h, 0x0088_6a60, &[i]);
        call(h, 0x0088_6800, &[i]);
        return b;
    }
    if t.cast_signed() >= 0 {
        call(h, 0x0088_6af0, &[t, b]);
    }
    b
}

/// `Native_0114_CtrlSetByte2Bits` (`0x0085b670`, `0x0114`): `(b, a)`: the current ctrl
/// work's `+2` bit 1 = a, bit 2 = b; `b != 0`.
pub fn native_0114<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let c = super::atel_get_cur_ctrl_work(h);
    let a = u8::from(pop(h, s) != 0);
    let b = u8::from(pop(h, s) != 0);
    let x = h.read::<u8>(c + 2);
    h.write(c + 2, (((b << 1) | a) << 1) | (x & 0xf9));
    u32::from(b)
}

/// `Native_0220_DisableLoveParam` (`0x0085e010`, `0x0220`): `0x00870250(v, 1)`; `v`.
pub fn native_0220<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0087_0250, &[v, 1]);
    v
}

/// `Native_0203_ChrLoadModel` (`0x0085d460`, start): `(chr)` into the args, state 0;
/// `chr`.
pub fn native_0203<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    h.write(a + 4, 0_u32);
    h.write(a, v);
    v
}

/// `Native_0203_ChrLoadModel_Poll` (`0x0085d550`): steps the args' state: 0 waits for
/// `0x0086ade0()` to clear; 1 for the loader's `+8(1)`, `0x00887b30()` and
/// `0x0076f060()` (then `0x0086f460(worker id)`); 2 `0x00836a40(chr, 0)`;
/// 3 waits for `0x00836a50(chr, 0)` (then `0x00829ef0`); 4 for
/// `0x0082a040(chr)`, then done (5).
pub fn native_0203_poll<H: NativeHost>(h: &mut H, w: u32, a: u32, _s: u32) -> u32 {
    let r = call(h, 0x0076_d0d0, &[]);
    let mut st = h.read::<u32>(a + 4);
    let c = h.read::<u32>(a);
    let mut out = 0;
    'done: {
        if st > 4 {
            break 'done;
        }
        if st == 0 {
            if call(h, 0x0086_ade0, &[]) != 0 {
                break 'done;
            }
            st = 1;
        }
        if st == 1 {
            let f = h.read::<u32>(r + 8);
            if h.call(f, &[1]) != 0
                || call(h, 0x0088_7b30, &[]) != 0
                || call(h, 0x0076_f060, &[]) != 0
            {
                break 'done;
            }
            let id = u32::from(h.read::<u16>(w + worker::ID));
            call(h, 0x0086_f460, &[id]);
            st = 2;
        }
        if st == 2 {
            call(h, 0x0083_6a40, &[c, 0]);
            st = 3;
        }
        if st == 3 {
            if call(h, 0x0083_6a50, &[c, 0]) == 1 {
                break 'done;
            }
            call(h, 0x0082_9ef0, &[c]);
            st = 4;
        }
        if call(h, 0x0082_a040, &[c]) == 1 {
            break 'done;
        }
        out = 5;
    }
    h.write(a + 4, st);
    out
}

/// `Native_0203_ChrLoadModel_Result` (`0x0085dc10`): the chr pushed back,
/// `0x0085d000(worker, args, stack)`, `Atel_ClearBindMask(worker id)`
/// (`0x00863650`); the
/// former's result.
pub fn native_0203_result<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    let v = h.read::<u32>(a);
    atel_push_stack_integer(h, s, v);
    let r = call(h, 0x0085_d000, &[w, a, s]);
    let id = u32::from(h.read::<u16>(w + worker::ID));
    super::helpers::atel_clear_bind_mask(h, id);
    r
}

/// `Native_01a8` (`0x00858390`, `0x01a8`): `(chr)`: `+0x54` of
/// `0x00785330(chr)`.
pub fn native_01a8<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let r = call(h, 0x0078_5330, &[v]);
    h.read(r + 0x54)
}

/// `Native_0192` (`0x0085f700`, `0x0192`, float): `(v)` into the current
/// ctrl work's `+0x1f0`; `ST0` it.
pub fn native_0192<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    let c = super::atel_get_cur_ctrl_work(h);
    let v = cp(popf(h, s));
    h.write(c + 0x1f0, v);
    x87::load(v)
}

/// `Native_01c4` (`0x0085a840`, `0x01c4`): `0x00873dd0(on != 0)`; that.
pub fn native_01c4<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = u32::from(pop(h, s) != 0);
    super::helpers::atel_set_save_event_word(h, 0xe8, v);
    v
}

/// `Native_012e_SaveCounterAdd` (`0x0085e040`, `0x012e`): `(i, v)`: word `i` (clamped to
/// 0..=7) of the record `0x0088c830()`'s `+0x2c` += v; the sum.
pub fn native_012e<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let i = pop(h, s);
    let g = call(h, 0x0088_c830, &[]);
    let i = i.cast_signed().clamp(0, 7).cast_unsigned();
    let o = g + 0x2c + i * 4;
    let x = h.read::<u32>(o).wrapping_add(v);
    h.write(o, x);
    x
}

/// `Native_012b_CtrlSetByte2Bit6` (`0x0085dc50`, `0x012b`): `(on)`: the current ctrl
/// work's `+2` bit 6; `on != 0`.
pub fn native_012b<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let c = super::atel_get_cur_ctrl_work(h);
    let v = u8::from(pop(h, s) != 0);
    let x = h.read::<u8>(c + 2);
    h.write(c + 2, x ^ (((v << 6) ^ x) & 0x40));
    u32::from(v)
}

/// `Native_0092_PosGetField20Of` (`0x0085eac0`, `0x0092`, float): `(worker)`: its
/// position's `+0x20`.
pub fn native_0092<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    let i = pop(h, s);
    let o = work(h, i);
    let p = atel_get_pos_work_adrs(h, o);
    x87::load(h.read(p + 0x20))
}

/// `Native_023a_SetDolby` (`0x00856250`, `0x023a`): `0x0086f9e0(v)`; 1.
pub fn native_023a<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    super::helpers::atel_set_dolby(h, v);
    1
}

/// `Native_01e5` (`0x00859000`, `0x01e5`): `(c, b, a)` ->
/// `0x0090a400(c, b, a)`.
pub fn native_01e5<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    let c = pop(h, s);
    call(h, 0x0090_a400, &[c, b, a])
}

/// `Native_0195_Poll` (`0x0085fb00`): done (5) once `0x0086b060()` is 0.
pub fn native_0195_poll<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    if call(h, 0x0086_b060, &[]) == 0 { 5 } else { 0 }
}

/// `Native_023f` (`0x00856770`, `0x023f`): `0x008749e0(v)`; `v`.
pub fn native_023f<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0087_49e0, &[v]);
    v
}

/// `Native_01cf_BindEffDrawOff` (`0x0085ea40`, `0x01cf`): a worker with a character gets
/// `0x007fba40(chr)` and `+0x38` bit 4 cleared; 1.
pub fn native_01cf<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    if h.read::<u8>(w + 0xaa) == 1 {
        let c = h.read::<u32>(w + 0x9c);
        if c != 0 {
            call(h, 0x007f_ba40, &[c]);
            let f = h.read::<u16>(w + 0x38);
            h.write(w + 0x38, f & 0xffef);
        }
    }
    1
}

/// `Native_01b0_MovieSetTexture` (`0x008590a0`, `0x01b0`): `(b, a)`: `0x00873eb0(b, a)`
/// (the original first prints both through the C runtime's `printf`, a
/// debug line left out here); `b`.
pub fn native_01b0<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    call(h, 0x0087_3eb0, &[b, a]);
    b
}

/// `Native_020f_SetForceBattleMap` (`0x0085c100`, `0x020f`): `(v)`: `0x00870130(v)`, then
/// the ctrl work `0x0086c1f0()` made current (`AtelChangeCurrentCtrl`)
/// and handed to `0x00780c70`; the previous one.
pub fn native_020f<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0087_0130, &[v]);
    let x = call(h, 0x0086_c1f0, &[]);
    let y = call(h, 0x0086_2d50, &[x]);
    call(h, 0x0078_0c70, &[y]);
    y
}

/// `Native_01dc_VibrateStop3` (`0x008571d0`, `0x01dc`): 3 pushed and popped back,
/// `0x00876290(3)`; 3.
pub fn native_01dc<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    atel_push_stack_integer(h, s, 3);
    let v = pop(h, s);
    call(h, 0x0087_6290, &[v]);
    v
}

/// `Native_0202_FootSoundOffChr` (`0x0085a770`, `0x0202`): `(v)`: `0x00887100(v, 0)` and
/// `0x00887130(v, 0)` (each pushed and popped back); 1.
pub fn native_0202<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    for f in [0x0088_7100, 0x0088_7130] {
        atel_push_stack_integer(h, s, v);
        let x = pop(h, s);
        call(h, f, &[x, 0]);
    }
    1
}

/// `Native_0098_VarSetCurLevel` (`0x0085fb50`, `0x0098`): `(var, n)`: `0x00870fc0(worker,
/// the variable's address)`, `0x00871610(worker, n as float)`,
/// `0x00871630(worker, 0)`; 1.
pub fn native_0098<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let n = pop(h, s);
    let f = x87::store(f64::from(n.cast_signed()));
    let v = pop(h, s);
    let x = atel_get_real_var_adrs(h, w, v);
    call(h, 0x0087_0fc0, &[w, x]);
    call(h, 0x0087_1610, &[w, cp(f)]);
    call(h, 0x0087_1630, &[w, x87::store(0.0)]);
    1
}

/// `Native_0217` (`0x0085d3a0`, `0x0217`): `0x00798b60()`; 1.
pub fn native_0217<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call(h, 0x0079_8b60, &[]);
    1
}

/// `Native_01bf` (`0x0085a190`, `0x01bf`): `0x008708d0()`; 1.
pub fn native_01bf<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call(h, 0x0087_08d0, &[]);
    1
}

/// `Native_0193` (`0x0085f4e0`, `0x0193`): `(track)`: unloaded tracks
/// (`0x00886680`) are requested (`0x00886e80`); a loaded one not marked
/// (bit 6) in the worker's track list (`0x0086ac60`, `0x0086ac30`,
/// `0x0086ac90`) gets `0x00886ff0(track, 0xc, 0x7f or 0x3f)`; `track`.
pub fn native_0193<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    if call(h, 0x0088_6680, &[v]) == 0 {
        call(h, 0x0088_6e80, &[v]);
        return v;
    }
    let g = call(h, 0x0088_c830, &[]);
    let k = h.read::<u8>(g + 0x28);
    let l = h.read::<u32>(w + 4);
    let n = call(h, 0x0086_ac60, &[l]);
    let mut f = 0;
    let mut i = 0_u32;
    while i.cast_signed() < n.cast_signed() {
        let l = h.read::<u32>(w + 4);
        let x = call(h, 0x0086_ac30, &[l, i]);
        let l = h.read::<u32>(w + 4);
        let y = call(h, 0x0086_ac90, &[l, i]);
        if x == v {
            f = y;
            break;
        }
        i += 1;
    }
    let vol = if k == 0 { 0x7f } else { 0x3f };
    if f & 0x40 == 0 {
        call(h, 0x0088_6ff0, &[v, 0xc, vol]);
    }
    v
}

/// `Native_0189_SetMapIdPage` (`0x0085e890`, start): `0x00873e60(v)`; `v` (that
/// function only sets `AL`, so `EAX` keeps the popped value's upper bytes).
pub fn native_0189<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0087_3e60, &[v]);
    v
}

/// `Native_0189_SetMapIdPage_Poll` (`0x0085ea30`): 1.
pub fn native_0189_poll<H: NativeHost>(_h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    1
}

/// `Native_0135_GetGil` (`0x0085ed00`, `0x0135`): jumps to `0x00784e80`.
pub fn native_0135<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0078_4e80)
}

/// `Native_00a2_MsgWinSetColorBytes` (`0x00856570`, `0x00a2`): `(window, a, b, c, d)` bytes:
/// the window's `+0x10` = `a << 24 | d << 16 | c << 8 | b` (pushed and
/// popped back); it.
pub fn native_00a2<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    let c = pop(h, s);
    let d = pop(h, s);
    let x = ((((a << 8) | (d & 0xff)) << 8 | (c & 0xff)) << 8) | (b & 0xff);
    atel_push_stack_integer(h, s, x);
    let v = pop(h, s);
    let i = pop(h, s);
    let m = super::atel_get_mes_win_work(h, i.cast_signed());
    h.write(m + 0x10, v);
    v
}

/// `Native_00ee_WorkerGetFlag34Bit5` (`0x008572b0`, `0x00ee`): `(worker)`: its `+0x34` bit 5.
pub fn native_00ee<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    let o = work(h, i);
    (u32::from(h.read::<u16>(o + 0x34)) >> 5) & 1
}

/// `Native_020d` (`0x0085bf70`, `0x020d`): `0x00870fa0(v)`; `v`.
pub fn native_020d<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    super::helpers::atel_set_save_event_word(h, 0xea, v);
    v
}

/// `Native_0149_CtrlSetByte2Bit7` (`0x00858580`, `0x0149`): `(on)`: the current ctrl
/// work's `+2` bit 7; `on != 0`.
pub fn native_0149<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = u8::from(pop(h, s) != 0);
    let c = super::atel_get_cur_ctrl_work(h);
    let x = h.read::<u8>(c + 2);
    h.write(c + 2, (x & 0x7f) | (v << 7));
    u32::from(v)
}

/// `Native_0146_MinimapSetSize` (`0x008581a0`, `0x0146`): `0x00919e80(v)`; `v`.
pub fn native_0146<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0091_9e80, &[v]);
    v
}

/// `Native_0145_MinimapSetPos` (`0x00857fc0`, `0x0145`): `(b, a)`: `0x00919e50(b, a)`; 1.
pub fn native_0145<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    call(h, 0x0091_9e50, &[b, a]);
    1
}

/// `Native_0226_FpSetDisp` (`0x0085eb80`, `0x0226`): `0x00729050(on != 0)`; that.
pub fn native_0226<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = u32::from(pop(h, s) != 0);
    call(h, 0x0072_9050, &[v]);
    v
}

/// `(f)()`; 1.
fn call0_1<H: NativeHost>(h: &mut H, f: u32) -> u32 {
    call(h, f, &[]);
    1
}

/// `Native_01b1` (`0x00859240`, `0x01b1`): `0x00873db0()`; 1.
pub fn native_01b1<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call0_1(h, 0x0087_3db0)
}

/// `Native_0167_MusicSetReplayPoint` (`0x00859c40`, `0x0167`): `0x008869f0()`; 1.
pub fn native_0167<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call0_1(h, 0x0088_69f0)
}

/// `Native_0234_FlushAlbhedDic` (`0x0085fff0`, `0x0234`): `0x0086a040()`; 1.
pub fn native_0234<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call0_1(h, 0x0086_a040)
}

/// `0x00863fe0(that worker, i)` for the nine levels of worker `i`.
fn levels_863fe0<H: NativeHost>(h: &mut H, i: u32) -> u32 {
    let o = work(h, i);
    for k in 0..9 {
        call(h, 0x0086_3fe0, &[o, k]);
    }
    0
}

/// `Native_00d2_StopTurnAll` (`0x0085c6f0`, `0x00d2`): `0x00863fe0(worker, i)` for the
/// nine levels (the worker id pushed and popped back first); 0.
pub fn native_00d2<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let id = u32::from(h.read::<u16>(w + worker::ID));
    atel_push_stack_integer(h, s, id);
    let id = pop(h, s);
    levels_863fe0(h, id)
}

/// `Native_0037_WorkerStopTurnAll` (`0x0085b780`, `0x0037`): `(worker)`: as `0x00d2` for it.
pub fn native_0037<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    levels_863fe0(h, i)
}

/// `Native_0159_PartySlotSetOf` (`0x0085d290`, `0x0159`): `(worker, slot)`: the party
/// slot for that worker; `slot`.
pub fn native_0159<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let c = super::atel_get_cur_ctrl_work(h);
    let slot = pop(h, s);
    let i = pop(h, s);
    let o = work(h, i);
    party_slot(h, o, c, slot);
    slot
}

/// `Native_0204` (`0x0085dc90`, `0x0204`, float): `(v)` into the
/// position's `+0x2c`; `ST0` it.
pub fn native_0204<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let p = atel_get_pos_work_adrs(h, w);
    let v = cp(popf(h, s));
    h.write(p + 0x2c, v);
    x87::load(v)
}

/// `Native_01c3` (`0x0085a750`, `0x01c3`): `0x00874030(v)`; `v`.
pub fn native_01c3<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    super::helpers::atel_set_save_event_e4(h, v);
    v
}

/// `Native_0260_AlwaysTrue` (`0x00859f10`, `0x0260`): one value dropped; 1.
pub fn native_0260<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    pop(h, s);
    1
}

/// `Native_0261_AlwaysTrue` (`0x0085a090`, `0x0261`): one value dropped; 1.
pub fn native_0261<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    pop(h, s);
    1
}

/// `Native_0251` (`0x0085e5e0`, `0x0251`): `(i)`: bit `i` of the flag word
/// for `0x0086af60()`.
pub fn native_0251<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    let x = call(h, 0x0086_af60, &[]);
    flag_bit(h, x, i)
}

/// A worker with a character gets `f(worker, k)`; 1.
fn chr_call<H: NativeHost>(h: &mut H, w: u32, f: u32, k: u32) -> u32 {
    if h.read::<u8>(w + 0xaa) == 1 && h.read::<u32>(w + 0x9c) != 0 {
        call(h, f, &[w, k]);
    }
    1
}

/// `Native_0205_BindEffWalkOn` (`0x0085ec00`, `0x0205`): `0x00862700(worker, 0)`.
pub fn native_0205<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    if h.read::<u8>(w + 0xaa) == 1 && h.read::<u32>(w + 0x9c) != 0 {
        super::helpers::atel_bind_eff2_on(h, w, 0);
    }
    1
}

/// `Native_0206_BindEffWalkOff` (`0x0085ee10`, `0x0206`): `0x00862680(worker, 0)`.
pub fn native_0206<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    chr_call(h, w, 0x0086_2680, 0)
}

/// `Native_020a_BindEffWaterOff` (`0x0085f660`, `0x020a`): `0x00862680(worker, 2)`.
pub fn native_020a<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    chr_call(h, w, 0x0086_2680, 2)
}

/// `Native_010a_Snd3dStop` (`0x0085a630`, `0x010a`): `(i)`: the record
/// `0x008773f0(i)` with a first word gets `0x00886eb0(it)` and
/// `0x008772f0(i)`; 1.
pub fn native_010a<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    let r = call(h, 0x0087_73f0, &[i]);
    let x = h.read::<u32>(r);
    if x != 0 {
        call(h, 0x0088_6eb0, &[x]);
        call(h, 0x0087_72f0, &[i]);
    }
    1
}

/// `Native_00e2_SetBattleStatusSys` (`0x00857dd0`, `0x00e2`): `0x0086f430(v)`; `v`.
pub fn native_00e2<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    super::helpers::atel_set_save_event_13_bit1(h, v);
    v
}

/// `Native_00c4_MoveIsActive` (`0x0085a340`, `0x00c4`): 1 while the move block runs
/// (mode, `+2 & 0x7f`).
pub fn native_00c4<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let m = move_block(h, w);
    u32::from(h.read::<u16>(m + 2) & 0x7f != 0)
}

/// `Native_0003_WorkerSetType3` (`0x0085ebd0`, `0x0003`): `(v)`: `+0xa8` = v, `+0xaa` =
/// 3; 0.
pub fn native_0003<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    #[allow(clippy::cast_possible_truncation)] // the original stores AX
    h.write(w + 0xa8, v as u16);
    h.write(w + 0xaa, 3_u8);
    0
}

/// `Native_024c` (`0x008580a0`, `0x024c`): `+0x38` bit 12; 1.
pub fn native_024c<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let f = h.read::<u16>(w + 0x38);
    h.write(w + 0x38, f | 0x1000);
    1
}

/// `Native_0201_FootSoundOff` (`0x0085a580`, `0x0201`): the bound character (when not
/// -1) gets `0x00887100(chr, 0)` and `0x00887130(chr, 0)`; 1.
pub fn native_0201<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    for f in [0x0088_7100, 0x0088_7130] {
        let x = h.read::<u32>(w + worker::BIND_PC);
        if x != u32::MAX {
            call(h, f, &[x, 0]);
        }
    }
    1
}

/// `Native_0200_FootSoundOnChr` (`0x0085a430`, `0x0200`): `(v)`: `0x00887100(v, 1)` and
/// `0x00887130(v, 1)` (each pushed and popped back); 1.
pub fn native_0200<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    for f in [0x0088_7100, 0x0088_7130] {
        atel_push_stack_integer(h, s, v);
        let x = pop(h, s);
        call(h, f, &[x, 1]);
    }
    1
}

/// `Native_01c5` (`0x0085b1a0`, start): `(a, b)`: the current ctrl work's
/// `+0x210` bit 4 cleared; the args get `+0xa` = b, `+0xb` = a, `+0xc` =
/// `0x40102` (others 0); `0x00888430(b)`; for worker `+0xa8` `0x5045`
/// `0x00648640(1)`, `0x0086e820(1, 1)`, `0x006486e0(2)` and
/// `0x00822570()`'s result, else `0x5045`.
pub fn native_01c5<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    let c = super::atel_get_cur_ctrl_work(h);
    let x = h.read::<u32>(c + 0x210);
    h.write(c + 0x210, x & 0xffff_ffef);
    let pa = pop(h, s);
    let pb = pop(h, s);
    #[allow(clippy::cast_possible_truncation)] // the original stores AL, BL
    {
        h.write(a + 0xa, pb as u8);
        h.write(a + 0xb, pa as u8);
    }
    h.write(a, 0_u32);
    h.write(a + 0xc, 0x0004_0102_u32);
    h.write(a + 4, 0_u32);
    h.write(a + 8, 0_u16);
    call(h, 0x0088_8430, &[pb]);
    if h.read::<u16>(w + 0xa8) != 0x5045 {
        return 0x5045;
    }
    call(h, 0x0064_8640, &[1]);
    call(h, 0x0086_e820, &[1, 1]);
    call(h, 0x0064_86e0, &[2]);
    call(h, 0x0082_2570, &[w, a, s])
}

/// `Native_01c5_Poll` (`0x0085b590`): for worker `+0xa8` `0x5045` waits
/// while `0x00822540()` is -1 and `0x00648100()` is 0; else done (1).
pub fn native_01c5_poll<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    if h.read::<u16>(w + 0xa8) == 0x5045
        && call(h, 0x0082_2540, &[]) == u32::MAX
        && call(h, 0x0064_8100, &[]) == 0
    {
        return 0;
    }
    1
}

/// `Native_01c5_Result` (`0x0085d8f0`): the outcome from the args (`+0xb`
/// the mode, `+0xc` the state, `+0xf`/`+4` for `0x0086e820`), 6 or 2 by
/// `0x00822540()` for worker `+0xa8` `0x5045` (the `dbgPrintf` before it, empty in this
/// build, is left out);
/// `0x00648640(0)`, the ctrl work's `+0x210` bit 4 set.
pub fn native_01c5_result<H: NativeHost>(h: &mut H, w: u32, a: u32, _s: u32) -> u32 {
    let c = super::atel_get_cur_ctrl_work(h);
    let mode = h.read::<u8>(a + 0xb);
    let mut st = i32::from(h.read::<i8>(a + 0xc)).cast_unsigned();
    if mode == 1 {
        st = match st {
            0 | 2 | 5 => 0,
            4 => 4,
            6 => 3,
            _ => st,
        };
    } else if st == 6 {
        let f = u32::from(h.read::<u8>(a + 0xf));
        let x = h.read::<u32>(a + 4);
        call(h, 0x0086_e820, &[f, x]);
        if h.read::<u8>(a + 0xf) == 0 {
            st = 2;
        }
    }
    if h.read::<u16>(w + 0xa8) == 0x5045 {
        st = if call(h, 0x0082_2540, &[]) == 1 { 6 } else { 2 };
    }
    call(h, 0x0064_8640, &[0]);
    let x = h.read::<u32>(c + 0x210);
    h.write(c + 0x210, x | 0x10);
    st
}

/// `Native_0115_CtrlSetByte2Bit3` (`0x0085b8c0`, `0x0115`): `(on)`: the current ctrl
/// work's `+2` bit 3; `on != 0`.
pub fn native_0115<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let c = super::atel_get_cur_ctrl_work(h);
    let v = u8::from(pop(h, s) != 0);
    let x = h.read::<u8>(c + 2);
    h.write(c + 2, x ^ (((v << 3) ^ x) & 8));
    u32::from(v)
}

/// `Native_00fa_SetEventNewGame` (`0x00858920`, `0x00fa`): `0x0088eb40(v)`; `v`.
pub fn native_00fa<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0088_eb40, &[v]);
    v
}

/// `Native_0259` (`0x00858fc0`, `0x0259`, float): `(i, v)`:
/// `0x00872530(i, v)`; `ST0` `v`.
pub fn native_0259<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    let v = popf(h, s);
    let i = pop(h, s);
    call(h, 0x0087_2530, &[i, cp(v)]);
    x87::load(v)
}

/// `Native_0254` (`0x008585d0`, `0x0254`): `(v)`: bits 14-15 of
/// `*0x00785290()` = v, then `0x00a53de0(0x0112ec7c)`; `v`.
pub fn native_0254<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let p = call(h, 0x0078_5290, &[]);
    let v = pop(h, s);
    let x = h.read::<u32>(p);
    h.write(p, x ^ (((v << 14) ^ x) & 0xc000));
    let g = h.rebase(0x0112_ec7c);
    call(h, 0x00a5_3de0, &[g]);
    v
}

/// `Native_01ac_GetMonsterCapture` (`0x00858950`, `0x01ac`): `(v)` -> `0x00790af0(v)`.
pub fn native_01ac<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0079_0af0, &[v])
}

/// `Native_01ab_GetBattleCount` (`0x00858800`, `0x01ab`): jumps to `0x00784f50`.
pub fn native_01ab<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0078_4f50)
}

/// `Native_01b5_TextureVideoPlay` (`0x00859580`, `0x01b5`): `0x00644280(1)`, then jumps to
/// `0x00874060`.
pub fn native_01b5<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    call(h, 0x0064_4280, &[1]);
    tail(h, w, a, s, 0x0087_4060)
}

/// `g_EventReward` (`0x02310f6c`): the pending event reward the
/// `0x015b`/`0x01a7` natives hand out (`+4` item, `+5` key item, `+6`
/// Al Bhed primer flags, `+0x8` item id, `+0x18` its count, `+0x20`
/// key item, `+0x22` primer, `+0` gil).
const EVENT_REWARD: u32 = 0x0231_0f6c;

/// `Native_01a7_FieldItemGet` (`0x00857b70`, start): `(v)`: `0x00798fd0(v)` and the
/// pending reward given without a window (item `0x00790550`, key item
/// `0x0088e790`, primer `0x007ab910(0x007abbd0(..))`, gil `0x007859a0`);
/// the args get `+0xc` = the primer, `+8` = whether it was new, `+0xa` =
/// 0; no new primer runs `0x00798b60()`.
pub fn native_01a7<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0079_8fd0, &[v]);
    let g = h.rebase(EVENT_REWARD);
    let mut primer = 0;
    let mut new = 0_u32;
    if h.read::<u8>(g + 4) != 0 {
        let n = u32::from(h.read::<u8>(g + 0x18));
        let i = u32::from(h.read::<u16>(g + 8));
        call(h, 0x0079_0550, &[i, n]);
    }
    if h.read::<u8>(g + 5) != 0 {
        let k = u32::from(h.read::<u16>(g + 0x20));
        call(h, 0x0088_e790, &[k]);
    }
    if h.read::<u8>(g + 6) != 0 {
        primer = u32::from(h.read::<u16>(g + 0x22));
        let o = h.scratch();
        h.write(o, 0_u32);
        let x = call(h, 0x007a_bbd0, &[primer, o]);
        new = u32::from(call(h, 0x007a_b910, &[x]) == 0);
    }
    let gil = h.read::<u32>(g);
    if gil != 0 {
        call(h, 0x0078_59a0, &[gil.wrapping_neg()]);
    }
    h.write(a + 0xc, primer);
    #[allow(clippy::cast_possible_truncation)] // the original stores SI
    h.write(a + 8, new as u16);
    h.write(a + 0xa, 0_u16);
    if new == 0 {
        return call(h, 0x0079_8b60, &[]);
    }
    a
}

/// `Native_01a7_FieldItemGet_Poll` (`0x00857ef0`): done (5) without a new primer; else
/// state `+0xa` 0 plays `0x00821750(0x10001)` (then 1), 1 ends (1).
pub fn native_01a7_poll<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    if h.read::<u16>(a + 8) == 0 {
        return 5;
    }
    match h.read::<u16>(a + 0xa) {
        0 => {
            call(h, 0x0082_1750, &[0x1_0001]);
            h.write(a + 0xa, 1_u16);
            0
        }
        1 => 1,
        _ => 0,
    }
}

/// `Native_01a7_FieldItemGet_Result` (`0x00858230`): a new primer runs
/// `0x00798b60()`; 1.
pub fn native_01a7_result<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    if h.read::<u16>(a + 8) != 0 {
        call(h, 0x0079_8b60, &[]);
    }
    1
}

/// `Native_01a0` (`0x0085f940`, `0x01a0`): `(track, b, a)`: as `0x0193`
/// with `0x00886e50(track, b, a)` for an unloaded track, the volume `a`
/// (halved when `[record +0x28]`) and also `0x00886e20(track, 0xc, b)`;
/// `track`.
pub fn native_01a0<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    let t = pop(h, s);
    if call(h, 0x0088_6680, &[t]) == 0 {
        call(h, 0x0088_6e50, &[t, b, a]);
        return t;
    }
    let g = call(h, 0x0088_c830, &[]);
    let k = h.read::<u8>(g + 0x28);
    let l = h.read::<u32>(w + 4);
    let n = call(h, 0x0086_ac60, &[l]);
    let mut f = 0;
    let mut i = 0_u32;
    while i.cast_signed() < n.cast_signed() {
        let l = h.read::<u32>(w + 4);
        let x = call(h, 0x0086_ac30, &[l, i]);
        let l = h.read::<u32>(w + 4);
        let y = call(h, 0x0086_ac90, &[l, i]);
        if x == t {
            f = y;
            break;
        }
        i += 1;
    }
    let vol = if k == 0 {
        a
    } else {
        (a << 7).cast_signed().wrapping_div(256).cast_unsigned()
    };
    if f & 0x40 == 0 {
        call(h, 0x0088_6ff0, &[t, 0xc, vol]);
        call(h, 0x0088_6e20, &[t, 0xc, b]);
    }
    t
}

/// `Native_019e_IsCtrl0Running` (`0x00856950`, `0x019e`): 0 pushed and popped back:
/// `0x0086c530(0)`.
pub fn native_019e<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    atel_push_stack_integer(h, s, 0);
    let v = pop(h, s);
    call(h, 0x0086_c530, &[v])
}

/// `Native_00c9_EntranceGetField6` (`0x0085b560`, `0x00c9`, float): `(i)`: the signed word
/// `+6` of `0x0086c020(worker, i)` as a float.
pub fn native_00c9<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let i = pop(h, s);
    let r = call(h, 0x0086_c020, &[w, i]);
    r32(f64::from(h.read::<i16>(r + 6)))
}

/// `Native_01d2_MsgTextMatchesSave` (`0x00860180`, `0x01d2`): `(msg)`: 1 when its text
/// (`0x0086b170` or `0x0086ba90` by `0x0086bbc0()`) equals the record
/// `0x0088c830()`'s string at `+0xec`.
pub fn native_01d2<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let g = call(h, 0x0088_c830, &[]);
    let v = pop(h, s);
    let f = if call(h, 0x0086_bbc0, &[]) == 0 {
        0x0086_ba90
    } else {
        0x0086_b170
    };
    let t = call(h, f, &[v]);
    let mut i = 0_u32;
    loop {
        let x = h.read::<u8>(t.wrapping_add(i));
        if x != h.read::<u8>(g.wrapping_add(0xec + i)) {
            return 0;
        }
        if x == 0 {
            return 1;
        }
        i += 1;
    }
}

/// `Native_01bc` (`0x00859ec0`, `0x01bc`): `0x00870140(v)`; 1.
pub fn native_01bc<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    super::helpers::atel_set_force_place_id(h, v);
    1
}

/// `Native_014f_AngleDiffWithin` (`0x00859120`, `0x014f`): `(c, b, a)` floats: 1 when `c`
/// is greater than `0x0088b400(wrap(b - a), c)`.
pub fn native_014f<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let a = popf(h, s);
    let b = popf(h, s);
    let c = popf(h, s);
    let d = x87::store(x87::sub(x87::load(b), x87::load(a)));
    let e = x87::store(callf(h, super::native::MATH_WRAP_ANGLE, &[cp(d)]));
    let r = callf(h, 0x0088_b400, &[cp(e), cp(c)]);
    u32::from(x87::load(c) > r)
}

/// `Native_025d` (`0x00859c20`, `0x025d`): `(v)` into `[0x01325ac4]`; `v`.
pub fn native_025d<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    h.write(h.rebase(0x0132_5ac4), v);
    v
}

/// `Native_0235` (`0x00860170`, `0x0235`): jumps to `0x008729d0`.
pub fn native_0235<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0087_29d0)
}

/// `Native_0231` (`0x0085fca0`, `0x0231`): `(worker)`: its `+0x38` bit 11;
/// 1.
pub fn native_0231<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    let o = work(h, i);
    let f = h.read::<u16>(o + 0x38);
    h.write(o + 0x38, f | 0x800);
    1
}

/// `Native_01ee_VibrateStop1` (`0x00857440`, `0x01ee`): 1 pushed and popped back,
/// `0x00876290(1)`; 1.
pub fn native_01ee<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    atel_push_stack_integer(h, s, 1);
    let v = pop(h, s);
    call(h, 0x0087_6290, &[v]);
    v
}

/// `Native_01e7_MusicPhraseStart` (`0x00859320`, `0x01e7`): `(track, v)`: `0x008869e0`,
/// `0x008868f0(track, v)`, current music = track; 1.
pub fn native_01e7<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let t = pop(h, s);
    call(h, 0x0088_69e0, &[t]);
    call(h, 0x0088_68f0, &[t, v]);
    call(h, 0x0087_3d00, &[t]);
    1
}

/// `Native_01e0` (`0x00858490`, `0x01e0`): `(v)`: a worker with a
/// character gets `0x0082a9d0(chr, v)`; 1, else 0.
pub fn native_01e0<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    if h.read::<u8>(w + 0xaa) == 1 {
        let c = h.read::<u32>(w + 0x9c);
        if c != 0 {
            call(h, 0x0082_a9d0, &[c, v]);
            return 1;
        }
    }
    0
}

/// `Native_018a_SetEffectFlag2` (`0x0085ebb0`, `0x018a`): `0x008702f0(v)`; `v`.
pub fn native_018a<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0087_02f0, &[v]);
    v
}

/// `Native_014d` (`0x00858e10`, `0x014d`): jumps to `0x0086bcc0`.
pub fn native_014d<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0086_bcc0)
}

/// `Native_009b_TriggerSetLineVec` (`0x0085f750`, `0x009b`): `(a, b)` vector variables: their
/// six floats pushed (`b` first) for [`native_trigger_set_line`].
pub fn native_009b<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    let pa = pop(h, s);
    let pb = pop(h, s);
    let x = atel_get_real_var_adrs(h, w, pb);
    let y = atel_get_real_var_adrs(h, w, pa);
    for p in [x, y] {
        for k in 0..3 {
            let v = cp(h.read(p + k * 4));
            atel_push_stack_float(h, s, v);
        }
    }
    native_trigger_set_line(h, w, a, s)
}

/// `Native_0047_PadMapButtons` (`0x0085db70`, `0x0047`): the flag word
/// `0x00888c40(0, 0, 0x0086af80())`.
pub fn native_0047<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    let x = call(h, 0x0086_af80, &[]) & 0xffff;
    call(h, 0x0088_8c40, &[0, 0, x]) & 0xffff
}

/// `Native_01de_VibrateWait3` (`0x00857920`, start): 3 pushed and popped back into the
/// args; 3.
pub fn native_01de<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    atel_push_stack_integer(h, s, 3);
    let v = pop(h, s);
    h.write(a, v);
    v
}

/// `Native_01de_VibrateWait3_Poll` (`0x008579d0`): done (5) once `0x00875de0(v)` is 0.
pub fn native_01de_poll<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    let v = h.read::<u32>(a);
    if call(h, 0x0087_5de0, &[v]) == 0 {
        5
    } else {
        0
    }
}

/// `Native_01bb_GetMusicPlayState` (`0x00859de0`, `0x01bb`): `0x008863c0(0)`.
pub fn native_01bb<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call(h, 0x0088_63c0, &[0])
}

/// `Native_01af_AddMonsterCaptureN` (`0x00858e70`, `0x01af`): `(b, a)` -> `0x00790b90(b, a)`.
pub fn native_01af<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    call(h, 0x0079_0b90, &[b, a])
}

/// `Native_0140_MsgBuildChoiceText` (`0x00856f20`, `0x0140`): `(dst, list, mask, n, _, msg)`:
/// as `0x0237` with the entries taken from the message table
/// `0x008b70a0(msg, 0x0086bbc0())` (entry `2 * id`, `+1` when
/// `0x0086bad0()`) and no copy after the text; `k << 8`.
pub fn native_0140<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let pd = pop(h, s);
    let pl = pop(h, s);
    let mask = pop(h, s);
    let n = pop(h, s);
    pop(h, s);
    let msg = pop(h, s);
    let dst = atel_get_real_var_adrs(h, w, pd);
    let mut list = atel_get_real_var_adrs(h, w, pl);
    h.write(dst, 0_u8);
    let mut prev = u32::MAX;
    let lang = call(h, 0x0086_bbc0, &[]);
    let txt = call(h, 0x008b_70a0, &[msg, lang]);
    let mut k = 0_u32;
    let head = h.rebase(0x00b5_b9f8);
    strcat(h, dst, head);
    let mut i = 0_u32;
    while i.cast_signed() < n.cast_signed() {
        let e = u32::from(h.read::<u16>(list));
        list = list.wrapping_add(4);
        let mut idx = e * 2;
        if call(h, 0x0086_bad0, &[]) != 0 {
            idx += 1;
        }
        let p = txt.wrapping_add(u32::from(h.read::<u16>(txt.wrapping_add(idx * 2))));
        let bit = u32::from(mask & (1 << (i & 31)) != 0);
        if bit != prev {
            let c = h.rebase(if bit != 0 { 0x00b5_b9e8 } else { 0x00b5_b9ec });
            strcat(h, dst, c);
        }
        prev = bit;
        if bit != 0 {
            let o = h.scratch();
            #[allow(clippy::cast_possible_truncation)] // the original adds to AL
            h.write(
                o,
                u32::from_le_bytes([0x10, (k as u8).wrapping_add(0x30), 0, 0]),
            );
            strcat(h, dst, o);
            k += 1;
        }
        strcat(h, dst, p);
        if i.cast_signed() < n.wrapping_sub(1).cast_signed() {
            let sep = h.rebase(0x00b5_b9f0);
            strcat(h, dst, sep);
        }
        i += 1;
    }
    let tail = h.rebase(0x00b5_b9f4);
    strcat(h, dst, tail);
    k << 8
}

/// `Native_00cc_PartyIsJoined` (`0x0085b9f0`, `0x00cc`): `(chr)`: 1 when
/// `0x00785380(chr)` is 1.
pub fn native_00cc<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    u32::from(call(h, 0x0078_5380, &[v]) == 1)
}

/// `Native_0265` (`0x0085a7f0`, `0x0265`): `[0x00cccacc]`.
pub fn native_0265<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    h.read(h.rebase(0x00cc_cacc))
}

/// `Native_0262_AlwaysTrue` (`0x0085a110`, `0x0262`): 1.
pub fn native_0262<H: NativeHost>(_h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    1
}

/// `Native_0263_AlwaysTrue` (`0x0085a120`, `0x0263`): 1.
pub fn native_0263<H: NativeHost>(_h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    1
}

/// `Native_0245` (`0x00857690`, `0x0245`): `0x007fb680()`; 1.
pub fn native_0245<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call0_1(h, 0x007f_b680)
}

/// `Native_0244` (`0x008574b0`, `0x0244`): `0x007fb690()`; 1.
pub fn native_0244<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call0_1(h, 0x007f_b690)
}

/// `Native_0243` (`0x00857350`, `0x0243`): jumps to `0x007787b0`.
pub fn native_0243<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0077_87b0)
}

/// `Native_023d` (`0x00856520`, `0x023d`): `0x00647e40()`, a pending
/// `[0x025d5ebc]` cleared after `0x00642390(7)`, then `0x00786b00()`,
/// `0x00782050()`, `0x0088d1c0(0)`, `0x00643e70()`; 1.
pub fn native_023d<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call(h, 0x0064_7e40, &[]);
    let g = h.rebase(0x025d_5ebc);
    if h.read::<u32>(g) != 0 {
        call(h, 0x0064_2390, &[7]);
        h.write(g, 0_u32);
    }
    call(h, 0x0078_6b00, &[]);
    call(h, 0x0078_2050, &[]);
    call(h, 0x0088_d1c0, &[0]);
    call(h, 0x0064_3e70, &[]);
    1
}

/// `Native_0239_SetEventFlag` (`0x0085e8b0`, `0x0239`): `(i)`: bit `i` (below 640) of
/// the record `0x0088c830()`'s flags at `+0x4c` set; 1.
pub fn native_0239<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let g = call(h, 0x0088_c830, &[]);
    let v = pop(h, s);
    let i = v >> 5;
    if i < 0x14 {
        let o = g + 0x4c + i * 4;
        let x = h.read::<u32>(o);
        h.write(o, x | (1 << (v & 31)));
    }
    1
}

/// `Native_01b6_EquipHasUnmarked` (`0x00859630`, `0x01b6`): `0x007ad6e0() != 0`.
pub fn native_01b6<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    u32::from(call(h, 0x007a_d6e0, &[]) != 0)
}

/// `Native_013d_SndTransSepGroup` (`0x008561b0`, start): `(v)`: the args get state 0, `v`,
/// a countdown of 2; `v`.
pub fn native_013d<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    h.write(a, 0_u32);
    h.write(a + 4, v);
    h.write(a + 8, 2_u32);
    v
}

/// `Native_013d_SndTransSepGroup_Poll` (`0x00856300`): state 0 waits for `0x00886630()` to
/// clear and the loader's `+8(1)`, then `0x00871750(ctrl +0x48, ctrl
/// +0x204, v)`; 1 `0x00887550()`; 2 waits for `0x00886630()`; 3 counts
/// down, done (1) below 0.
pub fn native_013d_poll<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    let c = super::atel_get_cur_ctrl_work(h);
    let st = h.read::<u32>(a);
    let v = h.read::<u32>(a + 4);
    match st {
        0 => {
            let r = call(h, 0x0076_d0d0, &[]);
            if call(h, 0x0088_6630, &[]) != 0 {
                return 0;
            }
            let f = h.read::<u32>(r + 8);
            if h.call(f, &[1]) != 0 {
                return 0;
            }
            let x = h.read::<u32>(c + 0x48);
            let y = h.read::<u32>(c + 0x204);
            super::helpers::atel_snd_trans_sep_spk_data_group(h, x, y, v);
            h.write(a, 1_u32);
            0
        }
        1 => {
            if call(h, 0x0088_6630, &[]) == 0 {
                call(h, 0x0088_7550, &[]);
                h.write(a, 2_u32);
            }
            0
        }
        2 => {
            if call(h, 0x0088_6630, &[]) == 0 {
                h.write(a, 3_u32);
            }
            0
        }
        3 => {
            let n = h.read::<u32>(a + 8).wrapping_sub(1);
            h.write(a + 8, n);
            u32::from(n.cast_signed() < 0)
        }
        _ => 0,
    }
}

/// `Native_025f` (`0x00859e20`, `0x025f`): `(b, a)`: `0x00643500(b, a)`; 0.
pub fn native_025f<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    call(h, 0x0064_3500, &[b, a]);
    0
}

/// `Native_025e` (`0x00859d50`, `0x025e`): `(v)` -> `0x00643e80(v)`.
pub fn native_025e<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0064_3e80, &[v])
}

/// `Native_024a_SetHddConfig` (`0x00857cb0`, `0x024a`): `(on)`: bit 13 of
/// `*0x00785290()`, then `0x008724a0()`; `on != 0`.
pub fn native_024a<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let p = call(h, 0x0078_5290, &[]);
    let v = u32::from(pop(h, s) != 0);
    let x = h.read::<u32>(p);
    h.write(p, x ^ (((v << 13) ^ x) & 0x2000));
    call(h, 0x0087_24a0, &[]);
    v
}

/// `Native_0242_GetHddStatus` (`0x00857200`, `0x0242`): jumps to `0x00777e90`.
pub fn native_0242<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0077_7e90)
}

/// `Native_0240_SetWeaponInvisible` (`0x008568d0`, `0x0240`): `(b, on)`: `0x007ad5e0(b, on !=
/// 0)`; that.
pub fn native_0240<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let a = u32::from(pop(h, s) != 0);
    let b = pop(h, s);
    call(h, 0x007a_d5e0, &[b, a]);
    a
}

/// `Native_021d` (`0x0085dcd0`, `0x021d`): every primer
/// (`0x007abd10(0, 0, 0, &n)`) whose record (`0x007abbd0`) has `+3` bit 3
/// gets `0x007ab970(0, 0, it)` and `0x008c30d0()`; 1 when any did.
pub fn native_021d<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    let o = h.scratch();
    h.write(o, 0_u32);
    h.write(o + 4, 0_u32);
    let mut p = call(h, 0x007a_bd10, &[0, 0, 0, o]);
    let n = h.read::<u32>(o);
    let mut r = 0;
    let mut i = 0_u32;
    while i.cast_signed() < n.cast_signed() {
        let e = u32::from(h.read::<u16>(p));
        let q = call(h, 0x007a_bbd0, &[e, o + 4]);
        if h.read::<u8>(q + 3) & 8 != 0 {
            call(h, 0x007a_b970, &[0, 0, e]);
            call(h, 0x008c_30d0, &[]);
            r = 1;
        }
        i += 1;
        p = p.wrapping_add(2);
    }
    r
}

/// `Native_0212` (`0x0085c950`, `0x0212`): `(on, v)`: the current ctrl
/// work's `+0x208` = v, `+2` bit 0 = on; `on != 0`.
pub fn native_0212<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let c = super::atel_get_cur_ctrl_work(h);
    let f = popf(h, s);
    let v = u8::from(pop(h, s) != 0);
    h.write(c + 0x208, cp(f));
    let x = h.read::<u8>(c + 2);
    h.write(c + 2, x ^ ((x ^ v) & 1));
    u32::from(v)
}

/// `Native_01f2_GetSummonJoinMask` (`0x008597d0`, `0x01f2`): bits 8-15 set for the
/// characters 8..=15 that `0x00785380` reports.
pub fn native_01f2<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    let mut r = 0;
    for i in 8..=15 {
        if call(h, 0x0078_5380, &[i]) != 0 {
            r |= 1 << i;
        }
    }
    r
}

/// `Native_01f0_SetHalfTurnAngle` (`0x00859550`, `0x01f0`, float): `0x00873e50(v)`; `ST0`
/// `v`.
pub fn native_01f0<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    let v = popf(h, s);
    call(h, 0x0087_3e50, &[cp(v)]);
    x87::load(v)
}

/// `Native_01c2` (`0x0085a570`, `0x01c2`): `0x00873d30()`; 1.
pub fn native_01c2<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call0_1(h, 0x0087_3d30)
}

/// `Native_01b8_EquipMarkFirst` (`0x00859ba0`, `0x01b8`): `0x007ad710(1)`.
pub fn native_01b8<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call(h, 0x007a_d710, &[1])
}

/// `Native_01a6_PartyFillSlots` (`0x008576d0`, `0x01a6`): fills the empty party slots
/// (`0x007852f0`) with the lowest characters 0..=7 that are available
/// (`0x00785380` 1) and not already in a slot, then `0x00786950(slots)`;
/// 1.
pub fn native_01a6<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    let o = h.scratch();
    for i in 0..3 {
        h.write(o + i * 4, 0_u32);
    }
    call(h, 0x0078_52f0, &[o, o + 4, o + 8]);
    let mut m = 0_u32;
    for i in 0..=7 {
        if call(h, 0x0078_5380, &[i]) == 1 {
            m |= 1 << i;
        }
    }
    for i in 0..3 {
        let c = h.read::<u32>(o + i * 4);
        if c != 0xff {
            m &= !(1 << (c & 31));
        }
    }
    let mut k = 0_u32;
    for i in 0..3 {
        if h.read::<u32>(o + i * 4) != 0xff {
            continue;
        }
        if m == 0 {
            break;
        }
        let mut b = 1_u32.rotate_left(k & 31);
        while m & b == 0 {
            b = b.rotate_left(1);
            k = k.wrapping_add(1);
        }
        h.write(o + i * 4, k);
        m &= !b;
        k = k.wrapping_add(1);
    }
    let s: Vec<u32> = (0..3).map(|i| h.read(o + i * 4)).collect();
    call(h, 0x0078_6950, &[s[0], s[1], s[2]]);
    1
}

/// `Native_018b_ClearEffectFlag2` (`0x0085ec90`, `0x018b`): `0x00863af0(v)`; `v`.
pub fn native_018b<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0086_3af0, &[v]);
    v
}

/// `Native_0184_MusicTempoDirect` (`0x0085dac0`, `0x0184`): `0x00886a90(v)` with `v`
/// clamped to -128..=127; 1.
pub fn native_0184<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s).cast_signed().clamp(-0x80, 0x7f).cast_unsigned();
    call(h, 0x0088_6a90, &[v]);
    1
}

/// `Native_0185_MusicTempoTranslate` (`0x0085dbb0`, `0x0185`): `(track, v)`:
/// `0x00886aa0(track, v)` with `v` clamped to -128..=127; 1.
pub fn native_0185<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s).cast_signed().clamp(-0x80, 0x7f).cast_unsigned();
    let t = pop(h, s);
    call(h, 0x0088_6aa0, &[t, v]);
    1
}

/// `Native_0166_MusicStart` (`0x008598f0`, `0x0166`): `(track)`: `0x00886a20`,
/// current music = track; 1.
pub fn native_0166<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let t = pop(h, s);
    call(h, 0x0088_6a20, &[t]);
    call(h, 0x0087_3d00, &[t]);
    1
}

/// `Native_017c_MusicMultiStart` (`0x0085d480`, `0x017c`): `(track)`: `0x008869e0`,
/// `0x008868b0`, current music = track; 1.
pub fn native_017c<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let t = pop(h, s);
    call(h, 0x0088_69e0, &[t]);
    call(h, 0x0088_68b0, &[t]);
    call(h, 0x0087_3d00, &[t]);
    1
}

/// The bound character (when not -1) gets `0x00887100(chr, v)`; 1, else 0.
fn bound_887100<H: NativeHost>(h: &mut H, w: u32, v: u32) -> u32 {
    let x = h.read::<u32>(w + worker::BIND_PC);
    if x == u32::MAX {
        return 0;
    }
    call(h, 0x0088_7100, &[x, v]);
    1
}

/// `Native_0156_SndFootOff` (`0x00859920`, `0x0156`): `0x00887100(chr, 0)`.
pub fn native_0156<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    bound_887100(h, w, 0)
}

/// `Native_0154_SndFootOn` (`0x008597a0`, `0x0154`): `0x00887100(chr, 1)`.
pub fn native_0154<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    bound_887100(h, w, 1)
}

/// `Native_0110_GetTable01868a90` (`0x0085b260`, `0x0110`): `(v)` -> `0x008b7c40(v)`.
pub fn native_0110<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x008b_7c40, &[v])
}

/// `Native_010f_MsgWinSetBytes22` (`0x0085afd0`, `0x010f`): `(window, c, b, a)`: the
/// window's `+0x22` = c, `+0x23` = b, `+0x24` = a; `window`.
pub fn native_010f<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    let c = pop(h, s);
    let i = pop(h, s);
    let m = super::atel_get_mes_win_work(h, i.cast_signed());
    #[allow(clippy::cast_possible_truncation)] // the original stores CL, BL
    {
        h.write(m + 0x23, b as u8);
        h.write(m + 0x22, c as u8);
    }
    h.write(m + 0x24, a);
    i
}

/// `Native_00f9_AlwaysTrue` (`0x00858780`, `0x00f9`): 1 (the debug build answers
/// `TkGetEventNewGame` instead when `TkIsDebug`).
pub fn native_00f9<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    if call(h, 0x0088_7d10, &[]) != 0 {
        return tail(h, w, a, s, 0x0088_d5d0);
    }
    1
}

/// `Native_00ef_WorkerSetFlag36Bit11` (`0x00857470`, `0x00ef`): `(on)`: `+0x36` bit 11;
/// `on != 0`.
pub fn native_00ef<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = u16::from(pop(h, s) != 0);
    let f = h.read::<u16>(w + worker::FLAGS2);
    h.write(w + worker::FLAGS2, f ^ (((v << 11) ^ f) & 0x800));
    u32::from(v)
}

/// `Native_00e6_PartyIsInSlot` (`0x0085bf90`, `0x00e6`): `(chr)`: 1 when it is in a
/// party slot (`0x007852f0`).
pub fn native_00e6<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let o = h.scratch();
    for i in 0..3 {
        h.write(o + i * 4, 0_u32);
    }
    call(h, 0x0078_52f0, &[o, o + 4, o + 8]);
    for i in 0..3 {
        let c = h.read::<u32>(o + i * 4);
        if v == c && c != 0xff {
            return 1;
        }
    }
    0
}

/// `Native_00e1_GetBattleStatusSys` (`0x00857fb0`, `0x00e1`): jumps to `0x0086a9d0`.
pub fn native_00e1<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0086_a9d0)
}

/// `Native_00da_GetBattleStatus` (`0x00857c30`, `0x00da`): jumps to `0x0086a9b0`.
pub fn native_00da<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0086_a9b0)
}

/// `Native_00ac_SetSaveDicValue` (`0x00858050`, `0x00ac`): `0x00870ed0(v)`; `v`.
pub fn native_00ac<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    super::helpers::atel_set_save_event_word(h, 0xe, v);
    v
}

/// `Native_0266` (`0x0085a410`, `0x0266`): jumps to `0x008cc140`.
pub fn native_0266<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x008c_c140)
}

/// `Native_0264_SetBgmType` (`0x0085a230`, `0x0264`): `0x008cc150(v)`; 0.
pub fn native_0264<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x008c_c150, &[v]);
    0
}

/// `Native_0255` (`0x00858430`, `0x0255`): bits 14-15 of
/// `*0x00785290()`.
pub fn native_0255<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    let p = call(h, 0x0078_5290, &[]);
    (h.read::<u32>(p).cast_signed() >> 14).cast_unsigned() & 3
}

/// `Native_022e_SetMcfpInWater` (`0x0085f860`, `0x022e`): `(d, c, b, a)`:
/// `0x00870280(d, c, b, a)`; 1.
pub fn native_022e<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    let c = pop(h, s);
    let d = pop(h, s);
    call(h, 0x0087_0280, &[d, c, b, a]);
    1
}

/// `Native_0229` (`0x0085ece0`, `0x0229`): `+0x38` bit 9; 1.
pub fn native_0229<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let f = h.read::<u16>(w + 0x38);
    h.write(w + 0x38, f | 0x200);
    1
}

/// `Native_01f4` (`0x00859fa0`, `0x01f4`): `(mask, n)`: the `n`-th set
/// bit of `mask` among bits 8-15, -1 when there are fewer.
pub fn native_01f4<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let mask = pop(h, s);
    let n = pop(h, s);
    let mut k = 0_u32;
    for c in 8..=15 {
        if mask & (1 << c) != 0 {
            if n == k {
                return c;
            }
            k += 1;
        }
    }
    u32::MAX
}

/// `Native_01f3_SummonChoiceText` (`0x00859990`, `0x01f3`): `(dst, all, enabled)`: a
/// choice text in `dst` of the characters 8..=15 in either mask (names
/// `0x00873b50`), the enabled ones with the enabled colour and a choice
/// code, the others with the disabled colour; `k << 8`.
pub fn native_01f3<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let pd = pop(h, s);
    let m1 = pop(h, s);
    let m2 = pop(h, s);
    let dst = atel_get_real_var_adrs(h, w, pd);
    let any = m2 | m1;
    h.write(dst, 0_u8);
    let mut count = 0_u32;
    let mut k = 0_u32;
    let head = h.rebase(0x00b5_b9f8);
    strcat(h, dst, head);
    for c in 8..=15 {
        let bit = 1 << c;
        if any & bit == 0 {
            continue;
        }
        let on = m2 & bit != 0;
        if count != 0 {
            let sep = h.rebase(0x00b5_b9f0);
            strcat(h, dst, sep);
        }
        if on {
            let col = h.rebase(0x00b5_b9e8);
            strcat(h, dst, col);
            let o = h.scratch();
            #[allow(clippy::cast_possible_truncation)] // the original adds to AL
            h.write(
                o,
                u32::from_le_bytes([0x10, (k as u8).wrapping_add(0x30), 0, 0]),
            );
            strcat(h, dst, o);
            k += 1;
        } else {
            let col = h.rebase(0x00b5_b9ec);
            strcat(h, dst, col);
        }
        let name = call(h, 0x0087_3b50, &[c]);
        strcat(h, dst, name);
        count += 1;
    }
    k << 8
}

/// `Native_01c6` (`0x0085de70`, `0x01c6`): jumps to `0x0088e5e0`.
pub fn native_01c6<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    tail(h, w, a, s, 0x0088_e5e0)
}

/// `Native_01c0_SetEffectParam` (`0x0085a250`, `0x01c0`): `(i, v)`: `0x0086fa60(i, v)`
/// with `i` clamped to 0..=3; `v`.
pub fn native_01c0<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let i = pop(h, s).cast_signed().clamp(0, 3).cast_unsigned();
    call(h, 0x0086_fa60, &[i, v]);
    v
}

/// `Native_01b7_ChangeWeaponWakka` (`0x00859800`, `0x01b7`): `(v)`: `0x007ad640(v)` when
/// `0x007ad6e0()`; 1, else 0.
pub fn native_01b7<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    if call(h, 0x007a_d6e0, &[]) == 0 {
        return 0;
    }
    call(h, 0x007a_d640, &[v]);
    1
}

/// `Native_01b2_GetItemCount` (`0x00859360`, `0x01b2`): `(v)` -> `0x007904b0(v)`.
pub fn native_01b2<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0079_04b0, &[v])
}

/// `Native_018f_VoiceVolumeTranslate` (`0x0085f240`, `0x018f`): `(b, a)`: `0x00873d20(a)`,
/// `0x008879b0(b, a)`; 0.
pub fn native_018f<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    call(h, 0x0087_3d20, &[a]);
    call(h, 0x0088_79b0, &[b, a]);
    0
}

/// `Native_0179_WaitEffectFlag` (`0x0085cd90`, start): `(v)` into the args; `v`.
pub fn native_0179<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    h.write(a, v);
    v
}

/// `Native_0179_WaitEffectFlag_Poll` (`0x0085cea0`): done (5) once `0x0086b030(v)` is set.
pub fn native_0179_poll<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    let v = h.read::<u32>(a);
    if call(h, 0x0086_b030, &[v]) == 0 {
        0
    } else {
        5
    }
}

/// `Native_0138_SetGil` (`0x0085f140`, `0x0138`): `0x00785c20(v)`, then jumps to
/// `0x00784e80`.
pub fn native_0138<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0078_5c20, &[v]);
    tail(h, w, a, s, 0x0078_4e80)
}

/// `Native_0127_WorkerInTouchRange` (`0x0085db10`, `0x0127`): `(worker)`: [`native_0128`]
/// with the own touch range (position `+0x48`) pushed.
pub fn native_0127<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    let p = atel_get_pos_work_adrs(h, w);
    let r = cp(h.read(p + 0x48));
    atel_push_stack_float(h, s, r);
    native_0128(h, w, a, s)
}

/// `Native_0107_Snd3dSetField8` (`0x0085a1f0`, `0x0107`): `(i, v)`: the record
/// `0x008773f0(i)` with an object (`+4`, its `+0` bit 1 set) gets `+8` =
/// v; 1.
pub fn native_0107<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let i = pop(h, s);
    let r = call(h, 0x0087_73f0, &[i]);
    let c = h.read::<u32>(r + 4);
    if c != 0 {
        let f = h.read::<u16>(c);
        h.write(c, f | 2);
        #[allow(clippy::cast_possible_truncation)] // the original stores BX
        h.write(r + 8, v as u16);
    }
    1
}

/// `Native_00eb` (`0x00856980`, `0x00eb`): the worker's level (`+0x32`).
pub fn native_00eb<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    lvl(h, w)
}

/// `Native_00ba` (`0x008594a0`, `0x00ba`): `+0x44` = 0; 1.
pub fn native_00ba<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    h.write(w + 0x44, 0_u32);
    1
}

/// The three words of `0x0083adf0`.
fn get_83adf0<H: NativeHost>(h: &mut H) -> (u32, u32, u32) {
    let o = h.scratch();
    for i in 0..3 {
        h.write(o + i * 4, 0_u32);
    }
    call(h, 0x0083_adf0, &[o, o + 4, o + 8]);
    (h.read(o), h.read(o + 4), h.read(o + 8))
}

/// `Native_00b4_SetTkAntiY` (`0x00858a20`, `0x00b4`): `(v)`: the second of
/// `0x0083adf0`'s three words replaced by `v` (`0x0083b110`); `v`.
pub fn native_00b4<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let (a, _, c) = get_83adf0(h);
    let v = pop(h, s);
    call(h, 0x0083_b110, &[a, v, c]);
    v
}

/// `Native_00b3_SetTkAntiX` (`0x00858810`, `0x00b3`): `(v)`: the first of
/// `0x0083adf0`'s words replaced by `v` (`0x0083b110`); `v`.
pub fn native_00b3<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let (_, b, c) = get_83adf0(h);
    let v = pop(h, s);
    call(h, 0x0083_b110, &[v, b, c]);
    v
}

/// `Native_00b2_SgEnableTkAnti` (`0x008586b0`, `0x00b2`): `(on)` -> `0x0083add0(on != 0)`.
pub fn native_00b2<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = u32::from(pop(h, s) != 0);
    call(h, 0x0083_add0, &[v])
}

/// `Native_0012_SetPosXZ` (`0x0085f800`, `0x0012`): `(x, z)` floats:
/// `0x00870970(worker, x, z, 0)`; 0.
pub fn native_0012<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let z = popf(h, s);
    let x = popf(h, s);
    call(h, 0x0087_0970, &[w, cp(x), cp(z), 0]);
    0
}

/// The reward/item window tail shared by the `0x015b` and `0x0216` starts:
/// `Native_MsgSetPos(win, 4, 0xd0, 0x100)`, `+0x20` = 0, the text `msg`
/// (`+8`, `+0xc`, `+0x16` = 0), `+0x18` = 0, `Native_MsgOpen(win, 0)`
/// (every value pushed and popped back through the stack, as the original).
fn reward_window<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32, win: u32, msg: u32) {
    for v in [win, 0x100, 0xd0, 4] {
        atel_push_stack_integer(h, s, v);
    }
    call(h, 0x0085_80c0, &[w, a, s]);
    atel_push_stack_integer(h, s, win);
    atel_push_stack_integer(h, s, 0);
    let x = pop(h, s);
    let i = pop(h, s);
    let m = super::atel_get_mes_win_work(h, i.cast_signed());
    #[allow(clippy::cast_possible_truncation)] // the original stores BL
    h.write(m + 0x20, x as u8);
    for v in [win, 0, msg] {
        atel_push_stack_integer(h, s, v);
    }
    let y = pop(h, s);
    let z = pop(h, s);
    let i = pop(h, s);
    let m = super::atel_get_mes_win_work(h, i.cast_signed());
    h.write(m + 8, y);
    h.write(m + 0xc, y);
    #[allow(clippy::cast_possible_truncation)] // the original stores SI
    h.write(m + 0x16, z as u16);
    atel_push_stack_integer(h, s, win);
    atel_push_stack_integer(h, s, 0);
    let z = pop(h, s);
    let i = pop(h, s);
    let m = super::atel_get_mes_win_work(h, i.cast_signed());
    h.write(m + 0x18, z);
    atel_push_stack_integer(h, s, win);
    atel_push_stack_integer(h, s, 0);
    call(h, 0x0085_8b50, &[w, a, s]);
}

/// `TkBtlEndGetText(id)` (`0x008bda60`).
fn text<H: NativeHost>(h: &mut H, id: u32) -> u32 {
    call(h, 0x008b_da60, &[id])
}

/// `Native_015b_GiveEventReward` (`0x0085a8a0`, start): `(event, window)`: the pending
/// [`EVENT_REWARD`] given with a window: sound `0x80000026`, the ctrl
/// work's `+3` bit 2, `0x00798fd0(event)`, the window's macros
/// (`TOMacroSetType`/`TOMacroSetValue`) and text (`0x401d`, item
/// `0x4018`/`0x4019`, key item `0x4017`, primer `0x4016`, gil `0x401a`),
/// each part given (`0x00790550`, `0x0088e790`, `0x007ab910`,
/// `0x007859a0`), the window opened; the args get the window, 1, whether
/// the primer was new and the primer (no new primer: `0x00798b60()`);
/// `0x0086a130()`, the window's `+0x1d` bit 4; the window record.
pub fn native_015b<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    call(h, 0x0088_6e80, &[0x8000_0026]);
    let c = super::atel_get_cur_ctrl_work(h);
    let b = h.read::<u8>(c + 3);
    h.write(c + 3, b | 4);
    let ev = pop(h, s);
    let win = pop(h, s);
    call(h, 0x0079_8fd0, &[ev]);
    call(h, 0x008b_8960, &[win, 0, 0]);
    call(h, 0x008b_8960, &[win, 1, 1]);
    let mut new = 0_u32;
    let mut primer = 0_u32;
    let mut msg = text(h, 0x401d);
    let g = h.rebase(EVENT_REWARD);
    let o = h.scratch();
    h.write(o, 0_u32);
    if h.read::<u8>(g + 4) != 0 {
        let i = u32::from(h.read::<u16>(g + 8));
        call(h, 0x008a_b280, &[i, o]);
        let n = u32::from(h.read::<u8>(g + 0x18));
        call(h, 0x008b_8980, &[win, 1, n]);
        msg = text(
            h,
            if h.read::<u8>(g + 0x18) == 1 {
                0x4018
            } else {
                0x4019
            },
        );
        let n = u32::from(h.read::<u8>(g + 0x18));
        let i = u32::from(h.read::<u16>(g + 8));
        call(h, 0x0079_0550, &[i, n]);
    }
    if h.read::<u8>(g + 5) != 0 {
        let k = u32::from(h.read::<u16>(g + 0x20));
        let x = call(h, 0x0079_0860, &[k]);
        h.write(o, x);
        msg = text(h, 0x4017);
        let k = u32::from(h.read::<u16>(g + 0x20));
        call(h, 0x0088_e790, &[k]);
    }
    if h.read::<u8>(g + 6) != 0 {
        primer = u32::from(h.read::<u16>(g + 0x22));
        msg = text(h, 0x4016);
        let x = call(h, 0x007a_bbd0, &[primer, o]);
        new = u32::from(call(h, 0x007a_b910, &[x]) == 0);
    }
    let gil = h.read::<u32>(g);
    if gil != 0 {
        call(h, 0x008b_8980, &[win, 1, gil]);
        msg = text(h, 0x401a);
        let gil = h.read::<u32>(g);
        call(h, 0x0078_59a0, &[gil.wrapping_neg()]);
    }
    let x = h.read::<u32>(o);
    call(h, 0x008b_8980, &[win, 0, x]);
    reward_window(h, w, a, s, win, msg);
    h.write(a + 0xa, 0_u16);
    h.write(a, win);
    h.write(a + 4, 1_u32);
    #[allow(clippy::cast_possible_truncation)] // the original stores AX
    h.write(a + 8, new as u16);
    h.write(a + 0xc, primer);
    if new == 0 {
        call(h, 0x0079_8b60, &[]);
    }
    super::helpers::atel_uc_resume(h);
    let m = super::atel_get_mes_win_work(h, win.cast_signed());
    let f = h.read::<u8>(m + 0x1d);
    h.write(m + 0x1d, f | 0x10);
    m
}

/// `Native_0216` (`0x0085b280`, start): `(window, item, n)`: when
/// `0x00785020(item, n)` refuses, the args' state `+0xe` = 2 and its
/// result; else the item window (sound `0x80000026`, text `0x401e`, the
/// ctrl work's `+3` bit 2, the macros with `0x008ac850(item)` and
/// `0x008c1a20(n)`, opened as `0x015b`'s); the args get the window, 1,
/// `+8` = item, `+0xc` = n, state 0; `0x0086a130()`, `+0x1d` bit 4; the
/// window record.
pub fn native_0216<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    let n = pop(h, s);
    let item = pop(h, s);
    let win = pop(h, s);
    let r = call(h, 0x0078_5020, &[item, n]);
    if r != 0 {
        h.write(a + 0xe, 2_u16);
        return r;
    }
    call(h, 0x0088_6e80, &[0x8000_0026]);
    let msg = text(h, 0x401e);
    let c = super::atel_get_cur_ctrl_work(h);
    let b = h.read::<u8>(c + 3);
    h.write(c + 3, b | 4);
    call(h, 0x008b_8960, &[win, 0, 0]);
    call(h, 0x008b_8960, &[win, 1, 0]);
    let x = call(h, 0x008a_c850, &[item]);
    call(h, 0x008b_8980, &[win, 0, x]);
    let y = call(h, 0x008c_1a20, &[n]);
    call(h, 0x008b_8980, &[win, 1, y]);
    reward_window(h, w, a, s, win, msg);
    #[allow(clippy::cast_possible_truncation)] // the original stores AX
    h.write(a + 0xc, n as u16);
    h.write(a + 8, item);
    h.write(a, win);
    h.write(a + 4, 1_u32);
    h.write(a + 0xe, 0_u16);
    super::helpers::atel_uc_resume(h);
    let m = super::atel_get_mes_win_work(h, win.cast_signed());
    let f = h.read::<u8>(m + 0x1d);
    h.write(m + 0x1d, f | 0x10);
    m
}

/// The Rust version of one of these handlers, if there is one.
#[must_use]
#[allow(clippy::too_many_lines)] // one arm per handler
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
        0x0085_e620 => Int(native_0006),
        0x0085_8f40 => Float(native_00f5),
        0x0085_b820 => Int(native_00cb),
        0x00a7_8210 => Int(native_menu_open),
        0x00a7_8270 => Int(native_menu_open_poll),
        0x00a7_82a0 => Int(native_menu_open_result),
        0x0085_a8a0 => Int(native_015b),
        0x0085_b280 => Int(native_0216),
        0x0085_7690 => Int(native_0245),
        0x0085_74b0 => Int(native_0244),
        0x0085_7350 => Int(native_0243),
        0x0085_6520 => Int(native_023d),
        0x0085_e8b0 => Int(native_0239),
        0x0085_9630 => Int(native_01b6),
        0x0085_61b0 => Int(native_013d),
        0x0085_6300 => Int(native_013d_poll),
        0x0085_9e20 => Int(native_025f),
        0x0085_9d50 => Int(native_025e),
        0x0085_7cb0 => Int(native_024a),
        0x0085_7200 => Int(native_0242),
        0x0085_68d0 => Int(native_0240),
        0x0085_dcd0 => Int(native_021d),
        0x0085_c950 => Int(native_0212),
        0x0085_97d0 => Int(native_01f2),
        0x0085_9550 => Float(native_01f0),
        0x0085_a570 => Int(native_01c2),
        0x0085_9ba0 => Int(native_01b8),
        0x0085_76d0 => Int(native_01a6),
        0x0085_ec90 => Int(native_018b),
        0x0085_dac0 => Int(native_0184),
        0x0085_dbb0 => Int(native_0185),
        0x0085_98f0 => Int(native_0166),
        0x0085_d480 => Int(native_017c),
        0x0085_9920 => Int(native_0156),
        0x0085_97a0 => Int(native_0154),
        0x0085_b260 => Int(native_0110),
        0x0085_afd0 => Int(native_010f),
        0x0085_8780 => Int(native_00f9),
        0x0085_7470 => Int(native_00ef),
        0x0085_bf90 => Int(native_00e6),
        0x0085_7fb0 => Int(native_00e1),
        0x0085_7c30 => Int(native_00da),
        0x0085_8050 => Int(native_00ac),
        0x0085_a410 => Int(native_0266),
        0x0085_a230 => Int(native_0264),
        0x0085_8430 => Int(native_0255),
        0x0085_f860 => Int(native_022e),
        0x0085_ece0 => Int(native_0229),
        0x0085_9fa0 => Int(native_01f4),
        0x0085_9990 => Int(native_01f3),
        0x0085_de70 => Int(native_01c6),
        0x0085_a250 => Int(native_01c0),
        0x0085_9800 => Int(native_01b7),
        0x0085_9360 => Int(native_01b2),
        0x0085_f240 => Int(native_018f),
        0x0085_cd90 => Int(native_0179),
        0x0085_cea0 => Int(native_0179_poll),
        0x0085_f140 => Int(native_0138),
        0x0085_db10 => Int(native_0127),
        0x0085_a1f0 => Int(native_0107),
        0x0085_6980 => Int(native_00eb),
        0x0085_94a0 => Int(native_00ba),
        0x0085_8a20 => Int(native_00b4),
        0x0085_8810 => Int(native_00b3),
        0x0085_86b0 => Int(native_00b2),
        0x0085_f800 => Int(native_0012),
        0x0085_bf70 => Int(native_020d),
        0x0085_8580 => Int(native_0149),
        0x0085_81a0 => Int(native_0146),
        0x0085_7fc0 => Int(native_0145),
        0x0085_eb80 => Int(native_0226),
        0x0085_9240 => Int(native_01b1),
        0x0085_9c40 => Int(native_0167),
        0x0085_fff0 => Int(native_0234),
        0x0085_c6f0 => Int(native_00d2),
        0x0085_b780 => Int(native_0037),
        0x0085_d290 => Int(native_0159),
        0x0085_dc90 => Float(native_0204),
        0x0085_a750 => Int(native_01c3),
        0x0085_9f10 => Int(native_0260),
        0x0085_a090 => Int(native_0261),
        0x0085_e5e0 => Int(native_0251),
        0x0085_ec00 => Int(native_0205),
        0x0085_ee10 => Int(native_0206),
        0x0085_f660 => Int(native_020a),
        0x0085_a630 => Int(native_010a),
        0x0085_7dd0 => Int(native_00e2),
        0x0085_a340 => Int(native_00c4),
        0x0085_ebd0 => Int(native_0003),
        0x0085_80a0 => Int(native_024c),
        0x0085_a580 => Int(native_0201),
        0x0085_a430 => Int(native_0200),
        0x0085_b1a0 => Int(native_01c5),
        0x0085_b590 => Int(native_01c5_poll),
        0x0085_d8f0 => Int(native_01c5_result),
        0x0085_b8c0 => Int(native_0115),
        0x0085_8920 => Int(native_00fa),
        0x0085_8fc0 => Float(native_0259),
        0x0085_85d0 => Int(native_0254),
        0x0085_8950 => Int(native_01ac),
        0x0085_8800 => Int(native_01ab),
        0x0085_9580 => Int(native_01b5),
        0x0085_7b70 => Int(native_01a7),
        0x0085_7ef0 => Int(native_01a7_poll),
        0x0085_8230 => Int(native_01a7_result),
        0x0085_f940 => Int(native_01a0),
        0x0085_6950 => Int(native_019e),
        0x0085_b560 => Float(native_00c9),
        0x0086_0180 => Int(native_01d2),
        0x0085_9ec0 => Int(native_01bc),
        0x0085_9120 => Int(native_014f),
        0x0085_9c20 => Int(native_025d),
        0x0086_0170 => Int(native_0235),
        0x0085_fca0 => Int(native_0231),
        0x0085_7440 => Int(native_01ee),
        0x0085_9320 => Int(native_01e7),
        0x0085_8490 => Int(native_01e0),
        0x0085_ebb0 => Int(native_018a),
        0x0085_8e10 => Int(native_014d),
        0x0085_f750 => Int(native_009b),
        0x0085_db70 => Int(native_0047),
        0x0085_7920 => Int(native_01de),
        0x0085_79d0 => Int(native_01de_poll),
        0x0085_9de0 => Int(native_01bb),
        0x0085_8e70 => Int(native_01af),
        0x0085_6f20 => Int(native_0140),
        0x0085_b9f0 => Int(native_00cc),
        0x0085_a7f0 => Int(native_0265),
        0x0085_a110 => Int(native_0262),
        0x0085_a120 => Int(native_0263),
        0x0085_7ad0 => Int(native_0143),
        0x0085_7d40 => Int(native_0144),
        0x0085_9270 => Int(native_025b),
        0x0085_ffa0 => Int(native_0198),
        0x0086_0200 => Int(native_0199),
        0x0085_7a00 => Int(native_0141),
        0x0085_c400 => Int(native_00cf),
        0x0085_c4b0 => Int(native_00d0),
        0x0085_be30 => Int(native_020b),
        0x0085_e340 => Int(native_01ca),
        0x0086_04c0 => Int(native_0237),
        0x0085_b520 => Int(native_0112),
        0x0085_b4b0 => Int(native_0111),
        0x0085_b170 => Int(native_01fc),
        0x0085_f0e0 => Int(native_0051),
        0x0085_82e0 => Int(native_01d9),
        0x0085_6270 => Int(native_00e8),
        0x0085_a7c0 => Float(native_0030),
        0x0085_5dd0 => Int(native_01d4),
        0x0085_5fb0 => Int(native_022f),
        0x0085_e3a0 => Float(native_0174),
        0x0085_b760 => Int(native_0161),
        0x0085_bbb0 => Int(native_016a),
        0x0086_0090 => Int(native_00fe),
        0x0085_d4e0 => Int(native_0044),
        0x0085_6dc0 => Int(native_024d),
        0x0085_f430 => Int(native_0209),
        0x0085_ac90 => Int(native_010d),
        0x0085_f8b0 => Int(native_0194),
        0x0085_8610 => Int(native_010c),
        0x0085_8330 => Int(native_00ab),
        0x0085_c0a0 => Int(native_011a),
        0x0085_8e40 => Int(native_021e),
        0x0085_8ea0 => Float(native_014e),
        0x0085_7e30 => Int(native_024b),
        0x0085_af60 => Int(native_01fa),
        0x0085_af10 => Int(native_01fb),
        0x0085_b5d0 => Int(native_0216_poll),
        0x0085_b920 => Int(native_0216_result),
        0x0085_e570 => Int(native_01f8),
        0x0085_b670 => Int(native_0114),
        0x0085_e010 => Int(native_0220),
        0x0085_d460 => Int(native_0203),
        0x0085_d550 => Int(native_0203_poll),
        0x0085_dc10 => Int(native_0203_result),
        0x0085_8390 => Int(native_01a8),
        0x0085_f700 => Float(native_0192),
        0x0085_a840 => Int(native_01c4),
        0x0085_e040 => Int(native_012e),
        0x0085_dc50 => Int(native_012b),
        0x0085_eac0 => Float(native_0092),
        0x0085_6250 => Int(native_023a),
        0x0085_9000 => Int(native_01e5),
        0x0085_fb00 => Int(native_0195_poll),
        0x0085_6770 => Int(native_023f),
        0x0085_ea40 => Int(native_01cf),
        0x0085_90a0 => Int(native_01b0),
        0x0085_c100 => Int(native_020f),
        0x0085_71d0 => Int(native_01dc),
        0x0085_a770 => Int(native_0202),
        0x0085_fb50 => Int(native_0098),
        0x0085_d3a0 => Int(native_0217),
        0x0085_a190 => Int(native_01bf),
        0x0085_f4e0 => Int(native_0193),
        0x0085_e890 => Int(native_0189),
        0x0085_ea30 => Int(native_0189_poll),
        0x0085_ed00 => Int(native_0135),
        0x0085_6570 => Int(native_00a2),
        0x0085_72b0 => Int(native_00ee),
        0x0085_b650 => Int(native_015f),
        0x0085_88b0 => Int(native_0257),
        0x0085_8710 => Int(native_0256),
        0x0085_e8f0 => Float(native_0093),
        0x0085_e410 => Int(native_0130),
        0x0085_71c0 => Int(native_00ed),
        0x0085_bd10 => Int(native_017f),
        0x0085_ee40 => Int(native_0136),
        0x0085_ef80 => Int(native_0137),
        0x0085_9dc0 => Int(native_0157),
        0x0085_9d00 => Int(native_0155),
        0x0086_0460 => Int(native_0100),
        0x0085_be70 => Int(native_016b),
        0x0085_6170 => Int(native_005a),
        0x0085_9950 => Int(native_00be),
        0x0085_7360 => Int(native_01a4),
        0x0085_d1d0 => Int(native_0121),
        0x0085_d310 => Int(native_0122),
        0x0086_0730 => Int(native_01c9),
        0x0086_0390 => Int(native_009a),
        0x0085_f280 => Int(native_00de_poll),
        0x0085_f0b0 => Float(native_0094),
        0x0085_8070 => Int(native_01eb),
        0x0085_d780 => Int(native_021a),
        0x0085_f010 => Int(native_0207),
        0x0085_e270 => Float(native_0090),
        0x0085_e400 => Int(native_01cc),
        0x0085_67a0 => Int(native_019f),
        0x0085_fce0 => Int(native_0196),
        0x0085_a870 => Int(native_0109),
        0x0085_a370 => Int(native_0108),
        0x0085_e9e0 => Int(native_00db),
        0x0085_eaf0 => Int(native_00db_poll),
        0x0085_c650 => Int(native_011d),
        0x0085_ee00 => Int(native_00dc),
        0x0085_ee70 => Int(native_00dc_poll),
        0x0085_9400 => Int(native_025c),
        0x0085_9840 => Int(native_00bd),
        0x0085_8b30 => Int(native_01ad),
        0x0085_8940 => Int(native_01e2),
        0x0085_a010 => Int(native_0106),
        0x0085_c9c0 => Int(native_0177),
        0x0085_b900 => Int(native_0160),
        0x0085_6910 => Int(native_0010),
        0x0085_c600 => Int(native_023c),
        0x0085_c490 => Int(native_023b),
        0x0085_fbb0 => Int(native_01d0),
        0x0085_e7c0 => Int(native_01ce),
        0x0085_ac50 => Int(native_01f9),
        0x0085_9d30 => Float(native_00bf),
        0x0085_9e00 => Float(native_00c0),
        0x0085_9ef0 => Float(native_00c1),
        0x0085_a070 => Float(native_00c2),
        0x0085_d3d0 => Int(native_017b),
        0x0085_94c0 => Int(native_016c),
        0x0085_6bc0 => Int(native_00ec),
        0x0085_d280 => Int(native_0089),
        0x0085_8ab0 => Int(native_01e3),
        0x0085_9ff0 => Int(native_01bd),
        0x0085_ec60 => Int(native_0050),
        0x0085_e110 => Int(native_0221),
        0x0085_fea0 => Int(native_set_pos_0126),
        0x0085_c130 => Int(native_line_extend),
        0x0085_ad00 => Int(native_get_turn_frames),
        0x0085_ce30 => Int(native_chr_get_attach_bone),
        0x0085_cb40 => Int(native_set_party_index),
        0x0085_cfa0 => Int(native_0158),
        0x0085_8d30 => Float(native_get_pitch_to_move_target),
        0x0085_b700 => Int(native_party_add),
        0x0085_f370 => Int(native_trigger_set_line),
        0x0085_ca00 => Int(worker_detach),
        0x0085_e920 => Int(native_attach_camera),
        0x0085_f460 => Float(native_set_pitch),
        0x0085_c5b0 => Int(native_00d1),
        0x0085_beb0 => Int(native_01ba),
        0x0085_b4c0 => Int(native_015b_result),
        0x0085_af70 => Int(native_015b_poll),
        0x0085_e290 => Int(native_music_fade_out),
        0x0086_03e0 => Int(native_trigger_set_point),
        0x0085_7df0 => Int(native_00f6),
        0x0085_7ff0 => Int(native_00f6_poll),
        0x0085_fa20 => Int(native_0055),
        0x0085_66c0 => Float(native_00a4),
        0x0085_6990 => Float(native_00a5),
        0x0085_8e50 => Int(native_music_load),
        0x0085_9040 => Int(native_music_load_poll),
        0x0085_9250 => Int(native_music_load_result),
        0x0085_9d70 => Int(native_music_wait_loaded),
        0x0085_9df0 => Int(native_music_wait_loaded_poll),
        0x0085_9ee0 => Int(native_music_wait_loaded_result),
        0x0085_d210 => Int(native_017a),
        0x0085_cf90 => Int(native_0087),
        0x0085_cdb0 => Int(native_0086),
        0x0085_d160 => Int(native_0088),
        0x0085_af50 => Int(native_010e),
        0x0085_9bb0 => Float(native_002a),
        0x0085_5cd0 => Int(native_0059),
        0x0085_7680 => Int(native_random16),
        0x0085_6890 => Int(native_013e),
        0x0085_e130 => Int(native_004c),
        0x0085_e820 => Int(native_004d),
        0x0085_c2d0 => Int(native_set_map_marker_pos),
        0x0085_93a0 => Int(native_0103),
        0x0085_95b0 => Int(native_016d),
        0x0085_90f0 => Int(native_01e9),
        0x0085_92c0 => Int(native_01e9_poll),
        0x0085_95f0 => Int(native_01e9_result),
        0x0085_7c40 => Int(native_minimap_set_corner),
        0x0085_7950 => Int(native_00a8),
        0x0085_8740 => Int(native_014a),
        0x0085_d170 => Int(native_0215),
        0x0085_fe30 => Int(native_0132),
        0x0085_7850 => Int(native_00f2),
        0x0085_7a80 => Int(native_015d),
        0x0085_ea10 => Int(native_0225),
        0x0085_df80 => Int(native_012d),
        0x0085_c2b0 => Int(native_00ce),
        0x0085_d830 => Int(native_017e),
        0x0085_9d80 => Float(native_0025),
        0x0085_edd0 => Int(native_0197),
        0x0085_d8c0 => Int(native_021b),
        0x0085_d7b0 => Int(native_0128),
        0x0085_89d0 => Int(native_0069),
        0x0085_f730 => Int(native_0139),
        0x0085_f850 => Int(native_0139_poll),
        0x0085_f900 => Int(native_0139_result),
        0x0085_6a50 => Float(native_00b1),
        0x0085_6660 => Int(native_00e9),
        0x0085_bc50 => Int(native_0117),
        0x0085_8210 => Int(native_lang_group_not_jp_asia),
        0x0085_6a80 => Int(native_01da),
        0x0085_e550 => Float(native_0181),
        0x0085_c7b0 => Float(native_get_entrance_x),
        0x0085_c8f0 => Float(native_get_entrance_y),
        0x0085_a1a0 => Float(native_002c),
        0x0085_a0b0 => Float(native_002b),
        0x0085_9430 => Float(native_0021),
        0x0085_7980 => Int(native_015e),
        0x0085_bd70 => Int(native_0119),
        0x0085_c320 => Int(native_set_trigger_mode),
        0x0085_c9a0 => Int(native_set_player),
        0x0085_e0b0 => Float(native_0091),
        0x0085_ecb0 => Int(native_0004),
        0x0085_5f60 => Int(native_00e0),
        0x0085_74f0 => Float(native_set_talk_angle),
        0x0085_6460 => Int(native_set_touch_range),
        0x0085_77c0 => Float(native_set_tyz_angle),
        0x0085_72e0 => Float(native_set_radius44),
        0x0085_7820 => Int(native_00a7),
        0x0085_5f80 => Int(native_0058),
        0x0085_8ac0 => Int(native_00fb),
        0x0085_c000 => Int(native_0180),
        0x0085_b030 => Float(native_set_turn_speed_all),
        0x0085_ab80 => Float(native_set_move_param3c),
        0x0085_acd0 => Float(native_set_move_param40),
        0x0085_af20 => Float(native_set_turn_param10),
        0x0085_b230 => Float(native_set_turn_param18),
        0x0085_b480 => Float(native_set_turn_param1c),
        0x0085_f5a0 => Int(native_get_own_worker_type),
        0x0085_7a60 => Int(native_00aa),
        0x0085_96d0 => Int(native_music_play),
        0x0085_c8c0 => Int(native_0007),
        0x0085_c890 => Int(native_012a),
        0x0086_0290 => Int(native_0133),
        0x0085_c430 => Int(native_016f),
        0x0086_0350 => Int(native_0236),
        _ => return super::native_btl::find(sym),
    })
}

/// Every handler [`find`] has, with whether it returns a float.
pub const PORTED: [(u32, bool); 411] = [
    (0x0085_a8a0, false),
    (0x0085_b280, false),
    (0x0085_7690, false),
    (0x0085_74b0, false),
    (0x0085_7350, false),
    (0x0085_6520, false),
    (0x0085_e8b0, false),
    (0x0085_9630, false),
    (0x0085_61b0, false),
    (0x0085_6300, false),
    (0x0085_9e20, false),
    (0x0085_9d50, false),
    (0x0085_7cb0, false),
    (0x0085_7200, false),
    (0x0085_68d0, false),
    (0x0085_dcd0, false),
    (0x0085_c950, false),
    (0x0085_97d0, false),
    (0x0085_9550, true),
    (0x0085_a570, false),
    (0x0085_9ba0, false),
    (0x0085_76d0, false),
    (0x0085_ec90, false),
    (0x0085_dac0, false),
    (0x0085_dbb0, false),
    (0x0085_98f0, false),
    (0x0085_d480, false),
    (0x0085_9920, false),
    (0x0085_97a0, false),
    (0x0085_b260, false),
    (0x0085_afd0, false),
    (0x0085_8780, false),
    (0x0085_7470, false),
    (0x0085_bf90, false),
    (0x0085_7fb0, false),
    (0x0085_7c30, false),
    (0x0085_8050, false),
    (0x0085_a410, false),
    (0x0085_a230, false),
    (0x0085_8430, false),
    (0x0085_f860, false),
    (0x0085_ece0, false),
    (0x0085_9fa0, false),
    (0x0085_9990, false),
    (0x0085_de70, false),
    (0x0085_a250, false),
    (0x0085_9800, false),
    (0x0085_9360, false),
    (0x0085_f240, false),
    (0x0085_cd90, false),
    (0x0085_cea0, false),
    (0x0085_f140, false),
    (0x0085_db10, false),
    (0x0085_a1f0, false),
    (0x0085_6980, false),
    (0x0085_94a0, false),
    (0x0085_8a20, false),
    (0x0085_8810, false),
    (0x0085_86b0, false),
    (0x0085_f800, false),
    (0x0085_bf70, false),
    (0x0085_8580, false),
    (0x0085_81a0, false),
    (0x0085_7fc0, false),
    (0x0085_eb80, false),
    (0x0085_9240, false),
    (0x0085_9c40, false),
    (0x0085_fff0, false),
    (0x0085_c6f0, false),
    (0x0085_b780, false),
    (0x0085_d290, false),
    (0x0085_dc90, true),
    (0x0085_a750, false),
    (0x0085_9f10, false),
    (0x0085_a090, false),
    (0x0085_e5e0, false),
    (0x0085_ec00, false),
    (0x0085_ee10, false),
    (0x0085_f660, false),
    (0x0085_a630, false),
    (0x0085_7dd0, false),
    (0x0085_a340, false),
    (0x0085_ebd0, false),
    (0x0085_80a0, false),
    (0x0085_a580, false),
    (0x0085_a430, false),
    (0x0085_b1a0, false),
    (0x0085_b590, false),
    (0x0085_d8f0, false),
    (0x0085_b8c0, false),
    (0x0085_8920, false),
    (0x0085_8fc0, true),
    (0x0085_85d0, false),
    (0x0085_8950, false),
    (0x0085_8800, false),
    (0x0085_9580, false),
    (0x0085_7b70, false),
    (0x0085_7ef0, false),
    (0x0085_8230, false),
    (0x0085_f940, false),
    (0x0085_6950, false),
    (0x0085_b560, true),
    (0x0086_0180, false),
    (0x0085_9ec0, false),
    (0x0085_9120, false),
    (0x0085_9c20, false),
    (0x0086_0170, false),
    (0x0085_fca0, false),
    (0x0085_7440, false),
    (0x0085_9320, false),
    (0x0085_8490, false),
    (0x0085_ebb0, false),
    (0x0085_8e10, false),
    (0x0085_f750, false),
    (0x0085_db70, false),
    (0x0085_7920, false),
    (0x0085_79d0, false),
    (0x0085_9de0, false),
    (0x0085_8e70, false),
    (0x0085_6f20, false),
    (0x0085_b9f0, false),
    (0x0085_a7f0, false),
    (0x0085_a110, false),
    (0x0085_a120, false),
    (0x0085_7ad0, false),
    (0x0085_7d40, false),
    (0x0085_9270, false),
    (0x0085_ffa0, false),
    (0x0086_0200, false),
    (0x0085_7a00, false),
    (0x0085_c400, false),
    (0x0085_c4b0, false),
    (0x0085_be30, false),
    (0x0085_e340, false),
    (0x0086_04c0, false),
    (0x0085_b520, false),
    (0x0085_b4b0, false),
    (0x0085_b170, false),
    (0x0085_f0e0, false),
    (0x0085_82e0, false),
    (0x0085_6270, false),
    (0x0085_a7c0, true),
    (0x0085_5dd0, false),
    (0x0085_5fb0, false),
    (0x0085_e3a0, true),
    (0x0085_b760, false),
    (0x0085_bbb0, false),
    (0x0086_0090, false),
    (0x0085_d4e0, false),
    (0x0085_6dc0, false),
    (0x0085_f430, false),
    (0x0085_ac90, false),
    (0x0085_f8b0, false),
    (0x0085_8610, false),
    (0x0085_8330, false),
    (0x0085_c0a0, false),
    (0x0085_8e40, false),
    (0x0085_8ea0, true),
    (0x0085_7e30, false),
    (0x0085_af60, false),
    (0x0085_af10, false),
    (0x0085_b5d0, false),
    (0x0085_b920, false),
    (0x0085_e570, false),
    (0x0085_b670, false),
    (0x0085_e010, false),
    (0x0085_d460, false),
    (0x0085_d550, false),
    (0x0085_dc10, false),
    (0x0085_8390, false),
    (0x0085_f700, true),
    (0x0085_a840, false),
    (0x0085_e040, false),
    (0x0085_dc50, false),
    (0x0085_eac0, true),
    (0x0085_6250, false),
    (0x0085_9000, false),
    (0x0085_fb00, false),
    (0x0085_6770, false),
    (0x0085_ea40, false),
    (0x0085_90a0, false),
    (0x0085_c100, false),
    (0x0085_71d0, false),
    (0x0085_a770, false),
    (0x0085_fb50, false),
    (0x0085_d3a0, false),
    (0x0085_a190, false),
    (0x0085_f4e0, false),
    (0x0085_e890, false),
    (0x0085_ea30, false),
    (0x0085_ed00, false),
    (0x0085_6570, false),
    (0x0085_72b0, false),
    (0x0085_b650, false),
    (0x0085_88b0, false),
    (0x0085_8710, false),
    (0x0085_e8f0, true),
    (0x0085_e410, false),
    (0x0085_71c0, false),
    (0x0085_bd10, false),
    (0x0085_ee40, false),
    (0x0085_ef80, false),
    (0x0085_9dc0, false),
    (0x0085_9d00, false),
    (0x0086_0460, false),
    (0x0085_be70, false),
    (0x0085_6170, false),
    (0x0085_9950, false),
    (0x0085_7360, false),
    (0x0085_d1d0, false),
    (0x0085_d310, false),
    (0x0086_0730, false),
    (0x0086_0390, false),
    (0x0085_f280, false),
    (0x0085_f0b0, true),
    (0x0085_8070, false),
    (0x0085_d780, false),
    (0x0085_f010, false),
    (0x0085_e270, true),
    (0x0085_e400, false),
    (0x0085_67a0, false),
    (0x0085_fce0, false),
    (0x0085_a870, false),
    (0x0085_a370, false),
    (0x0085_e9e0, false),
    (0x0085_eaf0, false),
    (0x0085_c650, false),
    (0x0085_ee00, false),
    (0x0085_ee70, false),
    (0x0085_9400, false),
    (0x0085_9840, false),
    (0x0085_8b30, false),
    (0x0085_8940, false),
    (0x0085_a010, false),
    (0x0085_c9c0, false),
    (0x0085_b900, false),
    (0x0085_6910, false),
    (0x0085_c600, false),
    (0x0085_c490, false),
    (0x0085_fbb0, false),
    (0x0085_e7c0, false),
    (0x0085_ac50, false),
    (0x0085_9d30, true),
    (0x0085_9e00, true),
    (0x0085_9ef0, true),
    (0x0085_a070, true),
    (0x0085_d3d0, false),
    (0x0085_94c0, false),
    (0x0085_6bc0, false),
    (0x0085_d280, false),
    (0x0085_8ab0, false),
    (0x0085_9ff0, false),
    (0x0085_ec60, false),
    (0x0085_e110, false),
    (0x0085_c130, false),
    (0x0085_fea0, false),
    (0x0085_ad00, false),
    (0x0085_ce30, false),
    (0x0085_cb40, false),
    (0x0085_cfa0, false),
    (0x0085_8d30, true),
    (0x0085_b700, false),
    (0x0085_f370, false),
    (0x0085_ca00, false),
    (0x0085_e920, false),
    (0x0085_f460, true),
    (0x0085_c5b0, false),
    (0x0085_beb0, false),
    (0x0085_b4c0, false),
    (0x0085_af70, false),
    (0x0085_e290, false),
    (0x0086_03e0, false),
    (0x0085_7df0, false),
    (0x0085_7ff0, false),
    (0x0085_fa20, false),
    (0x0085_66c0, true),
    (0x0085_6990, true),
    (0x0085_8e50, false),
    (0x0085_9040, false),
    (0x0085_9250, false),
    (0x0085_9d70, false),
    (0x0085_9df0, false),
    (0x0085_9ee0, false),
    (0x0085_d210, false),
    (0x0085_cf90, false),
    (0x0085_cdb0, false),
    (0x0085_d160, false),
    (0x0085_af50, false),
    (0x0085_9bb0, true),
    (0x0085_5cd0, false),
    (0x0085_7680, false),
    (0x0085_6890, false),
    (0x0085_e130, false),
    (0x0085_e820, false),
    (0x0085_c2d0, false),
    (0x0085_93a0, false),
    (0x0085_95b0, false),
    (0x0085_90f0, false),
    (0x0085_92c0, false),
    (0x0085_95f0, false),
    (0x0085_7c40, false),
    (0x0085_7950, false),
    (0x0085_8740, false),
    (0x0085_d170, false),
    (0x0085_fe30, false),
    (0x0085_7850, false),
    (0x0085_7a80, false),
    (0x0085_ea10, false),
    (0x0085_df80, false),
    (0x0085_c2b0, false),
    (0x0085_d830, false),
    (0x0085_9d80, true),
    (0x0085_edd0, false),
    (0x0085_d8c0, false),
    (0x0085_d7b0, false),
    (0x0085_89d0, false),
    (0x0085_f730, false),
    (0x0085_f850, false),
    (0x0085_f900, false),
    (0x0085_6a50, true),
    (0x0085_6660, false),
    (0x0085_bc50, false),
    (0x0085_8210, false),
    (0x0085_6a80, false),
    (0x0085_e550, true),
    (0x0085_c7b0, true),
    (0x0085_c8f0, true),
    (0x0085_a1a0, true),
    (0x0085_a0b0, true),
    (0x0085_9430, true),
    (0x0085_7980, false),
    (0x0085_bd70, false),
    (0x0085_c320, false),
    (0x0085_c9a0, false),
    (0x0085_e0b0, true),
    (0x0085_ecb0, false),
    (0x0085_5f60, false),
    (0x0085_74f0, true),
    (0x0085_6460, false),
    (0x0085_77c0, true),
    (0x0085_72e0, true),
    (0x0085_7820, false),
    (0x0085_5f80, false),
    (0x0085_8ac0, false),
    (0x0085_c000, false),
    (0x0085_b030, true),
    (0x0085_ab80, true),
    (0x0085_acd0, true),
    (0x0085_af20, true),
    (0x0085_b230, true),
    (0x0085_b480, true),
    (0x0085_f5a0, false),
    (0x0085_7a60, false),
    (0x0085_96d0, false),
    (0x0085_c8c0, false),
    (0x0085_c890, false),
    (0x0086_0290, false),
    (0x0085_c430, false),
    (0x0086_0350, false),
    (0x0085_e620, false),
    (0x0085_8f40, true),
    (0x0085_b820, false),
    (0x00a7_8210, false),
    (0x00a7_8270, false),
    (0x00a7_82a0, false),
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
