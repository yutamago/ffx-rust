//! `magic_0514.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::h6_setup::{ring_join, static_child, unlink_task};
use crate::l3_common::{ListAt, attach, node_init};
use crate::m0018::st0_f32;
use crate::runtime::slot_target;
use crate::sa_misc::leave;
use crate::sb_sse::{add, cvt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The task ring append the object set-ups start with: the object `obj`
/// (vtable `ops[0]`, callback `ops[1]`) goes after the ring's tail
/// (`ops[2]`/`ops[4]` read, `ops[3]` the ring), the ring's flag bit 0
/// cleared (`ops[5]`) and the tail set (`ops[6]`).
fn task_append<G: Game>(g: &mut G, img: &Image, f: u32, obj: u32, ops: [u32; 7]) {
    let w = |off: u32| img.word(f + off);
    g.write::<u32>(obj + 8, obj);
    g.write::<u32>(obj + 4, obj);
    g.write::<u32>(obj, w(ops[0]));
    g.write::<u32>(obj + 0x10, w(ops[1]));
    g.write::<u32>(obj + 0x14, obj);
    let tail = g.read::<u32>(w(ops[2]));
    g.write::<u32>(obj + 8, tail);
    g.write::<u32>(obj + 4, w(ops[3]));
    let tail = g.read::<u32>(w(ops[4]));
    g.write::<u32>(tail + 4, obj);
    let fl = g.read::<u32>(w(ops[5]));
    g.write::<u32>(w(ops[5]), fl & !1);
    g.write::<u32>(w(ops[6]), obj);
}

/// Unlinks the task's node `*task` from its ring (linked to itself) and
/// sets the record's state 20000. Returns the node's next.
fn finish<G: Game>(g: &mut G, task: u32, r: u32) -> u32 {
    let node = g.read::<u32>(task);
    let next = g.read::<u32>(node + 4);
    let prev = g.read::<u32>(node + 8);
    g.write::<u32>(next + 8, prev);
    let prev = g.read::<u32>(node + 8);
    let next = g.read::<u32>(node + 4);
    g.write::<u32>(prev + 4, next);
    g.write::<u32>(node + 8, node);
    g.write::<u32>(node + 4, node);
    g.write::<u32>(r + 0x18, 0x4e20);
    next
}

/// Sets up a timed task object (0x10007d70, internal: `ecx` = the
/// object, `edx` a word, cdecl `(a, b, c)`): appended to the task ring
/// ([`task_append`]), state 20000 at `+0x58`, `(a, b, c)` to
/// `+0x28..+0x30`, `edx` to `+0x24`, `+0x18` 0. Returns `a`.
pub fn timed_init<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let obj = fr.ecx;
    task_append(
        g,
        img,
        fr.at,
        obj,
        [0xb, 0x12, 0x1a, 0x24, 0x29, 0x32, 0x3c],
    );
    g.write::<u32>(obj + 0x58, 0x4e20);
    let c = fr.arg(g, 2);
    g.write::<u32>(obj + 0x30, c);
    let b = fr.arg(g, 1);
    g.write::<u32>(obj + 0x2c, b);
    let a = fr.arg(g, 0);
    g.write::<u32>(obj + 0x28, a);
    g.write::<u32>(obj + 0x24, fr.edx);
    g.write::<u32>(obj + 0x18, 0);
    a
}

/// [`timed_init`] (98 bytes, 1 overlay).
pub const TIMED_INIT: Sig = Sig {
    len: 98,
    hash: 0x698a_b87a_009d_5762,
    calls: &[],
    imports: &[],
};

/// Sets up a scaled task object (0x10006e30, internal: `ecx` = the
/// object, `edx` a word, cdecl `(owner)`): appended to the task ring
/// ([`task_append`]); its node `+0xd8` joins the node list (`+0x38` 1.0,
/// `+0x3c` the owner, vtable `+0xa6`); the three scales `+0x104..+0x10c`
/// are `K / s * M_i` with `s` the owner's model's `+0x28`
/// (`owner[0x140]`); `edx` to `+0x24`, `+0x18` 0. Returns the owner's
/// model.
pub fn scaled_init<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.ecx;
    task_append(g, img, f, obj, [0xf, 0x16, 0x1e, 0x28, 0x2d, 0x38, 0x45]);
    let d = obj + 0xd8;
    node_init(g, d, w(0x58));
    let at = ListAt {
        last: w(0x64),
        last2: w(0x6c),
        sentinel: w(0x71),
        first: w(0x79),
        count: w(0x85),
        last_w: w(0x96),
    };
    attach(g, d, w(0x5f), at);
    let owner = fr.arg(g, 0);
    let k = g.read::<f32>(w(0x90));
    g.write::<u32>(d + 0x38, 0x3f80_0000);
    g.write::<u32>(d + 0x3c, owner);
    g.write::<u32>(d, w(0xa6));
    let mut model = 0;
    for (op, dst) in [(0xbcu32, 0x104u32), (0xda, 0x108), (0xf8, 0x10c)] {
        model = g.read::<u32>(owner + 0x140);
        let v = mul(div(k, g.read::<f32>(model + 0x28)), g.read::<f32>(w(op)));
        if dst == 0x10c {
            g.write::<u32>(obj + 0x24, fr.edx);
            g.write::<u32>(obj + 0x18, 0);
        }
        g.write::<f32>(obj + dst, v);
    }
    model
}

