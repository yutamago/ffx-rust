//! `magic_0573.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::h6_setup::{ring_join, static_child, unlink_task};
use crate::l3_common::{ListAt, attach, node_init};
use crate::m0018::st0_f32;
use crate::runtime::slot_target;
use crate::s4_summon::trunc;
use crate::sb_sse::{add, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// The list operands of one node append ([`attach`]).
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

/// Shows an object's model and its parts (0x100035f0, internal: `ecx` =
/// the object): slot 1 `(obj[0x54])`; unless bit 0 of `+0x90` is set,
/// slot 219 `(obj[0x50], k)` for the parts 0 and 1 and slot 1 `(part)`
/// for each one that exists. Returns the last call's `eax`.
pub fn show<G: Game>(g: &mut G, img: &Image, f: u32, obj: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(0x1), 1);
    let m = g.read::<u32>(obj + 0x54);
    let mut eax = g.call(t, 0, &[m]).eax;
    if g.read::<u8>(obj + 0x90) & 1 != 0 {
        return eax;
    }
    for (k, t219, t1) in [(0u32, 0x1du32, 0x37u32), (1, 0x44, 0x5e)] {
        let t = slot_target(g, w(t219), 219);
        let o = g.read::<u32>(obj + 0x50);
        eax = g.call(t, 0, &[o, k]).eax;
        if eax != 0 {
            let t = slot_target(g, w(t1), 1);
            eax = g.call(t, 0, &[eax]).eax;
        }
    }
    eax
}

/// [`show`] (108 bytes, 1 overlay).
pub const SHOW: Sig = Sig {
    len: 108,
    hash: 0xdb2f_bac6_2cb0_087c,
    calls: &[],
    imports: &[],
};

/// The growing object's task (0x100070d0, `(task)`, pushed by the
/// set-up [`init`]) on the record `r = task[1]`: state 0 clears the size
/// `+0x24` and becomes 1, which grows it by `K` until it reaches `L`
/// (then state 10000); state 10000 waits for bit 2 of `+0x288` and takes
/// the task's node off its ring (state 20000). Every state then sets the
/// scale `+0x50` = `(size + base) * M * N`. Returns the unlinked node's
/// next, or the state's remainder (0 in states 0 and 1).
pub fn task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let state = g.read::<u32>(r + 0x18);
    let eax = match state {
        0 | 1 => {
            if state == 0 {
                g.write::<u32>(r + 0x24, 0);
                g.write::<u32>(r + 0x18, 1);
            }
            let x = add(g.read::<f32>(r + 0x24), g.read::<f32>(w(0x5f)));
            let limit = g.read::<f32>(w(0x66));
            g.write::<f32>(r + 0x24, x);
            // comiss x, L; jb: below or unordered
            if !(x < limit || x.is_nan() || limit.is_nan()) {
                g.write::<u32>(r + 0x18, 0x2710);
            }
            0
        }
        0x2710 if g.read::<u8>(r + 0x288) & 4 != 0 => {
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
        s => s.wrapping_sub(0x2710),
    };
    let s = add(g.read::<f32>(r + 0x24), g.read::<f32>(r + 0x20));
    let s = mul(mul(s, g.read::<f32>(w(0x86))), g.read::<f32>(w(0x8e)));
    g.write::<f32>(r + 0x50, s);
    Ret::eax(eax)
}

/// [`task`] (154 bytes, 1 overlay).
pub const TASK: Sig = Sig {
    len: 154,
    hash: 0x3f15_8298_8f4d_49ed,
    calls: &[],
    imports: &[],
};

/// Adds a node to the node list (0x10007e00, internal: `ecx` = the
/// node, cdecl `(owner)`): its vtable (`+0x05`), its own children list
/// empty, appended to the list ([`attach`]); `+0x28` 1.0, `+0x2c` the
/// owner, `+0x30` 17. Returns the owner.
pub fn add_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let node = fr.ecx;
    g.write::<u32>(node, w(0x5));
    node_init(g, node, w(0x18));
    attach(
        g,
        node,
        w(0x1f),
        list_at(img, f, [0x24, 0x2c, 0x31, 0x39, 0x45, 0x4e]),
    );
    let owner = fr.arg(g, 0);
    g.write::<u32>(node + 0x28, ONE);
    g.write::<u32>(node + 0x2c, owner);
    g.write::<u32>(node + 0x30, 0x11);
    owner
}

