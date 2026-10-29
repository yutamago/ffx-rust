//! The motion script of the `magic_0018` family (118 DLLs), ported by the
//! shared-3 list: one step per frame over the DLL's script of `(op,
//! arg)` word pairs, driving the effect's character through the motion
//! slots. Addresses are `magic_0018`'s.

use crate::runtime::slot_target;
use crate::sb_common::{SET_SPEED, set_speed};
use crate::{Callee, Game, Image, Sig};

/// The DLL's globals, read from the operands of [`motion_step`].
struct Globals {
    /// The script's result word (1 when it ended).
    done: u32,
    /// `u16` frames left to wait (the next `u16` counts the steps).
    wait: u32,
    /// `u16` the motion kind (0 effect motion, 1 or 2 system motion).
    mode: u32,
    /// The effect's character (the `Ms*` slots' first argument).
    chr: u32,
    /// Its motion object (the `Ch_*` slots' first argument).
    obj: u32,
    /// The script position (an index of 8-byte entries).
    pos: u32,
    /// The script's base.
    script: u32,
    /// The actor index for the speed and cloth commands (-1 = the
    /// effect's own).
    index: u32,
}

/// Slot `slot` through the table global named by the operand at `off`.
fn slot<G: Game>(g: &G, img: &Image, f: u32, off: u32, slot: u32) -> u32 {
    slot_target(g, img.word(f + off), slot)
}

