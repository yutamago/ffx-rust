//! `magic_0478.dll`'s own functions (besides the runtime and the shared
//! code): a blinking object (its set-up and its task).

use crate::l3_common::{ListAt, attach, node_init};
use crate::ob_model::st0_f32;
use crate::runtime::slot_target;
use crate::sb_sse::{add, cvt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// One blink step of the record `r`: the phase counter `+0x24` goes up;
/// the scale `+0x30` moves by `±k` (`+k` on even half-cycles `+0x28`,
/// `-k` on odd ones); at a full count (1) the counter restarts and the
/// half-cycle goes up. Returns whether the blink count `+0x20` ran out
/// (not above 0).
fn blink<G: Game>(g: &mut G, r: u32, k: f32) -> bool {
    let half = g.read::<u32>(r + 0x28);
    let n = g.read::<i32>(r + 0x24).wrapping_add(1);
    g.write::<i32>(r + 0x24, n);
    let sign = 1u32.wrapping_sub((half & 1) * 2);
    let s = add(mul(cvt(sign), k), g.read::<f32>(r + 0x30));
    g.write::<f32>(r + 0x30, s);
    if g.read::<i32>(r + 0x24) >= 1 {
        g.write::<u32>(r + 0x24, 0);
        g.write::<u32>(r + 0x28, half.wrapping_add(1));
    }
    g.read::<i32>(r + 0x20) <= 0
}

/// The blinking object's task (0x10006700, `(task)`, pushed by the
/// set-up [`init`]): a state machine on the record `r = task[1]`
/// (`+0x18`), run until a state waits:
/// - 0: the scale `+0x30` 1.0, six blinks (`+0x20`), then 1;
/// - 1: a blink by `K3` ([`blink`]); when the blinks ran out, state 20
///   (else it waits);
/// - 10: the counters cleared, then 11: a blink by `K2`; when out,
///   state 30;
/// - 20: the speed `+0x3c` 0.15, 25 blinks, then 21: the speed drops by
///   `K4`; while above 0 the scale drops by it (and waits), else state
///   10;
/// - 30: the mode `+0x1c` 10, then 31: the scale rises by `K3`; at `K1`
///   or more it is 1.0 and the state 10000;
/// - 10000: the task's node `*task` leaves its ring (linked to itself),
///   state 20000.
///
/// Then by the mode `+0x1c` (the model `o = +0x40`): 0 slot 228 `(o[0x50],
/// 0x18)`; 1 slot 411 `(o[0x50])` and, when that is nonzero, slot 228
/// `(o[0x50], 0x19)`; 10 slot 239 `(*G, o[0x50], 0)`; the mode goes up
/// after a call that ran (not after 411 said 0). `+0x90` = `+0x34 - (K1 -
/// scale) * +0x2c` and the blink count `+0x20` drops. Returns the last
/// call's `eax` (`mode - 10` for another mode, 0 after 411 said 0).
#[allow(clippy::too_many_lines)]
pub fn task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k1 = g.read::<f32>(w(0x7));
    let k3 = g.read::<f32>(w(0xf));
    let k4 = g.read::<f32>(w(0x17));
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    loop {
        let state = g.read::<u32>(r + 0x18);
        match state {
            0 | 1 => {
                if state == 0 {
                    g.write::<u32>(r + 0x30, ONE);
                    g.write::<u32>(r + 0x24, 0);
                    g.write::<u32>(r + 0x28, 0);
                    g.write::<u32>(r + 0x20, 6);
                    g.write::<u32>(r + 0x18, 1);
                }
                if !blink(g, r, k3) {
                    break;
                }
                g.write::<u32>(r + 0x18, 0x14);
            }
            10 | 11 => {
                if state == 10 {
                    g.write::<u32>(r + 0x24, 0);
                    g.write::<u32>(r + 0x28, 0);
                    g.write::<u32>(r + 0x18, 11);
                }
                let k2 = g.read::<f32>(w(0x104));
                if !blink(g, r, k2) {
                    break;
                }
                g.write::<u32>(r + 0x18, 0x1e);
            }
            0x14 | 0x15 => {
                if state == 0x14 {
                    g.write::<u32>(r + 0x3c, 0x3e19_999a);
                    g.write::<u32>(r + 0x20, 0x19);
                    g.write::<u32>(r + 0x18, 0x15);
                }
                let speed = sub(g.read::<f32>(r + 0x3c), k4);
                g.write::<f32>(r + 0x3c, speed);
                // comiss 0, speed; jb: above 0 or unordered
                if 0.0 < speed || speed.is_nan() {
                    let s = sub(g.read::<f32>(r + 0x30), speed);
                    g.write::<f32>(r + 0x30, s);
                    break;
                }
                g.write::<u32>(r + 0x18, 0xa);
            }
            0x1e | 0x1f => {
                if state == 0x1e {
                    g.write::<u32>(r + 0x1c, 0xa);
                    g.write::<u32>(r + 0x18, 0x1f);
                }
                let s = add(g.read::<f32>(r + 0x30), k3);
                g.write::<f32>(r + 0x30, s);
                // comiss s, K1; jb: below or unordered
                if s < k1 || s.is_nan() || k1.is_nan() {
                    break;
                }
                g.write::<u32>(r + 0x30, ONE);
                g.write::<u32>(r + 0x18, 0x2710);
            }
            0x2710 => {
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
                break;
            }
            _ => break,
        }
    }
    let mode = g.read::<u32>(r + 0x1c);
    let o = g.read::<u32>(r + 0x40);
    let mut eax = mode.wrapping_sub(10);
    let mut ran = false;
    let k = match mode {
        0 => {
            let t = slot_target(g, w(0x23d), 228);
            let m = g.read::<u32>(o + 0x50);
            eax = g.call(t, 0, &[m, 0x18]).eax;
            ran = true;
            g.read::<f32>(w(0x253))
        }
        1 => {
            let t = slot_target(g, w(0x21d), 411);
            let m = g.read::<u32>(o + 0x50);
            eax = g.call(t, 0, &[m]).eax;
            if eax != 0 {
                let t = slot_target(g, w(0x23d), 228);
                let m = g.read::<u32>(o + 0x50);
                eax = g.call(t, 0, &[m, 0x19]).eax;
                ran = true;
            }
            g.read::<f32>(w(0x253))
        }
        10 => {
            let m = g.read::<u32>(o + 0x50);
            let a = g.read::<u32>(w(0x205));
            let t = slot_target(g, w(0x1ff), 239);
            eax = g.call(t, 0, &[a, m, 0]).eax;
            ran = true;
            g.read::<f32>(w(0x253))
        }
        _ => k1,
    };
    if ran {
        g.write::<u32>(r + 0x1c, mode.wrapping_add(1));
    }
    let d = mul(sub(k, g.read::<f32>(r + 0x30)), g.read::<f32>(r + 0x2c));
    let v = sub(g.read::<f32>(r + 0x34), d);
    g.write::<f32>(r + 0x90, v);
    let n = g.read::<u32>(r + 0x20);
    g.write::<u32>(r + 0x20, n.wrapping_sub(1));
    Ret::eax(eax)
}

