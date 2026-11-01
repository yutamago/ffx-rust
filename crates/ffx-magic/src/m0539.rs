//! `magic_0539.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::part_draw::{Camera, camera};
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// An object's frame (0x100010a0, `(obj)`): slot 258 `(obj)`, slots 231
/// and 230 with the camera globals, `*G_P = ITEM` and slot 712 `(obj,
/// 0x30)` with the game flag `*G_LOCK` 1 (restored after; the earlier
/// words left on the stack). Returns 0 when slot 712 says nonzero, else
/// whether the second timeline's clock (operand `+0x8e`) is not 31.
pub fn frame_obj<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let obj = fr.arg(g, 0);
    let t = slot_target(g, w(0x4), 258);
    let mut stack = vec![obj];
    g.call(t, 0, &stack);
    for op in [0x16, 0x21, 0x2d] {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let t = slot_target(g, w(0x1b), 231);
    // call width: 4
    g.call(t, 0, &stack);
    for op in [0x35, 0x40] {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let t = slot_target(g, w(0x3a), 230);
    // call width: 6
    g.call(t, 0, &stack);
    let gp = g.read::<u32>(w(0x4d));
    g.write::<u32>(gp, w(0x58));
    let lock = g.read::<u32>(w(0x5d));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(0x6a), 712);
    stack.splice(0..0, [obj, 0x30]);
    // call width: 8
    let r = g.call(t, 0, &stack).eax;
    let lock = g.read::<u32>(w(0x78));
    g.write::<u32>(lock, saved);
    if r != 0 {
        return Ret::eax(0);
    }
    Ret::eax(u32::from(g.read::<u32>(w(0x8e)) != 0x1f))
}

/// 0x100010a0 (152 bytes, 1 overlay).
pub const FRAME_OBJ: Sig = Sig {
    len: 152,
    hash: 0x9a67_8df2_9a14_4b19,
    calls: &[],
    imports: &[],
};

/// The effect's frame (0x10001640, an overlay table entry, no
/// arguments): 0 while the flag at operand `+0x02` is clear. Else the
/// matrices ([`camera`]), `*G_V = *G_W`; slot 261 `(0)` and `(1)` each
/// run a timeline ([`timeline_run`](crate::sb_misc::timeline_run)) when
/// they say nonzero; when `(2)` does both stop
/// ([`timeline_stop`](crate::sb_misc::timeline_stop)) and slot 214
/// `(*G_E)` runs, else the frame count `+0xd2` goes up. Returns 1.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    if g.read::<u32>(w(0x2)) == 0 {
        return Ret::eax(0);
    }
    let c = Camera {
        gp: 0xd,
        cam: 0x13,
        item: 0x19,
        t723: 0x1e,
        t724: 0x2b,
        m: 0x31,
    };
    let mut stack = camera(g, img, f, &c, &[]);
    let v = g.read::<u32>(w(0x3f));
    let x = g.read::<u32>(w(0x44));
    g.write::<u32>(v, x);
    let t = slot_target(g, w(0x4b), 261);
    stack.insert(0, 0);
    // call width: 3
    if g.call(t, 0, &stack).eax != 0 {
        crate::sb_misc::timeline_run(g, img, img.rel32_target(f + 0x66), w(0x61));
    }
    let t = slot_target(g, w(0x6b), 261);
    if g.call(t, 0, &[1]).eax != 0 {
        crate::sb_misc::timeline_run(g, img, img.rel32_target(f + 0x86), w(0x81));
    }
    let t = slot_target(g, w(0x8b), 261);
    if g.call(t, 0, &[2]).eax == 0 {
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

/// 0x10001640 (220 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 220,
    hash: 0x90f5_2861_f3f4_6c07,
    calls: &[
        (0x66, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0x86, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0xa6, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
        (0xb0, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
    ],
    imports: &[],
};
