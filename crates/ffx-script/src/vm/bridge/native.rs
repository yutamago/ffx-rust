//! Native functions in Rust: the handlers of `g_NativeTables` that only
//! pop their arguments, read or write the worker, the VM's globals and the
//! save data, and call code the game keeps elsewhere (characters, sound,
//! messages; through [`NativeHost`], so the hooked Rust version where one
//! exists). Each takes `(worker, args, stack)` as the original does, `args`
//! being the level's `+0x2c` scratch words, and returns the `EAX` (or
//! `ST0`, as `f64`) the original leaves: the slot decides which
//! (`Script_FinishNative` reads `ST0` for slot 2, `EAX` for slot 3).
//! [`find`] maps a handler's address to its Rust version; the dispatch in
//! [`super::run`] calls it in place of the original.

// w, a, s: the worker, the args words and the stack, as everywhere
#![allow(clippy::many_single_char_names)]

use ffx_bridge::x87;

use super::globals as g;
use super::run::NativeHost;
use super::{
    CONTEXT, atel_get_basic_work, atel_get_char_handle, atel_get_mes_win_work,
    atel_get_pos_work_adrs, atel_pop_stack_float, atel_pop_stack_integer, script_worker_level,
};
use crate::vm::worker;

/// A Rust native: an integer (`EAX`) or float (`ST0`) result.
pub enum Native<H> {
    /// Slots 0, 1 and 3.
    Int(fn(&mut H, u32, u32, u32) -> u32),
    /// Slot 2.
    Float(fn(&mut H, u32, u32, u32) -> f64),
}

fn pop(h: &mut impl NativeHost, s: u32) -> u32 {
    atel_pop_stack_integer(h, s)
}

/// `AtelPopStackFloat` as `fstp m32` leaves it: `f32` bits.
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

fn work(h: &mut impl NativeHost, i: u32) -> u32 {
    atel_get_basic_work(h, i.cast_signed())
}

fn win(h: &mut impl NativeHost, i: u32) -> u32 {
    atel_get_mes_win_work(h, i.cast_signed())
}

/// The level index a native uses: the worker's `+0x32` as it is (no clamp).
fn lvl(h: &impl NativeHost, w: u32) -> u32 {
    u32::from(h.read::<u8>(w + worker::LEVEL))
}

/// `Native_MsgSetStyle` (`0x00858360`, `0x0066`): `(window, style)`, window
/// `+0x20` = style; returns the window.
pub fn native_msg_set_style<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let style = pop(h, s);
    let i = pop(h, s);
    let m = win(h, i);
    h.write(m + 0x20, style.to_le_bytes()[0]);
    i
}

/// `Native_MsgSetFlags` (`0x008584d0`, `0x009d`): window `+0x18`.
pub fn native_msg_set_flags<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let flags = pop(h, s);
    let i = pop(h, s);
    let m = win(h, i);
    h.write(m + 0x18, flags);
    i
}

/// `Native_MsgGetState` (`0x0085fa80`, `0x0097`): window `+0x14`.
pub fn native_msg_get_state<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    let m = win(h, i);
    u32::from(h.read::<u16>(m + 0x14))
}

/// `TOGetMessageStatus` (`0x008b7ca0`).
pub const TO_GET_MESSAGE_STATUS: u32 = 0x008b_7ca0;

/// `Native_MsgIsReady` (`0x0085de10`, `0x008f`): 1 when the window is
/// closed (0), in state 4, or open (2) with the text box done (3).
pub fn native_msg_is_ready<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    let m = win(h, i);
    let ready = match h.read::<u16>(m + 0x14) {
        0 => true,
        2 => call(h, TO_GET_MESSAGE_STATUS, &[i]) == 3,
        st => st == 4,
    };
    u32::from(ready)
}

/// One character call `f(chr, arg)` when the worker has a character;
/// returns 0.
fn chr_set<H: NativeHost>(h: &mut H, w: u32, s: u32, f: u32) -> u32 {
    let c = atel_get_char_handle(h, w);
    let v = pop(h, s);
    if c != 0 {
        call(h, f, &[c, v]);
    }
    0
}

/// As [`chr_set`], returning the call's `EAX` (0 without a character).
fn chr_get_set<H: NativeHost>(h: &mut H, w: u32, s: u32, f: u32) -> u32 {
    let c = atel_get_char_handle(h, w);
    let v = pop(h, s);
    if c == 0 {
        return 0;
    }
    call(h, f, &[c, v])
}

