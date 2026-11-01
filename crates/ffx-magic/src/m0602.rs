//! `magic_0602.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::sa_pool::give_back;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Whether the three sounds 0x301a..0x301c are all free (0x10006a20,
/// internal, no arguments): for each, slot 5 `(id)` must say 0 and slot
/// 28 `(id, 1)` must say 0. Returns 1 when they are, else 0.
pub fn sounds_free<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    for (id, o5, o28) in [
        (0x301a, 0x1, 0x1b),
        (0x301b, 0x39, 0x4f),
        (0x301c, 0x6d, 0x83),
    ] {
        let t = slot_target(g, w(o5), 5);
        if g.call(t, 0, &[id]).eax != 0 {
            return 0;
        }
        let t = slot_target(g, w(o28), 28);
        if g.call(t, 0, &[id, 1]).eax != 0 {
            return 0;
        }
    }
    1
}

/// [`sounds_free`] (164 bytes, 1 overlay).
pub const SOUNDS_FREE: Sig = Sig {
    len: 164,
    hash: 0x97a6_6636_0cdd_a3cb,
    calls: &[],
    imports: &[],
};

/// Stops the effect's sounds (0x100077c0, internal, no arguments): slot
/// 384 `(ACTORS[e[k]])` for the record bytes `e[3..6]` (the record
/// `*G_E` at operand `+0x01`, the table at `+0x0c`), slots 25 `(id, 1)`
/// and 24 `(id, 0)` for each sound 0x301a..0x301c, slot 271 `(1)`, each
/// seeing the earlier calls' words; then the node `*G_N` (`+0xce`), when
/// there is one, goes back to the pool ([`give_back`]) and `*G_N` is 0.
/// Returns the give-back's `eax`, else slot 271's.
pub fn sounds_stop<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let mut stack = Vec::new();
    for (e_op, k, t_op, s_op) in [
        (0x1, 3, 0xc, 0x11),
        (0x1e, 4, 0x29, 0x2e),
        (0x3b, 5, 0x46, 0x4b),
    ] {
        let e = g.read::<u32>(w(e_op));
        let b = u32::from(g.read::<u8>(e.wrapping_add(k)));
        let a = g.read::<u32>(w(t_op).wrapping_add(4 * b));
        stack.insert(0, a);
        let t = slot_target(g, w(s_op), 384);
        // call width: 3 at most (the earlier calls' words stay on the stack)
        g.call(t, 0, &stack);
    }
    for (id, o25, o24) in [
        (0x301a, 0x58, 0x69),
        (0x301b, 0x7a, 0x8b),
        (0x301c, 0x9c, 0xad),
    ] {
        for (op, slot, k) in [(o25, 25, 1), (o24, 24, 0)] {
            stack.splice(0..0, [id, k]);
            let t = slot_target(g, w(op), slot);
            // call width: 15 at most (the earlier calls' words stay on the stack)
            g.call(t, 0, &stack);
        }
    }
    stack.insert(0, 1);
    let t = slot_target(g, w(0xbe), 271);
    // call width: 16 (the earlier calls' words stay on the stack)
    let eax = g.call(t, 0, &stack).eax;
    let node = g.read::<u32>(w(0xce));
    if node == 0 {
        return eax;
    }
    let eax = give_back(g, img, img.rel32_target(f + 0xda), node);
    g.write::<u32>(w(0xe0), 0);
    eax
}

/// [`sounds_stop`] (233 bytes, 1 overlay).
pub const SOUNDS_STOP: Sig = Sig {
    len: 233,
    hash: 0x3e88_2253_0b7b_7d0a,
    calls: &[(0xda, Callee::Sig(&crate::sa_pool::GIVE_BACK))],
    imports: &[],
};

/// The effect's start (0x10006650, an overlay table entry, cdecl
/// `(arg)`): [`crate::m0599::start`]'s head (the object block, slots
/// 556, 555, 331, the scale, the task ring, the payloads, slot 720), then
/// the states 0x301a..0x301c ([`crate::s4_small::states_301a`]); slot 718
/// `(D)`, slots 222 and 221 `(D, 0, 1000, 1)`, slot 718 `(E)`, slot 221
/// `(E, 0, 1000, 1)`; a word 0. Each call sees the words the earlier
/// ones left. Returns 0.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let mut stack = crate::m0599::start_head(g, img, fr);
    // ebp, the two words slot 720 left, the return address
    let callee = Frame {
        at: img.rel32_target(f + 0xc7),
        esp: fr.esp.wrapping_sub(0x10),
        ..Frame::default()
    };
    crate::s4_small::states_301a(g, img, &callee);
    stack.insert(0, w(0xd1));
    let t = slot_target(g, w(0xcc), 718);
    // call width: 3
    g.call(t, 0, &stack);
    stack.splice(0..0, [w(0xf2), 0, 0x3e8, 1]);
    let t = slot_target(g, w(0xde), 222);
    // call width: 7
    g.call(t, 0, &stack);
    stack.splice(0..0, [w(0x10d), 0, 0x3e8, 1]);
    let t = slot_target(g, w(0xf9), 221);
    // call width: 11
    g.call(t, 0, &stack);
    stack.insert(0, w(0x119));
    let t = slot_target(g, w(0x114), 718);
    // call width: 12
    g.call(t, 0, &stack[..12]);
    let t = slot_target(g, w(0x126), 221);
    // call width: 4 (the earlier words under them past the twelve the
    // harness and the hosts pass)
    g.call(t, 0, &[w(0x13a), 0, 0x3e8, 1]);
    g.write::<u32>(w(0x145), 0);
    Ret::eax(0)
}

/// [`start`] (337 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 337,
    hash: 0x3e4e_76f7_3a7b_d05c,
    calls: &[
        (0x0d, Callee::Sig(&crate::l5_objhost::OBJ_BLOCK_INIT)),
        (0x7c, Callee::Sig(&crate::l5_objhost::TASKS_INIT)),
        (0xc7, Callee::Sig(&crate::s4_small::STATES_301A)),
    ],
    imports: &[],
};
