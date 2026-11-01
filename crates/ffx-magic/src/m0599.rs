//! `magic_0599.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The effect's start (0x10005ae0, an overlay table entry, cdecl
/// `(arg)`): `arg` into `G_ARG`; the object block set up
/// ([`crate::l5_objhost::obj_block_init`]); slots 556, 555 and 331 on the
/// three blocks; three words 1.0, slot 790's float into the scale; the
/// task ring ([`crate::l5_objhost::tasks_init`]); the 0x3ff payloads'
/// first words 0, the free count 0x3ff and the used count 0; slot 720
/// `(0x3400, 0x800)`, slot 718 `(D)`, slot 222 `(D, 0, 2, 1)`, slot 221
/// `(D, 0, 1000, 1)`, slot 718 `(E)`, slot 719 `(E, 6, 0)` gives the
/// entry, slot 551 `(entry, 0, 1)`; a word 0. Each call sees the words
/// the earlier ones left. Returns 0.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0x08), arg);
    crate::l5_objhost::obj_block_init(g, img, img.rel32_target(f + 0x0d), arg);
    let mut stack = vec![w(0x17)];
    let t = slot_target(g, w(0x12), 556);
    // call width: 1
    g.call(t, 0, &stack);
    stack.insert(0, w(0x29));
    let t = slot_target(g, w(0x24), 555);
    // call width: 2
    g.call(t, 0, &stack);
    stack.insert(0, w(0x3b));
    let t = slot_target(g, w(0x36), 331);
    // call width: 3
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x48), 790);
    for op in [0x57u32, 0x61, 0x6b] {
        g.write::<u32>(w(op), w(op + 4));
    }
    // call width: 0
    let scale = crate::ob_model::st0_f32(g.call(t, 0, &[]).st0);
    g.write::<f32>(w(0x77), scale);
    crate::l5_objhost::tasks_init(g, img, img.rel32_target(f + 0x7c));
    let payloads = w(0x81);
    for i in 0..0x3ff {
        g.write::<u32>(payloads + 12 * i, 0);
    }
    g.write::<u16>(w(0xa3), 0x3ff);
    g.write::<u16>(w(0xab), 0);
    let mut stack = vec![0x3400, 0x800];
    let t = slot_target(g, w(0xb0), 720);
    // call width: 2
    g.call(t, 0, &stack);
    stack.insert(0, w(0xcc));
    let t = slot_target(g, w(0xc7), 718);
    // call width: 3
    g.call(t, 0, &stack);
    stack.splice(0..0, [w(0xea), 0, 2, 1]);
    let t = slot_target(g, w(0xd9), 222);
    // call width: 7
    g.call(t, 0, &stack);
    stack.splice(0..0, [w(0x105), 0, 0x3e8, 1]);
    let t = slot_target(g, w(0xf1), 221);
    // call width: 11
    g.call(t, 0, &stack);
    stack.insert(0, w(0x111));
    let t = slot_target(g, w(0x10c), 718);
    // call width: 12
    g.call(t, 0, &stack[..12]);
    let t = slot_target(g, w(0x11e), 719);
    // call width: 3 (the words under them past the twelve the harness
    // and the hosts pass)
    let e = g.call(t, 0, &[w(0x12d), 6, 0]).eax;
    let t = slot_target(g, w(0x139), 551);
    // call width: 6
    g.call(t, 0, &[e, 0, 1, w(0x12d), 6, 0]);
    g.write::<u32>(w(0x14a), 0);
    Ret::eax(0)
}

/// [`start`] (342 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 342,
    hash: 0xe055_aba0_5746_0c13,
    calls: &[
        (0x0d, Callee::Sig(&crate::l5_objhost::OBJ_BLOCK_INIT)),
        (0x7c, Callee::Sig(&crate::l5_objhost::TASKS_INIT)),
    ],
    imports: &[],
};