/// [`add_node`] (101 bytes, 1 overlay).
pub const ADD_NODE: Sig = Sig {
    len: 101,
    hash: 0x5b8e_79b7_ad3d_50fb,
    calls: &[],
    imports: &[],
};

/// Sets up the growing object (0x10007170, internal: `ecx` = the
/// object, `edx` = the effect, cdecl `(a)`; a frame with locals): a task
/// (vtable `+0x12`, callback [`task`] at `+0x19`) appended to the task
/// ring; its node `+0x28` joins the node list (vtable `+0x92`); the
/// scales `+0x44..+0x4c` 1.0 (`+0x50` 0); the node group `+0x54`
/// inserted ([`node_insert`](crate::nodes::node_insert)) from the local
/// record `[+0x54, LIST, effect, a, DATA, node]` (`ebp - 0x1c`); the
/// particle node `+0x1a0` (vtable `+0xe7`) from `[+0x1a0, LIST2, +0x54,
/// *G_O, PART, 0]` ([`particle_node`](crate::sb_nodes::particle_node));
/// `+0x18..+0x20` 0. Returns the particle node's `eax`.
pub fn init<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ebp = fr.esp.wrapping_sub(4);
    let (obj, e) = (fr.ecx, fr.edx);
    g.write::<u32>(obj + 8, obj);
    g.write::<u32>(obj + 4, obj);
    g.write::<u32>(obj, w(0x12));
    g.write::<u32>(obj + 0x10, w(0x19));
    g.write::<u32>(obj + 0x14, obj);
    let tail = g.read::<u32>(w(0x21));
    g.write::<u32>(obj + 8, tail);
    g.write::<u32>(obj + 4, w(0x2b));
    let tail = g.read::<u32>(w(0x30));
    g.write::<u32>(tail + 4, obj);
    let fl = g.read::<u32>(w(0x3c));
    g.write::<u32>(w(0x3c), fl & !1);
    g.write::<u32>(w(0x43), obj);
    let c = obj + 0x28;
    node_init(g, c, w(0x56));
    attach(
        g,
        c,
        w(0x5d),
        list_at(img, f, [0x62, 0x6a, 0x6f, 0x77, 0x83, 0x8c]),
    );
    let a = fr.arg(g, 0);
    g.write::<u32>(c, w(0x92));
    let rec = ebp - 0x1c;
    let grp = obj + 0x54;
    for off in [0x44, 0x48, 0x4c] {
        g.write::<u32>(obj + off, ONE);
    }
    g.write::<u32>(obj + 0x50, 0);
    for (i, v) in (0u32..).zip([grp, w(0xc1), e, a, w(0xce), c]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    crate::nodes::node_insert(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + 0xd3),
            // push ebp; sub esp, 0x1c; two pushes: the call's esp
            esp: fr.esp.wrapping_sub(0x2c),
            ecx: rec,
            ..Frame::default()
        },
    );
    let o = g.read::<u32>(w(0xd8));
    let p = obj + 0x1a0;
    g.write::<u32>(p, w(0xe7));
    for (i, v) in (0u32..).zip([p, w(0xf1), grp, o, w(0xfe), 0]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    let eax = crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x10a), rec);
    for off in [0x20, 0x1c, 0x18] {
        g.write::<u32>(obj + off, 0);
    }
    eax
}