macro_rules! chr_natives {
    ($($(#[$m:meta])* $name:ident => $f:expr, $how:ident;)*) => {$(
        $(#[$m])*
        pub fn $name<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
            $how(h, w, s, $f)
        }
    )*};
}

chr_natives! {
    /// `Native_ChrSetMotionBlend` (`0x00a79f40`, `0x5034`): `Ch_MotionSetHokan`.
    native_chr_set_motion_blend => 0x0082_afb0, chr_set;
    /// `Native_ChrSetNextMotionBlend` (`0x00a78d70`, `0x505c`): `Ch_MotionSetNextHokan`.
    native_chr_set_next_motion_blend => 0x0082_b010, chr_set;
    /// `Native_ChrSetBodyHit` (`0x00a78da0`, `0x500f`): `Ch_DisableBodyHit`.
    native_chr_set_body_hit => 0x0082_a950, chr_set;
    /// `Native_ChrSetGroundFollow` (`0x00a78cb0`, `0x500e`): `Ch_DisableID`.
    native_chr_set_ground_follow => 0x0082_a980, chr_set;
    /// `Native_ChrSetGroundMode` (`0x00a79130`, `0x5017`): `Ch_SetGravMode`.
    native_chr_set_ground_mode => 0x0082_b240, chr_set;
    /// `Native_ChrSetMotionSpeed` (`0x00a79510`, `0x5021`): `Ch_SetMotionSpeed`.
    native_chr_set_motion_speed => 0x0082_b380, chr_set;
    /// `Native_ChrSetDistanceCull` (`0x00a79af0`, `0x502a`): `Ch_EnClip`'s result.
    native_chr_set_distance_cull => 0x0082_aa30, chr_get_set;
    /// `Native_ChrSetShadowMode` (`0x00a79a40`, `0x5029`): `Ch_SetShadowMode`'s result.
    native_chr_set_shadow_mode => 0x0082_b820, chr_get_set;
}

/// `Native_ChrSetAutoMotion` (`0x00a79670`, `0x5025`): `(slot, motion)`,
/// `Ch_MotionSetMoves(chr, slot, motion)`.
pub fn native_chr_set_auto_motion<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let c = atel_get_char_handle(h, w);
    let motion = pop(h, s);
    let slot = pop(h, s);
    if c != 0 {
        call(h, 0x0082_afe0, &[c, slot, motion]);
    }
    0
}

/// One call `f(chr)` when the worker has a character; returns 0.
fn chr_do<H: NativeHost>(h: &mut H, w: u32, f: u32) -> u32 {
    let c = atel_get_char_handle(h, w);
    if c != 0 {
        call(h, f, &[c]);
    }
    0
}

/// `Native_ChrReleaseMotion` (`0x00a78940`, `0x5005`): `Ch_ResetMotion`.
pub fn native_chr_release_motion<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    chr_do(h, w, 0x0082_b080)
}

/// `Native_ChrReleaseMotion2` (`0x00a794b0`, `0x5020`): the same.
pub fn native_chr_release_motion2<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    chr_do(h, w, 0x0082_b080)
}

/// `f(chr)`'s `EAX`, or 0 without a character.
fn chr_get<H: NativeHost>(h: &mut H, c: u32, f: u32) -> u32 {
    if c == 0 {
        return 0;
    }
    call(h, f, &[c])
}

/// `Native_ChrGetMotionFrame` (`0x00a78750`, `0x5051`): `Ch_MotionGetFrame`.
pub fn native_chr_get_motion_frame<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let c = atel_get_char_handle(h, w);
    chr_get(h, c, 0x0083_9860)
}

/// `Native_ChrGetMotion` (`0x00a78dd0`, `0x505d`): `Ch_MotionGetId`.
pub fn native_chr_get_motion<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let c = atel_get_char_handle(h, w);
    chr_get(h, c, 0x0083_71a0)
}

/// `Native_ChrGetMotionFrameOf` (`0x00a79dd0`, `0x5075`): `(worker)`,
/// that worker's `Ch_MotionGetFrame`.
pub fn native_chr_get_motion_frame_of<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    let o = work(h, i);
    let c = atel_get_char_handle(h, o);
    chr_get(h, c, 0x0083_9860)
}

/// `CT_507E_ret` (`0x00a7a210`, `0x507e`): `(worker, on)`,
/// `Ch_MouthSetAutoTalk` on that worker's character.
pub fn ct_507e_ret<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let on = pop(h, s);
    let i = pop(h, s);
    let o = work(h, i);
    let c = atel_get_char_handle(h, o);
    if c != 0 {
        call(h, 0x0083_4770, &[c, on]);
    }
    0
}