/// [`scaled_init`] (271 bytes, 1 overlay).
pub const SCALED_INIT: Sig = Sig {
    len: 271,
    hash: 0x7060_5eec_5da4_9e97,
    calls: &[],
    imports: &[],
};

/// A task (0x10007260, `(task)`, pushed by 0x10007345) on the record `r
/// = task[1]`: state 0 waits two frames (`+0x1c`) in state 1, then state
/// 10000 waits for bit 2 of `+0x2b8` and takes the nodes `+0x20`,
/// `+0x170`, `+0x19c` out of their parents ([`leave`]) and the task's
/// node off its ring ([`finish`]). The wait goes down at every exit.
/// Returns the unlinked node's next, or the state's remainder.
pub fn e7260<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let eax = loop {
        match g.read::<u32>(r + 0x18) {
            0 => {
                g.write::<u32>(r + 0x1c, 2);
                g.write::<u32>(r + 0x18, 1);
            }
            1 => {}
            0x2710 => {
                if g.read::<u8>(r + 0x2b8) & 4 == 0 {
                    break 0;
                }
                let mut eax = 0;
                for off in [0x20, 0x170, 0x19c] {
                    eax = leave(g, r + off, eax);
                }
                let _ = eax;
                break finish(g, task, r);
            }
            s => break s.wrapping_sub(0x2710),
        }
        if g.read::<i32>(r + 0x1c) > 0 {
            break 0;
        }
        g.write::<u32>(r + 0x18, 0x2710);
    };
    let n = g.read::<u32>(r + 0x1c);
    g.write::<u32>(r + 0x1c, n.wrapping_sub(1));
    Ret::eax(eax)
}

/// 0x10007260 (142 bytes, 1 overlay).
pub const E7260: Sig = Sig {
    len: 142,
    hash: 0x0e19_e99e_c896_ce51,
    calls: &[
        (0x2f, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x3a, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x45, Callee::Sig(&crate::sa_misc::LEAVE)),
    ],
    imports: &[],
};

/// A task (0x10007660, `(task)`, pushed by 0x1000770a) on the record `r
/// = task[1]`: state 0 becomes 1, which steps the byte `+0x1eb` by 0xf0
/// (16 down) each frame; when it wraps to 0, state 10000 takes the nodes
/// `+0x1c4`, `+0x20`, `+0x16c`, `+0x198` out of their parents ([`leave`])
/// and the task's node off its ring ([`finish`]). The count `+0x1c` goes
/// down at every exit. Returns the unlinked node's next, or the state's
/// remainder.
pub fn e7660<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let eax = loop {
        match g.read::<u32>(r + 0x18) {
            0 | 1 => {
                g.write::<u32>(r + 0x18, 1);
                let b = g.read::<u8>(r + 0x1eb).wrapping_add(0xf0);
                g.write::<u8>(r + 0x1eb, b);
                if b != 0 {
                    break 0;
                }
                g.write::<u32>(r + 0x18, 0x2710);
            }
            0x2710 => {
                let mut eax = 0;
                for off in [0x1c4, 0x20, 0x16c, 0x198] {
                    eax = leave(g, r + off, eax);
                }
                let _ = eax;
                break finish(g, task, r);
            }
            s => break s.wrapping_sub(0x2710),
        }
    };
    let n = g.read::<u32>(r + 0x1c);
    g.write::<u32>(r + 0x1c, n.wrapping_sub(1));
    Ret::eax(eax)
}

/// 0x10007660 (140 bytes, 1 overlay).
pub const E7660: Sig = Sig {
    len: 140,
    hash: 0x5be0_b0ed_ec8d_ec63,
    calls: &[
        (0x29, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x31, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x3c, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x47, Callee::Sig(&crate::sa_misc::LEAVE)),
    ],
    imports: &[],
};

