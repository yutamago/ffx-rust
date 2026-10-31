//! `magic_0664.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Starts a ring part (0x10001810, an overlay table entry, `(obj)`; two
/// table entries): [`start_pair`](crate::part_draw::start_pair) with
/// program 4; `+0x44` 0; the first part `+0x40` 78, `+0x48` 7, `+0x4c`
/// 0, any other 85, 8, 1; `+0x50..+0x58` 0, `+0x5c` 1, the colour
/// 0x3e21611c, `+0x6c` 1.0. Returns 1.
pub fn ring_part<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let c = 0x3e21_611c;
    crate::part_draw::start_pair(
        g,
        img,
        fr,
        (0x36, 0x10),
        &[(0x44, 0)],
        &[(0x40, 0x429c_0000), (0x48, 0x40e0_0000), (0x4c, 0)],
        &[
            (0x40, 0x42aa_0000),
            (0x48, 0x4100_0000),
            (0x4c, 0x3f80_0000),
        ],
        &[
            (0x50, 0),
            (0x54, 0),
            (0x58, 0),
            (0x5c, 1),
            (0x60, c),
            (0x64, c),
            (0x68, c),
            (0x6c, 0x3f80_0000),
        ],
    )
}

/// [`ring_part`] (190 bytes, 1 overlay).
pub const RING_PART: Sig = Sig {
    len: 190,
    hash: 0x09c2_7ac2_ea1e_c7b0,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// An object's frame (0x100012f0, an overlay table entry, `(obj)`): slot
/// 237 `(G_A, 3, 0, obj + 0x40)` (`+0x1d`), `+0x44` -0.01; slot 258
/// `(obj)`, slot 231 with the globals at `+0x51`, `+0x45`, `+0x3a` and
/// slot 230 with `+0x64`, `+0x59` (the earlier words left on the stack);
/// `*G_P = ITEM` (`+0x71`, `+0x79`), slot 712 `(obj, 0x30)` with the game
/// flag `*G_LOCK` (`+0x7e`) 1 and restored after (`+0x9a`). When 712
/// says nonzero, `*G_P = ITEM` again, slot 713 `(obj)` removes it and the
/// part starts again with program 6
/// ([`start_part`](crate::part_draw::start_part)). Returns 1.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let t = slot_target(g, w(0xd), 237);
    let a = g.read::<u32>(w(0x1d));
    let mut stack = vec![a, 3, 0, obj + 0x40];
    g.call(t, 0, &stack);
    g.write::<u32>(obj + 0x44, 0xbc23_d70a);
    let t = slot_target(g, w(0x24), 258);
    stack.insert(0, obj);
    // call width: 5 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    for op in [0x3a, 0x45, 0x51] {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let t = slot_target(g, w(0x3f), 231);
    // call width: 8 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    for op in [0x59, 0x64] {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let t = slot_target(g, w(0x5e), 230);
    // call width: 10 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    let gp = g.read::<u32>(w(0x71));
    g.write::<u32>(gp, w(0x79));
    let lock = g.read::<u32>(w(0x7e));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(0x8c), 712);
    stack.splice(0..0, [obj, 0x30]);
    // call width: 12 (the earlier calls' words stay on the stack)
    let r = g.call(t, 0, &stack).eax;
    let lock = g.read::<u32>(w(0x9a));
    g.write::<u32>(lock, saved);
    if r == 0 {
        return Ret::eax(1);
    }
    let gp = g.read::<u32>(w(0xa8));
    g.write::<u32>(gp, w(0xaf));
    let t = slot_target(g, w(0xb4), 713);
    g.call(t, 0, &[obj]);
    crate::part_draw::start_part(g, img, f, obj, [0xc1, 0xc7, 0xcc, 0xd4, 0xef], 0x18, &[obj]);
    Ret::eax(1)
}

/// [`frame`] (255 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 255,
    hash: 0x1a6a_5225_df7f_809a,
    calls: &[(0xef, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// The effect's frame (0x10002200, an overlay table entry, no
/// arguments): 0 while the flag at operand `+0x02` is clear. Else `*G_P =
/// ITEM` (`+0x0d`, `+0x19`), slots 723 `(*G_A)` and 724 `(*G_B)`, `*G_V =
/// *G_W` (`+0x3f`, `+0x44`); the timelines `A` (`+0x61`) and `B`
/// (`+0x81`) run while slot 261 says so for 0 and 2
/// ([`timeline_run`](crate::sb_misc::timeline_run)); when it says so
/// for 1 both stop ([`timeline_stop`](crate::sb_misc::timeline_stop))
/// and slot 214 `(*G_E)` (`+0xbb`) runs, else the frame count `+0xd2`
/// goes up. Returns 1.
pub fn effect_frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    if g.read::<u32>(w(0x2)) == 0 {
        return Ret::eax(0);
    }
    let gp = g.read::<u32>(w(0xd));
    let a = g.read::<u32>(w(0x13));
    g.write::<u32>(gp, w(0x19));
    let t = slot_target(g, w(0x1e), 723);
    g.call(t, 0, &[a]);
    let b = g.read::<u32>(w(0x31));
    let t = slot_target(g, w(0x2b), 724);
    g.call(t, 0, &[b, a]);
    let v = g.read::<u32>(w(0x3f));
    let x = g.read::<u32>(w(0x44));
    g.write::<u32>(v, x);
    let t = slot_target(g, w(0x4b), 261);
    if g.call(t, 0, &[0, b, a]).eax != 0 {
        crate::sb_misc::timeline_run(g, img, img.rel32_target(f + 0x66), w(0x61));
    }
    let t = slot_target(g, w(0x6b), 261);
    if g.call(t, 0, &[2]).eax != 0 {
        crate::sb_misc::timeline_run(g, img, img.rel32_target(f + 0x86), w(0x81));
    }
    let t = slot_target(g, w(0x8b), 261);
    if g.call(t, 0, &[1]).eax == 0 {
        let n = w(0xd2);
        g.write::<u32>(n, g.read::<u32>(n).wrapping_add(1));
        return Ret::eax(1);
    }
    crate::sb_misc::timeline_stop(g, w(0xa1));
    crate::sb_misc::timeline_stop(g, w(0xab));
    let t = slot_target(g, w(0xb5), 214);
    let e = g.read::<u32>(w(0xbb));
    g.call(t, 0, &[e]);
    Ret::eax(1)
}

/// [`effect_frame`] (220 bytes, 1 overlay).
pub const EFFECT_FRAME: Sig = Sig {
    len: 220,
    hash: 0xe003_90e3_2768_ff5b,
    calls: &[
        (0x66, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0x86, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0xa6, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
        (0xb0, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
    ],
    imports: &[],
};