/// `Native_ChrSetBoneUpdate` (`0x00a78af0`, `0x5057`): `(worker, on)`,
/// `Ch_SetCalcNorm`'s result on that worker's character (its handle, 0,
/// without one).
pub fn native_chr_set_bone_update<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let on = pop(h, s);
    let i = pop(h, s);
    let o = work(h, i);
    let c = atel_get_char_handle(h, o);
    if c == 0 {
        return 0;
    }
    call(h, 0x0083_5750, &[c, on])
}

/// `Native_ChrFadeAlpha` (`0x00a79ea0`, `0x5033`): `(alpha f32, frames)`,
/// `Ch_SetTransparent(chr, alpha, frames)`.
pub fn native_chr_fade_alpha<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let c = atel_get_char_handle(h, w);
    let frames = pop(h, s);
    let alpha = popf(h, s);
    if c != 0 {
        call(h, 0x0082_b900, &[c, alpha, frames]);
    }
    0
}

/// `Native_ChrSetOffset` (`0x00a790b0`, `0x5016`): `(x, y, z)` floats,
/// `Ch_SetOffset(chr, z, y, x)` (popped order).
pub fn native_chr_set_offset<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let c = atel_get_char_handle(h, w);
    let z = popf(h, s);
    let y = popf(h, s);
    let x = popf(h, s);
    if c != 0 {
        call(h, 0x0082_b3c0, &[c, x, y, z]);
    }
    0
}

/// `Native_ChrSetPushWeight` (`0x00a78b30`, `0x5009`, float): `Ch_SetWeight`'s
/// result, 0.0 without a character.
pub fn native_chr_set_push_weight<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let c = atel_get_char_handle(h, w);
    let v = popf(h, s);
    if c == 0 {
        return 0.0;
    }
    callf(h, 0x0082_b940, &[c, v])
}

/// `Native_ChrWaitMotion_5003` (`0x00a787d0`, `0x5003` start): the
/// character handle.
pub fn native_chr_wait_motion_5003<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    atel_get_char_handle(h, w)
}

/// `Native_ChrWaitMotion_Poll_5003` (`0x00a788a0`): done (5) once the
/// character's motion flag (`+0x736`) is clear; 0 without a character.
pub fn native_chr_wait_motion_poll_5003<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let c = atel_get_char_handle(h, w);
    if c == 0 {
        return 0;
    }
    if h.read::<u8>(c + 0x736) == 0 { 5 } else { 0 }
}

/// `Native_ChrWaitMotion_501e` (`0x00a79370`, start): nothing (`EAX` as
/// it was; the dispatch ignores it).
pub fn native_chr_wait_motion_501e<H: NativeHost>(_h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    0
}

/// `g_DebugMode` (`0x012fb80c`), 1 in the debug build's modes.
pub const DEBUG_MODE: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x012f_b80c);
/// `g_ChrWaitMotionFrames` (`0x022fb4d8`): frames waited, counted in debug mode.
pub const CHR_WAIT_MOTION_FRAMES: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x022f_b4d8);

/// `Native_ChrWaitMotion_Poll_501e` (`0x00a793c0`): as the `5003` poll,
/// counting frames in debug mode (reset when done outside it).
pub fn native_chr_wait_motion_poll_501e<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let c = atel_get_char_handle(h, w);
    if c == 0 {
        return 0;
    }
    let mode = DEBUG_MODE.get(h);
    let mut n = CHR_WAIT_MOTION_FRAMES.get(h);
    if mode == 1 {
        n = n.wrapping_add(1);
        CHR_WAIT_MOTION_FRAMES.set(h, n);
    }
    if h.read::<u8>(c + 0x736) != 0 {
        return 0;
    }
    CHR_WAIT_MOTION_FRAMES.set(h, if mode == 1 { 0 } else { n });
    5
}

/// `Native_GetWorkerId` (`0x0085ba50`, `0x0033`): `(worker, -1 self)` ->
/// its index.
pub fn native_get_worker_id<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    let o = if i.cast_signed() < 0 { w } else { work(h, i) };
    u32::from(h.read::<u16>(o + worker::ID))
}

fn worker_pos<H: NativeHost>(h: &mut H, s: u32, off: u32) -> f64 {
    let i = pop(h, s);
    let o = work(h, i);
    let p = atel_get_pos_work_adrs(h, o);
    x87::load(h.read(p + off))
}

/// `Native_GetWorkerX` (`0x0085bb80`, `0x0038`, float).
pub fn native_get_worker_x<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    worker_pos(h, s, 0)
}

/// `Native_GetWorkerY` (`0x0085bd40`, `0x0039`, float).
pub fn native_get_worker_y<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    worker_pos(h, s, 4)
}