/// [`task`] (668 bytes with its jump tables, 1 overlay).
pub const TASK: Sig = Sig {
    len: 668,
    hash: 0x6972_9092_57e6_a696,
    calls: &[],
    imports: &[],
};

/// The list operands of one node append ([`attach`]) in [`init`]: the
/// last-node reads, the sentinel, the first word, the count, the
/// last-node write.
fn list_at(img: &Image, f: u32, o: [u32; 6]) -> ListAt {
    let w = |off: u32| img.word(f + off);
    ListAt {
        last: w(o[0]),
        last2: w(o[1]),
        sentinel: w(o[2]),
        first: w(o[3]),
        count: w(o[4]),
        last_w: w(o[5]),
    }
}

/// Sets up the blinking object (0x100069a0, internal: `ecx` = the
/// object, `edx` = the effect; a frame with locals). The object becomes
/// a task (vtable `+0x15`, callback [`task`] at `+0x1c`) appended to the
/// task ring (tail `+0x24`, the ring's flag bit cleared); its nodes
/// `+0x70`, `+0x9c` and `+0x44` join the node list (vtables); slot 262
/// `(e[0x54], v)` fills the position `v = +0x8c` (`+0x34` its `y`,
/// `+0x98` 1.0); the blink count `+0xbc` is `(K - a) * M / D` truncated,
/// `a` slot 333 `(e[0x54])` when bit 0 of the effect's `+0x90` is set,
/// else slot 601 `(e[0x50], &a)`. The model state
/// ([`model_state`](crate::m0070::model_state)) of `[effect, +0x60]`
/// (`+0x38` its `y`), the node group `+0xc8` inserted
/// ([`node_insert`](crate::nodes::node_insert)) with the three nodes;
/// slot 211 `(e[0x54], +0x164)`, the effect flagged 0x10 (`+0x90`) with
/// `+0x4c` the group, slot 664 `(e[0x50])` stored at `+0x2c`; `+0x40` the
/// effect, the state and mode 0. Returns slot 664's `eax`.
#[allow(clippy::too_many_lines)]
pub fn init<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ebp = fr.esp.wrapping_sub(4);
    let (obj, e) = (fr.ecx, fr.edx);
    g.write::<u32>(obj + 8, obj);
    g.write::<u32>(obj + 4, obj);
    g.write::<u32>(obj, w(0x15));
    g.write::<u32>(obj + 0x10, w(0x1c));
    g.write::<u32>(obj + 0x14, obj);
    let tail = g.read::<u32>(w(0x24));
    g.write::<u32>(obj + 8, tail);
    g.write::<u32>(obj + 4, w(0x2e));
    let tail = g.read::<u32>(w(0x33));
    g.write::<u32>(tail + 4, obj);
    let flags = g.read::<u32>(w(0x3f));
    g.write::<u32>(w(0x3f), flags & !1);
    g.write::<u32>(w(0x46), obj);
    // the first node
    let a = obj + 0x70;
    node_init(g, a, w(0x59));
    attach(
        g,
        a,
        w(0x60),
        list_at(img, f, [0x65, 0x6d, 0x75, 0x7d, 0x89, 0x8f]),
    );
    g.write::<u32>(ebp - 0xc, e);
    g.write::<u32>(a, w(0x95));
    let v = obj + 0x8c;
    let s262 = slot_target(g, w(0xa8), 262);
    let m = g.read::<u32>(e + 0x54);
    g.call(s262, 0, &[m, v, v + 4, v + 8]);
    let y = g.read::<u32>(obj + 0x90);
    g.write::<u32>(obj + 0x34, y);
    g.write::<u32>(obj + 0x98, ONE);
    // the second node (its vtable first)
    let c = obj + 0x9c;
    g.write::<u32>(c, w(0xd6));
    node_init(g, c, w(0xe9));
    attach(
        g,
        c,
        w(0xf0),
        list_at(img, f, [0xf5, 0xfd, 0x102, 0x10a, 0x116, 0x11c]),
    );
    g.write::<u32>(obj + 0xc0, 0);
    g.write::<u32>(obj + 0xb8, 0);
    let x = if g.read::<u8>(e + 0x90) & 1 != 0 {
        let t = slot_target(g, w(0x13e), 333);
        let m = g.read::<u32>(e + 0x54);
        let a = st0_f32(g.call(t, 0, &[m]).st0);
        sub(g.read::<f32>(w(0x154)), a)
    } else {
        let t = slot_target(g, w(0x167), 601);
        let m = g.read::<u32>(e + 0x50);
        g.call(t, 0, &[m, ebp - 4]);
        sub(g.read::<f32>(w(0x17a)), g.read::<f32>(ebp - 4))
    };
    let x = div(mul(x, g.read::<f32>(w(0x18a))), g.read::<f32>(w(0x195)));
    g.write::<u32>(obj + 0xbc, crate::s4_summon::trunc(x));
    // the third node
    let d = obj + 0x44;
    node_init(g, d, w(0x1b2));
    attach(
        g,
        d,
        w(0x1b9),
        list_at(img, f, [0x1be, 0x1c6, 0x1cb, 0x1d3, 0x1df, 0x1e8]),
    );
    g.write::<u32>(d, w(0x1f1));
    g.write::<u32>(ebp - 8, e);
    g.write::<u32>(ebp - 4, obj + 0x60);
    crate::m0070::model_state(g, img, img.rel32_target(f + 0x1fc), ebp - 8);
    let y = g.read::<u32>(obj + 0x64);
    g.write::<u32>(obj + 0x38, y);
    let group = ebp - 0x24;
    let grp = obj + 0xc8;
    for (i, v) in (0u32..).zip([grp, w(0x221), w(0x228), a, c, d]) {
        g.write::<u32>(group + 4 * i, v);
    }
    crate::nodes::node_insert(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + 0x233),
            // push ebp; sub esp, 0x24; three pushes: the call's esp
            esp: fr.esp.wrapping_sub(0x38),
            ecx: group,
            ..Frame::default()
        },
    );
    let e = g.read::<u32>(ebp - 0xc);
    let t = slot_target(g, w(0x242), 211);
    let m = g.read::<u32>(e + 0x54);
    g.call(t, 0, &[m, grp + 0x9c]);
    let fl = g.read::<u16>(e + 0x90);
    g.write::<u16>(e + 0x90, fl | 0x10);
    let t = slot_target(g, w(0x25a), 664);
    g.write::<u32>(e + 0x4c, grp);
    let m50 = g.read::<u32>(e + 0x50);
    let r = g.call(t, 0, &[m50, m, grp + 0x9c]);
    g.write::<f32>(obj + 0x2c, st0_f32(r.st0));
    g.write::<u32>(obj + 0x40, e);
    g.write::<u32>(obj + 0x1c, 0);
    g.write::<u32>(obj + 0x18, 0);
    r.eax
}

/// [`init`] (650 bytes, 1 overlay).
pub const INIT: Sig = Sig {
    len: 650,
    hash: 0x4f6c_f64c_bff2_a0d6,
    calls: &[
        (0x1fc, Callee::Sig(&crate::m0070::MODEL_STATE)),
        (0x233, Callee::Sig(&crate::nodes::INSERT)),
    ],
    imports: &[],
};
