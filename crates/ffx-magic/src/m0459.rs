//! `magic_0459.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::sb_sse::add;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A node appended to the DLL's node list (operands: the list head `+h`
/// compared with the tail `*G_TAIL`, `+t1`/`+t2`, the first-node global
/// `+first`, the count `+n`, the tail written `+t3`, the vtable `+vt`;
/// the parent links `+p1`/`+p2`): pass byte `+0x1a` 0xff, an empty child
/// list, the parent `+0x14`/`+0x4`, the old tail `+0x8`, linked after it
/// (or first), the count up, the tail and the vtable set.
fn append<G: Game>(g: &mut G, w: &impl Fn(u32) -> u32, node: u32, ops: [u32; 8]) {
    let [p1, p2, t1, t2, h, first, n, t3] = ops;
    g.write::<u32>(node + 0x10, node);
    g.write::<u32>(node + 0xc, node);
    g.write::<u32>(node + 0x14, w(p1));
    g.write::<u32>(node + 4, w(p2));
    let tail = g.read::<u32>(w(t1));
    g.write::<u32>(node + 8, tail);
    let tail = g.read::<u32>(w(t2));
    if tail == w(h) {
        g.write::<u32>(w(first), node);
    } else {
        g.write::<u32>(tail + 4, node);
    }
    let c = g.read::<u16>(w(n)).wrapping_add(1);
    g.write::<u16>(w(n), c);
    g.write::<u32>(w(t3), node);
}