/// The motion task (0x10006b40, `(task)`, a task step) on the record `r
/// = task[1]` and its object `o = r[0x20]`, by the state `+0x18`; each
/// motion `id` is slot 228 `(o[0x50], id)` (the table global at `+0x4c`)
/// and the next state:
/// - 0: motion 0xb8; 1: once the motion is done (slot 411 `(o[0x50])`
///   non-zero), 0xb9; 10: 0xba and a wait `+0x1c` of 96;
/// - 11: when the wait is out, state 20 at once; 20: motion 0xbb;
/// - 21: once done, slot 213 `(o[0x50], 0)`, the task's node leaves its
///   ring, state 20000.
///
/// The wait goes down at every exit. Returns the last call's `eax`, the
/// node's next after the unlink, or the dispatch's index (`eax` of the
/// original's jump table) or state.
pub fn motion_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let mut state = g.read::<u32>(r + 0x18);
    let o = g.read::<u32>(r + 0x20);
    let model = |g: &G| g.read::<u32>(o + 0x50);
    let done = |g: &mut G, op: u32| {
        let t = slot_target(g, w(op), 411);
        let m = model(g);
        g.call(t, 0, &[m]).eax != 0
    };
    let play = |g: &mut G, op: u32, id: u32| {
        let t = slot_target(g, w(op), 228);
        let m = model(g);
        let e = g.call(t, 0, &[m, id]).eax;
        let s = g.read::<u32>(r + 0x18);
        g.write::<u32>(r + 0x18, s.wrapping_add(1));
        e
    };
    let eax = loop {
        break match state {
            0 => play(g, 0x4c, 0xb8),
            1 => {
                if done(g, 0x6a) {
                    play(g, 0x4c, 0xb9)
                } else {
                    0
                }
            }
            10 => {
                let e = play(g, 0x8c, 0xba);
                g.write::<u32>(r + 0x1c, 0x60);
                e
            }
            11 => {
                if g.read::<i32>(r + 0x1c) > 0 {
                    3
                } else {
                    g.write::<u32>(r + 0x18, 0x14);
                    state = 0x14;
                    continue;
                }
            }
            0x14 => play(g, 0x4c, 0xbb),
            0x15 => {
                if done(g, 0xbd) {
                    let t = slot_target(g, w(0xd4), 213);
                    let m = model(g);
                    g.call(t, 0, &[m, 0]);
                    let next = unlink_task(g, task);
                    g.write::<u32>(r + 0x18, 0x4e20);
                    next
                } else {
                    0
                }
            }
            // the jump table's index for the states it ignores
            2..=9 | 0xc..=0x13 => 6,
            s => s,
        };
    };
    let n = g.read::<u32>(r + 0x1c);
    g.write::<u32>(r + 0x1c, n.wrapping_sub(1));
    Ret::eax(eax)
}

/// [`motion_task`] (326 bytes, 1 overlay).
pub const MOTION_TASK: Sig = Sig {
    len: 326,
    hash: 0xdd89_9bd8_74ac_f3ca,
    calls: &[],
    imports: &[],
};

/// The fade task (0x10006c90, `(task)`, a task step) on the record `r =
/// task[1]`, by the state `+0x18`; the level bytes `+0xa8` and `+0xa9`,
/// a wait `+0x1c` and a count `+0x20`:
/// - 0: `+0xa8` down by 2; at 0x60, a wait of 26 and state 1; 1: when
///   the wait is out, state 30 at once;
/// - 30: a wait of 40 and state 31; 31: when the wait is out, state 32,
///   the count 16 and the wait 0, at once;
/// - 32: `+0xa9` down by 0x20 unless 0; when the wait is out, `+0xa9`
///   0x80 and the count down: a wait of 6, or at 0 state 33;
/// - 33: `+0xa9` up by 0xe0; at 0 state 10 at once; 10: state 11 and a
///   wait of 20; 11: when the wait is out, state 12;
/// - 12: `+0xa8` up by 2; at 0x80 state 13 and a wait of 2; 13: when the
///   wait is out, the task's node leaves its ring, state 20000.
///
/// The wait goes down at every exit. Returns the record (the original's
/// `eax` throughout).
pub fn fade_task<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let wait_out = |g: &G| g.read::<i32>(r + 0x1c) <= 0;
    let next_state = |g: &mut G| {
        let s = g.read::<u32>(r + 0x18);
        g.write::<u32>(r + 0x18, s.wrapping_add(1));
    };
    let mut state = g.read::<u32>(r + 0x18);
    loop {
        match state {
            1 if wait_out(g) => {
                g.write::<u32>(r + 0x18, 0x1e);
            }
            0x1e => {
                g.write::<u32>(r + 0x1c, 0x28);
                g.write::<u32>(r + 0x18, state + 1);
            }
            0x1f if wait_out(g) => {
                next_state(g);
                g.write::<u32>(r + 0x20, 0x10);
                g.write::<u32>(r + 0x1c, 0);
            }
            0x21 => {
                let b = g.read::<u8>(r + 0xa9).wrapping_add(0xe0);
                g.write::<u8>(r + 0xa9, b);
                if b != 0 {
                    break;
                }
                g.write::<u32>(r + 0x18, 10);
            }
            0 => {
                let b = g.read::<u8>(r + 0xa8).wrapping_add(0xfe);
                g.write::<u8>(r + 0xa8, b);
                if g.read::<u8>(r + 0xa8) == 0x60 {
                    g.write::<u32>(r + 0x1c, 0x1a);
                    next_state(g);
                }
                break;
            }
            0x20 => {
                let b = g.read::<u8>(r + 0xa9);
                if b != 0 {
                    g.write::<u8>(r + 0xa9, b.wrapping_sub(0x20));
                }
                if wait_out(g) {
                    let n = g.read::<u32>(r + 0x20).wrapping_sub(1);
                    g.write::<u32>(r + 0x20, n);
                    g.write::<u8>(r + 0xa9, 0x80);
                    if n == 0 {
                        next_state(g);
                    } else {
                        g.write::<u32>(r + 0x1c, 6);
                    }
                }
                break;
            }
            10 => {
                next_state(g);
                g.write::<u32>(r + 0x1c, 0x14);
                break;
            }
            11 => {
                if wait_out(g) {
                    next_state(g);
                }
                break;
            }
            12 => {
                let b = g.read::<u8>(r + 0xa8).wrapping_add(2);
                g.write::<u8>(r + 0xa8, b);
                if g.read::<u8>(r + 0xa8) == 0x80 {
                    next_state(g);
                    g.write::<u32>(r + 0x1c, 2);
                }
                break;
            }
            13 if wait_out(g) => {
                unlink_task(g, task);
                g.write::<u32>(r + 0x18, 0x4e20);
                break;
            }
            _ => break,
        }
        state = g.read::<u32>(r + 0x18);
        if state > 0x21 {
            break;
        }
    }
    let n = g.read::<u32>(r + 0x1c);
    g.write::<u32>(r + 0x1c, n.wrapping_sub(1));
    Ret::eax(r)
}