/// `Native_GetWorkerZ` (`0x0085bf10`, `0x003a`, float).
pub fn native_get_worker_z<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    worker_pos(h, s, 8)
}

/// `Math_WrapAngle` (`0x0088bc20`).
pub const MATH_WRAP_ANGLE: u32 = 0x0088_bc20;

/// `Native_TurnSetYaw`/`Native_TurnSetPitch`: the turn target at `+0x884`
/// (`+0x888`) of the worker's turn block for its level, the remaining turn
/// at `+0x890` (`+0x894`) from the position's `+0x34` (`+0x30`).
fn turn_set<H: NativeHost>(h: &mut H, w: u32, s: u32, which: u32, pos_off: u32) -> f64 {
    let p = atel_get_pos_work_adrs(h, w);
    let v = popf(h, s);
    let t = x87::store(callf(h, MATH_WRAP_ANGLE, &[v]));
    let i = lvl(h, w) * 0x44;
    h.write(w + i + 0x884 + which, t);
    let d = x87::Reg::load(f32::from_bits(t))
        .subr_m32(f32::from_bits(h.read(p + pos_off)))
        .store();
    let r = x87::store(callf(h, MATH_WRAP_ANGLE, &[d.to_bits()]));
    h.write(w + i + 0x890 + which, r);
    x87::load(t)
}

/// `Native_TurnSetYaw` (`0x00859640`, `0x0028`, float): returns the
/// wrapped yaw.
pub fn native_turn_set_yaw<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    turn_set(h, w, s, 0, 0x34)
}

/// `Native_TurnSetPitch` (`0x00859880`, `0x0029`, float).
pub fn native_turn_set_pitch<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    turn_set(h, w, s, 4, 0x30)
}

/// `(elapsed, frames)` into `+8`/`+0xc` of a move or turn block (frames
/// not above 0, or NaN, become 1.0); returns `elapsed`.
fn set_frames<H: NativeHost>(h: &mut H, s: u32, at: u32) -> f64 {
    let frames = popf(h, s);
    let elapsed = popf(h, s);
    let f = f64::from(f32::from_bits(frames));
    let v = if f > 0.0 { frames } else { 1f32.to_bits() };
    h.write(at + 0xc, v);
    h.write(at + 8, elapsed);
    x87::load(elapsed)
}

/// `Native_TurnSetFrames` (`0x008560d0`, `0x001d`, float): the turn block.
pub fn native_turn_set_frames<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let l = script_worker_level(h, w, lvl(h, w).cast_signed());
    let t = h.read::<u32>(l + 0x48);
    set_frames(h, s, t)
}

/// `Native_MoveSetFrames` (`0x00855f10`, `0x001c`, float): `(elapsed,
/// frames)` into `+0xc`/`+8` of the move block at worker `+0x5b8 + level *
/// 0x4c` (the level unclamped, the frames as they are); returns `elapsed`.
pub fn native_move_set_frames<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    let at = w + 0x5b8 + lvl(h, w) * 0x4c;
    let frames = popf(h, s);
    let elapsed = popf(h, s);
    h.write(at + 0xc, frames);
    h.write(at + 8, elapsed);
    x87::load(elapsed)
}

/// `Native_TurnGetField38` (`0x0085c590`, `0x003f`, float): the turn
/// block's `+0x38`.
pub fn native_turn_get_field38<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> f64 {
    let l = script_worker_level(h, w, lvl(h, w).cast_signed());
    let t = h.read::<u32>(l + 0x48);
    x87::load(h.read(t + 0x38))
}

/// `Native_MoveWait` (`0x008579a0`, `0x001a` poll): done (5) when the move
/// block's mode (`+2 & 0x7f`) is 0.
pub fn native_move_wait<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let m = h.read::<u16>(w + lvl(h, w) * 0x4c + 0x5ba);
    if m.trailing_zeros() >= 7 { 5 } else { 0 }
}

/// `Native_TurnWait` (`0x00857b30`, `0x001b` poll): the turn block's mode
/// (`+2 & 0x1f`).
pub fn native_turn_wait<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let l = script_worker_level(h, w, lvl(h, w).cast_signed());
    let t = h.read::<u32>(l + 0x48);
    if h.read::<u16>(t + 2).trailing_zeros() >= 5 {
        5
    } else {
        0
    }
}

