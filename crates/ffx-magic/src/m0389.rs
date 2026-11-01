//! `magic_0389.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::{Frame, Game, Image, Ret, Sig};

/// A strip node's record set-up (0x10001990, internal: `ecx` = `[node,
/// parent, _, _, v]`; no frame): the record's `+8` = the node's `+0x60`
/// and `+0xc` 0x400080; the node under its parent
/// ([`node_under`](crate::m0186::node_under)); the node's `+0x20` = its
/// `+0x60`, `+0x1c` 0x400080, `+0x30` (u16) and `+0x32` 0, `+0x48` = v,
/// `+0x58`/`+0x5c` 0. Returns v.
pub fn strip_record<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let r = fr.ecx;
    let node = g.read::<u32>(r);
    let parent = g.read::<u32>(r + 4);
    let buf = node.wrapping_add(0x60);
    g.write::<u32>(r + 8, buf);
    g.write::<u32>(r + 0xc, 0x0040_0080);
    crate::m0186::node_under(g, node, parent);
    g.write::<u32>(node + 0x20, buf);
    g.write::<u32>(node + 0x1c, 0x0040_0080);
    g.write::<u32>(node + 0x32, 0);
    g.write::<u16>(node + 0x30, 0);
    let v = g.read::<u32>(r + 0x10);
    g.write::<u32>(node + 0x48, v);
    g.write::<u32>(node + 0x58, 0);
    g.write::<u32>(node + 0x5c, 0);
    Ret::eax(v)
}

/// [`strip_record`] (116 bytes, 1 overlay).
pub const STRIP_RECORD: Sig = Sig {
    len: 116,
    hash: 0x6b3e_91c3_ff5b_192f,
    calls: &[],
    imports: &[],
};

/// The model's two-way switch to motion 0x10791000 (0x10006e30, no
/// arguments, no frame; `m0297::mode_switch` with the value fixed): with
/// bit 0 of the flags halfword `*F` (`+0x02`) set, slot 30 `(*A)` and
/// slot 36 `(*A, v)` (`A` `+0x10`/`+0x25`) and the flags gain 0x300;
/// else slot 228 `(*B, v)` (`B` `+0x51`) and they gain 0x100. `+0x6a` =
/// v. Returns the last slot's `eax` with its low half the new flags.
pub fn mode_switch<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::runtime::slot_target;
    let w = |off: u32| img.word(fr.at + off);
    let v = 0x1079_1000;
    let on = g.read::<u8>(w(0x02)) & 1 != 0;
    let (r, flags) = if on {
        let a = g.read::<u32>(w(0x10));
        let s = slot_target(g, w(0x08), 0x78 / 4);
        g.call(s, 0, &[a]);
        let s = slot_target(g, w(0x1a), 0x90 / 4);
        let a2 = g.read::<u32>(w(0x25));
        // call width: 3 (the earlier call's word under it)
        let r = g.call(s, 0, &[a2, v, a]).eax;
        // the path joins the other one's `or ax, 0x100`
        (r, g.read::<u16>(w(0x33)) | 0x200 | 0x100)
    } else {
        let b = g.read::<u32>(w(0x51));
        let s = slot_target(g, w(0x08), 0x390 / 4);
        let r = g.call(s, 0, &[b, v]).eax;
        (r, g.read::<u16>(w(0x59)) | 0x100)
    };
    g.write::<u32>(w(0x6a), v);
    g.write::<u16>(w(0x74), flags);
    Ret::eax((r & 0xffff_0000) | u32::from(flags))
}

/// [`mode_switch`] (121 bytes, 1 overlay).
pub const MODE_SWITCH: Sig = Sig {
    len: 121,
    hash: 0x5ed2_fb24_dbae_0b6d,
    calls: &[],
    imports: &[],
};