/// [`fade_task`] (410 bytes, 1 overlay).
pub const FADE_TASK: Sig = Sig {
    len: 410,
    hash: 0xa1c8_2afb_bda1_95ee,
    calls: &[],
    imports: &[],
};

/// The colour task (0x10006f40, `(task)`, a task step; a /GS frame) on
/// the record `r = task[1]`, by the state `+0x18`:
/// - 20: the four `u16` levels `+0x2a0..` times `K` into the floats
///   `+0x1fc..+0x208`, slot 325 `(r + 0x20c, &V)` with `V = (1, 1, 1, 0)`
///   on the frame (`ebp - 0x14`), bit 0 of `+0x2b8` set, `+0x2b4` =
///   0x00140014, state 21;
/// - 21: once bit 0 of `+0x2b8` is clear: the node `r + 0x1d0` freed and
///   unlinked ([`free_then_unlink`](crate::nodes::free_then_unlink), the
///   local `[node]` at `ebp - 0x18`); unless bit 2 of its `+0xe8` is set,
///   `**G_P` = its `+0x50` and slot 713 `(node + 0x60)`; the task's node
///   leaves its ring, state 20000.
///
/// The wait `+0x1c` goes down at every exit. Returns slot 325's `eax`,
/// the node's next after the unlink, or `state - 21`.
pub fn colour_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let state = g.read::<u32>(r + 0x18);
    let ebp = fr.esp.wrapping_sub(4);
    let eax = match state {
        0x14 => {
            let k = g.read::<f32>(w(0x3e));
            let v = ebp.wrapping_sub(0x14);
            for (i, x) in (0u32..).zip([0x3f80_0000u32, 0x3f80_0000, 0x3f80_0000, 0]) {
                g.write::<u32>(v + 4 * i, x);
            }
            for i in 0..4u32 {
                let n = u32::from(g.read::<u16>(r + 0x2a0 + 2 * i));
                g.write::<f32>(r + 0x1fc + 4 * i, mul(cvt(n), k));
            }
            let t = slot_target(g, w(0xca), 325);
            let e = g.call(t, 0, &[r + 0x20c, v]).eax;
            let flags = g.read::<u32>(r + 0x2b8);
            g.write::<u32>(r + 0x2b8, flags | 1);
            g.write::<u32>(r + 0x2b4, 0x0014_0014);
            g.write::<u32>(r + 0x18, 0x15);
            e
        }
        0x15 if g.read::<u8>(r + 0x2b8) & 1 != 0 => 0,
        0x15 => {
            let node = r + 0x1d0;
            let local = ebp.wrapping_sub(0x18);
            g.write::<u32>(local, node);
            crate::nodes::free_then_unlink(
                g,
                img,
                &Frame {
                    at: img.rel32_target(f + 0x10c),
                    // push ebp; sub esp, 0x18; three pushes: the call's esp
                    esp: fr.esp.wrapping_sub(0x2c),
                    ecx: local,
                    ..Frame::default()
                },
            );
            if g.read::<u8>(node + 0xe8) & 4 == 0 {
                let p = g.read::<u32>(w(0x11a));
                let v = g.read::<u32>(node + 0x50);
                g.write::<u32>(p, v);
                let t = slot_target(g, w(0x124), 713);
                g.call(t, 0, &[node + 0x60]);
            }
            let next = unlink_task(g, task);
            g.write::<u32>(r + 0x18, 0x4e20);
            next
        }
        s => s.wrapping_sub(0x15),
    };
    let n = g.read::<u32>(r + 0x1c);
    g.write::<u32>(r + 0x1c, n.wrapping_sub(1));
    Ret::eax(eax)
}

/// [`colour_task`] (364 bytes, 1 overlay).
pub const COLOUR_TASK: Sig = Sig {
    len: 364,
    hash: 0x242b_0145_1d01_b243,
    calls: &[
        // __security_check_cookie
        (0x32, Callee::Any),
        (0x10c, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x164, Callee::Any),
    ],
    imports: &[],
};