/// `Native_GetTurnKind` (`0x0085a670`, `0x00c6`): bit 0 the remaining yaw
/// is positive, bit 1 its size is at least the half-turn angle (degrees
/// to radians with `f32` pi, in doubles).
pub fn native_get_turn_kind<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    let f = x87::load(h.read(w + lvl(h, w) * 0x44 + 0x890));
    let mut r = u32::from(f > 0.0);
    let f = if f < 0.0 { -f } else { f };
    let pi = h.read::<f64>(h.rebase(0x00b9_2288));
    let deg = h.read::<f64>(h.rebase(0x00b4_8378));
    let t = x87::load(g::atel_get_half_turn_angle(h)) * pi / deg;
    if t <= f {
        r |= 2;
    }
    r
}

/// `Native_GetTurnMotion` (`0x0085abb0`, `0x00c7`): `(kind)` -> worker
/// `+0xae0[kind]`.
pub fn native_get_turn_motion<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let k = pop(h, s);
    h.read(w.wrapping_add(k.wrapping_mul(4)).wrapping_add(0xae0))
}

/// `Native_SetTurnMotions` (`0x0085a4e0`, `0x00c5`): four motions into
/// worker `+0xae0` in push order.
pub fn native_set_turn_motions<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let m3 = pop(h, s);
    let m2 = pop(h, s);
    let m1 = pop(h, s);
    let m0 = pop(h, s);
    h.write(w + 0xae0, m0);
    h.write(w + 0xae8, m2);
    h.write(w + 0xae4, m1);
    h.write(w + 0xaec, m3);
    0
}

fn trigger_bit(k: u32) -> u8 {
    // shl dl, cl: counts 8-31 shift the byte out
    let n = k & 31;
    if n < 8 { 1 << n } else { 0 }
}

/// `Native_EnableTrigger` (`0x0085c060`, `0x0034`): worker `+0xab` |= bit
/// `kind`; returns `kind`.
pub fn native_enable_trigger<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let k = pop(h, s);
    let b = h.read::<u8>(w + 0xab) | trigger_bit(k);
    h.write(w + 0xab, b);
    k
}

/// `Native_DisableTrigger` (`0x0085c280`, `0x0035`): clears it; 0.
pub fn native_disable_trigger<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let k = pop(h, s);
    let b = h.read::<u8>(w + 0xab) & !trigger_bit(k);
    h.write(w + 0xab, b);
    0
}

/// `Native_TriggerSetActive` (`0x0085fbf0`, `0x0056`) and
/// `Native_TriggerSetActive2` (`0x008600c0`, `0x005c`): `(on)`, for types
/// 2, 3, 5 and 6: worker `+0x34` bit 7 = on, `+0x36` |= 1; returns 1 (0 for
/// other types).
pub fn native_trigger_set_active<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let on = u16::from(pop(h, s) != 0);
    let k = h.read::<u8>(h.read::<u32>(w + worker::HEADER));
    if !matches!(k, 2 | 3 | 5 | 6) {
        return 0;
    }
    let f = h.read::<u16>(w + worker::FLAGS) & !0x80 | on << 7;
    h.write(w + worker::FLAGS, f);
    let f2 = h.read::<u16>(w + worker::FLAGS2) | 1;
    h.write(w + worker::FLAGS2, f2);
    1
}

/// `Native_WorkerSetFlag0200` (`0x0085b9b0`, `0x0116`): `(worker)`, its
/// `+0x36`: `0x400` cleared, `0x200` set; returns the index.
pub fn native_worker_set_flag0200<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    let o = work(h, i);
    let f = h.read::<u16>(o + worker::FLAGS2) & 0xfbff | 0x200;
    h.write(o + worker::FLAGS2, f);
    i
}

/// `Native_MinimapSetMarker` (`0x008583b0`, `0x0148`): `(marker, worker)`
/// (the worker on top), that worker's `+0xbc` = marker (bit 31 set when not 0) and `+0xb4` =
/// the default marker scale (`0x00b41db0`); returns the stored value.
pub fn native_minimap_set_marker<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let i = pop(h, s);
    let m = pop(h, s);
    let o = work(h, i);
    let v = if m == 0 { 0 } else { m | 0x8000_0000 };
    h.write(o + 0xbc, v);
    let scale = crate::vm::through_x87(h.read(h.rebase(0x00b4_1db0)));
    h.write(o + 0xb4, scale);
    v
}

/// `Native_Random` (`0x00857400`, `0x00a6`): `(n)` -> `brnd(2)`'s low 16
/// bits scaled to `0..n` (`n` at least 1).
pub fn native_random<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let n = pop(h, s);
    let n = if n.cast_signed() > 0 { n } else { 1 };
    let r = call(h, 0x0079_88f0, &[2]);
    (r & 0xffff).wrapping_mul(n) >> 16
}

