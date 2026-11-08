//! `magic_0665.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::{Callee, Frame, Game, Image, Sig};

/// Resets the timeline block (0x10002bd0, internal, no arguments): the
/// 0x110 bytes at operand `+0x08` zeroed (`memset`), its callback word
/// `+0x16` the immediate `+0x1a`, the counts `+0x20`/`+0x2a` 9 and the
/// nine flags (`+0x34` ... `+0x84`) 1. Returns the block (`memset`'s
/// `eax`).
pub fn timeline_reset<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let block = w(0x8);
    g.write_bytes(block, &[0u8; 0x110]);
    g.write::<u32>(w(0x16), w(0x1a));
    g.write::<u32>(w(0x20), 9);
    g.write::<u32>(w(0x2a), 9);
    for op in [0x34, 0x3e, 0x48, 0x52, 0x5c, 0x66, 0x70, 0x7a, 0x84] {
        g.write::<u32>(w(op), 1);
    }
    block
}

/// [`timeline_reset`] (141 bytes, 1 overlay).
pub const TIMELINE_RESET: Sig = Sig {
    len: 141,
    hash: 0xf969_ca9a_59d9_ac1b,
    calls: &[(0xd, Callee::Import("memset"))],
    imports: &[],
};

/// The particle effect's data set up (0x10001e10, a table entry, cdecl
/// `(arg)`; 336 bytes): [`ppp_effect_setup`](crate::s5_misc::ppp_effect_setup)'s
/// head and its three programs, then (the last program's word popped)
/// the timeline block reset ([`timeline_reset`], the `rel32` at `+0x119`)
/// and slot 260 `(0)` and `(1)` (the second over the first's word) into
/// `*G_A` (`+0x12d`) and `*G_B` (`+0x144`). Returns 1.
pub fn ppp_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> crate::Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    crate::s5_misc::ppp_effect_head(g, img, fr);
    let mut s = crate::pushes::Pushes::default();
    crate::s5_misc::ppp_effect_programs(g, img, f, &mut s, (0xf7, 0x114));
    s.pop(1);
    let reset = Frame {
        at: img.rel32_target(f + 0x119),
        // ebp, esi and the last program's word pushed, then the return
        esp: fr.esp.wrapping_sub(16),
        ..Frame::default()
    };
    timeline_reset(g, img, &reset);
    for (k, (table, dst)) in [(0x11eu32, 0x12du32), (0x132, 0x144)]
        .into_iter()
        .enumerate()
    {
        s.push(k as u32);
        // call width: 12 (the most `Pushes::words` holds)
        let r = s
            .call(g, crate::runtime::slot_target(g, w(table), 260), 0)
            .eax;
        g.write::<u32>(w(dst), r);
    }
    crate::Ret::eax(1)
}

/// [`ppp_setup`] (336 bytes, 1 overlay).
pub const PPP_SETUP: Sig = Sig {
    len: 336,
    hash: 0x4255_723d_018f_ea1e,
    calls: &[(0x119, Callee::Sig(&TIMELINE_RESET))],
    imports: &[],
};

/// An object's frame (0x10001970, an overlay table entry, `(obj)`):
/// [`frame_with`](crate::m0664::frame_with) with slot 237's mode 7 and
/// the restart's program 13. Returns 1.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> crate::Ret {
    crate::m0664::frame_with(g, img, fr, (7, 0x34))
}

/// [`frame`] (255 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 255,
    hash: 0x8e7f_c1c4_a957_916a,
    calls: &[(0xef, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};