/// Sets up the colour object (0x100070b0, internal: `ecx` = the object,
/// `edx` a word): the object joins the DLL's ring
/// ([`ring_join`](crate::h6_setup::ring_join), its step the colour task);
/// children of the DLL's node list `b = obj + 0x16c` (handlers `+0x95`
/// after) and `c = obj + 0x198` (`+0x10f` after); `+0x188..` = `(0,
/// -4.5, -1.5, 1)`, `+0x1b4..` = `(1, 1, 1, 120)`; the node insert
/// ([`node_insert`](crate::nodes::node_insert)) of `[obj + 0x20, LIST,
/// edx, b, NAME, c]` and a particle node
/// ([`particle_node`](crate::sb_nodes::particle_node)) of `[obj + 0x1d0,
/// LIST2, obj + 0x20, *G_O, PART, 0]` (vtable `+0x170` first), both on
/// the frame (`ebp - 0x18`); `+0x18` 0. Returns the particle node's
/// `eax`.
pub fn colour_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.ecx;
    ring_join(g, img, f, obj, [0x15, 0x1c, 0x24, 0x2e, 0x33, 0x42, 0x49]);
    let b = obj + 0x16c;
    static_child(
        g,
        img,
        f,
        b,
        [0x5c, 0x63, 0x68, 0x70, 0x75, 0x7d, 0x89, 0x8f],
    );
    g.write::<u32>(b, w(0x95));
    for (o, v) in [
        (0x188u32, 0u32),
        (0x18c, 0xc090_0000),
        (0x190, 0xbfc0_0000),
        (0x194, 0x3f80_0000),
    ] {
        g.write::<u32>(obj + o, v);
    }
    let c = obj + 0x198;
    static_child(
        g,
        img,
        f,
        c,
        [0xd6, 0xdd, 0xe2, 0xea, 0xef, 0xf7, 0x103, 0x109],
    );
    g.write::<u32>(c, w(0x10f));
    for (o, v) in [
        (0x1b4u32, 0x3f80_0000u32),
        (0x1b8, 0x3f80_0000),
        (0x1bc, 0x3f80_0000),
        (0x1c0, 0x42f0_0000),
    ] {
        g.write::<u32>(obj + o, v);
    }
    let rec = fr.esp.wrapping_sub(4).wrapping_sub(0x18);
    for (i, v) in (0u32..).zip([obj + 0x20, w(0x14a), fr.edx, b, w(0x157), c]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    crate::nodes::node_insert(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + 0x15c),
            // push ebp; sub esp, 0x1c; three pushes: the call's esp
            esp: fr.esp.wrapping_sub(0x30),
            ecx: rec,
            ..Frame::default()
        },
    );
    let o = g.read::<u32>(w(0x161));
    let node = obj + 0x1d0;
    g.write::<u32>(rec, node);
    g.write::<u32>(node, w(0x170));
    for (i, v) in (1u32..).zip([w(0x17a), obj + 0x20, o, w(0x187), 0]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    let r = crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x193), rec);
    g.write::<u32>(obj + 0x18, 0);
    r
}

/// [`colour_setup`] (421 bytes, 1 overlay).
pub const COLOUR_SETUP: Sig = Sig {
    len: 421,
    hash: 0x88c8_6afc_538b_19bc,
    calls: &[
        (0x15c, Callee::Sig(&crate::nodes::INSERT)),
        (0x193, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
    ],
    imports: &[],
};

/// Sets up the glow object over a source (0x100076f0, internal: `ecx` =
/// the object, `edx` a word, one stack word `src`): the object joins the
/// DLL's ring ([`ring_join`](crate::h6_setup::ring_join)); children of
/// the DLL's node list `b = obj + 0x16c` (handlers `+0x98`) and
/// `c = obj + 0x198` (`+0x110`); `+0x188..` = `(src[0x1c], -0.5, src[0x24], 1)`,
/// `+0x1b4..` = `(1, 1, 1, 0.05)`; the node insert
/// ([`node_insert`](crate::nodes::node_insert)) of `[obj + 0x20, LIST,
/// edx, b, NAME, c]` on the frame (`ebp - 0x1c`); `d = obj + 0x1c4` a
/// child of the second list with `+0x1c` = `obj + 0x20`, `+0x20` = slot
/// 719 `(NAME2, 4, 0)`, `+0x24` = 0x80808080 and its handlers (`+0x1d8`);
/// byte `+0x1eb` 0x40, `+0x18` 0. Returns slot 719's `eax`.
pub fn glow_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.ecx;
    let src = fr.arg(g, 0);
    ring_join(g, img, f, obj, [0x13, 0x1a, 0x22, 0x2c, 0x31, 0x3c, 0x49]);
    let b = obj + 0x16c;
    static_child(
        g,
        img,
        f,
        b,
        [0x5c, 0x63, 0x68, 0x70, 0x75, 0x7d, 0x89, 0x92],
    );
    g.write::<u32>(b, w(0x98));
    let x = g.read::<u32>(src + 0x1c);
    g.write::<u32>(obj + 0x188, x);
    g.write::<u32>(obj + 0x18c, 0xbf00_0000);
    let z = g.read::<u32>(src + 0x24);
    g.write::<u32>(obj + 0x190, z);
    g.write::<u32>(obj + 0x194, 0x3f80_0000);
    let c = obj + 0x198;
    static_child(
        g,
        img,
        f,
        c,
        [0xd7, 0xde, 0xe3, 0xeb, 0xf0, 0xf8, 0x104, 0x10a],
    );
    g.write::<u32>(c, w(0x110));
    for (o, v) in [
        (0x1b4u32, 0x3f80_0000u32),
        (0x1b8, 0x3f80_0000),
        (0x1bc, 0x3f80_0000),
        (0x1c0, 0x3d4c_cccc),
    ] {
        g.write::<u32>(obj + o, v);
    }
    let rec = fr.esp.wrapping_sub(4).wrapping_sub(0x1c);
    for (i, v) in (0u32..).zip([obj + 0x20, w(0x14b), fr.edx, b, w(0x158), c]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    crate::nodes::node_insert(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + 0x15d),
            // push ebp; sub esp, 0x20; three pushes: the call's esp
            esp: fr.esp.wrapping_sub(0x34),
            ecx: rec,
            ..Frame::default()
        },
    );
    let t = crate::runtime::slot_target(g, w(0x162), 719);
    let h = g.call(t, 0, &[w(0x171), 4, 0]).eax;
    let d = obj + 0x1c4;
    static_child(
        g,
        img,
        f,
        d,
        [0x18c, 0x193, 0x199, 0x1a2, 0x1ab, 0x1b3, 0x1bf, 0x1c5],
    );
    g.write::<u32>(d + 0x1c, obj + 0x20);
    g.write::<u32>(d + 0x20, h);
    g.write::<u32>(d + 0x24, 0x8080_8080);
    g.write::<u32>(d, w(0x1d8));
    g.write::<u8>(obj + 0x1eb, 0x40);
    g.write::<u32>(obj + 0x18, 0);
    h
}

