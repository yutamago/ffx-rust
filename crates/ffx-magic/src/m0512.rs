//! `magic_0512.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::h6_setup::{ring_join, static_child};
use crate::nodes::node_insert;
use crate::runtime::slot_target;
use crate::sb_sse::{cvt, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// Inserts a model node (0x10003680, internal: `ecx` = a group of words
/// `[node, parent, a, b, c, d]`): as `magic_0482`'s
/// [`insert`](crate::m0482::insert), with `+0x154` = `c` and `+0x150` =
/// `d` (the operands nine bytes further on). Returns slot 325's `eax`.
pub fn insert<G: Game>(g: &mut G, img: &Image, f: u32, group: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let parent = g.read::<u32>(group + 4);
    let node = g.read::<u32>(group);
    g.write::<u16>(node + 0x1a, 0xff);
    g.write::<u32>(node + 0x10, node);
    g.write::<u32>(node + 0xc, node);
    g.write::<u32>(node + 0x14, parent);
    if parent != 0 {
        g.write::<u32>(node + 4, parent);
        let last = g.read::<u32>(parent + 0x10);
        g.write::<u32>(node + 8, last);
        let last = g.read::<u32>(parent + 0x10);
        if last == parent {
            g.write::<u32>(parent + 0xc, node);
        } else {
            g.write::<u32>(last + 4, node);
        }
        let n = g.read::<u16>(parent + 0x18);
        g.write::<u16>(parent + 0x18, n.wrapping_add(1));
        g.write::<u32>(parent + 0x10, node);
    }
    for (src, dst) in [(8u32, 0x13cu32), (0x10, 0x154), (0xc, 0x140), (0x14, 0x150)] {
        let v = g.read::<u32>(group + src);
        g.write::<u32>(node + dst, v);
    }
    g.write::<u32>(node + 0x144, w(0x68));
    g.write::<u32>(node + 0x128, w(0x72));
    g.write::<u8>(node + 0x148, 0);
    g.write::<u32>(node + 0x14c, ONE);
    g.write::<u32>(node + 0xac, 0);
    g.write::<u32>(node + 0xb0, ONE);
    for off in [0xb4, 0xb8, 0x124, 0x120, 0x11c] {
        g.write::<u32>(node + off, 0);
    }
    let m = node + 0x5c;
    let t = slot_target(g, w(0x5e), 235);
    g.call(t, 0, &[m]);
    let t = slot_target(g, w(0xe2), 230);
    g.call(t, 0, &[node + 0xdc, m, m]);
    let t = slot_target(g, w(0xf4), 230);
    g.call(t, 0, &[node + 0x1c, m, node + 0xdc, m, m]);
    let t = slot_target(g, w(0x10d), 325);
    g.call(
        t,
        0,
        &[node + 0x12c, w(0x107), node + 0x1c, m, node + 0xdc, m, m],
    )
    .eax
}

/// [`insert`] (287 bytes, 1 overlay).
pub const INSERT: Sig = Sig {
    len: 287,
    hash: 0xbfb0_02c9_f7ac_6b30,
    calls: &[],
    imports: &[],
};

/// A task (0x100088b0, `(task)`, pushed by 0x1000899a; a frame with
/// locals) on the record `r = task[1]`, by its state `+0x18`:
/// - 0: state 1, a wait `+0x1c` of 8; 1: while the wait runs, it waits;
///   then a particle node ([`particle_node`](crate::sb_nodes::particle_node))
///   from the local record `[r + 0x1f0, LIST, r + 0x20, *G_O, PART, 5]`
///   (`ebp - 0x1c`; the node's vtable `+0x63` first) and state 2;
/// - 2: once bit 2 of `+0x2d8` is set, state 10000 and on;
/// - 10000: the task's node `*task` leaves its ring (linked to itself),
///   state 20000.
///
/// The wait goes down at every exit. Returns the particle node's `eax`,
/// the node's next after the unlink, or the state's remainder.
pub fn task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let state = g.read::<u32>(r + 0x18);
    let eax = match state {
        0 | 1 => {
            if state == 0 {
                g.write::<u32>(r + 0x18, 1);
                g.write::<u32>(r + 0x1c, 8);
            }
            if g.read::<i32>(r + 0x1c) > 0 {
                0
            } else {
                let rec = fr.esp.wrapping_sub(0x20);
                let node = r + 0x1f0;
                g.write::<u32>(rec + 8, r + 0x20);
                let o = g.read::<u32>(w(0x5a));
                g.write::<u32>(rec, node);
                g.write::<u32>(node, w(0x63));
                g.write::<u32>(rec + 4, w(0x6d));
                g.write::<u32>(rec + 0xc, o);
                g.write::<u32>(rec + 0x10, w(0x77));
                g.write::<u32>(rec + 0x14, 5);
                let eax = crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x83), rec);
                let s = g.read::<u32>(r + 0x18);
                g.write::<u32>(r + 0x18, s.wrapping_add(1));
                eax
            }
        }
        2 if g.read::<u8>(r + 0x2d8) & 4 == 0 => 2,
        2 | 0x2710 => {
            g.write::<u32>(r + 0x18, 0x2710);
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
        _ => state,
    };
    let n = g.read::<u32>(r + 0x1c);
    g.write::<u32>(r + 0x1c, n.wrapping_sub(1));
    Ret::eax(eax)
}