/// [`init`] (297 bytes, 1 overlay).
pub const INIT: Sig = Sig {
    len: 297,
    hash: 0x87dc_9bb7_06b6_c9be,
    calls: &[
        (0xd3, Callee::Sig(&crate::nodes::INSERT)),
        (0x10a, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
    ],
    imports: &[],
};

/// A summon node's refresh (0x10002d60, internal, `ecx` = the cursor
/// `[node, .., flag, mode]`): the walk of the node's children
/// ([`walk`](crate::nodes::walk)); when it ran, slot 275 `(node[0x20])`
/// and, with bit 0 of `node[0x24]`, its model `node[0x1c]` leaves its
/// parent ([`leave`](crate::sa_misc::leave)), the model is set up
/// ([`model_setup`](crate::sa_summon::model_setup), the local cursor
/// `[model]` at `ebp - 4`) and the node leaves its parent. Returns the
/// last call's `eax`.
pub fn refresh<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let cur = fr.ecx;
    let (eax, done) = crate::nodes::walk(g, cur, cur, fr.eax);
    let Some(node) = done else {
        return Ret::eax(eax);
    };
    let t = slot_target(g, w(0x4c), 275);
    let h = g.read::<u32>(node + 0x20);
    let eax = g.call(t, 0, &[h]).eax;
    if g.read::<u8>(node + 0x24) & 1 == 0 {
        return Ret::eax(eax);
    }
    let model = g.read::<u32>(node + 0x1c);
    crate::sa_misc::leave(g, model, eax);
    let model = g.read::<u32>(node + 0x1c);
    let local = fr.esp.wrapping_sub(8);
    g.write::<u32>(local, model);
    let r = crate::sa_summon::model_setup(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + 0x76),
            // push ebp; push ecx; push ebx; push esi; call
            esp: fr.esp.wrapping_sub(0x14),
            eax: model,
            ecx: local,
            ..Frame::default()
        },
    );
    Ret::eax(crate::sa_misc::leave(g, node, r.eax))
}

/// [`refresh`] (135 bytes, 1 overlay).
pub const REFRESH: Sig = Sig {
    len: 135,
    hash: 0x9bbe_9abe_42f8_8a17,
    calls: &[
        (0x68, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x76, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
        (0x7d, Callee::Sig(&crate::sa_misc::LEAVE)),
    ],
    imports: &[],
};

/// The summon's motion task (0x100072a0, `(task)`, a task step) on the
/// record `r = task[1]`, its object `o = r[0x2c]` and its motion set `m
/// = r[0x24]` (five words each in the DLL's table, operands
/// `+0x6c`..`+0x18f`); each motion is slot 228 `(o[0x50], id)` and the
/// next state:
/// - 0: motion 0; 1: once the motion is done (slot 411 `(o[0x50])`),
///   motion 1 and a wait `+0x1c` of 69; 2: when the wait is out, state 10
///   at once; 10: motion 2; 11: once done, motion 3 and a wait of 30;
/// - 12: when the wait is out, state 10000 at once; 10000: motion 4;
///   10001: once done, the object leaves its model
///   ([`object_to_node`](crate::sa_misc::object_to_node)), slot 213
///   `(o[0x50], 0)` and the task's node leaves its ring, state 20000.
///
/// Every exit counts `+0x20` up; from 40 on the size `+0x19c` shrinks by
/// `K` down to `+0x28` while above it. The wait goes down at every exit.
/// Returns the count before the step plus 1.
pub fn motion_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let model = |g: &G| {
        let o = g.read::<u32>(r + 0x2c);
        g.read::<u32>(o + 0x50)
    };
    let done = |g: &mut G, op: u32| {
        let t = slot_target(g, w(op), 411);
        let m = model(g);
        g.call(t, 0, &[m]).eax != 0
    };
    // slot 228 with the set's motion from the table at operand `tab`
    let play = |g: &mut G, tab: u32, op: u32| {
        let set = g.read::<u32>(r + 0x24);
        let id = g.read::<u32>(w(tab).wrapping_add(set.wrapping_mul(20)));
        let m = model(g);
        let t = slot_target(g, w(op), 228);
        g.call(t, 0, &[m, id]);
    };
    let next = |g: &mut G| {
        let s = g.read::<u32>(r + 0x18);
        g.write::<u32>(r + 0x18, s.wrapping_add(1));
    };
    loop {
        let state = g.read::<u32>(r + 0x18);
        match state {
            0 => {
                play(g, 0x6c, 0x19a);
                next(g);
            }
            1 if done(g, 0x7c) => {
                play(g, 0x9c, 0xa7);
                g.write::<u32>(r + 0x1c, 0x45);
                next(g);
            }
            2 if g.read::<i32>(r + 0x1c) <= 0 => {
                g.write::<u32>(r + 0x18, 10);
                continue;
            }
            10 => {
                play(g, 0xc8, 0x19a);
                next(g);
            }
            11 if done(g, 0xd8) => {
                play(g, 0xf8, 0x103);
                g.write::<u32>(r + 0x1c, 0x1e);
                next(g);
            }
            12 if g.read::<i32>(r + 0x1c) <= 0 => {
                g.write::<u32>(r + 0x18, 0x2710);
                continue;
            }
            0x2710 => {
                play(g, 0x18f, 0x19a);
                next(g);
            }
            0x2711 if done(g, 0x130) => {
                let o = g.read::<u32>(r + 0x2c);
                crate::sa_misc::object_to_node(g, img, img.rel32_target(f + 0x147), o, 0);
                let t = slot_target(g, w(0x154), 213);
                let m = model(g);
                g.call(t, 0, &[m, 0]);
                unlink_task(g, task);
                g.write::<u32>(r + 0x18, 0x4e20);
            }
            _ => {}
        }
        break;
    }
    let n = g.read::<u32>(r + 0x20);
    g.write::<u32>(r + 0x20, n.wrapping_add(1));
    if n.cast_signed() >= 0x28 {
        let x = g.read::<f32>(r + 0x19c);
        if x > g.read::<f32>(r + 0x28) {
            let x = sub(x, g.read::<f32>(w(0x1cc)));
            g.write::<f32>(r + 0x19c, x);
            let lo = g.read::<f32>(r + 0x28);
            // comiss + jb: the floor only when it is not below (ordered)
            if lo >= x {
                g.write::<f32>(r + 0x19c, lo);
            }
        }
    }
    let wt = g.read::<u32>(r + 0x1c);
    g.write::<u32>(r + 0x1c, wt.wrapping_sub(1));
    Ret::eax(n.wrapping_add(1))
}