/// `Native_PadPressed` (`0x0085d9b0`, `0x0046`): the pressed buttons
/// through `Pad_MapButtons(0, 0, pressed)`.
pub fn native_pad_pressed<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    let b = u32::from(g::atel_get_controler(h));
    call(h, 0x0088_8c40, &[0, 0, b]) & 0xffff
}

/// `Native_VoiceStop` (`0x0085e090`, `0x00d7`): `SndStopStream` unless
/// `0x012fb8a0` is 1.
pub fn native_voice_stop<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    if h.read::<u32>(h.rebase(0x012f_b8a0)) != 1 {
        call(h, 0x0088_7780, &[]);
    }
    0
}

/// `Native_AlwaysTrue019c` (`0x00856630`, `0x019c`).
pub fn native_always_true019c<H: NativeHost>(_h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    1
}

/// `Native_WaitForever` (`0x0085c420`, `0x005f` poll): never done.
pub fn native_wait_forever<H: NativeHost>(_h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    0
}

/// `Native_ControlEnable` (`0x008566b0`, `0x005d`): `AtelUcOn`; 1.
pub fn native_control_enable<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call(h, 0x0087_1ab0, &[]);
    1
}

/// `Native_ControlDisable` (`0x008567c0`, `0x005e`): `AtelUcOff`; 0.
pub fn native_control_disable<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call(h, 0x0087_19e0, &[]);
    0
}

/// `Native_ChrRefreshAllBones` (`0x00a7a6a0`, `0x5087`):
/// `Ch_AllReserveCalcNorm`; 0.
pub fn native_chr_refresh_all_bones<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call(h, 0x0083_2550, &[]);
    0
}

/// `Native_WaitCdIdle` (`0x00a795d0`, `0x5068` poll): 1 once
/// `Ch_ReadSync(1)` is 0.
pub fn native_wait_cd_idle<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    u32::from(call(h, 0x0082_a5e0, &[1]) == 0)
}

/// `Native_WaitFrames_01d1` (`0x0085fd20`, start): `(frames)` into the
/// args word; returns it.
pub fn native_wait_frames_01d1<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    let n = pop(h, s);
    h.write(a, n);
    n
}

/// `Native_WaitFrames_Poll_01d1` (`0x0085ff70`): done (5) at once when
/// `0x00b5ec4f` is clear or `0x0133c913` set, else counts the args word
/// down: 1 at 0 and below.
pub fn native_wait_frames_poll_01d1<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    if h.read::<u8>(h.rebase(0x00b5_ec4f)) == 0 || h.read::<u8>(h.rebase(0x0133_c913)) != 0 {
        return 5;
    }
    let n = h.read::<u32>(a).wrapping_sub(1);
    h.write(a, n);
    u32::from(n.cast_signed() <= 0)
}

/// `Native_SetMapFadeFrames` (`0x008596b0`, `0x00bb`): returns `n`.
pub fn native_set_map_fade_frames<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let n = pop(h, s);
    g::atel_set_map_fade_frames(h, n.to_le_bytes()[0]);
    n
}

/// `Native_SetMapFadeType` (`0x00859590`, `0x00bc`): returns `t`.
pub fn native_set_map_fade_type<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let t = pop(h, s);
    g::atel_set_map_fade_type(h, t.to_le_bytes()[0]);
    t
}

/// `MsGetSavePlayerPtr(chr)` (`0x00785330`).
pub const MS_GET_SAVE_PLAYER_PTR: u32 = 0x0078_5330;

/// `Native_CharRestoreHp` (`0x0085c680`, `0x0171`): `(chr)`, HP (`+0x1c`)
/// up to the maximum (`+0x24`), `+0x3d` cleared; returns the HP when it was
/// above the maximum, else the address.
pub fn native_char_restore_hp<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let c = pop(h, s);
    let p = call(h, MS_GET_SAVE_PLAYER_PTR, &[c]);
    let hp = h.read::<u32>(p + 0x1c);
    let max = h.read::<u32>(p + 0x24);
    h.write(p + 0x3d, 0u8);
    if hp > max {
        return hp;
    }
    h.write(p + 0x1c, max);
    max
}

/// `Native_CharRestoreMp` (`0x0085c860`, `0x0172`): MP (`+0x20`) up to
/// `+0x28`; returns the MP when above, else the HP (`+0x1c`).
pub fn native_char_restore_mp<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let c = pop(h, s);
    let p = call(h, MS_GET_SAVE_PLAYER_PTR, &[c]);
    let mp = h.read::<u32>(p + 0x20);
    let max = h.read::<u32>(p + 0x28);
    if mp > max {
        return mp;
    }
    h.write(p + 0x20, max);
    h.read(p + 0x1c)
}