/// [`glow_setup`] (497 bytes, 1 overlay).
pub const GLOW_SETUP: Sig = Sig {
    len: 497,
    hash: 0xaedf_df8d_9886_eb81,
    calls: &[(0x15d, Callee::Sig(&crate::nodes::INSERT))],
    imports: &[],
};

/// Sets up the burst object (0x10007a90, internal: `ecx` = the object,
/// `edx` a word, three stack words `a, b, c`; an 8-aligned frame): the
/// object joins the DLL's ring ([`ring_join`](crate::h6_setup::ring_join));
/// children of the DLL's node list `n1 = obj + 0x178` (handlers `+0x98`)
/// and `n2 = obj + 0x1a4` (`+0x112`); `+0x194..` = `(0, -4.5, -3.2, 1)`,
/// `+0x1c0..` = `(1, 1, 1, 75)`; the node insert
/// ([`node_insert`](crate::nodes::node_insert)) of `[obj + 0x24, LIST,
/// edx, n1, NAME, n2]` and a particle node
/// ([`particle_node`](crate::sb_nodes::particle_node)) of `[obj + 0x1d0,
/// LIST2, obj + 0x24, *G_O, PART, 1]` (vtable `+0x17b` first), both at
/// the aligned frame's `esp + 0x10`; the ten parts' states (`+0x2f0`
/// every `0x1ec`, five pairs) 20000; `+0x2d4` = `c`, `+0x2d0` = `b`,
/// `+0x170` = `edx`, `+0x174` = `a`, `+0x18` 0. Returns `a`.
pub fn burst_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.ecx;
    ring_join(g, img, f, obj, [0x18, 0x1f, 0x27, 0x31, 0x36, 0x45, 0x4c]);
    let n1 = obj + 0x178;
    static_child(
        g,
        img,
        f,
        n1,
        [0x5f, 0x66, 0x6b, 0x73, 0x78, 0x80, 0x8c, 0x92],
    );
    g.write::<u32>(n1, w(0x98));
    for (o, v) in [
        (0x194u32, 0u32),
        (0x198, 0xc090_0000),
        (0x19c, 0xc04c_cccd),
        (0x1a0, 0x3f80_0000),
    ] {
        g.write::<u32>(obj + o, v);
    }
    let n2 = obj + 0x1a4;
    static_child(
        g,
        img,
        f,
        n2,
        [0xd9, 0xe0, 0xe5, 0xed, 0xf2, 0xfa, 0x106, 0x10c],
    );
    g.write::<u32>(n2, w(0x112));
    for (o, v) in [
        (0x1c0u32, 0x3f80_0000u32),
        (0x1c4, 0x3f80_0000),
        (0x1c8, 0x3f80_0000),
        (0x1cc, 0x4296_0000),
    ] {
        g.write::<u32>(obj + o, v);
    }
    // push ebp; and esp, -8; sub esp, 0x1c; three pushes
    let esp = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x28);
    let rec = esp + 0x10;
    for (i, v) in (0u32..).zip([obj + 0x24, w(0x151), fr.edx, n1, w(0x161), n2]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    crate::nodes::node_insert(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + 0x166),
            esp: esp.wrapping_sub(4),
            ecx: rec,
            ..Frame::default()
        },
    );
    let o = g.read::<u32>(w(0x16b));
    let node = obj + 0x1d0;
    g.write::<u32>(rec, node);
    g.write::<u32>(node, w(0x17b));
    for (i, v) in (1u32..).zip([w(0x187), obj + 0x24, o, w(0x197), 1]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x1a4), rec);
    let c = fr.arg(g, 2);
    for o in [
        0x2f0u32, 0xc98, 0x4dc, 0xf68, 0x6c8, 0x1238, 0x8b4, 0x1508, 0xaa0, 0x17d8,
    ] {
        g.write::<u32>(obj + o, 0x4e20);
    }
    g.write::<u32>(obj + 0x2d4, c);
    let b = fr.arg(g, 1);
    g.write::<u32>(obj + 0x2d0, b);
    let a = fr.arg(g, 0);
    g.write::<u32>(obj + 0x170, fr.edx);
    g.write::<u32>(obj + 0x174, a);
    g.write::<u32>(obj + 0x18, 0);
    a
}