/// One step of the motion script. Returns the result word (1 at once
/// when it is 1 already).
///
/// - A wait (`u16`) counts down first; a step runs when it reaches 0 (or
///   was 0: the step counter goes up and the current motion must have
///   ended: `MsEffectCheckMotion(chr)` non-zero in mode 0, else
///   `Ch_GetWaitMotion(obj)` zero).
/// - The position moves to the next entry; while its `op` is -1..-5 it is
///   a command: -2 the actor's motion speed ([`set_speed`] with `arg`),
///   -3 `Ch_MotionSetNextHokan(obj, 0)`, -4 `Ch_ClothReset2` of the
///   actor's character (the effect's own for index -1, else the DLL's
///   character table entry of the actor list's byte) — each followed by
///   the next entry — and -5 (slot 529 with the DLL's word) or -1 end the
///   commands with a wait of 1.
/// - Otherwise `op` is a motion: mode 0 `MsEffectSetMotion(chr, op)`,
///   1 `Ch_ResetMotion(obj)`, slot 36 `(obj, op)`, 2 `Ch_ResetMotion`,
///   `Ch_SetSysMotion(obj, 1, op)`, then `Ch_GetSeqFrame(obj)`: the wait.
///   Any other mode: a wait of 1.
/// - Then the entry's `arg`: -1, -3, -5, -6 end the script (mode 0 first
///   `MsEffectEndMotion(chr, 0 / 1 / 3 / 2)`), -4 ends it at once, -2
///   clears the wait and the counter, -7 stays on the entry (the wait
///   the motion's length, `MsEffectSetMotion(chr, 0x10)`'s in mode 0
///   when that was not positive, else 1); any other `arg` is the wait.
///
/// Each slot call passes the words the original leaves on the stack.
///
/// `magic_0018` 0x10002c40 (732 bytes and two jump tables, no arguments;
/// 118 DLLs).
#[allow(clippy::too_many_lines)]
pub fn motion_step<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let v = Globals {
        done: w(0x02),
        wait: w(0x11),
        mode: w(0x24),
        chr: w(0x32),
        obj: w(0x45),
        pos: w(0x75),
        script: w(0x7a),
        index: w(0xa3),
    };
    if g.read::<u32>(v.done) == 1 {
        return 1;
    }
    let wait = g.read::<u16>(v.wait);
    if wait == 0 {
        let n = g.read::<u16>(v.wait + 2).wrapping_add(1);
        g.write::<u16>(v.wait + 2, n);
        let chr = g.read::<u32>(v.chr);
        let obj = g.read::<u32>(v.obj);
        let ended = if g.read::<u16>(v.mode) == 0 {
            // MsEffectCheckMotion
            g.call(slot(g, img, f, 0x2a, 411), 0, &[chr]).eax != 0
        } else {
            // Ch_GetWaitMotion
            g.call(slot(g, img, f, 0x2a, 20), 0, &[obj]).eax == 0
        };
        if !ended {
            return g.read::<u32>(v.done);
        }
    } else {
        let wait = wait.wrapping_sub(1);
        g.write::<u16>(v.wait, wait);
        if wait != 0 {
            return g.read::<u32>(v.done);
        }
    }
    // the commands, then the entry they stop at
    let (entry, mut next_wait) = loop {
        let pos = g.read::<u32>(v.pos).wrapping_add(1);
        g.write::<u32>(v.pos, pos);
        let entry = g.read::<u32>(v.script).wrapping_add(pos.wrapping_mul(8));
        let op = g.read::<u32>(entry);
        match op.wrapping_add(5) {
            // -2: the actor's speed
            3 => {
                let arg = g.read::<u32>(entry + 4);
                let index = g.read::<u32>(v.index);
                set_speed(g, img, img.rel32_target(f + 0xa8), index, arg);
            }
            // -3: Ch_MotionSetNextHokan(obj, 0)
            2 => {
                let obj = g.read::<u32>(v.obj);
                g.call(slot(g, img, f, 0xaf, 23), 0, &[obj, 0]);
            }
            // -4: Ch_ClothReset2 of the actor's character
            1 => {
                let index = g.read::<u32>(v.index);
                let chr = if index == u32::MAX {
                    g.read::<u32>(w(0xd1))
                } else {
                    let list = g.read::<u32>(w(0xd8));
                    let b = u32::from(g.read::<u8>(list.wrapping_add(index)));
                    g.read::<u32>(w(0xe3).wrapping_add(4 * b))
                };
                g.call(slot(g, img, f, 0xe9, 3), 0, &[chr]);
            }
            // -5: slot 529 with the DLL's word, a wait of 1
            0 => {
                let x = g.read::<u32>(w(0x13e));
                g.call(slot(g, img, f, 0x138, 529), 0, &[x]);
                break (entry, 1u32);
            }
            // -1: a wait of 1
            4 => break (entry, 1),
            // a motion
            _ => {
                let mode = g.read::<u16>(v.mode).cast_signed();
                let obj = |g: &G| g.read::<u32>(v.obj);
                let w8 = match mode {
                    0 => {
                        let chr = g.read::<u32>(v.chr);
                        // MsEffectSetMotion(chr, op)
                        g.call(slot(g, img, f, 0x1d0, 228), 0, &[chr, op]).eax
                    }
                    1 => {
                        let o = obj(g);
                        // Ch_ResetMotion(obj); slot 36 (obj, op); Ch_GetSeqFrame(obj)
                        g.call(slot(g, img, f, 0x194, 30), 0, &[o]);
                        let o2 = obj(g);
                        g.call(slot(g, img, f, 0x1a6, 36), 0, &[o2, op, o]);
                        let o3 = obj(g);
                        g.call(slot(g, img, f, 0x1b9, 18), 0, &[o3, o2, op, o]).eax
                    }
                    2 => {
                        let o = obj(g);
                        // Ch_ResetMotion(obj); Ch_SetSysMotion(obj, 1, op);
                        // Ch_GetSeqFrame(obj)
                        g.call(slot(g, img, f, 0x158, 30), 0, &[o]);
                        let o2 = obj(g);
                        g.call(slot(g, img, f, 0x16a, 37), 0, &[o2, 1, op, o]);
                        let o3 = obj(g);
                        g.call(slot(g, img, f, 0x17f, 18), 0, &[o3, o2, 1, op, o])
                            .eax
                    }
                    _ => 1,
                };
                break (entry, w8);
            }
        }
    };
    let mode = g.read::<u16>(v.mode);
    let arg = g.read::<u32>(entry + 4);
    // MsEffectEndMotion(chr, k) in mode 0, then the end
    let end = |g: &mut G, k: Option<u32>| {
        if let (Some(k), 0) = (k, mode) {
            let chr = g.read::<u32>(v.chr);
            g.call(slot(g, img, f, 0x235, 213), 0, &[chr, k]);
        }
        g.write::<u32>(v.done, 1);
        g.read::<u32>(v.done)
    };
    match arg.wrapping_add(7) {
        // -1, -3, -5, -6
        6 => end(g, Some(0)),
        4 => end(g, Some(1)),
        2 => end(g, Some(3)),
        1 => end(g, Some(2)),
        // -4
        3 => end(g, None),
        // -2: the wait and the step counter cleared
        5 => {
            g.write::<u32>(v.wait, 0);
            g.read::<u32>(v.done)
        }
        // -7: the same entry again after the motion's length
        0 => {
            if next_wait.cast_signed() <= 0 {
                next_wait = 1;
                if mode == 0 {
                    let chr = g.read::<u32>(v.chr);
                    let r = g.call(slot(g, img, f, 0x265, 228), 0, &[chr, 0x10]).eax;
                    if r.cast_signed() > 0 {
                        next_wait = r;
                    }
                }
            }
            let pos = g.read::<u32>(v.pos).wrapping_sub(1);
            g.write::<u32>(v.pos, pos);
            let done = g.read::<u32>(v.done);
            g.write::<u16>(v.wait, next_wait as u16);
            done
        }
        _ => {
            let a = g.read::<u16>(entry + 4);
            g.write::<u16>(v.wait, a);
            g.read::<u32>(v.done)
        }
    }
}

/// [`motion_step`].
pub const MOTION_STEP: Sig = Sig {
    len: 732,
    hash: 0x141f_7f1f_80a8_5101,
    calls: &[(0xa8, Callee::Sig(&SET_SPEED))],
    imports: &[],
};
