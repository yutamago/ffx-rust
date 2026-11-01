//! `magic_0297.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// A model's motion set-up (0x100031a0, internal: `ecx` = `[obj, a, b,
/// c, d, _, e]`): `obj`'s `+0x90` = a; with `m` = `obj +0x88`'s `+0x54`:
/// slot 363 `(m, d, e)` into the record's `+0x1c`, slots 357
/// `(m, -1, b >> 6)` and 356 `(m, -1, c >> 6)`; then slot 362 `(a, m, d, 0)` when
/// `e` is set, else slot 704 with the same words. Returns the last slot's
/// `eax`. Each slot call passes the words the original leaves on the
/// stack.
pub fn motion_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let r = fr.ecx;
    let obj = g.read::<u32>(r);
    let mut p = Pushes::default();
    p.push(g.read::<u32>(r + 0x18));
    let m = g.read::<u32>(g.read::<u32>(obj + 0x88) + 0x54);
    p.push(g.read::<u32>(r + 0x10));
    g.write::<u32>(obj + 0x90, g.read::<u32>(r + 4));
    p.push(m);
    let s = slot_target(g, w(0x1f), 0x5ac / 4);
    // call width: 12 (Pushes caps the words at 12)
    let v = p.call(g, s, 0).eax;
    g.write::<u32>(r + 0x1c, v);
    for (field, table, slot) in [(8, 0x37, 0x594), (0xc, 0x4e, 0x590)] {
        p.push_all(&[g.read::<u32>(r + field) >> 6, u32::MAX, m]);
        let s = slot_target(g, w(table), slot / 4);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, s, 0);
    }
    p.pop(9);
    let e = g.read::<u32>(r + 0x18);
    p.push_all(&[0, g.read::<u32>(r + 0x10), m, g.read::<u32>(r + 4)]);
    let s = slot_target(g, w(0x5e), if e != 0 { 0x5a8 / 4 } else { 0xb00 / 4 });
    // call width: 12 (Pushes caps the words at 12)
    Ret::eax(p.call(g, s, 0).eax)
}

/// [`motion_setup`] (144 bytes, 1 overlay).
pub const MOTION_SETUP: Sig = Sig {
    len: 144,
    hash: 0x558d_e090_a6bd_5dc7,
    calls: &[],
    imports: &[],
};

/// A model's two-way switch (0x100073d0, internal: `edx` = `v`, no
/// frame): with bit 0 of the flags halfword `*F` (`+0x02`) set, slot 30
/// `(*A)` and slot 36 `(*A, v)` (`A` `+0x13`/`+0x24`) and the flags gain
/// 0x300 (0x200, then the other path's 0x100); else slot 228 `(*B, v)`
/// (`B` `+0x4c`) and they gain 0x100.
/// `+0x65` = v. Returns the last slot's `eax` with its low half the new
/// flags.
pub fn mode_switch<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let v = fr.edx;
    let on = g.read::<u8>(w(0x02)) & 1 != 0;
    let (r, flags) = if on {
        let a = g.read::<u32>(w(0x13));
        let s = slot_target(g, w(0x08), 0x78 / 4);
        g.call(s, 0, &[a]);
        let s = slot_target(g, w(0x1d), 0x90 / 4);
        let a2 = g.read::<u32>(w(0x24));
        // call width: 3 (the earlier call's word under it)
        let r = g.call(s, 0, &[a2, v, a]).eax;
        // the path joins the other one's `or ax, 0x100`
        (r, g.read::<u16>(w(0x32)) | 0x200 | 0x100)
    } else {
        let b = g.read::<u32>(w(0x4c));
        let s = slot_target(g, w(0x08), 0x390 / 4);
        let r = g.call(s, 0, &[b, v]).eax;
        (r, g.read::<u16>(w(0x54)) | 0x100)
    };
    g.write::<u32>(w(0x65), v);
    g.write::<u16>(w(0x6b), flags);
    Ret::eax((r & 0xffff_0000) | u32::from(flags))
}

/// [`mode_switch`] (113 bytes, 1 overlay).
pub const MODE_SWITCH: Sig = Sig {
    len: 113,
    hash: 0xc2b3_64c1_759b_4af3,
    calls: &[],
    imports: &[],
};