/// [`burst_setup`] (571 bytes, 1 overlay).
pub const BURST_SETUP: Sig = Sig {
    len: 571,
    hash: 0x16ff_f55c_c9db_7d13,
    calls: &[
        (0x166, Callee::Sig(&crate::nodes::INSERT)),
        (0x1a4, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
    ],
    imports: &[],
};

/// Sets up the spark object at a target (0x100072f0, internal: `ecx` =
/// the object, `edx` a word, two stack words `a, b`): the target's size
/// `s` (slot 251 `(B)`, `B` the DLL's byte at `+0x09`; at most `K1`,
/// times `K2`); the object joins the DLL's ring
/// ([`ring_join`](crate::h6_setup::ring_join)); children `n1 = obj +
/// 0x170` (handlers `+0xdd`) and `n2 = obj + 0x19c` (`+0x21c`) of the
/// DLL's node list; slot 268 `(a[0x54], 0x11, obj + 0x18c)`, `+0x198` 1;
/// four random floats `r` (slot 307's ST0, each kept in the stack word
/// `a` as the original does) move the point: `+0x18c += ((r - K3) * K4 -
/// K5) * s`, the same for `+0x190`, `+0x194 -= ((r - K3) * K6 + K7) * s`,
/// and `+0x1c4 = r - K3 + K8`; `+0x1b8..+0x1c0` 1; the node insert
/// ([`node_insert`](crate::nodes::node_insert)) of `[obj + 0x20, LIST,
/// NAME, n1, b, n2]` and a particle node
/// ([`particle_node`](crate::sb_nodes::particle_node)) of `[obj + 0x1d0,
/// LIST2, obj + 0x20, *G_O, PART, 2]` (vtable `+0x2a4` first), both on
/// the frame (`ebp - 0x20`); slot 388 `(B, T[rand & 1], 1)` (`T` operand
/// `+0x2e0`, slot 324), slot 239 `(*G_A, B, 6)`, slot 306 `(7, 1, slot
/// 719 (NAME2, 5, 0))`; `+0x16c` = `edx`, `+0x18` 0. Returns `edx`.
#[allow(clippy::too_many_lines)] // the original's one long sequence
pub fn spark_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, op: u32| g.read::<f32>(w(op));
    let obj = fr.ecx;
    let word = fr.esp + 4;
    let byte = |g: &G, op: u32| u32::from(g.read::<u8>(w(op)));
    let t = slot_target(g, w(0x12), 251);
    let b0 = byte(g, 0x9);
    let x = st0_f32(g.call(t, 0, &[b0]).st0);
    let k1 = k(g, 0x27);
    let x = if x > k1 { k1 } else { x };
    let size = mul(x, k(g, 0x42));
    ring_join(g, img, f, obj, [0x4e, 0x55, 0x5d, 0x67, 0x6c, 0x7b, 0x82]);
    let n1 = obj + 0x170;
    static_child(
        g,
        img,
        f,
        n1,
        [0x95, 0x9c, 0xa1, 0xa9, 0xb3, 0xbb, 0xc7, 0xd6],
    );
    let a = g.read::<u32>(word);
    g.write::<u32>(n1, w(0xdd));
    let t = slot_target(g, w(0xe7), 268);
    let h = g.read::<u32>(a + 0x54);
    g.call(t, 0, &[h, 0x11, obj + 0x18c]);
    g.write::<u32>(obj + 0x198, 0x3f80_0000);
    // a random float through the stack word, as the original keeps it
    let random = |g: &mut G, op: u32| {
        let t = slot_target(g, w(op), 307);
        let r = st0_f32(g.call(t, 0, &[]).st0);
        g.write::<f32>(word, r);
        g.read::<f32>(word)
    };
    for (op, k3, k4, k5, at) in [
        (0xf4u32, 0x116u32, 0x123u32, 0x12bu32, 0x18cu32),
        (0x11b, 0x150, 0x15d, 0x165, 0x190),
    ] {
        let r = random(g, op);
        let y = mul(sub(mul(sub(r, k(g, k3)), k(g, k4)), k(g, k5)), size);
        let v = add(y, g.read::<f32>(obj + at));
        g.write::<f32>(obj + at, v);
    }
    let r = random(g, 0x155);
    let y = mul(
        add(mul(sub(r, k(g, 0x192)), k(g, 0x1b2)), k(g, 0x1ba)),
        size,
    );
    let v = sub(g.read::<f32>(obj + 0x194), y);
    g.write::<f32>(obj + 0x194, v);
    let n2 = obj + 0x19c;
    static_child(
        g,
        img,
        f,
        n2,
        [0x1de, 0x1e5, 0x1ea, 0x1f2, 0x1f7, 0x1ff, 0x20b, 0x216],
    );
    g.write::<u32>(n2, w(0x21c));
    for o in [0x1b8u32, 0x1bc, 0x1c0] {
        g.write::<u32>(obj + o, 0x3f80_0000);
    }
    let r = random(g, 0x210);
    let z = add(sub(r, k(g, 0x252)), k(g, 0x269));
    let b = fr.arg(g, 1);
    let rec = fr.esp.wrapping_sub(4).wrapping_sub(0x20);
    g.write::<f32>(obj + 0x1c4, z);
    for (i, v) in (0u32..).zip([obj + 0x20, w(0x276), w(0x27d), n1, b, n2]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    crate::nodes::node_insert(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + 0x290),
            // push ebp; sub esp, 0x24; three pushes: the call's esp
            esp: fr.esp.wrapping_sub(0x34),
            ecx: rec,
            ..Frame::default()
        },
    );
    let o = g.read::<u32>(w(0x295));
    let node = obj + 0x1d0;
    g.write::<u32>(rec, node);
    g.write::<u32>(node, w(0x2a4));
    for (i, v) in (1u32..).zip([w(0x2ae), obj + 0x20, o, w(0x2bb), 2]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x2c7), rec);
    let t = slot_target(g, w(0x2cc), 324);
    let pick = g.call(t, 0, &[]).eax & 1;
    let tab = g.read::<u32>(w(0x2e0) + 4 * pick);
    let b1 = byte(g, 0x2e7);
    let t = slot_target(g, w(0x2ed), 388);
    g.call(t, 0, &[b1, tab, 1]);
    let b2 = byte(g, 0x2fc);
    let t = slot_target(g, w(0x307), 239);
    let ga = g.read::<u32>(w(0x30d));
    g.call(t, 0, &[ga, b2, 6]);
    let t = slot_target(g, w(0x31a), 719);
    let h = g.call(t, 0, &[w(0x32c), 5, 0]).eax;
    let t = slot_target(g, w(0x337), 306);
    g.call(t, 0, &[7, 1, h]);
    g.write::<u32>(obj + 0x16c, fr.edx);
    g.write::<u32>(obj + 0x18, 0);
    fr.edx
}