/// [`task`] (194 bytes, 1 overlay).
pub const TASK: Sig = Sig {
    len: 194,
    hash: 0x5b27_20e2_f49f_5de0,
    calls: &[(0x83, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE))],
    imports: &[],
};

/// Sets up an effect object (0x10007830, internal: `ecx` = the object,
/// `edx` a word, one stack word `n`): the object joins the DLL's ring
/// (vtable `+0x16`, step `+0x1d`, the ring's last `*G_R` and sentinel,
/// the ring's flag bit 0 cleared); three children of the DLL's node list:
/// `b = obj + 0x170` (handlers `+0x9e` after), `c = obj + 0x1c8`
/// (handlers `+0xd2` first) and `d = obj + 0x19c` (handlers `+0x19d`);
/// `+0x18c..` = `(0, 0, 0, 1)`, `+0x1e4/+0x1e8` 0, `+0x1ec` 0x4000,
/// `+0x1b8..` = `(1, 1, K1 - n * K2, 0.2)`; the node insert
/// ([`node_insert`]) of the group `[obj + 0x24, LIST, edx, b, c, d]` on
/// the frame (`ebp - 0x20`); `+0x20` = `10 - 10 n`, `+0x18` 0. Returns
/// `10 - 10 n`.
pub fn setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.ecx;
    let n = fr.arg(g, 0);
    ring_join(g, img, f, obj, [0x16, 0x1d, 0x25, 0x2f, 0x34, 0x43, 0x4a]);
    let b = obj + 0x170;
    static_child(
        g,
        img,
        f,
        b,
        [0x5d, 0x64, 0x69, 0x71, 0x76, 0x7e, 0x8a, 0x98],
    );
    g.write::<u32>(b, w(0x9e));
    for (o, v) in [(0x18cu32, 0u32), (0x190, 0), (0x194, 0), (0x198, ONE)] {
        g.write::<u32>(obj + o, v);
    }
    let c = obj + 0x1c8;
    g.write::<u32>(c, w(0xd2));
    static_child(
        g,
        img,
        f,
        c,
        [0xe5, 0xec, 0xf1, 0xf9, 0xfe, 0x106, 0x112, 0x118],
    );
    for (o, v) in [(0x1e4u32, 0u32), (0x1e8, 0), (0x1ec, 0x4000)] {
        g.write::<u32>(obj + o, v);
    }
    let d = obj + 0x19c;
    static_child(
        g,
        img,
        f,
        d,
        [0x14f, 0x156, 0x15b, 0x163, 0x168, 0x170, 0x17f, 0x18c],
    );
    g.write::<u32>(d, w(0x19d));
    let k1 = g.read::<f32>(w(0x92));
    let k2 = g.read::<f32>(w(0x194));
    let x = sub(k1, mul(cvt(n), k2));
    let ebp = fr.esp.wrapping_sub(4);
    let group = ebp.wrapping_sub(0x20);
    for (i, v) in (0u32..).zip([obj + 0x24, w(0x1da), fr.edx, b, c, d]) {
        g.write::<u32>(group + 4 * i, v);
    }
    for (o, v) in [
        (0x1b8u32, ONE),
        (0x1bc, ONE),
        (0x1c0, x.to_bits()),
        (0x1c4, 0x3e4c_cccd),
    ] {
        g.write::<u32>(obj + o, v);
    }
    node_insert(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + 0x1e8),
            // push ebp; sub esp, 0x20; three pushes: the call's esp
            esp: fr.esp.wrapping_sub(0x34),
            ecx: group,
            ..Frame::default()
        },
    );
    let r = 10u32.wrapping_sub(n.wrapping_mul(10));
    g.write::<u32>(obj + 0x20, r);
    g.write::<u32>(obj + 0x18, 0);
    r
}