/// `CT_0241_ret` (`0x00856ad0`, `0x0241`): `(on)`, bit 3 of the current
/// ctrl work's `+0x210`; returns `on != 0`.
pub fn ct_0241_ret<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let on = u32::from(pop(h, s) != 0);
    let c = CONTEXT.get(h);
    let f = h.read::<u32>(c + 0x210);
    h.write(c + 0x210, f ^ (((on * 8) ^ f) & 8));
    on
}

/// `Native_FpRun` (`0x0091c670`, `0x8002`): `(id, restart)`: `Fp_Restart(id)`
/// or `Fp_End(id)`; returns `restart`.
pub fn native_fp_run<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let restart = pop(h, s);
    let id = pop(h, s);
    call(
        h,
        if restart != 0 {
            0x0072_9700
        } else {
            0x0072_9660
        },
        &[id],
    );
    restart
}

/// `Native_FpShow` (`0x0091c740`, `0x800d`): `(id, on)`:
/// `pppFpIDdispOn`/`Off(id)`; returns `on`.
pub fn native_fp_show<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let on = pop(h, s);
    let id = pop(h, s);
    call(h, if on != 0 { 0x0072_95b0 } else { 0x0072_9570 }, &[id]);
    on
}

/// `Native_FpRunMode2` (`0x0091c7a0`, `0x8003`): `Fp_RestartMode2(id)`; 1.
pub fn native_fp_run_mode2<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let id = pop(h, s);
    call(h, 0x0072_9810, &[id]);
    1
}

/// `Native_FieldSetMaskBit` (`0x0091c530`, `0x8000`): `(group, on)`,
/// `TkSetMapGroupDrawStatus(group, on)` (group 14 on map `0x188` always
/// on); returns `group`.
pub fn native_field_set_mask_bit<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let on = pop(h, s);
    let grp = pop(h, s);
    let map = h.read::<u32>(h.rebase(0x0133_0844));
    let on = if map == 0x188 && grp == 0xe { 1 } else { on };
    call(h, 0x0088_eba0, &[grp, on]);
    grp
}

/// The Rust version of the native handler at `sym` (an `ffx.sym` address).
#[allow(clippy::too_many_lines)] // a table
#[must_use]
pub fn find<H: NativeHost>(sym: u32) -> Option<Native<H>> {
    use Native::{Float, Int};
    Some(match sym {
        0x0085_8360 => Int(native_msg_set_style),
        0x0085_84d0 => Int(native_msg_set_flags),
        0x0085_fa80 => Int(native_msg_get_state),
        0x0085_de10 => Int(native_msg_is_ready),
        0x00a7_9f40 => Int(native_chr_set_motion_blend),
        0x00a7_8d70 => Int(native_chr_set_next_motion_blend),
        0x00a7_8da0 => Int(native_chr_set_body_hit),
        0x00a7_8cb0 => Int(native_chr_set_ground_follow),
        0x00a7_9130 => Int(native_chr_set_ground_mode),
        0x00a7_9510 => Int(native_chr_set_motion_speed),
        0x00a7_9af0 => Int(native_chr_set_distance_cull),
        0x00a7_9a40 => Int(native_chr_set_shadow_mode),
        0x00a7_9670 => Int(native_chr_set_auto_motion),
        0x00a7_8940 => Int(native_chr_release_motion),
        0x00a7_94b0 => Int(native_chr_release_motion2),
        0x00a7_8750 => Int(native_chr_get_motion_frame),
        0x00a7_8dd0 => Int(native_chr_get_motion),
        0x00a7_9dd0 => Int(native_chr_get_motion_frame_of),
        0x00a7_a210 => Int(ct_507e_ret),
        0x00a7_8af0 => Int(native_chr_set_bone_update),
        0x00a7_9ea0 => Int(native_chr_fade_alpha),
        0x00a7_90b0 => Int(native_chr_set_offset),
        0x00a7_8b30 => Float(native_chr_set_push_weight),
        0x00a7_87d0 => Int(native_chr_wait_motion_5003),
        0x00a7_88a0 => Int(native_chr_wait_motion_poll_5003),
        0x00a7_9370 => Int(native_chr_wait_motion_501e),
        0x00a7_93c0 => Int(native_chr_wait_motion_poll_501e),
        0x0085_ba50 => Int(native_get_worker_id),
        0x0085_bb80 => Float(native_get_worker_x),
        0x0085_bd40 => Float(native_get_worker_y),
        0x0085_bf10 => Float(native_get_worker_z),
        0x0085_9640 => Float(native_turn_set_yaw),
        0x0085_9880 => Float(native_turn_set_pitch),
        0x0085_60d0 => Float(native_turn_set_frames),
        0x0085_5f10 => Float(native_move_set_frames),
        0x0085_c590 => Float(native_turn_get_field38),
        0x0085_79a0 => Int(native_move_wait),
        0x0085_7b30 => Int(native_turn_wait),
        0x0085_a670 => Int(native_get_turn_kind),
        0x0085_abb0 => Int(native_get_turn_motion),
        0x0085_a4e0 => Int(native_set_turn_motions),
        0x0085_c060 => Int(native_enable_trigger),
        0x0085_c280 => Int(native_disable_trigger),
        0x0085_fbf0 | 0x0086_00c0 => Int(native_trigger_set_active),
        0x0085_b9b0 => Int(native_worker_set_flag0200),
        0x0085_83b0 => Int(native_minimap_set_marker),
        0x0085_7400 => Int(native_random),
        0x0085_d9b0 => Int(native_pad_pressed),
        0x0085_e090 => Int(native_voice_stop),
        0x0085_6630 => Int(native_always_true019c),
        0x0085_c420 => Int(native_wait_forever),
        0x0085_66b0 => Int(native_control_enable),
        0x0085_67c0 => Int(native_control_disable),
        0x00a7_a6a0 => Int(native_chr_refresh_all_bones),
        0x00a7_95d0 => Int(native_wait_cd_idle),
        0x0085_fd20 => Int(native_wait_frames_01d1),
        0x0085_ff70 => Int(native_wait_frames_poll_01d1),
        0x0085_96b0 => Int(native_set_map_fade_frames),
        0x0085_9590 => Int(native_set_map_fade_type),
        0x0085_c680 => Int(native_char_restore_hp),
        0x0085_c860 => Int(native_char_restore_mp),
        0x0085_6ad0 => Int(ct_0241_ret),
        0x0091_c670 => Int(native_fp_run),
        0x0091_c740 => Int(native_fp_show),
        0x0091_c7a0 => Int(native_fp_run_mode2),
        0x0091_c530 => Int(native_field_set_mask_bit),
        _ => return None,
    })
}

