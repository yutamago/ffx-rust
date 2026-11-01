//! `magic_0573.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::h6_setup::unlink_task;
use crate::l3_common::{ListAt, attach, node_init};
use crate::runtime::slot_target;
use crate::sb_sse::{add, mul, sub};
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