/// [`setup`] (521 bytes, 1 overlay).
pub const SETUP: Sig = Sig {
    len: 521,
    hash: 0xd0c7_794f_98a4_14cb,
    calls: &[(0x1e8, Callee::Sig(&crate::nodes::INSERT))],
    imports: &[],
};

/// Sets up an effect object from a source object (0x10008670, internal:
/// `ecx` = the object, `edx` = the source, one stack word `n`): the
/// object joins the DLL's ring ([`ring_join`]); children of the DLL's
/// node list `b = obj + 0x134` (handlers `+0xa9`), `c = obj + 0x160`
/// (`+0x109`) and `d = obj + 0x18c` (`+0x14e`, set first); slot 325
/// copies the source's `+0xcc` to `+0x150` and `[src + 0x140] + 0x1c` to
/// `+0x17c`; `+0x188` scaled by `K`; `+0x1a8..+0x1b0` from the source's
/// `[+0x144]` block (`+0x1c` plus 0x8000, `+0x20`, `+0x24`); the node
/// insert ([`node_insert`]) of `[obj + 0x1b8, LIST, NAME, b, d, c]` on the
/// frame (`ebp - 0x20`); `+0x30` the source, `+0x2c` slot 719 `(NAME2,
/// 9, 0)`, `+0x18` 0 and `+0x20` = `15 - 15 n`. Returns `15 - 15 n`.
pub fn setup_from<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.ecx;
    let src = fr.edx;
    let n = fr.arg(g, 0);
    ring_join(g, img, f, obj, [0x13, 0x1a, 0x22, 0x2c, 0x31, 0x40, 0x47]);
    let b = obj + 0x134;
    static_child(
        g,
        img,
        f,
        b,
        [0x5a, 0x61, 0x66, 0x6e, 0x76, 0x7e, 0x8a, 0xa3],
    );
    let t = slot_target(g, w(0x9d), 325);
    g.write::<u32>(b, w(0xa9));
    g.call(t, 0, &[obj + 0x150, src + 0xcc]);
    let c = obj + 0x160;
    static_child(
        g,
        img,
        f,
        c,
        [0xcd, 0xd4, 0xd9, 0xe1, 0xe6, 0xee, 0xfa, 0x103],
    );
    g.write::<u32>(c, w(0x109));
    let v = g.read::<u32>(src + 0x140) + 0x1c;
    let t = slot_target(g, w(0x11f), 325);
    g.call(t, 0, &[obj + 0x17c, v]);
    let x = mul(g.read::<f32>(obj + 0x188), g.read::<f32>(w(0x137)));
    g.write::<f32>(obj + 0x188, x);
    let d = obj + 0x18c;
    g.write::<u32>(d, w(0x14e));
    static_child(
        g,
        img,
        f,
        d,
        [0x161, 0x168, 0x16d, 0x175, 0x17a, 0x182, 0x18e, 0x197],
    );
    for (o, k, plus) in [
        (0x1a8u32, 0x1cu32, 0x8000u32),
        (0x1ac, 0x20, 0),
        (0x1b0, 0x24, 0),
    ] {
        let blk = g.read::<u32>(src + 0x144);
        let v = g.read::<u32>(blk + k).wrapping_add(plus);
        g.write::<u32>(obj + o, v);
    }
    let ebp = fr.esp.wrapping_sub(4);
    let group = ebp.wrapping_sub(0x20);
    for (i, v) in (0u32..).zip([obj + 0x1b8, w(0x1cd), w(0x1e6), b, d, c]) {
        g.write::<u32>(group + 4 * i, v);
    }
    node_insert(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + 0x1f1),
            // push ebp; sub esp, 0x20; three pushes: the call's esp
            esp: fr.esp.wrapping_sub(0x34),
            ecx: group,
            ..Frame::default()
        },
    );
    g.write::<u32>(obj + 0x30, src);
    let t = slot_target(g, w(0x1fc), 719);
    let h = g.call(t, 0, &[w(0x20b), 9, 0]).eax;
    g.write::<u32>(obj + 0x2c, h);
    let r = 15u32.wrapping_sub(n.wrapping_mul(15));
    g.write::<u32>(obj + 0x18, 0);
    g.write::<u32>(obj + 0x20, r);
    r
}

