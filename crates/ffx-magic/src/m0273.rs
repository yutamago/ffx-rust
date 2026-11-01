//! `magic_0273.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::m0078::stores;
use crate::{Frame, Game, Image, Ret, Sig};

/// A static node appended to its list (0x10007600, no arguments, no
/// frame): its vtable (`+0x07`), pass halfword (`+0x12`), links and
/// parent (`+0x1a`.. `+0x38`), the old last (`+0x01`) as its previous;
/// an empty list (last = the sentinel `+0x46`) gets it as first
/// (`+0x55`), else it follows the old last (`+0x6b`); the u16 count
/// (`+0x4f`/`+0x72`) goes up and it becomes the last (`+0x5f`/`+0x78`).
/// Returns the old last.
pub fn static_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let last = g.read::<u32>(w(0x01));
    stores(g, img, f, &[(0x07, 0x0b)]);
    g.write::<u16>(w(0x12), 0xff);
    stores(
        g,
        img,
        f,
        &[(0x1a, 0x1e), (0x24, 0x28), (0x2e, 0x32), (0x38, 0x3c)],
    );
    g.write::<u32>(w(0x41), last);
    let count = |g: &mut G, at: u32| {
        let c = g.read::<u16>(w(at));
        g.write::<u16>(w(at), c.wrapping_add(1));
    };
    if last == w(0x46) {
        count(g, 0x4f);
        stores(g, img, f, &[(0x55, 0x59), (0x5f, 0x63)]);
    } else {
        g.write::<u32>(last + 4, w(0x6b));
        count(g, 0x72);
        stores(g, img, f, &[(0x78, 0x7c)]);
    }
    Ret::eax(last)
}

/// [`static_node`] (129 bytes, 1 overlay).
pub const STATIC_NODE: Sig = Sig {
    len: 129,
    hash: 0xbc72_a8f4_7b65_6913,
    calls: &[],
    imports: &[],
};

/// A node put at the ring's tail (0x10008170, internal: one stack word
/// `x` and `edx`): the ring's last (`+0x04`) becomes the node's previous
/// (`+0x09`), the node's words (`+0x0f`.. `+0x2d`), the old last's `+4`
/// the node; the root's flag (`+0x41`) loses bit 0, the node is the last
/// (`+0x48`); `+0x51` = x, `+0x57` = `edx`, `+0x5d` 0. Returns x.
pub fn tail_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let last = g.read::<u32>(w(0x04));
    g.write::<u32>(w(0x09), last);
    stores(
        g,
        img,
        f,
        &[(0x0f, 0x13), (0x19, 0x1d), (0x23, 0x27), (0x2d, 0x31)],
    );
    g.write::<u32>(last + 4, w(0x38));
    let x = fr.arg(g, 0);
    let fl = g.read::<u32>(w(0x41));
    g.write::<u32>(w(0x41), fl & !1);
    stores(g, img, f, &[(0x48, 0x4c)]);
    g.write::<u32>(w(0x51), x);
    g.write::<u32>(w(0x57), fr.edx);
    g.write::<u32>(w(0x5d), 0);
    Ret::eax(x)
}

/// [`tail_node`] (103 bytes, 1 overlay).
pub const TAIL_NODE: Sig = Sig {
    len: 103,
    hash: 0x0dca_b6d2_bb66_bf55,
    calls: &[],
    imports: &[],
};

/// A sprite's set-up from its record (0x10002a30, internal, `ecx` = the
/// record `[sprite, a, b, c, kind byte, d]`; a /GS frame of 0x14 bytes):
/// unless bit 0 of the sprite's halfword `+0x90` is set, `+0x92` = the
/// kind, `+0x80` and `+0x78` = a, `+0x7c` = b, `+0x84` = d, `+0x88` = c,
/// `+0x64` = slot 538 `(h, 0)` (`h` = `+0x50`) with bit 6 of `+0x90` set,
/// and slot 217 `(h, &M)` (`M` on the frame, `ebp - 0x14`); bit 10 of
/// `+0x90` is cleared either way. Returns 0xfbff (the mask's register).
/// Slot 217 passes the words the original leaves on the stack.
pub fn sprite_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let ebp = fr.esp.wrapping_sub(4);
    let r = fr.ecx;
    let sp = g.read::<u32>(r);
    if g.read::<u8>(sp + 0x90) & 1 == 0 {
        let kind = u16::from(g.read::<u8>(r + 0x10));
        g.write::<u16>(sp + 0x92, kind);
        for (dst, src) in [(0x80, 4), (0x78, 4), (0x7c, 8), (0x84, 0x14), (0x88, 0xc)] {
            let v = g.read::<u32>(r + src);
            g.write::<u32>(sp + dst, v);
        }
        let mut st = crate::pushes::Pushes::default();
        st.push_all(&[0, g.read::<u32>(sp + 0x50)]);
        let s = crate::runtime::slot_target(g, w(0x4f), 0x868 / 4);
        // call width: 12 (Pushes caps the words at 12)
        let h = st.call(g, s, 0).eax;
        let fl = g.read::<u16>(sp + 0x90);
        g.write::<u16>(sp + 0x90, fl | 0x40);
        g.write::<u32>(sp + 0x64, h);
        st.push_all(&[ebp.wrapping_sub(0x14), g.read::<u32>(sp + 0x50)]);
        let s = crate::runtime::slot_target(g, w(0x70), 0x364 / 4);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, s, 0);
    }
    let fl = g.read::<u16>(sp + 0x90);
    g.write::<u16>(sp + 0x90, fl & 0xfbff);
    Ret::eax(0xfbff)
}

/// [`sprite_setup`] (157 bytes, 1 overlay).
pub const SPRITE_SETUP: Sig = Sig {
    len: 157,
    hash: 0xe101_5bb6_01ba_0c5e,
    calls: &[
        // __security_check_cookie
        (0x95, crate::Callee::Any),
    ],
    imports: &[],
};
