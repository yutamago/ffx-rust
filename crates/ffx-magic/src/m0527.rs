//! `magic_0527.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::part_draw::{Camera, Restore, Transform, camera, draw_restore, transform};
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Draws the part at the actor's bone (0x10001070, an overlay table
/// entry, no arguments; a cookie frame): the matrices ([`camera`]), slot
/// 237 `(*G_H, 0x2b, 0, &p)` fills the local point `p` (`ebp - 0x14`),
/// copied to the part's point (operands `+0x6b`, `+0x78`, `+0x85`); the
/// transform ([`transform`]) and the draw ([`draw_restore`]). Returns 1
/// when slot 712 says 0, else 0.
pub fn draw<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = fr.esp.wrapping_sub(0x18);
    let c = Camera {
        gp: 0x11,
        cam: 0x18,
        item: 0x1e,
        t723: 0x23,
        t724: 0x30,
        m: 0x36,
    };
    let mut stack = camera(g, img, f, &c, &[]);
    let t = slot_target(g, w(0x47), 237);
    stack.splice(0..0, [g.read::<u32>(w(0x57)), 0x2b, 0, p]);
    // call width: 6
    g.call(t, 0, &stack);
    for (i, op) in (0u32..).zip([0x6b, 0x78, 0x85]) {
        let v = g.read::<u32>(p + 4 * i);
        g.write::<u32>(w(op), v);
    }
    let t = Transform {
        t258: 0x63,
        part: 0x90,
        a231: [0x98, 0xa3, 0xaf],
        t231: 0x9d,
        a230: [0xb7, 0xc2],
        t230: 0xbc,
    };
    transform(g, img, f, &t, &[], &mut stack);
    let r = Restore {
        gp: 0xcf,
        item: 0xd7,
        lock: 0xdc,
        part: 0xe1,
        t712: 0xee,
        lock2: 0xfc,
    };
    let eax = draw_restore(g, img, f, &r, &stack);
    Ret::eax(u32::from(eax == 0))
}

/// 0x10001070 (285 bytes, 1 overlay).
pub const DRAW: Sig = Sig {
    len: 285,
    hash: 0x4c3f_27af_5d24_628d,
    calls: &[
        // __security_check_cookie
        (0x115, Callee::Any),
    ],
    imports: &[],
};

/// The effect's frame (0x10001410, an overlay table entry, no
/// arguments): 0 when slot 253 says 0 or the flag at operand `+0x16` is
/// clear. Else `*G_V = *G_W`; when slot 261 `(0)` says nonzero the
/// timeline runs ([`timeline`](crate::sa_misc::timeline)); when `(1)`
/// does it stops
/// ([`timeline_stop_global`](crate::sb_misc::timeline_stop_global)) and
/// slot 214 `(*G_E)` runs, else the frame count `+0x7e` goes up. Returns
/// 1.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(0x1), 253);
    if g.call(t, 0, &[]).eax == 0 || g.read::<u32>(w(0x16)) == 0 {
        return Ret::eax(0);
    }
    let v = g.read::<u32>(w(0x1f));
    let x = g.read::<u32>(w(0x24));
    g.write::<u32>(v, x);
    let t = slot_target(g, w(0x2b), 261);
    if g.call(t, 0, &[0]).eax != 0 {
        crate::sa_misc::timeline(g, img, img.rel32_target(f + 0x41));
    }
    let t = slot_target(g, w(0x46), 261);
    if g.call(t, 0, &[1]).eax == 0 {
        let n = w(0x7e);
        g.write::<u32>(n, g.read::<u32>(n).wrapping_add(1));
        return Ret::eax(1);
    }
    crate::sb_misc::timeline_stop_global(g, img, img.rel32_target(f + 0x5c));
    let t = slot_target(g, w(0x61), 214);
    let e = g.read::<u32>(w(0x67));
    g.call(t, 0, &[e]);
    Ret::eax(1)
}

/// 0x10001410 (136 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 136,
    hash: 0x9927_077d_8cd7_215d,
    calls: &[
        (0x41, Callee::Sig(&crate::sa_misc::TIMELINE)),
        (0x5c, Callee::Sig(&crate::sb_misc::TIMELINE_STOP_GLOBAL)),
    ],
    imports: &[],
};