/// [`setup_from`] (568 bytes, 1 overlay).
pub const SETUP_FROM: Sig = Sig {
    len: 568,
    hash: 0x7fd2_4105_8f9b_1e3a,
    calls: &[(0x1f1, Callee::Sig(&crate::nodes::INSERT))],
    imports: &[],
};

/// Sets up the effect's ground object (0x10008980, internal: `ecx` = the
/// object, `edx` a word): the object joins the DLL's ring
/// ([`ring_join`]); children of the DLL's node list `b = obj + 0x16c`
/// (handlers `+0x95` after), `c = obj + 0x1c4` (`+0xc9` first) and `d =
/// obj + 0x198` (`+0x17f` after); `+0x188..` = `(0, -15, 0, 1)`,
/// `+0x1e0..+0x1e8` 0, `+0x1b4..` = `(1, 1, 1, 300)`; the node insert
/// ([`node_insert`]) of `[obj + 0x20, LIST, edx, b, c, d]` on the frame
/// (`ebp - 0x18`); `+0x18` 0. Returns the insert's `eax` (`edx`).
pub fn ground<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.ecx;
    ring_join(g, img, f, obj, [0x13, 0x1a, 0x22, 0x2c, 0x31, 0x40, 0x47]);
    let b = obj + 0x16c;
    static_child(
        g,
        img,
        f,
        b,
        [0x5a, 0x61, 0x66, 0x6e, 0x75, 0x7d, 0x89, 0x8f],
    );
    g.write::<u32>(b, w(0x95));
    for (o, v) in [
        (0x188u32, 0u32),
        (0x18c, 0xc170_0000),
        (0x190, 0),
        (0x194, ONE),
    ] {
        g.write::<u32>(obj + o, v);
    }
    let c = obj + 0x1c4;
    g.write::<u32>(c, w(0xc9));
    static_child(
        g,
        img,
        f,
        c,
        [0xdc, 0xe3, 0xe8, 0xf0, 0xf5, 0xfd, 0x109, 0x10f],
    );
    for o in [0x1e0u32, 0x1e4, 0x1e8] {
        g.write::<u32>(obj + o, 0);
    }
    let d = obj + 0x198;
    static_child(
        g,
        img,
        f,
        d,
        [0x146, 0x14d, 0x152, 0x15a, 0x15f, 0x167, 0x173, 0x179],
    );
    g.write::<u32>(d, w(0x17f));
    for (o, v) in [
        (0x1b4u32, ONE),
        (0x1b8, ONE),
        (0x1bc, ONE),
        (0x1c0, 0x4396_0000),
    ] {
        g.write::<u32>(obj + o, v);
    }
    let ebp = fr.esp.wrapping_sub(4);
    let group = ebp.wrapping_sub(0x18);
    for (i, v) in (0u32..).zip([obj + 0x20, w(0x1ba), fr.edx, b, c, d]) {
        g.write::<u32>(group + 4 * i, v);
    }
    let r = node_insert(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + 0x1c8),
            // push ebp; sub esp, 0x1c; three pushes: the call's esp
            esp: fr.esp.wrapping_sub(0x30),
            ecx: group,
            ..Frame::default()
        },
    );
    g.write::<u32>(obj + 0x18, 0);
    r.eax
}

/// [`ground`] (474 bytes, 1 overlay).
pub const GROUND: Sig = Sig {
    len: 474,
    hash: 0x35ec_8e3b_e899_5324,
    calls: &[(0x1c8, Callee::Sig(&crate::nodes::INSERT))],
    imports: &[],
};