/// The summon's sprites set up (0x10006e70, no arguments): slot 371 `(0,
/// 0x10000)` and `(1, 0x20000)`; with the block `B = *G_B`, its colour
/// words `+0xb80..+0xb88` and the halfword `+0xb98` 0x48; four sprite
/// records from `B + 0xea8` (0x8a8 bytes apart): the texture word of
/// `(*G_T)[0x20]`, 1, halfwords -1 and 0x200, the colour 0x18808080, the
/// scales 1.0, 1.0, 4.0, the count 150 and 32 words cleared. `B[0x315c]`
/// = 1; unless `B[0x3148]` already holds one, a free-list node (inline:
/// the ring's head `G_FREE`, count `u16 G_N`; reset with the callback
/// `CB`) appended to the task list goes there. Returns the `eax` the
/// original leaves (-1, 0xffff from the count's decrement, or the list's
/// old last node).
pub fn sprites_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(0x05), 371);
    // call width: 2
    g.call(t, 0, &[0, 0x1_0000]);
    let t = slot_target(g, w(0x1c), 371);
    // call width: 4 (the first call's words under them)
    g.call(t, 0, &[1, 0x2_0000, 0, 0x1_0000]);
    let b = g.read::<u32>(w(0x31));
    g.write::<u32>(b + 0xb80, 0x80ff_ffff);
    g.write::<u32>(b + 0xb84, 0x00ff_ffff);
    g.write::<u32>(b + 0xb88, 0);
    g.write::<u16>(b + 0xb98, 0x48);
    let tex = g.read::<u32>(g.read::<u32>(w(0x60)) + 0x20);
    // the local the loop reads the texture word from
    g.write::<u32>(fr.esp.wrapping_sub(8), tex);
    for i in 0..4u32 {
        let r = b + 0xea8 + 0x8a8 * i;
        g.write::<u16>(r + 8, 0xffff);
        g.write::<u16>(r + 0xa, 0x200);
        g.write::<u32>(r, tex);
        g.write::<u32>(r + 4, 1);
        g.write::<u32>(r + 0x80c, 0x8080_8080);
        g.write::<u32>(r + 0x810, 0);
        g.write::<u32>(r + 0x820, 0);
        g.write::<u32>(r + 0x814, 0x3f80_0000);
        g.write::<u32>(r + 0x818, 0x3f80_0000);
        g.write_bytes(r + 0x828, &[0; 0x80]);
        g.write::<u32>(r + 0x824, 0x96);
        g.write::<u32>(r + 0x81c, 0x4080_0000);
        g.write::<u8>(r + 0x80f, 0x18);
    }
    let b = g.read::<u32>(w(0x102));
    let held = g.read::<u32>(b + 0x3148);
    g.write::<u32>(b + 0x315c, 1);
    if held != 0 {
        return Ret::eax(u32::MAX);
    }
    // the free-list pop, inline
    let count = g.read::<u16>(w(0x120));
    let mut eax = u32::MAX;
    let node = if count == 0 {
        0
    } else {
        let node = g.read::<u32>(w(0x12f));
        let (prev, next) = (g.read::<u32>(node), g.read::<u32>(node + 4));
        g.write::<u32>(prev.wrapping_add(4), next);
        let (next, prev) = (g.read::<u32>(node + 4), g.read::<u32>(node));
        g.write::<u32>(next, prev);
        if count > 1 {
            let next = g.read::<u32>(node + 4);
            g.write::<u32>(w(0x14c), next);
        } else {
            g.write::<u32>(w(0x154), 0);
        }
        eax = 0xffff;
        for o in [0u32, 4, 0xc, 0x14] {
            g.write::<u32>(node + o, 0);
        }
        g.write::<u32>(node + 8, node);
        g.write::<u32>(node + 0x10, w(0x185));
        g.write::<u8>(node + 0x18, 0);
        g.write::<u16>(w(0x190), count.wrapping_sub(1));
        node
    };
    let first = g.read::<u32>(w(0x196));
    g.write::<u32>(node.wrapping_add(4), 0);
    if first == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x1b8), node);
    } else {
        let last = g.read::<u32>(w(0x1a5));
        eax = last;
        g.write::<u32>(node, last);
        g.write::<u32>(last.wrapping_add(4), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(0x1c1), node);
    g.write::<u32>(b + 0x3148, node);
    Ret::eax(eax)
}

/// [`sprites_setup`] (466 bytes, 1 overlay).
pub const SPRITES_SETUP: Sig = Sig {
    len: 466,
    hash: 0xc70e_a469_ee0f_b055,
    calls: &[],
    imports: &[],
};