/// Every handler [`find`] has, with whether it returns a float.
pub const PORTED: [(u32, bool); 67] = [
    (0x0085_8360, false),
    (0x0085_84d0, false),
    (0x0085_fa80, false),
    (0x0085_de10, false),
    (0x00a7_9f40, false),
    (0x00a7_8d70, false),
    (0x00a7_8da0, false),
    (0x00a7_8cb0, false),
    (0x00a7_9130, false),
    (0x00a7_9510, false),
    (0x00a7_9af0, false),
    (0x00a7_9a40, false),
    (0x00a7_9670, false),
    (0x00a7_8940, false),
    (0x00a7_94b0, false),
    (0x00a7_8750, false),
    (0x00a7_8dd0, false),
    (0x00a7_9dd0, false),
    (0x00a7_a210, false),
    (0x00a7_8af0, false),
    (0x00a7_9ea0, false),
    (0x00a7_90b0, false),
    (0x00a7_8b30, true),
    (0x00a7_87d0, false),
    (0x00a7_88a0, false),
    (0x00a7_9370, false),
    (0x00a7_93c0, false),
    (0x0085_ba50, false),
    (0x0085_bb80, true),
    (0x0085_bd40, true),
    (0x0085_bf10, true),
    (0x0085_9640, true),
    (0x0085_9880, true),
    (0x0085_60d0, true),
    (0x0085_5f10, true),
    (0x0085_c590, true),
    (0x0085_79a0, false),
    (0x0085_7b30, false),
    (0x0085_a670, false),
    (0x0085_abb0, false),
    (0x0085_a4e0, false),
    (0x0085_c060, false),
    (0x0085_c280, false),
    (0x0085_fbf0, false),
    (0x0086_00c0, false),
    (0x0085_b9b0, false),
    (0x0085_83b0, false),
    (0x0085_7400, false),
    (0x0085_d9b0, false),
    (0x0085_e090, false),
    (0x0085_6630, false),
    (0x0085_c420, false),
    (0x0085_66b0, false),
    (0x0085_67c0, false),
    (0x00a7_a6a0, false),
    (0x00a7_95d0, false),
    (0x0085_fd20, false),
    (0x0085_ff70, false),
    (0x0085_96b0, false),
    (0x0085_9590, false),
    (0x0085_c680, false),
    (0x0085_c860, false),
    (0x0085_6ad0, false),
    (0x0091_c670, false),
    (0x0091_c740, false),
    (0x0091_c7a0, false),
    (0x0091_c530, false),
];
