//! `magic_0597.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::nodes::visit_gated;
use crate::runtime::slot_target;
use crate::sa_summon::{bind_models, bone_plain};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The summon's frame (0x1000aee0, a table entry, no arguments; nothing
/// when the phase word, operand `+0x0b`, is below 4): the DLL's pointer
/// (`+0x18`) gets its block (`+0x1e`), a word is copied (`+0x28` to
/// `+0x2d`), [`bind_models`]; the word `+0x3c` 0; the bone (`+0x48`)
/// updated with the pass byte (`+0x32`, not raised;
/// [`bone_plain`]); unless the flag `+0x57` is set, every child of the
/// root (`+0x66`, its first child `+0x5e`) gets its method `+0x20` with
/// the cursor; with no child left the done bit 0 of `+0xa0` is set. The
/// nodes `+0xb9` and `+0xd9` are visited ([`visit_gated`]) with the
/// passes `+0xa8`/`+0xc5`; when bit 0 of `+0xe3 & +0xe9` is set and the
/// halfword `+0xf7` is 0, slot 214 `(+0x105)`. Returns 0.
///
/// The cursor's bytes `+0x4`, `+0x6`, `+0x7` are never written (junk
/// reads: 0; brief 2026-10-24).
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    if g.read::<i32>(w(0x0b)) < 4 {
        return Ret::eax(0);
    }
    g.write::<u32>(g.read::<u32>(w(0x18)), w(0x1e));
    bind_models(g, img, img.rel32_target(f + 0x23));
    g.write::<u32>(w(0x2d), g.read::<u32>(w(0x28)));
    let pass = g.read::<u8>(w(0x32));
    g.write::<u32>(w(0x3c), 0);
    // push ebp; and esp, -8; sub esp, 0xc; push esi: the cursor at
    // [esp + 8]
    let e = fr.esp.wrapping_sub(4) & !7;
    let cursor = e.wrapping_sub(8);
    g.write::<u32>(cursor, w(0x48));
    g.write::<u8>(cursor + 4, 0);
    g.write::<u8>(cursor + 5, pass);
    g.write::<u16>(cursor + 6, 0);
    let bone = Frame {
        at: img.rel32_target(f + 0x51),
        esp: e.wrapping_sub(0x14),
        ecx: cursor,
        ..*fr
    };
    bone_plain(g, img, &bone);
    let root = w(0x66);
    let skip = g.read::<u32>(w(0x57)) != 0;
    let mut child = g.read::<u32>(w(0x5e));
    if !skip && child != root {
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
        child = g.read::<u32>(w(0x92));
    }
    if child == root {
        let done = w(0xa0);
        g.write::<u32>(done, g.read::<u32>(done) | 1);
    }
    for (pass_at, node_at, call_at) in [(0xa8u32, 0xb9u32, 0xbeu32), (0xc5, 0xd9, 0xde)] {
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
        visit_gated(g, img, &fr);
    }
    if g.read::<u32>(w(0xe3)) & g.read::<u32>(w(0xe9)) & 1 != 0 && g.read::<u16>(w(0xf7)) == 0 {
        let s = slot_target(g, w(0xff), 214);
        let x = g.read::<u32>(w(0x105));
        g.call(s, 0, &[x]);
    }
    Ret::eax(0)
}

/// [`frame`] (283 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 283,
    hash: 0xd31a_c52a_e93e_1143,
    calls: &[
        (0x23, Callee::Sig(&crate::sa_summon::BIND_MODELS)),
        (0x51, Callee::Sig(&crate::sa_summon::BONE)),
        (0xbe, Callee::Sig(&crate::nodes::GATED)),
        (0xde, Callee::Sig(&crate::nodes::GATED)),
    ],
    imports: &[],
};
