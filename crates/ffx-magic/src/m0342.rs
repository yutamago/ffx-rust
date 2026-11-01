//! `magic_0342.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The summon's frame (0x10007440, a table entry, no arguments, an
/// aligned frame; `s3_summon::summon_frame` without the pointer store):
/// 0 while the phase word (`+0x0b`) is below 4; else a word is copied
/// (`+0x18` to `+0x1d`), the models are bound
/// ([`bind_models`](crate::sa_summon::bind_models)) and the pass byte
/// (`+0x27`) goes up; unless the flag (`+0x35`) is set, every child of
/// the root (`+0x43`, the first child `+0x2d`) gets its method `+0x20`
/// with a cursor on the frame; with no child left the done bit 0 of
/// `+0x85` is set. Then the node `+0x92` is visited through
/// [`bone_plain`](crate::sa_summon::bone_plain) and `+0xb3`, `+0xd3`
/// through [`visit_gated`](crate::nodes::visit_gated), each with the pass;
/// when the done bit is set and the halfword `+0xeb` is 0, slot 214
/// `(+0xf9)`. Returns 0.
///
/// The children's cursor gets only its node word: its pass byte is never
/// written (`[esp + 0xd]`, junk read: 0; brief 2026-10-24).
pub fn summon_frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    if g.read::<i32>(w(0x0b)) < 4 {
        return Ret::eax(0);
    }
    g.write::<u32>(w(0x1d), g.read::<u32>(w(0x18)));
    crate::sa_summon::bind_models(g, img, img.rel32_target(f + 0x22));
    let mut pass = g.read::<u8>(w(0x27)).wrapping_add(1);
    let mut child = g.read::<u32>(w(0x2d));
    g.write::<u8>(w(0x3b), pass);
    // push ebp; and esp, -8; sub esp, 0xc; push esi: the cursor at
    // [esp + 8]
    let e = fr.esp.wrapping_sub(4) & !7;
    let cursor = e.wrapping_sub(8);
    let root = w(0x43);
    if g.read::<u32>(w(0x35)) == 0 && child != root {
        // junk read: [esp + 0xc] never written; 0 (brief 2026-10-24)
        g.write::<u32>(cursor + 4, 0);
        loop {
            g.write::<u32>(cursor, child);
            let method = g.read::<u32>(g.read::<u32>(child) + 0x20);
            child = g.read::<u32>(child + 8);
            // push ecx: the cursor's address, then the call
            g.write::<u32>(e.wrapping_sub(0x14), cursor);
            g.call(method, cursor, &[cursor]);
            if child == root {
                break;
            }
        }
        pass = g.read::<u8>(w(0x71));
        child = g.read::<u32>(w(0x77));
    }
    if child == root {
        let done = w(0x85);
        g.write::<u32>(done, g.read::<u32>(done) | 1);
    }
    g.write::<u32>(cursor, w(0x92));
    g.write::<u8>(cursor + 5, pass);
    let bone = Frame {
        at: img.rel32_target(f + 0x9b),
        esp: e.wrapping_sub(0x14),
        ecx: cursor,
        ..*fr
    };
    crate::sa_summon::bone_plain(g, img, &bone);
    for (pass_at, node_at, call_at) in [(0xa2u32, 0xb3u32, 0xb8u32), (0xbf, 0xd3, 0xd8)] {
        let pass = g.read::<u8>(w(pass_at));
        g.write::<u8>(cursor + 5, pass);
        // push eax: the cursor's address
        g.write::<u32>(e.wrapping_sub(0x14), cursor);
        g.write::<u32>(cursor, w(node_at));
        let fr = Frame {
            at: img.rel32_target(f + call_at),
            esp: e.wrapping_sub(0x18),
            ..*fr
        };
        crate::nodes::visit_gated(g, img, &fr);
    }
    if g.read::<u32>(w(0xe1)) & 1 != 0 && g.read::<u16>(w(0xeb)) == 0 {
        let s = slot_target(g, w(0xf3), 214);
        let x = g.read::<u32>(w(0xf9));
        g.call(s, 0, &[x]);
    }
    Ret::eax(0)
}

/// [`summon_frame`] (271 bytes, 1 overlay).
pub const SUMMON_FRAME: Sig = Sig {
    len: 271,
    hash: 0x92c8_5e1d_1f6b_a7b5,
    calls: &[
        (0x22, Callee::Sig(&crate::sa_summon::BIND_MODELS)),
        (0x9b, Callee::Sig(&crate::sa_summon::BONE)),
        (0xb8, Callee::Sig(&crate::nodes::GATED)),
        (0xd8, Callee::Sig(&crate::nodes::GATED)),
    ],
    imports: &[],
};