/// [`crate::s3_nodes::particle_node_with`] called at `f + call` with `ecx`
/// = `node`, the stack words `(a, _, _, index)` (the two in the middle
/// never written); its `esp` is `m - 0x1c` (`m` the aligned frame).
fn node_with<G: Game>(g: &mut G, img: &Image, call: u32, m: u32, node: u32, a: u32, index: u32) {
    let sp = m.wrapping_sub(0x1c);
    g.write::<u32>(sp + 4, a);
    g.write::<u32>(sp + 0x10, index);
    let callee = Frame {
        at: img.rel32_target(call),
        esp: sp,
        ecx: node,
        ..Frame::default()
    };
    crate::s3_nodes::particle_node_with(g, img, &callee);
}

/// The particles' task (0x10008b60, cdecl `(task)`, `r = task[1]`, its
/// step `+0x18`, the wait `+0x20`; the steps from the jump table at
/// operand `+0x37`):
/// - 0: step 1 and on to 1;
/// - 1: once the wait is down: with no `+0x1c`, one particle node
///   (`r + 0x360`, `(r + 0x2c, 3)`); otherwise two (`r + 0x360`, `(r +
///   0x2c, 4)`, and `r + 0x460`, `(r + 0x184, 6)`), `+0x54c` 1 and the
///   word `+0x536` = `trunc(max(K2 - r[0x28] / K1, 0) * K3)`; then the
///   next step and the wait `r[0x24] + 30`;
/// - 2: once the wait is down, the next step;
/// - 3: step 10000, then as 10000;
/// - 10000: the task's node `*task` leaves its ring, step 20000.
///
/// The wait goes down at every exit. Returns `r[0x24] + 30` after the
/// nodes, the node's next after the unlink, or the step.
pub fn particles_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{div, mul, sub};
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, op: u32| g.read::<f32>(w(op));
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    // push ebp; and esp, -8; push esi; push edi
    let m = fr.esp.wrapping_sub(4) & !7;
    let step = |g: &mut G| {
        let s = g.read::<u32>(r + 0x18);
        g.write::<u32>(r + 0x18, s.wrapping_add(1));
    };
    let mut state = g.read::<u32>(r + 0x18);
    if state == 3 {
        g.write::<u32>(r + 0x18, 0x2710);
        state = 0x2710;
    }
    let eax = match state {
        0 | 1 => {
            if state == 0 {
                step(g);
            }
            if g.read::<i32>(r + 0x20) > 0 {
                state
            } else {
                if g.read::<u32>(r + 0x1c) == 0 {
                    node_with(g, img, f + 0xea, m, r + 0x360, r + 0x2c, 3);
                } else {
                    node_with(g, img, f + 0x6c, m, r + 0x360, r + 0x2c, 4);
                    node_with(g, img, f + 0x86, m, r + 0x460, r + 0x184, 6);
                    g.write::<u32>(r + 0x54c, 1);
                    let x = sub(k(g, 0xa5), div(g.read::<f32>(r + 0x28), k(g, 0x9d)));
                    // comiss 0, x; jbe: below 0 is 0 (unordered stays)
                    let x = if 0.0 > x { 0.0 } else { x };
                    let v = crate::s4_summon::trunc(mul(x, k(g, 0xbf)));
                    g.write::<u16>(r + 0x536, v as u16);
                }
                let eax = g.read::<u32>(r + 0x24).wrapping_add(0x1e);
                step(g);
                g.write::<u32>(r + 0x20, eax);
                eax
            }
        }
        2 => {
            if g.read::<i32>(r + 0x20) <= 0 {
                step(g);
            }
            2
        }
        0x2710 => {
            let next = crate::h6_setup::unlink_task(g, task);
            g.write::<u32>(r + 0x18, 0x4e20);
            next
        }
        _ => state,
    };
    let n = g.read::<u32>(r + 0x20);
    g.write::<u32>(r + 0x20, n.wrapping_sub(1));
    Ret::eax(eax)
}

/// [`particles_task`] (322 bytes, 1 overlay).
pub const PARTICLES_TASK: Sig = Sig {
    len: 322,
    hash: 0xe730_d3da_a08c_ffe6,
    calls: &[
        (0x6c, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x86, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xea, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
    ],
    imports: &[],
};