/// [`motion_task`] (531 bytes, 1 overlay).
pub const MOTION_TASK: Sig = Sig {
    len: 531,
    hash: 0xabe7_6dd1_7540_8b6f,
    calls: &[(0x147, Callee::Sig(&crate::sa_misc::OBJECT_TO_NODE))],
    imports: &[],
};

/// Sets up the summon object for its motion task (0x100074c0, internal:
/// `ecx` = the object, `edx` = the source object): the object joins the
/// DLL's ring ([`ring_join`], its step the motion task); children `c =
/// obj + 0x17c` (handlers `+0x98`), `d = obj + 0x1d4` (`+0xf1`, set
/// first) and `e = obj + 0x1a8` (`+0x212`) of the DLL's node list; slot
/// 262 `(src[0x54], obj + 0x198, obj + 0x19c, obj + 0x1a0)` the source's
/// point (`+0x28` its height, `+0x19c` raised by `K`, `+0x1a4` 1); the
/// size `s` (slot 333 `(src[0x54])` with bit 0 of `src[0x90]`, else slot
/// 601 `(src[0x50], &s)`, through the frame word `ebp - 4`) gives `+0x1f4`
/// = `(K1 - s) * K2 / K3` truncated; the model state
/// ([`model_state`](crate::m0070::model_state)) of `[src, obj + 0x1c4]`
/// (`ebp - 8`), the node insert ([`node_insert`](crate::nodes::node_insert))
/// of `[obj + 0x30, LIST, NAME, c, d, e]` (`ebp - 0x24`); slot 211
/// `(src[0x54], obj + 0xcc)`, the source's flags `|= 0x10`, `src[0x4c]` =
/// `obj + 0x30`, slot 415 `(src[0x50])` the kind; `+0x2c` the source,
/// `+0x20`/`+0x18` 0, `+0x24` = kind is not 0xc9. Returns slot 415's
/// `eax`.
pub fn motion_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, op: u32| g.read::<f32>(w(op));
    let obj = fr.ecx;
    let src = fr.edx;
    ring_join(g, img, f, obj, [0x15, 0x1c, 0x24, 0x2e, 0x33, 0x42, 0x49]);
    let c = obj + 0x17c;
    static_child(
        g,
        img,
        f,
        c,
        [0x5c, 0x63, 0x68, 0x70, 0x78, 0x80, 0x8c, 0x92],
    );
    g.write::<u32>(c, w(0x98));
    let t = slot_target(g, w(0xab), 262);
    let h = g.read::<u32>(src + 0x54);
    g.call(t, 0, &[h, obj + 0x198, obj + 0x19c, obj + 0x1a0]);
    let y = g.read::<u32>(obj + 0x19c);
    g.write::<u32>(obj + 0x28, y);
    let x = add(g.read::<f32>(obj + 0x19c), k(g, 0xd0));
    g.write::<u32>(obj + 0x1a4, 0x3f80_0000);
    g.write::<f32>(obj + 0x19c, x);
    let d = obj + 0x1d4;
    g.write::<u32>(d, w(0xf1));
    static_child(
        g,
        img,
        f,
        d,
        [0x104, 0x10b, 0x110, 0x118, 0x11d, 0x125, 0x131, 0x137],
    );
    g.write::<u32>(obj + 0x1f8, 0);
    g.write::<u32>(obj + 0x1f0, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let word = ebp.wrapping_sub(4);
    let size = if g.read::<u8>(src + 0x90) & 1 != 0 {
        let t = slot_target(g, w(0x159), 333);
        let h = g.read::<u32>(src + 0x54);
        let v = st0_f32(g.call(t, 0, &[h]).st0);
        g.write::<f32>(word, v);
        sub(k(g, 0x16f), g.read::<f32>(word))
    } else {
        let t = slot_target(g, w(0x182), 601);
        let h = g.read::<u32>(src + 0x50);
        g.call(t, 0, &[h, word]);
        sub(k(g, 0x195), g.read::<f32>(word))
    };
    let n = trunc(div(mul(size, k(g, 0x1a5)), k(g, 0x1b3)));
    g.write::<u32>(obj + 0x1f4, n);
    let e = obj + 0x1a8;
    static_child(
        g,
        img,
        f,
        e,
        [0x1d0, 0x1d7, 0x1dc, 0x1e4, 0x1e9, 0x1f1, 0x1fd, 0x209],
    );
    g.write::<u32>(e, w(0x212));
    let rec = ebp.wrapping_sub(8);
    g.write::<u32>(rec, src);
    g.write::<u32>(rec + 4, obj + 0x1c4);
    crate::m0070::model_state(g, img, img.rel32_target(f + 0x21d), rec);
    let group = ebp.wrapping_sub(0x24);
    for (i, v) in (0u32..).zip([obj + 0x30, w(0x23c), w(0x243), c, d, e]) {
        g.write::<u32>(group + 4 * i, v);
    }
    crate::nodes::node_insert(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + 0x24e),
            // push ebp; sub esp, 0x24; three pushes: the call's esp
            esp: fr.esp.wrapping_sub(0x38),
            ecx: group,
            ..Frame::default()
        },
    );
    let t = slot_target(g, w(0x25d), 211);
    let h = g.read::<u32>(src + 0x54);
    let mut stack = vec![h, obj + 0xcc];
    g.call(t, 0, &stack);
    let fl = g.read::<u16>(src + 0x90);
    g.write::<u16>(src + 0x90, fl | 0x10);
    let t = slot_target(g, w(0x275), 415);
    g.write::<u32>(src + 0x4c, obj + 0x30);
    let m = g.read::<u32>(src + 0x50);
    stack.insert(0, m);
    // call width: 18 at most (the earlier calls' words)
    let kind = g.call(t, 0, &stack).eax;
    g.write::<u32>(obj + 0x2c, src);
    g.write::<u32>(obj + 0x20, 0);
    g.write::<u32>(obj + 0x18, 0);
    g.write::<u32>(obj + 0x24, u32::from(kind != 0xc9));
    kind
}

/// [`motion_setup`] (687 bytes, 1 overlay).
pub const MOTION_SETUP: Sig = Sig {
    len: 687,
    hash: 0x49ca_f9fc_0166_701b,
    calls: &[
        (0x21d, Callee::Sig(&crate::m0070::MODEL_STATE)),
        (0x24e, Callee::Sig(&crate::nodes::INSERT)),
    ],
    imports: &[],
};
