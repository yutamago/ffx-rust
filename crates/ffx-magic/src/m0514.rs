//! `magic_0514.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::l3_common::{ListAt, attach, node_init};
use crate::sa_misc::leave;
use crate::sb_sse::{div, mul};
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