/// [`spark_setup`] (865 bytes, 1 overlay).
pub const SPARK_SETUP: Sig = Sig {
    len: 865,
    hash: 0x522f_c349_1adb_6d32,
    calls: &[
        (0x290, Callee::Sig(&crate::nodes::INSERT)),
        (0x2c7, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
    ],
    imports: &[],
};

/// The burst's state step (0x10007cd0, cdecl `(p)`, `p = [node, s]`):
/// by the state `s + 0x18`: 0 clears the count `+0x1c` and the round
/// `+0x20` and becomes 1; 0 and 1 then start a burst when the count is
/// at or below 0 and `+0x58` is 20000 ([`burst_setup`]: `ecx` = `s +
/// 0x40`, `edx` = `+0x24`, the stack `(+0x28, +0x2c, +0x30)`; the round
/// up, the count 10). 10000 with a bit of 0x4e20 in `+0x58` unlinks the
/// node from its ring (a ring of its own), the count down and the state
/// 20000, and returns. Every other way ends with the count down.
/// Returns the `eax` the original leaves (0, the state's distance from
/// 10000, the burst's answer or the unlinked node's old next).
pub fn burst_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let p = fr.arg(g, 0);
    let s = g.read::<u32>(p + 4);
    let state = g.read::<u32>(s + 0x18);
    let mut eax = 0;
    let try_burst = match state {
        0 => {
            g.write::<u32>(s + 0x1c, 0);
            g.write::<u32>(s + 0x20, 0);
            g.write::<u32>(s + 0x18, 1);
            true
        }
        1 => true,
        0x2710 => {
            if g.read::<u32>(s + 0x58) & 0x4e20 != 0 {
                let d = g.read::<u32>(p);
                let next = g.read::<u32>(d + 4);
                let prev = g.read::<u32>(d + 8);
                g.write::<u32>(next + 8, prev);
                let prev = g.read::<u32>(d + 8);
                let next = g.read::<u32>(d + 4);
                g.write::<u32>(prev + 4, next);
                g.write::<u32>(d + 8, d);
                g.write::<u32>(d + 4, d);
                let n = g.read::<u32>(s + 0x1c);
                g.write::<u32>(s + 0x1c, n.wrapping_sub(1));
                g.write::<u32>(s + 0x18, 0x4e20);
                return Ret::eax(next);
            }
            false
        }
        _ => {
            eax = state.wrapping_sub(0x2710);
            false
        }
    };
    if try_burst && g.read::<u32>(s + 0x1c).cast_signed() <= 0 && g.read::<u32>(s + 0x58) == 0x4e20
    {
        // push ebp, esi, the three words, the return address
        let esp = fr.esp.wrapping_sub(0x18);
        for (i, o) in (1u32..).zip([0x28, 0x2c, 0x30]) {
            let v = g.read::<u32>(s + o);
            g.write::<u32>(esp + 4 * i, v);
        }
        let burst = Frame {
            at: img.rel32_target(f + 0x81),
            esp,
            ecx: s + 0x40,
            edx: g.read::<u32>(s + 0x24),
            ..*fr
        };
        eax = burst_setup(g, img, &burst);
        let r = g.read::<u32>(s + 0x20);
        g.write::<u32>(s + 0x20, r.wrapping_add(1));
        g.write::<u32>(s + 0x1c, 10);
    }
    let n = g.read::<u32>(s + 0x1c);
    g.write::<u32>(s + 0x1c, n.wrapping_sub(1));
    Ret::eax(eax)
}

/// [`burst_step`] (152 bytes, 1 overlay).
pub const BURST_STEP: Sig = Sig {
    len: 152,
    hash: 0xf6fd_5bfd_7b28_25d3,
    calls: &[(0x81, Callee::Sig(&BURST_SETUP))],
    imports: &[],
};