/// The model task's step (0x10006960, a task callback, cdecl `(task)`;
/// an 8-aligned frame; the state block `s = task[+4]`, its step `s[+0x18]`,
/// its frame `s[+0x28]`, counted up at every return). Each step first
/// moves the two scroll words `s[+0x1c]`, `s[+0x20]` by `K`. Step 0: the
/// model nodes ([`model_nodes`](crate::s3_summon::model_nodes), a record
/// on the frame: the node, its parent, the actor `*G_A`, 0, 0), the colour
/// node ([`colour_node`](crate::s3_misc::colour_node)), seven colour
/// words 0.5 and the camera move ([`move_start`](crate::sb_misc::move_start));
/// step 10: the model index 0 and the frame 100; step 11 (from frame 16):
/// model `i`'s nodes: `N` (with the point 0x11 of the target's model from
/// slot 268, `w` and `+0x28` 1.0) and `M` (`+0x1c..+0x28` 1.0) appended to
/// the node list ([`append`]), the group `[P, LIST, NAME, N, DATA, M]`
/// inserted ([`node_insert`](crate::nodes::node_insert)), the particle node
/// ([`particle_node`](crate::sb_nodes::particle_node), `[C, OWNER, P,
/// *G_O, Q, 0]`, `C`'s vtable first) and the effect node
/// ([`effect_node`](crate::sb_nodes::effect_node), `[E, LIST, P, NAME2,
/// DATA2, 0]`, then its vtable, `+0x2c |= 1`, `+0x30 = 0`); step 12 (from
/// frame 21): slot 239 `(*G_X, model i, 0)`, the next model, back to step
/// 11 until all `*G_N` are done; step 13 (from frame 81): the colours 1.0
/// and the camera's way back ([`move_start_2`](crate::sb_misc::move_start_2)),
/// step 10000. Step 10000 takes the task's node (`task[0]`) off its list.
/// Returns the `eax` the code leaves.
#[allow(clippy::too_many_lines)] // the steps as the original orders them
pub fn step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let r = |g: &G, off: u32| g.read::<u32>(w(off));
    let task = fr.arg(g, 0);
    let s = g.read::<u32>(task + 4);
    let fl = |g: &G, at: u32| f32::from_bits(g.read::<u32>(at));
    let x = add(fl(g, s + 0x1c), fl(g, w(0x1b)));
    let frame = g.read::<u32>(s + 0x28).wrapping_add(1);
    g.write::<u32>(s + 0x28, frame);
    let state = g.read::<u32>(s + 0x18);
    g.write::<u32>(s + 0x1c, x.to_bits());
    let y = add(fl(g, s + 0x20), fl(g, w(0x36)));
    g.write::<u32>(s + 0x20, y.to_bits());
    let count = |g: &mut G| {
        let v = g.read::<u32>(s + 0x28).wrapping_add(1);
        g.write::<u32>(s + 0x28, v);
    };
    // push ebp; and esp, -8; sub esp, 0x2c; push ebx, esi, edi
    let base = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x38);
    let callee = |at: u32, ecx: u32| Frame {
        at,
        esp: base.wrapping_sub(4),
        eax: 0,
        ecx,
        edx: 0,
        xmm: [0; 4],
    };
    let rec = base + 0x20;
    let mut eax = state;
    if state > 0xa {
        if state > 0xd {
            if state == 0x2710 {
                // the task's list node
                let e = g.read::<u32>(task);
                let (next, prev) = (g.read::<u32>(e + 4), g.read::<u32>(e + 8));
                g.write::<u32>(next + 8, prev);
                let prev = g.read::<u32>(e + 8);
                let next = g.read::<u32>(e + 4);
                g.write::<u32>(prev + 4, next);
                g.write::<u32>(e + 8, e);
                g.write::<u32>(e + 4, e);
                eax = next;
            }
            count(g);
            return Ret::eax(eax);
        }
        if state == 0xd {
            if frame.cast_signed() <= 0x50 {
                count(g);
                return Ret::eax(state);
            }
            let (yy, xx) = (fl(g, w(0x3c5)), fl(g, w(0x3cd)));
            for off in [0x3d3, 0x3dd, 0x3e7, 0x3f1, 0x3fb, 0x405, 0x40f] {
                g.write::<u32>(w(off), 0x3f80_0000);
            }
            let at = img.rel32_target(f + 0x418);
            let e = crate::sb_misc::move_start_2(g, img, at, xx, yy);
            count(g);
            g.write::<u32>(s + 0x18, 0x2710);
            return Ret::eax(e);
        }
        if state == 0xc {
            if frame.cast_signed() <= 0x14 {
                count(g);
                return Ret::eax(0);
            }
            let i = g.read::<u32>(s + 0x24);
            let model = g.read::<u32>(w(0x1ca).wrapping_add(i.wrapping_mul(0x98)));
            let mut p = Pushes::default();
            p.push_all(&[0, model, r(g, 0x1d5)]);
            let t = slot_target(g, w(0x1cf), 239);
            // call width: 12 (Pushes caps the words at 12)
            p.call(g, t, 0);
            let i = g.read::<u32>(s + 0x24).wrapping_add(1);
            g.write::<u32>(s + 0x24, i);
            let step = g.read::<u32>(s + 0x18);
            let back = i.cast_signed() < r(g, 0x1ec).cast_signed();
            g.write::<u32>(
                s + 0x18,
                if back {
                    step.wrapping_sub(1)
                } else {
                    step.wrapping_add(1)
                },
            );
            count(g);
            return Ret::eax(i);
        }
        // step 11
        eax = 0;
    } else if state == 0 || state == 0xa {
        if state == 0 {
            for (o, v) in [(0u32, w(0x63)), (4, w(0x6b)), (8, r(g, 0x57)), (0xc, 0)] {
                g.write::<u32>(rec + o, v);
            }
            g.write::<u8>(rec + 0x10, 0);
            g.write::<u32>(w(0x82), w(0x86));
            crate::s3_summon::model_nodes(g, img, &callee(img.rel32_target(f + 0x8b), rec));
            let pair = base + 0x18;
            g.write::<u32>(pair, w(0x97));
            g.write::<u32>(pair + 4, w(0x9f));
            g.write::<u32>(w(0xa5), w(0xa9));
            let e = crate::s3_misc::colour_node(g, img, &callee(img.rel32_target(f + 0xae), pair));
            let x = fl(g, w(0xb6));
            for off in [0xbc, 0xc6, 0xd0, 0xda, 0xe4, 0xee, 0xf8] {
                g.write::<u32>(w(off), 0x3f00_0000);
            }
            eax = crate::sb_misc::move_start(g, img, img.rel32_target(f + 0x101), x, e);
            g.write::<u32>(s + 0x18, 0xa);
        }
        let v = g.read::<u32>(s + 0x18).wrapping_add(1);
        g.write::<u32>(s + 0x18, v);
        g.write::<u32>(s + 0x28, 0x64);
        g.write::<u32>(s + 0x24, 0);
    } else {
        count(g);
        return Ret::eax(state);
    }
    // step 11: model `i`'s nodes from frame 16
    if g.read::<u32>(s + 0x28).cast_signed() <= 0xf {
        count(g);
        return Ret::eax(eax);
    }
    let i = g.read::<u32>(s + 0x24);
    let part = w(0x133).wrapping_add(i.wrapping_mul(0x14c));
    let q = i.wrapping_mul(0x2c);
    g.write::<u16>(w(0x143).wrapping_add(q), 0xff);
    let n = w(0x14b).wrapping_add(q);
    let m = w(0x151).wrapping_add(q);
    let wf = |o: u32| w(o);
    append(
        g,
        &wf,
        n,
        [0x162, 0x169, 0x16e, 0x179, 0x188, 0x190, 0x212, 0x218],
    );
    let c = (i << 8).wrapping_add(w(0x17f));
    g.write::<u32>(n, w(0x21e));
    let target = i.wrapping_mul(0x98).wrapping_add(w(0x22c));
    let mut p = Pushes::default();
    p.push_all(&[n + 0x1c, 0x11, g.read::<u32>(target + 0x54)]);
    let t = slot_target(g, w(0x23a), 268);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, t, 0);
    g.write::<u32>(n + 0x28, 0x3f80_0000);
    g.write::<u32>(n + 0x28, 0x3f80_0000);
    g.write::<u16>(m + 0x1a, 0xff);
    append(
        g,
        &wf,
        m,
        [0x267, 0x26e, 0x273, 0x27b, 0x283, 0x28b, 0x297, 0x2a1],
    );
    g.write::<u32>(m, w(0x2a7));
    for o in [0x1c, 0x20, 0x24, 0x28] {
        g.write::<u32>(m + o, 0x3f80_0000);
    }
    for (o, v) in [
        (0x14u32, m),
        (0, part),
        (4, w(0x2d7)),
        (8, w(0x2df)),
        (0xc, n),
        (0x10, w(0x2eb)),
    ] {
        g.write::<u32>(rec + o, v);
    }
    crate::nodes::node_insert(g, img, &callee(img.rel32_target(f + 0x2f0), rec));
    let owner = r(g, 0x2f9);
    g.write::<u8>(part + 0x148, 1);
    g.write::<u32>(rec, c);
    g.write::<u32>(c, w(0x30a));
    for (o, v) in [
        (4u32, w(0x316)),
        (8, part),
        (0xc, owner),
        (0x10, w(0x326)),
        (0x14, 0),
    ] {
        g.write::<u32>(rec + o, v);
    }
    crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x333), rec);
    let i = g.read::<u32>(s + 0x24);
    g.write::<u32>(rec + 4, w(0x33e));
    g.write::<u32>(rec + 8, part);
    let e = i.wrapping_mul(56).wrapping_add(w(0x352));
    for (o, v) in [(0u32, e), (0xc, w(0x362)), (0x10, w(0x36a)), (0x14, 0)] {
        g.write::<u32>(rec + o, v);
    }
    crate::sb_nodes::effect_node(g, img, img.rel32_target(f + 0x377), rec);
    let e = g.read::<u32>(rec);
    g.write::<u32>(e, w(0x381));
    let i = g.read::<u32>(s + 0x24);
    let at = i.wrapping_mul(56).wrapping_add(w(0x394));
    let v = g.read::<u32>(at) | 1;
    g.write::<u32>(at, v);
    g.write::<u32>(i.wrapping_mul(56).wrapping_add(w(0x39c)), 0);
    let v = g.read::<u32>(s + 0x18).wrapping_add(1);
    g.write::<u32>(s + 0x18, v);
    g.write::<u32>(s + 0x28, 1);
    Ret::eax(i)
}

/// [`step`] (1112 bytes, 1 overlay).
pub const STEP: Sig = Sig {
    len: 1112,
    hash: 0x0d2f_4f99_40de_68f0,
    calls: &[
        (0x8b, Callee::Sig(&crate::s3_summon::MODEL_NODES)),
        (0xae, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0x101, Callee::Sig(&crate::sb_misc::MOVE_START)),
        (0x2f0, Callee::Sig(&crate::nodes::INSERT)),
        (0x333, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x377, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x418, Callee::Sig(&crate::sb_misc::MOVE_START_2)),
    ],
    imports: &[],
};
